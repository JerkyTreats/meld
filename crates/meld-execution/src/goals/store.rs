//! Deterministic in-memory goal store for the first execution planning slice.

use crate::error::ExecutionInvariantError;
use crate::goals::contracts::{
    AddGoalCommand, ExecutionGoalRecord, GoalCommandCommitReceipt, GoalCommandKind,
    GoalCommandMetadata, GoalCommandOutcome, GoalCommandRequestContract,
    GoalCommandRequestIdentity, ModifyGoalCommand, RemoveGoalCommand, ResumeGoalCommand,
    SatisfyGoalCommand, SuspendGoalCommand,
};
use meld_lang::{Goal, GoalLifecycle};
use std::collections::BTreeMap;

/// In-memory goal set used by the first execution planning runtime slice.
#[derive(Debug, Default, Clone)]
pub struct GoalSetStore {
    records: BTreeMap<String, ExecutionGoalRecord>,
    command_identities: BTreeMap<String, GoalCommandRequestIdentity>,
    command_outcomes: BTreeMap<String, GoalCommandOutcome>,
    command_receipts: BTreeMap<String, GoalCommandCommitReceipt>,
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
        self.commit_add_goal(command).map(|(outcome, _)| outcome)
    }

    /// Apply an add command and return its complete in-memory commit receipt.
    pub fn commit_add_goal(
        &mut self,
        command: AddGoalCommand,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError> {
        let identity = request_identity(&command)?;
        self.commit_add_goal_with_identity(command, identity)
    }

    pub(crate) fn commit_add_goal_with_identity(
        &mut self,
        command: AddGoalCommand,
        identity: GoalCommandRequestIdentity,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError> {
        validate_metadata(&command.metadata)?;
        validate_goal(&command.goal)?;
        validate_request_identity(&command.metadata, GoalCommandKind::Add, &identity)?;
        if let Some(commit) = self.replayed_commit(&identity)? {
            return Ok(commit);
        }

        if let Some(existing_goal_id) = self.duplicate_goal_id(&command.metadata, &command.goal) {
            let outcome = GoalCommandOutcome::Duplicate { existing_goal_id };
            let receipt = self.record_commit(identity, outcome.clone())?;
            return Ok((outcome, receipt));
        }

        let record = ExecutionGoalRecord {
            goal: command.goal.clone(),
            source_command_id: Some(command.metadata.command_id.clone()),
            source_identity: command.metadata.source_identity.clone(),
            created_at_seq: command.metadata.seq,
            updated_at_seq: command.metadata.seq,
        };
        self.records
            .insert(command.goal.goal_id.clone(), record.clone());
        self.index_source_identity(&command.metadata, &command.goal.goal_id);
        let outcome = GoalCommandOutcome::Applied(Box::new(record));
        let receipt = self.record_commit(identity, outcome.clone())?;
        Ok((outcome, receipt))
    }

    /// Replace an existing goal while preserving creation sequence and dedupe state.
    pub fn modify_goal(
        &mut self,
        command: ModifyGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.commit_modify_goal(command).map(|(outcome, _)| outcome)
    }

    /// Apply a replacement and return its complete in-memory commit receipt.
    pub fn commit_modify_goal(
        &mut self,
        command: ModifyGoalCommand,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError> {
        let identity = request_identity(&command)?;
        self.commit_modify_goal_with_identity(command, identity)
    }

    pub(crate) fn commit_modify_goal_with_identity(
        &mut self,
        command: ModifyGoalCommand,
        identity: GoalCommandRequestIdentity,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError> {
        validate_metadata(&command.metadata)?;
        validate_goal(&command.goal)?;
        validate_request_identity(&command.metadata, GoalCommandKind::Modify, &identity)?;
        if let Some(commit) = self.replayed_commit(&identity)? {
            return Ok(commit);
        }

        if !self.records.contains_key(&command.goal.goal_id) {
            let outcome = GoalCommandOutcome::NotFound {
                goal_id: command.goal.goal_id.clone(),
            };
            let receipt = self.record_commit(identity, outcome.clone())?;
            return Ok((outcome, receipt));
        }
        let existing = self
            .records
            .get(&command.goal.goal_id)
            .expect("record existence checked");
        validate_newer_sequence(&command.metadata, existing)?;
        validate_modify_lifecycle(&command.goal.lifecycle, &existing.goal.lifecycle)?;
        if let Some(existing_goal_id) =
            self.duplicate_source_identity_for_other_goal(&command.metadata, &command.goal.goal_id)
        {
            let outcome = GoalCommandOutcome::Duplicate { existing_goal_id };
            let receipt = self.record_commit(identity, outcome.clone())?;
            return Ok((outcome, receipt));
        }

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
        let receipt = self.record_commit(identity, outcome.clone())?;
        Ok((outcome, receipt))
    }

    /// Apply an idempotent transition to abandoned.
    pub fn remove_goal(
        &mut self,
        command: RemoveGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.commit_remove_goal(command).map(|(outcome, _)| outcome)
    }

    /// Apply abandonment and return its complete in-memory commit receipt.
    pub fn commit_remove_goal(
        &mut self,
        command: RemoveGoalCommand,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError> {
        let identity = request_identity(&command)?;
        self.commit_remove_goal_with_identity(command, identity)
    }

    pub(crate) fn commit_remove_goal_with_identity(
        &mut self,
        command: RemoveGoalCommand,
        identity: GoalCommandRequestIdentity,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError> {
        validate_metadata(&command.metadata)?;
        validate_non_empty("remove reason", &command.reason)?;
        self.update_lifecycle(
            &command.metadata,
            &command.goal_id,
            GoalLifecycle::Abandoned {
                reason: command.reason,
            },
            GoalCommandKind::Remove,
            identity,
        )
    }

    /// Apply an idempotent transition to satisfied.
    pub fn satisfy_goal(
        &mut self,
        command: SatisfyGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.commit_satisfy_goal(command)
            .map(|(outcome, _)| outcome)
    }

    /// Apply satisfaction and return its complete in-memory commit receipt.
    pub fn commit_satisfy_goal(
        &mut self,
        command: SatisfyGoalCommand,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError> {
        let identity = request_identity(&command)?;
        self.commit_satisfy_goal_with_identity(command, identity)
    }

    pub(crate) fn commit_satisfy_goal_with_identity(
        &mut self,
        command: SatisfyGoalCommand,
        identity: GoalCommandRequestIdentity,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError> {
        validate_metadata(&command.metadata)?;
        validate_satisfaction_sequence(command.at_seq)?;
        self.update_lifecycle(
            &command.metadata,
            &command.goal_id,
            GoalLifecycle::Satisfied {
                at_seq: command.at_seq,
            },
            GoalCommandKind::Satisfy,
            identity,
        )
    }

    /// Apply an idempotent transition to suspended.
    pub fn suspend_goal(
        &mut self,
        command: SuspendGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.commit_suspend_goal(command)
            .map(|(outcome, _)| outcome)
    }

    /// Apply suspension and return its complete in-memory commit receipt.
    pub fn commit_suspend_goal(
        &mut self,
        command: SuspendGoalCommand,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError> {
        let identity = request_identity(&command)?;
        self.commit_suspend_goal_with_identity(command, identity)
    }

    pub(crate) fn commit_suspend_goal_with_identity(
        &mut self,
        command: SuspendGoalCommand,
        identity: GoalCommandRequestIdentity,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError> {
        validate_metadata(&command.metadata)?;
        validate_non_empty("suspend reason", &command.reason)?;
        self.update_lifecycle(
            &command.metadata,
            &command.goal_id,
            GoalLifecycle::Suspended {
                reason: command.reason,
            },
            GoalCommandKind::Suspend,
            identity,
        )
    }

    /// Apply an idempotent transition back to active.
    pub fn resume_goal(
        &mut self,
        command: ResumeGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.commit_resume_goal(command).map(|(outcome, _)| outcome)
    }

    /// Apply resume and return its complete in-memory commit receipt.
    pub fn commit_resume_goal(
        &mut self,
        command: ResumeGoalCommand,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError> {
        let identity = request_identity(&command)?;
        self.commit_resume_goal_with_identity(command, identity)
    }

    pub(crate) fn commit_resume_goal_with_identity(
        &mut self,
        command: ResumeGoalCommand,
        identity: GoalCommandRequestIdentity,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError> {
        validate_metadata(&command.metadata)?;
        self.update_lifecycle(
            &command.metadata,
            &command.goal_id,
            GoalLifecycle::Active,
            GoalCommandKind::Resume,
            identity,
        )
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
        command_kind: GoalCommandKind,
        identity: GoalCommandRequestIdentity,
    ) -> Result<(GoalCommandOutcome, GoalCommandCommitReceipt), ExecutionInvariantError> {
        validate_request_identity(metadata, command_kind, &identity)?;
        if let Some(commit) = self.replayed_commit(&identity)? {
            return Ok(commit);
        }
        validate_non_empty("goal id", goal_id)?;
        let Some(existing) = self.records.get(goal_id) else {
            let outcome = GoalCommandOutcome::NotFound {
                goal_id: goal_id.to_string(),
            };
            let receipt = self.record_commit(identity, outcome.clone())?;
            return Ok((outcome, receipt));
        };
        validate_newer_sequence(metadata, existing)?;
        validate_lifecycle_transition(&existing.goal.lifecycle, &lifecycle, command_kind)?;
        let mut record = existing.clone();
        record.goal.lifecycle = lifecycle;
        record.source_command_id = Some(metadata.command_id.clone());
        record.updated_at_seq = metadata.seq;
        self.records.insert(goal_id.to_string(), record.clone());
        let outcome = GoalCommandOutcome::Applied(Box::new(record));
        let receipt = self.record_commit(identity, outcome.clone())?;
        Ok((outcome, receipt))
    }

    fn replayed_commit(
        &self,
        identity: &GoalCommandRequestIdentity,
    ) -> Result<Option<(GoalCommandOutcome, GoalCommandCommitReceipt)>, ExecutionInvariantError>
    {
        let Some(existing_identity) = self.command_identities.get(&identity.command_id) else {
            return Ok(None);
        };
        if existing_identity != identity {
            return Err(replay_conflict(&identity.command_id));
        }
        let outcome = self
            .command_outcomes
            .get(&identity.command_id)
            .cloned()
            .ok_or_else(|| incomplete_commit(&identity.command_id, "outcome"))?;
        let receipt = self
            .command_receipts
            .get(&identity.command_id)
            .cloned()
            .ok_or_else(|| incomplete_commit(&identity.command_id, "receipt"))?;
        Ok(Some((outcome, receipt)))
    }

    fn record_commit(
        &mut self,
        identity: GoalCommandRequestIdentity,
        outcome: GoalCommandOutcome,
    ) -> Result<GoalCommandCommitReceipt, ExecutionInvariantError> {
        let receipt = commit_receipt(identity.clone(), &outcome)?;
        self.command_outcomes
            .insert(identity.command_id.clone(), outcome);
        self.command_identities
            .insert(identity.command_id.clone(), identity.clone());
        self.command_receipts
            .insert(identity.command_id, receipt.clone());
        Ok(receipt)
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

pub(crate) fn request_identity(
    command: &impl GoalCommandRequestContract,
) -> Result<GoalCommandRequestIdentity, ExecutionInvariantError> {
    command.request_identity().map_err(|error| {
        ExecutionInvariantError::ConfigError(format!(
            "goal command request identity failed: {error}"
        ))
    })
}

pub(crate) fn validate_request_identity(
    metadata: &GoalCommandMetadata,
    command_kind: GoalCommandKind,
    identity: &GoalCommandRequestIdentity,
) -> Result<(), ExecutionInvariantError> {
    if identity.command_id != metadata.command_id || identity.command_kind != command_kind {
        return Err(ExecutionInvariantError::ConfigError(
            "goal command request identity does not match command metadata".to_string(),
        ));
    }
    if identity.request_hash.len() != 64
        || !identity
            .request_hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(ExecutionInvariantError::ConfigError(
            "goal command request hash must be a BLAKE3 hex digest".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn commit_receipt(
    identity: GoalCommandRequestIdentity,
    outcome: &GoalCommandOutcome,
) -> Result<GoalCommandCommitReceipt, ExecutionInvariantError> {
    let encoded = serde_json::to_vec(outcome).map_err(|error| {
        ExecutionInvariantError::ConfigError(format!(
            "goal command outcome encoding failed: {error}"
        ))
    })?;
    Ok(GoalCommandCommitReceipt {
        identity,
        outcome_hash: blake3::hash(&encoded).to_hex().to_string(),
    })
}

pub(crate) fn replay_conflict(command_id: &str) -> ExecutionInvariantError {
    ExecutionInvariantError::ConfigError(format!(
        "goal command '{command_id}' was replayed with divergent intent"
    ))
}

fn incomplete_commit(command_id: &str, missing: &str) -> ExecutionInvariantError {
    ExecutionInvariantError::ConfigError(format!(
        "goal command '{command_id}' has identity without durable {missing}"
    ))
}

pub(crate) fn validate_metadata(
    metadata: &GoalCommandMetadata,
) -> Result<(), ExecutionInvariantError> {
    validate_non_empty("goal command id", &metadata.command_id)?;
    if metadata.seq == 0 {
        return Err(ExecutionInvariantError::ConfigError(
            "goal command sequence must be greater than zero".to_string(),
        ));
    }
    if let Some(source_identity) = &metadata.source_identity {
        validate_non_empty("goal source identity", source_identity)?;
    }
    Ok(())
}

pub(crate) fn validate_satisfaction_sequence(at_seq: u64) -> Result<(), ExecutionInvariantError> {
    if at_seq == 0 {
        return Err(ExecutionInvariantError::ConfigError(
            "goal satisfaction sequence must be greater than zero".to_string(),
        ));
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
    match &goal.lifecycle {
        GoalLifecycle::Suspended { reason } => validate_non_empty("suspend reason", reason)?,
        GoalLifecycle::Satisfied { at_seq } => validate_satisfaction_sequence(*at_seq)?,
        GoalLifecycle::Abandoned { reason } => validate_non_empty("remove reason", reason)?,
        GoalLifecycle::Proposed | GoalLifecycle::Active => {}
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

pub(crate) fn validate_newer_sequence(
    metadata: &GoalCommandMetadata,
    record: &ExecutionGoalRecord,
) -> Result<(), ExecutionInvariantError> {
    if metadata.seq <= record.updated_at_seq {
        return Err(ExecutionInvariantError::ConfigError(format!(
            "goal command '{}' sequence {} must be newer than stored sequence {}",
            metadata.command_id, metadata.seq, record.updated_at_seq
        )));
    }
    Ok(())
}

pub(crate) fn validate_modify_lifecycle(
    requested: &GoalLifecycle,
    stored: &GoalLifecycle,
) -> Result<(), ExecutionInvariantError> {
    if requested != stored {
        return Err(ExecutionInvariantError::ConfigError(
            "modify goal commands must preserve the stored lifecycle".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_lifecycle_transition(
    current: &GoalLifecycle,
    requested: &GoalLifecycle,
    command_kind: GoalCommandKind,
) -> Result<(), ExecutionInvariantError> {
    let legal = matches!(
        (command_kind, current, requested),
        (
            GoalCommandKind::Remove,
            GoalLifecycle::Proposed | GoalLifecycle::Active | GoalLifecycle::Suspended { .. },
            GoalLifecycle::Abandoned { .. }
        ) | (
            GoalCommandKind::Satisfy,
            GoalLifecycle::Active,
            GoalLifecycle::Satisfied { .. }
        ) | (
            GoalCommandKind::Suspend,
            GoalLifecycle::Active,
            GoalLifecycle::Suspended { .. }
        ) | (
            GoalCommandKind::Resume,
            GoalLifecycle::Suspended { .. },
            GoalLifecycle::Active
        )
    );
    if !legal {
        return Err(ExecutionInvariantError::ConfigError(format!(
            "illegal {command_kind:?} goal lifecycle transition from {current:?} to {requested:?}"
        )));
    }
    Ok(())
}
