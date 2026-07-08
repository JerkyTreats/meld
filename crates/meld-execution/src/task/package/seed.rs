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
    /// Source used to materialize this init artifact.
    pub source: SeedSourceSpec,
}

/// Source of one seeded init artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeedSourceSpec {
    /// Seed value is derived from the trigger force setting.
    ForcePosture,
    /// Seed value is derived from the selected target node.
    TargetNodeRef,
    /// Seed value comes from an expansion template.
    ExpansionTemplate {
        /// Expansion template identifier used as the seed source.
        template_id: String,
    },
    /// Hydrated belief context bundle for the trigger target subject.
    ///
    /// Only materialized when the workflow's `belief_context` flag is on;
    /// otherwise the seed is skipped so flag-off runs stay byte-identical.
    GoalBeliefHydration,
}
