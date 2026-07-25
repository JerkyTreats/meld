//! Public records for execution goal curation and lifecycle commands.

use serde::{Deserialize, Serialize};

/// Execution-owned wrapper around a shared-language goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionGoalRecord {
    /// Shared-language desired state owned by the execution goal set.
    pub goal: meld_lang::Goal,
    /// Last command that created or changed this record.
    pub source_command_id: Option<String>,
    /// Non-empty producer identity used to dedupe retries across command ids.
    pub source_identity: Option<String>,
    /// Lifecycle epoch of the stable goal identity. Reopening a satisfied
    /// goal advances the epoch in place; satisfaction evidence binds the
    /// epoch it was produced under, so satisfied is never an absorbing
    /// state. Zero on records that predate epochs.
    #[serde(default)]
    pub lifecycle_epoch: u64,
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
    /// Optional non-empty producer identity for deduping semantically identical goals.
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
    /// Lifecycle epoch the satisfaction decision observed. Satisfaction
    /// evidence binds its epoch: stores must treat a mismatch with the
    /// record's current epoch as a stale no-op, never a satisfied
    /// transition. Zero on commands that predate epochs.
    #[serde(default)]
    pub lifecycle_epoch: u64,
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

/// Reopen a satisfied goal in place after later drift.
///
/// The frozen future-drift rule: the same goal identity transitions from
/// satisfied to active through an idempotent agent-curated reopen appended
/// as a new goal revision with the lifecycle epoch advanced. Prior-epoch
/// satisfaction evidence never satisfies the reopened goal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReopenGoalCommand {
    /// Idempotency and ordering metadata.
    pub metadata: GoalCommandMetadata,
    /// Satisfied goal to reopen under an advanced epoch.
    pub goal_id: String,
    /// Belief revision whose drift triggered the reopen, as provenance.
    pub triggering_belief_revision_id: String,
}

/// Deterministic outcome for goal curation commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GoalCommandOutcome {
    /// Goal command was applied and returned the current record.
    Applied(Box<ExecutionGoalRecord>),
    /// Goal command was ignored because an equivalent goal already exists.
    Duplicate {
        /// Existing goal that caused idempotent duplicate handling.
        existing_goal_id: String,
    },
    /// Goal command targeted a goal that does not exist in the store.
    NotFound {
        /// Goal identifier that could not be found.
        goal_id: String,
    },
    /// Goal command was ignored as a deterministic stale no-op.
    ///
    /// The record is left byte-identical: no lifecycle transition, no epoch
    /// change, and no `updated_at_seq` advance. Stores return this instead of
    /// applying a satisfy whose observed epoch mismatches the record, or a
    /// reopen of a goal that is not satisfied.
    StaleNoOp {
        /// Goal whose record was left unchanged.
        goal_id: String,
        /// Machine-readable reason the command did not apply.
        reason: StaleGoalCommandReason,
    },
}

/// Reason a goal command resolved to a deterministic stale no-op.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StaleGoalCommandReason {
    /// Command observed a lifecycle epoch other than the record's current
    /// epoch. Satisfaction evidence binds the epoch it was produced under,
    /// so a prior-epoch satisfy can never transition a reopened goal.
    EpochMismatch {
        /// Epoch the command observed when its evidence was produced.
        command_epoch: u64,
        /// Epoch currently carried by the goal record.
        current_epoch: u64,
    },
    /// Reopen targeted a goal whose lifecycle is not satisfied. Reopen only
    /// moves satisfied goals back to active; it never advances the epoch of
    /// an already-active, suspended, proposed, or abandoned goal.
    LifecycleNotSatisfied,
}
