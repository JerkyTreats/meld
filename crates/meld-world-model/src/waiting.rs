//! Waiting-on declarations for world-model bounded reports (DBG-016).
//!
//! Owner: world model. A declaration records what a selector found absent
//! when it computed eligibility — the other half of provenance: not why a
//! record exists, but what would have to exist for a quiet actor to commit
//! work. Emission is observational and derives from state the tick already
//! computed; it never gates, reorders, or fails semantic work. The
//! `condition` vocabulary is owned by the emitting actor's domain.

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
