use serde::{Deserialize, Serialize};

use crate::{cost::CostEstimate, proposition::Proposition};

/// Proposition target with operational metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    /// Stable goal identity.
    pub goal_id: String,
    /// Agent that curated the goal.
    pub agent_id: String,
    /// Desired proposition.
    pub target: Proposition,
    /// Operational priority.
    pub priority: GoalPriority,
    /// Provenance for the goal.
    pub source: GoalSource,
    /// Current lifecycle state.
    pub lifecycle: GoalLifecycle,
}

/// Goal priority and optional cost ceiling.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalPriority {
    /// Lower number means higher urgency.
    pub urgency: u32,
    /// Maximum acceptable cost.
    pub cost_ceiling: Option<CostEstimate>,
}

/// Provenance for why a goal exists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GoalSource {
    /// Belief diverged from desired state.
    BeliefDivergence {
        /// Belief dimension that diverged.
        dimension: String,
        /// Observed state summary.
        observed: String,
        /// Desired state summary.
        desired: String,
    },
    /// User directly requested this goal.
    UserDirected {
        /// User directive text.
        directive: String,
    },
    /// Maintenance invariant.
    Maintenance {
        /// Human-readable invariant.
        invariant_description: String,
    },
    /// Decomposed from a parent goal.
    Decomposed {
        /// Parent goal identity.
        parent_goal_id: String,
    },
}

/// Lifecycle state for a goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GoalLifecycle {
    /// Proposed but not active.
    Proposed,
    /// Active for planning.
    Active,
    /// Suspended with a reason.
    Suspended {
        /// Reason for suspension.
        reason: String,
    },
    /// Satisfied at an event sequence.
    Satisfied {
        /// Sequence where satisfaction was observed.
        at_seq: u64,
    },
    /// Abandoned with a reason.
    Abandoned {
        /// Reason for abandonment.
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{condition::Condition, term::Term};

    #[test]
    fn goal_round_trips_and_transitions() {
        let mut goal = Goal {
            goal_id: "goal-1".into(),
            agent_id: "agent".into(),
            target: Proposition::Holds {
                subject: Term::Object(
                    meld_events::DomainObjectRef::new("domain", "node", "a").unwrap(),
                ),
                dimension: Term::Dimension("confidence".into()),
                condition: Condition::Present,
            },
            priority: GoalPriority {
                urgency: 1,
                cost_ceiling: Some(CostEstimate::zero()),
            },
            source: GoalSource::UserDirected {
                directive: "check".into(),
            },
            lifecycle: GoalLifecycle::Proposed,
        };

        goal.lifecycle = GoalLifecycle::Active;
        let encoded = serde_json::to_string(&goal).unwrap();
        let decoded: Goal = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, goal);
        assert!(decoded.target.is_ground());
    }
}
