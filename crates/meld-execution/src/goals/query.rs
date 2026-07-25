//! Read-only query facade over the execution goal set.

use crate::goals::{ExecutionGoalRecord, GoalSetStore};
use meld_lang::{Goal, GoalLifecycle};

/// Failure surface for active-goal reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveGoalQueryError {
    /// Human-readable failure description.
    pub message: String,
    /// Whether the caller may retry the same request unchanged.
    pub retryable: bool,
}

/// Narrow active-goal boundary consumed by the planning actor.
///
/// Planning discovers eligible work through this trait and nowhere else:
/// it never reads goal store internals, world-model belief state, or raw
/// storage. Root composes an implementor from the durable goal store.
pub trait ActiveGoalQuery {
    /// Active goal records in stable order, bounded by `limit` when given.
    fn active_goals(
        &mut self,
        limit: Option<usize>,
    ) -> Result<Vec<ExecutionGoalRecord>, ActiveGoalQueryError>;
}

/// Query surface used by planning and agent curation boundaries.
pub struct GoalSetQuery<'a> {
    store: &'a GoalSetStore,
}

impl<'a> GoalSetQuery<'a> {
    /// Create a query facade over one in-memory goal store.
    pub fn new(store: &'a GoalSetStore) -> Self {
        Self { store }
    }

    /// Return active goals in deterministic goal id order.
    pub fn active_goals(&self) -> Vec<Goal> {
        self.store
            .records()
            .filter(|record| matches!(record.goal.lifecycle, GoalLifecycle::Active))
            .map(|record| record.goal.clone())
            .collect()
    }

    /// Return one active goal when it exists.
    pub fn active_goal(&self, goal_id: &str) -> Option<Goal> {
        self.store
            .record(goal_id)
            .filter(|record| matches!(record.goal.lifecycle, GoalLifecycle::Active))
            .map(|record| record.goal.clone())
    }

    /// Return one goal record regardless of lifecycle.
    pub fn get_goal(&self, goal_id: &str) -> Option<ExecutionGoalRecord> {
        self.store.record(goal_id).cloned()
    }
}
