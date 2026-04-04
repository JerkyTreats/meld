//! Task package trigger contracts and prepared run outputs.

use crate::provider::ProviderExecutionBinding;
use crate::task::{CompiledTaskRecord, TaskInitializationPayload};
use crate::types::NodeID;
use std::path::PathBuf;

/// Compatibility-facing trigger request for one workflow-backed task package.
#[derive(Debug, Clone)]
pub struct WorkflowPackageTriggerRequest {
    pub package_id: String,
    pub workflow_id: String,
    pub node_id: Option<NodeID>,
    pub path: Option<PathBuf>,
    pub agent_id: String,
    pub provider: ProviderExecutionBinding,
    pub frame_type: String,
    pub force: bool,
    pub session_id: Option<String>,
}

/// Prepared compiled task and run payload for one package trigger.
#[derive(Debug, Clone)]
pub struct PreparedTaskRun {
    pub compiled_task: CompiledTaskRecord,
    pub init_payload: TaskInitializationPayload,
    pub target_node_id: NodeID,
}
