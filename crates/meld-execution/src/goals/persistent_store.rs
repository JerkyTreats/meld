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
    validate_satisfaction_sequence,
};
use crate::goals::{
    GoalPlanningClaim, GoalPlanningSelection, GoalPlanningSelectionError,
    GoalPlanningSelectionPort, MAX_PLANNING_SELECTION_LIMIT,
};
use meld_lang::{Goal, GoalLifecycle};
use sled::{
    transaction::{
        ConflictableTransactionError, TransactionError, Transactional, TransactionalTree,
    },
    Db, Tree,
};
use std::collections::BTreeMap;
use std::io;
use std::ops::Bound;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const TREE_RECORDS: &str = "execution_goal_records";
const TREE_COMMAND_IDENTITIES: &str = "execution_goal_command_identities";
const TREE_COMMAND_OUTCOMES: &str = "execution_goal_command_outcomes";
const TREE_COMMAND_RECEIPTS: &str = "execution_goal_command_receipts";
const TREE_SOURCE_IDENTITY: &str = "execution_goal_source_identity";
const TREE_PLANNING_ACTIVE_INDEX: &str = "execution_goal_planning_active_index";
const TREE_PLANNING_AVAILABLE_INDEX: &str = "execution_goal_planning_available_index";
const TREE_PLANNING_CLAIMED_INDEX: &str = "execution_goal_planning_claimed_index";
const TREE_PLANNING_CLAIMS: &str = "execution_goal_planning_claims";
const TREE_PLANNING_CLAIM_MEMBERS: &str = "execution_goal_planning_claim_members";
const TREE_PLANNING_CLAIM_EXPIRY: &str = "execution_goal_planning_claim_expiry";
const TREE_PLANNING_METADATA: &str = "execution_goal_planning_metadata";
const TREE_PLANNING_SELECTION: &str = "execution_goal_planning_selection";
const KEY_PLANNING_INDEX_METADATA: &[u8] = b"index";
const KEY_PLANNING_SELECTION_STATE: &[u8] = b"state";
const PLANNING_INDEX_SCHEMA_VERSION: u16 = 1;
const MAX_PLANNING_SELECTION_RETRIES: usize = 32;
const MAX_PLANNING_INDEX_VALIDATION_RETRIES: usize = 4_096;
const MAX_PLANNING_GOAL_ID_BYTES: usize = 1_024;
const MAX_PLANNING_GOAL_RECORD_BYTES: usize = 1_048_576;
const PLANNING_CLAIM_LEASE_MILLIS: u64 = 300_000;
const MAX_EXPIRED_CLAIM_RECLAIMS_PER_SELECTION: usize = 1;
const MAX_ORPHAN_EXPIRY_REPAIRS_PER_SELECTION: usize = 64;

type GoalCommandCommit = (GoalCommandOutcome, GoalCommandCommitReceipt);
type GoalTransactionError = ConflictableTransactionError<ExecutionInvariantError>;

impl GoalPlanningSelectionError {
    fn from_sled(error: sled::Error) -> Self {
        match error {
            sled::Error::Io(_) => Self::TransientStorage(error.to_string()),
            sled::Error::CollectionNotFound(_)
            | sled::Error::Unsupported(_)
            | sled::Error::ReportableBug(_)
            | sled::Error::Corruption { .. } => Self::CorruptState(error.to_string()),
        }
    }
}

type GoalStoreReadError = GoalPlanningSelectionError;

#[derive(Debug)]
struct PlanningGoalSelection {
    records: Vec<ExecutionGoalRecord>,
    active_goal_count: usize,
    budget_exhausted: bool,
    claim_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PlanningGoalOrderKey {
    urgency: u32,
    goal_id: String,
}

impl PlanningGoalOrderKey {
    fn for_record(record: &ExecutionGoalRecord) -> Self {
        Self {
            urgency: record.goal.priority.urgency,
            goal_id: record.goal.goal_id.clone(),
        }
    }

    fn encode(&self) -> Vec<u8> {
        let mut encoded = self.urgency.to_be_bytes().to_vec();
        encoded.extend_from_slice(self.goal_id.as_bytes());
        encoded
    }

    fn decode(encoded: &[u8]) -> Result<Self, GoalStoreReadError> {
        if encoded.len() <= std::mem::size_of::<u32>() {
            return Err(GoalStoreReadError::CorruptState(
                "planning index key is truncated".to_string(),
            ));
        }
        let urgency = u32::from_be_bytes(
            encoded[..4]
                .try_into()
                .expect("planning index urgency has a fixed width"),
        );
        let goal_id = std::str::from_utf8(&encoded[4..])
            .map_err(|error| GoalStoreReadError::CorruptState(error.to_string()))?
            .to_string();
        validate_non_empty("planning index goal id", &goal_id)
            .map_err(|error| GoalStoreReadError::CorruptState(error.to_string()))?;
        if goal_id.len() > MAX_PLANNING_GOAL_ID_BYTES {
            return Err(GoalStoreReadError::CorruptState(format!(
                "planning index goal id exceeds {MAX_PLANNING_GOAL_ID_BYTES} bytes"
            )));
        }
        Ok(Self { urgency, goal_id })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanningIndexMetadata {
    schema_version: u16,
    generation: u64,
    active_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanningIndexEntry {
    schema_version: u16,
    goal_id: String,
    updated_at_seq: u64,
    record_hash: String,
}

impl PlanningIndexEntry {
    fn for_record(record: &ExecutionGoalRecord) -> Result<Self, ExecutionInvariantError> {
        let entry = Self {
            schema_version: PLANNING_INDEX_SCHEMA_VERSION,
            goal_id: record.goal.goal_id.clone(),
            updated_at_seq: record.updated_at_seq,
            record_hash: planning_record_hash(record)?,
        };
        entry
            .validate_for_record(record)
            .map_err(|error| ExecutionInvariantError::ConfigError(error.to_string()))?;
        Ok(entry)
    }

    fn validate_for_record(&self, record: &ExecutionGoalRecord) -> Result<(), GoalStoreReadError> {
        if self.schema_version != PLANNING_INDEX_SCHEMA_VERSION {
            return Err(GoalStoreReadError::CorruptState(format!(
                "planning index entry schema version {} is unsupported",
                self.schema_version
            )));
        }
        if self.goal_id != record.goal.goal_id
            || self.updated_at_seq != record.updated_at_seq
            || self.record_hash
                != planning_record_hash(record)
                    .map_err(|error| GoalStoreReadError::CorruptState(error.to_string()))?
        {
            return Err(GoalStoreReadError::CorruptState(format!(
                "planning index entry for '{}' does not match its goal record",
                self.goal_id
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanningSelectionState {
    schema_version: u16,
    generation: u64,
    cursor: Option<Vec<u8>>,
}

impl Default for PlanningSelectionState {
    fn default() -> Self {
        Self {
            schema_version: PLANNING_INDEX_SCHEMA_VERSION,
            generation: 0,
            cursor: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanningClaim {
    schema_version: u16,
    claim_id: String,
    generation: u64,
    expires_at_millis: u64,
    member_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanningClaimedEntry {
    schema_version: u16,
    claim_id: String,
    entry: PlanningIndexEntry,
}

#[derive(Debug, Clone)]
struct PlanningCandidate {
    order_key: Vec<u8>,
    entry_value: Vec<u8>,
    entry: PlanningIndexEntry,
}

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
    planning_active_index: Tree,
    planning_available_index: Tree,
    planning_claimed_index: Tree,
    planning_claims: Tree,
    planning_claim_members: Tree,
    planning_claim_expiry: Tree,
    planning_metadata: Tree,
    planning_selection: Tree,
    legacy_replay_policy: LegacyGoalCommandReplayPolicy,
    deferred_planning_validation_error: Option<String>,
}

impl PersistentGoalSetStore {
    /// Open goal trees with verified reconstruction for compatible legacy outcomes.
    pub fn new(db: Db) -> Result<Self, ExecutionInvariantError> {
        Self::open_with_posture(
            db,
            LegacyGoalCommandReplayPolicy::ReconstructAndVerify,
            false,
        )
    }

    /// Open command storage while deferring invalid planning history to its read boundary.
    ///
    /// Runtime assembly uses this posture so construction does not query
    /// semantic goal work. Planning still fails closed before selection, while
    /// unrelated producer commands retain their durable write boundary.
    pub fn open_deferred_planning_validation(db: Db) -> Result<Self, ExecutionInvariantError> {
        Self::open_with_posture(
            db,
            LegacyGoalCommandReplayPolicy::ReconstructAndVerify,
            true,
        )
    }

    /// Open goal trees with an explicit crate-owned legacy replay posture.
    #[cfg(test)]
    pub(crate) fn with_legacy_replay_policy(
        db: Db,
        legacy_replay_policy: LegacyGoalCommandReplayPolicy,
    ) -> Result<Self, ExecutionInvariantError> {
        Self::open_with_posture(db, legacy_replay_policy, false)
    }

    fn open_with_posture(
        db: Db,
        legacy_replay_policy: LegacyGoalCommandReplayPolicy,
        defer_planning_validation: bool,
    ) -> Result<Self, ExecutionInvariantError> {
        let mut store = Self {
            records: db.open_tree(TREE_RECORDS).map_err(to_store_io)?,
            command_identities: db.open_tree(TREE_COMMAND_IDENTITIES).map_err(to_store_io)?,
            command_outcomes: db.open_tree(TREE_COMMAND_OUTCOMES).map_err(to_store_io)?,
            command_receipts: db.open_tree(TREE_COMMAND_RECEIPTS).map_err(to_store_io)?,
            source_identity_index: db.open_tree(TREE_SOURCE_IDENTITY).map_err(to_store_io)?,
            planning_active_index: db
                .open_tree(TREE_PLANNING_ACTIVE_INDEX)
                .map_err(to_store_io)?,
            planning_available_index: db
                .open_tree(TREE_PLANNING_AVAILABLE_INDEX)
                .map_err(to_store_io)?,
            planning_claimed_index: db
                .open_tree(TREE_PLANNING_CLAIMED_INDEX)
                .map_err(to_store_io)?,
            planning_claims: db.open_tree(TREE_PLANNING_CLAIMS).map_err(to_store_io)?,
            planning_claim_members: db
                .open_tree(TREE_PLANNING_CLAIM_MEMBERS)
                .map_err(to_store_io)?,
            planning_claim_expiry: db
                .open_tree(TREE_PLANNING_CLAIM_EXPIRY)
                .map_err(to_store_io)?,
            planning_metadata: db.open_tree(TREE_PLANNING_METADATA).map_err(to_store_io)?,
            planning_selection: db.open_tree(TREE_PLANNING_SELECTION).map_err(to_store_io)?,
            db,
            legacy_replay_policy,
            deferred_planning_validation_error: None,
        };
        if let Err(error) = store.initialize_planning_index() {
            if !defer_planning_validation {
                return Err(error);
            }
            store.initialize_planning_index_from_valid_records()?;
            store.deferred_planning_validation_error = Some(error.to_string());
        }
        Ok(store)
    }

    fn initialize_planning_index_from_valid_records(&self) -> Result<(), ExecutionInvariantError> {
        if self
            .planning_metadata
            .get(KEY_PLANNING_INDEX_METADATA)
            .map_err(to_store_io)?
            .is_some()
        {
            return Ok(());
        }
        let mut expected = BTreeMap::new();
        for item in self.records.iter() {
            let (key, value) = item.map_err(to_store_io)?;
            let Ok(durable_key) = std::str::from_utf8(&key) else {
                continue;
            };
            let Ok(record) = serde_json::from_slice::<ExecutionGoalRecord>(&value) else {
                continue;
            };
            if validate_planning_goal_record(durable_key, &record).is_err() {
                continue;
            }
            if matches!(record.goal.lifecycle, GoalLifecycle::Active) {
                let order_key = PlanningGoalOrderKey::for_record(&record).encode();
                let entry = PlanningIndexEntry::for_record(&record)?;
                expected.insert(
                    order_key,
                    serde_json::to_vec(&entry).map_err(to_store_data)?,
                );
            }
        }
        self.migrate_planning_index(&expected)?;
        self.flush()
    }

    /// Open the store behind a shared pointer for runtime facade wiring.
    pub fn shared(db: Db) -> Result<Arc<Self>, ExecutionInvariantError> {
        Ok(Arc::new(Self::new(db)?))
    }

    fn initialize_planning_index(&self) -> Result<(), ExecutionInvariantError> {
        let Some(raw_metadata) = self
            .planning_metadata
            .get(KEY_PLANNING_INDEX_METADATA)
            .map_err(to_store_io)?
        else {
            let expected = self.expected_planning_index()?;
            if let Err(error) = self.migrate_planning_index(&expected) {
                if self
                    .planning_metadata
                    .get(KEY_PLANNING_INDEX_METADATA)
                    .map_err(to_store_io)?
                    .is_some()
                {
                    return self.initialize_planning_index();
                }
                return Err(error);
            }
            self.flush()?;
            return Ok(());
        };
        self.repair_orphan_planning_claim_indexes()?;
        self.validate_stable_planning_index(raw_metadata.to_vec())
    }

    fn repair_orphan_planning_claim_indexes(&self) -> Result<(), ExecutionInvariantError> {
        let claims = tree_entries(&self.planning_claims)?;
        let orphan_members = tree_entries(&self.planning_claim_members)?
            .into_iter()
            .filter_map(|entry| {
                let separator = entry.0.iter().position(|byte| *byte == 0)?;
                let claim_key = &entry.0[..separator];
                (!claims.contains_key(claim_key)).then_some(entry)
            })
            .collect::<Vec<_>>();
        let orphan_expiry = tree_entries(&self.planning_claim_expiry)?
            .into_iter()
            .filter(|(_, claim_key)| !claims.contains_key(claim_key))
            .collect::<Vec<_>>();
        if orphan_members.is_empty() && orphan_expiry.is_empty() {
            return Ok(());
        }
        (
            &self.planning_claims,
            &self.planning_claim_members,
            &self.planning_claim_expiry,
            &self.planning_metadata,
        )
            .transaction(|(claims, members, expiry, metadata)| {
                let mut repaired = false;
                for (key, value) in &orphan_members {
                    let separator = key.iter().position(|byte| *byte == 0).ok_or_else(|| {
                        goal_transaction_config("planning claim member key is invalid")
                    })?;
                    if claims.get(&key[..separator])?.is_none()
                        && members.get(key.clone())?.as_deref() == Some(value.as_slice())
                    {
                        members.remove(key.clone())?;
                        repaired = true;
                    }
                }
                for (key, claim_key) in &orphan_expiry {
                    if claims.get(claim_key.clone())?.is_none()
                        && expiry.get(key.clone())?.as_deref() == Some(claim_key.as_slice())
                    {
                        expiry.remove(key.clone())?;
                        repaired = true;
                    }
                }
                if repaired {
                    let mut current: PlanningIndexMetadata = required_goal_transaction_value(
                        metadata,
                        KEY_PLANNING_INDEX_METADATA,
                        "planning index metadata",
                    )?;
                    current.generation = current.generation.checked_add(1).ok_or_else(|| {
                        goal_transaction_config("planning index generation overflowed")
                    })?;
                    metadata.insert(KEY_PLANNING_INDEX_METADATA, encode_transaction(&current)?)?;
                }
                Ok(())
            })
            .map_err(to_goal_transaction)?;
        self.flush()
    }

    fn validate_stable_planning_index(
        &self,
        mut raw_metadata: Vec<u8>,
    ) -> Result<(), ExecutionInvariantError> {
        for _ in 0..MAX_PLANNING_INDEX_VALIDATION_RETRIES {
            let metadata: PlanningIndexMetadata =
                serde_json::from_slice(&raw_metadata).map_err(to_store_data)?;
            if metadata.schema_version != PLANNING_INDEX_SCHEMA_VERSION {
                return Err(ExecutionInvariantError::ConfigError(format!(
                    "planning index schema version {} is unsupported",
                    metadata.schema_version
                )));
            }
            let expected = self.expected_planning_index()?;
            let observed = self
                .planning_metadata
                .get(KEY_PLANNING_INDEX_METADATA)
                .map_err(to_store_io)?
                .ok_or_else(|| {
                    ExecutionInvariantError::ConfigError(
                        "planning index metadata disappeared during validation".to_string(),
                    )
                })?;
            if observed.as_ref() != raw_metadata.as_slice() {
                raw_metadata = observed.to_vec();
                std::thread::yield_now();
                continue;
            }
            let validation = if metadata.active_count != expected.len() {
                Err(ExecutionInvariantError::ConfigError(format!(
                    "planning active count {} does not match reconstructed count {}",
                    metadata.active_count,
                    expected.len()
                )))
            } else {
                self.validate_existing_planning_index(&expected)
            };
            let final_metadata = self
                .planning_metadata
                .get(KEY_PLANNING_INDEX_METADATA)
                .map_err(to_store_io)?
                .ok_or_else(|| {
                    ExecutionInvariantError::ConfigError(
                        "planning index metadata disappeared during validation".to_string(),
                    )
                })?;
            if final_metadata.as_ref() != raw_metadata.as_slice() {
                raw_metadata = final_metadata.to_vec();
                std::thread::yield_now();
                continue;
            }
            return validation;
        }
        Err(ExecutionInvariantError::ConfigError(
            "planning index validation remained contended".to_string(),
        ))
    }

    fn expected_planning_index(
        &self,
    ) -> Result<BTreeMap<Vec<u8>, Vec<u8>>, ExecutionInvariantError> {
        let mut expected = BTreeMap::new();
        for item in self.records.iter() {
            let (key, value) = item.map_err(to_store_io)?;
            let durable_key = std::str::from_utf8(&key).map_err(|error| {
                ExecutionInvariantError::ConfigError(format!(
                    "goal record key is not valid UTF-8: {error}"
                ))
            })?;
            let record: ExecutionGoalRecord =
                serde_json::from_slice(&value).map_err(to_store_data)?;
            validate_planning_goal_record(durable_key, &record)
                .map_err(|error| ExecutionInvariantError::ConfigError(error.to_string()))?;
            if matches!(record.goal.lifecycle, GoalLifecycle::Active) {
                let order_key = PlanningGoalOrderKey::for_record(&record).encode();
                let entry = PlanningIndexEntry::for_record(&record)?;
                expected.insert(
                    order_key,
                    serde_json::to_vec(&entry).map_err(to_store_data)?,
                );
            }
        }
        Ok(expected)
    }

    fn migrate_planning_index(
        &self,
        expected: &BTreeMap<Vec<u8>, Vec<u8>>,
    ) -> Result<(), ExecutionInvariantError> {
        let active_keys = tree_keys(&self.planning_active_index)?;
        let available_keys = tree_keys(&self.planning_available_index)?;
        let claimed_keys = tree_keys(&self.planning_claimed_index)?;
        let claim_keys = tree_keys(&self.planning_claims)?;
        let member_keys = tree_keys(&self.planning_claim_members)?;
        let expiry_keys = tree_keys(&self.planning_claim_expiry)?;
        let selection_keys = tree_keys(&self.planning_selection)?;
        let metadata_keys = tree_keys(&self.planning_metadata)?;
        let expected = expected
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Vec<_>>();
        let metadata = serde_json::to_vec(&PlanningIndexMetadata {
            schema_version: PLANNING_INDEX_SCHEMA_VERSION,
            generation: 0,
            active_count: expected.len(),
        })
        .map_err(to_store_data)?;
        let selection =
            serde_json::to_vec(&PlanningSelectionState::default()).map_err(to_store_data)?;
        (
            &self.planning_active_index,
            &self.planning_available_index,
            &self.planning_claimed_index,
            &self.planning_claims,
            &self.planning_claim_members,
            &self.planning_claim_expiry,
            &self.planning_metadata,
            &self.planning_selection,
        )
            .transaction(
                |(
                    active,
                    available,
                    claimed,
                    claims,
                    members,
                    expiry,
                    index_meta,
                    selection_state,
                )| {
                    // Metadata is the migration ownership token. Including it
                    // with every index tree prevents a slower opener from
                    // replacing a newer transactional index installation.
                    if index_meta.get(KEY_PLANNING_INDEX_METADATA)?.is_some() {
                        return Err(goal_transaction_config(
                            "planning index migration lost its metadata claim",
                        ));
                    }
                    remove_transaction_keys(active, &active_keys)?;
                    remove_transaction_keys(available, &available_keys)?;
                    remove_transaction_keys(claimed, &claimed_keys)?;
                    remove_transaction_keys(claims, &claim_keys)?;
                    remove_transaction_keys(members, &member_keys)?;
                    remove_transaction_keys(expiry, &expiry_keys)?;
                    remove_transaction_keys(index_meta, &metadata_keys)?;
                    remove_transaction_keys(selection_state, &selection_keys)?;
                    for (key, value) in &expected {
                        active.insert(key.clone(), value.clone())?;
                        available.insert(key.clone(), value.clone())?;
                    }
                    index_meta.insert(KEY_PLANNING_INDEX_METADATA, metadata.clone())?;
                    selection_state.insert(KEY_PLANNING_SELECTION_STATE, selection.clone())?;
                    Ok(())
                },
            )
            .map_err(to_goal_transaction)
    }

    fn validate_existing_planning_index(
        &self,
        expected: &BTreeMap<Vec<u8>, Vec<u8>>,
    ) -> Result<(), ExecutionInvariantError> {
        let active = tree_entries(&self.planning_active_index)?;
        if &active != expected {
            return Err(ExecutionInvariantError::ConfigError(
                "planning active index diverges from durable goal records".to_string(),
            ));
        }
        let available = tree_entries(&self.planning_available_index)?;
        let claimed = tree_entries(&self.planning_claimed_index)?;
        for (key, value) in &available {
            if expected.get(key) != Some(value) {
                return Err(ExecutionInvariantError::ConfigError(
                    "planning available index contains a divergent entry".to_string(),
                ));
            }
        }
        let mut partition = available
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        for (key, value) in &claimed {
            let claimed: PlanningClaimedEntry =
                serde_json::from_slice(value).map_err(to_store_data)?;
            validate_planning_claimed_entry(key, &claimed, expected)?;
            if !partition.insert(key.clone()) {
                return Err(ExecutionInvariantError::ConfigError(
                    "planning entry is both available and claimed".to_string(),
                ));
            }
        }
        if partition != expected.keys().cloned().collect() {
            return Err(ExecutionInvariantError::ConfigError(
                "planning available and claimed indexes do not partition active goals".to_string(),
            ));
        }
        let selection: PlanningSelectionState = decode_required_tree_value(
            &self.planning_selection,
            KEY_PLANNING_SELECTION_STATE,
            "planning selection state",
        )?;
        validate_planning_selection_state(&selection)
            .map_err(|error| ExecutionInvariantError::ConfigError(error.to_string()))?;
        validate_planning_claim_trees(self)
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
            &self.planning_active_index,
            &self.planning_available_index,
            &self.planning_claimed_index,
            &self.planning_claims,
            &self.planning_claim_members,
            &self.planning_claim_expiry,
            &self.planning_metadata,
        )
            .transaction(
                |(
                    records,
                    source_identity_index,
                    command_identities,
                    command_outcomes,
                    command_receipts,
                    planning_active,
                    planning_available,
                    planning_claimed,
                    planning_claims,
                    planning_members,
                    planning_expiry,
                    planning_metadata,
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
                            PlanningIndexTransactionTrees {
                                active: planning_active,
                                available: planning_available,
                                claimed: planning_claimed,
                                claims: planning_claims,
                                members: planning_members,
                                expiry: planning_expiry,
                                metadata: planning_metadata,
                            }
                            .update(None, Some(&expected_record))?;
                            source_identity_index
                                .insert(source_identity_key, source_identity_value.clone())?;
                            GoalCommandOutcome::Applied(Box::new(expected_record.clone()))
                        }
                    } else {
                        records.insert(goal_key.clone(), encode_transaction(&expected_record)?)?;
                        PlanningIndexTransactionTrees {
                            active: planning_active,
                            available: planning_available,
                            claimed: planning_claimed,
                            claims: planning_claims,
                            members: planning_members,
                            expiry: planning_expiry,
                            metadata: planning_metadata,
                        }
                        .update(None, Some(&expected_record))?;
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
            &self.planning_active_index,
            &self.planning_available_index,
            &self.planning_claimed_index,
            &self.planning_claims,
            &self.planning_claim_members,
            &self.planning_claim_expiry,
            &self.planning_metadata,
        )
            .transaction(
                |(
                    records,
                    source_identity_index,
                    command_identities,
                    command_outcomes,
                    command_receipts,
                    planning_active,
                    planning_available,
                    planning_claimed,
                    planning_claims,
                    planning_members,
                    planning_expiry,
                    planning_metadata,
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
                    PlanningIndexTransactionTrees {
                        active: planning_active,
                        available: planning_available,
                        claimed: planning_claimed,
                        claims: planning_claims,
                        members: planning_members,
                        expiry: planning_expiry,
                        metadata: planning_metadata,
                    }
                    .update(Some(&existing), Some(&record))?;
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
        validate_satisfaction_sequence(command.at_seq)?;
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

    fn select_planning_goal_records(
        &self,
        limit: usize,
    ) -> Result<PlanningGoalSelection, GoalStoreReadError> {
        if let Some(error) = &self.deferred_planning_validation_error {
            return Err(GoalStoreReadError::CorruptState(error.clone()));
        }
        debug_assert!((1..=MAX_PLANNING_SELECTION_LIMIT).contains(&limit));
        for _ in 0..MAX_EXPIRED_CLAIM_RECLAIMS_PER_SELECTION {
            if !self.reclaim_one_expired_planning_claim()? {
                break;
            }
        }
        for _ in 0..MAX_PLANNING_SELECTION_RETRIES {
            let metadata = self.planning_index_metadata()?;
            let selection_state = self.planning_selection_state()?;
            let (candidates, wrapped) =
                self.available_planning_candidates(selection_state.cursor.as_deref(), limit)?;
            if candidates.is_empty() {
                return Ok(PlanningGoalSelection {
                    records: Vec::new(),
                    active_goal_count: metadata.active_count,
                    budget_exhausted: metadata.active_count > 0,
                    claim_id: None,
                });
            }
            let claim_id = uuid::Uuid::new_v4().to_string();
            let expires_at_millis = now_millis()?.saturating_add(PLANNING_CLAIM_LEASE_MILLIS);
            match self.claim_planning_candidates(
                &metadata,
                &selection_state,
                &candidates,
                wrapped,
                &claim_id,
                expires_at_millis,
            ) {
                Ok(records) => {
                    self.db.flush().map_err(GoalStoreReadError::from_sled)?;
                    return Ok(PlanningGoalSelection {
                        budget_exhausted: metadata.active_count > records.len(),
                        active_goal_count: metadata.active_count,
                        records,
                        claim_id: Some(claim_id),
                    });
                }
                Err(GoalStoreReadError::SelectionContended) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(GoalStoreReadError::SelectionContended)
    }

    fn release_planning_goal_claim(&self, claim_id: &str) -> Result<(), GoalStoreReadError> {
        let Some(raw_claim) = self
            .planning_claims
            .get(claim_id.as_bytes())
            .map_err(GoalStoreReadError::from_sled)?
        else {
            return Ok(());
        };
        let claim: PlanningClaim = decode_planning_value(&raw_claim, "planning claim")?;
        validate_planning_claim(claim_id, &claim)?;
        let prefix = planning_claim_member_prefix(claim_id);
        let members = self
            .planning_claim_members
            .scan_prefix(&prefix)
            .take(MAX_PLANNING_SELECTION_LIMIT + 1)
            .map(|item| item.map_err(GoalStoreReadError::from_sled))
            .collect::<Result<Vec<_>, _>>()?;
        if members.len() != claim.member_count || members.len() > MAX_PLANNING_SELECTION_LIMIT {
            return Err(GoalStoreReadError::CorruptState(format!(
                "planning claim '{}' member count is inconsistent",
                claim_id
            )));
        }
        let claim_key = claim_id.as_bytes().to_vec();
        let expiry_key = planning_claim_expiry_key(claim.expires_at_millis, claim_id);
        (
            &self.planning_active_index,
            &self.planning_available_index,
            &self.planning_claimed_index,
            &self.planning_claims,
            &self.planning_claim_members,
            &self.planning_claim_expiry,
            &self.planning_metadata,
        )
            .transaction(
                |(active, available, claimed, claims, claim_members, expiry, metadata)| {
                    if claims.get(claim_key.clone())?.as_deref() != Some(raw_claim.as_ref()) {
                        return Err(ConflictableTransactionError::Abort(
                            GoalStoreReadError::SelectionContended,
                        ));
                    }
                    for (member_key, entry_value) in &members {
                        let order_key = planning_order_key_from_member(&prefix, member_key)?;
                        let raw_claimed = claimed.get(order_key.clone())?.ok_or_else(|| {
                            ConflictableTransactionError::Abort(GoalStoreReadError::CorruptState(
                                "planning claim member has no claimed index entry".to_string(),
                            ))
                        })?;
                        let claimed_entry: PlanningClaimedEntry =
                            decode_planning_transaction_value(&raw_claimed, "claimed entry")?;
                        if claimed_entry.claim_id != claim_id
                            || claimed_entry.entry
                                != decode_planning_transaction_value(entry_value, "claim member")?
                        {
                            return Err(ConflictableTransactionError::Abort(
                                GoalStoreReadError::CorruptState(
                                    "planning claimed entry does not match claim membership"
                                        .to_string(),
                                ),
                            ));
                        }
                        claimed.remove(order_key.clone())?;
                        claim_members.remove(member_key.clone())?;
                        if active.get(order_key.clone())?.as_deref() == Some(entry_value.as_ref()) {
                            available.insert(order_key, entry_value.clone())?;
                        }
                    }
                    claims.remove(claim_key.clone())?;
                    expiry.remove(expiry_key.clone())?;
                    let mut current: PlanningIndexMetadata = required_planning_transaction_value(
                        metadata,
                        KEY_PLANNING_INDEX_METADATA,
                        "planning metadata",
                    )?;
                    current.generation = current.generation.checked_add(1).ok_or_else(|| {
                        ConflictableTransactionError::Abort(GoalStoreReadError::CorruptState(
                            "planning index generation overflowed".to_string(),
                        ))
                    })?;
                    metadata.insert(
                        KEY_PLANNING_INDEX_METADATA,
                        encode_planning_transaction_value(&current, "planning metadata")?,
                    )?;
                    Ok(())
                },
            )
            .map_err(planning_transaction_error)?;
        self.db.flush().map_err(GoalStoreReadError::from_sled)?;
        Ok(())
    }

    fn planning_index_metadata(&self) -> Result<PlanningIndexMetadata, GoalStoreReadError> {
        let raw = self
            .planning_metadata
            .get(KEY_PLANNING_INDEX_METADATA)
            .map_err(GoalStoreReadError::from_sled)?
            .ok_or_else(|| {
                GoalStoreReadError::CorruptState("planning index metadata is missing".to_string())
            })?;
        let metadata: PlanningIndexMetadata = decode_planning_value(&raw, "planning metadata")?;
        if metadata.schema_version != PLANNING_INDEX_SCHEMA_VERSION {
            return Err(GoalStoreReadError::CorruptState(format!(
                "planning index schema version {} is unsupported",
                metadata.schema_version
            )));
        }
        Ok(metadata)
    }

    fn planning_selection_state(&self) -> Result<PlanningSelectionState, GoalStoreReadError> {
        let raw = self
            .planning_selection
            .get(KEY_PLANNING_SELECTION_STATE)
            .map_err(GoalStoreReadError::from_sled)?
            .ok_or_else(|| {
                GoalStoreReadError::CorruptState("planning selection state is missing".to_string())
            })?;
        let state: PlanningSelectionState = decode_planning_value(&raw, "selection state")?;
        validate_planning_selection_state(&state)?;
        Ok(state)
    }

    fn available_planning_candidates(
        &self,
        cursor: Option<&[u8]>,
        limit: usize,
    ) -> Result<(Vec<PlanningCandidate>, bool), GoalStoreReadError> {
        let mut candidates = Vec::with_capacity(limit);
        let after_range = match cursor {
            Some(cursor) => (Bound::Excluded(cursor.to_vec()), Bound::Unbounded),
            None => (Bound::Unbounded, Bound::Unbounded),
        };
        for item in self.planning_available_index.range(after_range).take(limit) {
            let (key, value) = item.map_err(GoalStoreReadError::from_sled)?;
            candidates.push(decode_planning_candidate(key.to_vec(), value.to_vec())?);
        }
        let mut wrapped = false;
        if let Some(cursor) = cursor.filter(|_| candidates.len() < limit) {
            let remaining = limit - candidates.len();
            for item in self
                .planning_available_index
                .range((Bound::Unbounded, Bound::Included(cursor.to_vec())))
                .take(remaining)
            {
                let (key, value) = item.map_err(GoalStoreReadError::from_sled)?;
                candidates.push(decode_planning_candidate(key.to_vec(), value.to_vec())?);
                wrapped = true;
            }
        }
        Ok((candidates, wrapped))
    }

    fn claim_planning_candidates(
        &self,
        metadata: &PlanningIndexMetadata,
        selection_state: &PlanningSelectionState,
        candidates: &[PlanningCandidate],
        wrapped: bool,
        claim_id: &str,
        expires_at_millis: u64,
    ) -> Result<Vec<ExecutionGoalRecord>, GoalStoreReadError> {
        let claim_generation = metadata.generation.checked_add(1).ok_or_else(|| {
            GoalStoreReadError::CorruptState("planning index generation overflowed".to_string())
        })?;
        let claim = PlanningClaim {
            schema_version: PLANNING_INDEX_SCHEMA_VERSION,
            claim_id: claim_id.to_string(),
            generation: claim_generation,
            expires_at_millis,
            member_count: candidates.len(),
        };
        let claim_value = encode_planning_value(&claim, "planning claim")?;
        let claim_key = claim_id.as_bytes().to_vec();
        let expiry_key = planning_claim_expiry_key(expires_at_millis, claim_id);
        let mut next_state = selection_state.clone();
        next_state.cursor = candidates
            .last()
            .map(|candidate| candidate.order_key.clone());
        if wrapped {
            next_state.generation = next_state.generation.saturating_add(1);
        }
        let next_state_value = encode_planning_value(&next_state, "planning selection state")?;
        (
            &self.records,
            &self.planning_active_index,
            &self.planning_available_index,
            &self.planning_claimed_index,
            &self.planning_claims,
            &self.planning_claim_members,
            &self.planning_claim_expiry,
            &self.planning_metadata,
            &self.planning_selection,
        )
            .transaction(
                |(
                    records,
                    active,
                    available,
                    claimed,
                    claims,
                    members,
                    expiry,
                    index_meta,
                    selection,
                )| {
                    // Claim membership and the cyclic cursor move together so
                    // concurrent selectors cannot observe the same wrapped
                    // window as available work.
                    let current_metadata: PlanningIndexMetadata =
                        required_planning_transaction_value(
                            index_meta,
                            KEY_PLANNING_INDEX_METADATA,
                            "planning metadata",
                        )?;
                    let current_selection: PlanningSelectionState =
                        required_planning_transaction_value(
                            selection,
                            KEY_PLANNING_SELECTION_STATE,
                            "planning selection state",
                        )?;
                    if current_metadata != *metadata || current_selection != *selection_state {
                        return Err(ConflictableTransactionError::Abort(
                            GoalStoreReadError::SelectionContended,
                        ));
                    }
                    if claims.get(claim_key.clone())?.is_some() {
                        return Err(ConflictableTransactionError::Abort(
                            GoalStoreReadError::SelectionContended,
                        ));
                    }
                    let mut selected_records = Vec::with_capacity(candidates.len());
                    for candidate in candidates {
                        if available.get(candidate.order_key.clone())?.as_deref()
                            != Some(candidate.entry_value.as_slice())
                            || active.get(candidate.order_key.clone())?.as_deref()
                                != Some(candidate.entry_value.as_slice())
                        {
                            return Err(ConflictableTransactionError::Abort(
                                GoalStoreReadError::SelectionContended,
                            ));
                        }
                        let raw_record = records
                            .get(candidate.entry.goal_id.as_bytes())?
                            .ok_or_else(|| {
                                ConflictableTransactionError::Abort(
                                    GoalStoreReadError::CorruptState(format!(
                                        "planning index goal '{}' has no durable record",
                                        candidate.entry.goal_id
                                    )),
                                )
                            })?;
                        if raw_record.len() > MAX_PLANNING_GOAL_RECORD_BYTES {
                            return Err(ConflictableTransactionError::Abort(
                                GoalStoreReadError::CorruptState(format!(
                                    "planning goal '{}' exceeds {MAX_PLANNING_GOAL_RECORD_BYTES} bytes",
                                    candidate.entry.goal_id
                                )),
                            ));
                        }
                        let record: ExecutionGoalRecord =
                            decode_planning_transaction_value(&raw_record, "planning goal record")?;
                        validate_planning_goal_record(&candidate.entry.goal_id, &record)
                            .map_err(ConflictableTransactionError::Abort)?;
                        candidate
                            .entry
                            .validate_for_record(&record)
                            .map_err(ConflictableTransactionError::Abort)?;
                        let claimed_entry = PlanningClaimedEntry {
                            schema_version: PLANNING_INDEX_SCHEMA_VERSION,
                            claim_id: claim_id.to_string(),
                            entry: candidate.entry.clone(),
                        };
                        let claimed_value = encode_planning_transaction_value(
                            &claimed_entry,
                            "planning claimed entry",
                        )?;
                        available.remove(candidate.order_key.clone())?;
                        claimed.insert(candidate.order_key.clone(), claimed_value)?;
                        members.insert(
                            planning_claim_member_key(claim_id, &candidate.order_key),
                            candidate.entry_value.clone(),
                        )?;
                        selected_records.push(record);
                    }
                    claims.insert(claim_key.clone(), claim_value.clone())?;
                    expiry.insert(expiry_key.clone(), claim_key.clone())?;
                    selection.insert(KEY_PLANNING_SELECTION_STATE, next_state_value.clone())?;
                    let mut next_metadata = current_metadata;
                    next_metadata.generation = claim_generation;
                    index_meta.insert(
                        KEY_PLANNING_INDEX_METADATA,
                        encode_planning_transaction_value(&next_metadata, "planning metadata")?,
                    )?;
                    Ok(selected_records)
                },
            )
            .map_err(planning_transaction_error)
    }

    fn reclaim_one_expired_planning_claim(&self) -> Result<bool, GoalStoreReadError> {
        let now = now_millis()?;
        let mut upper = now.to_be_bytes().to_vec();
        upper.extend_from_slice(&[u8::MAX; 64]);
        for _ in 0..MAX_ORPHAN_EXPIRY_REPAIRS_PER_SELECTION {
            let Some(item) = self
                .planning_claim_expiry
                .range::<Vec<u8>, _>(..=upper.clone())
                .next()
            else {
                return Ok(false);
            };
            let (key, value) = item.map_err(GoalStoreReadError::from_sled)?;
            if key.len() <= std::mem::size_of::<u64>() {
                return Err(GoalStoreReadError::CorruptState(
                    "planning claim expiry key is truncated".to_string(),
                ));
            }
            let claim_id = std::str::from_utf8(&value)
                .map_err(|error| GoalStoreReadError::CorruptState(error.to_string()))?;
            let expected_key = planning_claim_expiry_key(
                u64::from_be_bytes(
                    key[..8]
                        .try_into()
                        .expect("planning claim expiry has a fixed width"),
                ),
                claim_id,
            );
            if expected_key.as_slice() != key.as_ref() {
                return Err(GoalStoreReadError::CorruptState(
                    "planning claim expiry entry is inconsistent".to_string(),
                ));
            }
            if self
                .planning_claims
                .get(claim_id.as_bytes())
                .map_err(GoalStoreReadError::from_sled)?
                .is_none()
            {
                self.repair_one_orphan_expiry(key.to_vec(), value.to_vec())?;
                continue;
            }
            self.release_planning_goal_claim(claim_id)?;
            return Ok(true);
        }
        Err(GoalStoreReadError::CorruptState(format!(
            "planning expiry index contains at least {MAX_ORPHAN_EXPIRY_REPAIRS_PER_SELECTION} orphan entries"
        )))
    }

    fn repair_one_orphan_expiry(
        &self,
        key: Vec<u8>,
        claim_key: Vec<u8>,
    ) -> Result<(), GoalStoreReadError> {
        (
            &self.planning_claims,
            &self.planning_claim_expiry,
            &self.planning_metadata,
        )
            .transaction(|(claims, expiry, metadata)| {
                if claims.get(claim_key.clone())?.is_some() {
                    return Err(ConflictableTransactionError::Abort(
                        GoalStoreReadError::SelectionContended,
                    ));
                }
                if expiry.get(key.clone())?.as_deref() != Some(claim_key.as_slice()) {
                    return Err(ConflictableTransactionError::Abort(
                        GoalStoreReadError::SelectionContended,
                    ));
                }
                expiry.remove(key.clone())?;
                let mut current: PlanningIndexMetadata = required_planning_transaction_value(
                    metadata,
                    KEY_PLANNING_INDEX_METADATA,
                    "planning metadata",
                )?;
                current.generation = current.generation.checked_add(1).ok_or_else(|| {
                    ConflictableTransactionError::Abort(GoalStoreReadError::CorruptState(
                        "planning index generation overflowed".to_string(),
                    ))
                })?;
                metadata.insert(
                    KEY_PLANNING_INDEX_METADATA,
                    encode_planning_transaction_value(&current, "planning metadata")?,
                )?;
                Ok(())
            })
            .map_err(planning_transaction_error)?;
        self.db.flush().map_err(GoalStoreReadError::from_sled)?;
        Ok(())
    }

    fn planning_goal_record(
        &self,
        goal_id: &str,
    ) -> Result<Option<ExecutionGoalRecord>, GoalStoreReadError> {
        let Some(value) = self
            .records
            .get(goal_id.as_bytes())
            .map_err(GoalStoreReadError::from_sled)?
        else {
            return Ok(None);
        };
        if value.len() > MAX_PLANNING_GOAL_RECORD_BYTES {
            return Err(GoalStoreReadError::CorruptState(format!(
                "planning goal '{goal_id}' exceeds {MAX_PLANNING_GOAL_RECORD_BYTES} bytes"
            )));
        }
        let record: ExecutionGoalRecord = serde_json::from_slice(&value)
            .map_err(|error| GoalStoreReadError::CorruptState(error.to_string()))?;
        validate_planning_goal_record(goal_id, &record)?;
        Ok(Some(record))
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

    /// Return one complete verified durable command result.
    ///
    /// Missing components fail closed because an identity, outcome, or receipt
    /// without its peers represents an incomplete authority transaction.
    pub fn command_commit(
        &self,
        command_id: &str,
    ) -> Result<Option<(GoalCommandOutcome, GoalCommandCommitReceipt)>, ExecutionInvariantError>
    {
        self.command_commit_with_snapshot_probe(command_id, || {})
    }

    fn command_commit_with_snapshot_probe<F>(
        &self,
        command_id: &str,
        after_identity_read: F,
    ) -> Result<Option<(GoalCommandOutcome, GoalCommandCommitReceipt)>, ExecutionInvariantError>
    where
        F: Fn(),
    {
        let command_key = command_id.as_bytes().to_vec();
        let (raw_identity, raw_outcome, raw_receipt) = (
            &self.command_identities,
            &self.command_outcomes,
            &self.command_receipts,
        )
            .transaction(
                |(identities, outcomes, receipts)| -> Result<_, GoalTransactionError> {
                    let identity = identities.get(command_key.clone())?;
                    after_identity_read();
                    Ok((
                        identity,
                        outcomes.get(command_key.clone())?,
                        receipts.get(command_key.clone())?,
                    ))
                },
            )
            .map_err(to_goal_transaction)?;

        // The snapshot can include a transaction that has not yet crossed a
        // durability boundary. Flush after the snapshot so every present
        // result returned below is covered by this observer's barrier.
        self.flush()?;
        let identity: Option<GoalCommandRequestIdentity> = decode_optional(raw_identity)?;
        let outcome = decode_optional(raw_outcome)?;
        let receipt = decode_optional(raw_receipt)?;
        match (identity, outcome, receipt) {
            (None, None, None) => Ok(None),
            (Some(identity), Some(outcome), Some(receipt)) => {
                if identity.command_id != command_id {
                    return Err(ExecutionInvariantError::ConfigError(format!(
                        "goal command '{command_id}' has a divergent request identity"
                    )));
                }
                let expected = commit_receipt(identity, &outcome)?;
                if receipt != expected {
                    return Err(ExecutionInvariantError::ConfigError(format!(
                        "goal command '{command_id}' has a divergent commit receipt"
                    )));
                }
                Ok(Some((outcome, receipt)))
            }
            _ => Err(ExecutionInvariantError::ConfigError(format!(
                "goal command '{command_id}' has an incomplete durable commit"
            ))),
        }
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
            &self.planning_active_index,
            &self.planning_available_index,
            &self.planning_claimed_index,
            &self.planning_claims,
            &self.planning_claim_members,
            &self.planning_claim_expiry,
            &self.planning_metadata,
        )
            .transaction(
                |(
                    records,
                    command_identities,
                    command_outcomes,
                    command_receipts,
                    planning_active,
                    planning_available,
                    planning_claimed,
                    planning_claims,
                    planning_members,
                    planning_expiry,
                    planning_metadata,
                )| {
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
                            let previous = record.clone();
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
                            PlanningIndexTransactionTrees {
                                active: planning_active,
                                available: planning_available,
                                claimed: planning_claimed,
                                claims: planning_claims,
                                members: planning_members,
                                expiry: planning_expiry,
                                metadata: planning_metadata,
                            }
                            .update(Some(&previous), Some(&record))?;
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

impl GoalPlanningSelectionPort for PersistentGoalSetStore {
    fn claim_active_goals(
        &self,
        limit: usize,
    ) -> Result<GoalPlanningSelection, GoalPlanningSelectionError> {
        let selection = self.select_planning_goal_records(limit)?;
        Ok(GoalPlanningSelection {
            records: selection.records,
            active_goal_count: selection.active_goal_count,
            budget_exhausted: selection.budget_exhausted,
            claim: selection.claim_id.map(GoalPlanningClaim::new),
        })
    }

    fn release_goal_claim(
        &self,
        claim: &GoalPlanningClaim,
    ) -> Result<(), GoalPlanningSelectionError> {
        self.release_planning_goal_claim(claim.claim_id())
    }

    fn planning_goal_record(
        &self,
        goal_id: &str,
    ) -> Result<Option<ExecutionGoalRecord>, GoalPlanningSelectionError> {
        PersistentGoalSetStore::planning_goal_record(self, goal_id)
    }
}

fn planning_record_hash(record: &ExecutionGoalRecord) -> Result<String, ExecutionInvariantError> {
    let bytes = serde_json::to_vec(record).map_err(to_store_data)?;
    if record.goal.goal_id.len() > MAX_PLANNING_GOAL_ID_BYTES {
        return Err(ExecutionInvariantError::ConfigError(format!(
            "planning goal id exceeds {MAX_PLANNING_GOAL_ID_BYTES} bytes"
        )));
    }
    if bytes.len() > MAX_PLANNING_GOAL_RECORD_BYTES {
        return Err(ExecutionInvariantError::ConfigError(format!(
            "planning goal record exceeds {MAX_PLANNING_GOAL_RECORD_BYTES} bytes"
        )));
    }
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn tree_keys(tree: &Tree) -> Result<Vec<Vec<u8>>, ExecutionInvariantError> {
    tree.iter()
        .map(|item| {
            let (key, _) = item.map_err(to_store_io)?;
            Ok(key.to_vec())
        })
        .collect()
}

fn tree_entries(tree: &Tree) -> Result<BTreeMap<Vec<u8>, Vec<u8>>, ExecutionInvariantError> {
    tree.iter()
        .map(|item| {
            let (key, value) = item.map_err(to_store_io)?;
            Ok((key.to_vec(), value.to_vec()))
        })
        .collect()
}

fn remove_transaction_keys(
    tree: &TransactionalTree,
    keys: &[Vec<u8>],
) -> Result<(), GoalTransactionError> {
    for key in keys {
        tree.remove(key.clone())?;
    }
    Ok(())
}

fn decode_required_tree_value<T: serde::de::DeserializeOwned>(
    tree: &Tree,
    key: &[u8],
    name: &str,
) -> Result<T, ExecutionInvariantError> {
    let raw = tree
        .get(key)
        .map_err(to_store_io)?
        .ok_or_else(|| ExecutionInvariantError::ConfigError(format!("{name} is missing")))?;
    serde_json::from_slice(&raw).map_err(to_store_data)
}

fn decode_planning_candidate(
    order_key: Vec<u8>,
    entry_value: Vec<u8>,
) -> Result<PlanningCandidate, GoalStoreReadError> {
    let key = PlanningGoalOrderKey::decode(&order_key)?;
    let entry: PlanningIndexEntry = decode_planning_value(&entry_value, "planning index entry")?;
    if entry.schema_version != PLANNING_INDEX_SCHEMA_VERSION || entry.goal_id != key.goal_id {
        return Err(GoalStoreReadError::CorruptState(
            "planning index key and entry are inconsistent".to_string(),
        ));
    }
    Ok(PlanningCandidate {
        order_key,
        entry_value,
        entry,
    })
}

fn validate_planning_selection_state(
    state: &PlanningSelectionState,
) -> Result<(), GoalStoreReadError> {
    if state.schema_version != PLANNING_INDEX_SCHEMA_VERSION {
        return Err(GoalStoreReadError::CorruptState(format!(
            "planning selection state schema version {} is unsupported",
            state.schema_version
        )));
    }
    if let Some(cursor) = &state.cursor {
        PlanningGoalOrderKey::decode(cursor)?;
    }
    Ok(())
}

fn validate_planning_claim(
    durable_claim_id: &str,
    claim: &PlanningClaim,
) -> Result<(), GoalStoreReadError> {
    if claim.schema_version != PLANNING_INDEX_SCHEMA_VERSION
        || claim.claim_id != durable_claim_id
        || uuid::Uuid::parse_str(&claim.claim_id).is_err()
        || claim.generation == 0
        || claim.expires_at_millis == 0
        || claim.member_count == 0
        || claim.member_count > MAX_PLANNING_SELECTION_LIMIT
    {
        return Err(GoalStoreReadError::CorruptState(format!(
            "planning claim '{}' is invalid",
            durable_claim_id
        )));
    }
    Ok(())
}

fn validate_planning_claimed_entry(
    order_key: &[u8],
    claimed: &PlanningClaimedEntry,
    expected: &BTreeMap<Vec<u8>, Vec<u8>>,
) -> Result<(), ExecutionInvariantError> {
    if claimed.schema_version != PLANNING_INDEX_SCHEMA_VERSION
        || uuid::Uuid::parse_str(&claimed.claim_id).is_err()
    {
        return Err(ExecutionInvariantError::ConfigError(
            "planning claimed index entry is invalid".to_string(),
        ));
    }
    let expected_entry = expected.get(order_key).ok_or_else(|| {
        ExecutionInvariantError::ConfigError(
            "planning claimed index references an inactive goal".to_string(),
        )
    })?;
    let entry: PlanningIndexEntry =
        serde_json::from_slice(expected_entry).map_err(to_store_data)?;
    if claimed.entry != entry {
        return Err(ExecutionInvariantError::ConfigError(
            "planning claimed index entry diverges from active index".to_string(),
        ));
    }
    Ok(())
}

fn validate_planning_claim_trees(
    store: &PersistentGoalSetStore,
) -> Result<(), ExecutionInvariantError> {
    let claims = tree_entries(&store.planning_claims)?;
    let members = tree_entries(&store.planning_claim_members)?;
    let claimed = tree_entries(&store.planning_claimed_index)?;
    let expiry = tree_entries(&store.planning_claim_expiry)?;
    let mut covered = std::collections::BTreeSet::new();
    let mut covered_members = std::collections::BTreeSet::new();
    let mut covered_expiry = std::collections::BTreeSet::new();
    for (claim_key, claim_value) in claims {
        let claim_id = std::str::from_utf8(&claim_key)
            .map_err(|error| ExecutionInvariantError::ConfigError(error.to_string()))?;
        let claim: PlanningClaim = serde_json::from_slice(&claim_value).map_err(to_store_data)?;
        validate_planning_claim(claim_id, &claim)
            .map_err(|error| ExecutionInvariantError::ConfigError(error.to_string()))?;
        let prefix = planning_claim_member_prefix(claim_id);
        let claim_members = members
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .collect::<Vec<_>>();
        if claim_members.len() != claim.member_count {
            return Err(ExecutionInvariantError::ConfigError(format!(
                "planning claim '{}' member count is inconsistent",
                claim_id
            )));
        }
        for (member_key, member_value) in claim_members {
            let order_key = planning_order_key_from_member_read(&prefix, member_key)?;
            let raw_claimed = claimed.get(&order_key).ok_or_else(|| {
                ExecutionInvariantError::ConfigError(
                    "planning claim member has no claimed index entry".to_string(),
                )
            })?;
            let claimed_entry: PlanningClaimedEntry =
                serde_json::from_slice(raw_claimed).map_err(to_store_data)?;
            let member_entry: PlanningIndexEntry =
                serde_json::from_slice(member_value).map_err(to_store_data)?;
            if claimed_entry.claim_id != claim_id || claimed_entry.entry != member_entry {
                return Err(ExecutionInvariantError::ConfigError(
                    "planning claim membership diverges from claimed index".to_string(),
                ));
            }
            covered.insert(order_key);
            covered_members.insert(member_key.clone());
        }
        let expiry_key = planning_claim_expiry_key(claim.expires_at_millis, claim_id);
        if expiry.get(&expiry_key).map(Vec::as_slice) != Some(claim_key.as_slice()) {
            return Err(ExecutionInvariantError::ConfigError(
                "planning claim expiry index is inconsistent".to_string(),
            ));
        }
        covered_expiry.insert(expiry_key);
    }
    if covered != claimed.keys().cloned().collect() {
        return Err(ExecutionInvariantError::ConfigError(
            "planning claimed index contains unowned entries".to_string(),
        ));
    }
    if covered_members != members.keys().cloned().collect() {
        return Err(ExecutionInvariantError::ConfigError(
            "planning claim member index contains orphan entries".to_string(),
        ));
    }
    if covered_expiry != expiry.keys().cloned().collect() {
        return Err(ExecutionInvariantError::ConfigError(
            "planning claim expiry index contains orphan entries".to_string(),
        ));
    }
    Ok(())
}

fn planning_claim_member_prefix(claim_id: &str) -> Vec<u8> {
    let mut key = claim_id.as_bytes().to_vec();
    key.push(0);
    key
}

fn planning_claim_member_key(claim_id: &str, order_key: &[u8]) -> Vec<u8> {
    let mut key = planning_claim_member_prefix(claim_id);
    key.extend_from_slice(order_key);
    key
}

fn planning_order_key_from_member(
    prefix: &[u8],
    member_key: &[u8],
) -> Result<Vec<u8>, ConflictableTransactionError<GoalStoreReadError>> {
    member_key
        .strip_prefix(prefix)
        .filter(|key| !key.is_empty())
        .map(Vec::from)
        .ok_or_else(|| {
            ConflictableTransactionError::Abort(GoalStoreReadError::CorruptState(
                "planning claim member key is invalid".to_string(),
            ))
        })
}

fn planning_order_key_from_member_read(
    prefix: &[u8],
    member_key: &[u8],
) -> Result<Vec<u8>, ExecutionInvariantError> {
    member_key
        .strip_prefix(prefix)
        .filter(|key| !key.is_empty())
        .map(Vec::from)
        .ok_or_else(|| {
            ExecutionInvariantError::ConfigError("planning claim member key is invalid".to_string())
        })
}

fn planning_claim_expiry_key(expires_at_millis: u64, claim_id: &str) -> Vec<u8> {
    let mut key = expires_at_millis.to_be_bytes().to_vec();
    key.extend_from_slice(claim_id.as_bytes());
    key
}

fn now_millis() -> Result<u64, GoalStoreReadError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            GoalStoreReadError::CorruptState(format!("system clock precedes Unix epoch: {error}"))
        })?;
    u64::try_from(duration.as_millis()).map_err(|_| {
        GoalStoreReadError::CorruptState("system clock millisecond value overflowed".to_string())
    })
}

fn encode_planning_value<T: serde::Serialize>(
    value: &T,
    name: &str,
) -> Result<Vec<u8>, GoalStoreReadError> {
    serde_json::to_vec(value)
        .map_err(|error| GoalStoreReadError::CorruptState(format!("{name}: {error}")))
}

fn decode_planning_value<T: serde::de::DeserializeOwned>(
    value: &[u8],
    name: &str,
) -> Result<T, GoalStoreReadError> {
    serde_json::from_slice(value)
        .map_err(|error| GoalStoreReadError::CorruptState(format!("{name}: {error}")))
}

fn encode_planning_transaction_value<T: serde::Serialize>(
    value: &T,
    name: &str,
) -> Result<Vec<u8>, ConflictableTransactionError<GoalStoreReadError>> {
    encode_planning_value(value, name).map_err(ConflictableTransactionError::Abort)
}

fn decode_planning_transaction_value<T: serde::de::DeserializeOwned>(
    value: &[u8],
    name: &str,
) -> Result<T, ConflictableTransactionError<GoalStoreReadError>> {
    decode_planning_value(value, name).map_err(ConflictableTransactionError::Abort)
}

fn required_planning_transaction_value<T: serde::de::DeserializeOwned>(
    tree: &TransactionalTree,
    key: &[u8],
    name: &str,
) -> Result<T, ConflictableTransactionError<GoalStoreReadError>> {
    let value = tree.get(key)?.ok_or_else(|| {
        ConflictableTransactionError::Abort(GoalStoreReadError::CorruptState(format!(
            "{name} is missing"
        )))
    })?;
    decode_planning_transaction_value(&value, name)
}

fn planning_transaction_error(error: TransactionError<GoalStoreReadError>) -> GoalStoreReadError {
    match error {
        TransactionError::Abort(error) => error,
        TransactionError::Storage(error) => GoalStoreReadError::from_sled(error),
    }
}

struct PlanningIndexTransactionTrees<'a> {
    active: &'a TransactionalTree,
    available: &'a TransactionalTree,
    claimed: &'a TransactionalTree,
    claims: &'a TransactionalTree,
    members: &'a TransactionalTree,
    expiry: &'a TransactionalTree,
    metadata: &'a TransactionalTree,
}

impl PlanningIndexTransactionTrees<'_> {
    fn update(
        self,
        previous: Option<&ExecutionGoalRecord>,
        current: Option<&ExecutionGoalRecord>,
    ) -> Result<(), GoalTransactionError> {
        let mut metadata: PlanningIndexMetadata = required_goal_transaction_value(
            self.metadata,
            KEY_PLANNING_INDEX_METADATA,
            "planning index metadata",
        )?;
        if metadata.schema_version != PLANNING_INDEX_SCHEMA_VERSION {
            return Err(goal_transaction_config(format!(
                "planning index schema version {} is unsupported",
                metadata.schema_version
            )));
        }
        if previous.is_some_and(|record| matches!(record.goal.lifecycle, GoalLifecycle::Active)) {
            let previous = previous.expect("active planning record is present");
            self.remove_active(previous, &mut metadata)?;
        }
        if current.is_some_and(|record| matches!(record.goal.lifecycle, GoalLifecycle::Active)) {
            let current = current.expect("active planning record is present");
            self.insert_active(current, &mut metadata)?;
        }
        metadata.generation = metadata
            .generation
            .checked_add(1)
            .ok_or_else(|| goal_transaction_config("planning index generation overflowed"))?;
        self.metadata
            .insert(KEY_PLANNING_INDEX_METADATA, encode_transaction(&metadata)?)?;
        Ok(())
    }

    fn remove_active(
        &self,
        previous: &ExecutionGoalRecord,
        metadata: &mut PlanningIndexMetadata,
    ) -> Result<(), GoalTransactionError> {
        let order_key = PlanningGoalOrderKey::for_record(previous).encode();
        let entry = PlanningIndexEntry::for_record(previous)
            .map_err(ConflictableTransactionError::Abort)?;
        let entry_value = encode_transaction(&entry)?;
        if self.active.get(order_key.clone())?.as_deref() != Some(entry_value.as_slice()) {
            return Err(goal_transaction_config(format!(
                "planning active index diverged for goal '{}'",
                previous.goal.goal_id
            )));
        }
        self.active.remove(order_key.clone())?;
        if self.available.get(order_key.clone())?.as_deref() == Some(entry_value.as_slice()) {
            self.available.remove(order_key)?;
        } else {
            self.remove_claimed(previous, order_key, entry, entry_value)?;
        }
        metadata.active_count = metadata
            .active_count
            .checked_sub(1)
            .ok_or_else(|| goal_transaction_config("planning active count underflowed"))?;
        Ok(())
    }

    fn remove_claimed(
        &self,
        previous: &ExecutionGoalRecord,
        order_key: Vec<u8>,
        entry: PlanningIndexEntry,
        entry_value: Vec<u8>,
    ) -> Result<(), GoalTransactionError> {
        let raw_claimed = self.claimed.get(order_key.clone())?.ok_or_else(|| {
            goal_transaction_config(format!(
                "planning goal '{}' is neither available nor claimed",
                previous.goal.goal_id
            ))
        })?;
        let claimed_entry: PlanningClaimedEntry = decode_transaction(&raw_claimed)?;
        if claimed_entry.schema_version != PLANNING_INDEX_SCHEMA_VERSION
            || claimed_entry.entry != entry
        {
            return Err(goal_transaction_config(format!(
                "planning claimed index diverged for goal '{}'",
                previous.goal.goal_id
            )));
        }
        let claim_key = claimed_entry.claim_id.as_bytes().to_vec();
        let raw_claim = self
            .claims
            .get(claim_key.clone())?
            .ok_or_else(|| goal_transaction_config("planning claimed entry has no owning claim"))?;
        let mut claim: PlanningClaim = decode_transaction(&raw_claim)?;
        validate_planning_claim(&claimed_entry.claim_id, &claim)
            .map_err(|error| goal_transaction_config(error.to_string()))?;
        let member_key = planning_claim_member_key(&claimed_entry.claim_id, &order_key);
        if self.members.get(member_key.clone())?.as_deref() != Some(entry_value.as_slice()) {
            return Err(goal_transaction_config(
                "planning claim membership diverged during goal mutation",
            ));
        }
        self.claimed.remove(order_key)?;
        self.members.remove(member_key)?;
        claim.member_count -= 1;
        if claim.member_count == 0 {
            self.claims.remove(claim_key)?;
            self.expiry.remove(planning_claim_expiry_key(
                claim.expires_at_millis,
                &claim.claim_id,
            ))?;
        } else {
            self.claims.insert(claim_key, encode_transaction(&claim)?)?;
        }
        Ok(())
    }

    fn insert_active(
        &self,
        current: &ExecutionGoalRecord,
        metadata: &mut PlanningIndexMetadata,
    ) -> Result<(), GoalTransactionError> {
        let order_key = PlanningGoalOrderKey::for_record(current).encode();
        if self.active.get(order_key.clone())?.is_some()
            || self.available.get(order_key.clone())?.is_some()
            || self.claimed.get(order_key.clone())?.is_some()
        {
            return Err(goal_transaction_config(format!(
                "planning index already contains goal '{}'",
                current.goal.goal_id
            )));
        }
        let entry =
            PlanningIndexEntry::for_record(current).map_err(ConflictableTransactionError::Abort)?;
        let entry_value = encode_transaction(&entry)?;
        self.active.insert(order_key.clone(), entry_value.clone())?;
        self.available.insert(order_key, entry_value)?;
        metadata.active_count = metadata
            .active_count
            .checked_add(1)
            .ok_or_else(|| goal_transaction_config("planning active count overflowed"))?;
        Ok(())
    }
}

fn required_goal_transaction_value<T: serde::de::DeserializeOwned>(
    tree: &TransactionalTree,
    key: &[u8],
    name: &str,
) -> Result<T, GoalTransactionError> {
    let raw = tree
        .get(key)?
        .ok_or_else(|| goal_transaction_config(format!("{name} is missing")))?;
    decode_transaction(&raw)
}

fn goal_transaction_config(
    message: impl Into<String>,
) -> ConflictableTransactionError<ExecutionInvariantError> {
    ConflictableTransactionError::Abort(ExecutionInvariantError::ConfigError(message.into()))
}

fn validate_planning_goal_record(
    durable_key: &str,
    record: &ExecutionGoalRecord,
) -> Result<(), GoalStoreReadError> {
    if durable_key != record.goal.goal_id {
        return Err(GoalStoreReadError::CorruptState(format!(
            "goal record key '{durable_key}' does not match embedded goal id '{}'",
            record.goal.goal_id
        )));
    }
    validate_goal(&record.goal)
        .map_err(|error| GoalStoreReadError::CorruptState(error.to_string()))?;
    if record.created_at_seq == 0 || record.updated_at_seq == 0 {
        return Err(GoalStoreReadError::CorruptState(format!(
            "goal '{}' durable sequences must be greater than zero",
            record.goal.goal_id
        )));
    }
    if record.created_at_seq > record.updated_at_seq {
        return Err(GoalStoreReadError::CorruptState(format!(
            "goal '{}' created sequence {} exceeds updated sequence {}",
            record.goal.goal_id, record.created_at_seq, record.updated_at_seq
        )));
    }
    if let Some(command_id) = &record.source_command_id {
        validate_non_empty("goal source command id", command_id)
            .map_err(|error| GoalStoreReadError::CorruptState(error.to_string()))?;
    }
    if let Some(source_identity) = &record.source_identity {
        validate_non_empty("goal source identity", source_identity)
            .map_err(|error| GoalStoreReadError::CorruptState(error.to_string()))?;
    }
    Ok(())
}

// TODO compat-shim: remove this replay branch after the minimum supported
// goal-store schema requires request identities beside every command outcome.
// It preserves verified replay for outcomes written before request hashes
// existed. Before deletion, keep
// compatible_legacy_applied_outcome_is_verified_and_upgraded and
// strict_legacy_policy_rejects_unverified_outcome green while proving all
// supported stores have completed the identity upgrade.
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

#[cfg(test)]
mod tests {
    use super::*;
    use meld_events::DomainObjectRef;
    use meld_lang::{GoalPriority, GoalSource, Proposition, Term};

    fn active_goal_record() -> ExecutionGoalRecord {
        ExecutionGoalRecord {
            goal: Goal {
                goal_id: "goal-a".to_string(),
                agent_id: "agent-a".to_string(),
                target: Proposition::Accessible {
                    scope: Term::Object(
                        DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap(),
                    ),
                },
                priority: GoalPriority {
                    urgency: 1,
                    cost_ceiling: None,
                },
                source: GoalSource::UserDirected {
                    directive: "inspect node-a".to_string(),
                },
                lifecycle: GoalLifecycle::Active,
            },
            source_command_id: Some("command-a".to_string()),
            source_identity: Some("source-a".to_string()),
            created_at_seq: 1,
            updated_at_seq: 1,
        }
    }

    #[test]
    fn planning_record_validation_rejects_key_lifecycle_and_sequence_corruption() {
        let record = active_goal_record();
        validate_planning_goal_record("goal-a", &record).unwrap();

        assert!(validate_planning_goal_record("goal-other", &record).is_err());

        let mut invalid_sequence = record.clone();
        invalid_sequence.updated_at_seq = 0;
        assert!(validate_planning_goal_record("goal-a", &invalid_sequence).is_err());

        let mut reversed_sequence = record.clone();
        reversed_sequence.created_at_seq = 2;
        assert!(validate_planning_goal_record("goal-a", &reversed_sequence).is_err());

        let mut invalid_lifecycle = record;
        invalid_lifecycle.goal.lifecycle = GoalLifecycle::Suspended {
            reason: " ".to_string(),
        };
        assert!(validate_planning_goal_record("goal-a", &invalid_lifecycle).is_err());
    }

    #[test]
    fn concurrent_planning_selections_claim_disjoint_bounded_windows() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = Arc::new(PersistentGoalSetStore::new(db).unwrap());
        for index in 0..4 {
            let mut goal = active_goal_record().goal;
            goal.goal_id = format!("goal-{index}");
            store
                .add_goal(AddGoalCommand {
                    metadata: GoalCommandMetadata {
                        command_id: format!("command-{index}"),
                        source_identity: None,
                        seq: index + 1,
                    },
                    goal,
                })
                .unwrap();
        }
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let handles = (0..2)
            .map(|_| {
                let store = Arc::clone(&store);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    store
                        .select_planning_goal_records(2)
                        .unwrap()
                        .records
                        .into_iter()
                        .map(|record| record.goal.goal_id)
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        let mut selected = handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        selected.sort();

        assert_eq!(selected, vec!["goal-0", "goal-1", "goal-2", "goal-3"]);
    }

    #[test]
    fn planning_index_migrates_legacy_records_and_reopens_with_valid_count() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let record = active_goal_record();
        db.open_tree(TREE_RECORDS)
            .unwrap()
            .insert(
                record.goal.goal_id.as_bytes(),
                serde_json::to_vec(&record).unwrap(),
            )
            .unwrap();
        db.open_tree(TREE_PLANNING_SELECTION)
            .unwrap()
            .insert(
                b"cursor",
                &br#"{"schema_version":1,"urgency":1,"goal_id":"goal-a"}"#[..],
            )
            .unwrap();
        db.flush().unwrap();

        let store = PersistentGoalSetStore::new(db.clone()).unwrap();
        let selection = store.select_planning_goal_records(1).unwrap();
        assert_eq!(selection.active_goal_count, 1);
        assert_eq!(selection.records[0].goal.goal_id, "goal-a");
        store
            .release_planning_goal_claim(selection.claim_id.as_deref().unwrap())
            .unwrap();
        drop(store);

        let reopened = PersistentGoalSetStore::new(db).unwrap();
        assert_eq!(reopened.planning_index_metadata().unwrap().active_count, 1);
    }

    #[test]
    fn concurrent_planning_index_migration_installs_one_exact_transactional_index() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let record = active_goal_record();
        db.open_tree(TREE_RECORDS)
            .unwrap()
            .insert(
                record.goal.goal_id.as_bytes(),
                serde_json::to_vec(&record).unwrap(),
            )
            .unwrap();
        db.flush().unwrap();
        let barrier = Arc::new(std::sync::Barrier::new(4));
        let handles = (0..4)
            .map(|_| {
                let db = db.clone();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    PersistentGoalSetStore::new(db)
                })
            })
            .collect::<Vec<_>>();
        let stores = handles
            .into_iter()
            .map(|handle| handle.join().unwrap().unwrap())
            .collect::<Vec<_>>();

        for store in stores {
            assert_eq!(store.planning_index_metadata().unwrap().active_count, 1);
            let selection = store.select_planning_goal_records(1).unwrap();
            if let Some(claim_id) = selection.claim_id {
                store.release_planning_goal_claim(&claim_id).unwrap();
            }
        }
    }

    #[test]
    fn planning_index_corruption_fails_reopen_validation() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = PersistentGoalSetStore::new(db.clone()).unwrap();
        let record = active_goal_record();
        store
            .add_goal(AddGoalCommand {
                metadata: GoalCommandMetadata {
                    command_id: "command-a".to_string(),
                    source_identity: None,
                    seq: 1,
                },
                goal: record.goal,
            })
            .unwrap();
        store
            .planning_active_index
            .insert(
                PlanningGoalOrderKey {
                    urgency: 1,
                    goal_id: "goal-a".to_string(),
                }
                .encode(),
                b"not-json",
            )
            .unwrap();
        store.flush().unwrap();
        drop(store);

        let error = match PersistentGoalSetStore::new(db) {
            Ok(_) => panic!("corrupt planning index must fail reopen"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("planning active index diverges"));
    }

    #[test]
    fn bounded_selection_does_not_decode_inactive_history_during_tick() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = PersistentGoalSetStore::new(db.clone()).unwrap();
        let record = active_goal_record();
        store
            .add_goal(AddGoalCommand {
                metadata: GoalCommandMetadata {
                    command_id: "command-a".to_string(),
                    source_identity: None,
                    seq: 1,
                },
                goal: record.goal,
            })
            .unwrap();
        db.open_tree(TREE_RECORDS)
            .unwrap()
            .insert(b"inactive-corrupt-history", b"not-json")
            .unwrap();

        let selection = store.select_planning_goal_records(1).unwrap();

        assert_eq!(selection.records.len(), 1);
        assert_eq!(selection.records[0].goal.goal_id, "goal-a");
    }

    #[test]
    fn concurrent_planning_claims_remain_disjoint_across_forced_wrap() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = Arc::new(PersistentGoalSetStore::new(db).unwrap());
        for index in 0..4 {
            let mut goal = active_goal_record().goal;
            goal.goal_id = format!("goal-{index}");
            store
                .add_goal(AddGoalCommand {
                    metadata: GoalCommandMetadata {
                        command_id: format!("command-{index}"),
                        source_identity: None,
                        seq: index + 1,
                    },
                    goal,
                })
                .unwrap();
        }
        let initial = store.select_planning_goal_records(3).unwrap();
        store
            .release_planning_goal_claim(initial.claim_id.as_deref().unwrap())
            .unwrap();
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let handles = (0..2)
            .map(|_| {
                let store = Arc::clone(&store);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    store.select_planning_goal_records(2).unwrap()
                })
            })
            .collect::<Vec<_>>();
        let selections = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();
        let first = selections[0]
            .records
            .iter()
            .map(|record| record.goal.goal_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let second = selections[1]
            .records
            .iter()
            .map(|record| record.goal.goal_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let all = first.union(&second).copied().collect::<Vec<_>>();

        assert!(first.is_disjoint(&second));
        assert_eq!(all, vec!["goal-0", "goal-1", "goal-2", "goal-3"]);
        assert!(selections.iter().any(|selection| {
            let ids = selection
                .records
                .iter()
                .map(|record| record.goal.goal_id.as_str())
                .collect::<Vec<_>>();
            ids == vec!["goal-3", "goal-0"]
        }));
    }

    #[test]
    fn reopen_validation_is_stable_during_concurrent_claim_release_churn() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = Arc::new(PersistentGoalSetStore::new(db.clone()).unwrap());
        for index in 0..8 {
            let mut goal = active_goal_record().goal;
            goal.goal_id = format!("goal-{index}");
            store
                .add_goal(AddGoalCommand {
                    metadata: GoalCommandMetadata {
                        command_id: format!("command-{index}"),
                        source_identity: None,
                        seq: index + 1,
                    },
                    goal,
                })
                .unwrap();
        }
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let churn_store = Arc::clone(&store);
        let churn_barrier = Arc::clone(&barrier);
        let churn = std::thread::spawn(move || {
            churn_barrier.wait();
            for _ in 0..128 {
                let selection = churn_store.select_planning_goal_records(2).unwrap();
                churn_store
                    .release_planning_goal_claim(selection.claim_id.as_deref().unwrap())
                    .unwrap();
                std::thread::yield_now();
            }
        });
        barrier.wait();
        for _ in 0..64 {
            let reopened = PersistentGoalSetStore::new(db.clone()).unwrap();
            assert_eq!(reopened.planning_index_metadata().unwrap().active_count, 8);
            std::thread::yield_now();
        }
        churn.join().unwrap();
    }

    #[test]
    fn reopen_repairs_orphan_claim_member_and_expiry_indexes() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = PersistentGoalSetStore::new(db.clone()).unwrap();
        let record = active_goal_record();
        store
            .add_goal(AddGoalCommand {
                metadata: GoalCommandMetadata {
                    command_id: "command-a".to_string(),
                    source_identity: None,
                    seq: 1,
                },
                goal: record.goal,
            })
            .unwrap();
        let orphan = uuid::Uuid::new_v4().to_string();
        store
            .planning_claim_members
            .insert(
                planning_claim_member_key(&orphan, b"orphan-order"),
                b"orphan-member",
            )
            .unwrap();
        store
            .planning_claim_expiry
            .insert(planning_claim_expiry_key(1, &orphan), orphan.as_bytes())
            .unwrap();
        store.flush().unwrap();
        drop(store);

        let reopened = PersistentGoalSetStore::new(db).unwrap();

        assert!(reopened.planning_claim_members.is_empty());
        assert!(reopened.planning_claim_expiry.is_empty());
        let selection = reopened.select_planning_goal_records(1).unwrap();
        assert_eq!(selection.records[0].goal.goal_id, "goal-a");
    }

    #[test]
    fn orphan_expiry_cannot_mask_a_legitimate_expired_claim_during_selection() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = PersistentGoalSetStore::new(db).unwrap();
        let record = active_goal_record();
        store
            .add_goal(AddGoalCommand {
                metadata: GoalCommandMetadata {
                    command_id: "command-a".to_string(),
                    source_identity: None,
                    seq: 1,
                },
                goal: record.goal,
            })
            .unwrap();
        let claimed = store.select_planning_goal_records(1).unwrap();
        let claim_id = claimed.claim_id.unwrap();
        let raw_claim = store.planning_claims.get(&claim_id).unwrap().unwrap();
        let mut claim: PlanningClaim = serde_json::from_slice(&raw_claim).unwrap();
        store
            .planning_claim_expiry
            .remove(planning_claim_expiry_key(
                claim.expires_at_millis,
                &claim_id,
            ))
            .unwrap();
        claim.expires_at_millis = 1;
        store
            .planning_claims
            .insert(&claim_id, serde_json::to_vec(&claim).unwrap())
            .unwrap();
        store
            .planning_claim_expiry
            .insert(planning_claim_expiry_key(1, &claim_id), claim_id.as_bytes())
            .unwrap();
        let orphan = uuid::Uuid::new_v4().to_string();
        store
            .planning_claim_expiry
            .insert(planning_claim_expiry_key(0, &orphan), orphan.as_bytes())
            .unwrap();
        store.flush().unwrap();

        let selection = store.select_planning_goal_records(1).unwrap();

        assert_eq!(selection.records[0].goal.goal_id, "goal-a");
        assert!(store
            .planning_claim_expiry
            .get(planning_claim_expiry_key(0, &orphan))
            .unwrap()
            .is_none());
    }

    fn add_command(command_id: &str) -> AddGoalCommand {
        AddGoalCommand {
            metadata: GoalCommandMetadata {
                command_id: command_id.to_string(),
                source_identity: None,
                seq: 1,
            },
            goal: active_goal_record().goal,
        }
    }

    #[test]
    fn strict_legacy_policy_rejects_unverified_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let db = sled::open(dir.path().join("goals")).unwrap();
        let goal = Goal {
            goal_id: "goal-a".to_string(),
            agent_id: "agent-a".to_string(),
            target: Proposition::Accessible {
                scope: Term::Object(
                    DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap(),
                ),
            },
            priority: GoalPriority {
                urgency: 1,
                cost_ceiling: None,
            },
            source: GoalSource::UserDirected {
                directive: "inspect node-a".to_string(),
            },
            lifecycle: GoalLifecycle::Active,
        };
        let command = AddGoalCommand {
            metadata: GoalCommandMetadata {
                command_id: "cmd-legacy".to_string(),
                source_identity: None,
                seq: 1,
            },
            goal: goal.clone(),
        };
        let record = ExecutionGoalRecord {
            goal,
            source_command_id: Some(command.metadata.command_id.clone()),
            source_identity: None,
            created_at_seq: 1,
            updated_at_seq: 1,
        };
        db.open_tree(TREE_COMMAND_OUTCOMES)
            .unwrap()
            .insert(
                "cmd-legacy",
                serde_json::to_vec(&GoalCommandOutcome::Applied(Box::new(record))).unwrap(),
            )
            .unwrap();
        db.flush().unwrap();
        let store = PersistentGoalSetStore::with_legacy_replay_policy(
            db,
            LegacyGoalCommandReplayPolicy::RejectUnverified,
        )
        .unwrap();

        let error = store.add_goal(command).unwrap_err();

        assert!(error
            .to_string()
            .contains("has no verified request identity"));
        assert!(store.command_identity("cmd-legacy").unwrap().is_none());
    }

    #[test]
    fn command_commit_returns_one_verified_authority_result() {
        let dir = tempfile::tempdir().unwrap();
        let store = PersistentGoalSetStore::new(sled::open(dir.path()).unwrap()).unwrap();
        let expected = store.add_goal(add_command("cmd-a")).unwrap();

        let (outcome, receipt) = store.command_commit("cmd-a").unwrap().unwrap();

        assert_eq!(outcome, expected);
        assert_eq!(receipt.identity.command_id, "cmd-a");
        assert!(store.command_commit("missing").unwrap().is_none());
    }

    #[test]
    fn command_commit_fails_closed_on_incomplete_or_divergent_products() {
        let dir = tempfile::tempdir().unwrap();
        let store = PersistentGoalSetStore::new(sled::open(dir.path()).unwrap()).unwrap();
        store.add_goal(add_command("cmd-a")).unwrap();
        let mut receipt = store.command_receipt("cmd-a").unwrap().unwrap();
        receipt.outcome_hash = "divergent".to_string();
        store
            .command_receipts
            .insert("cmd-a", serde_json::to_vec(&receipt).unwrap())
            .unwrap();

        let error = store.command_commit("cmd-a").unwrap_err();

        assert!(error.to_string().contains("divergent commit receipt"));

        store.command_receipts.remove("cmd-a").unwrap();
        let error = store.command_commit("cmd-a").unwrap_err();
        assert!(error.to_string().contains("incomplete durable commit"));
    }

    #[test]
    fn command_commit_rejects_divergent_lookup_identity_without_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let store = PersistentGoalSetStore::new(sled::open(dir.path()).unwrap()).unwrap();
        store.add_goal(add_command("cmd-a")).unwrap();
        let identity = store.command_identity("cmd-a").unwrap().unwrap();
        let (outcome, receipt) = store.command_commit("cmd-a").unwrap().unwrap();
        store
            .command_identities
            .insert("cmd-alias", serde_json::to_vec(&identity).unwrap())
            .unwrap();
        store
            .command_outcomes
            .insert("cmd-alias", serde_json::to_vec(&outcome).unwrap())
            .unwrap();
        store
            .command_receipts
            .insert("cmd-alias", serde_json::to_vec(&receipt).unwrap())
            .unwrap();
        store.flush().unwrap();
        let before = (
            store.command_identities.get("cmd-alias").unwrap(),
            store.command_outcomes.get("cmd-alias").unwrap(),
            store.command_receipts.get("cmd-alias").unwrap(),
        );

        let error = store.command_commit("cmd-alias").unwrap_err();

        assert!(matches!(
            error,
            ExecutionInvariantError::ConfigError(message)
                if message.contains("divergent request identity")
        ));
        let after = (
            store.command_identities.get("cmd-alias").unwrap(),
            store.command_outcomes.get("cmd-alias").unwrap(),
            store.command_receipts.get("cmd-alias").unwrap(),
        );
        assert_eq!(after, before);
    }

    #[test]
    fn command_commit_atomically_snapshots_and_flushes_a_concurrent_commit() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::mpsc::sync_channel;

        let dir = tempfile::tempdir().unwrap();
        let db = sled::Config::new()
            .path(dir.path().join("goals"))
            .flush_every_ms(None)
            .open()
            .unwrap();
        let store = PersistentGoalSetStore::new(db.clone()).unwrap();
        let command = add_command("cmd-race");
        let identity = request_identity(&command).unwrap();
        let observer = store.clone();
        let writer = store.clone();
        let probe_called = Arc::new(AtomicBool::new(false));
        let reader_probe_called = Arc::clone(&probe_called);
        let (snapshot_started_tx, snapshot_started_rx) = sync_channel(0);
        let (snapshot_resume_tx, snapshot_resume_rx) = sync_channel(0);
        let reader = std::thread::spawn(move || {
            observer.command_commit_with_snapshot_probe("cmd-race", || {
                if !reader_probe_called.swap(true, Ordering::SeqCst) {
                    snapshot_started_tx.send(()).unwrap();
                    snapshot_resume_rx.recv().unwrap();
                }
            })
        });

        snapshot_started_rx.recv().unwrap();
        let (writer_started_tx, writer_started_rx) = sync_channel(0);
        let writer = std::thread::spawn(move || {
            writer_started_tx.send(()).unwrap();
            writer
                .commit_add_goal_with_identity(command, identity)
                .unwrap()
        });
        writer_started_rx.recv().unwrap();
        snapshot_resume_tx.send(()).unwrap();

        let first_observation = reader.join().unwrap().unwrap();
        let expected = writer.join().unwrap();
        let before_complete_observation = (
            store.command_identities.get("cmd-race").unwrap(),
            store.command_outcomes.get("cmd-race").unwrap(),
            store.command_receipts.get("cmd-race").unwrap(),
        );
        let complete_observation = store.command_commit("cmd-race").unwrap().unwrap();
        let after_complete_observation = (
            store.command_identities.get("cmd-race").unwrap(),
            store.command_outcomes.get("cmd-race").unwrap(),
            store.command_receipts.get("cmd-race").unwrap(),
        );

        assert!(first_observation.is_none());
        assert!(probe_called.load(Ordering::SeqCst));
        assert_eq!(complete_observation, expected);
        assert_eq!(after_complete_observation, before_complete_observation);
        assert_eq!(db.flush().unwrap(), 0);
    }
}
