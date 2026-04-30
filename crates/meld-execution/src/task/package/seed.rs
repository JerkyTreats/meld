//! Task package seed authoring contracts.

use serde::{Deserialize, Serialize};

/// Declarative init artifact requirements for one package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitialSeedSpec {
    pub artifacts: Vec<SeedArtifactSpec>,
}

/// Declarative seed artifact contract for one init slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeedArtifactSpec {
    pub init_slot_id: String,
    pub artifact_type_id: String,
    pub schema_version: u32,
    pub source: SeedSourceSpec,
}

/// Source of one seeded init artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeedSourceSpec {
    ForcePosture,
    TargetNodeRef,
    ExpansionTemplate { template_id: String },
}
