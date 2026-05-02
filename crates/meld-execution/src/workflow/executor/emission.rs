use meld_events::EventEnvelope;
use serde_json::json;

use crate::execution::{EventPublicationPort, ExecutionEventContext, ExecutionProgressPort};
use crate::generation::{
    FrameMetadataValidationProgressEventData, NodeId, PreparedPromptLineage,
    PreviousMetadataSnapshotView,
};
use crate::workflow::{
    workflow_turn_completed_envelope, workflow_turn_failed_envelope,
    workflow_turn_started_envelope, ExecutionWorkflowTurnEventData, WorkflowExecutionRequest,
    WorkflowExecutionSummary, WorkflowForceResetProgressEventData, WorkflowTargetProgressEventData,
    WorkflowTurnProgressEventData,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn metadata_event_payload(
    request: &WorkflowExecutionRequest,
    target_path: &str,
    turn_frame_type: &str,
    prepared_lineage: &PreparedPromptLineage,
    previous_metadata: &PreviousMetadataSnapshotView,
    workflow_id: &str,
    thread_id: &str,
    turn_id: &str,
    turn_seq: u32,
    attempt: usize,
    error: Option<String>,
) -> FrameMetadataValidationProgressEventData {
    FrameMetadataValidationProgressEventData {
        node_id: hex::encode(request.node_id),
        path: target_path.to_string(),
        agent_id: request.agent_id.clone(),
        provider_name: request.provider.provider_name.clone(),
        frame_type: turn_frame_type.to_string(),
        prompt_digest: prepared_lineage.metadata_input.prompt_digest.clone(),
        context_digest: prepared_lineage.metadata_input.context_digest.clone(),
        prompt_link_id: prepared_lineage.metadata_input.prompt_link_id.clone(),
        previous_frame_id: previous_metadata.frame_id.clone(),
        previous_prompt_digest: previous_metadata.prompt_digest.clone(),
        previous_context_digest: previous_metadata.context_digest.clone(),
        previous_prompt_link_id: previous_metadata.prompt_link_id.clone(),
        workflow_id: Some(workflow_id.to_string()),
        thread_id: Some(thread_id.to_string()),
        turn_id: Some(turn_id.to_string()),
        turn_seq: Some(turn_seq),
        attempt: Some(attempt),
        plan_id: request.plan_id.clone(),
        level_index: request.level_index,
        error,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn workflow_turn_payload(
    request: &WorkflowExecutionRequest,
    workflow_id: String,
    thread_id: String,
    turn_id: String,
    turn_seq: u32,
    target_path: String,
    frame_type: String,
    attempt: usize,
    final_frame_id: Option<String>,
    error: Option<String>,
) -> WorkflowTurnProgressEventData {
    WorkflowTurnProgressEventData {
        workflow_id,
        thread_id,
        turn_id,
        turn_seq,
        node_id: hex::encode(request.node_id),
        path: target_path,
        agent_id: request.agent_id.clone(),
        provider_name: request.provider.provider_name.clone(),
        frame_type,
        attempt,
        plan_id: request.plan_id.clone(),
        level_index: request.level_index,
        final_frame_id,
        error,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn completed_target_summary<E>(
    api: &(impl ExecutionProgressPort<Error = E> + ?Sized),
    event_context: Option<&ExecutionEventContext>,
    workflow_id: String,
    thread_id: String,
    request: &WorkflowExecutionRequest,
    target_path: String,
    final_frame_id: Option<NodeId>,
    turns_completed: usize,
    reused_existing_head: bool,
) -> WorkflowExecutionSummary {
    emit_workflow_target_event(
        api,
        event_context,
        "workflow_target_completed",
        WorkflowTargetProgressEventData {
            workflow_id: workflow_id.clone(),
            thread_id: thread_id.clone(),
            node_id: hex::encode(request.node_id),
            path: target_path,
            agent_id: request.agent_id.clone(),
            provider_name: request.provider.provider_name.clone(),
            frame_type: request.frame_type.clone(),
            plan_id: request.plan_id.clone(),
            level_index: request.level_index,
            final_frame_id: final_frame_id.map(hex::encode),
            turns_completed: Some(turns_completed),
            reused_existing_head: Some(reused_existing_head),
        },
    );

    WorkflowExecutionSummary {
        workflow_id,
        thread_id,
        turns_completed,
        final_frame_id,
    }
}

pub(super) fn emit_workflow_target_event<E>(
    api: &(impl ExecutionProgressPort<Error = E> + ?Sized),
    event_context: Option<&ExecutionEventContext>,
    event_type: &str,
    payload: WorkflowTargetProgressEventData,
) {
    if let Some(ctx) = event_context {
        let _ = api.emit_progress_event(ctx, event_type, json!(payload));
    }
}

pub(super) fn emit_workflow_turn_event<E>(
    api: &(impl EventPublicationPort<Error = E, EventEnvelope = EventEnvelope> + ?Sized),
    event_context: Option<&ExecutionEventContext>,
    event_type: &str,
    payload: WorkflowTurnProgressEventData,
) {
    if let Some(ctx) = event_context {
        let payload = ExecutionWorkflowTurnEventData {
            workflow_id: payload.workflow_id,
            thread_id: payload.thread_id,
            turn_id: payload.turn_id,
            turn_seq: payload.turn_seq,
            node_id: payload.node_id,
            path: payload.path,
            agent_id: payload.agent_id,
            provider_name: payload.provider_name,
            frame_type: payload.frame_type,
            attempt: payload.attempt,
            plan_id: payload.plan_id,
            level_index: payload.level_index,
            final_frame_id: payload.final_frame_id,
            error: payload.error,
        };
        let envelope = match event_type {
            "execution.workflow.turn_started" => {
                workflow_turn_started_envelope(&ctx.session_id, payload)
            }
            "execution.workflow.turn_completed" => {
                workflow_turn_completed_envelope(&ctx.session_id, payload)
            }
            "execution.workflow.turn_failed" => {
                workflow_turn_failed_envelope(&ctx.session_id, payload)
            }
            _ => return,
        };
        let _ = api.publish_execution_envelope(ctx, envelope);
    }
}

pub(super) fn emit_workflow_force_reset_event<E>(
    api: &(impl ExecutionProgressPort<Error = E> + ?Sized),
    event_context: Option<&ExecutionEventContext>,
    event_type: &str,
    payload: WorkflowForceResetProgressEventData,
) {
    if let Some(ctx) = event_context {
        let _ = api.emit_progress_event(ctx, event_type, json!(payload));
    }
}

pub(super) fn emit_metadata_validation_event<E>(
    api: &(impl ExecutionProgressPort<Error = E> + ?Sized),
    event_context: Option<&ExecutionEventContext>,
    event_type: &str,
    payload: FrameMetadataValidationProgressEventData,
) {
    if let Some(ctx) = event_context {
        let _ = api.emit_progress_event(ctx, event_type, json!(payload));
    }
}
