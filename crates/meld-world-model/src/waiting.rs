//! Waiting-on declarations for world-model bounded reports (DBG-016).
//!
//! Owner: world model. A declaration records what a selector found absent
//! when it computed eligibility — the other half of provenance: not why a
//! record exists, but what would have to exist for a quiet actor to commit
//! work. Emission is observational and derives from state the tick already
//! computed; it never gates, reorders, or fails semantic work. The
//! `condition` vocabulary is owned by the emitting actor's domain.

/// Frozen condition vocabulary emitted by world-model actors.
///
/// These constants are the contract between the emitting selectors and
/// every downstream consumer (the eligibility walk, the projections, the
/// debugger surface): both sides compile against the same string, so a
/// rename cannot silently diverge the substrate's answer.
pub mod conditions {
    /// Assessment waits on a current graph anchor for its subject.
    pub const GRAPH_ANCHOR_ABSENT: &str = "graph_anchor_absent";
    /// Assessment waits on an installed belief family revision.
    pub const BELIEF_FAMILY_ABSENT: &str = "belief_family_absent";
    /// The assessment selector found no dirty key and no unassessed
    /// subject binding.
    pub const BELIEF_WORK_INELIGIBLE: &str = "belief_work_ineligible";
    /// No belief family id is configured for the actor at all.
    pub const BELIEF_FAMILIES_UNCONFIGURED: &str = "belief_families_unconfigured";
    /// An active lease owns the selected key.
    pub const ASSESSMENT_LEASE_HELD: &str = "assessment_lease_held";
    /// An assessment attempt failed past its declared preconditions.
    pub const ASSESSMENT_BLOCKED: &str = "assessment_blocked";
    /// Ingestion waits on committed events past its durable cursor.
    pub const LEDGER_QUIET_PAST_CURSOR: &str = "ledger_quiet_past_cursor";
    /// The replay window matched no installed source mapping.
    pub const NO_MAPPABLE_EVENTS: &str = "no_mappable_events";
    /// Goal curation waits on a revision newer than its delivery cursor.
    pub const NO_UNDELIVERED_REVISIONS: &str = "no_undelivered_revisions";
    /// Satisfaction waits on an unreviewed revision or receipted decision.
    pub const NO_PENDING_SATISFACTION_REVIEWS: &str = "no_pending_satisfaction_reviews";
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
