//! Deterministic in-memory goal store for the first execution planning slice.

use crate::error::ExecutionInvariantError;
use crate::goals::contracts::{
    AddGoalCommand, ExecutionGoalRecord, GoalCommandMetadata, GoalCommandOutcome,
    ModifyGoalCommand, RemoveGoalCommand, ResumeGoalCommand, SatisfyGoalCommand,
    SuspendGoalCommand,
};
use meld_lang::{Goal, GoalLifecycle};
use std::collections::BTreeMap;

/// In-memory goal set used by the first execution planning runtime slice.
#[derive(Debug, Default, Clone)]
pub struct GoalSetStore {
    records: BTreeMap<String, ExecutionGoalRecord>,
    command_outcomes: BTreeMap<String, GoalCommandOutcome>,
    source_identity_index: BTreeMap<String, String>,
}

impl GoalSetStore {
    /// Create an empty deterministic goal store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate and store a new goal, replaying or deduping by command metadata.
    pub fn add_goal(
        &mut self,
        command: AddGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        if let Some(outcome) = self.replayed_outcome(&command.metadata) {
            return Ok(outcome);
        }
        validate_metadata(&command.metadata)?;
        validate_goal(&command.goal)?;

        if let Some(existing_goal_id) = self.duplicate_goal_id(&command.metadata, &command.goal) {
            let outcome = GoalCommandOutcome::Duplicate { existing_goal_id };
            self.record_outcome(&command.metadata, outcome.clone());
            return Ok(outcome);
        }

        let record = ExecutionGoalRecord {
            goal: command.goal.clone(),
            source_command_id: Some(command.metadata.command_id.clone()),
            source_identity: command.metadata.source_identity.clone(),
            lifecycle_epoch: 0,
            created_at_seq: command.metadata.seq,
            updated_at_seq: command.metadata.seq,
        };
        self.records
            .insert(command.goal.goal_id.clone(), record.clone());
        self.index_source_identity(&command.metadata, &command.goal.goal_id);
        let outcome = GoalCommandOutcome::Applied(Box::new(record));
        self.record_outcome(&command.metadata, outcome.clone());
        Ok(outcome)
    }

    /// Replace an existing goal while preserving creation sequence and dedupe state.
    pub fn modify_goal(
        &mut self,
        command: ModifyGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        if let Some(outcome) = self.replayed_outcome(&command.metadata) {
            return Ok(outcome);
        }
        validate_metadata(&command.metadata)?;
        validate_goal(&command.goal)?;

        if !self.records.contains_key(&command.goal.goal_id) {
            let outcome = GoalCommandOutcome::NotFound {
                goal_id: command.goal.goal_id.clone(),
            };
            self.record_outcome(&command.metadata, outcome.clone());
            return Ok(outcome);
        }
        if let Some(existing_goal_id) =
            self.duplicate_source_identity_for_other_goal(&command.metadata, &command.goal.goal_id)
        {
            let outcome = GoalCommandOutcome::Duplicate { existing_goal_id };
            self.record_outcome(&command.metadata, outcome.clone());
            return Ok(outcome);
        }

        let existing = self
            .records
            .get(&command.goal.goal_id)
            .expect("record existence checked");
        let previous_source_identity = existing.source_identity.clone();
        let source_identity = command
            .metadata
            .source_identity
            .clone()
            .or_else(|| existing.source_identity.clone());
        let record = ExecutionGoalRecord {
            goal: command.goal.clone(),
            source_command_id: Some(command.metadata.command_id.clone()),
            source_identity: source_identity.clone(),
            // Modification revises content under the same identity; only the
            // reopen command may advance the epoch.
            lifecycle_epoch: existing.lifecycle_epoch,
            created_at_seq: existing.created_at_seq,
            updated_at_seq: command.metadata.seq,
        };
        self.records
            .insert(command.goal.goal_id.clone(), record.clone());
        self.reindex_source_identity(
            previous_source_identity,
            source_identity,
            &command.goal.goal_id,
        );
        let outcome = GoalCommandOutcome::Applied(Box::new(record));
        self.record_outcome(&command.metadata, outcome.clone());
        Ok(outcome)
    }

    /// Apply an idempotent transition to abandoned.
    pub fn remove_goal(
        &mut self,
        command: RemoveGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        if let Some(outcome) = self.replayed_outcome(&command.metadata) {
            return Ok(outcome);
        }
        validate_metadata(&command.metadata)?;
        validate_non_empty("remove reason", &command.reason)?;
        self.update_lifecycle(
            &command.metadata,
            &command.goal_id,
            GoalLifecycle::Abandoned {
                reason: command.reason,
            },
        )
    }

    /// Apply an idempotent transition to satisfied.
    pub fn satisfy_goal(
        &mut self,
        command: SatisfyGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        if let Some(outcome) = self.replayed_outcome(&command.metadata) {
            return Ok(outcome);
        }
        validate_metadata(&command.metadata)?;
        self.update_lifecycle(
            &command.metadata,
            &command.goal_id,
            GoalLifecycle::Satisfied {
                at_seq: command.at_seq,
            },
        )
    }

    /// Apply an idempotent transition to suspended.
    pub fn suspend_goal(
        &mut self,
        command: SuspendGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        if let Some(outcome) = self.replayed_outcome(&command.metadata) {
            return Ok(outcome);
        }
        validate_metadata(&command.metadata)?;
        validate_non_empty("suspend reason", &command.reason)?;
        self.update_lifecycle(
            &command.metadata,
            &command.goal_id,
            GoalLifecycle::Suspended {
                reason: command.reason,
            },
        )
    }

    /// Apply an idempotent transition back to active.
    pub fn resume_goal(
        &mut self,
        command: ResumeGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        if let Some(outcome) = self.replayed_outcome(&command.metadata) {
            return Ok(outcome);
        }
        validate_metadata(&command.metadata)?;
        self.update_lifecycle(&command.metadata, &command.goal_id, GoalLifecycle::Active)
    }

    pub(crate) fn record(&self, goal_id: &str) -> Option<&ExecutionGoalRecord> {
        self.records.get(goal_id)
    }

    pub(crate) fn records(&self) -> impl Iterator<Item = &ExecutionGoalRecord> {
        self.records.values()
    }

    fn update_lifecycle(
        &mut self,
        metadata: &GoalCommandMetadata,
        goal_id: &str,
        lifecycle: GoalLifecycle,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        validate_non_empty("goal id", goal_id)?;
        let Some(existing) = self.records.get(goal_id) else {
            let outcome = GoalCommandOutcome::NotFound {
                goal_id: goal_id.to_string(),
            };
            self.record_outcome(metadata, outcome.clone());
            return Ok(outcome);
        };
        let mut record = existing.clone();
        record.goal.lifecycle = lifecycle;
        record.source_command_id = Some(metadata.command_id.clone());
        record.updated_at_seq = metadata.seq;
        self.records.insert(goal_id.to_string(), record.clone());
        let outcome = GoalCommandOutcome::Applied(Box::new(record));
        self.record_outcome(metadata, outcome.clone());
        Ok(outcome)
    }

    fn replayed_outcome(&self, metadata: &GoalCommandMetadata) -> Option<GoalCommandOutcome> {
        self.command_outcomes.get(&metadata.command_id).cloned()
    }

    fn record_outcome(&mut self, metadata: &GoalCommandMetadata, outcome: GoalCommandOutcome) {
        self.command_outcomes
            .insert(metadata.command_id.clone(), outcome);
    }

    fn duplicate_goal_id(&self, metadata: &GoalCommandMetadata, goal: &Goal) -> Option<String> {
        if self.records.contains_key(&goal.goal_id) {
            return Some(goal.goal_id.clone());
        }
        metadata
            .source_identity
            .as_ref()
            .and_then(|source_identity| self.source_identity_index.get(source_identity))
            .cloned()
    }

    fn duplicate_source_identity_for_other_goal(
        &self,
        metadata: &GoalCommandMetadata,
        goal_id: &str,
    ) -> Option<String> {
        metadata
            .source_identity
            .as_ref()
            .and_then(|source_identity| self.source_identity_index.get(source_identity))
            .filter(|existing_goal_id| existing_goal_id.as_str() != goal_id)
            .cloned()
    }

    fn index_source_identity(&mut self, metadata: &GoalCommandMetadata, goal_id: &str) {
        if let Some(source_identity) = &metadata.source_identity {
            self.source_identity_index
                .insert(source_identity.clone(), goal_id.to_string());
        }
    }

    fn reindex_source_identity(
        &mut self,
        previous: Option<String>,
        current: Option<String>,
        goal_id: &str,
    ) {
        if previous != current {
            if let Some(previous) = previous {
                self.source_identity_index.remove(&previous);
            }
        }
        if let Some(current) = current {
            self.source_identity_index
                .insert(current, goal_id.to_string());
        }
    }
}

pub(crate) fn validate_metadata(
    metadata: &GoalCommandMetadata,
) -> Result<(), ExecutionInvariantError> {
    validate_non_empty("goal command id", &metadata.command_id)?;
    if let Some(source_identity) = &metadata.source_identity {
        validate_non_empty("goal source identity", source_identity)?;
    }
    Ok(())
}

pub(crate) fn validate_goal(goal: &Goal) -> Result<(), ExecutionInvariantError> {
    validate_non_empty("goal id", &goal.goal_id)?;
    validate_non_empty("agent id", &goal.agent_id)?;
    if let Some(variable) = goal.target.grounding_issue() {
        return Err(ExecutionInvariantError::ConfigError(format!(
            "Execution goal '{}' target must be ground: {}",
            goal.goal_id, variable
        )));
    }
    Ok(())
}

pub(crate) fn validate_non_empty(label: &str, value: &str) -> Result<(), ExecutionInvariantError> {
    if value.trim().is_empty() {
        return Err(ExecutionInvariantError::ConfigError(format!(
            "{label} must be non-empty"
        )));
    }
    Ok(())
}
