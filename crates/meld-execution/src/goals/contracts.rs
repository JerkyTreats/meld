//! Public records for execution goal curation and lifecycle commands.

use serde::{Deserialize, Serialize};

/// Stable command kind included in conflict-aware request identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalCommandKind {
    /// Insert a new goal.
    Add,
    /// Replace an existing goal.
    Modify,
    /// Abandon an existing goal.
    Remove,
    /// Mark an existing goal satisfied.
    Satisfy,
    /// Suspend an existing goal.
    Suspend,
    /// Resume an existing goal.
    Resume,
}

/// Canonical identity for one complete goal command request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalCommandRequestIdentity {
    /// Stable id used to replay the persisted command outcome.
    pub command_id: String,
    /// Command variant included in the canonical request hash.
    pub command_kind: GoalCommandKind,
    /// Digest of the complete normalized command request.
    pub request_hash: String,
}

// TODO compat-shim: remove this policy after the minimum supported goal-store
// schema requires request identities beside every command outcome. It preserves
// verified replay for outcomes written before request hashes existed. Before
// deletion, keep compatible_legacy_applied_outcome_is_verified_and_upgraded and
// strict_legacy_policy_rejects_unverified_outcome green while proving all
// supported stores have completed the identity upgrade.
/// Crate-owned compatibility posture for outcomes written before request hashes existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LegacyGoalCommandReplayPolicy {
    /// Treat the prior outcome as unverified and reject replay.
    RejectUnverified,
    /// Reconstruct the old request only when durable source data is sufficient.
    ReconstructAndVerify,
}

/// Durable acknowledgement returned only after the full command boundary flushes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalCommandCommitReceipt {
    /// Request identity committed beside the outcome.
    pub identity: GoalCommandRequestIdentity,
    /// Digest of the persisted command outcome.
    pub outcome_hash: String,
}

/// Complete goal command request that can derive its own canonical identity.
pub trait GoalCommandRequestContract: Serialize {
    /// Return immutable command metadata.
    fn metadata(&self) -> &GoalCommandMetadata;

    /// Return the command variant included in request identity.
    fn command_kind(&self) -> GoalCommandKind;

    /// Derive identity from this complete canonical command product.
    fn request_identity(&self) -> Result<GoalCommandRequestIdentity, String> {
        let encoded = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        Ok(GoalCommandRequestIdentity {
            command_id: self.metadata().command_id.clone(),
            command_kind: self.command_kind(),
            request_hash: blake3::hash(&encoded).to_hex().to_string(),
        })
    }
}

/// Execution-owned wrapper around a shared-language goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionGoalRecord {
    /// Shared-language desired state owned by the execution goal set.
    pub goal: meld_lang::Goal,
    /// Last command that created or changed this record.
    pub source_command_id: Option<String>,
    /// Non-empty producer identity used to dedupe retries across command ids.
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

macro_rules! impl_goal_command_request_contract {
    ($command:ty, $kind:expr) => {
        impl GoalCommandRequestContract for $command {
            fn metadata(&self) -> &GoalCommandMetadata {
                &self.metadata
            }

            fn command_kind(&self) -> GoalCommandKind {
                $kind
            }
        }
    };
}

impl_goal_command_request_contract!(AddGoalCommand, GoalCommandKind::Add);
impl_goal_command_request_contract!(ModifyGoalCommand, GoalCommandKind::Modify);
impl_goal_command_request_contract!(RemoveGoalCommand, GoalCommandKind::Remove);
impl_goal_command_request_contract!(SatisfyGoalCommand, GoalCommandKind::Satisfy);
impl_goal_command_request_contract!(SuspendGoalCommand, GoalCommandKind::Suspend);
impl_goal_command_request_contract!(ResumeGoalCommand, GoalCommandKind::Resume);

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
}

#[cfg(test)]
mod contract_freeze_tests {
    use super::*;

    #[test]
    fn goal_command_identity_and_commit_receipt_round_trip() {
        let receipt = GoalCommandCommitReceipt {
            identity: GoalCommandRequestIdentity {
                command_id: "command-a".to_string(),
                command_kind: GoalCommandKind::Modify,
                request_hash: "blake3:request-a".to_string(),
            },
            outcome_hash: "blake3:outcome-a".to_string(),
        };

        let encoded = serde_json::to_vec(&receipt).unwrap();
        let decoded: GoalCommandCommitReceipt = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, receipt);
        let policy =
            serde_json::to_string(&LegacyGoalCommandReplayPolicy::RejectUnverified).unwrap();
        assert_eq!(policy, "\"reject_unverified\"");
    }

    #[test]
    fn complete_command_payload_derives_conflict_aware_identity() {
        let metadata = GoalCommandMetadata {
            command_id: "command-a".to_string(),
            source_identity: None,
            seq: 7,
        };
        let first = RemoveGoalCommand {
            metadata: metadata.clone(),
            goal_id: "goal-a".to_string(),
            reason: "first reason".to_string(),
        };
        let second = RemoveGoalCommand {
            metadata,
            goal_id: "goal-a".to_string(),
            reason: "changed reason".to_string(),
        };

        assert_ne!(
            first.request_identity().unwrap().request_hash,
            second.request_identity().unwrap().request_hash
        );
    }
}
