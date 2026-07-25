//! World initialization command surface over stages 2 through 4.
//!
//! Owner: root. Machine initialization hydrates; world initialization
//! creates the first durable meaning, once per world: theory installation,
//! identity genesis, and epistemic seeding. Every stage is idempotent by
//! content identity — reinstalling the same content is a no-op, changed
//! content is a new revision. Registrations install through domain
//! commands, epistemic facts append through the canonical event
//! capability, and nothing writes a domain store directly from root or
//! CLI code. Activation is stage 5, owned by the runtime, and never
//! creates semantic state.
//!
//! This module carries no implementation. The initialization command
//! surface workstream binds the stages to domain command paths.

use serde::{Deserialize, Serialize};

/// One world-initialization stage of the staged pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldInitStage {
    /// Stage 2: install belief families, curation rules, evidence
    /// mappings, methods, and task packages into their domain registries.
    InstallTheory,
    /// Stage 3: create seed agent identity, perspective, and
    /// subscriptions with recorded provenance.
    GenesisIdentities,
    /// Stage 4: append the epistemic genesis facts for the selected
    /// scope through the canonical append capability.
    SeedEpistemicFacts,
}

/// Idempotent disposition of one stage run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageDisposition {
    /// New content or identity was durably recorded.
    Applied,
    /// Everything the stage covers already existed unchanged.
    Unchanged,
}

/// Request to run world initialization stages in pipeline order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldInitRequest {
    /// Stages to run. Order is fixed by the pipeline; the request only
    /// selects the subset, allowing isolate boots to scope stages to one
    /// registration subset.
    pub stages: Vec<WorldInitStage>,
}

/// Report from one stage run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldInitStageReport {
    /// Stage that ran.
    pub stage: WorldInitStage,
    /// Whether the stage changed anything.
    pub disposition: StageDisposition,
    /// Identities of records the stage created or found, for provenance.
    pub record_ids: Vec<String>,
}

/// Report from one world-initialization run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldInitReport {
    /// Per-stage reports in pipeline order.
    pub stage_reports: Vec<WorldInitStageReport>,
}
