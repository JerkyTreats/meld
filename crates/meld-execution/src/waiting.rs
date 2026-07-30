//! Waiting-on declarations for execution bounded reports (DBG-016).
//!
//! Owner: execution. A declaration records what a selector found absent
//! when it computed eligibility — the other half of provenance: not why a
//! record exists, but what would have to exist for a quiet actor to commit
//! work. Emission is observational and derives from state the tick already
//! computed; it never gates, reorders, or fails semantic work. The
//! `condition` vocabulary is owned by the emitting actor's domain.

/// Frozen condition vocabulary emitted by execution actors.
///
/// These constants are the contract between the emitting selectors and
/// every downstream consumer (the eligibility walk, the projections, the
/// debugger surface): both sides compile against the same string, so a
/// rename cannot silently diverge the substrate's answer.
pub mod conditions {
    /// Planning waits on an active goal record.
    pub const NO_ACTIVE_GOALS: &str = "no_active_goals";
    /// Dispatch waits on a claimable ready task.
    pub const NO_READY_TASKS: &str = "no_ready_tasks";
    /// A dependency edge names a task that does not exist.
    pub const DEPENDENCY_ENDPOINT_MISSING: &str = "dependency_endpoint_missing";
    /// The active task graph contains a cycle.
    pub const DEPENDENCY_CYCLE: &str = "dependency_cycle";
    /// A conditional edge is not dispatchable in this slice.
    pub const CONDITIONAL_EDGE_DEFERRED: &str = "conditional_edge_deferred";
    /// A data-flow edge waits on an upstream artifact.
    pub const UPSTREAM_ARTIFACT_UNAVAILABLE: &str = "upstream_artifact_unavailable";
    /// The publication outbox holds no pending task publications.
    pub const NO_PENDING_PUBLICATIONS: &str = "no_pending_publications";
    /// An aggregate publication waits on the run's terminal outcome.
    pub const AGGREGATE_RUN_NOT_TERMINAL: &str = "aggregate_run_not_terminal";
    /// No verified method applied to an active goal.
    pub const NO_APPLICABLE_METHOD: &str = "no_applicable_method";
    /// The projected world state lacked facts a goal evaluation needs.
    pub const WORLD_STATE_INDETERMINATE: &str = "world_state_indeterminate";
}

/// One domain-owned statement of what would make work eligible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitingOnDeclaration {
    /// Stable domain-vocabulary code naming the awaited condition.
    pub condition: String,
    /// Exact subject key the condition is about, when the domain knows it.
    pub subject_key: Option<String>,
    /// Human-readable detail in the emitting domain's vocabulary.
    pub detail: String,
}

impl WaitingOnDeclaration {
    /// Build a declaration with an exact subject key.
    pub fn about(
        condition: impl Into<String>,
        subject_key: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            condition: condition.into(),
            subject_key: Some(subject_key.into()),
            detail: detail.into(),
        }
    }

    /// Build a declaration whose condition has no single subject.
    pub fn broad(condition: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            condition: condition.into(),
            subject_key: None,
            detail: detail.into(),
        }
    }
}
