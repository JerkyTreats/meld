//! Task-owned expansion contracts.

use crate::capability::BoundCapabilityInstance;
use crate::task::contracts::{ArtifactRecord, TaskDependencyEdge, TaskInitSlotSpec};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Constant used by the task expansion schema version execution contract.
pub const TASK_EXPANSION_SCHEMA_VERSION: u32 = 1;
/// Constant used by the task expansion template artifact type identifier execution contract.
pub const TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID: &str = "task_expansion_template";
/// Constant used by the task expansion request artifact type identifier execution contract.
pub const TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID: &str = "task_expansion_request";

/// Structured task expansion template passed into discovery capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskExpansionTemplate {
    /// Expansion kind owned by this execution contract.
    pub expansion_kind: String,
    /// Structured artifact content owned by the producing capability.
    pub content: Value,
}

/// Structured task expansion request emitted by one capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskExpansionRequest {
    /// Expansion identifier carried across the execution boundary.
    pub expansion_id: String,
    /// Expansion kind owned by this execution contract.
    pub expansion_kind: String,
    /// Structured artifact content owned by the producing capability.
    pub content: Value,
}

/// Persisted record for one applied task expansion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskExpansionRecord {
    /// Expansion identifier carried across the execution boundary.
    pub expansion_id: String,
    /// Expansion kind owned by this execution contract.
    pub expansion_kind: String,
    /// Source artifact identifier for this lineage link.
    pub source_artifact_id: String,
}

/// Append-only task delta produced by expansion compilation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CompiledTaskDelta {
    /// Init slots owned by this execution contract.
    pub init_slots: Vec<TaskInitSlotSpec>,
    /// Init artifacts owned by this execution contract.
    pub init_artifacts: Vec<ArtifactRecord>,
    /// Capability instances owned by this execution contract.
    pub capability_instances: Vec<BoundCapabilityInstance>,
    /// Dependency edges owned by this execution contract.
    pub dependency_edges: Vec<TaskDependencyEdge>,
}
