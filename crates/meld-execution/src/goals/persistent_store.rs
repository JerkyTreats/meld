//! Durable goal set storage for execution-owned goal lifecycle state.

use crate::error::ExecutionInvariantError;
use crate::goals::contracts::{
    AddGoalCommand, ExecutionGoalRecord, GoalCommandCommitReceipt, GoalCommandKind,
    GoalCommandMetadata, GoalCommandOutcome, GoalCommandRequestIdentity,
    LegacyGoalCommandReplayPolicy, ModifyGoalCommand, RemoveGoalCommand, ResumeGoalCommand,
    SatisfyGoalCommand, SuspendGoalCommand,
};
use crate::goals::store::{
    commit_receipt, replay_conflict, request_identity, validate_goal,
    validate_lifecycle_transition, validate_metadata, validate_modify_lifecycle,
    validate_newer_sequence, validate_non_empty, validate_request_identity,
};
use meld_lang::{Goal, GoalLifecycle};
use sled::{
    transaction::{
        ConflictableTransactionError, TransactionError, Transactional, TransactionalTree,
    },
    Db, Tree,
};
use std::io;
use std::sync::Arc;

const TREE_RECORDS: &str = "execution_goal_records";
const TREE_COMMAND_IDENTITIES: &str = "execution_goal_command_identities";
const TREE_COMMAND_OUTCOMES: &str = "execution_goal_command_outcomes";
const TREE_COMMAND_RECEIPTS: &str = "execution_goal_command_receipts";
const TREE_SOURCE_IDENTITY: &str = "execution_goal_source_identity";

type GoalCommandCommit = (GoalCommandOutcome, GoalCommandCommitReceipt);
type GoalTransactionError = ConflictableTransactionError<ExecutionInvariantError>;

/// Sled-backed goal set used when execution lifecycle state must survive restart.
///
/// Every mutating command stores its request identity, goal mutation, source
/// index changes, outcome, and commit receipt in one sled transaction. Public
/// commit methods flush that transaction before returning the receipt.
#[derive(Clone)]
pub struct PersistentGoalSetStore {
    db: Db,
    records: Tree,
    command_identities: Tree,
    command_outcomes: Tree,
    command_receipts: Tree,
    source_identity_index: Tree,
    legacy_replay_policy: LegacyGoalCommandReplayPolicy,
}

impl PersistentGoalSetStore {
    /// Open goal trees with verified reconstruction for compatible legacy outcomes.
    pub fn new(db: Db) -> Result<Self, ExecutionInvariantError> {
        Self::with_legacy_replay_policy(db, LegacyGoalCommandReplayPolicy::ReconstructAndVerify)
    }

    /// Open goal trees with an explicit legacy replay posture.
    pub fn with_legacy_replay_policy(
        db: Db,
        legacy_replay_policy: LegacyGoalCommandReplayPolicy,
    ) -> Result<Self, ExecutionInvariantError> {
        Ok(Self {
            records: db.open_tree(TREE_RECORDS).map_err(to_store_io)?,
            command_identities: db.open_tree(TREE_COMMAND_IDENTITIES).map_err(to_store_io)?,
            command_outcomes: db.open_tree(TREE_COMMAND_OUTCOMES).map_err(to_store_io)?,
            command_receipts: db.open_tree(TREE_COMMAND_RECEIPTS).map_err(to_store_io)?,
            source_identity_index: db.open_tree(TREE_SOURCE_IDENTITY).map_err(to_store_io)?,
            db,
            legacy_replay_policy,
        })
    }

    /// Open the store behind a shared pointer for runtime facade wiring.
    pub fn shared(db: Db) -> Result<Arc<Self>, ExecutionInvariantError> {
        Ok(Arc::new(Self::new(db)?))
    }

    /// Validate and durably store a new goal.
    pub fn add_goal(
        &self,
        command: AddGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.commit_add_goal(command).map(|(outcome, _)| outcome)
    }

    /// Atomically commit, flush, and acknowledge a new goal command.
    pub fn commit_add_goal(
        &self,
        command: AddGoalCommand,
    ) -> Result<GoalCommandCommit, ExecutionInvariantError> {
        let identity = request_identity(&command)?;
        let commit = self.commit_add_goal_with_identity(command, identity)?;
        self.flush()?;
        Ok(commit)
    }

    pub(crate) fn commit_add_goal_with_identity(
        &self,
        command: AddGoalCommand,
        identity: GoalCommandRequestIdentity,
    ) -> Result<GoalCommandCommit, ExecutionInvariantError> {
        validate_metadata(&command.metadata)?;
        validate_goal(&command.goal)?;
        validate_request_identity(&command.metadata, GoalCommandKind::Add, &identity)?;
        let legacy_identity_verifiable = request_identity(&command)? == identity;

        let expected_record = ExecutionGoalRecord {
            goal: command.goal.clone(),
            source_command_id: Some(command.metadata.command_id.clone()),
            source_identity: command.metadata.source_identity.clone(),
            created_at_seq: command.metadata.seq,
            updated_at_seq: command.metadata.seq,
        };
        let legacy_expected = expected_record.clone();
        let command_key = command.metadata.command_id.as_bytes().to_vec();
        let goal_key = command.goal.goal_id.as_bytes().to_vec();
        let goal_id = command.goal.goal_id.clone();
        let source_identity_key = command
            .metadata
            .source_identity
            .as_ref()
            .map(|source_identity| source_identity.as_bytes().to_vec());
        let source_identity_value = goal_id.as_bytes().to_vec();

        let commit = (
            &self.records,
            &self.source_identity_index,
            &self.command_identities,
            &self.command_outcomes,
            &self.command_receipts,
        )
            .transaction(
                |(
                    records,
                    source_identity_index,
                    command_identities,
                    command_outcomes,
                    command_receipts,
                )| {
                    if let Some(commit) = replay_or_upgrade(
                        command_identities,
                        command_outcomes,
                        command_receipts,
                        &identity,
                        self.legacy_replay_policy,
                        |outcome| {
                            legacy_identity_verifiable
                                && matches!(
                                outcome,
                                GoalCommandOutcome::Applied(record)
                                    if record.as_ref() == &legacy_expected
                                )
                        },
                    )? {
                        return Ok(commit);
                    }

                    let outcome = if let Some(raw) = records.get(goal_key.clone())? {
                        let existing: ExecutionGoalRecord = decode_transaction(&raw)?;
                        if existing.source_command_id.as_deref()
                            == Some(command.metadata.command_id.as_str())
                        {
                            if existing != expected_record {
                                return Err(ConflictableTransactionError::Abort(replay_conflict(
                                    &command.metadata.command_id,
                                )));
                            }
                            if let Some(source_identity_key) = source_identity_key.clone() {
                                source_identity_index
                                    .insert(source_identity_key, source_identity_value.clone())?;
                            }
                            GoalCommandOutcome::Applied(Box::new(existing))
                        } else {
                            GoalCommandOutcome::Duplicate {
                                existing_goal_id: goal_id.clone(),
                            }
                        }
                    } else if let Some(source_identity_key) = source_identity_key.clone() {
                        if let Some(raw) = source_identity_index.get(source_identity_key.clone())? {
                            GoalCommandOutcome::Duplicate {
                                existing_goal_id: String::from_utf8(raw.to_vec())
                                    .map_err(to_transaction_utf8)?,
                            }
                        } else {
                            records
                                .insert(goal_key.clone(), encode_transaction(&expected_record)?)?;
                            source_identity_index
                                .insert(source_identity_key, source_identity_value.clone())?;
                            GoalCommandOutcome::Applied(Box::new(expected_record.clone()))
                        }
                    } else {
                        records.insert(goal_key.clone(), encode_transaction(&expected_record)?)?;
                        GoalCommandOutcome::Applied(Box::new(expected_record.clone()))
                    };

                    persist_commit(
                        command_identities,
                        command_outcomes,
                        command_receipts,
                        command_key.clone(),
                        identity.clone(),
                        outcome,
                    )
                },
            )
            .map_err(to_goal_transaction)?;
        Ok(commit)
    }

    /// Replace an existing goal while preserving creation sequence and dedupe state.
    pub fn modify_goal(
        &self,
        command: ModifyGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.commit_modify_goal(command).map(|(outcome, _)| outcome)
    }

    /// Atomically commit, flush, and acknowledge a goal replacement.
    pub fn commit_modify_goal(
        &self,
        command: ModifyGoalCommand,
    ) -> Result<GoalCommandCommit, ExecutionInvariantError> {
        let identity = request_identity(&command)?;
        let commit = self.commit_modify_goal_with_identity(command, identity)?;
        self.flush()?;
        Ok(commit)
    }

    pub(crate) fn commit_modify_goal_with_identity(
        &self,
        command: ModifyGoalCommand,
        identity: GoalCommandRequestIdentity,
    ) -> Result<GoalCommandCommit, ExecutionInvariantError> {
        validate_metadata(&command.metadata)?;
        validate_goal(&command.goal)?;
        validate_request_identity(&command.metadata, GoalCommandKind::Modify, &identity)?;

        let command_key = command.metadata.command_id.as_bytes().to_vec();
        let goal_key = command.goal.goal_id.as_bytes().to_vec();
        let goal_id = command.goal.goal_id.clone();
        let legacy_goal = command.goal.clone();
        let legacy_metadata = command.metadata.clone();

        let commit = (
            &self.records,
            &self.source_identity_index,
            &self.command_identities,
            &self.command_outcomes,
            &self.command_receipts,
        )
            .transaction(
                |(
                    records,
                    source_identity_index,
                    command_identities,
                    command_outcomes,
                    command_receipts,
                )| {
                    if let Some(commit) = replay_or_upgrade(
                        command_identities,
                        command_outcomes,
                        command_receipts,
                        &identity,
                        self.legacy_replay_policy,
                        |outcome| legacy_modify_matches(outcome, &legacy_metadata, &legacy_goal),
                    )? {
                        return Ok(commit);
                    }

                    let Some(raw) = records.get(goal_key.clone())? else {
                        return persist_commit(
                            command_identities,
                            command_outcomes,
                            command_receipts,
                            command_key.clone(),
                            identity.clone(),
                            GoalCommandOutcome::NotFound {
                                goal_id: goal_id.clone(),
                            },
                        );
                    };
                    let existing: ExecutionGoalRecord = decode_transaction(&raw)?;
                    validate_newer_sequence(&command.metadata, &existing)
                        .map_err(ConflictableTransactionError::Abort)?;
                    validate_modify_lifecycle(&command.goal.lifecycle, &existing.goal.lifecycle)
                        .map_err(ConflictableTransactionError::Abort)?;

                    if let Some(source_identity) = command.metadata.source_identity.as_ref() {
                        if let Some(raw) = source_identity_index.get(source_identity.as_bytes())? {
                            let existing_goal_id =
                                String::from_utf8(raw.to_vec()).map_err(to_transaction_utf8)?;
                            if existing_goal_id != goal_id {
                                return persist_commit(
                                    command_identities,
                                    command_outcomes,
                                    command_receipts,
                                    command_key.clone(),
                                    identity.clone(),
                                    GoalCommandOutcome::Duplicate { existing_goal_id },
                                );
                            }
                        }
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
                    records.insert(goal_key.clone(), encode_transaction(&record)?)?;
                    reindex_source_identity_transaction(
                        source_identity_index,
                        previous_source_identity,
                        source_identity,
                        &goal_id,
                    )?;
                    persist_commit(
                        command_identities,
                        command_outcomes,
                        command_receipts,
                        command_key.clone(),
                        identity.clone(),
                        GoalCommandOutcome::Applied(Box::new(record)),
                    )
                },
            )
            .map_err(to_goal_transaction)?;
        Ok(commit)
    }

    /// Apply an idempotent transition to abandoned.
    pub fn remove_goal(
        &self,
        command: RemoveGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.commit_remove_goal(command).map(|(outcome, _)| outcome)
    }

    /// Atomically commit, flush, and acknowledge abandonment.
    pub fn commit_remove_goal(
        &self,
        command: RemoveGoalCommand,
    ) -> Result<GoalCommandCommit, ExecutionInvariantError> {
        let identity = request_identity(&command)?;
        let commit = self.commit_remove_goal_with_identity(command, identity)?;
        self.flush()?;
        Ok(commit)
    }

    pub(crate) fn commit_remove_goal_with_identity(
        &self,
        command: RemoveGoalCommand,
        identity: GoalCommandRequestIdentity,
    ) -> Result<GoalCommandCommit, ExecutionInvariantError> {
        validate_metadata(&command.metadata)?;
        validate_non_empty("remove reason", &command.reason)?;
        self.commit_lifecycle(
            command.metadata,
            command.goal_id,
            GoalLifecycle::Abandoned {
                reason: command.reason,
            },
            GoalCommandKind::Remove,
            identity,
        )
    }

    /// Apply an idempotent transition to satisfied.
    pub fn satisfy_goal(
        &self,
        command: SatisfyGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.commit_satisfy_goal(command)
            .map(|(outcome, _)| outcome)
    }

    /// Atomically commit, flush, and acknowledge satisfaction.
    pub fn commit_satisfy_goal(
        &self,
        command: SatisfyGoalCommand,
    ) -> Result<GoalCommandCommit, ExecutionInvariantError> {
        let identity = request_identity(&command)?;
        let commit = self.commit_satisfy_goal_with_identity(command, identity)?;
        self.flush()?;
        Ok(commit)
    }

    pub(crate) fn commit_satisfy_goal_with_identity(
        &self,
        command: SatisfyGoalCommand,
        identity: GoalCommandRequestIdentity,
    ) -> Result<GoalCommandCommit, ExecutionInvariantError> {
        validate_metadata(&command.metadata)?;
        self.commit_lifecycle(
            command.metadata,
            command.goal_id,
            GoalLifecycle::Satisfied {
                at_seq: command.at_seq,
            },
            GoalCommandKind::Satisfy,
            identity,
        )
    }

    /// Apply an idempotent transition to suspended.
    pub fn suspend_goal(
        &self,
        command: SuspendGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.commit_suspend_goal(command)
            .map(|(outcome, _)| outcome)
    }

    /// Atomically commit, flush, and acknowledge suspension.
    pub fn commit_suspend_goal(
        &self,
        command: SuspendGoalCommand,
    ) -> Result<GoalCommandCommit, ExecutionInvariantError> {
        let identity = request_identity(&command)?;
        let commit = self.commit_suspend_goal_with_identity(command, identity)?;
        self.flush()?;
        Ok(commit)
    }

    pub(crate) fn commit_suspend_goal_with_identity(
        &self,
        command: SuspendGoalCommand,
        identity: GoalCommandRequestIdentity,
    ) -> Result<GoalCommandCommit, ExecutionInvariantError> {
        validate_metadata(&command.metadata)?;
        validate_non_empty("suspend reason", &command.reason)?;
        self.commit_lifecycle(
            command.metadata,
            command.goal_id,
            GoalLifecycle::Suspended {
                reason: command.reason,
            },
            GoalCommandKind::Suspend,
            identity,
        )
    }

    /// Apply an idempotent transition back to active.
    pub fn resume_goal(
        &self,
        command: ResumeGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        self.commit_resume_goal(command).map(|(outcome, _)| outcome)
    }

    /// Atomically commit, flush, and acknowledge resume.
    pub fn commit_resume_goal(
        &self,
        command: ResumeGoalCommand,
    ) -> Result<GoalCommandCommit, ExecutionInvariantError> {
        let identity = request_identity(&command)?;
        let commit = self.commit_resume_goal_with_identity(command, identity)?;
        self.flush()?;
        Ok(commit)
    }

    pub(crate) fn commit_resume_goal_with_identity(
        &self,
        command: ResumeGoalCommand,
        identity: GoalCommandRequestIdentity,
    ) -> Result<GoalCommandCommit, ExecutionInvariantError> {
        validate_metadata(&command.metadata)?;
        self.commit_lifecycle(
            command.metadata,
            command.goal_id,
            GoalLifecycle::Active,
            GoalCommandKind::Resume,
            identity,
        )
    }

    /// Return active goals ordered by urgency, then stable goal id.
    pub fn active_goals(&self) -> Result<Vec<Goal>, ExecutionInvariantError> {
        let mut goals = self
            .records()?
            .into_iter()
            .filter(|record| matches!(record.goal.lifecycle, GoalLifecycle::Active))
            .map(|record| record.goal)
            .collect::<Vec<_>>();
        goals.sort_by(|left, right| {
            left.priority
                .urgency
                .cmp(&right.priority.urgency)
                .then_with(|| left.goal_id.cmp(&right.goal_id))
        });
        Ok(goals)
    }

    /// Return one active goal when it exists.
    pub fn active_goal(&self, goal_id: &str) -> Result<Option<Goal>, ExecutionInvariantError> {
        Ok(self
            .record(goal_id)?
            .filter(|record| matches!(record.goal.lifecycle, GoalLifecycle::Active))
            .map(|record| record.goal))
    }

    /// Return one goal record regardless of lifecycle.
    pub fn get_goal(
        &self,
        goal_id: &str,
    ) -> Result<Option<ExecutionGoalRecord>, ExecutionInvariantError> {
        self.record(goal_id)
    }

    /// Return all goal records in deterministic goal id order.
    pub fn goal_records(&self) -> Result<Vec<ExecutionGoalRecord>, ExecutionInvariantError> {
        self.records()
    }

    /// Return the verified request identity for one command.
    pub fn command_identity(
        &self,
        command_id: &str,
    ) -> Result<Option<GoalCommandRequestIdentity>, ExecutionInvariantError> {
        decode_optional(
            self.command_identities
                .get(command_id.as_bytes())
                .map_err(to_store_io)?,
        )
    }

    /// Return the durable commit receipt for one command.
    pub fn command_receipt(
        &self,
        command_id: &str,
    ) -> Result<Option<GoalCommandCommitReceipt>, ExecutionInvariantError> {
        self.flush()?;
        decode_optional(
            self.command_receipts
                .get(command_id.as_bytes())
                .map_err(to_store_io)?,
        )
    }

    /// Flush durable writes to the backing database.
    pub fn flush(&self) -> Result<(), ExecutionInvariantError> {
        self.db.flush().map_err(to_store_io)?;
        Ok(())
    }

    fn commit_lifecycle(
        &self,
        metadata: GoalCommandMetadata,
        goal_id: String,
        lifecycle: GoalLifecycle,
        command_kind: GoalCommandKind,
        identity: GoalCommandRequestIdentity,
    ) -> Result<GoalCommandCommit, ExecutionInvariantError> {
        validate_request_identity(&metadata, command_kind, &identity)?;
        validate_non_empty("goal id", &goal_id)?;

        let command_key = metadata.command_id.as_bytes().to_vec();
        let goal_key = goal_id.as_bytes().to_vec();
        let commit = (
            &self.records,
            &self.command_identities,
            &self.command_outcomes,
            &self.command_receipts,
        )
            .transaction(
                |(records, command_identities, command_outcomes, command_receipts)| {
                    if let Some(commit) = replay_or_upgrade(
                        command_identities,
                        command_outcomes,
                        command_receipts,
                        &identity,
                        self.legacy_replay_policy,
                        legacy_lifecycle_matches,
                    )? {
                        return Ok(commit);
                    }

                    let outcome = match records.get(goal_key.clone())? {
                        Some(raw) => {
                            let mut record: ExecutionGoalRecord = decode_transaction(&raw)?;
                            validate_newer_sequence(&metadata, &record)
                                .map_err(ConflictableTransactionError::Abort)?;
                            validate_lifecycle_transition(
                                &record.goal.lifecycle,
                                &lifecycle,
                                command_kind,
                            )
                            .map_err(ConflictableTransactionError::Abort)?;
                            record.goal.lifecycle = lifecycle.clone();
                            record.source_command_id = Some(metadata.command_id.clone());
                            record.updated_at_seq = metadata.seq;
                            records.insert(goal_key.clone(), encode_transaction(&record)?)?;
                            GoalCommandOutcome::Applied(Box::new(record))
                        }
                        None => GoalCommandOutcome::NotFound {
                            goal_id: goal_id.clone(),
                        },
                    };
                    persist_commit(
                        command_identities,
                        command_outcomes,
                        command_receipts,
                        command_key.clone(),
                        identity.clone(),
                        outcome,
                    )
                },
            )
            .map_err(to_goal_transaction)?;
        Ok(commit)
    }

    fn record(
        &self,
        goal_id: &str,
    ) -> Result<Option<ExecutionGoalRecord>, ExecutionInvariantError> {
        decode_optional(self.records.get(goal_id.as_bytes()).map_err(to_store_io)?)
    }

    fn records(&self) -> Result<Vec<ExecutionGoalRecord>, ExecutionInvariantError> {
        let mut out = Vec::new();
        for item in self.records.iter() {
            let (_, value) = item.map_err(to_store_io)?;
            out.push(serde_json::from_slice(&value).map_err(to_store_data)?);
        }
        out.sort_by(|left: &ExecutionGoalRecord, right: &ExecutionGoalRecord| {
            left.goal.goal_id.cmp(&right.goal.goal_id)
        });
        Ok(out)
    }
}

fn replay_or_upgrade<F>(
    identities: &TransactionalTree,
    outcomes: &TransactionalTree,
    receipts: &TransactionalTree,
    identity: &GoalCommandRequestIdentity,
    legacy_policy: LegacyGoalCommandReplayPolicy,
    legacy_matches: F,
) -> Result<Option<GoalCommandCommit>, GoalTransactionError>
where
    F: Fn(&GoalCommandOutcome) -> bool,
{
    let key = identity.command_id.as_bytes().to_vec();
    if let Some(raw) = identities.get(key.clone())? {
        let existing: GoalCommandRequestIdentity = decode_transaction(&raw)?;
        if existing != *identity {
            return Err(ConflictableTransactionError::Abort(replay_conflict(
                &identity.command_id,
            )));
        }
        let outcome: GoalCommandOutcome =
            required_transaction_value(outcomes, &key, &identity.command_id, "outcome")?;
        let receipt: GoalCommandCommitReceipt =
            required_transaction_value(receipts, &key, &identity.command_id, "receipt")?;
        let expected_receipt =
            commit_receipt(existing, &outcome).map_err(ConflictableTransactionError::Abort)?;
        if receipt != expected_receipt {
            return Err(ConflictableTransactionError::Abort(incomplete_commit(
                &identity.command_id,
                "valid receipt",
            )));
        }
        return Ok(Some((outcome, receipt)));
    }

    if receipts.get(key.clone())?.is_some() {
        return Err(ConflictableTransactionError::Abort(incomplete_commit(
            &identity.command_id,
            "request identity",
        )));
    }

    let Some(raw_outcome) = outcomes.get(key.clone())? else {
        return Ok(None);
    };
    if legacy_policy == LegacyGoalCommandReplayPolicy::RejectUnverified {
        return Err(ConflictableTransactionError::Abort(legacy_replay_rejected(
            &identity.command_id,
        )));
    }
    let outcome: GoalCommandOutcome = decode_transaction(&raw_outcome)?;
    if !legacy_matches(&outcome) {
        return Err(ConflictableTransactionError::Abort(replay_conflict(
            &identity.command_id,
        )));
    }
    let receipt =
        commit_receipt(identity.clone(), &outcome).map_err(ConflictableTransactionError::Abort)?;
    identities.insert(key.clone(), encode_transaction(identity)?)?;
    receipts.insert(key, encode_transaction(&receipt)?)?;
    Ok(Some((outcome, receipt)))
}

fn persist_commit(
    identities: &TransactionalTree,
    outcomes: &TransactionalTree,
    receipts: &TransactionalTree,
    key: Vec<u8>,
    identity: GoalCommandRequestIdentity,
    outcome: GoalCommandOutcome,
) -> Result<GoalCommandCommit, GoalTransactionError> {
    let receipt =
        commit_receipt(identity.clone(), &outcome).map_err(ConflictableTransactionError::Abort)?;
    identities.insert(key.clone(), encode_transaction(&identity)?)?;
    outcomes.insert(key.clone(), encode_transaction(&outcome)?)?;
    receipts.insert(key, encode_transaction(&receipt)?)?;
    Ok((outcome, receipt))
}

fn reindex_source_identity_transaction(
    index: &TransactionalTree,
    previous: Option<String>,
    current: Option<String>,
    goal_id: &str,
) -> Result<(), GoalTransactionError> {
    if previous != current {
        if let Some(previous) = previous {
            if let Some(raw) = index.get(previous.as_bytes())? {
                let indexed_goal = String::from_utf8(raw.to_vec()).map_err(to_transaction_utf8)?;
                if indexed_goal != goal_id {
                    return Err(ConflictableTransactionError::Abort(
                        ExecutionInvariantError::ConfigError(format!(
                            "goal source identity '{previous}' points to unexpected goal '{indexed_goal}'"
                        )),
                    ));
                }
                index.remove(previous.as_bytes())?;
            }
        }
    }
    if let Some(current) = current {
        index.insert(current.as_bytes(), goal_id.as_bytes())?;
    }
    Ok(())
}

fn legacy_modify_matches(
    outcome: &GoalCommandOutcome,
    metadata: &GoalCommandMetadata,
    goal: &Goal,
) -> bool {
    matches!(
        outcome,
        GoalCommandOutcome::Applied(record)
            if record.goal == *goal
                && record.source_command_id.as_deref() == Some(metadata.command_id.as_str())
                && record.updated_at_seq == metadata.seq
                && metadata.source_identity.is_none()
                && record.source_identity.is_none()
    )
}

fn legacy_lifecycle_matches(_outcome: &GoalCommandOutcome) -> bool {
    // Legacy lifecycle outcomes never retained command source identity, so no
    // complete lifecycle request can be reconstructed without ambiguity.
    false
}

fn required_transaction_value<T: serde::de::DeserializeOwned>(
    tree: &TransactionalTree,
    key: &[u8],
    command_id: &str,
    name: &str,
) -> Result<T, GoalTransactionError> {
    let raw = tree
        .get(key)?
        .ok_or_else(|| ConflictableTransactionError::Abort(incomplete_commit(command_id, name)))?;
    decode_transaction(&raw)
}

fn decode_optional<T: serde::de::DeserializeOwned>(
    raw: Option<sled::IVec>,
) -> Result<Option<T>, ExecutionInvariantError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&raw).map_err(to_store_data)?))
}

fn decode_transaction<T: serde::de::DeserializeOwned>(
    raw: &[u8],
) -> Result<T, GoalTransactionError> {
    serde_json::from_slice(raw).map_err(to_transaction_data)
}

fn encode_transaction<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, GoalTransactionError> {
    serde_json::to_vec(value).map_err(to_transaction_data)
}

fn incomplete_commit(command_id: &str, missing: &str) -> ExecutionInvariantError {
    ExecutionInvariantError::ConfigError(format!(
        "goal command '{command_id}' is missing durable {missing}"
    ))
}

fn legacy_replay_rejected(command_id: &str) -> ExecutionInvariantError {
    ExecutionInvariantError::ConfigError(format!(
        "legacy goal command '{command_id}' has no verified request identity"
    ))
}

fn to_store_io(err: sled::Error) -> ExecutionInvariantError {
    ExecutionInvariantError::ConfigError(format!("goal store IO failed: {err}"))
}

fn to_store_data(err: serde_json::Error) -> ExecutionInvariantError {
    ExecutionInvariantError::ConfigError(format!("goal store JSON failed: {err}"))
}

fn to_store_utf8(err: std::string::FromUtf8Error) -> ExecutionInvariantError {
    ExecutionInvariantError::ConfigError(format!(
        "goal store UTF-8 decode failed: {}",
        io::Error::new(io::ErrorKind::InvalidData, err)
    ))
}

fn to_transaction_data(err: serde_json::Error) -> GoalTransactionError {
    ConflictableTransactionError::Abort(to_store_data(err))
}

fn to_transaction_utf8(err: std::string::FromUtf8Error) -> GoalTransactionError {
    ConflictableTransactionError::Abort(to_store_utf8(err))
}

fn to_goal_transaction(err: TransactionError<ExecutionInvariantError>) -> ExecutionInvariantError {
    match err {
        TransactionError::Abort(error) => error,
        TransactionError::Storage(error) => to_store_io(error),
    }
}
