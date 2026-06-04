//! Task-owned expansion contracts.

use crate::capability::BoundCapabilityInstance;
use crate::task::contracts::{ArtifactRecord, TaskDependencyEdge, TaskInitSlotSpec};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current schema version for task expansion artifacts.
pub const TASK_EXPANSION_SCHEMA_VERSION: u32 = 1;
/// Artifact type identifier for task expansion templates.
pub const TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID: &str = "task_expansion_template";
/// Artifact type identifier for task expansion requests.
pub const TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID: &str = "task_expansion_request";

/// Structured task expansion template passed into discovery capabilities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskExpansionTemplate {
    /// Expansion kind understood by a registered compiler.
    pub expansion_kind: String,
    /// Template payload consumed by discovery capabilities.
    pub content: Value,
}

/// Structured task expansion request emitted by one capability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskExpansionRequest {
    /// Expansion identifier carried across the execution boundary.
    pub expansion_id: String,
    /// Expansion kind used to select a compiler.
    pub expansion_kind: String,
    /// Expansion payload emitted by a capability.
    pub content: Value,
}

/// Persisted record for one applied task expansion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskExpansionRecord {
    /// Expansion identifier carried across the execution boundary.
    pub expansion_id: String,
    /// Expansion kind that was compiled and applied.
    pub expansion_kind: String,
    /// Source artifact identifier for this lineage link.
    pub source_artifact_id: String,
}

/// Append-only task delta produced by expansion compilation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CompiledTaskDelta {
    /// Init slots appended by expansion compilation.
    pub init_slots: Vec<TaskInitSlotSpec>,
    /// Init artifacts emitted by expansion compilation.
    pub init_artifacts: Vec<ArtifactRecord>,
    /// Capability instances appended by expansion compilation.
    pub capability_instances: Vec<BoundCapabilityInstance>,
    /// Dependency edges appended by expansion compilation.
    pub dependency_edges: Vec<TaskDependencyEdge>,
}
