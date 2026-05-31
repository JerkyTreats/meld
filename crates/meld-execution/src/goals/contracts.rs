//! Public records for execution goal curation and lifecycle commands.

use serde::{Deserialize, Serialize};

/// Execution-owned wrapper around a shared-language goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionGoalRecord {
    /// Shared-language desired state owned by the execution goal set.
    pub goal: meld_lang::Goal,
    /// Last command that created or changed this record.
    pub source_command_id: Option<String>,
    /// Optional producer identity used to dedupe retries across command ids.
    pub source_identity: Option<String>,
    /// Monotonic sequence observed when the record was created.
    pub created_at_seq: u64,
    /// Monotonic sequence observed when the record was last changed.
    pub updated_at_seq: u64,
}

/// Common metadata attached to idempotent goal commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalCommandMetadata {
    /// Stable command id used for exact outcome replay.
    pub command_id: String,
    /// Optional producer scoped identity used to dedupe semantically identical goals.
    pub source_identity: Option<String>,
    /// Caller supplied ordering sequence used in lifecycle records.
    pub seq: u64,
}

/// Add a new execution goal record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddGoalCommand {
    /// Idempotency and ordering metadata.
    pub metadata: GoalCommandMetadata,
    /// Goal to insert into execution-owned storage.
    pub goal: meld_lang::Goal,
}

/// Replace an existing execution goal record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModifyGoalCommand {
    /// Idempotency and ordering metadata.
    pub metadata: GoalCommandMetadata,
    /// Complete replacement goal with the same goal id.
    pub goal: meld_lang::Goal,
}

/// Abandon a goal without removing its record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveGoalCommand {
    /// Idempotency and ordering metadata.
    pub metadata: GoalCommandMetadata,
    /// Goal to mark abandoned.
    pub goal_id: String,
    /// Non-empty abandonment reason stored in the lifecycle.
    pub reason: String,
}

/// Mark a goal satisfied at a known sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SatisfyGoalCommand {
    /// Idempotency and ordering metadata.
    pub metadata: GoalCommandMetadata,
    /// Goal to mark satisfied.
    pub goal_id: String,
    /// Sequence at which satisfaction was established by the caller.
    pub at_seq: u64,
}

/// Suspend a goal while retaining its record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuspendGoalCommand {
    /// Idempotency and ordering metadata.
    pub metadata: GoalCommandMetadata,
    /// Goal to mark suspended.
    pub goal_id: String,
    /// Non-empty suspension reason stored in the lifecycle.
    pub reason: String,
}

/// Resume a suspended goal into active planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeGoalCommand {
    /// Idempotency and ordering metadata.
    pub metadata: GoalCommandMetadata,
    /// Goal to mark active again.
    pub goal_id: String,
}

/// Deterministic outcome for goal curation commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GoalCommandOutcome {
    Applied(Box<ExecutionGoalRecord>),
    Duplicate { existing_goal_id: String },
    NotFound { goal_id: String },
}
