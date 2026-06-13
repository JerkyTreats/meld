//! Durable goal set storage for execution-owned goal lifecycle state.

use crate::error::ExecutionInvariantError;
use crate::goals::contracts::{
    AddGoalCommand, ExecutionGoalRecord, GoalCommandMetadata, GoalCommandOutcome,
    ModifyGoalCommand, RemoveGoalCommand, ResumeGoalCommand, SatisfyGoalCommand,
    SuspendGoalCommand,
};
use crate::goals::store::{validate_goal, validate_metadata, validate_non_empty};
use meld_lang::{Goal, GoalLifecycle};
use sled::{
    transaction::{ConflictableTransactionError, TransactionError, Transactional},
    Db, Tree,
};
use std::io;
use std::sync::Arc;

const TREE_RECORDS: &str = "execution_goal_records";
const TREE_COMMAND_OUTCOMES: &str = "execution_goal_command_outcomes";
const TREE_SOURCE_IDENTITY: &str = "execution_goal_source_identity";

/// Sled-backed goal set used when execution lifecycle state must survive restart.
///
/// The caller owns database path selection so runtime assembly can enforce the
/// repository storage policy and keep execution state outside the target
/// workspace.
#[derive(Clone)]
pub struct PersistentGoalSetStore {
    db: Db,
    records: Tree,
    command_outcomes: Tree,
    source_identity_index: Tree,
}

impl PersistentGoalSetStore {
    /// Open goal trees from a caller-provided database.
    pub fn new(db: Db) -> Result<Self, ExecutionInvariantError> {
        Ok(Self {
            records: db.open_tree(TREE_RECORDS).map_err(to_store_io)?,
            command_outcomes: db.open_tree(TREE_COMMAND_OUTCOMES).map_err(to_store_io)?,
            source_identity_index: db.open_tree(TREE_SOURCE_IDENTITY).map_err(to_store_io)?,
            db,
        })
    }

    /// Open the store behind a shared pointer for runtime facade wiring.
    pub fn shared(db: Db) -> Result<Arc<Self>, ExecutionInvariantError> {
        Ok(Arc::new(Self::new(db)?))
    }

    /// Validate and durably store a new goal, replaying or deduping by command metadata.
    pub fn add_goal(
        &self,
        command: AddGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        if let Some(outcome) = self.replayed_outcome(&command.metadata)? {
            return Ok(outcome);
        }
        validate_metadata(&command.metadata)?;
        validate_goal(&command.goal)?;

        let record = ExecutionGoalRecord {
            goal: command.goal.clone(),
            source_command_id: Some(command.metadata.command_id.clone()),
            source_identity: command.metadata.source_identity.clone(),
            created_at_seq: command.metadata.seq,
            updated_at_seq: command.metadata.seq,
        };
        let outcome = GoalCommandOutcome::Applied(Box::new(record));
        self.persist_add_goal(&command.metadata, &command.goal, &outcome)
    }

    fn persist_add_goal(
        &self,
        metadata: &GoalCommandMetadata,
        goal: &Goal,
        applied_outcome: &GoalCommandOutcome,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        let command_key = metadata.command_id.as_bytes().to_vec();
        let goal_key = goal.goal_id.as_bytes().to_vec();
        let goal_id = goal.goal_id.clone();
        let command_id = metadata.command_id.clone();
        let source_identity_key = metadata
            .source_identity
            .as_ref()
            .map(|source_identity| source_identity.as_bytes().to_vec());
        let source_identity_value = goal.goal_id.as_bytes().to_vec();
        let record = match applied_outcome {
            GoalCommandOutcome::Applied(record) => record,
            _ => {
                return Err(ExecutionInvariantError::ConfigError(
                    "add goal transaction requires applied outcome".to_string(),
                ));
            }
        };
        let record_value = serde_json::to_vec(record).map_err(to_store_data)?;
        let applied_value = serde_json::to_vec(applied_outcome).map_err(to_store_data)?;
        let applied_outcome = applied_outcome.clone();

        (
            &self.records,
            &self.source_identity_index,
            &self.command_outcomes,
        )
            .transaction(|(records, source_identity_index, command_outcomes)| {
                if let Some(raw) = command_outcomes.get(command_key.clone())? {
                    return serde_json::from_slice(&raw).map_err(to_transaction_data);
                }

                if let Some(raw) = records.get(goal_key.clone())? {
                    let existing: ExecutionGoalRecord =
                        serde_json::from_slice(&raw).map_err(to_transaction_data)?;
                    let outcome =
                        if existing.source_command_id.as_deref() == Some(command_id.as_str()) {
                            if let Some(source_identity_key) = source_identity_key.clone() {
                                source_identity_index
                                    .insert(source_identity_key, source_identity_value.clone())?;
                            }
                            GoalCommandOutcome::Applied(Box::new(existing))
                        } else {
                            GoalCommandOutcome::Duplicate {
                                existing_goal_id: goal_id.clone(),
                            }
                        };
                    command_outcomes.insert(
                        command_key.clone(),
                        serde_json::to_vec(&outcome).map_err(to_transaction_data)?,
                    )?;
                    return Ok(outcome);
                }

                if let Some(source_identity_key) = source_identity_key.clone() {
                    if let Some(raw) = source_identity_index.get(source_identity_key)? {
                        let existing_goal_id =
                            String::from_utf8(raw.to_vec()).map_err(to_transaction_utf8)?;
                        let outcome = GoalCommandOutcome::Duplicate { existing_goal_id };
                        command_outcomes.insert(
                            command_key.clone(),
                            serde_json::to_vec(&outcome).map_err(to_transaction_data)?,
                        )?;
                        return Ok(outcome);
                    }
                }

                records.insert(goal_key.clone(), record_value.clone())?;
                if let Some(source_identity_key) = source_identity_key.clone() {
                    source_identity_index
                        .insert(source_identity_key, source_identity_value.clone())?;
                }
                command_outcomes.insert(command_key.clone(), applied_value.clone())?;
                Ok(applied_outcome.clone())
            })
            .map_err(to_goal_transaction)
    }

    /// Replace an existing goal while preserving creation sequence and dedupe state.
    pub fn modify_goal(
        &self,
        command: ModifyGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        if let Some(outcome) = self.replayed_outcome(&command.metadata)? {
            return Ok(outcome);
        }
        validate_metadata(&command.metadata)?;
        validate_goal(&command.goal)?;

        let Some(existing) = self.record(&command.goal.goal_id)? else {
            let outcome = GoalCommandOutcome::NotFound {
                goal_id: command.goal.goal_id.clone(),
            };
            self.record_outcome(&command.metadata, &outcome)?;
            return Ok(outcome);
        };
        if let Some(existing_goal_id) =
            self.duplicate_source_identity_for_other_goal(&command.metadata, &command.goal.goal_id)?
        {
            let outcome = GoalCommandOutcome::Duplicate { existing_goal_id };
            self.record_outcome(&command.metadata, &outcome)?;
            return Ok(outcome);
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
        self.put_record(&record)?;
        self.reindex_source_identity(
            previous_source_identity,
            source_identity,
            &command.goal.goal_id,
        )?;
        let outcome = GoalCommandOutcome::Applied(Box::new(record));
        self.record_outcome(&command.metadata, &outcome)?;
        Ok(outcome)
    }

    /// Apply an idempotent transition to abandoned.
    pub fn remove_goal(
        &self,
        command: RemoveGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        if let Some(outcome) = self.replayed_outcome(&command.metadata)? {
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
        &self,
        command: SatisfyGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        if let Some(outcome) = self.replayed_outcome(&command.metadata)? {
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
        &self,
        command: SuspendGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        if let Some(outcome) = self.replayed_outcome(&command.metadata)? {
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
        &self,
        command: ResumeGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        if let Some(outcome) = self.replayed_outcome(&command.metadata)? {
            return Ok(outcome);
        }
        validate_metadata(&command.metadata)?;
        self.update_lifecycle(&command.metadata, &command.goal_id, GoalLifecycle::Active)
    }

    /// Return active goals in deterministic goal id order.
    pub fn active_goals(&self) -> Result<Vec<Goal>, ExecutionInvariantError> {
        Ok(self
            .records()?
            .into_iter()
            .filter(|record| matches!(record.goal.lifecycle, GoalLifecycle::Active))
            .map(|record| record.goal)
            .collect())
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

    /// Flush durable writes to the backing database.
    pub fn flush(&self) -> Result<(), ExecutionInvariantError> {
        self.db.flush().map_err(to_store_io)?;
        Ok(())
    }

    fn update_lifecycle(
        &self,
        metadata: &GoalCommandMetadata,
        goal_id: &str,
        lifecycle: GoalLifecycle,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        validate_non_empty("goal id", goal_id)?;
        let Some(existing) = self.record(goal_id)? else {
            let outcome = GoalCommandOutcome::NotFound {
                goal_id: goal_id.to_string(),
            };
            self.record_outcome(metadata, &outcome)?;
            return Ok(outcome);
        };
        let mut record = existing;
        record.goal.lifecycle = lifecycle;
        record.source_command_id = Some(metadata.command_id.clone());
        record.updated_at_seq = metadata.seq;
        self.put_record(&record)?;
        let outcome = GoalCommandOutcome::Applied(Box::new(record));
        self.record_outcome(metadata, &outcome)?;
        Ok(outcome)
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

    fn put_record(&self, record: &ExecutionGoalRecord) -> Result<(), ExecutionInvariantError> {
        self.records
            .insert(
                record.goal.goal_id.as_bytes(),
                serde_json::to_vec(record).map_err(to_store_data)?,
            )
            .map_err(to_store_io)?;
        Ok(())
    }

    fn replayed_outcome(
        &self,
        metadata: &GoalCommandMetadata,
    ) -> Result<Option<GoalCommandOutcome>, ExecutionInvariantError> {
        decode_optional(
            self.command_outcomes
                .get(metadata.command_id.as_bytes())
                .map_err(to_store_io)?,
        )
    }

    fn record_outcome(
        &self,
        metadata: &GoalCommandMetadata,
        outcome: &GoalCommandOutcome,
    ) -> Result<(), ExecutionInvariantError> {
        self.command_outcomes
            .insert(
                metadata.command_id.as_bytes(),
                serde_json::to_vec(outcome).map_err(to_store_data)?,
            )
            .map_err(to_store_io)?;
        Ok(())
    }

    fn duplicate_source_identity_for_other_goal(
        &self,
        metadata: &GoalCommandMetadata,
        goal_id: &str,
    ) -> Result<Option<String>, ExecutionInvariantError> {
        Ok(metadata
            .source_identity
            .as_ref()
            .map(|source_identity| self.source_identity_goal_id(source_identity))
            .transpose()?
            .flatten()
            .filter(|existing_goal_id| existing_goal_id.as_str() != goal_id))
    }

    fn source_identity_goal_id(
        &self,
        source_identity: &str,
    ) -> Result<Option<String>, ExecutionInvariantError> {
        let Some(raw) = self
            .source_identity_index
            .get(source_identity.as_bytes())
            .map_err(to_store_io)?
        else {
            return Ok(None);
        };
        Ok(Some(
            String::from_utf8(raw.to_vec()).map_err(to_store_utf8)?,
        ))
    }

    fn reindex_source_identity(
        &self,
        previous: Option<String>,
        current: Option<String>,
        goal_id: &str,
    ) -> Result<(), ExecutionInvariantError> {
        if previous != current {
            if let Some(previous) = previous {
                self.source_identity_index
                    .remove(previous.as_bytes())
                    .map_err(to_store_io)?;
            }
        }
        if let Some(current) = current {
            self.source_identity_index
                .insert(current.as_bytes(), goal_id.as_bytes())
                .map_err(to_store_io)?;
        }
        Ok(())
    }
}

fn decode_optional<T: serde::de::DeserializeOwned>(
    raw: Option<sled::IVec>,
) -> Result<Option<T>, ExecutionInvariantError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&raw).map_err(to_store_data)?))
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

fn to_transaction_data(
    err: serde_json::Error,
) -> ConflictableTransactionError<ExecutionInvariantError> {
    ConflictableTransactionError::Abort(to_store_data(err))
}

fn to_transaction_utf8(
    err: std::string::FromUtf8Error,
) -> ConflictableTransactionError<ExecutionInvariantError> {
    ConflictableTransactionError::Abort(to_store_utf8(err))
}

fn to_goal_transaction(err: TransactionError<ExecutionInvariantError>) -> ExecutionInvariantError {
    match err {
        TransactionError::Abort(error) => error,
        TransactionError::Storage(error) => to_store_io(error),
    }
}
