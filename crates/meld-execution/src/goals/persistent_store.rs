//! Durable goal set storage for execution-owned goal lifecycle state.

use crate::error::ExecutionInvariantError;
use crate::goals::contracts::{
    AddGoalCommand, ExecutionGoalRecord, GoalCommandMetadata, GoalCommandOutcome,
    ModifyGoalCommand, RemoveGoalCommand, ResumeGoalCommand, SatisfyGoalCommand,
    SuspendGoalCommand,
};
use crate::goals::store::{validate_goal, validate_metadata, validate_non_empty};
use meld_lang::{Goal, GoalLifecycle};
use sled::{Db, Tree};
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

    /// Add a new goal or return an idempotent duplicate outcome.
    pub fn add_goal(
        &self,
        command: AddGoalCommand,
    ) -> Result<GoalCommandOutcome, ExecutionInvariantError> {
        if let Some(outcome) = self.replayed_outcome(&command.metadata)? {
            return Ok(outcome);
        }
        validate_metadata(&command.metadata)?;
        validate_goal(&command.goal)?;

        if let Some(existing_goal_id) = self.duplicate_goal_id(&command.metadata, &command.goal)? {
            let outcome = GoalCommandOutcome::Duplicate { existing_goal_id };
            self.record_outcome(&command.metadata, &outcome)?;
            return Ok(outcome);
        }

        let record = ExecutionGoalRecord {
            goal: command.goal.clone(),
            source_command_id: Some(command.metadata.command_id.clone()),
            source_identity: command.metadata.source_identity.clone(),
            created_at_seq: command.metadata.seq,
            updated_at_seq: command.metadata.seq,
        };
        self.put_record(&record)?;
        self.index_source_identity(&command.metadata, &command.goal.goal_id)?;
        let outcome = GoalCommandOutcome::Applied(Box::new(record));
        self.record_outcome(&command.metadata, &outcome)?;
        Ok(outcome)
    }

    /// Replace an existing goal record while preserving its creation sequence.
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

    /// Mark a goal abandoned.
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

    /// Mark a goal satisfied.
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

    /// Suspend a goal.
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

    /// Resume a goal into active state.
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

    fn duplicate_goal_id(
        &self,
        metadata: &GoalCommandMetadata,
        goal: &Goal,
    ) -> Result<Option<String>, ExecutionInvariantError> {
        if self.record(&goal.goal_id)?.is_some() {
            return Ok(Some(goal.goal_id.clone()));
        }
        metadata
            .source_identity
            .as_ref()
            .map(|source_identity| self.source_identity_goal_id(source_identity))
            .transpose()
            .map(Option::flatten)
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

    fn index_source_identity(
        &self,
        metadata: &GoalCommandMetadata,
        goal_id: &str,
    ) -> Result<(), ExecutionInvariantError> {
        if let Some(source_identity) = &metadata.source_identity {
            self.source_identity_index
                .insert(source_identity.as_bytes(), goal_id.as_bytes())
                .map_err(to_store_io)?;
        }
        Ok(())
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
