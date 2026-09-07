//! Task runtime loop over ready capability invocations.

use crate::capability::{
    BoundCapabilityInstance, CapabilityCatalog, CapabilityExecutionContext,
    CapabilityInvocationPayload, CapabilityInvocationResult,
};
use crate::error::ExecutionInvariantError;
use crate::execution::{EventPublicationPort, ExecutionEventContext};
use crate::task::events::build_execution_task_envelope;
use crate::task::executor::TaskExecutor;
use crate::task::{
    parse_task_expansion_request_artifact, ArtifactRecord, CompiledTaskDelta, CompiledTaskRecord,
    TaskExpansionRequest,
};
use futures::stream::{FuturesUnordered, StreamExt};
use meld_events::EventEnvelope;
use std::fmt::Display;
use std::future::Future;
use std::pin::Pin;

/// Summary for one completed task runtime execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRunSummary {
    /// Authored or compiled task identifier within the execution domain.
    pub task_id: String,
    /// Concrete task run identifier within the execution runtime.
    pub task_run_id: String,
    /// Number of capability instances completed by the run.
    pub completed_instances: usize,
    /// Number of capability invocation attempts recorded by the run.
    pub invocation_count: usize,
    /// Number of artifacts persisted in the task artifact repo.
    pub artifact_count: usize,
}

/// Execute a complete Task through explicitly supplied invocation and expansion owners.
#[allow(clippy::too_many_arguments)]
pub async fn execute_task_to_completion<A, E, InvokeState, InvokeCapability, CompileExpansion>(
    api: &A,
    executor: &mut TaskExecutor,
    catalog: &CapabilityCatalog,
    invoke_state: &InvokeState,
    invoke_capability: InvokeCapability,
    compile_expansion: CompileExpansion,
    event_context: Option<&ExecutionEventContext>,
) -> Result<TaskRunSummary, E>
where
    A: EventPublicationPort<Error = E, EventEnvelope = EventEnvelope> + ?Sized,
    E: From<ExecutionInvariantError> + Display,
    InvokeCapability: Copy
        + for<'a> Fn(
            &'a A,
            &'a InvokeState,
            &'a BoundCapabilityInstance,
            &'a CapabilityInvocationPayload,
            Option<&'a ExecutionEventContext>,
        ) -> Pin<
            Box<dyn Future<Output = Result<CapabilityInvocationResult, E>> + Send + 'a>,
        >,
    CompileExpansion: Copy
        + Fn(
            &A,
            &CompiledTaskRecord,
            &TaskExpansionRequest,
            &CapabilityCatalog,
        ) -> Result<CompiledTaskDelta, E>,
{
    let mut emitted_task_event_count = 0usize;
    loop {
        if executor.is_complete() {
            return Ok(TaskRunSummary {
                task_id: executor.compiled_task().task_id.clone(),
                task_run_id: executor.init_payload().task_run_context.task_run_id.clone(),
                completed_instances: executor.completed_count(),
                invocation_count: executor.invocation_records().len(),
                artifact_count: executor.artifact_repo().record().artifacts.len(),
            });
        }

        let ready = executor.release_ready_invocations(CapabilityExecutionContext::default())?;
        emit_new_task_events(api, event_context, executor, &mut emitted_task_event_count);
        if ready.is_empty() {
            let reasons = crate::task::readiness::blocked_instance_diagnostics(
                executor.compiled_task(),
                executor.artifact_repo(),
                executor.completed_instances(),
                executor.in_flight_instances(),
            );
            return Err(E::from(ExecutionInvariantError::GenerationFailed(format!(
                "Task '{}' is blocked with no ready capability instances: {}",
                executor.compiled_task().task_id,
                if reasons.is_empty() {
                    "no blocked-instance diagnostics available".to_string()
                } else {
                    reasons.join("; ")
                }
            ))));
        }

        let mut futures = FuturesUnordered::new();
        for payload in ready {
            let compiled_task = executor.compiled_task().clone();
            let instance = compiled_task
                .capability_instances
                .iter()
                .find(|instance| instance.capability_instance_id == payload.capability_instance_id)
                .ok_or_else(|| {
                    ExecutionInvariantError::ConfigError(format!(
                        "Task '{}' is missing capability instance '{}'",
                        compiled_task.task_id, payload.capability_instance_id
                    ))
                })
                .map_err(E::from)?
                .clone();
            let invocation_id = payload.invocation_id.clone();
            futures.push(async move {
                let outcome =
                    invoke_capability(api, invoke_state, &instance, &payload, event_context).await;
                (
                    invocation_id,
                    payload.capability_instance_id.clone(),
                    outcome,
                )
            });
        }

        while let Some((invocation_id, capability_instance_id, outcome)) = futures.next().await {
            match outcome {
                Ok(result) => {
                    let mut expansion_requests = Vec::new();
                    for artifact in &result.emitted_artifacts {
                        if let Some(request) = parse_task_expansion_request_artifact(artifact)? {
                            expansion_requests.push((artifact.artifact_id.clone(), request));
                        }
                    }
                    executor.record_success(&invocation_id, result.emitted_artifacts)?;
                    for (source_artifact_id, expansion_request) in expansion_requests {
                        let delta = compile_expansion(
                            api,
                            executor.compiled_task(),
                            &expansion_request,
                            catalog,
                        )?;
                        let _ = executor.apply_task_expansion(
                            &expansion_request.expansion_id,
                            &expansion_request.expansion_kind,
                            &source_artifact_id,
                            delta,
                        )?;
                    }
                    emit_new_task_events(
                        api,
                        event_context,
                        executor,
                        &mut emitted_task_event_count,
                    );
                }
                Err(err) => {
                    executor.record_failure(
                        &invocation_id,
                        failure_artifact(
                            executor.compiled_task().task_id.clone(),
                            capability_instance_id,
                            invocation_id.clone(),
                            err.to_string(),
                        ),
                        err.to_string(),
                    )?;
                    emit_new_task_events(
                        api,
                        event_context,
                        executor,
                        &mut emitted_task_event_count,
                    );
                    return Err(err);
                }
            }
        }
    }
}

fn emit_new_task_events(
    api: &(impl EventPublicationPort<EventEnvelope = EventEnvelope> + ?Sized),
    event_context: Option<&ExecutionEventContext>,
    executor: &TaskExecutor,
    emitted_task_event_count: &mut usize,
) {
    let Some(ctx) = event_context else {
        *emitted_task_event_count = executor.events().len();
        return;
    };

    for event in executor.events().iter().skip(*emitted_task_event_count) {
        if let Some(envelope) = build_execution_task_envelope(&ctx.session_id, event) {
            let _ = api.publish_execution_envelope(ctx, envelope);
        }
    }

    *emitted_task_event_count = executor.events().len();
}

// Shared with the bounded package stepper so failure artifacts keep one
// identity shape across the completion and bounded execution paths.
pub(crate) fn failure_artifact(
    task_id: String,
    capability_instance_id: String,
    invocation_id: String,
    message: String,
) -> ArtifactRecord {
    ArtifactRecord {
        artifact_id: format!("{invocation_id}::failure"),
        artifact_type_id: "capability_failure".to_string(),
        schema_version: 1,
        content: serde_json::json!({
            "message": message,
        }),
        producer: crate::task::ArtifactProducerRef {
            task_id,
            capability_instance_id,
            invocation_id: Some(invocation_id),
            output_slot_id: Some("failure".to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{
        BoundCapabilityInstance, ExecutionClass, ExecutionContract, ScopeContract,
    };
    use crate::execution::ExecutionEventContext;
    use crate::task::{
        ArtifactProducerRef, TaskDependencyEdge, TaskDependencyKind, TaskInitializationPayload,
        TaskRunContext,
    };
    use futures::executor::block_on;
    use serde_json::json;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingApi {
        envelopes: Mutex<Vec<EventEnvelope>>,
    }

    impl EventPublicationPort for RecordingApi {
        type Error = ExecutionInvariantError;
        type EventEnvelope = EventEnvelope;

        fn publish_execution_envelope(
            &self,
            _event_context: &ExecutionEventContext,
            envelope: Self::EventEnvelope,
        ) -> Result<(), Self::Error> {
            self.envelopes.lock().unwrap().push(envelope);
            Ok(())
        }
    }

    fn instance(capability_instance_id: &str) -> BoundCapabilityInstance {
        BoundCapabilityInstance {
            capability_instance_id: capability_instance_id.to_string(),
            capability_type_id: "test_capability".to_string(),
            capability_version: 1,
            scope_ref: "node_a".to_string(),
            scope_kind: "node".to_string(),
            binding_values: vec![],
            input_wiring: vec![],
        }
    }

    fn compiled_task() -> CompiledTaskRecord {
        CompiledTaskRecord {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: vec![],
            capability_instances: vec![instance("capinst_prepare")],
            dependency_edges: vec![],
        }
    }

    fn init_payload() -> TaskInitializationPayload {
        TaskInitializationPayload {
            task_id: "task_docs_writer".to_string(),
            compiled_task_ref: "compiled_task_docs_writer".to_string(),
            init_artifacts: vec![],
            task_run_context: TaskRunContext {
                task_run_id: "taskrun_1".to_string(),
                session_id: Some("session_1".to_string()),
                trigger: "workflow.execute".to_string(),
            },
        }
    }

    fn emitted_artifact(payload: &CapabilityInvocationPayload) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: format!("{}::artifact", payload.invocation_id),
            artifact_type_id: "provider_execute_result".to_string(),
            schema_version: 1,
            content: json!({ "ok": true }),
            producer: ArtifactProducerRef {
                task_id: "task_docs_writer".to_string(),
                capability_instance_id: payload.capability_instance_id.clone(),
                invocation_id: Some(payload.invocation_id.clone()),
                output_slot_id: Some("provider_result".to_string()),
            },
        }
    }

    fn invoke_success<'a>(
        _api: &'a RecordingApi,
        _state: &'a (),
        _instance: &'a BoundCapabilityInstance,
        payload: &'a CapabilityInvocationPayload,
        _event_context: Option<&'a ExecutionEventContext>,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<CapabilityInvocationResult, ExecutionInvariantError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            Ok(CapabilityInvocationResult {
                emitted_artifacts: vec![emitted_artifact(payload)],
            })
        })
    }

    fn invoke_failure<'a>(
        _api: &'a RecordingApi,
        _state: &'a (),
        _instance: &'a BoundCapabilityInstance,
        _payload: &'a CapabilityInvocationPayload,
        _event_context: Option<&'a ExecutionEventContext>,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<CapabilityInvocationResult, ExecutionInvariantError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async {
            Err(ExecutionInvariantError::GenerationFailed(
                "capability failed".to_string(),
            ))
        })
    }

    fn invoke_expansion<'a>(
        _api: &'a RecordingApi,
        _state: &'a (),
        _instance: &'a BoundCapabilityInstance,
        payload: &'a CapabilityInvocationPayload,
        _event_context: Option<&'a ExecutionEventContext>,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<CapabilityInvocationResult, ExecutionInvariantError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            Ok(CapabilityInvocationResult {
                emitted_artifacts: vec![ArtifactRecord {
                    artifact_id: format!("{}::expansion", payload.invocation_id),
                    artifact_type_id: crate::task::TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID
                        .to_string(),
                    schema_version: crate::task::TASK_EXPANSION_SCHEMA_VERSION,
                    content: json!({
                        "expansion_id": "expansion_1",
                        "expansion_kind": "discover_children",
                        "content": {}
                    }),
                    producer: ArtifactProducerRef {
                        task_id: "task_docs_writer".to_string(),
                        capability_instance_id: payload.capability_instance_id.clone(),
                        invocation_id: Some(payload.invocation_id.clone()),
                        output_slot_id: Some("expansion".to_string()),
                    },
                }],
            })
        })
    }

    fn compile_no_expansion(
        _api: &RecordingApi,
        _compiled_task: &CompiledTaskRecord,
        _request: &TaskExpansionRequest,
        _catalog: &CapabilityCatalog,
    ) -> Result<CompiledTaskDelta, ExecutionInvariantError> {
        Ok(CompiledTaskDelta::default())
    }

    fn compile_expansion_failure(
        _api: &RecordingApi,
        _compiled_task: &CompiledTaskRecord,
        _request: &TaskExpansionRequest,
        _catalog: &CapabilityCatalog,
    ) -> Result<CompiledTaskDelta, ExecutionInvariantError> {
        Err(ExecutionInvariantError::ConfigError(
            "expansion compile failed".to_string(),
        ))
    }

    fn event_types(api: &RecordingApi) -> Vec<String> {
        api.envelopes
            .lock()
            .unwrap()
            .iter()
            .map(|envelope| envelope.event_type.clone())
            .collect()
    }

    #[test]
    fn runtime_publishes_task_events_with_execution_context() {
        let api = RecordingApi::default();
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let context = ExecutionEventContext {
            effect_authority: None,
            session_id: "session_1".to_string(),
        };

        let summary = block_on(execute_task_to_completion(
            &api,
            &mut executor,
            &CapabilityCatalog::new(),
            &(),
            invoke_success,
            compile_no_expansion,
            Some(&context),
        ))
        .unwrap();

        assert_eq!(summary.completed_instances, 1);
        assert_eq!(
            event_types(&api),
            vec![
                "execution.task.requested",
                "execution.task.started",
                "execution.task.progressed",
                "execution.task.artifact_emitted",
                "execution.task.succeeded",
            ]
        );
        let envelopes = api.envelopes.lock().unwrap();
        assert_eq!(envelopes[0].session, "session_1");
        assert!(envelopes
            .iter()
            .any(|envelope| envelope.stream_id == "taskrun_1"));
    }

    #[test]
    fn runtime_publishes_failure_event_before_returning_error() {
        let api = RecordingApi::default();
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();
        let context = ExecutionEventContext {
            effect_authority: None,
            session_id: "session_1".to_string(),
        };

        let error = block_on(execute_task_to_completion(
            &api,
            &mut executor,
            &CapabilityCatalog::new(),
            &(),
            invoke_failure,
            compile_no_expansion,
            Some(&context),
        ))
        .unwrap_err();

        assert!(error.to_string().contains("capability failed"));
        assert!(event_types(&api)
            .iter()
            .any(|event_type| event_type == "execution.task.failed"));
    }

    #[test]
    fn runtime_exits_with_blocked_event_when_no_invocation_is_ready() {
        let api = RecordingApi::default();
        let blocked_task = CompiledTaskRecord {
            dependency_edges: vec![TaskDependencyEdge {
                from_capability_instance_id: "capinst_missing".to_string(),
                to_capability_instance_id: "capinst_prepare".to_string(),
                kind: TaskDependencyKind::Artifact,
                reason: "missing dependency".to_string(),
            }],
            ..compiled_task()
        };
        let mut executor =
            TaskExecutor::new(blocked_task, init_payload(), "repo_docs_writer").unwrap();
        let context = ExecutionEventContext {
            effect_authority: None,
            session_id: "session_1".to_string(),
        };

        let error = block_on(execute_task_to_completion(
            &api,
            &mut executor,
            &CapabilityCatalog::new(),
            &(),
            invoke_success,
            compile_no_expansion,
            Some(&context),
        ))
        .unwrap_err();

        assert!(error.to_string().contains("blocked"));
        assert!(event_types(&api)
            .iter()
            .any(|event_type| event_type == "execution.task.blocked"));
    }

    #[test]
    fn runtime_surfaces_expansion_compile_failure() {
        let api = RecordingApi::default();
        let mut executor =
            TaskExecutor::new(compiled_task(), init_payload(), "repo_docs_writer").unwrap();

        let error = block_on(execute_task_to_completion(
            &api,
            &mut executor,
            &CapabilityCatalog::new(),
            &(),
            invoke_expansion,
            compile_expansion_failure,
            None,
        ))
        .unwrap_err();

        assert!(error.to_string().contains("expansion compile failed"));
    }

    #[test]
    fn runtime_test_contract_fixture_remains_minimal() {
        let contract = crate::capability::CapabilityTypeContract {
            capability_type_id: "test_capability".to_string(),
            capability_version: 1,
            owning_domain: "test".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "node".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![],
            input_contract: vec![],
            output_contract: vec![],
            effect_contract: vec![],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Inline,
                completion_semantics: "artifact".to_string(),
                retry_class: "none".to_string(),
                cancellation_supported: false,
            },
        };

        contract.validate().unwrap();
    }
}
