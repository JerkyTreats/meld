use crate::capability::BoundCapabilityInstance;
use crate::task::contracts::{ArtifactRecord, TaskDependencyEdge, TaskInitSlotSpec};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID: &str = "task_expansion_request";
pub const TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID: &str = "task_expansion_template";
pub const TASK_EXPANSION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskExpansionTemplate {
    pub expansion_kind: String,
    pub content: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskExpansionRequest {
    pub expansion_id: String,
    pub expansion_kind: String,
    pub content: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskExpansionRecord {
    pub expansion_id: String,
    pub expansion_kind: String,
    pub source_artifact_id: String,
    pub applied_capability_instance_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CompiledTaskDelta {
    pub init_slots: Vec<TaskInitSlotSpec>,
    pub init_artifacts: Vec<ArtifactRecord>,
    pub capability_instances: Vec<BoundCapabilityInstance>,
    pub explicit_dependency_edges: Vec<TaskDependencyEdge>,
}
