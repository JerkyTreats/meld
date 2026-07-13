//! Read-only query facade over the execution goal set.

use crate::goals::{ExecutionGoalRecord, GoalSetStore};
use meld_lang::{Goal, GoalLifecycle};

/// Query surface used by planning and agent curation boundaries.
pub struct GoalSetQuery<'a> {
    store: &'a GoalSetStore,
}

impl<'a> GoalSetQuery<'a> {
    /// Create a query facade over one in-memory goal store.
    pub fn new(store: &'a GoalSetStore) -> Self {
        Self { store }
    }

    /// Return active goals by urgency, then stable goal id.
    pub fn active_goals(&self) -> Vec<Goal> {
        let mut goals = self
            .store
            .records()
            .filter(|record| matches!(record.goal.lifecycle, GoalLifecycle::Active))
            .map(|record| record.goal.clone())
            .collect::<Vec<_>>();
        goals.sort_by(|left, right| {
            left.priority
                .urgency
                .cmp(&right.priority.urgency)
                .then_with(|| left.goal_id.cmp(&right.goal_id))
        });
        goals
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
