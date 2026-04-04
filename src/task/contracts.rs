//! Durable task contracts and records.

use crate::capability::{BoundCapabilityInstance, InputSlotSpec};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Structured init slot published by one task definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskInitSlotSpec {
    pub init_slot_id: String,
    pub artifact_type_id: String,
    pub schema_version: u32,
    pub required: bool,
}

/// Authored task definition consumed by the task compiler.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub task_id: String,
    pub task_version: u32,
    pub init_slots: Vec<TaskInitSlotSpec>,
    pub capability_instances: Vec<BoundCapabilityInstance>,
}

/// Dependency edge kinds derived during compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskDependencyKind {
    Artifact,
    Effect,
}

/// Deterministic task dependency edge between two capability instances.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskDependencyEdge {
    pub from_capability_instance_id: String,
    pub to_capability_instance_id: String,
    pub kind: TaskDependencyKind,
    pub reason: String,
}

/// Durable compiled task graph record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledTaskRecord {
    pub task_id: String,
    pub task_version: u32,
    pub init_slots: Vec<TaskInitSlotSpec>,
    pub capability_instances: Vec<BoundCapabilityInstance>,
    pub dependency_edges: Vec<TaskDependencyEdge>,
}

/// Producer lineage for one persisted artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactProducerRef {
    pub task_id: String,
    pub capability_instance_id: String,
    pub invocation_id: Option<String>,
    pub output_slot_id: Option<String>,
}

/// One durable artifact entry in the task repo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub artifact_id: String,
    pub artifact_type_id: String,
    pub schema_version: u32,
    pub content: Value,
    pub producer: ArtifactProducerRef,
}

/// Link relation between artifact records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactLinkRelation {
    ConsumedBySlot,
    Supersedes,
}

/// Durable relation between two artifact records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactLinkRecord {
    pub from_artifact_id: String,
    pub to_artifact_id: String,
    pub relation: ArtifactLinkRelation,
    pub detail: String,
}

/// Durable task-scoped artifact store record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRepoRecord {
    pub repo_id: String,
    pub artifacts: Vec<ArtifactRecord>,
    pub artifact_links: Vec<ArtifactLinkRecord>,
}

/// Durable record for one capability invocation attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityInvocationRecord {
    pub invocation_id: String,
    pub capability_instance_id: String,
    pub supplied_inputs: Vec<ArtifactRecord>,
    pub emitted_artifacts: Vec<String>,
    pub failure_summary: Option<ArtifactRecord>,
    pub attempt_index: u32,
}

/// Returns true when the artifact record satisfies the published input slot contract.
pub fn artifact_matches_input_slot(artifact: &ArtifactRecord, input_slot: &InputSlotSpec) -> bool {
    input_slot
        .accepted_artifact_type_ids
        .iter()
        .any(|artifact_type_id| artifact_type_id == &artifact.artifact_type_id)
        && input_slot.schema_versions.accepts(artifact.schema_version)
}
