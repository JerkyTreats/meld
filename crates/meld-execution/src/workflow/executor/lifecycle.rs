use crate::error::ExecutionInvariantError;
use crate::generation::NodeId;
use crate::workflow::{
    WorkflowExecutionRequest, WorkflowStateStore, WorkflowThreadRecord, WorkflowThreadStatus,
};

pub(super) fn mark_thread_failed<E>(
    state_store: &WorkflowStateStore,
    workflow_id: String,
    thread_id: &str,
    request: &WorkflowExecutionRequest,
    next_turn_seq: u32,
    final_frame_id: Option<NodeId>,
    now_millis: fn() -> u64,
) -> Result<(), E>
where
    E: From<ExecutionInvariantError>,
{
    state_store.upsert_thread(&WorkflowThreadRecord {
        thread_id: thread_id.to_string(),
        workflow_id,
        node_id: hex::encode(request.node_id),
        frame_type: request.frame_type.clone(),
        status: WorkflowThreadStatus::Failed,
        next_turn_seq,
        updated_at_ms: now_millis(),
        final_frame_id: final_frame_id.map(hex::encode),
    })?;
    Ok(())
}
