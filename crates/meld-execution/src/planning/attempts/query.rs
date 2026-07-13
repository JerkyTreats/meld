//! Read-only public query facade for planning attempt audit and recovery state.

use std::sync::Arc;

use crate::planning::attempts::contracts::{
    PlanningAttemptCommandOutcome, PlanningAttemptContinuation, PlanningAttemptDecisionAudit,
    PlanningAttemptHead, PlanningAttemptHistory, PlanningAttemptRecoverySelection,
    PlanningAttemptSelection, PlanningAttemptTerminalDiagnostic, PlanningPreparedCommand,
};
use crate::planning::attempts::store::{PlanningAttemptStorageError, PlanningAttemptStore};

/// Cloneable read-only query surface over one planning attempt authority.
#[derive(Clone)]
pub struct PlanningAttemptQuery {
    store: Arc<PlanningAttemptStore>,
}

impl PlanningAttemptQuery {
    /// Build a query surface over an opened planning attempt store.
    pub fn new(store: Arc<PlanningAttemptStore>) -> Self {
        Self { store }
    }

    /// Return one attempt head by exact stable identity.
    pub fn attempt(
        &self,
        attempt_id: &str,
    ) -> Result<Option<PlanningAttemptHead>, PlanningAttemptStorageError> {
        self.store.get_head(attempt_id)
    }

    /// Return one bounded history page after the supplied record ordinal.
    pub fn history_bounded(
        &self,
        attempt_id: &str,
        after_ordinal: Option<u32>,
        max_items: usize,
    ) -> Result<PlanningAttemptHistory, PlanningAttemptStorageError> {
        self.store
            .history_bounded(attempt_id, after_ordinal, max_items)
    }

    /// Select bounded attempts for one goal in sequence and identity order.
    pub fn attempts_for_goal_bounded(
        &self,
        goal_id: &str,
        continuation: Option<&PlanningAttemptContinuation>,
        max_items: usize,
    ) -> Result<PlanningAttemptSelection, PlanningAttemptStorageError> {
        self.store
            .attempts_for_goal_bounded(goal_id, continuation, max_items)
    }

    /// Select exact prepared commands that lack a durable terminal response.
    pub fn recoverable_commands_bounded(
        &self,
        continuation: Option<&PlanningAttemptContinuation>,
        max_items: usize,
    ) -> Result<PlanningAttemptRecoverySelection, PlanningAttemptStorageError> {
        self.store
            .recoverable_commands_bounded(continuation, max_items)
    }

    /// Return the terminal diagnostic when this attempt ended without a command.
    pub fn terminal_diagnostic(
        &self,
        attempt_id: &str,
    ) -> Result<Option<PlanningAttemptTerminalDiagnostic>, PlanningAttemptStorageError> {
        self.store.terminal_diagnostic(attempt_id)
    }

    /// Return the exact bounded planning decision audit when present.
    pub fn decision_audit(
        &self,
        attempt_id: &str,
    ) -> Result<Option<PlanningAttemptDecisionAudit>, PlanningAttemptStorageError> {
        self.store.decision_audit(attempt_id)
    }

    /// Return the exact durable prepared command when present.
    pub fn prepared_command(
        &self,
        attempt_id: &str,
    ) -> Result<Option<PlanningPreparedCommand>, PlanningAttemptStorageError> {
        self.store.prepared_command(attempt_id)
    }

    /// Return the terminal task-network response when present.
    pub fn command_outcome(
        &self,
        attempt_id: &str,
    ) -> Result<Option<PlanningAttemptCommandOutcome>, PlanningAttemptStorageError> {
        self.store.command_outcome(attempt_id)
    }
}
