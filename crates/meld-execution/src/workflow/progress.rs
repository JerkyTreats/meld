use crate::execution::contracts::ProviderExecutionBinding;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionRequest {
    pub node_id: [u8; 32],
    pub agent_id: String,
    pub provider: ProviderExecutionBinding,
    pub frame_type: String,
    pub force: bool,
    pub path: Option<String>,
    pub plan_id: Option<String>,
    pub level_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionSummary {
    pub workflow_id: String,
    pub thread_id: String,
    pub turns_completed: usize,
    pub final_frame_id: Option<[u8; 32]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTargetProgressEventData {
    pub workflow_id: String,
    pub thread_id: String,
    pub node_id: String,
    pub path: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub plan_id: Option<String>,
    pub level_index: Option<usize>,
    pub final_frame_id: Option<String>,
    pub turns_completed: Option<usize>,
    pub reused_existing_head: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTurnProgressEventData {
    pub workflow_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub turn_seq: u32,
    pub node_id: String,
    pub path: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub attempt: usize,
    pub plan_id: Option<String>,
    pub level_index: Option<usize>,
    pub final_frame_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowForceResetProgressEventData {
    pub workflow_id: String,
    pub thread_id: String,
    pub node_id: String,
    pub path: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub previous_frame_id: Option<String>,
    pub plan_id: Option<String>,
    pub level_index: Option<usize>,
}
