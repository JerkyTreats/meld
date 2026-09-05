//! Product initialization: compilation, Agent genesis, inert preparation,
//! and epistemic seeding behind the retained world-init adapter.
//!
//! Owner: root. The staged initialization contract in
//! `design/plan/integration/runtime_initialization.md` splits initialization
//! in two: machine initialization hydrates physical resources — config
//! resolution and store opening before these stages, actor activation
//! after them — while world initialization creates the first durable
//! meaning, once per world. "World" is that contract's term for the
//! runtime's initialized epistemic state, not the world-model domain
//! crate. Its four stages, numbered 2 through 5 in the contract:
//!
//! - install theory: belief families and curation rules into their
//!   domain registries
//! - genesis identities: the seed agent, its rule binding, and its
//!   subscriptions
//! - prepare activation: exact owner and physical inputs into one inert
//!   product closure
//! - seed epistemic facts: the unobserved-scope genesis fact appended to
//!   the ledger
//!
//! Every stage is idempotent by content identity — reinstalling the same
//! content is a no-op, changed content is a new revision. Registrations
//! install through domain commands, epistemic facts append through the
//! canonical event capability, and nothing writes a domain store directly
//! from root or CLI code. Activation never creates semantic state.
//!
//! This file carries the frozen request and report shapes; [`pipeline`]
//! executes them, [`theory`] loads authored theory files, and [`tooling`]
//! adapts the CLI command.

use serde::{Deserialize, Serialize};

/// Stage execution over domain commands and the canonical append.
pub mod pipeline;
/// Compiled owner route publication for generic package installation.
pub mod routes;
/// Theory-source provisioning into the XDG theory root.
pub mod source;
/// Authored theory file loading from the XDG config home.
pub mod theory;
/// CLI adapter binding the command surface to the pipeline.
pub mod tooling;

/// One world-initialization stage of the staged pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldInitStage {
    /// Stage 2: install belief families, curation rules, evidence
    /// mappings, methods, and task packages into their domain registries.
    InstallTheory,
    /// Stage 3: create seed agent identity, perspective, and
    /// subscriptions with recorded provenance.
    GenesisIdentities,
    /// Stage 4: bind exact physical inputs and prepare one inert activation closure.
    PrepareActivation,
    /// Stage 5: append the epistemic genesis facts for the selected
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
