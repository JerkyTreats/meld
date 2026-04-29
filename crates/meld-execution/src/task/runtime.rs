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
            let invoke_capability = invoke_capability;
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
