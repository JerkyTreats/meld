//! Durable task contracts and records.

use crate::capability::{BoundCapabilityInstance, InputSlotSpec};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Structured init slot published by one task definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskInitSlotSpec {
    /// Task initialization slot that supplies this artifact.
    pub init_slot_id: String,
    /// Artifact type identifier used for contract validation and routing.
    pub artifact_type_id: String,
    /// Schema version for the serialized contract or artifact shape.
    pub schema_version: u32,
    /// True when callers must provide this contract element.
    pub required: bool,
}

/// Authored task definition consumed by the task compiler.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskDefinition {
    /// Authored or compiled task identifier within the execution domain.
    pub task_id: String,
    /// Authored task definition version.
    pub task_version: u32,
    /// Init slots callers may supply at run creation.
    pub init_slots: Vec<TaskInitSlotSpec>,
    /// Bound capability instances that make up the task graph.
    pub capability_instances: Vec<BoundCapabilityInstance>,
}

/// Dependency edge kinds derived during compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskDependencyKind {
    /// Artifact value supplied through task artifact handoff.
    Artifact,
    /// Ordering imposed by side-effect contracts.
    Effect,
}

/// Deterministic task dependency edge between two capability instances.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskDependencyEdge {
    /// From capability instance identifier carried across the execution boundary.
    pub from_capability_instance_id: String,
    /// To capability instance identifier carried across the execution boundary.
    pub to_capability_instance_id: String,
    /// Contract kind used by the owning runtime.
    pub kind: TaskDependencyKind,
    /// Human-readable reason this dependency was derived.
    pub reason: String,
}

/// Durable compiled task graph record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompiledTaskRecord {
    /// Authored or compiled task identifier within the execution domain.
    pub task_id: String,
    /// Authored task definition version compiled into this record.
    pub task_version: u32,
    /// Init slot contracts accepted by this compiled task.
    pub init_slots: Vec<TaskInitSlotSpec>,
    /// Bound capability instances in the compiled task graph.
    pub capability_instances: Vec<BoundCapabilityInstance>,
    /// Dependency edges derived by the compiler.
    pub dependency_edges: Vec<TaskDependencyEdge>,
}

/// Producer lineage for one persisted artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactProducerRef {
    /// Authored or compiled task identifier within the execution domain.
    pub task_id: String,
    /// Deterministic capability instance identifier within a compiled task.
    pub capability_instance_id: String,
    /// Capability invocation attempt identifier within the task run.
    pub invocation_id: Option<String>,
    /// Output slot identifier on the producing capability instance.
    pub output_slot_id: Option<String>,
}

/// One durable artifact entry in the task repo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRecord {
    /// Stable artifact identifier within the owning artifact repository.
    pub artifact_id: String,
    /// Artifact type identifier used for contract validation and routing.
    pub artifact_type_id: String,
    /// Schema version for the serialized contract or artifact shape.
    pub schema_version: u32,
    /// Structured artifact payload persisted for this artifact.
    pub content: Value,
    /// Producer lineage for this artifact record.
    pub producer: ArtifactProducerRef,
}

/// Link relation between artifact records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactLinkRelation {
    /// Source artifact was consumed by a capability input slot.
    ConsumedBySlot,
    /// Target artifact supersedes the source artifact.
    Supersedes,
}

/// Durable relation between two artifact records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactLinkRecord {
    /// From artifact identifier carried across the execution boundary.
    pub from_artifact_id: String,
    /// To artifact identifier carried across the execution boundary.
    pub to_artifact_id: String,
    /// Lineage relation between source and target artifacts.
    pub relation: ArtifactLinkRelation,
    /// Optional detail describing the relation source.
    pub detail: String,
}

/// Durable task-scoped artifact store record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArtifactRepoRecord {
    /// Stable artifact repository identifier for this task run.
    pub repo_id: String,
    /// Artifact records persisted by the task artifact repository.
    pub artifacts: Vec<ArtifactRecord>,
    /// Lineage and supersession links between artifacts.
    pub artifact_links: Vec<ArtifactLinkRecord>,
}

/// Durable record for one capability invocation attempt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityInvocationRecord {
    /// Capability invocation attempt identifier within the task run.
    pub invocation_id: String,
    /// Deterministic capability instance identifier within a compiled task.
    pub capability_instance_id: String,
    /// Artifact records supplied to the invocation.
    pub supplied_inputs: Vec<ArtifactRecord>,
    /// Artifact identifiers emitted by the invocation.
    pub emitted_artifacts: Vec<String>,
    /// Failure artifact captured for the invocation when one exists.
    pub failure_summary: Option<ArtifactRecord>,
    /// One-based attempt index for this capability invocation.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{ArtifactSchemaVersionRange, InputCardinality};
    use serde_json::json;

    fn artifact(artifact_type_id: &str, schema_version: u32) -> ArtifactRecord {
        ArtifactRecord {
            artifact_id: "artifact-1".to_string(),
            artifact_type_id: artifact_type_id.to_string(),
            schema_version,
            content: json!({ "patch": "content" }),
            producer: ArtifactProducerRef {
                task_id: "task-docs".to_string(),
                capability_instance_id: "write".to_string(),
                invocation_id: Some("invocation-1".to_string()),
                output_slot_id: Some("patch".to_string()),
            },
        }
    }

    fn input_slot() -> InputSlotSpec {
        InputSlotSpec {
            slot_id: "patch".to_string(),
            accepted_artifact_type_ids: vec!["docs_patch".to_string()],
            schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
            required: true,
            cardinality: InputCardinality::One,
        }
    }

    #[test]
    fn artifact_matches_input_slot_requires_type_and_schema() {
        let slot = input_slot();

        assert!(artifact_matches_input_slot(
            &artifact("docs_patch", 1),
            &slot
        ));
        assert!(!artifact_matches_input_slot(
            &artifact("other_patch", 1),
            &slot
        ));
        assert!(!artifact_matches_input_slot(
            &artifact("docs_patch", 2),
            &slot
        ));
    }
}
