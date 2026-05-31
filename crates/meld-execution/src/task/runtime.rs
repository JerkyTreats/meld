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
use crate::workflow::{
    workflow_turn_completed_envelope, workflow_turn_failed_envelope,
    workflow_turn_started_envelope, ExecutionWorkflowTurnEventData,
};
use futures::stream::{FuturesUnordered, StreamExt};
use meld_events::EventEnvelope;
use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;
use std::pin::Pin;

/// Summary for one completed task runtime execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRunSummary {
    pub task_id: String,
    pub task_run_id: String,
    pub completed_instances: usize,
    pub invocation_count: usize,
    pub artifact_count: usize,
}

/// Workflow compatibility telemetry carried into task execution.
#[derive(Debug, Clone)]
pub struct WorkflowTaskTelemetry {
    pub workflow_id: String,
    pub thread_id: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub plan_id: Option<String>,
    pub level_index: Option<usize>,
    pub turn_seq_by_id: HashMap<String, u32>,
}

/// Executes one task to completion using the registered capability invokers.
#[allow(clippy::too_many_arguments)]
pub async fn execute_task_to_completion<A, E, InvokeState, InvokeCapability, CompileExpansion>(
    api: &A,
    executor: &mut TaskExecutor,
    catalog: &CapabilityCatalog,
    invoke_state: &InvokeState,
    invoke_capability: InvokeCapability,
    compile_expansion: CompileExpansion,
    event_context: Option<&ExecutionEventContext>,
    workflow_telemetry: Option<&WorkflowTaskTelemetry>,
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
            return Err(E::from(ExecutionInvariantError::GenerationFailed(format!(
                "Task '{}' is blocked with no ready capability instances",
                executor.compiled_task().task_id
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
            if let (Some(ctx), Some(telemetry), Some((turn_id, stage))) = (
                event_context,
                workflow_telemetry,
                parse_turn_stage(&payload.capability_instance_id),
            ) {
                if stage == "prepare" {
                    emit_workflow_turn_event(
                        api,
                        ctx,
                        "execution.workflow.turn_started",
                        workflow_turn_event_data(telemetry, &payload, &turn_id, None, None),
                    );
                }
            }
            futures.push(async move {
                let outcome =
                    invoke_capability(api, invoke_state, &instance, &payload, event_context).await;
                (
                    invocation_id,
                    payload.capability_instance_id.clone(),
                    payload.clone(),
                    outcome,
                )
            });
        }

        while let Some((invocation_id, capability_instance_id, payload, outcome)) =
            futures.next().await
        {
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
                    if let (Some(ctx), Some(telemetry), Some((turn_id, stage))) = (
                        event_context,
                        workflow_telemetry,
                        parse_turn_stage(&capability_instance_id),
                    ) {
                        if stage == "finalize" {
                            emit_workflow_turn_event(
                                api,
                                ctx,
                                "execution.workflow.turn_completed",
                                workflow_turn_event_data(telemetry, &payload, &turn_id, None, None),
                            );
                        }
                    }
                    emit_new_task_events(
                        api,
                        event_context,
                        executor,
                        &mut emitted_task_event_count,
                    );
                }
                Err(err) => {
                    if let (Some(ctx), Some(telemetry), Some((turn_id, _stage))) = (
                        event_context,
                        workflow_telemetry,
                        parse_turn_stage(&capability_instance_id),
                    ) {
                        emit_workflow_turn_event(
                            api,
                            ctx,
                            "execution.workflow.turn_failed",
                            workflow_turn_event_data(
                                telemetry,
                                &payload,
                                &turn_id,
                                None,
                                Some(err.to_string()),
                            ),
                        );
                    }
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

fn emit_workflow_turn_event(
    api: &(impl EventPublicationPort<EventEnvelope = EventEnvelope> + ?Sized),
    event_context: &ExecutionEventContext,
    event_type: &str,
    payload: ExecutionWorkflowTurnEventData,
) {
    let envelope = match event_type {
        "execution.workflow.turn_started" => {
            workflow_turn_started_envelope(&event_context.session_id, payload)
        }
        "execution.workflow.turn_completed" => {
            workflow_turn_completed_envelope(&event_context.session_id, payload)
        }
        "execution.workflow.turn_failed" => {
            workflow_turn_failed_envelope(&event_context.session_id, payload)
        }
        _ => return,
    };

    let _ = api.publish_execution_envelope(event_context, envelope);
}

fn workflow_turn_event_data(
    telemetry: &WorkflowTaskTelemetry,
    payload: &CapabilityInvocationPayload,
    turn_id: &str,
    final_frame_id: Option<String>,
    error: Option<String>,
) -> ExecutionWorkflowTurnEventData {
    ExecutionWorkflowTurnEventData {
        workflow_id: telemetry.workflow_id.clone(),
        thread_id: telemetry.thread_id.clone(),
        turn_id: turn_id.to_string(),
        turn_seq: telemetry
            .turn_seq_by_id
            .get(turn_id)
            .copied()
            .unwrap_or_default(),
        node_id: payload_node_id(payload).unwrap_or_default(),
        path: payload_path(payload).unwrap_or_default(),
        agent_id: telemetry.agent_id.clone(),
        provider_name: telemetry.provider_name.clone(),
        frame_type: telemetry.frame_type.clone(),
        attempt: payload.execution_context.attempt as usize,
        plan_id: telemetry.plan_id.clone(),
        level_index: telemetry.level_index,
        final_frame_id,
        error,
    }
}

fn parse_turn_stage(capability_instance_id: &str) -> Option<(String, String)> {
    let parts = capability_instance_id.split("::").collect::<Vec<_>>();
    if parts.len() < 5 || parts[2] != "turn" {
        return None;
    }
    Some((parts[3].to_string(), parts[4].to_string()))
}

fn payload_node_id(payload: &crate::capability::CapabilityInvocationPayload) -> Option<String> {
    payload
        .supplied_inputs
        .iter()
        .find(|input| input.slot_id == "resolved_node_ref")
        .and_then(|input| match &input.value {
            crate::capability::SuppliedValueRef::Artifact(artifact) => artifact
                .content
                .get("node_id")
                .and_then(|value| value.as_str()),
            crate::capability::SuppliedValueRef::StructuredValue(value) => {
                value.get("node_id").and_then(|value| value.as_str())
            }
        })
        .map(ToString::to_string)
}

fn payload_path(payload: &crate::capability::CapabilityInvocationPayload) -> Option<String> {
    payload
        .supplied_inputs
        .iter()
        .find(|input| input.slot_id == "resolved_node_ref")
        .and_then(|input| match &input.value {
            crate::capability::SuppliedValueRef::Artifact(artifact) => artifact
                .content
                .get("path")
                .and_then(|value| value.as_str()),
            crate::capability::SuppliedValueRef::StructuredValue(value) => {
                value.get("path").and_then(|value| value.as_str())
            }
        })
        .map(ToString::to_string)
}

fn failure_artifact(
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
            None,
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
            None,
        ))
        .unwrap_err();

        assert!(error.to_string().contains("capability failed"));
        assert!(event_types(&api)
            .iter()
            .any(|event_type| event_type == "execution.task.failed"));
    }

    #[test]
    fn runtime_publishes_workflow_failed_event_for_task_stage_failure() {
        let api = RecordingApi::default();
        let mut task = compiled_task();
        task.capability_instances[0].capability_instance_id =
            "task::pkg::turn::turn-1::finalize".to_string();
        let mut executor = TaskExecutor::new(task, init_payload(), "repo_docs_writer").unwrap();
        let context = ExecutionEventContext {
            session_id: "session_1".to_string(),
        };
        let telemetry = WorkflowTaskTelemetry {
            workflow_id: "workflow_docs".to_string(),
            thread_id: "thread-1".to_string(),
            agent_id: "agent_docs".to_string(),
            provider_name: "provider".to_string(),
            frame_type: "summary".to_string(),
            plan_id: Some("plan-1".to_string()),
            level_index: Some(0),
            turn_seq_by_id: HashMap::from([("turn-1".to_string(), 1)]),
        };

        let error = block_on(execute_task_to_completion(
            &api,
            &mut executor,
            &CapabilityCatalog::new(),
            &(),
            invoke_failure,
            compile_no_expansion,
            Some(&context),
            Some(&telemetry),
        ))
        .unwrap_err();

        assert!(error.to_string().contains("capability failed"));
        assert!(event_types(&api)
            .iter()
            .any(|event_type| event_type == "execution.workflow.turn_failed"));
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
            None,
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
            None,
        ))
        .unwrap_err();

        assert!(error.to_string().contains("expansion compile failed"));
    }

    #[test]
    fn workflow_turn_telemetry_emits_turn_envelopes_for_task_stage_ids() {
        let api = RecordingApi::default();
        let mut task = compiled_task();
        task.capability_instances[0].capability_instance_id =
            "task::pkg::turn::turn-1::prepare".to_string();
        let mut executor = TaskExecutor::new(task, init_payload(), "repo_docs_writer").unwrap();
        let context = ExecutionEventContext {
            session_id: "session_1".to_string(),
        };
        let telemetry = WorkflowTaskTelemetry {
            workflow_id: "workflow_docs".to_string(),
            thread_id: "thread-1".to_string(),
            agent_id: "agent_docs".to_string(),
            provider_name: "provider".to_string(),
            frame_type: "summary".to_string(),
            plan_id: Some("plan-1".to_string()),
            level_index: Some(0),
            turn_seq_by_id: HashMap::from([("turn-1".to_string(), 1)]),
        };

        block_on(execute_task_to_completion(
            &api,
            &mut executor,
            &CapabilityCatalog::new(),
            &(),
            invoke_success,
            compile_no_expansion,
            Some(&context),
            Some(&telemetry),
        ))
        .unwrap();

        assert!(event_types(&api)
            .iter()
            .any(|event_type| event_type == "execution.workflow.turn_started"));
    }

    #[test]
    fn workflow_turn_telemetry_emits_completed_envelope_for_finalize_stage() {
        let api = RecordingApi::default();
        let mut task = compiled_task();
        task.capability_instances[0].capability_instance_id =
            "task::pkg::turn::turn-1::finalize".to_string();
        let mut executor = TaskExecutor::new(task, init_payload(), "repo_docs_writer").unwrap();
        let context = ExecutionEventContext {
            session_id: "session_1".to_string(),
        };
        let telemetry = WorkflowTaskTelemetry {
            workflow_id: "workflow_docs".to_string(),
            thread_id: "thread-1".to_string(),
            agent_id: "agent_docs".to_string(),
            provider_name: "provider".to_string(),
            frame_type: "summary".to_string(),
            plan_id: Some("plan-1".to_string()),
            level_index: Some(0),
            turn_seq_by_id: HashMap::from([("turn-1".to_string(), 1)]),
        };

        block_on(execute_task_to_completion(
            &api,
            &mut executor,
            &CapabilityCatalog::new(),
            &(),
            invoke_success,
            compile_no_expansion,
            Some(&context),
            Some(&telemetry),
        ))
        .unwrap();

        assert!(event_types(&api)
            .iter()
            .any(|event_type| event_type == "execution.workflow.turn_completed"));
    }

    #[test]
    fn workflow_turn_payload_extracts_node_and_path_from_resolved_node_ref() {
        let artifact_payload = CapabilityInvocationPayload {
            invocation_id: "invk_1".to_string(),
            capability_instance_id: "capinst_1".to_string(),
            supplied_inputs: vec![crate::capability::SuppliedInputValue {
                slot_id: "resolved_node_ref".to_string(),
                source: crate::capability::InputValueSource::ArtifactHandoff,
                value: crate::capability::SuppliedValueRef::Artifact(
                    crate::capability::ArtifactValueRef {
                        artifact_id: "artifact_node".to_string(),
                        artifact_type_id: "resolved_node_ref".to_string(),
                        schema_version: 1,
                        content: json!({
                            "node_id": "node-artifact",
                            "path": "artifact.md",
                        }),
                    },
                ),
            }],
            upstream_lineage: None,
            execution_context: CapabilityExecutionContext::default(),
        };
        let structured_payload = CapabilityInvocationPayload {
            supplied_inputs: vec![crate::capability::SuppliedInputValue {
                slot_id: "resolved_node_ref".to_string(),
                source: crate::capability::InputValueSource::InitPayload,
                value: crate::capability::SuppliedValueRef::StructuredValue(json!({
                    "node_id": "node-structured",
                    "path": "structured.md",
                })),
            }],
            ..artifact_payload.clone()
        };

        assert_eq!(
            payload_node_id(&artifact_payload),
            Some("node-artifact".to_string())
        );
        assert_eq!(
            payload_path(&artifact_payload),
            Some("artifact.md".to_string())
        );
        assert_eq!(
            payload_node_id(&structured_payload),
            Some("node-structured".to_string())
        );
        assert_eq!(
            payload_path(&structured_payload),
            Some("structured.md".to_string())
        );
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
