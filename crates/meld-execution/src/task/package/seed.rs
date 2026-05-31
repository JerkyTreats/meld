//! Task package seed authoring contracts.

use serde::{Deserialize, Serialize};

/// Declarative init artifact requirements for one package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitialSeedSpec {
    /// Artifact records persisted by the task artifact repository.
    pub artifacts: Vec<SeedArtifactSpec>,
}

/// Declarative seed artifact contract for one init slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedArtifactSpec {
    /// Task initialization slot that supplies this artifact.
    pub init_slot_id: String,
    /// Artifact type identifier used for contract validation and routing.
    pub artifact_type_id: String,
    /// Schema version for the serialized contract or artifact shape.
    pub schema_version: u32,
    /// Source owned by this execution contract.
    pub source: SeedSourceSpec,
}

/// Source of one seeded init artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeedSourceSpec {
    /// Force posture variant for this execution contract.
    ForcePosture,
    /// Target node reference variant for this execution contract.
    TargetNodeRef,
    /// Seed value comes from an expansion template.
    ExpansionTemplate {
        /// Expansion template identifier used as the seed source.
        template_id: String,
    },
}
