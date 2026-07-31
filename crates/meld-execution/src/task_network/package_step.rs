//! Bounded package-step contract for resumable package execution.
//!
//! Owner: the task network dispatch boundary. One dispatch actor tick uses
//! this contract to advance one package run by at most one bounded ready
//! wave instead of calling an execute-to-completion helper over an unbounded
//! tree.
//!
//! The contract is domain-neutral. It carries budgets, progress counters, and
//! step reports only. Task compilation, capability invocation, artifact
//! meaning, and package authoring shapes stay in their owning domains behind
//! each implementor.
//!
//! Invariants every implementor must uphold:
//!
//! - One step releases no more than the requested budget of ready
//!   invocations, while preserving sibling fan-out and every compiled
//!   dependency edge.
//! - Readiness, expansion, artifact, and completion progress is durable
//!   before `step` returns.
//! - A fresh implementor instance constructed over the same durable state
//!   resumes exactly where the last step stopped, without repeating completed
//!   invocations.
//! - Steps issued after completion attempt no work.
//!
//! The existing task package executor is the first implementor of this
//! contract and is compatibility-scoped. Task-network composition graphs are
//! its second consumer.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::task_network::package_step::PackageStepRequest;
//!
//! let request = PackageStepRequest {
//!     max_ready_invocations: 2,
//! };
//!
//! assert_eq!(request.max_ready_invocations, 2);
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Bounded ready-wave budget for one package step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageStepRequest {
    /// Maximum ready invocations one step may release. A zero budget attempts
    /// no work and leaves durable state unchanged.
    pub max_ready_invocations: usize,
}

/// Progress counters projected from durable package-run state.
///
/// The counters are a receiver-owned projection for callers and reports. The
/// durable products they summarize stay in the implementor's own stores.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageStepProgress {
    /// Work units currently known to the package run, including units added
    /// by applied expansions.
    pub known_units: usize,
    /// Work units completed durably.
    pub completed_units: usize,
    /// Work units ready for release right now.
    pub ready_units: usize,
    /// Expansions applied durably to the package run.
    pub applied_expansions: usize,
    /// Artifacts persisted durably by the package run.
    pub persisted_artifacts: usize,
}

/// Report for one bounded package step.
///
/// Field names mirror the root runtime worker tick report shape, bounded
/// attempted and committed counts around input and output checkpoints, so
/// root adapters can translate this report without this crate depending on
/// root runtime types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageStepReport {
    /// Ready invocations released by this step.
    pub items_attempted: usize,
    /// Released invocations completed durably by this step.
    pub items_committed: usize,
    /// Durable progress observed before the step.
    pub input_progress: PackageStepProgress,
    /// Durable progress observed after the step.
    pub output_progress: PackageStepProgress,
    /// True when ready work remained beyond the budget at release time.
    pub budget_exhausted: bool,
    /// True when the package run reached aggregate completion.
    pub package_complete: bool,
}

/// Bounded, resumable stepping over one durable package run.
///
/// The dispatch boundary owns this contract. Implementors own how released
/// work executes and where progress persists; callers own budgets and when
/// the next step happens.
#[async_trait]
pub trait PackageStep {
    /// Implementor-owned error surfaced by one failed step.
    type Error;

    /// Returns progress counters projected from durable state.
    fn progress(&self) -> PackageStepProgress;

    /// Returns true when every known work unit completed.
    fn is_complete(&self) -> bool {
        let progress = self.progress();
        // Zero known units is never complete, matching the aggregate
        // publication guard: zero-of-zero is absent work, not finished work.
        progress.known_units > 0 && progress.completed_units == progress.known_units
    }

    /// Advances the package run by at most one bounded ready wave.
    ///
    /// The step releases no more than `max_ready_invocations` ready
    /// invocations, resolves each released invocation, and persists all
    /// readiness, expansion, artifact, and completion progress before
    /// returning. Dependent work that becomes ready during the step waits for
    /// a later step.
    async fn step(
        &mut self,
        request: &PackageStepRequest,
    ) -> Result<PackageStepReport, Self::Error>;
}
