//! Sled-backed append-only planning attempt authority.

use std::collections::BTreeMap;
use std::ops::Bound::{Excluded, Unbounded};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, Weak};

#[cfg(test)]
use std::io;

use serde::de::DeserializeOwned;
use serde::Serialize;
use sled::transaction::{ConflictableTransactionError, TransactionError, Transactional};
use sled::{Db, Tree};
use thiserror::Error;

use crate::planning::attempts::contracts::{
    PlanningAttemptCommandOutcome, PlanningAttemptContinuation, PlanningAttemptDecisionAudit,
    PlanningAttemptHead, PlanningAttemptHistory, PlanningAttemptIdentity,
    PlanningAttemptOwnerFence, PlanningAttemptRecord, PlanningAttemptRecordKind,
    PlanningAttemptRecovery, PlanningAttemptRecoverySelection, PlanningAttemptSelection,
    PlanningAttemptState, PlanningAttemptTerminalDiagnostic, PlanningPreparedCommand,
    MAX_PLANNING_ATTEMPT_QUERY_ITEMS,
};
use crate::task_network::command;

const TREE_RECORDS: &str = "execution_planning_attempt_records";
const TREE_HEADS: &str = "execution_planning_attempt_heads";
const TREE_GOAL_INDEX: &str = "execution_planning_attempt_goal_index";
const TREE_RECOVERY_INDEX: &str = "execution_planning_attempt_recovery_index";
const TREE_OWNER: &str = "execution_planning_attempt_owner";
const KEY_SCHEMA: &[u8] = b"__schema";
const KEY_ACTIVE_OWNER: &[u8] = b"active";
const SCHEMA_V1: &[u8] = b"execution_planning_attempts.v1";
const GOAL_INDEX_HASH_DOMAIN: &[u8] = b"meld.execution.planning-attempt-goal-index.v1";

static DURABLE_VISIBILITY_GATES: OnceLock<Mutex<BTreeMap<String, Weak<Mutex<()>>>>> =
    OnceLock::new();

/// Retry posture for one planning attempt storage failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanningAttemptErrorClass {
    /// The same operation may succeed after transient storage or contention clears.
    Retryable,
    /// The input or durable state requires correction before retry.
    Fatal,
}

/// Typed planning attempt authority storage failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlanningAttemptStorageError {
    /// Caller input violated a durable planning attempt contract.
    #[error("planning attempt input is invalid: {0}")]
    InvalidInput(String),
    /// An idempotency key was replayed with divergent intent.
    #[error("planning attempt identity conflicts with durable state: {0}")]
    IdentityConflict(String),
    /// A compare-and-set fence changed during an append.
    #[error("planning attempt storage is contended: {0}")]
    Backpressure(String),
    /// The caller no longer owns the durable planning actor generation.
    #[error("planning attempt owner is stale: {0}")]
    StaleOwner(String),
    /// The durable store could not serve an operation.
    #[error("planning attempt storage is unavailable: {0}")]
    Unavailable(String),
    /// A transaction committed but its flush acknowledgement failed.
    #[error("planning attempt durability is indeterminate: {0}")]
    DurabilityIndeterminate(String),
    /// Durable records or indexes violate the append-only authority invariants.
    #[error("planning attempt storage is corrupt: {0}")]
    Corrupt(String),
    /// Durable schema markers do not agree with this runtime.
    #[error("planning attempt schema conflicts with this runtime: {0}")]
    SchemaConflict(String),
}

impl PlanningAttemptStorageError {
    /// Return the stable retry posture for this failure.
    pub fn class(&self) -> PlanningAttemptErrorClass {
        match self {
            Self::Backpressure(_) | Self::Unavailable(_) | Self::DurabilityIndeterminate(_) => {
                PlanningAttemptErrorClass::Retryable
            }
            Self::InvalidInput(_)
            | Self::IdentityConflict(_)
            | Self::StaleOwner(_)
            | Self::Corrupt(_)
            | Self::SchemaConflict(_) => PlanningAttemptErrorClass::Fatal,
        }
    }

    /// Return true when retry is safe without changing caller intent.
    pub fn is_retryable(&self) -> bool {
        self.class() == PlanningAttemptErrorClass::Retryable
    }
}

#[derive(Debug, Clone)]
enum TransactionAbort {
    Conflict(String),
    Retry(String),
    StaleOwner(String),
    Corrupt(String),
}

#[cfg(test)]
#[derive(Default)]
struct FlushProbe {
    flushes_before_failure: Option<usize>,
}

/// Execution-owned append-only planning attempt audit store.
///
/// Each mutating method commits an immutable record, derived head, and owned
/// selector indexes in one transaction, then flushes before acknowledgement.
#[derive(Clone)]
pub struct PlanningAttemptStore {
    db: Db,
    records: Tree,
    heads: Tree,
    goal_index: Tree,
    recovery_index: Tree,
    owner: Tree,
    durable_visibility_gate: Arc<Mutex<()>>,
    bound_owner: Option<PlanningAttemptOwnerFence>,
    #[cfg(test)]
    flush_probe: Arc<Mutex<FlushProbe>>,
}

impl PlanningAttemptStore {
    /// Open and fully validate the planning attempt authority.
    pub fn new(db: Db) -> Result<Self, PlanningAttemptStorageError> {
        let durable_visibility_gate = shared_durable_visibility_gate(&db)?;
        let store = Self {
            records: db.open_tree(TREE_RECORDS).map_err(to_unavailable)?,
            heads: db.open_tree(TREE_HEADS).map_err(to_unavailable)?,
            goal_index: db.open_tree(TREE_GOAL_INDEX).map_err(to_unavailable)?,
            recovery_index: db.open_tree(TREE_RECOVERY_INDEX).map_err(to_unavailable)?,
            owner: db.open_tree(TREE_OWNER).map_err(to_unavailable)?,
            durable_visibility_gate,
            bound_owner: None,
            #[cfg(test)]
            flush_probe: Arc::new(Mutex::new(FlushProbe::default())),
            db,
        };
        let _visibility = store.lock_durable_visibility()?;
        store.ensure_schema()?;
        store.validate_reopen()?;
        drop(_visibility);
        Ok(store)
    }

    /// Open the validated authority behind a shared pointer.
    pub fn shared(db: Db) -> Result<Arc<Self>, PlanningAttemptStorageError> {
        Ok(Arc::new(Self::new(db)?))
    }

    /// Bind a new planning writer to the exact current owner fence.
    ///
    /// The expected fence prevents two replacement actors from successively
    /// stealing ownership after they raced on the same prior generation.
    pub fn bind_owner(
        &self,
        expected: Option<&PlanningAttemptOwnerFence>,
        runtime_id: impl Into<String>,
        lease_id: impl Into<String>,
    ) -> Result<Self, PlanningAttemptStorageError> {
        let _visibility = self.lock_durable_visibility()?;
        if let Some(expected) = expected {
            expected
                .validate()
                .map_err(PlanningAttemptStorageError::InvalidInput)?;
        }
        let runtime_id = runtime_id.into();
        let lease_id = lease_id.into();
        let candidate = if expected
            .is_some_and(|fence| fence.runtime_id() == runtime_id && fence.lease_id() == lease_id)
        {
            expected.cloned().expect("matching owner fence exists")
        } else {
            let epoch = expected.map_or(Ok(1), |fence| {
                fence.epoch().checked_add(1).ok_or_else(|| {
                    PlanningAttemptStorageError::InvalidInput(
                        "planning owner epoch is exhausted".to_string(),
                    )
                })
            })?;
            PlanningAttemptOwnerFence::issued(runtime_id, lease_id, epoch)
                .map_err(PlanningAttemptStorageError::InvalidInput)?
        };
        let expected_bytes = expected.map(encode).transpose()?;
        let candidate_bytes = encode(&candidate)?;
        self.owner
            .transaction(|owner| {
                let current = owner.get(KEY_ACTIVE_OWNER)?;
                if current.as_deref() == Some(candidate_bytes.as_slice()) {
                    return Ok(());
                }
                if current.as_deref() != expected_bytes.as_deref() {
                    return Err(ConflictableTransactionError::Abort(
                        TransactionAbort::StaleOwner(
                            "planning owner changed before actor activation".to_string(),
                        ),
                    ));
                }
                owner.insert(KEY_ACTIVE_OWNER, candidate_bytes.as_slice())?;
                Ok(())
            })
            .map_err(map_transaction_error)?;
        self.flush_durable("planning owner activation")?;
        let mut bound = self.clone();
        bound.bound_owner = Some(candidate);
        Ok(bound)
    }

    /// Read the current durable planning owner fence.
    pub fn active_owner_fence(
        &self,
    ) -> Result<Option<PlanningAttemptOwnerFence>, PlanningAttemptStorageError> {
        let _visibility = self.query_visibility_guard()?;
        let owner: Option<PlanningAttemptOwnerFence> =
            decode_optional(self.owner.get(KEY_ACTIVE_OWNER).map_err(to_unavailable)?)?;
        if let Some(owner) = &owner {
            owner
                .validate()
                .map_err(PlanningAttemptStorageError::Corrupt)?;
        }
        Ok(owner)
    }

    /// Revalidate this writer capability immediately before an external commit.
    pub fn validate_owner_fence(&self) -> Result<(), PlanningAttemptStorageError> {
        let _visibility = self.query_visibility_guard()?;
        let owner_bytes = encode(self.require_bound_owner()?)?;
        self.validate_bound_owner(&owner_bytes)
    }

    /// Borrow the owner capability captured by this writer handle.
    pub fn bound_owner_fence(&self) -> Option<&PlanningAttemptOwnerFence> {
        self.bound_owner.as_ref()
    }

    /// Durably establish one exact immutable planning attempt identity.
    pub fn open_attempt(
        &self,
        identity: PlanningAttemptIdentity,
    ) -> Result<PlanningAttemptHead, PlanningAttemptStorageError> {
        let _visibility = self.lock_durable_visibility()?;
        let owner_bytes = encode(self.require_bound_owner()?)?;
        identity
            .validate()
            .map_err(PlanningAttemptStorageError::InvalidInput)?;
        if let Some(existing) = self.get_head_without_visibility_barrier(identity.attempt_id())? {
            self.validate_open_replay(&existing, &identity)?;
            self.validate_bound_owner(&owner_bytes)?;
            self.flush_durable("planning attempt opening replay")?;
            return Ok(existing);
        }

        let record = PlanningAttemptRecord::opened(identity.clone())
            .map_err(PlanningAttemptStorageError::InvalidInput)?;
        let head = PlanningAttemptHead::from_opened(&record)
            .map_err(PlanningAttemptStorageError::InvalidInput)?;
        let record_key = record_key(identity.attempt_id(), 1);
        let goal_key = goal_index_key(&identity);
        let record_bytes = encode(&record)?;
        let head_bytes = encode(&head)?;
        let attempt_id = identity.attempt_id().as_bytes().to_vec();

        let transaction = (&self.records, &self.heads, &self.goal_index, &self.owner).transaction(
            |(records, heads, goal_index, owner)| {
                require_transaction_owner(owner, &owner_bytes)?;
                if let Some(existing) = heads.get(attempt_id.as_slice())? {
                    if existing.as_ref() == head_bytes.as_slice()
                        && records.get(record_key.as_slice())?.as_deref()
                            == Some(record_bytes.as_slice())
                        && goal_index.get(goal_key.as_slice())?.as_deref()
                            == Some(attempt_id.as_slice())
                    {
                        return Ok(());
                    }
                    return Err(ConflictableTransactionError::Abort(
                        TransactionAbort::Conflict(
                            "attempt opening replay differs from durable state".to_string(),
                        ),
                    ));
                }
                if records.get(record_key.as_slice())?.is_some()
                    || goal_index.get(goal_key.as_slice())?.is_some()
                {
                    return Err(ConflictableTransactionError::Abort(
                        TransactionAbort::Corrupt(
                            "attempt indexes exist without a durable head".to_string(),
                        ),
                    ));
                }
                records.insert(record_key.as_slice(), record_bytes.as_slice())?;
                heads.insert(attempt_id.as_slice(), head_bytes.as_slice())?;
                goal_index.insert(goal_key.as_slice(), attempt_id.as_slice())?;
                Ok(())
            },
        );
        if let Err(error) = transaction {
            let mapped = map_transaction_error(error);
            if matches!(mapped, PlanningAttemptStorageError::IdentityConflict(_)) {
                if let Some(existing) =
                    self.get_head_without_visibility_barrier(identity.attempt_id())?
                {
                    self.validate_open_replay(&existing, &identity)?;
                    self.validate_bound_owner(&owner_bytes)?;
                    self.flush_durable("concurrent planning attempt opening replay")?;
                    return Ok(existing);
                }
            }
            return Err(mapped);
        }
        self.flush_durable("planning attempt opening")?;
        self.get_head_without_visibility_barrier(identity.attempt_id())?
            .ok_or_else(|| {
                PlanningAttemptStorageError::Corrupt(
                    "planning attempt opening committed without a durable head".to_string(),
                )
            })
    }

    /// Durably append the bounded non-authoritative decision audit.
    pub fn record_decision(
        &self,
        attempt_id: &str,
        decision: PlanningAttemptDecisionAudit,
    ) -> Result<PlanningAttemptHead, PlanningAttemptStorageError> {
        let _visibility = self.lock_durable_visibility()?;
        let owner_bytes = encode(self.require_bound_owner()?)?;
        decision
            .validate()
            .map_err(PlanningAttemptStorageError::InvalidInput)?;
        let head = self.require_head(attempt_id)?;
        validate_decision_source(&head.identity, &decision)
            .map_err(PlanningAttemptStorageError::InvalidInput)?;
        if head.state != PlanningAttemptState::Open {
            let durable = self.require_decision(attempt_id)?;
            if durable != decision {
                return Err(PlanningAttemptStorageError::IdentityConflict(
                    "planning decision replay differs from durable state".to_string(),
                ));
            }
            self.validate_bound_owner(&owner_bytes)?;
            self.flush_durable("planning decision audit replay")?;
            return Ok(head);
        }
        self.append_record(
            &head,
            PlanningAttemptRecordKind::DecisionAudited { decision },
        )
    }

    /// Durably terminate an audited attempt with one bounded diagnostic.
    pub fn record_terminal_diagnostic(
        &self,
        attempt_id: &str,
        diagnostic: PlanningAttemptTerminalDiagnostic,
    ) -> Result<PlanningAttemptHead, PlanningAttemptStorageError> {
        let _visibility = self.lock_durable_visibility()?;
        let owner_bytes = encode(self.require_bound_owner()?)?;
        diagnostic
            .validate()
            .map_err(PlanningAttemptStorageError::InvalidInput)?;
        let head = self.require_head(attempt_id)?;
        if head.state == PlanningAttemptState::DiagnosticTerminal {
            let existing = self.require_record(attempt_id, 3)?;
            let PlanningAttemptRecordKind::TerminalDiagnostic {
                diagnostic: durable,
            } = existing.kind
            else {
                return Err(PlanningAttemptStorageError::Corrupt(
                    "terminal attempt head does not name a diagnostic record".to_string(),
                ));
            };
            if durable != diagnostic {
                return Err(PlanningAttemptStorageError::IdentityConflict(
                    "terminal diagnostic replay differs from durable state".to_string(),
                ));
            }
            self.validate_bound_owner(&owner_bytes)?;
            self.flush_durable("planning terminal diagnostic replay")?;
            return Ok(head);
        }
        if head.state != PlanningAttemptState::DecisionAudited {
            return Err(PlanningAttemptStorageError::IdentityConflict(
                "planning attempt requires a durable decision audit before termination".to_string(),
            ));
        }
        let decision = self.require_decision(attempt_id)?;
        if decision.result_summary().terminal_disposition() != Some(diagnostic.disposition) {
            return Err(PlanningAttemptStorageError::InvalidInput(
                "planning terminal diagnostic conflicts with the decision result".to_string(),
            ));
        }
        self.append_record(
            &head,
            PlanningAttemptRecordKind::TerminalDiagnostic { diagnostic },
        )
    }

    /// Durably prepare the exact command before any submission is permitted.
    pub fn prepare_command(
        &self,
        attempt_id: &str,
        request: command::Request,
    ) -> Result<PlanningAttemptHead, PlanningAttemptStorageError> {
        let _visibility = self.lock_durable_visibility()?;
        let owner_bytes = encode(self.require_bound_owner()?)?;
        let head = self.require_head(attempt_id)?;
        let prepared = PlanningPreparedCommand::bind(&head.identity, request)
            .map_err(PlanningAttemptStorageError::InvalidInput)?;
        if matches!(
            head.state,
            PlanningAttemptState::CommandPrepared | PlanningAttemptState::CommandCompleted
        ) {
            let durable = self.require_prepared(attempt_id)?;
            if durable != prepared {
                return Err(PlanningAttemptStorageError::IdentityConflict(
                    "prepared command replay differs from durable state".to_string(),
                ));
            }
            self.validate_bound_owner(&owner_bytes)?;
            self.flush_durable("prepared planning command replay")?;
            return Ok(head);
        }
        if head.state != PlanningAttemptState::DecisionAudited {
            return Err(PlanningAttemptStorageError::IdentityConflict(
                "planning attempt requires a composed decision before command preparation"
                    .to_string(),
            ));
        }
        let decision = self.require_decision(attempt_id)?;
        let command::Command::ApplyMutationSet(set) = &prepared.request.command else {
            unreachable!("prepared planning commands are validated as mutation sets");
        };
        if !matches!(
            decision.result_summary(),
            crate::planning::attempts::contracts::PlanningAttemptResultSummary::Composed { .. }
        ) || decision.result_summary().composition_id() != Some(&set.source_composition_id)
        {
            return Err(PlanningAttemptStorageError::InvalidInput(
                "prepared command does not match the composed planning decision".to_string(),
            ));
        }
        self.append_record(
            &head,
            PlanningAttemptRecordKind::CommandPrepared {
                prepared: Box::new(prepared),
            },
        )
    }

    /// Durably bind a terminal response to the exact prepared command.
    pub fn record_command_outcome(
        &self,
        attempt_id: &str,
        receipt: command::OutcomeReceipt,
    ) -> Result<PlanningAttemptHead, PlanningAttemptStorageError> {
        let _visibility = self.lock_durable_visibility()?;
        let owner_bytes = encode(self.require_bound_owner()?)?;
        let head = self.require_head(attempt_id)?;
        if head.state == PlanningAttemptState::CommandCompleted {
            let prepared = self.require_prepared(attempt_id)?;
            let outcome = PlanningAttemptCommandOutcome::bind(&prepared, receipt)
                .map_err(PlanningAttemptStorageError::InvalidInput)?;
            let durable = self.require_command_outcome(attempt_id)?;
            if durable != outcome {
                return Err(PlanningAttemptStorageError::IdentityConflict(
                    "planning command outcome replay differs from durable state".to_string(),
                ));
            }
            self.validate_bound_owner(&owner_bytes)?;
            self.flush_durable("planning command outcome replay")?;
            return Ok(head);
        }
        if head.state != PlanningAttemptState::CommandPrepared {
            return Err(PlanningAttemptStorageError::IdentityConflict(
                "planning command outcome requires a durable prepared command".to_string(),
            ));
        }
        let prepared = self.require_prepared(attempt_id)?;
        let outcome = PlanningAttemptCommandOutcome::bind(&prepared, receipt)
            .map_err(PlanningAttemptStorageError::InvalidInput)?;
        self.append_record(&head, PlanningAttemptRecordKind::CommandOutcome { outcome })
    }

    /// Flush every pending planning attempt write.
    pub fn flush(&self) -> Result<(), PlanningAttemptStorageError> {
        #[cfg(test)]
        {
            let mut probe = self.flush_probe.lock().expect("flush probe lock");
            if let Some(remaining) = probe.flushes_before_failure {
                if remaining == 0 {
                    probe.flushes_before_failure = None;
                    return Err(PlanningAttemptStorageError::Unavailable(
                        io::Error::other("injected planning attempt flush failure").to_string(),
                    ));
                }
                probe.flushes_before_failure = Some(remaining - 1);
            }
        }
        self.db.flush().map_err(to_unavailable)?;
        Ok(())
    }

    pub(crate) fn get_head(
        &self,
        attempt_id: &str,
    ) -> Result<Option<PlanningAttemptHead>, PlanningAttemptStorageError> {
        let _visibility = self.query_visibility_guard()?;
        self.get_head_without_visibility_barrier(attempt_id)
    }

    fn get_head_without_visibility_barrier(
        &self,
        attempt_id: &str,
    ) -> Result<Option<PlanningAttemptHead>, PlanningAttemptStorageError> {
        decode_optional(
            self.heads
                .get(attempt_id.as_bytes())
                .map_err(to_unavailable)?,
        )
    }

    pub(crate) fn history_bounded(
        &self,
        attempt_id: &str,
        after_ordinal: Option<u32>,
        max_items: usize,
    ) -> Result<PlanningAttemptHistory, PlanningAttemptStorageError> {
        let _visibility = self.query_visibility_guard()?;
        validate_query_limit(max_items)?;
        let prefix = format!("{attempt_id}::");
        let after = after_ordinal.unwrap_or(0);
        let mut records = Vec::new();
        for item in self.records.scan_prefix(prefix.as_bytes()) {
            let (_, raw) = item.map_err(to_unavailable)?;
            let record: PlanningAttemptRecord = decode(&raw)?;
            if record.ordinal > after {
                records.push(record);
                if records.len() > max_items {
                    break;
                }
            }
        }
        let budget_exhausted = records.len() > max_items;
        records.truncate(max_items);
        Ok(PlanningAttemptHistory {
            records,
            budget_exhausted,
        })
    }

    pub(crate) fn attempts_for_goal_bounded(
        &self,
        goal_id: &str,
        continuation: Option<&PlanningAttemptContinuation>,
        max_items: usize,
    ) -> Result<PlanningAttemptSelection, PlanningAttemptStorageError> {
        let _visibility = self.query_visibility_guard()?;
        validate_query_limit(max_items)?;
        if goal_id.trim().is_empty() {
            return Err(PlanningAttemptStorageError::InvalidInput(
                "planning attempt goal selector must be non-empty".to_string(),
            ));
        }
        let prefix = goal_index_prefix(goal_id);
        let goal_hash = prefix.trim_end_matches("::");
        let after_key = continuation
            .map(|continuation| continuation.validate_goal(goal_hash, &prefix))
            .transpose()
            .map_err(PlanningAttemptStorageError::InvalidInput)?;
        let mut attempts = Vec::new();
        let mut keys = Vec::new();
        let iterator = if let Some(after_key) = after_key {
            self.goal_index
                .range((Excluded(after_key.as_bytes().to_vec()), Unbounded))
        } else {
            self.goal_index.scan_prefix(prefix.as_bytes())
        };
        for item in iterator {
            let (key, attempt_id) = item.map_err(to_unavailable)?;
            if !key.starts_with(prefix.as_bytes()) {
                break;
            }
            let attempt_id = std::str::from_utf8(attempt_id.as_ref()).map_err(|error| {
                PlanningAttemptStorageError::Corrupt(format!(
                    "planning attempt goal index contains invalid identity bytes: {error}"
                ))
            })?;
            attempts.push(self.require_head(attempt_id)?);
            keys.push(String::from_utf8(key.to_vec()).map_err(|error| {
                PlanningAttemptStorageError::Corrupt(format!(
                    "planning attempt goal index key is not UTF-8: {error}"
                ))
            })?);
            if attempts.len() > max_items {
                break;
            }
        }
        let budget_exhausted = attempts.len() > max_items;
        attempts.truncate(max_items);
        keys.truncate(max_items);
        let continuation = if budget_exhausted {
            Some(
                PlanningAttemptContinuation::for_goal(
                    goal_hash.to_string(),
                    keys.last().cloned().ok_or_else(|| {
                        PlanningAttemptStorageError::Corrupt(
                            "exhausted planning selector has no continuation key".to_string(),
                        )
                    })?,
                )
                .map_err(PlanningAttemptStorageError::Corrupt)?,
            )
        } else {
            None
        };
        Ok(PlanningAttemptSelection {
            attempts,
            budget_exhausted,
            continuation,
        })
    }

    pub(crate) fn recoverable_commands_bounded(
        &self,
        continuation: Option<&PlanningAttemptContinuation>,
        max_items: usize,
    ) -> Result<PlanningAttemptRecoverySelection, PlanningAttemptStorageError> {
        let _visibility = self.query_visibility_guard()?;
        validate_query_limit(max_items)?;
        let after_key = continuation
            .map(PlanningAttemptContinuation::validate_recovery)
            .transpose()
            .map_err(PlanningAttemptStorageError::InvalidInput)?;
        let mut recoveries = Vec::new();
        let mut keys = Vec::new();
        let iterator = if let Some(after_key) = after_key {
            self.recovery_index
                .range((Excluded(after_key.as_bytes().to_vec()), Unbounded))
        } else {
            self.recovery_index.scan_prefix(b"prepared::")
        };
        for item in iterator {
            let (key, attempt_id) = item.map_err(to_unavailable)?;
            if !key.starts_with(b"prepared::") {
                break;
            }
            let attempt_id = std::str::from_utf8(attempt_id.as_ref()).map_err(|error| {
                PlanningAttemptStorageError::Corrupt(format!(
                    "planning recovery index contains invalid identity bytes: {error}"
                ))
            })?;
            let head = self.require_head(attempt_id)?;
            if head.state != PlanningAttemptState::CommandPrepared {
                return Err(PlanningAttemptStorageError::Corrupt(
                    "planning recovery index names a non-prepared attempt".to_string(),
                ));
            }
            recoveries.push(PlanningAttemptRecovery {
                prepared: self.require_prepared(attempt_id)?,
                head,
            });
            keys.push(String::from_utf8(key.to_vec()).map_err(|error| {
                PlanningAttemptStorageError::Corrupt(format!(
                    "planning recovery index key is not UTF-8: {error}"
                ))
            })?);
            if recoveries.len() > max_items {
                break;
            }
        }
        let budget_exhausted = recoveries.len() > max_items;
        recoveries.truncate(max_items);
        keys.truncate(max_items);
        let continuation = if budget_exhausted {
            Some(
                PlanningAttemptContinuation::for_recovery(keys.last().cloned().ok_or_else(
                    || {
                        PlanningAttemptStorageError::Corrupt(
                            "exhausted planning recovery has no continuation key".to_string(),
                        )
                    },
                )?)
                .map_err(PlanningAttemptStorageError::Corrupt)?,
            )
        } else {
            None
        };
        Ok(PlanningAttemptRecoverySelection {
            recoveries,
            budget_exhausted,
            continuation,
        })
    }

    pub(crate) fn terminal_diagnostic(
        &self,
        attempt_id: &str,
    ) -> Result<Option<PlanningAttemptTerminalDiagnostic>, PlanningAttemptStorageError> {
        let _visibility = self.query_visibility_guard()?;
        let Some(head) = self.get_head_without_visibility_barrier(attempt_id)? else {
            return Ok(None);
        };
        if head.state != PlanningAttemptState::DiagnosticTerminal {
            return Ok(None);
        }
        let record = self.require_record(attempt_id, 3)?;
        match record.kind {
            PlanningAttemptRecordKind::TerminalDiagnostic { diagnostic } => Ok(Some(diagnostic)),
            _ => Err(PlanningAttemptStorageError::Corrupt(
                "terminal diagnostic head points to the wrong record kind".to_string(),
            )),
        }
    }

    pub(crate) fn prepared_command(
        &self,
        attempt_id: &str,
    ) -> Result<Option<PlanningPreparedCommand>, PlanningAttemptStorageError> {
        let _visibility = self.query_visibility_guard()?;
        let Some(head) = self.get_head_without_visibility_barrier(attempt_id)? else {
            return Ok(None);
        };
        if !matches!(
            head.state,
            PlanningAttemptState::CommandPrepared | PlanningAttemptState::CommandCompleted
        ) {
            return Ok(None);
        }
        self.require_prepared(attempt_id).map(Some)
    }

    pub(crate) fn decision_audit(
        &self,
        attempt_id: &str,
    ) -> Result<Option<PlanningAttemptDecisionAudit>, PlanningAttemptStorageError> {
        let _visibility = self.query_visibility_guard()?;
        let Some(head) = self.get_head_without_visibility_barrier(attempt_id)? else {
            return Ok(None);
        };
        if head.state == PlanningAttemptState::Open {
            return Ok(None);
        }
        self.require_decision(attempt_id).map(Some)
    }

    pub(crate) fn command_outcome(
        &self,
        attempt_id: &str,
    ) -> Result<Option<PlanningAttemptCommandOutcome>, PlanningAttemptStorageError> {
        let _visibility = self.query_visibility_guard()?;
        let Some(head) = self.get_head_without_visibility_barrier(attempt_id)? else {
            return Ok(None);
        };
        if head.state != PlanningAttemptState::CommandCompleted {
            return Ok(None);
        }
        self.require_command_outcome(attempt_id).map(Some)
    }

    fn append_record(
        &self,
        head: &PlanningAttemptHead,
        kind: PlanningAttemptRecordKind,
    ) -> Result<PlanningAttemptHead, PlanningAttemptStorageError> {
        let owner_bytes = encode(self.require_bound_owner()?)?;
        let record = PlanningAttemptRecord::successor(head, kind)
            .map_err(PlanningAttemptStorageError::InvalidInput)?;
        let decision = (head.state != PlanningAttemptState::Open)
            .then(|| self.require_decision(head.identity.attempt_id()))
            .transpose()?;
        let prepared = matches!(
            head.state,
            PlanningAttemptState::CommandPrepared | PlanningAttemptState::CommandCompleted
        )
        .then(|| self.require_prepared(head.identity.attempt_id()))
        .transpose()?;
        validate_record_against_chain(head, &record, decision.as_ref(), prepared.as_ref())?;
        let advanced = head
            .advance(&record)
            .map_err(PlanningAttemptStorageError::InvalidInput)?;
        let attempt_id = head.identity.attempt_id().as_bytes().to_vec();
        let expected_head = encode(head)?;
        let advanced_head = encode(&advanced)?;
        let record_bytes = encode(&record)?;
        let record_key = record_key(head.identity.attempt_id(), record.ordinal);
        let recovery_key = recovery_index_key(&head.identity);

        let transaction = (
            &self.records,
            &self.heads,
            &self.recovery_index,
            &self.owner,
        )
            .transaction(|(records, heads, recovery_index, owner)| {
                require_transaction_owner(owner, &owner_bytes)?;
                if heads.get(attempt_id.as_slice())?.as_deref() != Some(expected_head.as_slice()) {
                    return Err(ConflictableTransactionError::Abort(
                        TransactionAbort::Retry(
                            "planning attempt head changed during append".to_string(),
                        ),
                    ));
                }
                if records.get(record_key.as_slice())?.is_some() {
                    return Err(ConflictableTransactionError::Abort(
                        TransactionAbort::Corrupt(
                            "planning attempt append ordinal already exists".to_string(),
                        ),
                    ));
                }
                match advanced.state {
                    PlanningAttemptState::CommandPrepared => {
                        if recovery_index.get(recovery_key.as_slice())?.is_some() {
                            return Err(ConflictableTransactionError::Abort(
                                TransactionAbort::Corrupt(
                                    "planning recovery index already contains the attempt"
                                        .to_string(),
                                ),
                            ));
                        }
                        recovery_index.insert(recovery_key.as_slice(), attempt_id.as_slice())?;
                    }
                    PlanningAttemptState::CommandCompleted => {
                        if recovery_index.get(recovery_key.as_slice())?.as_deref()
                            != Some(attempt_id.as_slice())
                        {
                            return Err(ConflictableTransactionError::Abort(
                                TransactionAbort::Corrupt(
                                    "prepared planning attempt is missing its recovery index"
                                        .to_string(),
                                ),
                            ));
                        }
                        recovery_index.remove(recovery_key.as_slice())?;
                    }
                    PlanningAttemptState::Open
                    | PlanningAttemptState::DecisionAudited
                    | PlanningAttemptState::DiagnosticTerminal => {}
                }
                records.insert(record_key.as_slice(), record_bytes.as_slice())?;
                heads.insert(attempt_id.as_slice(), advanced_head.as_slice())?;
                Ok(())
            });
        if let Err(error) = transaction {
            let mapped = map_transaction_error(error);
            if matches!(mapped, PlanningAttemptStorageError::Backpressure(_)) {
                let durable = self.require_record(head.identity.attempt_id(), record.ordinal)?;
                if durable == record {
                    let current = self.require_head(head.identity.attempt_id())?;
                    self.flush_durable("concurrent planning attempt lifecycle replay")?;
                    return Ok(current);
                }
                return Err(PlanningAttemptStorageError::IdentityConflict(
                    "concurrent planning attempt transition differs from durable state".to_string(),
                ));
            }
            return Err(mapped);
        }
        self.flush_durable("planning attempt lifecycle append")?;
        Ok(advanced)
    }

    fn validate_open_replay(
        &self,
        head: &PlanningAttemptHead,
        identity: &PlanningAttemptIdentity,
    ) -> Result<(), PlanningAttemptStorageError> {
        if &head.identity != identity {
            return Err(PlanningAttemptStorageError::IdentityConflict(
                "planning attempt opening identity differs from durable state".to_string(),
            ));
        }
        let opened = self.require_record(identity.attempt_id(), 1)?;
        match opened.kind {
            PlanningAttemptRecordKind::Opened { identity: durable } if &durable == identity => {}
            _ => {
                return Err(PlanningAttemptStorageError::Corrupt(
                    "planning attempt head is missing its exact opening record".to_string(),
                ))
            }
        }
        let goal_key = goal_index_key(identity);
        if self
            .goal_index
            .get(goal_key)
            .map_err(to_unavailable)?
            .as_deref()
            != Some(identity.attempt_id().as_bytes())
        {
            return Err(PlanningAttemptStorageError::Corrupt(
                "planning attempt is missing its goal index".to_string(),
            ));
        }
        Ok(())
    }

    fn require_head(
        &self,
        attempt_id: &str,
    ) -> Result<PlanningAttemptHead, PlanningAttemptStorageError> {
        self.get_head_without_visibility_barrier(attempt_id)?
            .ok_or_else(|| {
                PlanningAttemptStorageError::InvalidInput(format!(
                    "unknown planning attempt '{attempt_id}'"
                ))
            })
    }

    fn require_record(
        &self,
        attempt_id: &str,
        ordinal: u32,
    ) -> Result<PlanningAttemptRecord, PlanningAttemptStorageError> {
        decode_optional(
            self.records
                .get(record_key(attempt_id, ordinal))
                .map_err(to_unavailable)?,
        )?
        .ok_or_else(|| {
            PlanningAttemptStorageError::Corrupt(format!(
                "planning attempt '{attempt_id}' is missing record {ordinal}"
            ))
        })
    }

    fn require_prepared(
        &self,
        attempt_id: &str,
    ) -> Result<PlanningPreparedCommand, PlanningAttemptStorageError> {
        let record = self.require_record(attempt_id, 3)?;
        match record.kind {
            PlanningAttemptRecordKind::CommandPrepared { prepared } => Ok(*prepared),
            _ => Err(PlanningAttemptStorageError::Corrupt(
                "prepared planning attempt points to the wrong record kind".to_string(),
            )),
        }
    }

    fn require_command_outcome(
        &self,
        attempt_id: &str,
    ) -> Result<PlanningAttemptCommandOutcome, PlanningAttemptStorageError> {
        let record = self.require_record(attempt_id, 4)?;
        match record.kind {
            PlanningAttemptRecordKind::CommandOutcome { outcome } => Ok(outcome),
            _ => Err(PlanningAttemptStorageError::Corrupt(
                "completed planning attempt points to the wrong record kind".to_string(),
            )),
        }
    }

    fn require_decision(
        &self,
        attempt_id: &str,
    ) -> Result<PlanningAttemptDecisionAudit, PlanningAttemptStorageError> {
        let record = self.require_record(attempt_id, 2)?;
        match record.kind {
            PlanningAttemptRecordKind::DecisionAudited { decision } => Ok(decision),
            _ => Err(PlanningAttemptStorageError::Corrupt(
                "advanced planning attempt points to the wrong decision record kind".to_string(),
            )),
        }
    }

    fn ensure_schema(&self) -> Result<(), PlanningAttemptStorageError> {
        let trees = [
            &self.records,
            &self.heads,
            &self.goal_index,
            &self.recovery_index,
            &self.owner,
        ];
        let markers = trees
            .iter()
            .map(|tree| tree.get(KEY_SCHEMA).map_err(to_unavailable))
            .collect::<Result<Vec<_>, _>>()?;
        let present = markers.iter().filter(|marker| marker.is_some()).count();
        if present == trees.len() {
            if markers
                .iter()
                .all(|marker| marker.as_deref() == Some(SCHEMA_V1))
            {
                return Ok(());
            }
            return Err(PlanningAttemptStorageError::SchemaConflict(
                "planning attempt tree schema markers disagree".to_string(),
            ));
        }
        if present != 0 {
            return Err(PlanningAttemptStorageError::SchemaConflict(
                "planning attempt tree schema markers are incomplete".to_string(),
            ));
        }
        if trees.iter().any(|tree| !tree.is_empty()) {
            return Err(PlanningAttemptStorageError::SchemaConflict(
                "unversioned planning attempt data cannot be opened".to_string(),
            ));
        }
        (
            &self.records,
            &self.heads,
            &self.goal_index,
            &self.recovery_index,
            &self.owner,
        )
            .transaction(|(records, heads, goal_index, recovery_index, owner)| {
                for tree in [records, heads, goal_index, recovery_index, owner] {
                    if tree.get(KEY_SCHEMA)?.is_some() {
                        return Err(ConflictableTransactionError::Abort(
                            TransactionAbort::Retry(
                                "planning attempt schema changed during initialization".to_string(),
                            ),
                        ));
                    }
                    tree.insert(KEY_SCHEMA, SCHEMA_V1)?;
                }
                Ok(())
            })
            .map_err(map_transaction_error)?;
        self.flush_durable("planning attempt schema initialization")
    }

    fn validate_reopen(&self) -> Result<(), PlanningAttemptStorageError> {
        validate_owner_tree(&self.owner)?;
        let mut chains = BTreeMap::<String, BTreeMap<u32, PlanningAttemptRecord>>::new();
        for item in &self.records {
            let (key, raw) = item.map_err(to_unavailable)?;
            if key.as_ref() == KEY_SCHEMA {
                continue;
            }
            let record: PlanningAttemptRecord = decode(&raw)?;
            record
                .validate_shape()
                .map_err(PlanningAttemptStorageError::Corrupt)?;
            if key.as_ref() != record_key(&record.attempt_id, record.ordinal).as_slice() {
                return Err(PlanningAttemptStorageError::Corrupt(
                    "planning attempt record key conflicts with embedded identity".to_string(),
                ));
            }
            if encode(&record)?.as_slice() != raw.as_ref() {
                return Err(PlanningAttemptStorageError::Corrupt(
                    "planning attempt record is not canonically encoded".to_string(),
                ));
            }
            if chains
                .entry(record.attempt_id.clone())
                .or_default()
                .insert(record.ordinal, record)
                .is_some()
            {
                return Err(PlanningAttemptStorageError::Corrupt(
                    "planning attempt record ordinal is duplicated".to_string(),
                ));
            }
        }

        let mut expected_heads = BTreeMap::new();
        let mut expected_goal_index = BTreeMap::new();
        let mut expected_recovery_index = BTreeMap::new();
        for (attempt_id, records) in chains {
            let first = records.get(&1).ok_or_else(|| {
                PlanningAttemptStorageError::Corrupt(format!(
                    "planning attempt '{attempt_id}' has no opening record"
                ))
            })?;
            let mut head = PlanningAttemptHead::from_opened(first)
                .map_err(PlanningAttemptStorageError::Corrupt)?;
            let mut decision = None;
            let mut prepared = None;
            for expected_ordinal in 2..=records.len() as u32 {
                let record = records.get(&expected_ordinal).ok_or_else(|| {
                    PlanningAttemptStorageError::Corrupt(format!(
                        "planning attempt '{attempt_id}' has a record gap"
                    ))
                })?;
                validate_record_against_chain(&head, record, decision.as_ref(), prepared.as_ref())?;
                match &record.kind {
                    PlanningAttemptRecordKind::DecisionAudited { decision: audit } => {
                        decision = Some(audit.clone())
                    }
                    PlanningAttemptRecordKind::CommandPrepared { prepared: command } => {
                        prepared = Some(command.as_ref().clone());
                    }
                    PlanningAttemptRecordKind::Opened { .. }
                    | PlanningAttemptRecordKind::TerminalDiagnostic { .. }
                    | PlanningAttemptRecordKind::CommandOutcome { .. } => {}
                }
                head = head
                    .advance(record)
                    .map_err(PlanningAttemptStorageError::Corrupt)?;
            }
            let head_bytes = encode(&head)?;
            expected_heads.insert(attempt_id.as_bytes().to_vec(), head_bytes);
            expected_goal_index.insert(
                goal_index_key(&head.identity),
                attempt_id.as_bytes().to_vec(),
            );
            if head.state == PlanningAttemptState::CommandPrepared {
                expected_recovery_index.insert(
                    recovery_index_key(&head.identity),
                    attempt_id.as_bytes().to_vec(),
                );
            }
        }

        require_tree_parity(&self.heads, expected_heads, "head")?;
        require_tree_parity(&self.goal_index, expected_goal_index, "goal index")?;
        require_tree_parity(
            &self.recovery_index,
            expected_recovery_index,
            "recovery index",
        )?;
        Ok(())
    }

    fn require_bound_owner(
        &self,
    ) -> Result<&PlanningAttemptOwnerFence, PlanningAttemptStorageError> {
        self.bound_owner.as_ref().ok_or_else(|| {
            PlanningAttemptStorageError::StaleOwner(
                "planning writes require an activated owner fence".to_string(),
            )
        })
    }

    fn validate_bound_owner(&self, owner_bytes: &[u8]) -> Result<(), PlanningAttemptStorageError> {
        self.owner
            .transaction(|owner| require_transaction_owner(owner, owner_bytes))
            .map_err(map_transaction_error)
    }

    fn lock_durable_visibility(&self) -> Result<MutexGuard<'_, ()>, PlanningAttemptStorageError> {
        self.durable_visibility_gate.lock().map_err(|_| {
            PlanningAttemptStorageError::Unavailable(
                "planning attempt durable visibility gate is poisoned".to_string(),
            )
        })
    }

    fn query_visibility_guard(&self) -> Result<MutexGuard<'_, ()>, PlanningAttemptStorageError> {
        let guard = self.lock_durable_visibility()?;
        self.flush_durable("planning attempt query visibility")?;
        Ok(guard)
    }

    fn flush_durable(&self, product: &str) -> Result<(), PlanningAttemptStorageError> {
        self.flush().map_err(|error| {
            PlanningAttemptStorageError::DurabilityIndeterminate(format!(
                "{product} flush failed: {error}"
            ))
        })
    }

    #[cfg(test)]
    fn fail_next_flush(&self) {
        self.flush_probe
            .lock()
            .expect("flush probe lock")
            .flushes_before_failure = Some(0);
    }
}

fn require_transaction_owner(
    owner: &sled::transaction::TransactionalTree,
    expected: &[u8],
) -> Result<(), ConflictableTransactionError<TransactionAbort>> {
    if owner.get(KEY_ACTIVE_OWNER)?.as_deref() != Some(expected) {
        return Err(ConflictableTransactionError::Abort(
            TransactionAbort::StaleOwner(
                "planning writer lost its durable owner fence".to_string(),
            ),
        ));
    }
    Ok(())
}

fn shared_durable_visibility_gate(db: &Db) -> Result<Arc<Mutex<()>>, PlanningAttemptStorageError> {
    let identity = format!("planning-attempt-live-db-{:p}", &*db.context.pagecache);
    let registry = DURABLE_VISIBILITY_GATES.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut registry = registry.lock().map_err(|_| {
        PlanningAttemptStorageError::Unavailable(
            "planning attempt visibility registry is poisoned".to_string(),
        )
    })?;
    registry.retain(|_, gate| gate.strong_count() > 0);
    if let Some(gate) = registry.get(&identity).and_then(Weak::upgrade) {
        return Ok(gate);
    }
    let gate = Arc::new(Mutex::new(()));
    registry.insert(identity, Arc::downgrade(&gate));
    Ok(gate)
}

fn validate_owner_tree(owner: &Tree) -> Result<(), PlanningAttemptStorageError> {
    for item in owner {
        let (key, raw) = item.map_err(to_unavailable)?;
        match key.as_ref() {
            KEY_SCHEMA => {}
            KEY_ACTIVE_OWNER => {
                let fence: PlanningAttemptOwnerFence = decode(&raw)?;
                fence
                    .validate()
                    .map_err(PlanningAttemptStorageError::Corrupt)?;
                if encode(&fence)?.as_slice() != raw.as_ref() {
                    return Err(PlanningAttemptStorageError::Corrupt(
                        "planning owner fence is not canonically encoded".to_string(),
                    ));
                }
            }
            _ => {
                return Err(PlanningAttemptStorageError::Corrupt(
                    "planning owner tree contains an unknown record".to_string(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_record_against_chain(
    head: &PlanningAttemptHead,
    record: &PlanningAttemptRecord,
    decision: Option<&PlanningAttemptDecisionAudit>,
    prepared: Option<&PlanningPreparedCommand>,
) -> Result<(), PlanningAttemptStorageError> {
    record
        .validate_shape()
        .map_err(PlanningAttemptStorageError::Corrupt)?;
    match &record.kind {
        PlanningAttemptRecordKind::Opened { .. } => Err(PlanningAttemptStorageError::Corrupt(
            "planning attempt chain contains a second opening record".to_string(),
        )),
        PlanningAttemptRecordKind::DecisionAudited { decision } => {
            decision
                .validate()
                .map_err(PlanningAttemptStorageError::Corrupt)?;
            validate_decision_source(&head.identity, decision)
                .map_err(PlanningAttemptStorageError::Corrupt)
        }
        PlanningAttemptRecordKind::TerminalDiagnostic { diagnostic } => {
            diagnostic
                .validate()
                .map_err(PlanningAttemptStorageError::Corrupt)?;
            let decision = decision.ok_or_else(|| {
                PlanningAttemptStorageError::Corrupt(
                    "planning terminal diagnostic has no decision audit".to_string(),
                )
            })?;
            if decision.result_summary().terminal_disposition() != Some(diagnostic.disposition) {
                return Err(PlanningAttemptStorageError::Corrupt(
                    "planning terminal diagnostic conflicts with its decision audit".to_string(),
                ));
            }
            Ok(())
        }
        PlanningAttemptRecordKind::CommandPrepared { prepared } => {
            let decision = decision.ok_or_else(|| {
                PlanningAttemptStorageError::Corrupt(
                    "prepared planning command has no decision audit".to_string(),
                )
            })?;
            let command::Command::ApplyMutationSet(set) = &prepared.request.command else {
                return Err(PlanningAttemptStorageError::Corrupt(
                    "prepared planning command is not a mutation set".to_string(),
                ));
            };
            if !matches!(
                decision.result_summary(),
                crate::planning::attempts::contracts::PlanningAttemptResultSummary::Composed { .. }
            ) || decision.result_summary().composition_id() != Some(&set.source_composition_id)
            {
                return Err(PlanningAttemptStorageError::Corrupt(
                    "prepared command conflicts with its composed planning decision".to_string(),
                ));
            }
            prepared
                .validate(&head.identity)
                .map_err(PlanningAttemptStorageError::Corrupt)
        }
        PlanningAttemptRecordKind::CommandOutcome { outcome } => {
            let prepared = prepared.ok_or_else(|| {
                PlanningAttemptStorageError::Corrupt(
                    "planning command outcome has no prepared command".to_string(),
                )
            })?;
            outcome
                .validate(prepared)
                .map_err(PlanningAttemptStorageError::Corrupt)
        }
    }
}

fn validate_decision_source(
    identity: &PlanningAttemptIdentity,
    decision: &PlanningAttemptDecisionAudit,
) -> Result<(), String> {
    let projection_failed = matches!(
        decision.result_summary(),
        crate::planning::attempts::contracts::PlanningAttemptResultSummary::ProjectionFailed
    );
    if identity.projection_failure().is_some() != projection_failed {
        return Err(
            "planning decision projection result conflicts with the attempt identity".to_string(),
        );
    }
    Ok(())
}

fn require_tree_parity(
    tree: &Tree,
    expected: BTreeMap<Vec<u8>, Vec<u8>>,
    label: &str,
) -> Result<(), PlanningAttemptStorageError> {
    let mut actual = BTreeMap::new();
    for item in tree {
        let (key, value) = item.map_err(to_unavailable)?;
        if key.as_ref() != KEY_SCHEMA {
            actual.insert(key.to_vec(), value.to_vec());
        }
    }
    if actual != expected {
        return Err(PlanningAttemptStorageError::Corrupt(format!(
            "planning attempt {label} parity failed"
        )));
    }
    Ok(())
}

fn validate_query_limit(max_items: usize) -> Result<(), PlanningAttemptStorageError> {
    if max_items == 0 || max_items > MAX_PLANNING_ATTEMPT_QUERY_ITEMS {
        return Err(PlanningAttemptStorageError::InvalidInput(format!(
            "planning attempt query limit must be between 1 and {MAX_PLANNING_ATTEMPT_QUERY_ITEMS}"
        )));
    }
    Ok(())
}

fn record_key(attempt_id: &str, ordinal: u32) -> Vec<u8> {
    format!("{attempt_id}::{ordinal:010}").into_bytes()
}

fn goal_index_prefix(goal_id: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(GOAL_INDEX_HASH_DOMAIN);
    hasher.update(goal_id.as_bytes());
    format!("{}::", hasher.finalize().to_hex())
}

fn goal_index_key(identity: &PlanningAttemptIdentity) -> Vec<u8> {
    format!(
        "{}{:020}::{}",
        goal_index_prefix(identity.goal_id()),
        identity.goal_updated_at_seq(),
        identity.attempt_id()
    )
    .into_bytes()
}

fn recovery_index_key(identity: &PlanningAttemptIdentity) -> Vec<u8> {
    format!(
        "prepared::{:020}::{}",
        identity.goal_updated_at_seq(),
        identity.attempt_id()
    )
    .into_bytes()
}

fn encode(value: &impl Serialize) -> Result<Vec<u8>, PlanningAttemptStorageError> {
    serde_json::to_vec(value).map_err(|error| {
        PlanningAttemptStorageError::InvalidInput(format!(
            "planning attempt value is not serializable: {error}"
        ))
    })
}

fn decode<T: DeserializeOwned>(raw: &[u8]) -> Result<T, PlanningAttemptStorageError> {
    serde_json::from_slice(raw).map_err(|error| {
        PlanningAttemptStorageError::Corrupt(format!(
            "planning attempt value cannot be decoded: {error}"
        ))
    })
}

fn decode_optional<T: DeserializeOwned>(
    raw: Option<sled::IVec>,
) -> Result<Option<T>, PlanningAttemptStorageError> {
    raw.map(|bytes| decode(bytes.as_ref())).transpose()
}

fn map_transaction_error(error: TransactionError<TransactionAbort>) -> PlanningAttemptStorageError {
    match error {
        TransactionError::Abort(TransactionAbort::Conflict(message)) => {
            PlanningAttemptStorageError::IdentityConflict(message)
        }
        TransactionError::Abort(TransactionAbort::Retry(message)) => {
            PlanningAttemptStorageError::Backpressure(message)
        }
        TransactionError::Abort(TransactionAbort::StaleOwner(message)) => {
            PlanningAttemptStorageError::StaleOwner(message)
        }
        TransactionError::Abort(TransactionAbort::Corrupt(message)) => {
            PlanningAttemptStorageError::Corrupt(message)
        }
        TransactionError::Storage(error) => to_unavailable(error),
    }
}

fn to_unavailable(error: impl ToString) -> PlanningAttemptStorageError {
    PlanningAttemptStorageError::Unavailable(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planning::attempts::contracts::{
        planning_task_network_command_id, PlanningAttemptDiagnosticDisposition,
        PlanningAttemptResultSummary, PlanningProjectionFailureIdentityInputs,
    };
    use crate::planning::world_state::canonical_world_state_hash;
    use crate::planning::{
        PlanningPerspectiveRef, PlanningProjectionIdentityInputs, PlanningRequestIdentity,
        PlanningRequestIdentityInputs, PlanningWorldStateFrameRef, PlanningWorldStateRequest,
    };
    use crate::task_network::mutation::Set;
    use meld_events::DomainObjectRef;
    use meld_lang::{Proposition, Term, WorldState};

    fn digest(label: &str) -> String {
        blake3::hash(label.as_bytes()).to_hex().to_string()
    }

    fn owned_store(db: Db) -> PlanningAttemptStore {
        let store = PlanningAttemptStore::new(db).unwrap();
        let current = store.active_owner_fence().unwrap();
        store
            .bind_owner(current.as_ref(), "execution.planning.runtime", "lease-test")
            .unwrap()
    }

    fn identity_named(goal_id: &str, goal_seq: u64, frame_id: &str) -> PlanningAttemptIdentity {
        let subject = DomainObjectRef::new("workspace", "node", "readme").unwrap();
        let request = PlanningWorldStateRequest {
            goal_id: goal_id.to_string(),
            agent_id: "agent-a".to_string(),
            subject: subject.clone(),
            source_seq: goal_seq,
            target: Proposition::Accessible {
                scope: Term::Object(subject),
            },
            perspective: PlanningPerspectiveRef::new("agent", "agent-a").unwrap(),
            branch_id: "main".to_string(),
            requested_dimensions: Vec::new(),
            required_preconditions: Vec::new(),
        };
        let world_state = WorldState::empty();
        let frame = PlanningWorldStateFrameRef::identified_from_authority(
            "planner.v1",
            digest(&format!("projection-{frame_id}")),
            canonical_world_state_hash(&world_state).unwrap(),
            &request,
            &world_state,
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let projection_identity =
            PlanningProjectionIdentityInputs::from_projection(&request, &world_state, &frame)
                .unwrap();
        let request = PlanningRequestIdentity::derive(PlanningRequestIdentityInputs {
            goal_id: goal_id.to_string(),
            goal_updated_at_seq: goal_seq,
            projection_identity,
            method_library_digest: digest("methods"),
            capability_catalog_digest: digest("capabilities"),
            planning_version: "execution.planning.v1".to_string(),
        })
        .unwrap();
        PlanningAttemptIdentity::bind(request).unwrap()
    }

    fn identity() -> PlanningAttemptIdentity {
        identity_named("goal-a", 7, "frame-a")
    }

    fn command(identity: &PlanningAttemptIdentity, composition_id: &str) -> command::Request {
        let network_id = "network-a";
        command::Request {
            command_id: planning_task_network_command_id(identity, composition_id).unwrap(),
            network_id: network_id.to_string(),
            base_revision: 0,
            base_state_hash: digest("empty-state"),
            read_preconditions: Vec::new(),
            command: command::Command::ApplyMutationSet(Set::empty(
                network_id,
                composition_id,
                identity.attempt_id(),
            )),
        }
    }

    fn response() -> command::Response {
        command::Response::Accepted {
            revision: 1,
            state_hash: digest("state-one"),
        }
    }

    fn composed_decision(composition_id: &str) -> PlanningAttemptDecisionAudit {
        PlanningAttemptDecisionAudit::new(
            Some("method-a".to_string()),
            Vec::new(),
            vec!["projection-warning-a".to_string()],
            Vec::new(),
            Vec::new(),
            PlanningAttemptResultSummary::Composed {
                composition_id: composition_id.to_string(),
            },
        )
        .unwrap()
    }

    fn diagnostic_decision(
        result_summary: PlanningAttemptResultSummary,
    ) -> PlanningAttemptDecisionAudit {
        PlanningAttemptDecisionAudit::new(
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            result_summary,
        )
        .unwrap()
    }

    fn receipt(request: &command::Request, response: command::Response) -> command::OutcomeReceipt {
        command::OutcomeReceipt::issued(
            request.command_id.clone(),
            command::request_hash(request),
            request.clone(),
            response,
        )
    }

    #[test]
    fn exact_command_lifecycle_replays_and_reopens() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("attempts");
        let identity = identity();
        let request = command(&identity, "composition-a");
        let completed;
        {
            let store = owned_store(sled::open(&path).unwrap());
            let opened = store.open_attempt(identity.clone()).unwrap();
            assert_eq!(store.open_attempt(identity.clone()).unwrap(), opened);
            let decision = composed_decision("composition-a");
            let audited = store
                .record_decision(identity.attempt_id(), decision.clone())
                .unwrap();
            assert_eq!(
                store
                    .record_decision(identity.attempt_id(), decision)
                    .unwrap(),
                audited
            );
            let prepared = store
                .prepare_command(identity.attempt_id(), request.clone())
                .unwrap();
            assert_eq!(prepared.state, PlanningAttemptState::CommandPrepared);
            assert_eq!(
                store
                    .prepare_command(identity.attempt_id(), request.clone())
                    .unwrap(),
                prepared
            );
            completed = store
                .record_command_outcome(identity.attempt_id(), receipt(&request, response()))
                .unwrap();
            assert_eq!(completed.state, PlanningAttemptState::CommandCompleted);
            assert_eq!(
                store
                    .record_command_outcome(identity.attempt_id(), receipt(&request, response()),)
                    .unwrap(),
                completed
            );
        }

        let store = owned_store(sled::open(&path).unwrap());
        assert_eq!(
            store.get_head(identity.attempt_id()).unwrap(),
            Some(completed)
        );
        assert!(store
            .recoverable_commands_bounded(None, 1)
            .unwrap()
            .recoveries
            .is_empty());
        assert_eq!(
            store
                .command_outcome(identity.attempt_id())
                .unwrap()
                .unwrap()
                .command_id,
            request.command_id
        );
        assert_eq!(
            store
                .history_bounded(identity.attempt_id(), None, 4)
                .unwrap()
                .records
                .len(),
            4
        );
    }

    #[test]
    fn terminal_diagnostic_is_queryable_and_rejects_divergence() {
        let store = owned_store(sled::Config::new().temporary(true).open().unwrap());
        let identity = identity();
        store.open_attempt(identity.clone()).unwrap();
        store
            .record_decision(
                identity.attempt_id(),
                diagnostic_decision(PlanningAttemptResultSummary::NoApplicableMethod),
            )
            .unwrap();
        let diagnostic = PlanningAttemptTerminalDiagnostic::new(
            PlanningAttemptDiagnosticDisposition::NoApplicableMethod,
            "no_applicable_method",
            "no verified method matched the projected state",
        )
        .unwrap();
        let terminal = store
            .record_terminal_diagnostic(identity.attempt_id(), diagnostic.clone())
            .unwrap();
        assert_eq!(terminal.state, PlanningAttemptState::DiagnosticTerminal);
        assert_eq!(
            store
                .record_terminal_diagnostic(identity.attempt_id(), diagnostic.clone())
                .unwrap(),
            terminal
        );
        assert_eq!(
            store.terminal_diagnostic(identity.attempt_id()).unwrap(),
            Some(diagnostic)
        );
        let divergent = PlanningAttemptTerminalDiagnostic::new(
            PlanningAttemptDiagnosticDisposition::Indeterminate,
            "indeterminate",
            "projection was incomplete",
        )
        .unwrap();
        assert!(matches!(
            store.record_terminal_diagnostic(identity.attempt_id(), divergent),
            Err(PlanningAttemptStorageError::IdentityConflict(_))
        ));
        assert!(matches!(
            store.prepare_command(identity.attempt_id(), command(&identity, "composition-a")),
            Err(PlanningAttemptStorageError::IdentityConflict(_))
        ));
    }

    #[test]
    fn divergent_prepared_command_replay_fails_closed() {
        let store = owned_store(sled::Config::new().temporary(true).open().unwrap());
        let identity = identity();
        store.open_attempt(identity.clone()).unwrap();
        store
            .record_decision(identity.attempt_id(), composed_decision("composition-a"))
            .unwrap();
        store
            .prepare_command(identity.attempt_id(), command(&identity, "composition-a"))
            .unwrap();

        assert!(matches!(
            store.prepare_command(identity.attempt_id(), command(&identity, "composition-b")),
            Err(PlanningAttemptStorageError::IdentityConflict(_))
        ));
    }

    #[test]
    fn out_of_order_command_outcome_reports_lifecycle_conflict() {
        let store = owned_store(sled::Config::new().temporary(true).open().unwrap());
        let identity = identity();
        let request = command(&identity, "composition-a");
        store.open_attempt(identity.clone()).unwrap();

        assert!(matches!(
            store.record_command_outcome(identity.attempt_id(), receipt(&request, response())),
            Err(PlanningAttemptStorageError::IdentityConflict(_))
        ));

        store
            .record_decision(identity.attempt_id(), composed_decision("composition-a"))
            .unwrap();
        assert!(matches!(
            store.record_command_outcome(identity.attempt_id(), receipt(&request, response())),
            Err(PlanningAttemptStorageError::IdentityConflict(_))
        ));
    }

    #[test]
    fn concurrent_exact_replays_converge_and_divergent_commands_fail_closed() {
        use std::sync::Barrier;
        use std::thread;

        let store = Arc::new(owned_store(
            sled::Config::new().temporary(true).open().unwrap(),
        ));
        let identity = identity();
        let barrier = Arc::new(Barrier::new(8));
        let mut opens = Vec::new();
        for _ in 0..8 {
            let store = Arc::clone(&store);
            let identity = identity.clone();
            let barrier = Arc::clone(&barrier);
            opens.push(thread::spawn(move || {
                barrier.wait();
                store.open_attempt(identity)
            }));
        }
        for result in opens {
            assert_eq!(
                result.join().unwrap().unwrap().identity,
                identity,
                "exact concurrent openings must converge"
            );
        }

        let request = command(&identity, "composition-a");
        store
            .record_decision(identity.attempt_id(), composed_decision("composition-a"))
            .unwrap();
        let barrier = Arc::new(Barrier::new(8));
        let mut prepares = Vec::new();
        for _ in 0..8 {
            let store = Arc::clone(&store);
            let request = request.clone();
            let attempt_id = identity.attempt_id().to_string();
            let barrier = Arc::clone(&barrier);
            prepares.push(thread::spawn(move || {
                barrier.wait();
                store.prepare_command(&attempt_id, request)
            }));
        }
        for result in prepares {
            assert_eq!(
                result.join().unwrap().unwrap().state,
                PlanningAttemptState::CommandPrepared
            );
        }

        let divergent_identity = identity_named("goal-b", 8, "frame-b");
        store.open_attempt(divergent_identity.clone()).unwrap();
        store
            .record_decision(
                divergent_identity.attempt_id(),
                composed_decision("composition-a"),
            )
            .unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let mut divergent = Vec::new();
        for base_state_hash in [digest("base-a"), digest("base-b")] {
            let store = Arc::clone(&store);
            let mut request = command(&divergent_identity, "composition-a");
            request.base_state_hash = base_state_hash;
            let attempt_id = divergent_identity.attempt_id().to_string();
            let barrier = Arc::clone(&barrier);
            divergent.push(thread::spawn(move || {
                barrier.wait();
                store.prepare_command(&attempt_id, request)
            }));
        }
        let results = divergent
            .into_iter()
            .map(|join| join.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(
                    result,
                    Err(PlanningAttemptStorageError::IdentityConflict(_))
                ))
                .count(),
            1
        );
    }

    #[test]
    fn prepared_command_is_not_visible_until_indeterminate_flush_is_resolved() {
        let store = owned_store(sled::Config::new().temporary(true).open().unwrap());
        let identity = identity();
        let request = command(&identity, "composition-a");
        store.open_attempt(identity.clone()).unwrap();
        store
            .record_decision(identity.attempt_id(), composed_decision("composition-a"))
            .unwrap();
        store.fail_next_flush();
        assert!(matches!(
            store.prepare_command(identity.attempt_id(), request.clone()),
            Err(PlanningAttemptStorageError::DurabilityIndeterminate(_))
        ));

        store.fail_next_flush();
        assert!(matches!(
            store.recoverable_commands_bounded(None, 1),
            Err(PlanningAttemptStorageError::DurabilityIndeterminate(_))
        ));
        assert_eq!(
            store
                .prepare_command(identity.attempt_id(), request.clone())
                .unwrap()
                .state,
            PlanningAttemptState::CommandPrepared
        );
        let recovery = store
            .recoverable_commands_bounded(None, 1)
            .unwrap()
            .recoveries;
        assert_eq!(recovery.len(), 1);
        assert_eq!(recovery[0].prepared.request, request);
    }

    #[test]
    fn recovery_query_cannot_cross_a_concurrent_prepare_flush_boundary() {
        use std::sync::mpsc;
        use std::thread;
        use std::time::Duration;

        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = Arc::new(owned_store(db.clone()));
        let query_store = Arc::new(PlanningAttemptStore::new(db).unwrap());
        assert!(Arc::ptr_eq(
            &store.durable_visibility_gate,
            &query_store.durable_visibility_gate
        ));
        let identity = identity();
        let request = command(&identity, "composition-a");
        store.open_attempt(identity.clone()).unwrap();
        store
            .record_decision(identity.attempt_id(), composed_decision("composition-a"))
            .unwrap();

        let mut flush_probe = store.flush_probe.lock().expect("flush probe lock");
        let prepare_store = Arc::clone(&store);
        let attempt_id = identity.attempt_id().to_string();
        let prepared_request = request.clone();
        let prepare =
            thread::spawn(move || prepare_store.prepare_command(&attempt_id, prepared_request));
        let recovery_key = recovery_index_key(&identity);
        while store.recovery_index.get(&recovery_key).unwrap().is_none() {
            thread::yield_now();
        }

        let (query_tx, query_rx) = mpsc::channel();
        let query = thread::spawn(move || {
            query_tx
                .send(query_store.recoverable_commands_bounded(None, 1))
                .unwrap();
        });
        assert!(query_rx.recv_timeout(Duration::from_millis(50)).is_err());

        flush_probe.flushes_before_failure = Some(0);
        drop(flush_probe);
        assert!(matches!(
            prepare.join().unwrap(),
            Err(PlanningAttemptStorageError::DurabilityIndeterminate(_))
        ));
        let recoveries = query_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap()
            .recoveries;
        query.join().unwrap();
        assert_eq!(recoveries.len(), 1);
        assert_eq!(recoveries[0].prepared.request, request);
    }

    #[test]
    fn accepted_and_duplicate_receipts_converge_under_concurrent_recovery() {
        use std::sync::Barrier;
        use std::thread;

        let store = Arc::new(owned_store(
            sled::Config::new().temporary(true).open().unwrap(),
        ));
        let identity = identity();
        let request = command(&identity, "composition-a");
        store.open_attempt(identity.clone()).unwrap();
        store
            .record_decision(identity.attempt_id(), composed_decision("composition-a"))
            .unwrap();
        store
            .prepare_command(identity.attempt_id(), request.clone())
            .unwrap();

        let barrier = Arc::new(Barrier::new(2));
        let mut writers = Vec::new();
        for response in [
            response(),
            command::Response::Duplicate {
                revision: 1,
                state_hash: digest("state-one"),
            },
        ] {
            let store = Arc::clone(&store);
            let barrier = Arc::clone(&barrier);
            let attempt_id = identity.attempt_id().to_string();
            let receipt = receipt(&request, response);
            writers.push(thread::spawn(move || {
                barrier.wait();
                store.record_command_outcome(&attempt_id, receipt)
            }));
        }
        for writer in writers {
            assert_eq!(
                writer.join().unwrap().unwrap().state,
                PlanningAttemptState::CommandCompleted
            );
        }
        assert!(matches!(
            store
                .command_outcome(identity.attempt_id())
                .unwrap()
                .unwrap()
                .response,
            command::Response::Accepted { .. }
        ));
    }

    #[test]
    fn outcome_receipt_must_name_the_exact_prepared_request() {
        let store = owned_store(sled::Config::new().temporary(true).open().unwrap());
        let identity = identity();
        let request = command(&identity, "composition-a");
        store.open_attempt(identity.clone()).unwrap();
        store
            .record_decision(identity.attempt_id(), composed_decision("composition-a"))
            .unwrap();
        store
            .prepare_command(identity.attempt_id(), request.clone())
            .unwrap();

        let mut wrong_request = request.clone();
        wrong_request.base_state_hash = digest("wrong-state");
        let wrong = command::OutcomeReceipt::issued(
            request.command_id.clone(),
            command::request_hash(&request),
            wrong_request,
            response(),
        );
        assert!(matches!(
            store.record_command_outcome(identity.attempt_id(), wrong),
            Err(PlanningAttemptStorageError::InvalidInput(_))
        ));
        assert_eq!(
            store
                .recoverable_commands_bounded(None, 1)
                .unwrap()
                .recoveries
                .len(),
            1
        );
    }

    #[test]
    fn recovery_continuation_bypasses_a_retryable_head_and_survives_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("recovery-pages");
        let identities = [
            identity_named("goal-a", 7, "frame-a"),
            identity_named("goal-b", 8, "frame-b"),
            identity_named("goal-c", 9, "frame-c"),
        ];
        let continuation;
        let first_attempt;
        {
            let store = owned_store(sled::open(&path).unwrap());
            for identity in &identities {
                let request = command(identity, "composition-a");
                store.open_attempt(identity.clone()).unwrap();
                store
                    .record_decision(identity.attempt_id(), composed_decision("composition-a"))
                    .unwrap();
                store
                    .prepare_command(identity.attempt_id(), request)
                    .unwrap();
            }
            let first = store.recoverable_commands_bounded(None, 1).unwrap();
            assert!(first.budget_exhausted);
            first_attempt = first.recoveries[0].head.identity.attempt_id().to_string();
            continuation = first.continuation.unwrap();
        }

        let store = owned_store(sled::open(&path).unwrap());
        let second = store
            .recoverable_commands_bounded(Some(&continuation), 1)
            .unwrap();
        assert_eq!(second.recoveries.len(), 1);
        assert_ne!(
            second.recoveries[0].head.identity.attempt_id(),
            first_attempt
        );
        assert_eq!(
            store
                .recoverable_commands_bounded(None, 1)
                .unwrap()
                .recoveries[0]
                .head
                .identity
                .attempt_id(),
            first_attempt
        );
    }

    #[test]
    fn selectors_and_histories_are_bounded_and_deterministic() {
        let store = owned_store(sled::Config::new().temporary(true).open().unwrap());
        let identities = [
            identity_named("goal-a", 9, "frame-c"),
            identity_named("goal-a", 7, "frame-a"),
            identity_named("goal-a", 8, "frame-b"),
        ];
        for identity in &identities {
            store.open_attempt(identity.clone()).unwrap();
        }
        let selected = store.attempts_for_goal_bounded("goal-a", None, 2).unwrap();
        assert!(selected.budget_exhausted);
        assert_eq!(
            selected
                .attempts
                .iter()
                .map(|head| head.identity.goal_updated_at_seq())
                .collect::<Vec<_>>(),
            vec![7, 8]
        );
        let next = store
            .attempts_for_goal_bounded("goal-a", selected.continuation.as_ref(), 2)
            .unwrap();
        assert!(!next.budget_exhausted);
        assert_eq!(
            next.attempts
                .iter()
                .map(|head| head.identity.goal_updated_at_seq())
                .collect::<Vec<_>>(),
            vec![9]
        );
        let continuation = selected.continuation.as_ref().unwrap();
        assert!(matches!(
            store.attempts_for_goal_bounded("goal-b", Some(continuation), 2),
            Err(PlanningAttemptStorageError::InvalidInput(_))
        ));
        let mut tampered = serde_json::to_value(continuation).unwrap();
        tampered["after_key"] = serde_json::json!("wrong::continuation");
        assert!(serde_json::from_value::<PlanningAttemptContinuation>(tampered).is_err());
        assert!(matches!(
            store.attempts_for_goal_bounded("goal-a", None, 0),
            Err(PlanningAttemptStorageError::InvalidInput(_))
        ));
    }

    #[test]
    fn reopen_rejects_key_chain_head_goal_index_and_schema_corruption() {
        for corrupt in [
            "key", "chain", "head", "goal", "recovery", "decision", "schema",
        ] {
            let temp = tempfile::tempdir().unwrap();
            let path = temp.path().join(corrupt);
            let identity = identity();
            {
                let store = owned_store(sled::open(&path).unwrap());
                store.open_attempt(identity.clone()).unwrap();
                store
                    .record_decision(identity.attempt_id(), composed_decision("composition-a"))
                    .unwrap();
                store
                    .prepare_command(identity.attempt_id(), command(&identity, "composition-a"))
                    .unwrap();
                match corrupt {
                    "key" => {
                        let raw = store
                            .records
                            .remove(record_key(identity.attempt_id(), 3))
                            .unwrap()
                            .unwrap();
                        store.records.insert(b"wrong-record-key", raw).unwrap();
                    }
                    "chain" => {
                        let key = record_key(identity.attempt_id(), 3);
                        let raw = store.records.get(&key).unwrap().unwrap();
                        let mut value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
                        value["previous_record_hash"] = serde_json::json!("wrong");
                        store
                            .records
                            .insert(key, serde_json::to_vec(&value).unwrap())
                            .unwrap();
                    }
                    "head" => {
                        store
                            .heads
                            .remove(identity.attempt_id().as_bytes())
                            .unwrap();
                    }
                    "goal" => {
                        store.goal_index.remove(goal_index_key(&identity)).unwrap();
                    }
                    "recovery" => {
                        store
                            .recovery_index
                            .remove(recovery_index_key(&identity))
                            .unwrap();
                    }
                    "decision" => {
                        let key = record_key(identity.attempt_id(), 2);
                        let raw = store.records.get(&key).unwrap().unwrap();
                        let mut value: serde_json::Value = serde_json::from_slice(&raw).unwrap();
                        value["kind"]["DecisionAudited"]["decision"]["decision_digest"] =
                            serde_json::json!(digest("wrong-decision"));
                        store
                            .records
                            .insert(key, serde_json::to_vec(&value).unwrap())
                            .unwrap();
                    }
                    "schema" => {
                        store.heads.insert(KEY_SCHEMA, b"wrong-schema").unwrap();
                    }
                    _ => unreachable!(),
                }
                store.db.flush().unwrap();
            }
            let error = PlanningAttemptStore::new(sled::open(&path).unwrap())
                .err()
                .expect("corrupt store must fail closed");
            assert!(matches!(
                error,
                PlanningAttemptStorageError::Corrupt(_)
                    | PlanningAttemptStorageError::SchemaConflict(_)
            ));
        }
    }

    #[test]
    fn identity_fences_change_for_each_required_replay_input() {
        let base = identity();
        let changed_goal_seq = identity_named("goal-a", 8, "frame-a");
        let changed_frame = identity_named("goal-a", 7, "frame-b");
        assert_ne!(base.attempt_id(), changed_goal_seq.attempt_id());
        assert_ne!(base.attempt_id(), changed_frame.attempt_id());

        let mut inputs = base.planning_request().unwrap().inputs().clone();
        inputs.method_library_digest = digest("different-methods");
        let changed_methods =
            PlanningAttemptIdentity::bind(PlanningRequestIdentity::derive(inputs).unwrap())
                .unwrap();
        assert_ne!(base.attempt_id(), changed_methods.attempt_id());

        let mut inputs = base.planning_request().unwrap().inputs().clone();
        inputs.capability_catalog_digest = digest("different-capabilities");
        let changed_capabilities =
            PlanningAttemptIdentity::bind(PlanningRequestIdentity::derive(inputs).unwrap())
                .unwrap();
        assert_ne!(base.attempt_id(), changed_capabilities.attempt_id());
    }

    #[test]
    fn replacement_owner_fences_every_planning_attempt_transition() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("owner-fence");
        let authority = PlanningAttemptStore::new(sled::open(&path).unwrap()).unwrap();
        let owner_a = authority
            .bind_owner(None, "execution.planning.runtime", "lease-a")
            .unwrap();
        let decision_identity = identity_named("goal-decision", 7, "frame-decision");
        let prepare_identity = identity_named("goal-prepare", 8, "frame-prepare");
        let outcome_identity = identity_named("goal-outcome", 9, "frame-outcome");
        for identity in [&decision_identity, &prepare_identity, &outcome_identity] {
            owner_a.open_attempt(identity.clone()).unwrap();
        }
        for identity in [&prepare_identity, &outcome_identity] {
            owner_a
                .record_decision(identity.attempt_id(), composed_decision("composition-a"))
                .unwrap();
        }
        let outcome_request = command(&outcome_identity, "composition-a");
        owner_a
            .prepare_command(outcome_identity.attempt_id(), outcome_request.clone())
            .unwrap();

        let prior = owner_a.active_owner_fence().unwrap().unwrap();
        let owner_b = authority
            .bind_owner(Some(&prior), "execution.planning.runtime", "lease-b")
            .unwrap();
        let replacement = owner_b.active_owner_fence().unwrap().unwrap();
        assert_eq!(replacement.epoch(), prior.epoch() + 1);
        assert!(matches!(
            owner_a.validate_owner_fence(),
            Err(PlanningAttemptStorageError::StaleOwner(_))
        ));
        owner_b.validate_owner_fence().unwrap();

        let late_identity = identity_named("goal-late", 10, "frame-late");
        for result in [
            owner_a.open_attempt(late_identity.clone()),
            owner_a.record_decision(
                decision_identity.attempt_id(),
                composed_decision("composition-a"),
            ),
            owner_a.prepare_command(
                prepare_identity.attempt_id(),
                command(&prepare_identity, "composition-a"),
            ),
            owner_a.record_command_outcome(
                outcome_identity.attempt_id(),
                receipt(&outcome_request, response()),
            ),
        ] {
            let error = result.unwrap_err();
            assert!(matches!(error, PlanningAttemptStorageError::StaleOwner(_)));
            assert_eq!(error.class(), PlanningAttemptErrorClass::Fatal);
        }

        owner_b.open_attempt(late_identity).unwrap();
        owner_b
            .record_decision(
                decision_identity.attempt_id(),
                composed_decision("composition-a"),
            )
            .unwrap();
        owner_b
            .prepare_command(
                prepare_identity.attempt_id(),
                command(&prepare_identity, "composition-a"),
            )
            .unwrap();
        owner_b
            .record_command_outcome(
                outcome_identity.attempt_id(),
                receipt(&outcome_request, response()),
            )
            .unwrap();
        drop(owner_a);
        drop(owner_b);
        drop(authority);

        let reopened = PlanningAttemptStore::new(sled::open(&path).unwrap()).unwrap();
        assert_eq!(reopened.active_owner_fence().unwrap(), Some(replacement));
        assert_eq!(
            reopened
                .get_head(outcome_identity.attempt_id())
                .unwrap()
                .unwrap()
                .state,
            PlanningAttemptState::CommandCompleted
        );
    }

    #[test]
    fn owner_activation_replays_indeterminate_flush_and_rejects_racing_replacement() {
        let store =
            PlanningAttemptStore::new(sled::Config::new().temporary(true).open().unwrap()).unwrap();
        store.fail_next_flush();
        assert!(matches!(
            store.bind_owner(None, "execution.planning.runtime", "lease-a"),
            Err(PlanningAttemptStorageError::DurabilityIndeterminate(_))
        ));
        let owner_a = store
            .bind_owner(None, "execution.planning.runtime", "lease-a")
            .unwrap();
        let prior = owner_a.active_owner_fence().unwrap().unwrap();
        let owner_b = store
            .bind_owner(Some(&prior), "execution.planning.runtime", "lease-b")
            .unwrap();
        assert!(matches!(
            store.bind_owner(Some(&prior), "execution.planning.runtime", "lease-c"),
            Err(PlanningAttemptStorageError::StaleOwner(_))
        ));
        assert_eq!(
            owner_b.active_owner_fence().unwrap().unwrap().lease_id(),
            "lease-b"
        );
    }

    #[test]
    fn projection_failure_attempt_terminates_without_fabricating_a_frame() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("projection-failure");
        let subject = DomainObjectRef::new("workspace", "node", "readme").unwrap();
        let request = PlanningWorldStateRequest {
            goal_id: "goal-projection-failed".to_string(),
            agent_id: "agent-a".to_string(),
            subject: subject.clone(),
            source_seq: 17,
            target: Proposition::Accessible {
                scope: Term::Object(subject),
            },
            perspective: PlanningPerspectiveRef::new("agent", "agent-a").unwrap(),
            branch_id: "main".to_string(),
            requested_dimensions: vec!["docs_freshness".to_string()],
            required_preconditions: Vec::new(),
        };
        let failure = PlanningProjectionFailureIdentityInputs::bind(
            request,
            digest("methods"),
            digest("capabilities"),
            "execution.planning.v1".to_string(),
        )
        .unwrap();
        let identity = PlanningAttemptIdentity::bind_projection_failure(failure).unwrap();
        assert!(identity.planning_request().is_none());
        assert!(identity.projection_frame_id().is_none());
        assert!(identity.projection_failure().is_some());
        assert!(planning_task_network_command_id(&identity, "composition-a").is_err());

        let terminal;
        {
            let store = owned_store(sled::open(&path).unwrap());
            store.open_attempt(identity.clone()).unwrap();
            store
                .record_decision(
                    identity.attempt_id(),
                    diagnostic_decision(PlanningAttemptResultSummary::ProjectionFailed),
                )
                .unwrap();
            terminal = store
                .record_terminal_diagnostic(
                    identity.attempt_id(),
                    PlanningAttemptTerminalDiagnostic::new(
                        PlanningAttemptDiagnosticDisposition::ProjectionFailed,
                        "projection_unavailable",
                        "world-model projection authority was unavailable",
                    )
                    .unwrap(),
                )
                .unwrap();
        }

        let reopened = owned_store(sled::open(&path).unwrap());
        assert_eq!(
            reopened.get_head(identity.attempt_id()).unwrap(),
            Some(terminal)
        );
    }
}
