//! Durable agent storage.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::ops::Bound;
use std::sync::Arc;

#[cfg(test)]
use parking_lot::Mutex;
use sled::{
    transaction::{ConflictableTransactionError, TransactionError, Transactional},
    Db, Tree,
};

use crate::activation::{
    AgentBootstrapReceipt, AgentCurationRuleRecord, DirectiveRecord,
    LegacyDirectiveMigrationReceipt,
};
use crate::agent::bootstrap::{
    AgentBootstrapProgress, AgentBootstrapProgressStatus, AgentBootstrapStage,
};
use crate::agent::contracts::{
    deterministic_id, AgentActivationRecord, AgentActivationStatus, AgentAuthoredCommand,
    AgentCurationDecision, AgentCurationDedupeKey, AgentCurationOutcome, AgentDecisionKind,
    AgentDecisionOutboxRecord, AgentDeliverySelection, AgentHydrationCheckpoint,
    AgentHydrationCheckpointStage, AgentRecord, AgentSatisfactionCursorCasIntent,
    AgentSatisfactionCursorIdentity, AgentSatisfactionReview, AgentSatisfactionReviewCursor,
    AgentSatisfactionReviewSelection, AgentSemanticEnablementAudit, AgentSinkReceipt,
    AgentSinkReceiptKind, AgentStatus, AgentSubscriptionCursorCasIntent, AgentSubscriptionRecord,
    AgentSubscriptionStatus,
};
use crate::agent::hydration::{
    AgentHydrationOwnerSnapshot, AgentProcessHydrationRecord, AgentProcessHydrationStatus,
    AgentReadinessProof, FailAgentHydrationCommand, MarkAgentOperationalCommand,
    StartAgentHydrationCommand,
};
use crate::belief::{BeliefConfigSnapshotFence, BeliefKey, BeliefReadinessReopenContract};
use crate::error::StorageError;

const TREE_AGENT_RECORDS: &str = "agent_records";
const TREE_AGENT_BY_STATUS: &str = "agent_by_status";
const TREE_SUBSCRIPTIONS: &str = "agent_subscriptions";
const TREE_SUBSCRIPTIONS_BY_AGENT: &str = "agent_subscriptions_by_agent";
const TREE_SUBSCRIPTIONS_BY_KEY: &str = "agent_subscriptions_by_key";
const TREE_ACTIVATIONS: &str = "agent_activations";
const TREE_ACTIVATIONS_BY_AGENT: &str = "agent_activations_by_agent";
const TREE_DECISIONS: &str = "agent_curation_decisions";
const TREE_DECISIONS_BY_AGENT: &str = "agent_decisions_by_agent";
const TREE_DECISIONS_BY_DEDUPE: &str = "agent_decisions_by_dedupe";
const TREE_DECISIONS_BY_REVISION: &str = "agent_decisions_by_revision";
const TREE_SATISFACTION_DECISIONS_BY_REVIEW: &str = "agent_satisfaction_decisions_by_review";
const TREE_SINK_RECEIPTS: &str = "agent_sink_receipts";
const TREE_SINK_RECEIPTS_BY_COMMAND: &str = "agent_sink_receipts_by_command";
const TREE_DECISION_OUTBOX: &str = "agent_decision_outbox";
const TREE_SATISFACTION_REVIEW_CURSORS: &str = "agent_satisfaction_review_cursors";
const TREE_HYDRATION_CHECKPOINTS: &str = "agent_hydration_checkpoints";
const TREE_RUNTIME_SCHEMA: &str = "agent_runtime_schema";
const TREE_DIRECTIVES: &str = "agent_directives";
const TREE_CURATION_RULES: &str = "agent_curation_rules";
const TREE_BOOTSTRAP_PROGRESS: &str = "agent_bootstrap_progress";
const TREE_BOOTSTRAP_RECEIPTS: &str = "agent_bootstrap_receipts";
const TREE_BOOTSTRAP_RECEIPTS_BY_AGENT: &str = "agent_bootstrap_receipts_by_agent";
const TREE_LEGACY_MIGRATION_RECEIPTS: &str = "agent_legacy_directive_migration_receipts";
const TREE_PROCESS_HYDRATIONS: &str = "agent_process_hydrations";
const TREE_PROCESS_HYDRATIONS_BY_AGENT: &str = "agent_process_hydrations_by_agent";
const TREE_PROCESS_HYDRATIONS_BY_LEASE: &str = "agent_process_hydrations_by_lease";
const TREE_HYDRATION_EPOCHS: &str = "agent_hydration_epochs";
const TREE_CURRENT_HYDRATIONS: &str = "agent_current_hydrations";
const TREE_HYDRATION_RUNTIME_STATE: &str = "agent_hydration_runtime_state";
const TREE_READINESS_PROOFS: &str = "agent_readiness_proofs";
const TREE_READINESS_SCHEMA: &str = "agent_readiness_schema";
const KEY_READINESS_SCHEMA_STATE: &[u8] = b"state";
const READINESS_SCHEMA_VERSION: u16 = 2;
const READINESS_MIGRATION_BATCH: usize = 128;
const HYDRATION_CONTINUATION_KEY: &[u8] = b"registered_continuation";
const RUNTIME_SCHEMA_VERSION_KEY: &[u8] = b"semantic_runtime";
const RUNTIME_SCHEMA_VERSION: u16 = 1;
const KEY_PAD: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentReadinessMigrationState {
    schema_version: u16,
    cursor: Option<Vec<u8>>,
    complete: bool,
}

impl AgentReadinessMigrationState {
    fn initial() -> Self {
        Self {
            schema_version: READINESS_SCHEMA_VERSION,
            cursor: None,
            complete: false,
        }
    }

    fn complete() -> Self {
        Self {
            schema_version: READINESS_SCHEMA_VERSION,
            cursor: None,
            complete: true,
        }
    }

    fn validate(&self) -> Result<(), StorageError> {
        if self.schema_version != READINESS_SCHEMA_VERSION || self.complete && self.cursor.is_some()
        {
            return Err(StorageError::MigrationConflict(
                "agent readiness schema marker is invalid".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[derive(Default)]
struct FlushProbe {
    fail_next: bool,
    calls: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AgentHydrationContinuation {
    generation: u64,
    last_status_key: String,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentHydrationSelection {
    pub(crate) agents: Vec<AgentRecord>,
    pub(crate) budget_exhausted: bool,
    pub(crate) expected_continuation: Option<AgentHydrationContinuation>,
    pub(crate) advanced_continuation: Option<AgentHydrationContinuation>,
}

/// Exact owner records that must remain stable through one hydration mutation.
#[derive(Debug, Clone)]
pub(crate) struct AgentHydrationOwnerFence {
    pub(crate) agent: AgentRecord,
    pub(crate) receipt: AgentBootstrapReceipt,
    pub(crate) directive: DirectiveRecord,
    pub(crate) rule: AgentCurationRuleRecord,
    pub(crate) subscription: AgentSubscriptionRecord,
    pub(crate) belief_config: BeliefConfigSnapshotFence,
}

struct AgentHydrationOwnerFenceBytes {
    agent: Vec<u8>,
    receipt: Vec<u8>,
    directive: Vec<u8>,
    rule: Vec<u8>,
    subscription: Vec<u8>,
    belief_config: Vec<u8>,
}

/// Durable exact inputs carried by the opaque hydration fence capability.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentHydrationFenceSnapshot {
    owner: AgentHydrationOwnerSnapshot,
    hydration: AgentProcessHydrationRecord,
    checkpoint: AgentHydrationCheckpoint,
}

/// Opaque cross-domain capability for one exact hydration fence.
pub(crate) struct AgentHydrationFenceCapability {
    agents: Tree,
    receipts: Tree,
    directives: Tree,
    rules: Tree,
    subscriptions: Tree,
    belief_configs: Tree,
    hydrations: Tree,
    epochs: Tree,
    current: Tree,
    checkpoints: Tree,
    snapshot: AgentHydrationFenceSnapshot,
    owner_hash: String,
    owner_bytes: AgentHydrationOwnerFenceBytes,
    hydration_bytes: Vec<u8>,
    checkpoint_bytes: Vec<u8>,
    epoch_bytes: [u8; 8],
}

impl AgentHydrationFenceCapability {
    /// Reopen one exact durable capability without exposing agent tree layout.
    pub(crate) fn reopen(db: &Db, encoded: &[u8]) -> Result<Self, StorageError> {
        let snapshot: AgentHydrationFenceSnapshot =
            serde_json::from_slice(encoded).map_err(to_storage_data)?;
        snapshot.owner.fence_hash()?;
        snapshot
            .hydration
            .validate()
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        snapshot.checkpoint.validate()?;
        if snapshot.owner.agent.agent_id != snapshot.hydration.agent_id
            || snapshot.checkpoint.agent_id != snapshot.hydration.agent_id
            || snapshot.checkpoint.hydration_id != snapshot.hydration.hydration_id
            || snapshot.checkpoint.attempt_epoch != snapshot.hydration.attempt_epoch
            || snapshot.checkpoint.lease_id != snapshot.hydration.lease_id
            || snapshot.checkpoint.subscription_id != snapshot.owner.subscription.subscription_id
        {
            return Err(StorageError::InvalidPath(
                "durable hydration fence snapshot has divergent identities".to_string(),
            ));
        }
        let owner_bytes = AgentHydrationOwnerFenceBytes {
            agent: serde_json::to_vec(&snapshot.owner.agent).map_err(to_storage_data)?,
            receipt: serde_json::to_vec(&snapshot.owner.receipt).map_err(to_storage_data)?,
            directive: serde_json::to_vec(&snapshot.owner.directive).map_err(to_storage_data)?,
            rule: serde_json::to_vec(&snapshot.owner.rule).map_err(to_storage_data)?,
            subscription: serde_json::to_vec(&snapshot.owner.subscription)
                .map_err(to_storage_data)?,
            belief_config: snapshot.owner.belief_config_json.as_bytes().to_vec(),
        };
        let hydration_bytes = serde_json::to_vec(&snapshot.hydration).map_err(to_storage_data)?;
        let checkpoint_bytes = serde_json::to_vec(&snapshot.checkpoint).map_err(to_storage_data)?;
        let owner_hash = snapshot.owner.fence_hash()?;
        let epoch_bytes = encode_epoch(snapshot.hydration.attempt_epoch);
        let belief_config = BeliefConfigSnapshotFence::reopen(
            db,
            snapshot.owner.belief_config_hash.clone(),
            snapshot.owner.belief_config_json.clone(),
        )?;
        Ok(Self {
            agents: db.open_tree(TREE_AGENT_RECORDS).map_err(to_storage_io)?,
            receipts: db
                .open_tree(TREE_BOOTSTRAP_RECEIPTS)
                .map_err(to_storage_io)?,
            directives: db.open_tree(TREE_DIRECTIVES).map_err(to_storage_io)?,
            rules: db.open_tree(TREE_CURATION_RULES).map_err(to_storage_io)?,
            subscriptions: db.open_tree(TREE_SUBSCRIPTIONS).map_err(to_storage_io)?,
            belief_configs: belief_config.tree,
            hydrations: db
                .open_tree(TREE_PROCESS_HYDRATIONS)
                .map_err(to_storage_io)?,
            epochs: db.open_tree(TREE_HYDRATION_EPOCHS).map_err(to_storage_io)?,
            current: db
                .open_tree(TREE_CURRENT_HYDRATIONS)
                .map_err(to_storage_io)?,
            checkpoints: db
                .open_tree(TREE_HYDRATION_CHECKPOINTS)
                .map_err(to_storage_io)?,
            snapshot,
            owner_hash,
            owner_bytes,
            hydration_bytes,
            checkpoint_bytes,
            epoch_bytes,
        })
    }

    /// Serialize the exact owner, hydration, and checkpoint snapshot.
    pub(crate) fn snapshot_bytes(&self) -> Result<Vec<u8>, StorageError> {
        serde_json::to_vec(&self.snapshot).map_err(to_storage_data)
    }

    /// Return the canonical owner content digest.
    pub(crate) fn owner_hash(&self) -> &str {
        &self.owner_hash
    }

    /// Match one readiness product to the exact agent-owned subscription scope.
    pub(crate) fn matches_readiness_scope(
        &self,
        agent_id: &str,
        subscription_id: &str,
        belief_key: &BeliefKey,
    ) -> bool {
        self.snapshot.owner.agent.agent_id == agent_id
            && self.snapshot.owner.subscription.subscription_id == subscription_id
            && self.snapshot.owner.subscription.belief_key == *belief_key
    }

    /// Validate the immutable scope carried by one planner terminal permit.
    pub(crate) fn validate_planner_terminal_scope(
        &self,
        request_id: &str,
        request_agent_id: &str,
        pending_updated_at_seq: u64,
    ) -> Result<(), StorageError> {
        if self.snapshot.owner.agent.status != AgentStatus::Registered
            || self.snapshot.owner.agent.agent_id != request_agent_id
            || self.snapshot.hydration.status != AgentProcessHydrationStatus::Started
            || self.snapshot.checkpoint.stage != AgentHydrationCheckpointStage::ProjectionRequested
            || self.snapshot.checkpoint.planner_request_id.as_deref() != Some(request_id)
            || self.snapshot.checkpoint.updated_at_seq.checked_add(1)
                != Some(pending_updated_at_seq)
        {
            return Err(StorageError::InvalidPath(
                "planner terminal permit conflicts with its exact owner, hydration, or checkpoint"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// Run one strict cross-domain transition over three receiver trees.
    pub(crate) fn transaction_three<R, F>(
        &self,
        first: &Tree,
        second: &Tree,
        third: &Tree,
        operation: F,
    ) -> Result<R, StorageError>
    where
        F: Fn(
            &sled::transaction::TransactionalTree,
            &sled::transaction::TransactionalTree,
            &sled::transaction::TransactionalTree,
        ) -> Result<R, ConflictableTransactionError<String>>,
    {
        (
            &self.agents,
            &self.receipts,
            &self.directives,
            &self.rules,
            &self.subscriptions,
            &self.belief_configs,
            &self.hydrations,
            &self.epochs,
            &self.current,
            &self.checkpoints,
            first,
            second,
            third,
        )
            .transaction(
                |(
                    agents,
                    receipts,
                    directives,
                    rules,
                    subscriptions,
                    belief_configs,
                    hydrations,
                    epochs,
                    current,
                    checkpoints,
                    first,
                    second,
                    third,
                )| {
                    require_hydration_activation_fence_transaction(
                        self,
                        [
                            agents,
                            receipts,
                            directives,
                            rules,
                            subscriptions,
                            belief_configs,
                            hydrations,
                            epochs,
                            current,
                            checkpoints,
                        ],
                    )?;
                    operation(first, second, third)
                },
            )
            .map_err(to_agent_transition_error)
    }

    /// Run one strict cross-domain transition over four receiver trees.
    pub(crate) fn transaction_four<R, F>(
        &self,
        first: &Tree,
        second: &Tree,
        third: &Tree,
        fourth: &Tree,
        operation: F,
    ) -> Result<R, StorageError>
    where
        F: Fn(
            &sled::transaction::TransactionalTree,
            &sled::transaction::TransactionalTree,
            &sled::transaction::TransactionalTree,
            &sled::transaction::TransactionalTree,
        ) -> Result<R, ConflictableTransactionError<String>>,
    {
        (
            &self.agents,
            &self.receipts,
            &self.directives,
            &self.rules,
            &self.subscriptions,
            &self.belief_configs,
            &self.hydrations,
            &self.epochs,
            &self.current,
            &self.checkpoints,
            first,
            second,
            third,
            fourth,
        )
            .transaction(
                |(
                    agents,
                    receipts,
                    directives,
                    rules,
                    subscriptions,
                    belief_configs,
                    hydrations,
                    epochs,
                    current,
                    checkpoints,
                    first,
                    second,
                    third,
                    fourth,
                )| {
                    require_hydration_activation_fence_transaction(
                        self,
                        [
                            agents,
                            receipts,
                            directives,
                            rules,
                            subscriptions,
                            belief_configs,
                            hydrations,
                            epochs,
                            current,
                            checkpoints,
                        ],
                    )?;
                    operation(first, second, third, fourth)
                },
            )
            .map_err(to_agent_transition_error)
    }

    /// Run one receiver transition that may atomically revoke a stale fence.
    pub(crate) fn transaction_four_checked<R, F>(
        &self,
        first: &Tree,
        second: &Tree,
        third: &Tree,
        fourth: &Tree,
        operation: F,
    ) -> Result<R, StorageError>
    where
        F: Fn(
            &sled::transaction::TransactionalTree,
            &sled::transaction::TransactionalTree,
            &sled::transaction::TransactionalTree,
            &sled::transaction::TransactionalTree,
            bool,
        ) -> Result<R, ConflictableTransactionError<String>>,
    {
        (
            &self.agents,
            &self.receipts,
            &self.directives,
            &self.rules,
            &self.subscriptions,
            &self.belief_configs,
            &self.hydrations,
            &self.epochs,
            &self.current,
            &self.checkpoints,
            first,
            second,
            third,
            fourth,
        )
            .transaction(
                |(
                    agents,
                    receipts,
                    directives,
                    rules,
                    subscriptions,
                    belief_configs,
                    hydrations,
                    epochs,
                    current,
                    checkpoints,
                    first,
                    second,
                    third,
                    fourth,
                )| {
                    let current = hydration_activation_fence_matches_transaction(
                        self,
                        [
                            agents,
                            receipts,
                            directives,
                            rules,
                            subscriptions,
                            belief_configs,
                            hydrations,
                            epochs,
                            current,
                            checkpoints,
                        ],
                    )?;
                    operation(first, second, third, fourth, current)
                },
            )
            .map_err(to_agent_transition_error)
    }

    /// Run one terminal hydration transition under the complete durable fence.
    pub(crate) fn transaction_hydration_and_one<R, F>(
        &self,
        receiver: &Tree,
        operation: F,
    ) -> Result<R, StorageError>
    where
        F: Fn(
            &sled::transaction::TransactionalTree,
            &sled::transaction::TransactionalTree,
        ) -> Result<R, ConflictableTransactionError<String>>,
    {
        (
            &self.agents,
            &self.receipts,
            &self.directives,
            &self.rules,
            &self.subscriptions,
            &self.belief_configs,
            &self.hydrations,
            &self.epochs,
            &self.current,
            &self.checkpoints,
            receiver,
        )
            .transaction(
                |(
                    agents,
                    receipts,
                    directives,
                    rules,
                    subscriptions,
                    belief_configs,
                    hydrations,
                    epochs,
                    current,
                    checkpoints,
                    receiver,
                )| {
                    require_hydration_activation_fence_transaction(
                        self,
                        [
                            agents,
                            receipts,
                            directives,
                            rules,
                            subscriptions,
                            belief_configs,
                            hydrations,
                            epochs,
                            current,
                            checkpoints,
                        ],
                    )?;
                    operation(hydrations, receiver)
                },
            )
            .map_err(to_agent_transition_error)
    }
}

/// Sled backed storage for durable agent records and indexes.
#[derive(Clone)]
pub struct AgentStore {
    db: Db,
    agents: Tree,
    agent_by_status: Tree,
    subscriptions: Tree,
    subscriptions_by_agent: Tree,
    subscriptions_by_key: Tree,
    activations: Tree,
    activations_by_agent: Tree,
    decisions: Tree,
    decisions_by_agent: Tree,
    decisions_by_dedupe: Tree,
    decisions_by_revision: Tree,
    satisfaction_decisions_by_review: Tree,
    sink_receipts: Tree,
    sink_receipts_by_command: Tree,
    decision_outbox: Tree,
    satisfaction_review_cursors: Tree,
    hydration_checkpoints: Tree,
    runtime_schema: Tree,
    directives: Tree,
    curation_rules: Tree,
    bootstrap_progress: Tree,
    bootstrap_receipts: Tree,
    bootstrap_receipts_by_agent: Tree,
    legacy_migration_receipts: Tree,
    process_hydrations: Tree,
    process_hydrations_by_agent: Tree,
    process_hydrations_by_lease: Tree,
    hydration_epochs: Tree,
    current_hydrations: Tree,
    hydration_runtime_state: Tree,
    readiness_proofs: Tree,
    readiness_schema: Tree,
    #[cfg(test)]
    flush_probe: Arc<Mutex<FlushProbe>>,
}

impl AgentStore {
    /// Open all agent trees from the shared world model database.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        let store = Self {
            agents: db.open_tree(TREE_AGENT_RECORDS).map_err(to_storage_io)?,
            agent_by_status: db.open_tree(TREE_AGENT_BY_STATUS).map_err(to_storage_io)?,
            subscriptions: db.open_tree(TREE_SUBSCRIPTIONS).map_err(to_storage_io)?,
            subscriptions_by_agent: db
                .open_tree(TREE_SUBSCRIPTIONS_BY_AGENT)
                .map_err(to_storage_io)?,
            subscriptions_by_key: db
                .open_tree(TREE_SUBSCRIPTIONS_BY_KEY)
                .map_err(to_storage_io)?,
            activations: db.open_tree(TREE_ACTIVATIONS).map_err(to_storage_io)?,
            activations_by_agent: db
                .open_tree(TREE_ACTIVATIONS_BY_AGENT)
                .map_err(to_storage_io)?,
            decisions: db.open_tree(TREE_DECISIONS).map_err(to_storage_io)?,
            decisions_by_agent: db
                .open_tree(TREE_DECISIONS_BY_AGENT)
                .map_err(to_storage_io)?,
            decisions_by_dedupe: db
                .open_tree(TREE_DECISIONS_BY_DEDUPE)
                .map_err(to_storage_io)?,
            decisions_by_revision: db
                .open_tree(TREE_DECISIONS_BY_REVISION)
                .map_err(to_storage_io)?,
            satisfaction_decisions_by_review: db
                .open_tree(TREE_SATISFACTION_DECISIONS_BY_REVIEW)
                .map_err(to_storage_io)?,
            sink_receipts: db.open_tree(TREE_SINK_RECEIPTS).map_err(to_storage_io)?,
            sink_receipts_by_command: db
                .open_tree(TREE_SINK_RECEIPTS_BY_COMMAND)
                .map_err(to_storage_io)?,
            decision_outbox: db.open_tree(TREE_DECISION_OUTBOX).map_err(to_storage_io)?,
            satisfaction_review_cursors: db
                .open_tree(TREE_SATISFACTION_REVIEW_CURSORS)
                .map_err(to_storage_io)?,
            hydration_checkpoints: db
                .open_tree(TREE_HYDRATION_CHECKPOINTS)
                .map_err(to_storage_io)?,
            runtime_schema: db.open_tree(TREE_RUNTIME_SCHEMA).map_err(to_storage_io)?,
            directives: db.open_tree(TREE_DIRECTIVES).map_err(to_storage_io)?,
            curation_rules: db.open_tree(TREE_CURATION_RULES).map_err(to_storage_io)?,
            bootstrap_progress: db
                .open_tree(TREE_BOOTSTRAP_PROGRESS)
                .map_err(to_storage_io)?,
            bootstrap_receipts: db
                .open_tree(TREE_BOOTSTRAP_RECEIPTS)
                .map_err(to_storage_io)?,
            bootstrap_receipts_by_agent: db
                .open_tree(TREE_BOOTSTRAP_RECEIPTS_BY_AGENT)
                .map_err(to_storage_io)?,
            legacy_migration_receipts: db
                .open_tree(TREE_LEGACY_MIGRATION_RECEIPTS)
                .map_err(to_storage_io)?,
            process_hydrations: db
                .open_tree(TREE_PROCESS_HYDRATIONS)
                .map_err(to_storage_io)?,
            process_hydrations_by_agent: db
                .open_tree(TREE_PROCESS_HYDRATIONS_BY_AGENT)
                .map_err(to_storage_io)?,
            process_hydrations_by_lease: db
                .open_tree(TREE_PROCESS_HYDRATIONS_BY_LEASE)
                .map_err(to_storage_io)?,
            hydration_epochs: db.open_tree(TREE_HYDRATION_EPOCHS).map_err(to_storage_io)?,
            current_hydrations: db
                .open_tree(TREE_CURRENT_HYDRATIONS)
                .map_err(to_storage_io)?,
            hydration_runtime_state: db
                .open_tree(TREE_HYDRATION_RUNTIME_STATE)
                .map_err(to_storage_io)?,
            readiness_proofs: db.open_tree(TREE_READINESS_PROOFS).map_err(to_storage_io)?,
            readiness_schema: db.open_tree(TREE_READINESS_SCHEMA).map_err(to_storage_io)?,
            #[cfg(test)]
            flush_probe: Arc::new(Mutex::new(FlushProbe::default())),
            db,
        };
        store.migrate_runtime_schema()?;
        store.migrate_legacy_readiness_proofs()?;
        Ok(store)
    }

    /// Open the store behind a shared pointer for facade wiring.
    pub fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        Ok(Arc::new(Self::new(db)?))
    }

    pub(super) fn shared_database(&self) -> Db {
        self.db.clone()
    }

    /// Write an agent record and its status index within the agent domain.
    pub(super) fn put_agent(&self, record: &AgentRecord) -> Result<(), StorageError> {
        record.validate()?;
        self.agents
            .insert(
                record.agent_id.as_bytes(),
                serde_json::to_vec(record).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.agent_by_status
            .insert(
                agent_status_key(
                    record.status.index_key(),
                    record.updated_at_seq,
                    &record.agent_id,
                )
                .as_bytes(),
                record.agent_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Read one agent record by id.
    pub fn get_agent(&self, agent_id: &str) -> Result<Option<AgentRecord>, StorageError> {
        let Some(raw) = self
            .agents
            .get(agent_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        match serde_json::from_slice(&raw) {
            Ok(record) => Ok(Some(record)),
            Err(canonical_error) => crate::agent::bootstrap::decode_legacy_agent_for_read(&raw)
                .map(Some)
                .map_err(|legacy_error| {
                    StorageError::IoError(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "cannot decode canonical agent: {canonical_error}; cannot decode legacy agent: {legacy_error}"
                        ),
                    ))
                }),
        }
    }

    /// Read one durable directive by id.
    pub fn get_directive(
        &self,
        directive_id: &str,
    ) -> Result<Option<DirectiveRecord>, StorageError> {
        decode_optional(
            self.directives
                .get(directive_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Read one configured curation rule by id.
    pub fn get_curation_rule(
        &self,
        rule_id: &str,
    ) -> Result<Option<AgentCurationRuleRecord>, StorageError> {
        decode_optional(
            self.curation_rules
                .get(rule_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Read durable progress for one logical bootstrap.
    pub fn get_bootstrap_progress(
        &self,
        bootstrap_id: &str,
    ) -> Result<Option<AgentBootstrapProgress>, StorageError> {
        decode_optional(
            self.bootstrap_progress
                .get(bootstrap_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Read the final durable receipt for one logical bootstrap.
    pub fn get_bootstrap_receipt(
        &self,
        bootstrap_id: &str,
    ) -> Result<Option<AgentBootstrapReceipt>, StorageError> {
        decode_optional(
            self.bootstrap_receipts
                .get(bootstrap_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Read the exact completed bootstrap receipt indexed for one seed agent.
    pub fn bootstrap_receipt_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentBootstrapReceipt>, StorageError> {
        let Some(raw_bootstrap_id) = self
            .bootstrap_receipts_by_agent
            .get(agent_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let bootstrap_id = std::str::from_utf8(&raw_bootstrap_id).map_err(|error| {
            StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, error))
        })?;
        let receipt = self
            .get_bootstrap_receipt(bootstrap_id)?
            .ok_or_else(|| {
                StorageError::MigrationConflict(format!(
                    "bootstrap receipt index for agent '{agent_id}' references missing receipt '{bootstrap_id}'"
                ))
            })?;
        let progress = self.get_bootstrap_progress(bootstrap_id)?.ok_or_else(|| {
            StorageError::MigrationConflict(format!(
                "bootstrap receipt '{bootstrap_id}' has no completed progress"
            ))
        })?;
        validate_bootstrap_join(agent_id, bootstrap_id, &receipt, &progress)?;
        Ok(Some(receipt))
    }

    /// Read one completed embedded-directive migration receipt.
    pub fn get_legacy_directive_migration_receipt(
        &self,
        receipt_id: &str,
    ) -> Result<Option<LegacyDirectiveMigrationReceipt>, StorageError> {
        decode_optional(
            self.legacy_migration_receipts
                .get(receipt_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// List agents with a status in deterministic sequence and id order.
    pub fn agents_by_status(&self, status: AgentStatus) -> Result<Vec<AgentRecord>, StorageError> {
        let prefix = format!("{}::", status.index_key());
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        for item in self.agent_by_status.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.map_err(to_storage_io)?;
            let agent_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if seen.insert(agent_id.clone()) {
                let Some(agent) = self.get_agent(&agent_id)? else {
                    continue;
                };
                if agent.status == status {
                    out.push(agent);
                }
            }
        }
        out.sort_by(|left, right| {
            left.updated_at_seq
                .cmp(&right.updated_at_seq)
                .then_with(|| left.agent_id.cmp(&right.agent_id))
        });
        Ok(out)
    }

    /// Select registered agents through a durable fair continuation cursor.
    pub(crate) fn registered_agents_bounded(
        &self,
        max_items: usize,
    ) -> Result<AgentHydrationSelection, StorageError> {
        if max_items == 0 {
            return Err(StorageError::InvalidPath(
                "agent hydration selection budget must be greater than zero".to_string(),
            ));
        }
        let expected_continuation = self.hydration_continuation()?;
        let prefix = format!("{}::", AgentStatus::Registered.index_key());
        let mut candidates = Vec::with_capacity(max_items.saturating_add(1));
        let selection_limit = max_items.saturating_add(1);

        if let Some(continuation) = expected_continuation.as_ref() {
            for item in self.agent_by_status.range::<Vec<u8>, _>((
                Bound::Excluded(continuation.last_status_key.as_bytes().to_vec()),
                Bound::Unbounded,
            )) {
                let (key, value) = item.map_err(to_storage_io)?;
                if !key.starts_with(prefix.as_bytes()) {
                    break;
                }
                candidates.push(self.require_status_index_record(&key, &value)?);
                if candidates.len() == selection_limit {
                    break;
                }
            }
        }
        if candidates.len() < selection_limit {
            for item in self.agent_by_status.scan_prefix(prefix.as_bytes()) {
                let (key, value) = item.map_err(to_storage_io)?;
                if expected_continuation
                    .as_ref()
                    .is_some_and(|cursor| key.as_ref() > cursor.last_status_key.as_bytes())
                {
                    break;
                }
                candidates.push(self.require_status_index_record(&key, &value)?);
                if candidates.len() == selection_limit {
                    break;
                }
            }
        }

        let budget_exhausted = candidates.len() > max_items;
        candidates.truncate(max_items);
        let advanced_continuation = candidates.last().map(|agent| AgentHydrationContinuation {
            generation: expected_continuation
                .as_ref()
                .map_or(1, |cursor| cursor.generation.saturating_add(1)),
            last_status_key: agent_status_key(
                agent.status.index_key(),
                agent.updated_at_seq,
                &agent.agent_id,
            ),
        });
        if advanced_continuation
            .as_ref()
            .is_some_and(|cursor| cursor.generation == u64::MAX)
            && expected_continuation
                .as_ref()
                .is_some_and(|cursor| cursor.generation == u64::MAX)
        {
            return Err(StorageError::InvalidPath(
                "agent hydration continuation generation is exhausted".to_string(),
            ));
        }
        Ok(AgentHydrationSelection {
            agents: candidates,
            budget_exhausted,
            expected_continuation,
            advanced_continuation,
        })
    }

    /// Advance the exact durable hydration continuation and reflush exact replay.
    pub(crate) fn advance_hydration_continuation(
        &self,
        expected: Option<&AgentHydrationContinuation>,
        advanced: &AgentHydrationContinuation,
    ) -> Result<(), StorageError> {
        validate_hydration_continuation(advanced)?;
        let expected_bytes = expected
            .map(serde_json::to_vec)
            .transpose()
            .map_err(to_storage_data)?;
        let advanced_bytes = serde_json::to_vec(advanced).map_err(to_storage_data)?;
        if self
            .hydration_runtime_state
            .get(HYDRATION_CONTINUATION_KEY)
            .map_err(to_storage_io)?
            .as_deref()
            == Some(advanced_bytes.as_slice())
        {
            return self.flush_durable("agent hydration continuation replay");
        }
        self.hydration_runtime_state
            .compare_and_swap(
                HYDRATION_CONTINUATION_KEY,
                expected_bytes.as_deref(),
                Some(advanced_bytes),
            )
            .map_err(to_storage_io)?
            .map_err(|_| {
                StorageError::Backpressure(
                    "agent hydration continuation changed during bounded tick".to_string(),
                )
            })?;
        self.flush_durable("agent hydration continuation advancement")
    }

    fn hydration_continuation(&self) -> Result<Option<AgentHydrationContinuation>, StorageError> {
        let continuation: Option<AgentHydrationContinuation> = decode_optional(
            self.hydration_runtime_state
                .get(HYDRATION_CONTINUATION_KEY)
                .map_err(to_storage_io)?,
        )?;
        if let Some(continuation) = continuation.as_ref() {
            validate_hydration_continuation(continuation)?;
        }
        Ok(continuation)
    }

    fn require_status_index_record(
        &self,
        key: &[u8],
        value: &[u8],
    ) -> Result<AgentRecord, StorageError> {
        let agent_id = std::str::from_utf8(value).map_err(|error| {
            StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, error))
        })?;
        let agent = self.get_agent(agent_id)?.ok_or_else(|| {
            StorageError::MigrationConflict(format!(
                "registered status index references missing agent '{agent_id}'"
            ))
        })?;
        agent.validate()?;
        let expected_key = agent_status_key(
            AgentStatus::Registered.index_key(),
            agent.updated_at_seq,
            &agent.agent_id,
        );
        if agent.agent_id != agent_id
            || agent.status != AgentStatus::Registered
            || key != expected_key.as_bytes()
        {
            return Err(StorageError::MigrationConflict(format!(
                "registered status index conflicts with agent '{agent_id}'"
            )));
        }
        Ok(agent)
    }

    /// Write a subscription record and its lookup indexes.
    pub(super) fn put_subscription(
        &self,
        record: &AgentSubscriptionRecord,
    ) -> Result<(), StorageError> {
        record.validate()?;
        self.subscriptions
            .insert(
                record.subscription_id.as_bytes(),
                serde_json::to_vec(record).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.subscriptions_by_agent
            .insert(
                subscription_agent_key(
                    &record.agent_id,
                    record.created_at_seq,
                    &record.subscription_id,
                )
                .as_bytes(),
                record.subscription_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        self.subscriptions_by_key
            .insert(
                AgentSubscriptionRecord::natural_key(&record.agent_id, &record.belief_key)
                    .as_bytes(),
                record.subscription_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Read one subscription record by id.
    pub fn get_subscription(
        &self,
        subscription_id: &str,
    ) -> Result<Option<AgentSubscriptionRecord>, StorageError> {
        decode_optional(
            self.subscriptions
                .get(subscription_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Atomically advance one subscription from its exact observed cursor.
    pub fn advance_subscription_cursor_cas(
        &self,
        intent: &AgentSubscriptionCursorCasIntent,
    ) -> Result<AgentSubscriptionRecord, StorageError> {
        let command = intent.command();
        let key = command.subscription_id.as_bytes();
        let Some(raw) = self.subscriptions.get(key).map_err(to_storage_io)? else {
            return Err(StorageError::InvalidPath(format!(
                "unknown subscription '{}'",
                command.subscription_id
            )));
        };
        let current: AgentSubscriptionRecord =
            serde_json::from_slice(&raw).map_err(to_storage_data)?;
        if current.agent_id != command.agent_id {
            return Err(StorageError::InvalidPath(format!(
                "subscription '{}' does not belong to agent '{}'",
                command.subscription_id, command.agent_id
            )));
        }
        if current.last_delivered_revision_id.as_deref()
            == Some(command.delivered_revision_id.as_str())
            && current.last_delivered_seq == command.delivered_seq
        {
            return Ok(current);
        }
        if current.last_delivered_revision_id.as_deref() != intent.expected_delivered_revision_id()
            || current.last_delivered_seq != intent.expected_delivered_seq()
        {
            return Err(StorageError::Backpressure(format!(
                "subscription cursor changed for '{}'",
                command.subscription_id
            )));
        }

        let mut updated = current;
        updated.last_delivered_revision_id = Some(command.delivered_revision_id.clone());
        updated.last_delivered_seq = command.delivered_seq;
        updated.updated_at_seq = command.delivered_seq;
        let encoded = serde_json::to_vec(&updated).map_err(to_storage_data)?;
        match self
            .subscriptions
            .compare_and_swap(key, Some(raw), Some(encoded))
            .map_err(to_storage_io)?
        {
            Ok(()) => Ok(updated),
            Err(_) => Err(StorageError::Backpressure(format!(
                "subscription cursor changed for '{}'",
                command.subscription_id
            ))),
        }
    }

    /// Advance a selected delivery only after its decision and sink work are durable.
    pub fn advance_selected_delivery_cursor(
        &self,
        selection: &AgentDeliverySelection,
        decision_id: &str,
    ) -> Result<AgentSubscriptionRecord, StorageError> {
        selection.validate()?;
        let agent = self.require_selected_agent(
            &selection.delivery.agent_id,
            selection.expected_agent_updated_at_seq,
        )?;
        let subscription = self.require_selected_subscription(
            &selection.delivery.subscription_id,
            &selection.delivery.agent_id,
            &selection.belief_key,
            selection.expected_subscription_updated_at_seq,
        )?;
        if subscription.last_delivered_revision_id != selection.expected_delivered_revision_id
            || subscription.last_delivered_seq != selection.expected_delivered_seq
        {
            return Err(StorageError::Backpressure(
                "subscription cursor changed before selected advancement".to_string(),
            ));
        }
        let decision = self.get_decision(decision_id)?.ok_or_else(|| {
            StorageError::Backpressure(
                "selected delivery cursor cannot advance before its decision".to_string(),
            )
        })?;
        if self
            .satisfaction_review_index_map()?
            .contains_key(decision_id)
        {
            return Err(StorageError::InvalidPath(
                "satisfaction decision cannot advance the delivery cursor".to_string(),
            ));
        }
        if decision.agent_id != selection.delivery.agent_id
            || decision.subscription_id != selection.delivery.subscription_id
            || decision.input_refs.belief_key != selection.belief_key
            || decision.input_refs.belief_revision_id.as_deref()
                != Some(selection.delivery.belief_revision_id.as_str())
        {
            return Err(StorageError::InvalidPath(
                "selected delivery decision identity mismatch".to_string(),
            ));
        }
        if matches!(decision.decision, AgentDecisionKind::GoalCommand) {
            let receipt = self.sink_receipt_by_decision(decision_id)?.ok_or_else(|| {
                StorageError::Backpressure(
                    "selected delivery cursor cannot advance before its sink receipt".to_string(),
                )
            })?;
            let outbox = self.decision_outbox(decision_id)?.ok_or_else(|| {
                StorageError::MigrationConflict(
                    "selected delivery command has no exact outbox".to_string(),
                )
            })?;
            outbox.validate_for(&decision)?;
            validate_sink_receipt_for_decision(&receipt, &decision, Some(&outbox))?;
        }

        let mut updated = subscription.clone();
        updated.last_delivered_revision_id = Some(selection.delivery.belief_revision_id.clone());
        updated.last_delivered_seq = selection.delivery.revision_seq;
        updated.updated_at_seq = selection.delivery.revision_seq;
        updated.validate()?;
        let agent_bytes = serde_json::to_vec(&agent).map_err(to_storage_data)?;
        let subscription_bytes = serde_json::to_vec(&subscription).map_err(to_storage_data)?;
        let updated_bytes = serde_json::to_vec(&updated).map_err(to_storage_data)?;
        (&self.agents, &self.subscriptions)
            .transaction(|(agents, subscriptions)| {
                require_transaction_value(
                    agents,
                    agent.agent_id.as_bytes(),
                    &agent_bytes,
                    "selected agent",
                )?;
                require_transaction_value(
                    subscriptions,
                    subscription.subscription_id.as_bytes(),
                    &subscription_bytes,
                    "selected subscription",
                )?;
                subscriptions.insert(
                    subscription.subscription_id.as_bytes(),
                    updated_bytes.as_slice(),
                )?;
                Ok(())
            })
            .map_err(to_agent_transition_error)?;
        self.flush_durable("selected delivery cursor advancement")?;
        Ok(updated)
    }

    /// Read the idempotent subscription for one agent and belief key.
    pub fn subscription_by_agent_and_key(
        &self,
        agent_id: &str,
        belief_key: &crate::belief::BeliefKey,
    ) -> Result<Option<AgentSubscriptionRecord>, StorageError> {
        let key = AgentSubscriptionRecord::natural_key(agent_id, belief_key);
        let Some(raw) = self
            .subscriptions_by_key
            .get(key.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let subscription_id = String::from_utf8(raw.to_vec()).map_err(to_storage_utf8)?;
        self.get_subscription(&subscription_id)
    }

    /// List subscriptions for an agent in deterministic sequence and id order.
    pub fn subscriptions_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Vec<AgentSubscriptionRecord>, StorageError> {
        let prefix = format!("{agent_id}::");
        let mut out = Vec::new();
        for item in self.subscriptions_by_agent.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.map_err(to_storage_io)?;
            let subscription_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(subscription) = self.get_subscription(&subscription_id)? {
                out.push(subscription);
            }
        }
        out.sort_by(|left, right| {
            left.created_at_seq
                .cmp(&right.created_at_seq)
                .then_with(|| left.subscription_id.cmp(&right.subscription_id))
        });
        Ok(out)
    }

    /// List active subscriptions for an agent.
    pub fn pending_subscriptions(
        &self,
        agent_id: &str,
    ) -> Result<Vec<AgentSubscriptionRecord>, StorageError> {
        Ok(self
            .subscriptions_for_agent(agent_id)?
            .into_iter()
            .filter(|subscription| subscription.status == AgentSubscriptionStatus::Active)
            .collect())
    }

    /// Read one activation record by id.
    pub fn get_activation(
        &self,
        activation_id: &str,
    ) -> Result<Option<AgentActivationRecord>, StorageError> {
        decode_optional(
            self.activations
                .get(activation_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// List activations for an agent in deterministic sequence and id order.
    pub fn activations_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Vec<AgentActivationRecord>, StorageError> {
        let prefix = format!("{agent_id}::");
        let mut out = Vec::new();
        for item in self.activations_by_agent.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.map_err(to_storage_io)?;
            let activation_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(activation) = self.get_activation(&activation_id)? {
                out.push(activation);
            }
        }
        out.sort_by(|left, right| {
            left.started_at_seq
                .cmp(&right.started_at_seq)
                .then_with(|| left.activation_id.cmp(&right.activation_id))
        });
        Ok(out)
    }

    /// Atomically begin one fenced hydration attempt and its activation diagnostic.
    #[cfg(test)]
    pub(super) fn start_process_hydration(
        &self,
        command: &StartAgentHydrationCommand,
    ) -> Result<AgentProcessHydrationRecord, StorageError> {
        self.start_process_hydration_inner(command, None)
    }

    pub(crate) fn start_process_hydration_fenced(
        &self,
        command: &StartAgentHydrationCommand,
        owner: &AgentHydrationOwnerFence,
    ) -> Result<AgentProcessHydrationRecord, StorageError> {
        self.start_process_hydration_inner(command, Some(owner))
    }

    fn start_process_hydration_inner(
        &self,
        command: &StartAgentHydrationCommand,
        owner: Option<&AgentHydrationOwnerFence>,
    ) -> Result<AgentProcessHydrationRecord, StorageError> {
        command
            .validate()
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        let agent = self.get_agent(&command.agent_id)?.ok_or_else(|| {
            StorageError::InvalidPath(format!("unknown agent '{}'", command.agent_id))
        })?;
        if let Some(owner) = owner {
            validate_hydration_owner_fence(owner)?;
            if owner.agent != agent {
                return Err(StorageError::Backpressure(
                    "hydration owner agent changed before start".to_string(),
                ));
            }
        }
        if !matches!(
            agent.status,
            AgentStatus::Registered | AgentStatus::Operational
        ) {
            return Err(StorageError::Backpressure(format!(
                "agent '{}' is not eligible for hydration",
                command.agent_id
            )));
        }

        let durable_epoch = self.current_hydration_epoch(&command.agent_id)?;
        let durable_hydration_id = self.current_hydration_id(&command.agent_id)?;
        let started = AgentProcessHydrationRecord {
            hydration_id: command.hydration_id.clone(),
            agent_id: command.agent_id.clone(),
            attempt_epoch: command.attempt_epoch,
            status: AgentProcessHydrationStatus::Started,
            readiness_proof_id: None,
            lease_id: command.lease_id.clone(),
            last_error: None,
            started_at_seq: command.started_at_seq,
            updated_at_seq: command.started_at_seq,
        };
        started
            .validate()
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        let activation = AgentActivationRecord {
            activation_id: command.hydration_id.clone(),
            agent_id: command.agent_id.clone(),
            started_at_seq: command.started_at_seq,
            attempt_epoch: command.attempt_epoch,
            updated_at_seq: command.started_at_seq,
            status: AgentActivationStatus::Started,
            last_error: None,
            lease_id: Some(command.lease_id.clone()),
        };
        activation.validate()?;

        if durable_epoch == command.attempt_epoch
            && durable_hydration_id.as_deref() == Some(command.hydration_id.as_str())
        {
            let durable_hydration = self
                .get_process_hydration(&command.hydration_id)?
                .ok_or_else(|| {
                    StorageError::Backpressure(
                        "current hydration record is missing during start replay".to_string(),
                    )
                })?;
            let durable_activation =
                self.get_activation(&command.hydration_id)?.ok_or_else(|| {
                    StorageError::Backpressure(
                        "current activation diagnostic is missing during start replay".to_string(),
                    )
                })?;
            if durable_hydration != started || durable_activation != activation {
                return Err(StorageError::Backpressure(
                    "hydration start replay conflicts with durable state".to_string(),
                ));
            }
            self.verify_hydration_replay_fences(&agent, &started, &activation, owner)?;
            self.flush_durable("hydration start replay")?;
            return Ok(started);
        }
        if durable_epoch != command.expected_prior_attempt_epoch {
            return Err(StorageError::Backpressure(format!(
                "hydration epoch changed for agent '{}'",
                command.agent_id
            )));
        }

        let prior = durable_hydration_id
            .as_deref()
            .map(|hydration_id| self.require_hydration_pair(hydration_id))
            .transpose()?;
        let mut failed_prior = None;
        if let Some((prior_hydration, prior_activation)) = &prior {
            if prior_hydration.agent_id != command.agent_id
                || prior_hydration.attempt_epoch != durable_epoch
                || prior_activation.agent_id != command.agent_id
                || prior_activation.attempt_epoch != durable_epoch
            {
                return Err(StorageError::Backpressure(
                    "prior hydration products do not match the durable epoch".to_string(),
                ));
            }
            if prior_hydration.status == AgentProcessHydrationStatus::Started {
                if command.started_at_seq <= prior_hydration.updated_at_seq
                    || prior_activation.status != AgentActivationStatus::Started
                {
                    return Err(StorageError::Backpressure(
                        "prior hydration cannot be superseded at the requested sequence"
                            .to_string(),
                    ));
                }
                let diagnostic =
                    format!("superseded by hydration attempt '{}'", command.hydration_id);
                let mut hydration = prior_hydration.clone();
                hydration.status = AgentProcessHydrationStatus::Failed;
                hydration.last_error = Some(diagnostic.clone());
                hydration.updated_at_seq = command.started_at_seq;
                let mut activation = prior_activation.clone();
                activation.status = AgentActivationStatus::Failed;
                activation.last_error = Some(diagnostic);
                activation.updated_at_seq = command.started_at_seq;
                failed_prior = Some((hydration, activation));
            }
        }

        let expected_agent = serde_json::to_vec(&agent).map_err(to_storage_data)?;
        let started_bytes = serde_json::to_vec(&started).map_err(to_storage_data)?;
        let activation_bytes = serde_json::to_vec(&activation).map_err(to_storage_data)?;
        let epoch_bytes = encode_epoch(command.attempt_epoch);
        let expected_epoch_bytes = encode_epoch(durable_epoch);
        let hydration_index = hydration_agent_key(
            &command.agent_id,
            command.started_at_seq,
            &command.hydration_id,
        );
        let activation_index = activation_agent_key(
            &command.agent_id,
            command.started_at_seq,
            &command.hydration_id,
        );
        let lease_index =
            hydration_lease_key(&command.agent_id, &command.lease_id, command.attempt_epoch);
        let prior_bytes = prior
            .as_ref()
            .map(|(hydration, activation)| -> Result<_, StorageError> {
                Ok((
                    serde_json::to_vec(hydration).map_err(to_storage_data)?,
                    serde_json::to_vec(activation).map_err(to_storage_data)?,
                ))
            })
            .transpose()?;
        let failed_prior_bytes = failed_prior
            .as_ref()
            .map(|(hydration, activation)| -> Result<_, StorageError> {
                Ok((
                    serde_json::to_vec(hydration).map_err(to_storage_data)?,
                    serde_json::to_vec(activation).map_err(to_storage_data)?,
                ))
            })
            .transpose()?;
        let owner_bytes = owner.map(encode_hydration_owner_fence).transpose()?;
        let belief_config_tree = owner
            .map(|owner| &owner.belief_config.tree)
            .unwrap_or(&self.runtime_schema);

        (
            &self.agents,
            &self.bootstrap_receipts,
            &self.directives,
            &self.curation_rules,
            &self.subscriptions,
            belief_config_tree,
            &self.hydration_epochs,
            &self.current_hydrations,
            &self.process_hydrations,
            &self.process_hydrations_by_agent,
            &self.process_hydrations_by_lease,
            &self.activations,
            &self.activations_by_agent,
        )
            .transaction(
                |(
                    agents,
                    receipts,
                    directives,
                    rules,
                    subscriptions,
                    belief_configs,
                    epochs,
                    current,
                    hydrations,
                    hydration_index_tree,
                    hydration_lease_index_tree,
                    activations,
                    activation_index_tree,
                )| {
                    require_transaction_value(
                        agents,
                        command.agent_id.as_bytes(),
                        &expected_agent,
                        "agent",
                    )?;
                    if let (Some(owner), Some(bytes)) = (owner, owner_bytes.as_ref()) {
                        require_transaction_value(
                            receipts,
                            owner.receipt.bootstrap_id.as_bytes(),
                            &bytes.receipt,
                            "bootstrap receipt",
                        )?;
                        require_transaction_value(
                            directives,
                            owner.directive.directive_id.as_bytes(),
                            &bytes.directive,
                            "directive",
                        )?;
                        require_transaction_value(
                            rules,
                            owner.rule.rule_id.as_bytes(),
                            &bytes.rule,
                            "curation rule",
                        )?;
                        require_transaction_value(
                            subscriptions,
                            owner.subscription.subscription_id.as_bytes(),
                            &bytes.subscription,
                            "subscription",
                        )?;
                        require_transaction_value(
                            belief_configs,
                            owner.belief_config.hash.as_bytes(),
                            &bytes.belief_config,
                            "belief config snapshot",
                        )?;
                    }
                    require_optional_transaction_value(
                        epochs,
                        command.agent_id.as_bytes(),
                        (durable_epoch != 0).then_some(expected_epoch_bytes.as_slice()),
                        "hydration epoch",
                    )?;
                    require_optional_transaction_value(
                        current,
                        command.agent_id.as_bytes(),
                        durable_hydration_id.as_deref().map(str::as_bytes),
                        "current hydration",
                    )?;
                    if let Some((prior_hydration, prior_activation)) = &prior {
                        let (expected_hydration, expected_activation) = prior_bytes
                            .as_ref()
                            .expect("prior bytes must exist for prior records");
                        require_transaction_value(
                            hydrations,
                            prior_hydration.hydration_id.as_bytes(),
                            expected_hydration,
                            "prior hydration",
                        )?;
                        require_transaction_value(
                            activations,
                            prior_activation.activation_id.as_bytes(),
                            expected_activation,
                            "prior activation",
                        )?;
                        if let Some((failed_hydration, failed_activation)) = &failed_prior_bytes {
                            hydrations.insert(
                                prior_hydration.hydration_id.as_bytes(),
                                failed_hydration.as_slice(),
                            )?;
                            activations.insert(
                                prior_activation.activation_id.as_bytes(),
                                failed_activation.as_slice(),
                            )?;
                        }
                    }
                    require_optional_transaction_value(
                        hydrations,
                        command.hydration_id.as_bytes(),
                        None,
                        "new hydration",
                    )?;
                    require_optional_transaction_value(
                        hydration_lease_index_tree,
                        lease_index.as_bytes(),
                        None,
                        "new hydration lease",
                    )?;
                    require_optional_transaction_value(
                        activations,
                        command.hydration_id.as_bytes(),
                        None,
                        "new activation",
                    )?;
                    hydrations.insert(command.hydration_id.as_bytes(), started_bytes.as_slice())?;
                    hydration_index_tree
                        .insert(hydration_index.as_bytes(), command.hydration_id.as_bytes())?;
                    hydration_lease_index_tree
                        .insert(lease_index.as_bytes(), command.hydration_id.as_bytes())?;
                    activations
                        .insert(command.hydration_id.as_bytes(), activation_bytes.as_slice())?;
                    activation_index_tree
                        .insert(activation_index.as_bytes(), command.hydration_id.as_bytes())?;
                    epochs.insert(command.agent_id.as_bytes(), epoch_bytes.as_slice())?;
                    current.insert(command.agent_id.as_bytes(), command.hydration_id.as_bytes())?;
                    Ok(())
                },
            )
            .map_err(to_agent_transition_error)?;
        self.flush_durable("hydration start")?;
        Ok(started)
    }

    /// Atomically fail one exact current hydration attempt.
    pub(super) fn fail_process_hydration_fenced(
        &self,
        command: &FailAgentHydrationCommand,
        owner: &AgentHydrationOwnerFence,
        checkpoint: &AgentHydrationCheckpoint,
    ) -> Result<AgentProcessHydrationRecord, StorageError> {
        command
            .validate()
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        let (hydration, activation) = self.require_hydration_pair(&command.hydration_id)?;
        if hydration.attempt_epoch != command.attempt_epoch
            || hydration.lease_id != command.lease_id
            || activation.attempt_epoch != command.attempt_epoch
            || activation.lease_id.as_deref() != Some(command.lease_id.as_str())
            || checkpoint.hydration_id != command.hydration_id
            || checkpoint.attempt_epoch != command.attempt_epoch
            || checkpoint.lease_id != command.lease_id
        {
            return Err(StorageError::Backpressure(
                "hydration failure command lost its identity, epoch, or lease fence".to_string(),
            ));
        }
        let fence = self.hydration_activation_fence(owner, &hydration, checkpoint)?;
        if hydration.status == AgentProcessHydrationStatus::Failed {
            if hydration.updated_at_seq == command.failed_at_seq
                && hydration.started_at_seq == command.expected_updated_at_seq
                && hydration.last_error.as_deref() == Some(command.error.as_str())
                && activation.status == AgentActivationStatus::Failed
                && activation.updated_at_seq == command.failed_at_seq
                && activation.last_error.as_deref() == Some(command.error.as_str())
            {
                let expected_activation =
                    serde_json::to_vec(&activation).map_err(to_storage_data)?;
                fence.transaction_hydration_and_one(&self.activations, |_, activations| {
                    require_transaction_value(
                        activations,
                        command.hydration_id.as_bytes(),
                        &expected_activation,
                        "failed hydration activation",
                    )?;
                    Ok(())
                })?;
                self.flush_durable("hydration failure replay")?;
                return Ok(hydration);
            }
            return Err(StorageError::Backpressure(
                "hydration failure replay conflicts with durable state".to_string(),
            ));
        }
        if hydration.status != AgentProcessHydrationStatus::Started
            || activation.status != AgentActivationStatus::Started
            || hydration.updated_at_seq != command.expected_updated_at_seq
            || activation.updated_at_seq != command.expected_updated_at_seq
        {
            return Err(StorageError::Backpressure(
                "hydration failure update fence changed".to_string(),
            ));
        }
        let mut failed_hydration = hydration.clone();
        failed_hydration.status = AgentProcessHydrationStatus::Failed;
        failed_hydration.last_error = Some(command.error.clone());
        failed_hydration.updated_at_seq = command.failed_at_seq;
        let mut failed_activation = activation.clone();
        failed_activation.status = AgentActivationStatus::Failed;
        failed_activation.last_error = Some(command.error.clone());
        failed_activation.updated_at_seq = command.failed_at_seq;
        failed_hydration
            .validate()
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        failed_activation.validate()?;

        let expected_hydration = serde_json::to_vec(&hydration).map_err(to_storage_data)?;
        let expected_activation = serde_json::to_vec(&activation).map_err(to_storage_data)?;
        let failed_hydration_bytes =
            serde_json::to_vec(&failed_hydration).map_err(to_storage_data)?;
        let failed_activation_bytes =
            serde_json::to_vec(&failed_activation).map_err(to_storage_data)?;
        fence.transaction_hydration_and_one(&self.activations, |hydrations, activations| {
            require_transaction_value(
                hydrations,
                command.hydration_id.as_bytes(),
                &expected_hydration,
                "hydration",
            )?;
            require_transaction_value(
                activations,
                command.hydration_id.as_bytes(),
                &expected_activation,
                "activation",
            )?;
            hydrations.insert(
                command.hydration_id.as_bytes(),
                failed_hydration_bytes.as_slice(),
            )?;
            activations.insert(
                command.hydration_id.as_bytes(),
                failed_activation_bytes.as_slice(),
            )?;
            Ok(())
        })?;
        self.flush_durable("hydration failure")?;
        Ok(failed_hydration)
    }

    /// Read one process-hydration attempt.
    pub fn get_process_hydration(
        &self,
        hydration_id: &str,
    ) -> Result<Option<AgentProcessHydrationRecord>, StorageError> {
        decode_optional(
            self.process_hydrations
                .get(hydration_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// List hydration attempts for one agent in durable sequence order.
    pub fn process_hydrations_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Vec<AgentProcessHydrationRecord>, StorageError> {
        let prefix = format!("{agent_id}::");
        let mut out = Vec::new();
        for item in self
            .process_hydrations_by_agent
            .scan_prefix(prefix.as_bytes())
        {
            let (_, value) = item.map_err(to_storage_io)?;
            let hydration_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(record) = self.get_process_hydration(&hydration_id)? {
                out.push(record);
            }
        }
        out.sort_by(|left, right| {
            left.started_at_seq
                .cmp(&right.started_at_seq)
                .then_with(|| left.hydration_id.cmp(&right.hydration_id))
        });
        Ok(out)
    }

    /// Read the exact hydration attempt historically owned by one supervisor lease.
    pub fn process_hydration_for_lease(
        &self,
        agent_id: &str,
        lease_id: &str,
    ) -> Result<Option<AgentProcessHydrationRecord>, StorageError> {
        if let Some(current) = self.current_process_hydration(agent_id)? {
            if current.lease_id == lease_id {
                let key = hydration_lease_key(agent_id, lease_id, current.attempt_epoch);
                let indexed = self
                    .process_hydrations_by_lease
                    .get(key.as_bytes())
                    .map_err(to_storage_io)?;
                if indexed.as_deref() != Some(current.hydration_id.as_bytes()) {
                    return Err(StorageError::MigrationConflict(format!(
                        "current hydration lease index conflicts for agent '{agent_id}' and lease '{lease_id}'"
                    )));
                }
                return Ok(Some(current));
            }
        }
        let prefix = hydration_lease_prefix(agent_id, lease_id);
        let Some(item) = self
            .process_hydrations_by_lease
            .scan_prefix(prefix.as_bytes())
            .next_back()
        else {
            return Ok(None);
        };
        let (key, raw_hydration_id) = item.map_err(to_storage_io)?;
        let hydration_id = std::str::from_utf8(&raw_hydration_id).map_err(|error| {
            StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, error))
        })?;
        let (hydration, _) = self.require_hydration_pair(hydration_id)?;
        if hydration.agent_id != agent_id
            || hydration.lease_id != lease_id
            || key.as_ref()
                != hydration_lease_key(agent_id, lease_id, hydration.attempt_epoch).as_bytes()
        {
            return Err(StorageError::MigrationConflict(format!(
                "hydration lease index conflicts for agent '{agent_id}' and lease '{lease_id}'"
            )));
        }
        Ok(Some(hydration))
    }

    /// Read the exact current hydration selected by the domain-owned epoch.
    pub fn current_process_hydration(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentProcessHydrationRecord>, StorageError> {
        let Some(hydration_id) = self.current_hydration_id(agent_id)? else {
            if self.current_hydration_epoch(agent_id)? != 0 {
                return Err(StorageError::MigrationConflict(format!(
                    "agent '{agent_id}' has a hydration epoch without a current hydration"
                )));
            }
            return Ok(None);
        };
        let (hydration, _) = self
            .require_hydration_pair(&hydration_id)
            .map_err(|error| {
                StorageError::MigrationConflict(format!(
                    "current hydration '{hydration_id}' is invalid for agent '{agent_id}': {error}"
                ))
            })?;
        let epoch = self.current_hydration_epoch(agent_id)?;
        if hydration.hydration_id != hydration_id
            || hydration.agent_id != agent_id
            || hydration.attempt_epoch != epoch
        {
            return Err(StorageError::MigrationConflict(format!(
                "current hydration '{hydration_id}' conflicts with agent epoch"
            )));
        }
        Ok(Some(hydration))
    }

    /// Read one durable operational-readiness proof.
    pub fn get_readiness_proof(
        &self,
        proof_id: &str,
    ) -> Result<Option<AgentReadinessProof>, StorageError> {
        let mut proof: Option<AgentReadinessProof> = decode_optional(
            self.readiness_proofs
                .get(proof_id.as_bytes())
                .map_err(to_storage_io)?,
        )?;
        if proof
            .as_ref()
            .is_some_and(AgentReadinessProof::requires_legacy_upgrade)
        {
            // TODO compat-shim: remove this lazy v1 rewrite after late-inserted W3A
            // fixture coverage is retired and all supported stores carry a complete
            // readiness schema marker produced after the final v1-capable release.
            let belief = BeliefReadinessReopenContract::open(self.db.clone())?;
            self.migrate_readiness_proof(proof_id.as_bytes(), &belief)?;
            proof = decode_optional(
                self.readiness_proofs
                    .get(proof_id.as_bytes())
                    .map_err(to_storage_io)?,
            )?;
        }
        if let Some(proof) = proof.as_ref() {
            proof.validate().map_err(|error| {
                StorageError::MigrationConflict(format!(
                    "invalid readiness proof '{}': {error}",
                    proof.proof_id
                ))
            })?;
            if proof.proof_id != proof_id {
                return Err(StorageError::MigrationConflict(
                    "readiness proof key conflicts with its identity".to_string(),
                ));
            }
            let belief = BeliefReadinessReopenContract::open(self.db.clone())?;
            let attestation =
                belief.verified_attestation(&proof.signal.attestation.attestation_id)?;
            if attestation != proof.signal.attestation {
                return Err(StorageError::MigrationConflict(format!(
                    "readiness proof '{}' diverges from belief authority",
                    proof.proof_id
                )));
            }
        }
        Ok(proof)
    }

    pub(crate) fn mark_agent_operational_fenced(
        &self,
        command: &MarkAgentOperationalCommand,
        owner: &AgentHydrationOwnerFence,
        checkpoint: &AgentHydrationCheckpoint,
    ) -> Result<AgentRecord, StorageError> {
        self.mark_agent_operational_inner(command, Some((owner, checkpoint)))
    }

    fn mark_agent_operational_inner(
        &self,
        command: &MarkAgentOperationalCommand,
        fence: Option<(&AgentHydrationOwnerFence, &AgentHydrationCheckpoint)>,
    ) -> Result<AgentRecord, StorageError> {
        command
            .validate()
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        let attestation = &command.readiness.signal.attestation;
        let agent_id = &attestation.agent_id;
        let Some(agent) = self.get_agent(agent_id)? else {
            return Err(StorageError::InvalidPath(format!(
                "unknown agent '{agent_id}'"
            )));
        };
        if let Some((owner, checkpoint)) = fence {
            validate_hydration_owner_fence(owner)?;
            checkpoint.validate()?;
            if owner.agent != agent
                || checkpoint.agent_id != *agent_id
                || checkpoint.hydration_id != command.hydration_id
                || checkpoint.stage != AgentHydrationCheckpointStage::ProjectionRequested
            {
                return Err(StorageError::Backpressure(
                    "readiness owner or checkpoint fence changed".to_string(),
                ));
            }
        }
        if agent.status == AgentStatus::Operational
            && self
                .get_process_hydration(&command.hydration_id)?
                .is_some_and(|hydration| hydration.status == AgentProcessHydrationStatus::Ready)
        {
            let replay = self.validate_operational_replay(command, agent)?;
            if let Some((owner, checkpoint)) = fence {
                self.verify_hydration_owner_checkpoint_fence(owner, checkpoint)?;
            }
            return Ok(replay);
        }
        if !matches!(
            agent.status,
            AgentStatus::Registered | AgentStatus::Operational
        ) || agent.updated_at_seq != command.readiness.expected_agent_updated_at_seq
            || agent.subject != attestation.belief_key.subject
            || agent.perspective_key != attestation.belief_key.perspective
            || agent.branch_scope != attestation.belief_key.branch_scope
        {
            return Err(StorageError::Backpressure(format!(
                "agent lifecycle, scope, or sequence fence changed for '{agent_id}'"
            )));
        }

        let subscription = self
            .get_subscription(&attestation.subscription_id)?
            .ok_or_else(|| {
                StorageError::InvalidPath(format!(
                    "unknown readiness subscription '{}'",
                    attestation.subscription_id
                ))
            })?;
        if subscription.agent_id != *agent_id
            || subscription.status != AgentSubscriptionStatus::Active
            || subscription.belief_key != attestation.belief_key
            || subscription.last_delivered_seq > attestation.attested_at_seq
            || subscription.last_delivered_seq == attestation.attested_at_seq
                && subscription.last_delivered_revision_id.as_deref()
                    != Some(attestation.belief_revision_id.as_str())
            || fence.is_some_and(|(owner, _)| owner.subscription != subscription)
        {
            return Err(StorageError::Backpressure(format!(
                "readiness subscription fence changed for '{}'",
                attestation.subscription_id
            )));
        }

        let (hydration, activation) = self.require_hydration_pair(&command.hydration_id)?;
        if hydration.agent_id != *agent_id
            || hydration.status != AgentProcessHydrationStatus::Started
            || hydration.readiness_proof_id.is_some()
            || hydration.attempt_epoch != command.attempt_epoch
            || hydration.lease_id != command.lease_id
            || hydration.updated_at_seq != command.expected_hydration_updated_at_seq
            || self.current_hydration_epoch(agent_id)? != command.attempt_epoch
            || self.current_hydration_id(agent_id)?.as_deref()
                != Some(command.hydration_id.as_str())
        {
            return Err(StorageError::Backpressure(format!(
                "process hydration fence changed for '{}'",
                command.hydration_id
            )));
        }

        if activation.agent_id != *agent_id
            || activation.status != AgentActivationStatus::Started
            || activation.attempt_epoch != command.attempt_epoch
            || activation.lease_id.as_deref() != Some(command.lease_id.as_str())
            || activation.updated_at_seq != command.expected_hydration_updated_at_seq
        {
            return Err(StorageError::Backpressure(format!(
                "activation diagnostic fence changed for '{}'",
                command.hydration_id
            )));
        }

        let mut operational = agent.clone();
        operational.status = AgentStatus::Operational;
        operational.updated_at_seq = command.updated_at_seq;
        let mut ready_hydration = hydration.clone();
        ready_hydration.status = AgentProcessHydrationStatus::Ready;
        ready_hydration.readiness_proof_id = Some(command.readiness.proof_id.clone());
        ready_hydration.updated_at_seq = command.updated_at_seq;
        let mut activated = activation.clone();
        activated.status = AgentActivationStatus::Activated;
        activated.last_error = None;
        activated.updated_at_seq = command.updated_at_seq;

        let expected_agent = serde_json::to_vec(&agent).map_err(to_storage_data)?;
        let expected_subscription = serde_json::to_vec(&subscription).map_err(to_storage_data)?;
        let expected_hydration = serde_json::to_vec(&hydration).map_err(to_storage_data)?;
        let expected_activation = serde_json::to_vec(&activation).map_err(to_storage_data)?;
        let operational_bytes = serde_json::to_vec(&operational).map_err(to_storage_data)?;
        let hydration_bytes = serde_json::to_vec(&ready_hydration).map_err(to_storage_data)?;
        let activation_bytes = serde_json::to_vec(&activated).map_err(to_storage_data)?;
        let proof_bytes = serde_json::to_vec(&command.readiness).map_err(to_storage_data)?;
        let owner_bytes = fence
            .map(|(owner, _)| encode_hydration_owner_fence(owner))
            .transpose()?;
        let checkpoint_bytes = fence
            .map(|(_, checkpoint)| serde_json::to_vec(checkpoint).map_err(to_storage_data))
            .transpose()?;
        let belief_config_tree = fence
            .map(|(owner, _)| &owner.belief_config.tree)
            .unwrap_or(&self.runtime_schema);
        let epoch_bytes = encode_epoch(command.attempt_epoch);
        let status_key = agent_status_key(
            operational.status.index_key(),
            operational.updated_at_seq,
            &operational.agent_id,
        );
        let prior_status_key = agent_status_key(
            agent.status.index_key(),
            agent.updated_at_seq,
            &agent.agent_id,
        );

        (
            &self.agents,
            &self.agent_by_status,
            &self.bootstrap_receipts,
            &self.directives,
            &self.curation_rules,
            belief_config_tree,
            &self.subscriptions,
            &self.process_hydrations,
            &self.activations,
            &self.hydration_checkpoints,
            &self.readiness_proofs,
            &self.hydration_epochs,
            &self.current_hydrations,
        )
            .transaction(
                |(
                    agents,
                    status,
                    receipts,
                    directives,
                    rules,
                    belief_configs,
                    subscriptions,
                    hydrations,
                    activations,
                    checkpoints,
                    proofs,
                    epochs,
                    current,
                )| {
                    require_transaction_value(
                        agents,
                        agent_id.as_bytes(),
                        &expected_agent,
                        "agent",
                    )?;
                    if let (Some((owner, checkpoint)), Some(bytes), Some(checkpoint_bytes)) =
                        (fence, owner_bytes.as_ref(), checkpoint_bytes.as_ref())
                    {
                        require_transaction_value(
                            receipts,
                            owner.receipt.bootstrap_id.as_bytes(),
                            &bytes.receipt,
                            "bootstrap receipt",
                        )?;
                        require_transaction_value(
                            directives,
                            owner.directive.directive_id.as_bytes(),
                            &bytes.directive,
                            "directive",
                        )?;
                        require_transaction_value(
                            rules,
                            owner.rule.rule_id.as_bytes(),
                            &bytes.rule,
                            "curation rule",
                        )?;
                        require_transaction_value(
                            belief_configs,
                            owner.belief_config.hash.as_bytes(),
                            &bytes.belief_config,
                            "belief config snapshot",
                        )?;
                        require_transaction_value(
                            checkpoints,
                            checkpoint.hydration_id.as_bytes(),
                            checkpoint_bytes,
                            "hydration checkpoint",
                        )?;
                    }
                    require_transaction_value(
                        subscriptions,
                        attestation.subscription_id.as_bytes(),
                        &expected_subscription,
                        "subscription",
                    )?;
                    require_transaction_value(
                        hydrations,
                        command.hydration_id.as_bytes(),
                        &expected_hydration,
                        "hydration",
                    )?;
                    require_transaction_value(
                        activations,
                        command.hydration_id.as_bytes(),
                        &expected_activation,
                        "activation",
                    )?;
                    require_transaction_value(
                        epochs,
                        agent_id.as_bytes(),
                        &epoch_bytes,
                        "hydration epoch",
                    )?;
                    require_transaction_value(
                        current,
                        agent_id.as_bytes(),
                        command.hydration_id.as_bytes(),
                        "current hydration",
                    )?;
                    require_transaction_value(
                        status,
                        prior_status_key.as_bytes(),
                        agent_id.as_bytes(),
                        "prior agent status index",
                    )?;
                    if let Some(existing) = proofs.get(command.readiness.proof_id.as_bytes())? {
                        if existing.as_ref() != proof_bytes.as_slice() {
                            return Err(ConflictableTransactionError::Abort(format!(
                                "readiness proof '{}' conflicts with durable state",
                                command.readiness.proof_id
                            )));
                        }
                    }
                    agents.insert(agent_id.as_bytes(), operational_bytes.clone())?;
                    status.remove(prior_status_key.as_bytes())?;
                    status.insert(status_key.as_bytes(), agent_id.as_bytes())?;
                    hydrations.insert(command.hydration_id.as_bytes(), hydration_bytes.clone())?;
                    activations
                        .insert(command.hydration_id.as_bytes(), activation_bytes.clone())?;
                    proofs.insert(command.readiness.proof_id.as_bytes(), proof_bytes.clone())?;
                    Ok(())
                },
            )
            .map_err(to_agent_transition_error)?;
        self.flush_durable("operational transition")?;
        Ok(operational)
    }

    fn validate_operational_replay(
        &self,
        command: &MarkAgentOperationalCommand,
        agent: AgentRecord,
    ) -> Result<AgentRecord, StorageError> {
        let proof = self.get_readiness_proof(&command.readiness.proof_id)?;
        let hydration = self.get_process_hydration(&command.hydration_id)?;
        let activation = self.get_activation(&command.hydration_id)?;
        if agent.updated_at_seq == command.updated_at_seq
            && proof.as_ref() == Some(&command.readiness)
            && hydration.as_ref().is_some_and(|record| {
                record.agent_id == agent.agent_id
                    && record.status == AgentProcessHydrationStatus::Ready
                    && record.attempt_epoch == command.attempt_epoch
                    && record.lease_id == command.lease_id
                    && record.updated_at_seq == command.updated_at_seq
                    && record.readiness_proof_id.as_deref()
                        == Some(command.readiness.proof_id.as_str())
            })
            && activation.as_ref().is_some_and(|record| {
                record.agent_id == agent.agent_id
                    && record.status == AgentActivationStatus::Activated
                    && record.attempt_epoch == command.attempt_epoch
                    && record.lease_id.as_deref() == Some(command.lease_id.as_str())
                    && record.updated_at_seq == command.updated_at_seq
            })
            && self.current_hydration_epoch(&agent.agent_id)? == command.attempt_epoch
            && self.current_hydration_id(&agent.agent_id)?.as_deref()
                == Some(command.hydration_id.as_str())
        {
            self.flush_durable("operational transition replay")?;
            return Ok(agent);
        }
        Err(StorageError::InvalidPath(format!(
            "operational replay conflicts for agent '{}'",
            agent.agent_id
        )))
    }

    /// Persist one complete decision and exact command outbox atomically.
    ///
    /// This is the durable boundary used by recurring curation actors. Command
    /// decisions cannot become visible without the payload needed after reopen.
    pub fn put_curation_outcome(
        &self,
        outcome: &AgentCurationOutcome,
    ) -> Result<AgentCurationOutcome, StorageError> {
        if outcome.decision.decision == AgentDecisionKind::GoalMutationCommand {
            return Err(StorageError::InvalidPath(
                "goal mutation outcome requires an exact selected satisfaction review".to_string(),
            ));
        }
        let outbox = AgentDecisionOutboxRecord::from_outcome(outcome)?;
        self.persist_outcome_transaction(outcome, outbox.as_ref(), None)?;
        self.flush_durable("agent curation outcome")?;
        self.outcome_for_decision(&outcome.decision.decision_id)
    }

    /// Persist one selected delivery outcome under exact lifecycle and cursor fences.
    pub fn put_selected_delivery_outcome(
        &self,
        selection: &AgentDeliverySelection,
        outcome: &AgentCurationOutcome,
    ) -> Result<AgentCurationOutcome, StorageError> {
        selection.validate()?;
        validate_delivery_outcome(selection, outcome)?;
        let outbox = AgentDecisionOutboxRecord::from_outcome(outcome)?;
        let agent = self.require_selected_agent(
            &selection.delivery.agent_id,
            selection.expected_agent_updated_at_seq,
        )?;
        let subscription = self.require_selected_subscription(
            &selection.delivery.subscription_id,
            &selection.delivery.agent_id,
            &selection.belief_key,
            selection.expected_subscription_updated_at_seq,
        )?;
        if subscription.last_delivered_revision_id != selection.expected_delivered_revision_id
            || subscription.last_delivered_seq != selection.expected_delivered_seq
        {
            return Err(StorageError::Backpressure(
                "subscription cursor changed before decision commit".to_string(),
            ));
        }
        self.persist_outcome_with_owner_fences(
            outcome,
            outbox.as_ref(),
            &agent,
            &subscription,
            None,
            None,
        )?;
        self.flush_durable("selected delivery outcome")?;
        self.outcome_for_decision(&outcome.decision.decision_id)
    }

    /// Persist one selected satisfaction outcome under exact owner and cursor fences.
    pub fn put_selected_satisfaction_outcome(
        &self,
        selection: &AgentSatisfactionReviewSelection,
        outcome: &AgentCurationOutcome,
    ) -> Result<AgentCurationOutcome, StorageError> {
        selection.validate()?;
        validate_satisfaction_outcome(selection, outcome)?;
        let outbox = AgentDecisionOutboxRecord::from_outcome(outcome)?;
        let agent = self.require_selected_agent(
            &selection.review.agent_id,
            selection.expected_agent_updated_at_seq,
        )?;
        let subscription = self.require_selected_subscription(
            &selection.review.subscription_id,
            &selection.review.agent_id,
            &selection.belief_key,
            selection.expected_subscription_updated_at_seq,
        )?;
        let cursor = self
            .satisfaction_review_cursor(&selection.cursor_identity)?
            .unwrap_or_else(|| {
                AgentSatisfactionReviewCursor::empty(selection.cursor_identity.clone())
            });
        if cursor.last_reviewed_revision_id != selection.expected_reviewed_revision_id
            || cursor.last_reviewed_seq != selection.expected_reviewed_seq
            || cursor.updated_at_seq != selection.expected_cursor_updated_at_seq
        {
            return Err(StorageError::Backpressure(
                "satisfaction cursor changed before decision commit".to_string(),
            ));
        }
        self.persist_outcome_with_owner_fences(
            outcome,
            outbox.as_ref(),
            &agent,
            &subscription,
            Some(&selection.review),
            Some((
                &selection.cursor_identity,
                (cursor.last_reviewed_seq != 0).then_some(&cursor),
            )),
        )?;
        self.flush_durable("selected satisfaction outcome")?;
        self.outcome_for_decision(&outcome.decision.decision_id)
    }

    /// Read one exact persisted outcome including its durable command payload.
    pub fn outcome_for_decision(
        &self,
        decision_id: &str,
    ) -> Result<AgentCurationOutcome, StorageError> {
        let decision = self.get_decision(decision_id)?.ok_or_else(|| {
            StorageError::InvalidPath(format!("unknown agent decision '{decision_id}'"))
        })?;
        let outbox = self.decision_outbox(decision_id)?;
        outcome_from_durable(decision, outbox)
    }

    /// Read the complete durable outcome for one exact selected delivery.
    pub fn outcome_for_selected_delivery(
        &self,
        selection: &AgentDeliverySelection,
    ) -> Result<Option<AgentCurationOutcome>, StorageError> {
        selection.validate()?;
        let prefix = format!("{}::", selection.delivery.belief_revision_id);
        let satisfaction_reviews = self.satisfaction_review_index_map()?;
        let mut matching = Vec::new();
        for item in self.decisions_by_revision.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.map_err(to_storage_io)?;
            let decision_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            let decision = self.get_decision(&decision_id)?.ok_or_else(|| {
                StorageError::MigrationConflict(format!(
                    "selected delivery index references missing decision '{decision_id}'"
                ))
            })?;
            if !satisfaction_reviews.contains_key(&decision.decision_id)
                && decision.agent_id == selection.delivery.agent_id
                && decision.subscription_id == selection.delivery.subscription_id
                && decision.input_refs.belief_key == selection.belief_key
                && decision.input_refs.belief_revision_id.as_deref()
                    == Some(selection.delivery.belief_revision_id.as_str())
                && decision.created_at_seq == selection.delivery.revision_seq
            {
                matching.push(decision);
            }
        }
        if matching.len() > 1 {
            return Err(StorageError::MigrationConflict(
                "selected delivery has multiple durable decisions".to_string(),
            ));
        }
        matching
            .pop()
            .map(|decision| {
                let outbox = self.decision_outbox(&decision.decision_id)?;
                outcome_from_durable(decision, outbox)
            })
            .transpose()
    }

    /// Read the complete durable outcome for one exact satisfaction review.
    pub fn outcome_for_satisfaction_review(
        &self,
        review: &AgentSatisfactionReview,
    ) -> Result<Option<AgentCurationOutcome>, StorageError> {
        let Some(decision) = self.decision_by_satisfaction_review(review)? else {
            return Ok(None);
        };
        let outbox = self.decision_outbox(&decision.decision_id)?;
        outcome_from_durable(decision, outbox).map(Some)
    }

    /// Read the exact command outbox retained for one decision.
    pub fn decision_outbox(
        &self,
        decision_id: &str,
    ) -> Result<Option<AgentDecisionOutboxRecord>, StorageError> {
        decode_optional(
            self.decision_outbox
                .get(decision_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Persist a curation decision unless the dedupe and revision key already exists.
    ///
    /// This compatibility operation may create a legacy command decision without
    /// an outbox. The semantic enablement audit rejects such state.
    pub fn put_decision(
        &self,
        decision: &AgentCurationDecision,
    ) -> Result<AgentCurationDecision, StorageError> {
        decision.validate()?;
        if let Some(existing) = self.decision_by_dedupe_and_revision(
            &decision.dedupe_key,
            decision.input_refs.belief_revision_id.as_deref(),
        )? {
            return Ok(existing);
        }
        let dedupe_index_key = decision_dedupe_revision_key(
            &decision.dedupe_key,
            decision.input_refs.belief_revision_id.as_deref(),
        );
        self.insert_decision_record(decision, &dedupe_index_key)
    }

    /// Persist a satisfaction decision unless its review was already recorded.
    ///
    /// Satisfaction reviews use review identity for idempotency because the
    /// same belief revision may be reviewed more than once across retries and
    /// reopen checkpoints.
    pub fn put_satisfaction_decision(
        &self,
        review: &AgentSatisfactionReview,
        decision: &AgentCurationDecision,
    ) -> Result<AgentCurationDecision, StorageError> {
        review.validate()?;
        decision.validate()?;
        if decision.agent_id != review.agent_id {
            return Err(StorageError::InvalidPath(
                "satisfaction review agent mismatch".to_string(),
            ));
        }
        if decision.subscription_id != review.subscription_id {
            return Err(StorageError::InvalidPath(
                "satisfaction review subscription mismatch".to_string(),
            ));
        }
        if decision.created_at_seq != review.review_seq {
            return Err(StorageError::InvalidPath(
                "satisfaction review seq mismatch".to_string(),
            ));
        }
        if let Some(existing) = self.decision_by_satisfaction_review(review)? {
            return Ok(existing);
        }
        if let Some(existing) = self.get_decision(&decision.decision_id)? {
            if existing != *decision {
                return Err(StorageError::InvalidPath(
                    "satisfaction decision id conflict".to_string(),
                ));
            }
        }
        let dedupe_index_key =
            decision_dedupe_satisfaction_review_key(&decision.dedupe_key, review);
        self.insert_legacy_decision_transaction(decision, &dedupe_index_key, Some(review))
    }

    /// Read one curation decision by id.
    pub fn get_decision(
        &self,
        decision_id: &str,
    ) -> Result<Option<AgentCurationDecision>, StorageError> {
        decode_optional(
            self.decisions
                .get(decision_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// List recent decisions for an agent in deterministic sequence and id order.
    pub fn recent_decisions(
        &self,
        agent_id: &str,
        limit: usize,
    ) -> Result<Vec<AgentCurationDecision>, StorageError> {
        let prefix = format!("{agent_id}::");
        let mut out = Vec::new();
        for item in self.decisions_by_agent.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.map_err(to_storage_io)?;
            let decision_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(decision) = self.get_decision(&decision_id)? {
                out.push(decision);
            }
        }
        out.sort_by(|left, right| {
            left.created_at_seq
                .cmp(&right.created_at_seq)
                .then_with(|| left.decision_id.cmp(&right.decision_id))
        });
        if out.len() > limit {
            out = out.split_off(out.len() - limit);
        }
        Ok(out)
    }

    /// Return the latest decision stored for a dedupe key.
    pub fn decision_by_dedupe_key(
        &self,
        dedupe_key: &AgentCurationDedupeKey,
    ) -> Result<Option<AgentCurationDecision>, StorageError> {
        let prefix = format!("{}::", dedupe_key.index_key());
        let mut decisions = Vec::new();
        for item in self.decisions_by_dedupe.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.map_err(to_storage_io)?;
            let decision_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(decision) = self.get_decision(&decision_id)? {
                decisions.push(decision);
            }
        }
        decisions.sort_by(|left, right| {
            left.created_at_seq
                .cmp(&right.created_at_seq)
                .then_with(|| left.decision_id.cmp(&right.decision_id))
        });
        Ok(decisions.pop())
    }

    /// Read the decision for an exact dedupe and belief revision pair.
    pub fn decision_by_dedupe_and_revision(
        &self,
        dedupe_key: &AgentCurationDedupeKey,
        revision_id: Option<&str>,
    ) -> Result<Option<AgentCurationDecision>, StorageError> {
        let key = decision_dedupe_revision_key(dedupe_key, revision_id);
        let Some(raw) = self
            .decisions_by_dedupe
            .get(key.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let decision_id = String::from_utf8(raw.to_vec()).map_err(to_storage_utf8)?;
        self.get_decision(&decision_id)
    }

    /// Read the decision recorded for one satisfaction review.
    pub fn decision_by_satisfaction_review(
        &self,
        review: &AgentSatisfactionReview,
    ) -> Result<Option<AgentCurationDecision>, StorageError> {
        review.validate()?;
        let Some(raw) = self
            .satisfaction_decisions_by_review
            .get(review.index_key().as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let decision_id = String::from_utf8(raw.to_vec()).map_err(to_storage_utf8)?;
        self.get_decision(&decision_id)
    }

    /// Read the independent cursor for one satisfaction-review consumer.
    pub fn satisfaction_review_cursor(
        &self,
        identity: &AgentSatisfactionCursorIdentity,
    ) -> Result<Option<AgentSatisfactionReviewCursor>, StorageError> {
        identity.validate()?;
        decode_optional(
            self.satisfaction_review_cursors
                .get(identity.index_key().as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Advance a satisfaction cursor after its decision and sink receipt are durable.
    pub fn advance_satisfaction_cursor_cas(
        &self,
        intent: &AgentSatisfactionCursorCasIntent,
    ) -> Result<AgentSatisfactionReviewCursor, StorageError> {
        intent.validate()?;
        let selection = &intent.selection;
        let agent = self.require_selected_agent(
            &selection.review.agent_id,
            selection.expected_agent_updated_at_seq,
        )?;
        let subscription = self.require_selected_subscription(
            &selection.review.subscription_id,
            &selection.review.agent_id,
            &selection.belief_key,
            selection.expected_subscription_updated_at_seq,
        )?;
        let decision = self
            .decision_by_satisfaction_review(&selection.review)?
            .ok_or_else(|| {
                StorageError::Backpressure(
                    "satisfaction cursor cannot advance before its decision".to_string(),
                )
            })?;
        if matches!(decision.decision, AgentDecisionKind::GoalMutationCommand) {
            let receipt = self
                .sink_receipt_by_decision(&decision.decision_id)?
                .ok_or_else(|| {
                    StorageError::Backpressure(
                        "satisfaction cursor cannot advance before its sink receipt".to_string(),
                    )
                })?;
            let outbox = self
                .decision_outbox(&decision.decision_id)?
                .ok_or_else(|| {
                    StorageError::MigrationConflict(
                        "satisfaction mutation has no exact outbox".to_string(),
                    )
                })?;
            outbox.validate_for(&decision)?;
            validate_sink_receipt_for_decision(&receipt, &decision, Some(&outbox))?;
        }
        let expected = self
            .satisfaction_review_cursor(&selection.cursor_identity)?
            .unwrap_or_else(|| {
                AgentSatisfactionReviewCursor::empty(selection.cursor_identity.clone())
            });
        if expected.last_reviewed_revision_id != selection.expected_reviewed_revision_id
            || expected.last_reviewed_seq != selection.expected_reviewed_seq
            || expected.updated_at_seq != selection.expected_cursor_updated_at_seq
        {
            return Err(StorageError::Backpressure(
                "satisfaction cursor changed before advancement".to_string(),
            ));
        }
        let advanced = AgentSatisfactionReviewCursor {
            identity: selection.cursor_identity.clone(),
            last_reviewed_revision_id: Some(selection.belief_revision_id.clone()),
            last_reviewed_seq: selection.belief_revision_seq,
            updated_at_seq: intent.advanced_at_seq,
        };
        advanced.validate()?;
        let key = selection.cursor_identity.index_key();
        let expected_bytes = if expected.last_reviewed_seq == 0 {
            None
        } else {
            Some(serde_json::to_vec(&expected).map_err(to_storage_data)?)
        };
        let advanced_bytes = serde_json::to_vec(&advanced).map_err(to_storage_data)?;
        let agent_bytes = serde_json::to_vec(&agent).map_err(to_storage_data)?;
        let subscription_bytes = serde_json::to_vec(&subscription).map_err(to_storage_data)?;
        (
            &self.agents,
            &self.subscriptions,
            &self.satisfaction_review_cursors,
        )
            .transaction(|(agents, subscriptions, cursors)| {
                require_transaction_value(
                    agents,
                    selection.review.agent_id.as_bytes(),
                    &agent_bytes,
                    "selected agent",
                )?;
                require_transaction_value(
                    subscriptions,
                    selection.review.subscription_id.as_bytes(),
                    &subscription_bytes,
                    "selected subscription",
                )?;
                require_optional_transaction_value(
                    cursors,
                    key.as_bytes(),
                    expected_bytes.as_deref(),
                    "satisfaction cursor",
                )?;
                cursors.insert(key.as_bytes(), advanced_bytes.as_slice())?;
                Ok(())
            })
            .map_err(to_agent_transition_error)?;
        self.flush_durable("satisfaction cursor advancement")?;
        Ok(advanced)
    }

    /// Create one hydration checkpoint or replay its exact durable value.
    #[cfg(test)]
    pub(super) fn put_hydration_checkpoint(
        &self,
        checkpoint: &AgentHydrationCheckpoint,
    ) -> Result<AgentHydrationCheckpoint, StorageError> {
        self.put_hydration_checkpoint_inner(checkpoint, None)
    }

    pub(crate) fn put_hydration_checkpoint_fenced(
        &self,
        checkpoint: &AgentHydrationCheckpoint,
        owner: &AgentHydrationOwnerFence,
    ) -> Result<AgentHydrationCheckpoint, StorageError> {
        self.put_hydration_checkpoint_inner(checkpoint, Some(owner))
    }

    fn put_hydration_checkpoint_inner(
        &self,
        checkpoint: &AgentHydrationCheckpoint,
        owner: Option<&AgentHydrationOwnerFence>,
    ) -> Result<AgentHydrationCheckpoint, StorageError> {
        checkpoint.validate()?;
        if let Some(owner) = owner {
            validate_hydration_owner_fence(owner)?;
            if owner.agent.agent_id != checkpoint.agent_id
                || owner.subscription.subscription_id != checkpoint.subscription_id
            {
                return Err(StorageError::InvalidPath(
                    "hydration checkpoint owner fence has divergent identities".to_string(),
                ));
            }
        }
        let hydration = self
            .get_process_hydration(&checkpoint.hydration_id)?
            .ok_or_else(|| {
                StorageError::InvalidPath(format!(
                    "unknown process hydration '{}'",
                    checkpoint.hydration_id
                ))
            })?;
        if hydration.agent_id != checkpoint.agent_id
            || hydration.attempt_epoch != checkpoint.attempt_epoch
            || hydration.lease_id != checkpoint.lease_id
            || hydration.status != AgentProcessHydrationStatus::Started
        {
            return Err(StorageError::Backpressure(
                "hydration checkpoint lost its process-hydration fence".to_string(),
            ));
        }
        let subscription = self
            .get_subscription(&checkpoint.subscription_id)?
            .ok_or_else(|| {
                StorageError::InvalidPath(format!(
                    "unknown hydration subscription '{}'",
                    checkpoint.subscription_id
                ))
            })?;
        if subscription.agent_id != checkpoint.agent_id
            || subscription.status != AgentSubscriptionStatus::Active
            || owner.is_some_and(|owner| owner.subscription != subscription)
        {
            return Err(StorageError::Backpressure(
                "hydration checkpoint lost its active subscription fence".to_string(),
            ));
        }
        let key = checkpoint.hydration_id.as_bytes();
        let encoded = serde_json::to_vec(checkpoint).map_err(to_storage_data)?;
        let hydration_bytes = serde_json::to_vec(&hydration).map_err(to_storage_data)?;
        let subscription_bytes = serde_json::to_vec(&subscription).map_err(to_storage_data)?;
        let owner_bytes = owner.map(encode_hydration_owner_fence).transpose()?;
        let belief_config_tree = owner
            .map(|owner| &owner.belief_config.tree)
            .unwrap_or(&self.runtime_schema);
        (
            &self.agents,
            &self.bootstrap_receipts,
            &self.directives,
            &self.curation_rules,
            belief_config_tree,
            &self.process_hydrations,
            &self.subscriptions,
            &self.hydration_checkpoints,
        )
            .transaction(
                |(
                    agents,
                    receipts,
                    directives,
                    rules,
                    belief_configs,
                    hydrations,
                    subscriptions,
                    checkpoints,
                )| {
                    if let (Some(owner), Some(bytes)) = (owner, owner_bytes.as_ref()) {
                        require_transaction_value(
                            agents,
                            owner.agent.agent_id.as_bytes(),
                            &bytes.agent,
                            "agent",
                        )?;
                        require_transaction_value(
                            receipts,
                            owner.receipt.bootstrap_id.as_bytes(),
                            &bytes.receipt,
                            "bootstrap receipt",
                        )?;
                        require_transaction_value(
                            directives,
                            owner.directive.directive_id.as_bytes(),
                            &bytes.directive,
                            "directive",
                        )?;
                        require_transaction_value(
                            rules,
                            owner.rule.rule_id.as_bytes(),
                            &bytes.rule,
                            "curation rule",
                        )?;
                        require_transaction_value(
                            belief_configs,
                            owner.belief_config.hash.as_bytes(),
                            &bytes.belief_config,
                            "belief config snapshot",
                        )?;
                    }
                    require_transaction_value(
                        hydrations,
                        key,
                        &hydration_bytes,
                        "process hydration",
                    )?;
                    require_transaction_value(
                        subscriptions,
                        checkpoint.subscription_id.as_bytes(),
                        &subscription_bytes,
                        "hydration subscription",
                    )?;
                    insert_exact_transaction_value(
                        checkpoints,
                        key,
                        &encoded,
                        "hydration checkpoint",
                    )
                },
            )
            .map_err(to_agent_transition_error)?;
        self.flush_durable("hydration checkpoint creation")?;
        Ok(checkpoint.clone())
    }

    /// Advance one hydration checkpoint under exact stage and lease fencing.
    #[cfg(test)]
    pub(super) fn advance_hydration_checkpoint_cas(
        &self,
        expected: &AgentHydrationCheckpoint,
        advanced: &AgentHydrationCheckpoint,
    ) -> Result<AgentHydrationCheckpoint, StorageError> {
        self.advance_hydration_checkpoint_cas_inner(expected, advanced, None)
    }

    pub(crate) fn advance_hydration_checkpoint_cas_fenced(
        &self,
        expected: &AgentHydrationCheckpoint,
        advanced: &AgentHydrationCheckpoint,
        owner: &AgentHydrationOwnerFence,
    ) -> Result<AgentHydrationCheckpoint, StorageError> {
        self.advance_hydration_checkpoint_cas_inner(expected, advanced, Some(owner))
    }

    fn advance_hydration_checkpoint_cas_inner(
        &self,
        expected: &AgentHydrationCheckpoint,
        advanced: &AgentHydrationCheckpoint,
        owner: Option<&AgentHydrationOwnerFence>,
    ) -> Result<AgentHydrationCheckpoint, StorageError> {
        expected.validate()?;
        advanced.validate()?;
        if let Some(owner) = owner {
            validate_hydration_owner_fence(owner)?;
            if owner.agent.agent_id != expected.agent_id
                || owner.subscription.subscription_id != expected.subscription_id
            {
                return Err(StorageError::InvalidPath(
                    "hydration checkpoint owner fence has divergent identities".to_string(),
                ));
            }
        }
        if expected.hydration_id != advanced.hydration_id
            || expected.agent_id != advanced.agent_id
            || expected.attempt_epoch != advanced.attempt_epoch
            || expected.lease_id != advanced.lease_id
            || expected.subscription_id != advanced.subscription_id
            || expected.selected_revision_id != advanced.selected_revision_id
            || expected.selected_revision_seq != advanced.selected_revision_seq
            || (expected.stage == AgentHydrationCheckpointStage::Attested
                && expected.attestation_id != advanced.attestation_id)
            || advanced.updated_at_seq <= expected.updated_at_seq
        {
            return Err(StorageError::InvalidPath(
                "hydration checkpoint advancement must preserve identity and advance stage"
                    .to_string(),
            ));
        }
        let adjacent_stage = matches!(
            (expected.stage, advanced.stage),
            (
                AgentHydrationCheckpointStage::Selected,
                AgentHydrationCheckpointStage::Attested
            ) | (
                AgentHydrationCheckpointStage::Attested,
                AgentHydrationCheckpointStage::ProjectionRequested
            )
        );
        if !adjacent_stage {
            return Err(StorageError::InvalidPath(
                "hydration checkpoint must advance exactly one stage".to_string(),
            ));
        }
        let key = expected.hydration_id.as_bytes();
        let current: AgentHydrationCheckpoint =
            decode_optional(self.hydration_checkpoints.get(key).map_err(to_storage_io)?)?
                .ok_or_else(|| {
                    StorageError::Backpressure("hydration checkpoint disappeared".to_string())
                })?;
        if current == *advanced {
            self.flush_durable("hydration checkpoint advancement replay")?;
            return Ok(advanced.clone());
        }
        if current != *expected {
            return Err(StorageError::Backpressure(
                "hydration checkpoint changed before advancement".to_string(),
            ));
        }
        let hydration = self
            .get_process_hydration(&expected.hydration_id)?
            .ok_or_else(|| {
                StorageError::Backpressure("process hydration disappeared".to_string())
            })?;
        if hydration.agent_id != expected.agent_id
            || hydration.attempt_epoch != expected.attempt_epoch
            || hydration.lease_id != expected.lease_id
            || hydration.status != AgentProcessHydrationStatus::Started
        {
            return Err(StorageError::Backpressure(
                "hydration checkpoint lost its active epoch or lease".to_string(),
            ));
        }
        let subscription = self
            .get_subscription(&expected.subscription_id)?
            .ok_or_else(|| {
                StorageError::Backpressure("hydration subscription disappeared".to_string())
            })?;
        if subscription.agent_id != expected.agent_id
            || subscription.status != AgentSubscriptionStatus::Active
            || owner.is_some_and(|owner| owner.subscription != subscription)
        {
            return Err(StorageError::Backpressure(
                "hydration checkpoint lost its active subscription fence".to_string(),
            ));
        }
        let expected_bytes = serde_json::to_vec(expected).map_err(to_storage_data)?;
        let advanced_bytes = serde_json::to_vec(advanced).map_err(to_storage_data)?;
        let hydration_bytes = serde_json::to_vec(&hydration).map_err(to_storage_data)?;
        let subscription_bytes = serde_json::to_vec(&subscription).map_err(to_storage_data)?;
        let owner_bytes = owner.map(encode_hydration_owner_fence).transpose()?;
        let belief_config_tree = owner
            .map(|owner| &owner.belief_config.tree)
            .unwrap_or(&self.runtime_schema);
        (
            &self.agents,
            &self.bootstrap_receipts,
            &self.directives,
            &self.curation_rules,
            belief_config_tree,
            &self.process_hydrations,
            &self.subscriptions,
            &self.hydration_checkpoints,
        )
            .transaction(
                |(
                    agents,
                    receipts,
                    directives,
                    rules,
                    belief_configs,
                    hydrations,
                    subscriptions,
                    checkpoints,
                )| {
                    if let (Some(owner), Some(bytes)) = (owner, owner_bytes.as_ref()) {
                        require_transaction_value(
                            agents,
                            owner.agent.agent_id.as_bytes(),
                            &bytes.agent,
                            "agent",
                        )?;
                        require_transaction_value(
                            receipts,
                            owner.receipt.bootstrap_id.as_bytes(),
                            &bytes.receipt,
                            "bootstrap receipt",
                        )?;
                        require_transaction_value(
                            directives,
                            owner.directive.directive_id.as_bytes(),
                            &bytes.directive,
                            "directive",
                        )?;
                        require_transaction_value(
                            rules,
                            owner.rule.rule_id.as_bytes(),
                            &bytes.rule,
                            "curation rule",
                        )?;
                        require_transaction_value(
                            belief_configs,
                            owner.belief_config.hash.as_bytes(),
                            &bytes.belief_config,
                            "belief config snapshot",
                        )?;
                    }
                    require_transaction_value(
                        hydrations,
                        key,
                        &hydration_bytes,
                        "process hydration",
                    )?;
                    require_transaction_value(
                        subscriptions,
                        expected.subscription_id.as_bytes(),
                        &subscription_bytes,
                        "hydration subscription",
                    )?;
                    require_transaction_value(
                        checkpoints,
                        key,
                        &expected_bytes,
                        "hydration checkpoint",
                    )?;
                    checkpoints.insert(key, advanced_bytes.as_slice())?;
                    Ok(())
                },
            )
            .map_err(to_agent_transition_error)?;
        self.flush_durable("hydration checkpoint advancement")?;
        Ok(advanced.clone())
    }

    /// Read one durable hydration orchestration checkpoint.
    pub fn hydration_checkpoint(
        &self,
        hydration_id: &str,
    ) -> Result<Option<AgentHydrationCheckpoint>, StorageError> {
        decode_optional(
            self.hydration_checkpoints
                .get(hydration_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Build an exact capability for activating a prepared cross-domain product.
    pub(crate) fn hydration_activation_fence(
        &self,
        owner: &AgentHydrationOwnerFence,
        hydration: &AgentProcessHydrationRecord,
        checkpoint: &AgentHydrationCheckpoint,
    ) -> Result<AgentHydrationFenceCapability, StorageError> {
        validate_hydration_owner_fence(owner)?;
        hydration
            .validate()
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        checkpoint.validate()?;
        if owner.agent.agent_id != hydration.agent_id
            || checkpoint.agent_id != hydration.agent_id
            || checkpoint.hydration_id != hydration.hydration_id
            || checkpoint.attempt_epoch != hydration.attempt_epoch
            || checkpoint.lease_id != hydration.lease_id
            || checkpoint.subscription_id != owner.subscription.subscription_id
        {
            return Err(StorageError::InvalidPath(
                "hydration activation fence has divergent identities".to_string(),
            ));
        }
        let owner_snapshot = hydration_owner_snapshot(owner);
        Ok(AgentHydrationFenceCapability {
            agents: self.agents.clone(),
            receipts: self.bootstrap_receipts.clone(),
            directives: self.directives.clone(),
            rules: self.curation_rules.clone(),
            subscriptions: self.subscriptions.clone(),
            belief_configs: owner.belief_config.tree.clone(),
            hydrations: self.process_hydrations.clone(),
            epochs: self.hydration_epochs.clone(),
            current: self.current_hydrations.clone(),
            checkpoints: self.hydration_checkpoints.clone(),
            snapshot: AgentHydrationFenceSnapshot {
                owner: owner_snapshot,
                hydration: hydration.clone(),
                checkpoint: checkpoint.clone(),
            },
            owner_hash: hydration_owner_fence_hash(owner)?,
            owner_bytes: encode_hydration_owner_fence(owner)?,
            hydration_bytes: serde_json::to_vec(hydration).map_err(to_storage_data)?,
            checkpoint_bytes: serde_json::to_vec(checkpoint).map_err(to_storage_data)?,
            epoch_bytes: encode_epoch(hydration.attempt_epoch),
        })
    }

    /// Persist a sink receipt unless the decision already has one.
    pub fn put_sink_receipt(
        &self,
        receipt: &AgentSinkReceipt,
    ) -> Result<AgentSinkReceipt, StorageError> {
        receipt.validate()?;
        let decision = self.get_decision(&receipt.decision_id)?.ok_or_else(|| {
            StorageError::InvalidPath("sink receipt references an unknown decision".to_string())
        })?;
        let outbox = self.decision_outbox(&receipt.decision_id)?;
        if let Some(outbox) = &outbox {
            outbox.validate_for(&decision)?;
        }
        validate_sink_receipt_for_decision(receipt, &decision, outbox.as_ref())?;
        if let Some(existing) = self.sink_receipt_by_decision(&receipt.decision_id)? {
            if existing != *receipt {
                return Err(StorageError::InvalidPath(
                    "sink receipt decision conflict".to_string(),
                ));
            }
            return Ok(existing);
        }
        let receipt_bytes = serde_json::to_vec(receipt).map_err(to_storage_data)?;
        let command_key =
            sink_receipt_command_key(&receipt.submission.command_id, &receipt.decision_id);
        (&self.sink_receipts, &self.sink_receipts_by_command)
            .transaction(|(receipts, by_command)| {
                insert_exact_transaction_value(
                    receipts,
                    receipt.decision_id.as_bytes(),
                    &receipt_bytes,
                    "sink receipt",
                )?;
                insert_exact_transaction_value(
                    by_command,
                    command_key.as_bytes(),
                    receipt.decision_id.as_bytes(),
                    "sink receipt command index",
                )
            })
            .map_err(to_agent_transition_error)?;
        Ok(receipt.clone())
    }

    /// Read the sink receipt recorded for one curation decision.
    pub fn sink_receipt_by_decision(
        &self,
        decision_id: &str,
    ) -> Result<Option<AgentSinkReceipt>, StorageError> {
        decode_optional(
            self.sink_receipts
                .get(decision_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// List receipts for a command id in deterministic decision order.
    pub fn sink_receipts_by_command(
        &self,
        command_id: &str,
    ) -> Result<Vec<AgentSinkReceipt>, StorageError> {
        let prefix = format!("{command_id}::");
        let mut out = Vec::new();
        for item in self.sink_receipts_by_command.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.map_err(to_storage_io)?;
            let decision_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(receipt) = self.sink_receipt_by_decision(&decision_id)? {
                out.push(receipt);
            }
        }
        out.sort_by(|left, right| left.decision_id.cmp(&right.decision_id));
        Ok(out)
    }

    /// Audit durable state before recurring semantic agent actors are enabled.
    ///
    /// Legacy command decisions without exact outbox payloads fail closed. The
    /// audit never reconstructs commands from current beliefs or goal state.
    pub fn audit_semantic_enablement(&self) -> Result<AgentSemanticEnablementAudit, StorageError> {
        let schema_version = self.runtime_schema_version()?;
        if schema_version != RUNTIME_SCHEMA_VERSION {
            return Err(StorageError::Backpressure(format!(
                "agent runtime schema {schema_version} is not enableable"
            )));
        }
        let mut decision_count = 0usize;
        let mut command_outbox_count = 0usize;
        for item in &self.decisions {
            let (_, value) = item.map_err(to_storage_io)?;
            let decision: AgentCurationDecision =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            decision.validate()?;
            decision_count += 1;
            let outbox = self.decision_outbox(&decision.decision_id)?;
            if matches!(
                decision.decision,
                AgentDecisionKind::GoalCommand | AgentDecisionKind::GoalMutationCommand
            ) {
                let outbox = outbox.ok_or_else(|| {
                    StorageError::MigrationConflict(format!(
                        "command decision '{}' has no exact durable outbox",
                        decision.decision_id
                    ))
                })?;
                outbox.validate_for(&decision)?;
                command_outbox_count += 1;
            } else if outbox.is_some() {
                return Err(StorageError::MigrationConflict(format!(
                    "command-free decision '{}' has an outbox",
                    decision.decision_id
                )));
            }
            if decision.decision == AgentDecisionKind::GoalMutationCommand {
                let review_key = AgentSatisfactionReview {
                    agent_id: decision.agent_id.clone(),
                    subscription_id: decision.subscription_id.clone(),
                    review_seq: decision.created_at_seq,
                }
                .index_key();
                let indexed_decision = self
                    .satisfaction_decisions_by_review
                    .get(review_key.as_bytes())
                    .map_err(to_storage_io)?;
                if indexed_decision.as_deref() != Some(decision.decision_id.as_bytes()) {
                    return Err(StorageError::MigrationConflict(format!(
                        "mutation decision '{}' has no exact satisfaction review identity",
                        decision.decision_id
                    )));
                }
            }
        }
        let mut sink_receipt_count = 0usize;
        for item in &self.sink_receipts {
            let (_, value) = item.map_err(to_storage_io)?;
            let receipt: AgentSinkReceipt =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            receipt.validate()?;
            let decision = self.get_decision(&receipt.decision_id)?.ok_or_else(|| {
                StorageError::MigrationConflict(format!(
                    "sink receipt '{}' references a missing decision",
                    receipt.receipt_id
                ))
            })?;
            let outbox = self.decision_outbox(&receipt.decision_id)?.ok_or_else(|| {
                StorageError::MigrationConflict(format!(
                    "sink receipt '{}' has no exact command outbox",
                    receipt.receipt_id
                ))
            })?;
            outbox.validate_for(&decision)?;
            validate_sink_receipt_for_decision(&receipt, &decision, Some(&outbox)).map_err(
                |error| {
                    StorageError::MigrationConflict(format!(
                        "sink receipt '{}' is invalid: {error}",
                        receipt.receipt_id
                    ))
                },
            )?;
            sink_receipt_count += 1;
        }
        Ok(AgentSemanticEnablementAudit {
            schema_version,
            decision_count,
            command_outbox_count,
            sink_receipt_count,
        })
    }

    /// Flush all sled writes for this store.
    pub fn flush(&self) -> Result<(), StorageError> {
        #[cfg(test)]
        {
            let mut probe = self.flush_probe.lock();
            probe.calls = probe.calls.saturating_add(1);
            if probe.fail_next {
                probe.fail_next = false;
                return Err(StorageError::IoError(io::Error::other(
                    "injected agent store flush failure",
                )));
            }
        }
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    fn flush_durable(&self, product: &str) -> Result<(), StorageError> {
        self.flush().map_err(|error| {
            StorageError::DurabilityIndeterminate(format!("{product} flush failed: {error}"))
        })
    }

    #[cfg(test)]
    fn fail_next_flush(&self) {
        self.flush_probe.lock().fail_next = true;
    }

    #[cfg(test)]
    fn flush_calls(&self) -> usize {
        self.flush_probe.lock().calls
    }

    fn runtime_schema_version(&self) -> Result<u16, StorageError> {
        let Some(raw) = self
            .runtime_schema
            .get(RUNTIME_SCHEMA_VERSION_KEY)
            .map_err(to_storage_io)?
        else {
            return Ok(0);
        };
        let encoded: [u8; 2] = raw.as_ref().try_into().map_err(|_| {
            StorageError::MigrationConflict(
                "agent runtime schema version must contain two bytes".to_string(),
            )
        })?;
        Ok(u16::from_be_bytes(encoded))
    }

    fn migrate_runtime_schema(&self) -> Result<(), StorageError> {
        let version = self.runtime_schema_version()?;
        if version > RUNTIME_SCHEMA_VERSION {
            return Err(StorageError::MigrationConflict(format!(
                "agent runtime schema version {version} is newer than supported version {RUNTIME_SCHEMA_VERSION}"
            )));
        }

        let satisfaction_reviews = self.satisfaction_review_index_map()?;

        // Primary records are authoritative during the additive migration. Every
        // reconstructable index is repaired exactly before the version is enabled.
        for item in &self.decisions {
            let (key, value) = item.map_err(to_storage_io)?;
            let decision: AgentCurationDecision =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            decision.validate()?;
            if key.as_ref() != decision.decision_id.as_bytes() {
                return Err(StorageError::MigrationConflict(format!(
                    "decision primary key diverges for '{}'",
                    decision.decision_id
                )));
            }
            if decision.decision == AgentDecisionKind::GoalMutationCommand
                && !satisfaction_reviews.contains_key(&decision.decision_id)
            {
                return Err(StorageError::MigrationConflict(format!(
                    "mutation decision '{}' has no exact satisfaction review identity",
                    decision.decision_id
                )));
            }
            repair_exact_tree_value(
                &self.decisions_by_agent,
                decision_agent_key(
                    &decision.agent_id,
                    decision.created_at_seq,
                    &decision.decision_id,
                )
                .as_bytes(),
                decision.decision_id.as_bytes(),
                "decision agent index",
            )?;
            repair_exact_tree_value(
                &self.decisions_by_dedupe,
                satisfaction_reviews
                    .get(&decision.decision_id)
                    .map(|review_key| {
                        format!(
                            "{}::satisfaction-review::{review_key}",
                            decision.dedupe_key.index_key()
                        )
                    })
                    .unwrap_or_else(|| {
                        decision_dedupe_revision_key(
                            &decision.dedupe_key,
                            decision.input_refs.belief_revision_id.as_deref(),
                        )
                    })
                    .as_bytes(),
                decision.decision_id.as_bytes(),
                "decision dedupe index",
            )?;
            if let Some(revision_id) = &decision.input_refs.belief_revision_id {
                repair_exact_tree_value(
                    &self.decisions_by_revision,
                    decision_revision_key(revision_id, &decision.decision_id).as_bytes(),
                    decision.decision_id.as_bytes(),
                    "decision revision index",
                )?;
            }
        }
        self.validate_decision_indexes()?;

        for item in &self.sink_receipts {
            let (key, value) = item.map_err(to_storage_io)?;
            let receipt: AgentSinkReceipt =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            receipt.validate()?;
            if key.as_ref() != receipt.decision_id.as_bytes() {
                return Err(StorageError::MigrationConflict(format!(
                    "sink receipt primary key diverges for '{}'",
                    receipt.receipt_id
                )));
            }
            repair_exact_tree_value(
                &self.sink_receipts_by_command,
                sink_receipt_command_key(&receipt.submission.command_id, &receipt.decision_id)
                    .as_bytes(),
                receipt.decision_id.as_bytes(),
                "sink receipt command index",
            )?;
        }
        self.validate_sink_receipt_indexes()?;

        for item in &self.bootstrap_receipts {
            let (key, value) = item.map_err(to_storage_io)?;
            let bootstrap_id = std::str::from_utf8(&key).map_err(|error| {
                StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, error))
            })?;
            let receipt: AgentBootstrapReceipt =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            let progress = self.get_bootstrap_progress(bootstrap_id)?.ok_or_else(|| {
                StorageError::MigrationConflict(format!(
                    "bootstrap receipt '{bootstrap_id}' has no completed progress"
                ))
            })?;
            validate_bootstrap_join(&receipt.agent_id, bootstrap_id, &receipt, &progress)?;
            repair_exact_tree_value(
                &self.bootstrap_receipts_by_agent,
                receipt.agent_id.as_bytes(),
                bootstrap_id.as_bytes(),
                "bootstrap receipt agent index",
            )?;
        }
        self.validate_bootstrap_receipt_agent_index()?;

        for item in &self.process_hydrations {
            let (key, value) = item.map_err(to_storage_io)?;
            let hydration: AgentProcessHydrationRecord =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            hydration.validate().map_err(|error| {
                StorageError::MigrationConflict(format!(
                    "invalid process hydration '{}': {error}",
                    hydration.hydration_id
                ))
            })?;
            if key.as_ref() != hydration.hydration_id.as_bytes() {
                return Err(StorageError::MigrationConflict(format!(
                    "process hydration primary key diverges for '{}'",
                    hydration.hydration_id
                )));
            }
            let (_, activation) = self.require_hydration_pair(&hydration.hydration_id)?;
            validate_hydration_pair(&hydration, &activation)?;
            repair_exact_tree_value(
                &self.process_hydrations_by_agent,
                hydration_agent_key(
                    &hydration.agent_id,
                    hydration.started_at_seq,
                    &hydration.hydration_id,
                )
                .as_bytes(),
                hydration.hydration_id.as_bytes(),
                "process hydration agent index",
            )?;
            repair_exact_tree_value(
                &self.process_hydrations_by_lease,
                hydration_lease_key(
                    &hydration.agent_id,
                    &hydration.lease_id,
                    hydration.attempt_epoch,
                )
                .as_bytes(),
                hydration.hydration_id.as_bytes(),
                "process hydration lease index",
            )?;
            self.process_hydrations_by_lease
                .remove(hydration_legacy_lease_key(
                    &hydration.agent_id,
                    &hydration.lease_id,
                ))
                .map_err(to_storage_io)?;
        }
        self.validate_hydration_lease_index()?;

        repair_exact_tree_value(
            &self.runtime_schema,
            RUNTIME_SCHEMA_VERSION_KEY,
            &RUNTIME_SCHEMA_VERSION.to_be_bytes(),
            "agent runtime schema version",
        )?;
        self.flush_durable("agent runtime schema migration")
    }

    fn migrate_legacy_readiness_proofs(&self) -> Result<(), StorageError> {
        let mut state = match self
            .readiness_schema
            .get(KEY_READINESS_SCHEMA_STATE)
            .map_err(to_storage_io)?
        {
            Some(raw) => serde_json::from_slice(&raw).map_err(to_storage_data)?,
            None => {
                let initial = if self.readiness_proofs.is_empty() {
                    AgentReadinessMigrationState::complete()
                } else {
                    AgentReadinessMigrationState::initial()
                };
                self.put_agent_readiness_migration_state(&initial)?;
                initial
            }
        };
        state.validate()?;
        if state.complete {
            return Ok(());
        }
        let belief = BeliefReadinessReopenContract::open(self.db.clone())?;
        while !state.complete {
            let mut keys = Vec::with_capacity(READINESS_MIGRATION_BATCH);
            match state.cursor.as_deref() {
                Some(cursor) => {
                    for item in self
                        .readiness_proofs
                        .range::<&[u8], _>((Bound::Excluded(cursor), Bound::Unbounded))
                        .take(READINESS_MIGRATION_BATCH)
                    {
                        let (key, _) = item.map_err(to_storage_io)?;
                        keys.push(key.to_vec());
                    }
                }
                None => {
                    for item in self.readiness_proofs.iter().take(READINESS_MIGRATION_BATCH) {
                        let (key, _) = item.map_err(to_storage_io)?;
                        keys.push(key.to_vec());
                    }
                }
            }
            if keys.is_empty() {
                state.complete = true;
                state.cursor = None;
            } else {
                for key in &keys {
                    self.migrate_readiness_proof(key, &belief)?;
                }
                state.cursor = keys.last().cloned();
            }
            self.put_agent_readiness_migration_state(&state)?;
        }
        Ok(())
    }

    fn migrate_readiness_proof(
        &self,
        key: &[u8],
        belief: &BeliefReadinessReopenContract,
    ) -> Result<(), StorageError> {
        let raw = self
            .readiness_proofs
            .get(key)
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::MigrationConflict(
                    "readiness proof disappeared during schema migration".to_string(),
                )
            })?;
        let proof: AgentReadinessProof = serde_json::from_slice(&raw).map_err(to_storage_data)?;
        proof.validate().map_err(|error| {
            StorageError::MigrationConflict(format!(
                "invalid readiness proof '{}': {error}",
                proof.proof_id
            ))
        })?;
        if key != proof.proof_id.as_bytes() {
            return Err(StorageError::MigrationConflict(format!(
                "readiness proof primary key diverges for '{}'",
                proof.proof_id
            )));
        }
        let attestation = belief.verified_attestation(&proof.signal.attestation.attestation_id)?;
        if !proof.requires_legacy_upgrade() {
            if proof.signal.attestation != attestation {
                return Err(StorageError::MigrationConflict(format!(
                    "readiness proof '{}' diverges from belief authority",
                    proof.proof_id
                )));
            }
            return Ok(());
        }
        // TODO compat-shim: remove the v1 proof rewrite after every supported
        // store carries schema v2 proofs and accepted W3A fixture, reopen, and
        // hash-preimage parity tests remain green without this branch.
        let proof_id = proof.proof_id.clone();
        let upgraded = proof.upgrade_legacy(attestation).map_err(|error| {
            StorageError::MigrationConflict(format!(
                "legacy readiness proof '{}' cannot be upgraded: {error}",
                proof_id
            ))
        })?;
        let upgraded_bytes = serde_json::to_vec(&upgraded).map_err(to_storage_data)?;
        self.readiness_proofs
            .transaction(|proofs| {
                require_transaction_value(proofs, key, raw.as_ref(), "legacy readiness proof")?;
                proofs.insert(key, upgraded_bytes.as_slice())?;
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::MigrationConflict(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })
    }

    fn put_agent_readiness_migration_state(
        &self,
        state: &AgentReadinessMigrationState,
    ) -> Result<(), StorageError> {
        state.validate()?;
        self.readiness_schema
            .insert(
                KEY_READINESS_SCHEMA_STATE,
                serde_json::to_vec(state).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.flush_durable("agent readiness schema migration")
    }

    fn satisfaction_review_index_map(&self) -> Result<BTreeMap<String, String>, StorageError> {
        let mut by_decision = BTreeMap::new();
        for item in &self.satisfaction_decisions_by_review {
            let (key, value) = item.map_err(to_storage_io)?;
            let review_key = String::from_utf8(key.to_vec()).map_err(to_storage_utf8)?;
            let decision_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(existing) = by_decision.insert(decision_id.clone(), review_key.clone()) {
                if existing != review_key {
                    return Err(StorageError::MigrationConflict(format!(
                        "decision '{decision_id}' has divergent satisfaction review indexes"
                    )));
                }
            }
        }
        Ok(by_decision)
    }

    fn validate_bootstrap_receipt_agent_index(&self) -> Result<(), StorageError> {
        for item in &self.bootstrap_receipts_by_agent {
            let (agent_id, bootstrap_id) = item.map_err(to_storage_io)?;
            let agent_id = std::str::from_utf8(&agent_id).map_err(|error| {
                StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, error))
            })?;
            let bootstrap_id = std::str::from_utf8(&bootstrap_id).map_err(|error| {
                StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, error))
            })?;
            let receipt = self.get_bootstrap_receipt(bootstrap_id)?.ok_or_else(|| {
                StorageError::MigrationConflict(format!(
                    "bootstrap receipt agent index references missing receipt '{bootstrap_id}'"
                ))
            })?;
            let progress = self.get_bootstrap_progress(bootstrap_id)?.ok_or_else(|| {
                StorageError::MigrationConflict(format!(
                    "bootstrap receipt '{bootstrap_id}' has no completed progress"
                ))
            })?;
            validate_bootstrap_join(agent_id, bootstrap_id, &receipt, &progress)?;
        }
        Ok(())
    }

    fn validate_hydration_lease_index(&self) -> Result<(), StorageError> {
        for item in &self.process_hydrations_by_lease {
            let (key, hydration_id) = item.map_err(to_storage_io)?;
            let hydration_id = std::str::from_utf8(&hydration_id).map_err(|error| {
                StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, error))
            })?;
            let (hydration, _) = self.require_hydration_pair(hydration_id)?;
            if key.as_ref()
                != hydration_lease_key(
                    &hydration.agent_id,
                    &hydration.lease_id,
                    hydration.attempt_epoch,
                )
                .as_bytes()
            {
                return Err(StorageError::MigrationConflict(format!(
                    "process hydration lease index diverges for '{hydration_id}'"
                )));
            }
        }
        Ok(())
    }

    fn validate_decision_indexes(&self) -> Result<(), StorageError> {
        let satisfaction_reviews = self.satisfaction_review_index_map()?;
        for item in &self.decisions_by_agent {
            let (key, value) = item.map_err(to_storage_io)?;
            let decision_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            let decision = self.get_decision(&decision_id)?.ok_or_else(|| {
                StorageError::MigrationConflict(format!(
                    "decision agent index references missing decision '{decision_id}'"
                ))
            })?;
            let expected = decision_agent_key(
                &decision.agent_id,
                decision.created_at_seq,
                &decision.decision_id,
            );
            if key.as_ref() != expected.as_bytes() {
                return Err(StorageError::MigrationConflict(format!(
                    "decision agent index diverges for '{decision_id}'"
                )));
            }
        }
        for item in &self.decisions_by_dedupe {
            let (key, value) = item.map_err(to_storage_io)?;
            let decision_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            let decision = self.get_decision(&decision_id)?.ok_or_else(|| {
                StorageError::MigrationConflict(format!(
                    "decision dedupe index references missing decision '{decision_id}'"
                ))
            })?;
            let expected = satisfaction_reviews
                .get(&decision_id)
                .map(|review_key| {
                    format!(
                        "{}::satisfaction-review::{review_key}",
                        decision.dedupe_key.index_key()
                    )
                })
                .unwrap_or_else(|| {
                    decision_dedupe_revision_key(
                        &decision.dedupe_key,
                        decision.input_refs.belief_revision_id.as_deref(),
                    )
                });
            if key.as_ref() != expected.as_bytes() {
                return Err(StorageError::MigrationConflict(format!(
                    "decision dedupe index diverges for '{decision_id}'"
                )));
            }
        }
        for item in &self.decisions_by_revision {
            let (key, value) = item.map_err(to_storage_io)?;
            let decision_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            let decision = self.get_decision(&decision_id)?.ok_or_else(|| {
                StorageError::MigrationConflict(format!(
                    "decision revision index references missing decision '{decision_id}'"
                ))
            })?;
            let revision_id = decision
                .input_refs
                .belief_revision_id
                .as_deref()
                .ok_or_else(|| {
                    StorageError::MigrationConflict(format!(
                        "decision revision index references revision-free decision '{decision_id}'"
                    ))
                })?;
            let expected = decision_revision_key(revision_id, &decision_id);
            if key.as_ref() != expected.as_bytes() {
                return Err(StorageError::MigrationConflict(format!(
                    "decision revision index diverges for '{decision_id}'"
                )));
            }
        }
        for item in &self.satisfaction_decisions_by_review {
            let (key, value) = item.map_err(to_storage_io)?;
            let decision_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            let decision = self.get_decision(&decision_id)?.ok_or_else(|| {
                StorageError::MigrationConflict(format!(
                    "satisfaction review index references missing decision '{decision_id}'"
                ))
            })?;
            if decision.decision == AgentDecisionKind::GoalCommand {
                return Err(StorageError::MigrationConflict(format!(
                    "satisfaction review index references goal decision '{decision_id}'"
                )));
            }
            let expected = AgentSatisfactionReview {
                agent_id: decision.agent_id,
                subscription_id: decision.subscription_id,
                review_seq: decision.created_at_seq,
            }
            .index_key();
            if key.as_ref() != expected.as_bytes() {
                return Err(StorageError::MigrationConflict(format!(
                    "satisfaction review index diverges for '{decision_id}'"
                )));
            }
        }
        Ok(())
    }

    fn validate_sink_receipt_indexes(&self) -> Result<(), StorageError> {
        for item in &self.sink_receipts_by_command {
            let (key, value) = item.map_err(to_storage_io)?;
            let decision_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            let receipt = self
                .sink_receipt_by_decision(&decision_id)?
                .ok_or_else(|| {
                    StorageError::MigrationConflict(format!(
                        "sink receipt command index references missing decision '{decision_id}'"
                    ))
                })?;
            let expected = sink_receipt_command_key(&receipt.submission.command_id, &decision_id);
            if key.as_ref() != expected.as_bytes() {
                return Err(StorageError::MigrationConflict(format!(
                    "sink receipt command index diverges for '{decision_id}'"
                )));
            }
        }
        Ok(())
    }

    fn require_selected_agent(
        &self,
        agent_id: &str,
        expected_updated_at_seq: u64,
    ) -> Result<AgentRecord, StorageError> {
        let agent = self
            .get_agent(agent_id)?
            .ok_or_else(|| StorageError::Backpressure("selected agent disappeared".to_string()))?;
        if agent.status != AgentStatus::Operational
            || agent.updated_at_seq != expected_updated_at_seq
        {
            return Err(StorageError::Backpressure(
                "selected agent lifecycle changed".to_string(),
            ));
        }
        Ok(agent)
    }

    fn require_selected_subscription(
        &self,
        subscription_id: &str,
        agent_id: &str,
        belief_key: &crate::belief::BeliefKey,
        expected_updated_at_seq: u64,
    ) -> Result<AgentSubscriptionRecord, StorageError> {
        let subscription = self.get_subscription(subscription_id)?.ok_or_else(|| {
            StorageError::Backpressure("selected subscription disappeared".to_string())
        })?;
        if subscription.agent_id != agent_id
            || subscription.status != AgentSubscriptionStatus::Active
            || subscription.belief_key != *belief_key
            || subscription.updated_at_seq != expected_updated_at_seq
        {
            return Err(StorageError::Backpressure(
                "selected subscription changed".to_string(),
            ));
        }
        Ok(subscription)
    }

    fn persist_outcome_transaction(
        &self,
        outcome: &AgentCurationOutcome,
        outbox: Option<&AgentDecisionOutboxRecord>,
        review: Option<&AgentSatisfactionReview>,
    ) -> Result<(), StorageError> {
        outcome.decision.validate()?;
        if let Some(outbox) = outbox {
            outbox.validate_for(&outcome.decision)?;
        }
        if let Some(review) = review {
            review.validate()?;
        }
        let data = DecisionPersistenceData::new(outcome, outbox, review)?;
        (
            &self.decisions,
            &self.decisions_by_agent,
            &self.decisions_by_dedupe,
            &self.decisions_by_revision,
            &self.satisfaction_decisions_by_review,
            &self.decision_outbox,
        )
            .transaction(
                |(decisions, by_agent, by_dedupe, by_revision, by_review, outboxes)| {
                    persist_decision_values(
                        decisions,
                        by_agent,
                        by_dedupe,
                        by_revision,
                        by_review,
                        outboxes,
                        &data,
                    )
                },
            )
            .map_err(to_agent_transition_error)
    }

    fn persist_outcome_with_owner_fences(
        &self,
        outcome: &AgentCurationOutcome,
        outbox: Option<&AgentDecisionOutboxRecord>,
        agent: &AgentRecord,
        subscription: &AgentSubscriptionRecord,
        review: Option<&AgentSatisfactionReview>,
        cursor_fence: Option<(
            &AgentSatisfactionCursorIdentity,
            Option<&AgentSatisfactionReviewCursor>,
        )>,
    ) -> Result<(), StorageError> {
        outcome.decision.validate()?;
        if let Some(outbox) = outbox {
            outbox.validate_for(&outcome.decision)?;
        }
        let data = DecisionPersistenceData::new(outcome, outbox, review)?;
        let agent_bytes = serde_json::to_vec(agent).map_err(to_storage_data)?;
        let subscription_bytes = serde_json::to_vec(subscription).map_err(to_storage_data)?;
        let cursor_key = cursor_fence.map(|fence| fence.0.index_key());
        let cursor_bytes = cursor_fence
            .and_then(|fence| fence.1)
            .map(serde_json::to_vec)
            .transpose()
            .map_err(to_storage_data)?;
        (
            &self.agents,
            &self.subscriptions,
            &self.satisfaction_review_cursors,
            &self.decisions,
            &self.decisions_by_agent,
            &self.decisions_by_dedupe,
            &self.decisions_by_revision,
            &self.satisfaction_decisions_by_review,
            &self.decision_outbox,
        )
            .transaction(
                |(
                    agents,
                    subscriptions,
                    cursors,
                    decisions,
                    by_agent,
                    by_dedupe,
                    by_revision,
                    by_review,
                    outboxes,
                )| {
                    require_transaction_value(
                        agents,
                        agent.agent_id.as_bytes(),
                        &agent_bytes,
                        "selected agent",
                    )?;
                    require_transaction_value(
                        subscriptions,
                        subscription.subscription_id.as_bytes(),
                        &subscription_bytes,
                        "selected subscription",
                    )?;
                    if let Some(cursor_key) = &cursor_key {
                        require_optional_transaction_value(
                            cursors,
                            cursor_key.as_bytes(),
                            cursor_bytes.as_deref(),
                            "satisfaction cursor",
                        )?;
                    }
                    persist_decision_values(
                        decisions,
                        by_agent,
                        by_dedupe,
                        by_revision,
                        by_review,
                        outboxes,
                        &data,
                    )
                },
            )
            .map_err(to_agent_transition_error)
    }

    fn insert_legacy_decision_transaction(
        &self,
        decision: &AgentCurationDecision,
        dedupe_index_key: &str,
        review: Option<&AgentSatisfactionReview>,
    ) -> Result<AgentCurationDecision, StorageError> {
        let data = DecisionPersistenceData::legacy(decision, dedupe_index_key, review)?;
        (
            &self.decisions,
            &self.decisions_by_agent,
            &self.decisions_by_dedupe,
            &self.decisions_by_revision,
            &self.satisfaction_decisions_by_review,
            &self.decision_outbox,
        )
            .transaction(
                |(decisions, by_agent, by_dedupe, by_revision, by_review, outboxes)| {
                    persist_decision_values(
                        decisions,
                        by_agent,
                        by_dedupe,
                        by_revision,
                        by_review,
                        outboxes,
                        &data,
                    )
                },
            )
            .map_err(to_agent_transition_error)?;
        Ok(decision.clone())
    }

    fn current_hydration_epoch(&self, agent_id: &str) -> Result<u64, StorageError> {
        let Some(raw) = self
            .hydration_epochs
            .get(agent_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(0);
        };
        decode_epoch(&raw)
    }

    fn current_hydration_id(&self, agent_id: &str) -> Result<Option<String>, StorageError> {
        self.current_hydrations
            .get(agent_id.as_bytes())
            .map_err(to_storage_io)?
            .map(|raw| String::from_utf8(raw.to_vec()).map_err(to_storage_utf8))
            .transpose()
    }

    fn require_hydration_pair(
        &self,
        hydration_id: &str,
    ) -> Result<(AgentProcessHydrationRecord, AgentActivationRecord), StorageError> {
        let hydration = self.get_process_hydration(hydration_id)?.ok_or_else(|| {
            StorageError::InvalidPath(format!("unknown process hydration '{hydration_id}'"))
        })?;
        let activation = self.get_activation(hydration_id)?.ok_or_else(|| {
            StorageError::InvalidPath(format!("activation diagnostic '{hydration_id}' is missing"))
        })?;
        if hydration.hydration_id != hydration_id || activation.activation_id != hydration_id {
            return Err(StorageError::MigrationConflict(format!(
                "hydration primary identity diverges for '{hydration_id}'"
            )));
        }
        validate_hydration_pair(&hydration, &activation)?;
        Ok((hydration, activation))
    }

    fn verify_hydration_replay_fences(
        &self,
        agent: &AgentRecord,
        hydration: &AgentProcessHydrationRecord,
        activation: &AgentActivationRecord,
        owner: Option<&AgentHydrationOwnerFence>,
    ) -> Result<(), StorageError> {
        let agent_bytes = serde_json::to_vec(agent).map_err(to_storage_data)?;
        let hydration_bytes = serde_json::to_vec(hydration).map_err(to_storage_data)?;
        let activation_bytes = serde_json::to_vec(activation).map_err(to_storage_data)?;
        let epoch_bytes = encode_epoch(hydration.attempt_epoch);
        let lease_key = hydration_lease_key(
            &hydration.agent_id,
            &hydration.lease_id,
            hydration.attempt_epoch,
        );
        let owner_bytes = owner.map(encode_hydration_owner_fence).transpose()?;
        let belief_config_tree = owner
            .map(|owner| &owner.belief_config.tree)
            .unwrap_or(&self.runtime_schema);
        (
            &self.agents,
            &self.bootstrap_receipts,
            &self.directives,
            &self.curation_rules,
            &self.subscriptions,
            belief_config_tree,
            &self.hydration_epochs,
            &self.current_hydrations,
            &self.process_hydrations,
            &self.process_hydrations_by_lease,
            &self.activations,
        )
            .transaction(
                |(
                    agents,
                    receipts,
                    directives,
                    rules,
                    subscriptions,
                    belief_configs,
                    epochs,
                    current,
                    hydrations,
                    leases,
                    activations,
                )| {
                    require_transaction_value(
                        agents,
                        agent.agent_id.as_bytes(),
                        &agent_bytes,
                        "agent",
                    )?;
                    if let (Some(owner), Some(bytes)) = (owner, owner_bytes.as_ref()) {
                        require_transaction_value(
                            receipts,
                            owner.receipt.bootstrap_id.as_bytes(),
                            &bytes.receipt,
                            "bootstrap receipt",
                        )?;
                        require_transaction_value(
                            directives,
                            owner.directive.directive_id.as_bytes(),
                            &bytes.directive,
                            "directive",
                        )?;
                        require_transaction_value(
                            rules,
                            owner.rule.rule_id.as_bytes(),
                            &bytes.rule,
                            "curation rule",
                        )?;
                        require_transaction_value(
                            subscriptions,
                            owner.subscription.subscription_id.as_bytes(),
                            &bytes.subscription,
                            "subscription",
                        )?;
                        require_transaction_value(
                            belief_configs,
                            owner.belief_config.hash.as_bytes(),
                            &bytes.belief_config,
                            "belief config snapshot",
                        )?;
                    }
                    require_transaction_value(
                        epochs,
                        agent.agent_id.as_bytes(),
                        &epoch_bytes,
                        "hydration epoch",
                    )?;
                    require_transaction_value(
                        current,
                        agent.agent_id.as_bytes(),
                        hydration.hydration_id.as_bytes(),
                        "current hydration",
                    )?;
                    require_transaction_value(
                        hydrations,
                        hydration.hydration_id.as_bytes(),
                        &hydration_bytes,
                        "hydration",
                    )?;
                    require_transaction_value(
                        leases,
                        lease_key.as_bytes(),
                        hydration.hydration_id.as_bytes(),
                        "hydration lease",
                    )?;
                    require_transaction_value(
                        activations,
                        activation.activation_id.as_bytes(),
                        &activation_bytes,
                        "activation",
                    )?;
                    Ok(())
                },
            )
            .map_err(to_agent_transition_error)
    }

    fn verify_hydration_owner_checkpoint_fence(
        &self,
        owner: &AgentHydrationOwnerFence,
        checkpoint: &AgentHydrationCheckpoint,
    ) -> Result<(), StorageError> {
        let owner_bytes = encode_hydration_owner_fence(owner)?;
        let checkpoint_bytes = serde_json::to_vec(checkpoint).map_err(to_storage_data)?;
        (
            &self.agents,
            &self.bootstrap_receipts,
            &self.directives,
            &self.curation_rules,
            &self.subscriptions,
            &owner.belief_config.tree,
            &self.hydration_checkpoints,
        )
            .transaction(
                |(
                    agents,
                    receipts,
                    directives,
                    rules,
                    subscriptions,
                    belief_configs,
                    checkpoints,
                )| {
                    require_transaction_value(
                        agents,
                        owner.agent.agent_id.as_bytes(),
                        &owner_bytes.agent,
                        "agent",
                    )?;
                    require_transaction_value(
                        receipts,
                        owner.receipt.bootstrap_id.as_bytes(),
                        &owner_bytes.receipt,
                        "bootstrap receipt",
                    )?;
                    require_transaction_value(
                        directives,
                        owner.directive.directive_id.as_bytes(),
                        &owner_bytes.directive,
                        "directive",
                    )?;
                    require_transaction_value(
                        rules,
                        owner.rule.rule_id.as_bytes(),
                        &owner_bytes.rule,
                        "curation rule",
                    )?;
                    require_transaction_value(
                        subscriptions,
                        owner.subscription.subscription_id.as_bytes(),
                        &owner_bytes.subscription,
                        "subscription",
                    )?;
                    require_transaction_value(
                        belief_configs,
                        owner.belief_config.hash.as_bytes(),
                        &owner_bytes.belief_config,
                        "belief config snapshot",
                    )?;
                    require_transaction_value(
                        checkpoints,
                        checkpoint.hydration_id.as_bytes(),
                        &checkpoint_bytes,
                        "hydration checkpoint",
                    )?;
                    Ok(())
                },
            )
            .map_err(to_agent_transition_error)
    }

    fn insert_decision_record(
        &self,
        decision: &AgentCurationDecision,
        dedupe_index_key: &str,
    ) -> Result<AgentCurationDecision, StorageError> {
        self.insert_legacy_decision_transaction(decision, dedupe_index_key, None)
    }
}

struct DecisionPersistenceData {
    decision_id: Vec<u8>,
    decision: Vec<u8>,
    agent_index_key: Vec<u8>,
    dedupe_index_key: Vec<u8>,
    revision_index_key: Option<Vec<u8>>,
    review_index_key: Option<Vec<u8>>,
    outbox: Option<Vec<u8>>,
}

impl DecisionPersistenceData {
    fn new(
        outcome: &AgentCurationOutcome,
        outbox: Option<&AgentDecisionOutboxRecord>,
        review: Option<&AgentSatisfactionReview>,
    ) -> Result<Self, StorageError> {
        let dedupe_index_key = match review {
            Some(review) => {
                decision_dedupe_satisfaction_review_key(&outcome.decision.dedupe_key, review)
            }
            None => decision_dedupe_revision_key(
                &outcome.decision.dedupe_key,
                outcome.decision.input_refs.belief_revision_id.as_deref(),
            ),
        };
        Self::build(&outcome.decision, &dedupe_index_key, review, outbox)
    }

    fn legacy(
        decision: &AgentCurationDecision,
        dedupe_index_key: &str,
        review: Option<&AgentSatisfactionReview>,
    ) -> Result<Self, StorageError> {
        Self::build(decision, dedupe_index_key, review, None)
    }

    fn build(
        decision: &AgentCurationDecision,
        dedupe_index_key: &str,
        review: Option<&AgentSatisfactionReview>,
        outbox: Option<&AgentDecisionOutboxRecord>,
    ) -> Result<Self, StorageError> {
        decision.validate()?;
        if let Some(review) = review {
            review.validate()?;
        }
        Ok(Self {
            decision_id: decision.decision_id.as_bytes().to_vec(),
            decision: serde_json::to_vec(decision).map_err(to_storage_data)?,
            agent_index_key: decision_agent_key(
                &decision.agent_id,
                decision.created_at_seq,
                &decision.decision_id,
            )
            .into_bytes(),
            dedupe_index_key: dedupe_index_key.as_bytes().to_vec(),
            revision_index_key: decision
                .input_refs
                .belief_revision_id
                .as_deref()
                .map(|revision_id| decision_revision_key(revision_id, &decision.decision_id))
                .map(String::into_bytes),
            review_index_key: review
                .map(AgentSatisfactionReview::index_key)
                .map(String::into_bytes),
            outbox: outbox
                .map(serde_json::to_vec)
                .transpose()
                .map_err(to_storage_data)?,
        })
    }
}

fn persist_decision_values(
    decisions: &sled::transaction::TransactionalTree,
    by_agent: &sled::transaction::TransactionalTree,
    by_dedupe: &sled::transaction::TransactionalTree,
    by_revision: &sled::transaction::TransactionalTree,
    by_review: &sled::transaction::TransactionalTree,
    outboxes: &sled::transaction::TransactionalTree,
    data: &DecisionPersistenceData,
) -> Result<(), ConflictableTransactionError<String>> {
    insert_exact_transaction_value(
        decisions,
        &data.decision_id,
        &data.decision,
        "agent decision",
    )?;
    insert_exact_transaction_value(
        by_agent,
        &data.agent_index_key,
        &data.decision_id,
        "decision agent index",
    )?;
    insert_exact_transaction_value(
        by_dedupe,
        &data.dedupe_index_key,
        &data.decision_id,
        "decision dedupe index",
    )?;
    if let Some(key) = &data.revision_index_key {
        insert_exact_transaction_value(
            by_revision,
            key,
            &data.decision_id,
            "decision revision index",
        )?;
    }
    if let Some(key) = &data.review_index_key {
        insert_exact_transaction_value(
            by_review,
            key,
            &data.decision_id,
            "satisfaction review index",
        )?;
    }
    match &data.outbox {
        Some(outbox) => {
            insert_exact_transaction_value(outboxes, &data.decision_id, outbox, "decision outbox")
        }
        None => {
            require_optional_transaction_value(outboxes, &data.decision_id, None, "decision outbox")
        }
    }
}

fn validate_delivery_outcome(
    selection: &AgentDeliverySelection,
    outcome: &AgentCurationOutcome,
) -> Result<(), StorageError> {
    let decision = &outcome.decision;
    if matches!(decision.decision, AgentDecisionKind::GoalMutationCommand)
        || decision.agent_id != selection.delivery.agent_id
        || decision.subscription_id != selection.delivery.subscription_id
        || decision.input_refs.belief_key != selection.belief_key
        || decision.input_refs.belief_revision_id.as_deref()
            != Some(selection.delivery.belief_revision_id.as_str())
        || decision.created_at_seq != selection.delivery.revision_seq
    {
        return Err(StorageError::InvalidPath(
            "selected delivery and curation outcome disagree".to_string(),
        ));
    }
    Ok(())
}

fn validate_satisfaction_outcome(
    selection: &AgentSatisfactionReviewSelection,
    outcome: &AgentCurationOutcome,
) -> Result<(), StorageError> {
    let decision = &outcome.decision;
    if matches!(decision.decision, AgentDecisionKind::GoalCommand)
        || decision.agent_id != selection.review.agent_id
        || decision.subscription_id != selection.review.subscription_id
        || decision.input_refs.belief_key != selection.belief_key
        || decision.input_refs.belief_revision_id.as_deref()
            != Some(selection.belief_revision_id.as_str())
        || decision.created_at_seq != selection.belief_revision_seq
    {
        return Err(StorageError::InvalidPath(
            "selected satisfaction review and curation outcome disagree".to_string(),
        ));
    }
    Ok(())
}

fn validate_sink_receipt_for_decision(
    receipt: &AgentSinkReceipt,
    decision: &AgentCurationDecision,
    outbox: Option<&AgentDecisionOutboxRecord>,
) -> Result<(), StorageError> {
    let (expected_command_id, expected_kind) = match &decision.decision {
        AgentDecisionKind::GoalCommand => (
            decision.goal_command_id.as_deref(),
            AgentSinkReceiptKind::GoalCommand,
        ),
        AgentDecisionKind::GoalMutationCommand => (
            decision.goal_mutation_command_id.as_deref(),
            AgentSinkReceiptKind::GoalMutationCommand,
        ),
        AgentDecisionKind::Absorbed | AgentDecisionKind::Indeterminate => {
            return Err(StorageError::InvalidPath(
                "sink receipt decision did not emit a command".to_string(),
            ));
        }
    };
    let expected_command_id = expected_command_id.ok_or_else(|| {
        StorageError::InvalidPath("sink receipt decision did not emit a command".to_string())
    })?;
    if receipt.kind != expected_kind
        || receipt.recorded_at_seq != decision.created_at_seq
        || receipt.submission.command_id != expected_command_id
    {
        return Err(StorageError::InvalidPath(
            "sink receipt kind, sequence, or command does not match its decision".to_string(),
        ));
    }
    if let Some(outbox) = outbox {
        if outbox.command.command_id() != receipt.submission.command_id
            || outbox.command.goal_id() != receipt.submission.goal_id
        {
            return Err(StorageError::InvalidPath(
                "sink receipt does not match the durable command outbox".to_string(),
            ));
        }
    }
    Ok(())
}

fn outcome_from_durable(
    decision: AgentCurationDecision,
    outbox: Option<AgentDecisionOutboxRecord>,
) -> Result<AgentCurationOutcome, StorageError> {
    decision.validate()?;
    if let Some(outbox) = &outbox {
        outbox.validate_for(&decision)?;
    }
    let (goal_command, goal_mutation_command) = match (&decision.decision, outbox) {
        (AgentDecisionKind::GoalCommand, Some(outbox)) => match outbox.command {
            AgentAuthoredCommand::Goal(command) => (Some(*command), None),
            AgentAuthoredCommand::GoalMutation(_) => {
                return Err(StorageError::MigrationConflict(format!(
                    "decision '{}' has the wrong command outbox kind",
                    decision.decision_id
                )));
            }
        },
        (AgentDecisionKind::GoalMutationCommand, Some(outbox)) => match outbox.command {
            AgentAuthoredCommand::GoalMutation(command) => (None, Some(*command)),
            AgentAuthoredCommand::Goal(_) => {
                return Err(StorageError::MigrationConflict(format!(
                    "decision '{}' has the wrong command outbox kind",
                    decision.decision_id
                )));
            }
        },
        (AgentDecisionKind::GoalCommand | AgentDecisionKind::GoalMutationCommand, None) => {
            return Err(StorageError::MigrationConflict(format!(
                "command decision '{}' has no exact durable outbox",
                decision.decision_id
            )));
        }
        (AgentDecisionKind::Absorbed | AgentDecisionKind::Indeterminate, Some(_)) => {
            return Err(StorageError::MigrationConflict(format!(
                "command-free decision '{}' has a durable outbox",
                decision.decision_id
            )));
        }
        (AgentDecisionKind::Absorbed | AgentDecisionKind::Indeterminate, None) => (None, None),
    };
    Ok(AgentCurationOutcome {
        decision,
        goal_command,
        goal_mutation_command,
    })
}

fn agent_status_key(status: &str, seq: u64, agent_id: &str) -> String {
    format!("{status}::{seq:0KEY_PAD$}::{agent_id}")
}

fn subscription_agent_key(agent_id: &str, seq: u64, subscription_id: &str) -> String {
    format!("{agent_id}::{seq:0KEY_PAD$}::{subscription_id}")
}

fn activation_agent_key(agent_id: &str, seq: u64, activation_id: &str) -> String {
    format!("{agent_id}::{seq:0KEY_PAD$}::{activation_id}")
}

fn hydration_agent_key(agent_id: &str, seq: u64, hydration_id: &str) -> String {
    format!("{agent_id}::{seq:0KEY_PAD$}::{hydration_id}")
}

fn hydration_lease_prefix(agent_id: &str, lease_id: &str) -> String {
    format!(
        "{:0KEY_PAD$}{agent_id}{:0KEY_PAD$}{lease_id}",
        agent_id.len(),
        lease_id.len()
    )
}

fn hydration_lease_key(agent_id: &str, lease_id: &str, attempt_epoch: u64) -> String {
    format!(
        "{}::{attempt_epoch:0KEY_PAD$}",
        hydration_lease_prefix(agent_id, lease_id)
    )
}

fn hydration_legacy_lease_key(agent_id: &str, lease_id: &str) -> String {
    format!("{:0KEY_PAD$}{agent_id}{lease_id}", agent_id.len())
}

fn validate_hydration_continuation(
    continuation: &AgentHydrationContinuation,
) -> Result<(), StorageError> {
    let prefix = format!("{}::", AgentStatus::Registered.index_key());
    if continuation.generation == 0 || !continuation.last_status_key.starts_with(&prefix) {
        return Err(StorageError::InvalidPath(
            "agent hydration continuation is malformed".to_string(),
        ));
    }
    Ok(())
}

fn validate_hydration_owner_fence(owner: &AgentHydrationOwnerFence) -> Result<(), StorageError> {
    owner.agent.validate()?;
    owner.rule.config.validate()?;
    owner.subscription.validate()?;
    if owner.receipt.agent_id != owner.agent.agent_id
        || owner.receipt.directive_id != owner.agent.directive_id
        || owner.directive.directive_id != owner.receipt.directive_id
        || owner.receipt.rule_id != owner.rule.rule_id
        || owner.rule.agent_id != owner.agent.agent_id
        || owner.receipt.subscription_id != owner.subscription.subscription_id
        || owner.subscription.agent_id != owner.agent.agent_id
        || owner.belief_config.hash != owner.receipt.belief.config_snapshot_hash
        || owner.belief_config.json.trim().is_empty()
    {
        return Err(StorageError::InvalidPath(
            "hydration owner fence has divergent identities".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn hydration_owner_fence_hash(
    owner: &AgentHydrationOwnerFence,
) -> Result<String, StorageError> {
    hydration_owner_snapshot(owner).fence_hash()
}

fn hydration_owner_snapshot(owner: &AgentHydrationOwnerFence) -> AgentHydrationOwnerSnapshot {
    AgentHydrationOwnerSnapshot {
        agent: owner.agent.clone(),
        receipt: owner.receipt.clone(),
        directive: owner.directive.clone(),
        rule: owner.rule.clone(),
        subscription: owner.subscription.clone(),
        belief_config_hash: owner.belief_config.hash.clone(),
        belief_config_json: owner.belief_config.json.clone(),
    }
}

fn encode_hydration_owner_fence(
    owner: &AgentHydrationOwnerFence,
) -> Result<AgentHydrationOwnerFenceBytes, StorageError> {
    Ok(AgentHydrationOwnerFenceBytes {
        agent: serde_json::to_vec(&owner.agent).map_err(to_storage_data)?,
        receipt: serde_json::to_vec(&owner.receipt).map_err(to_storage_data)?,
        directive: serde_json::to_vec(&owner.directive).map_err(to_storage_data)?,
        rule: serde_json::to_vec(&owner.rule).map_err(to_storage_data)?,
        subscription: serde_json::to_vec(&owner.subscription).map_err(to_storage_data)?,
        belief_config: owner.belief_config.json.as_bytes().to_vec(),
    })
}

fn validate_bootstrap_join(
    agent_id: &str,
    bootstrap_id: &str,
    receipt: &AgentBootstrapReceipt,
    progress: &AgentBootstrapProgress,
) -> Result<(), StorageError> {
    let required = [
        ("receipt id", receipt.receipt_id.as_str()),
        ("bootstrap id", receipt.bootstrap_id.as_str()),
        ("activation hash", receipt.activation_hash.as_str()),
        ("activation id", receipt.activation_id.as_str()),
        ("input hash", receipt.input_hash.as_str()),
        ("belief family id", receipt.belief.family_id.as_str()),
        (
            "belief config snapshot hash",
            receipt.belief.config_snapshot_hash.as_str(),
        ),
        (
            "belief activation hash",
            receipt.belief.activation_hash.as_str(),
        ),
        (
            "belief activation id",
            receipt.belief.activation_id.as_str(),
        ),
        ("directive id", receipt.directive_id.as_str()),
        ("agent id", receipt.agent_id.as_str()),
        ("rule id", receipt.rule_id.as_str()),
        ("subscription id", receipt.subscription_id.as_str()),
    ];
    if let Some((field, _)) = required.iter().find(|(_, value)| value.trim().is_empty()) {
        return Err(StorageError::MigrationConflict(format!(
            "bootstrap receipt {field} is empty"
        )));
    }
    if receipt.completed_at_seq == 0
        || receipt.receipt_id != deterministic_id("agent-bootstrap-receipt", &receipt.bootstrap_id)
        || receipt.bootstrap_id != bootstrap_id
        || receipt.agent_id != agent_id
        || receipt.belief.activation_hash != receipt.activation_hash
        || receipt.belief.activation_id != receipt.activation_id
        || progress.bootstrap_id != receipt.bootstrap_id
        || progress.activation_id != receipt.activation_id
        || progress.activation_hash != receipt.activation_hash
        || progress.input_hash != receipt.input_hash
        || progress.stage != AgentBootstrapStage::Completed
        || progress.status != AgentBootstrapProgressStatus::Completed
        || progress.updated_at_seq != receipt.completed_at_seq
    {
        return Err(StorageError::MigrationConflict(format!(
            "bootstrap receipt '{bootstrap_id}' conflicts with its key, progress, or owner identity"
        )));
    }
    Ok(())
}

fn validate_hydration_pair(
    hydration: &AgentProcessHydrationRecord,
    activation: &AgentActivationRecord,
) -> Result<(), StorageError> {
    hydration
        .validate()
        .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
    activation.validate()?;
    let status_matches = matches!(
        (hydration.status, activation.status.clone()),
        (
            AgentProcessHydrationStatus::Started,
            AgentActivationStatus::Started
        ) | (
            AgentProcessHydrationStatus::Ready,
            AgentActivationStatus::Activated
        ) | (
            AgentProcessHydrationStatus::Failed,
            AgentActivationStatus::Failed
        )
    );
    if hydration.hydration_id != activation.activation_id
        || hydration.agent_id != activation.agent_id
        || hydration.started_at_seq != activation.started_at_seq
        || hydration.attempt_epoch != activation.attempt_epoch
        || hydration.updated_at_seq != activation.updated_at_seq
        || activation.lease_id.as_deref() != Some(hydration.lease_id.as_str())
        || hydration.last_error != activation.last_error
        || !status_matches
    {
        return Err(StorageError::MigrationConflict(format!(
            "hydration '{}' conflicts with its activation diagnostic",
            hydration.hydration_id
        )));
    }
    Ok(())
}

fn require_transaction_value(
    tree: &sled::transaction::TransactionalTree,
    key: &[u8],
    expected: &[u8],
    product: &str,
) -> Result<(), ConflictableTransactionError<String>> {
    let current = tree.get(key)?;
    if current.as_deref() != Some(expected) {
        return Err(ConflictableTransactionError::Abort(format!(
            "{product} changed during operational transition"
        )));
    }
    Ok(())
}

fn require_hydration_activation_fence_transaction(
    fence: &AgentHydrationFenceCapability,
    trees: [&sled::transaction::TransactionalTree; 10],
) -> Result<(), ConflictableTransactionError<String>> {
    if !hydration_activation_fence_matches_transaction(fence, trees)? {
        return Err(ConflictableTransactionError::Abort(
            "hydration owner, epoch, lease, current attempt, or checkpoint changed".to_string(),
        ));
    }
    Ok(())
}

fn hydration_activation_fence_matches_transaction(
    fence: &AgentHydrationFenceCapability,
    trees: [&sled::transaction::TransactionalTree; 10],
) -> Result<bool, ConflictableTransactionError<String>> {
    let [agents, receipts, directives, rules, subscriptions, belief_configs, hydrations, epochs, current, checkpoints] =
        trees;
    Ok(transaction_value_matches(
        agents,
        fence.snapshot.owner.agent.agent_id.as_bytes(),
        &fence.owner_bytes.agent,
    )? && transaction_value_matches(
        receipts,
        fence.snapshot.owner.receipt.bootstrap_id.as_bytes(),
        &fence.owner_bytes.receipt,
    )? && transaction_value_matches(
        directives,
        fence.snapshot.owner.directive.directive_id.as_bytes(),
        &fence.owner_bytes.directive,
    )? && transaction_value_matches(
        rules,
        fence.snapshot.owner.rule.rule_id.as_bytes(),
        &fence.owner_bytes.rule,
    )? && transaction_value_matches(
        subscriptions,
        fence.snapshot.owner.subscription.subscription_id.as_bytes(),
        &fence.owner_bytes.subscription,
    )? && transaction_value_matches(
        belief_configs,
        fence.snapshot.owner.belief_config_hash.as_bytes(),
        &fence.owner_bytes.belief_config,
    )? && transaction_value_matches(
        hydrations,
        fence.snapshot.hydration.hydration_id.as_bytes(),
        &fence.hydration_bytes,
    )? && transaction_value_matches(
        epochs,
        fence.snapshot.owner.agent.agent_id.as_bytes(),
        &fence.epoch_bytes,
    )? && transaction_value_matches(
        current,
        fence.snapshot.owner.agent.agent_id.as_bytes(),
        fence.snapshot.hydration.hydration_id.as_bytes(),
    )? && transaction_value_matches(
        checkpoints,
        fence.snapshot.checkpoint.hydration_id.as_bytes(),
        &fence.checkpoint_bytes,
    )?)
}

fn transaction_value_matches(
    tree: &sled::transaction::TransactionalTree,
    key: &[u8],
    expected: &[u8],
) -> Result<bool, ConflictableTransactionError<String>> {
    Ok(tree.get(key)?.as_deref() == Some(expected))
}

fn insert_exact_transaction_value(
    tree: &sled::transaction::TransactionalTree,
    key: &[u8],
    value: &[u8],
    product: &str,
) -> Result<(), ConflictableTransactionError<String>> {
    match tree.get(key)? {
        Some(current) if current.as_ref() != value => Err(ConflictableTransactionError::Abort(
            format!("{product} conflicts with its durable value"),
        )),
        Some(_) => Ok(()),
        None => {
            tree.insert(key, value)?;
            Ok(())
        }
    }
}

fn require_optional_transaction_value(
    tree: &sled::transaction::TransactionalTree,
    key: &[u8],
    expected: Option<&[u8]>,
    product: &str,
) -> Result<(), ConflictableTransactionError<String>> {
    if tree.get(key)?.as_deref() != expected {
        return Err(ConflictableTransactionError::Abort(format!(
            "{product} changed during fenced transition"
        )));
    }
    Ok(())
}

fn repair_exact_tree_value(
    tree: &Tree,
    key: &[u8],
    value: &[u8],
    product: &str,
) -> Result<(), StorageError> {
    match tree.get(key).map_err(to_storage_io)? {
        Some(current) if current.as_ref() != value => Err(StorageError::MigrationConflict(
            format!("{product} conflicts with its primary record"),
        )),
        Some(_) => Ok(()),
        None => {
            tree.insert(key, value).map_err(to_storage_io)?;
            Ok(())
        }
    }
}

fn encode_epoch(epoch: u64) -> [u8; 8] {
    epoch.to_be_bytes()
}

fn decode_epoch(raw: &[u8]) -> Result<u64, StorageError> {
    let encoded: [u8; 8] = raw.try_into().map_err(|_| {
        StorageError::IoError(io::Error::new(
            io::ErrorKind::InvalidData,
            "hydration epoch must contain eight bytes",
        ))
    })?;
    Ok(u64::from_be_bytes(encoded))
}

fn to_agent_transition_error(error: TransactionError<String>) -> StorageError {
    match error {
        TransactionError::Abort(message) => StorageError::Backpressure(message),
        TransactionError::Storage(error) => StorageError::IoError(io::Error::other(error)),
    }
}

fn decision_agent_key(agent_id: &str, seq: u64, decision_id: &str) -> String {
    format!("{agent_id}::{seq:0KEY_PAD$}::{decision_id}")
}

fn decision_dedupe_revision_key(
    dedupe_key: &AgentCurationDedupeKey,
    revision_id: Option<&str>,
) -> String {
    format!(
        "{}::{}",
        dedupe_key.index_key(),
        revision_id.unwrap_or("missing")
    )
}

fn decision_dedupe_satisfaction_review_key(
    dedupe_key: &AgentCurationDedupeKey,
    review: &AgentSatisfactionReview,
) -> String {
    format!(
        "{}::satisfaction-review::{}",
        dedupe_key.index_key(),
        review.index_key()
    )
}

fn decision_revision_key(revision_id: &str, decision_id: &str) -> String {
    format!("{revision_id}::{decision_id}")
}

fn sink_receipt_command_key(command_id: &str, decision_id: &str) -> String {
    format!("{command_id}::{decision_id}")
}

fn decode_optional<T: serde::de::DeserializeOwned>(
    raw: Option<sled::IVec>,
) -> Result<Option<T>, StorageError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&raw).map_err(to_storage_data)?))
}

fn to_storage_io(err: sled::Error) -> StorageError {
    StorageError::IoError(io::Error::other(err.to_string()))
}

fn to_storage_data(err: serde_json::Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

fn to_storage_utf8(err: std::string::FromUtf8Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

#[cfg(feature = "test-support")]
pub(crate) fn fuzz_terminal_hydration_fence(data: &[u8]) {
    if data.is_empty() {
        return;
    }
    let suffix = data
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let store = AgentStore::new(
        sled::Config::new()
            .temporary(true)
            .open()
            .expect("temporary hydration fuzz database"),
    )
    .expect("hydration fuzz store");
    let agent = AgentRecord {
        agent_id: format!("agent-{suffix}"),
        perspective_key: crate::world_state::graph::PerspectiveKey::new("agent", &suffix)
            .expect("hydration fuzz perspective"),
        subject: crate::events::DomainObjectRef::new("workspace", "node", &suffix)
            .expect("hydration fuzz subject"),
        branch_scope: crate::belief::BranchScope::main(),
        observation_scope: format!("scope-{suffix}"),
        directive_id: format!("directive-{suffix}"),
        seed_provenance: "fuzz".to_string(),
        status: AgentStatus::Registered,
        created_at_seq: 1,
        updated_at_seq: 1,
    };
    let directive = DirectiveRecord {
        directive_id: agent.directive_id.clone(),
        text: "hydrate fuzz agent".to_string(),
    };
    let rule = AgentCurationRuleRecord {
        rule_id: format!("rule-{suffix}"),
        agent_id: agent.agent_id.clone(),
        config: crate::agent::AgentCurationRuleConfig {
            dimension_id: format!("dimension-{suffix}"),
            threshold: 0.7,
            priority_urgency: 10,
            desired_summary: "ready".to_string(),
            source_kind: "belief".to_string(),
        },
    };
    let subscription = AgentSubscriptionRecord {
        subscription_id: format!("subscription-{suffix}"),
        agent_id: agent.agent_id.clone(),
        belief_key: crate::belief::BeliefKey {
            subject: agent.subject.clone(),
            dimension_id: rule.config.dimension_id.clone(),
            predicate_id: "confidence".to_string(),
            perspective: agent.perspective_key.clone(),
            branch_scope: agent.branch_scope.clone(),
            evidence_policy_id: format!("policy-{suffix}"),
        },
        status: AgentSubscriptionStatus::Active,
        last_delivered_revision_id: None,
        last_delivered_seq: 0,
        created_at_seq: 2,
        updated_at_seq: 2,
    };
    let bootstrap_id = format!("bootstrap-{suffix}");
    let receipt = AgentBootstrapReceipt {
        receipt_id: deterministic_id("agent-bootstrap-receipt", &bootstrap_id),
        bootstrap_id,
        activation_hash: "a".repeat(64),
        activation_id: format!("activation-{suffix}"),
        input_hash: "b".repeat(64),
        belief: crate::activation::BeliefActivationReceipt {
            family_id: format!("family-{suffix}"),
            config_snapshot_hash: "c".repeat(64),
            activation_hash: "a".repeat(64),
            activation_id: format!("activation-{suffix}"),
        },
        directive_id: agent.directive_id.clone(),
        agent_id: agent.agent_id.clone(),
        rule_id: rule.rule_id.clone(),
        subscription_id: subscription.subscription_id.clone(),
        completed_at_seq: 3,
    };
    store.put_agent(&agent).expect("persist fuzz agent");
    store
        .directives
        .insert(
            directive.directive_id.as_bytes(),
            serde_json::to_vec(&directive).expect("encode fuzz directive"),
        )
        .expect("persist fuzz directive");
    store
        .curation_rules
        .insert(
            rule.rule_id.as_bytes(),
            serde_json::to_vec(&rule).expect("encode fuzz rule"),
        )
        .expect("persist fuzz rule");
    store
        .put_subscription(&subscription)
        .expect("persist fuzz subscription");
    store
        .bootstrap_receipts
        .insert(
            receipt.bootstrap_id.as_bytes(),
            serde_json::to_vec(&receipt).expect("encode fuzz receipt"),
        )
        .expect("persist fuzz receipt");
    let belief_config = BeliefConfigSnapshotFence::open(
        &store.db,
        receipt.belief.config_snapshot_hash.clone(),
        format!("{{\"seed\":\"{suffix}\"}}"),
    )
    .expect("fuzz belief config fence");
    belief_config
        .tree
        .insert(belief_config.hash.as_bytes(), belief_config.json.as_bytes())
        .expect("persist fuzz belief config");
    let owner = AgentHydrationOwnerFence {
        agent,
        receipt,
        directive,
        rule,
        subscription,
        belief_config,
    };
    let command = StartAgentHydrationCommand {
        hydration_id: format!("hydration-{suffix}"),
        agent_id: owner.agent.agent_id.clone(),
        expected_prior_attempt_epoch: 0,
        attempt_epoch: 1,
        lease_id: format!("lease-{suffix}"),
        started_at_seq: 4,
    };
    let hydration = store
        .start_process_hydration_fenced(&command, &owner)
        .expect("start fuzz hydration");
    let selected = AgentHydrationCheckpoint {
        hydration_id: command.hydration_id.clone(),
        agent_id: command.agent_id.clone(),
        attempt_epoch: command.attempt_epoch,
        lease_id: command.lease_id.clone(),
        subscription_id: owner.subscription.subscription_id.clone(),
        selected_revision_id: format!("revision-{suffix}"),
        selected_revision_seq: 3,
        stage: AgentHydrationCheckpointStage::Selected,
        attestation_id: None,
        planner_request_id: None,
        updated_at_seq: 5,
    };
    store
        .put_hydration_checkpoint_fenced(&selected, &owner)
        .expect("persist fuzz checkpoint");
    let initial_capability = store
        .hydration_activation_fence(&owner, &hydration, &selected)
        .expect("build initial fuzz capability");
    let encoded = initial_capability
        .snapshot_bytes()
        .expect("encode initial fuzz capability");
    let reopened = AgentHydrationFenceCapability::reopen(&store.db, &encoded)
        .expect("reopen initial fuzz capability");
    reopened
        .transaction_hydration_and_one(&store.activations, |_, _| Ok(()))
        .expect("verify reopened initial fuzz capability");

    let planner_request = crate::planner::PlannerProjectionRequest::identified_attested(
        format!("fuzz-source-{suffix}"),
        owner.agent.agent_id.clone(),
        owner.agent.subject.clone(),
        owner.agent.perspective_key.clone(),
        owner.agent.branch_scope.clone(),
        vec![owner.rule.config.dimension_id.clone()],
        Vec::new(),
        crate::planner::PlannerAttestedBeliefSnapshot {
            attestation_id: format!("attestation-{suffix}"),
            revision_id: format!("revision-{suffix}"),
            revision_hash: "d".repeat(64),
            view_id: format!("view-{suffix}"),
            view_hash: "e".repeat(64),
            source_cursor_start: 3,
            source_cursor_end: 3,
        },
    )
    .expect("fuzz planner request");
    let planner_store =
        crate::planner::PlannerProjectionStore::new(store.db.clone()).expect("fuzz planner store");
    let mut attested = selected.clone();
    attested.stage = AgentHydrationCheckpointStage::Attested;
    attested.attestation_id = planner_request
        .attested_belief
        .as_ref()
        .map(|snapshot| snapshot.attestation_id.clone());
    attested.updated_at_seq = 7;
    store
        .advance_hydration_checkpoint_cas_fenced(&selected, &attested, &owner)
        .expect("persist fuzz attested checkpoint");
    let mut projected = attested.clone();
    projected.stage = AgentHydrationCheckpointStage::ProjectionRequested;
    projected.planner_request_id = Some(planner_request.request_id.clone());
    projected.updated_at_seq = 9;
    store
        .advance_hydration_checkpoint_cas_fenced(&attested, &projected, &owner)
        .expect("persist fuzz projection checkpoint");
    let planner_capability = store
        .hydration_activation_fence(&owner, &hydration, &projected)
        .expect("build fuzz planner capability");
    let pending = planner_store
        .put_fenced_pending_for_fuzz(planner_request.clone(), 8, &planner_capability)
        .expect("persist fuzz fenced planner request");
    let drift_mode = data.get(1).copied().unwrap_or(0) % 4;
    let mut concurrent_writer = None;
    let mut release_writer = None;
    match drift_mode {
        0 => {
            let directive_tree = store.directives.clone();
            let mut drifted = owner.directive.clone();
            drifted.text.push_str(" planner drift");
            let (written_tx, written_rx) = std::sync::mpsc::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            concurrent_writer = Some(std::thread::spawn(move || {
                directive_tree
                    .insert(
                        drifted.directive_id.as_bytes(),
                        serde_json::to_vec(&drifted).expect("encode planner owner drift"),
                    )
                    .expect("persist planner owner drift");
                written_tx.send(()).expect("report planner owner drift");
                release_rx.recv().expect("release planner owner writer");
            }));
            written_rx.recv().expect("observe planner owner drift");
            release_writer = Some(release_tx);
        }
        1 => {
            let mut drifted = projected.clone();
            drifted.selected_revision_id.push_str("-drift");
            store
                .hydration_checkpoints
                .insert(
                    drifted.hydration_id.as_bytes(),
                    serde_json::to_vec(&drifted).expect("encode planner checkpoint drift"),
                )
                .expect("persist planner checkpoint drift");
        }
        2 => {
            store
                .current_hydrations
                .insert(owner.agent.agent_id.as_bytes(), b"foreign-hydration")
                .expect("persist planner current hydration drift");
        }
        _ => {
            let mut drifted = hydration.clone();
            drifted.lease_id.push_str("-drift");
            store
                .process_hydrations
                .insert(
                    drifted.hydration_id.as_bytes(),
                    serde_json::to_vec(&drifted).expect("encode planner hydration drift"),
                )
                .expect("persist planner hydration drift");
        }
    }
    let reopened_planner = crate::planner::PlannerProjectionStore::new(store.db.clone())
        .expect("reopen fuzz planner store");
    let terminal = if data.get(2).copied().unwrap_or(0) % 2 == 0 {
        let output = crate::planner::PlannerProjectionOutput {
            world_state: meld_lang::WorldState::new(Vec::new())
                .expect("empty fuzz planner world state"),
            projection_version: crate::planner::PLANNER_PROJECTION_VERSION.to_string(),
            source_refs: vec![crate::planner::PlannerSourceRef::BeliefRevision {
                revision_id: format!("revision-{suffix}"),
            }],
            hydration_refs: crate::planner::PlannerHydrationRefs {
                revision_ids: vec![format!("revision-{suffix}")],
                ..crate::planner::PlannerHydrationRefs::default()
            },
            warnings: Vec::new(),
        };
        let frame = crate::planner::PlannerProjectionFrame {
            identity: crate::planner::PlannerProjectionFrameIdentity::identified(
                &planner_request,
                &output,
            )
            .expect("fuzz planner frame identity"),
            output,
            completed_at_seq: pending.updated_at_seq + 1,
        };
        reopened_planner.complete_attested_for_fuzz(
            &planner_request.request_id,
            pending.updated_at_seq,
            frame,
            pending.updated_at_seq + 1,
        )
    } else {
        reopened_planner.fail_attested_for_fuzz(
            &planner_request.request_id,
            pending.updated_at_seq,
            "fuzz planner failure",
            pending.updated_at_seq + 1,
        )
    };
    assert!(terminal.is_err());
    assert!(reopened_planner
        .get_request(&planner_request.request_id)
        .expect("read revoked fuzz planner request")
        .is_none());
    assert!(reopened_planner
        .pending_requests_bounded(1)
        .expect("read revoked fuzz planner selection")
        .records
        .is_empty());
    assert!(!reopened_planner
        .has_owner_fence_for_fuzz(&planner_request.request_id)
        .expect("read revoked fuzz planner owner fence"));
    if let Some(release) = release_writer {
        release.send(()).expect("release planner owner writer");
    }
    if let Some(writer) = concurrent_writer {
        writer.join().expect("join planner owner writer");
    }
    match drift_mode {
        0 => {
            store
                .directives
                .insert(
                    owner.directive.directive_id.as_bytes(),
                    serde_json::to_vec(&owner.directive).expect("encode restored directive"),
                )
                .expect("restore planner directive");
        }
        1 => {
            store
                .hydration_checkpoints
                .insert(
                    projected.hydration_id.as_bytes(),
                    serde_json::to_vec(&projected).expect("encode restored checkpoint"),
                )
                .expect("restore planner checkpoint");
        }
        2 => {
            store
                .current_hydrations
                .insert(
                    owner.agent.agent_id.as_bytes(),
                    command.hydration_id.as_bytes(),
                )
                .expect("restore planner current hydration");
        }
        _ => {
            store
                .process_hydrations
                .insert(
                    hydration.hydration_id.as_bytes(),
                    serde_json::to_vec(&hydration).expect("encode restored hydration"),
                )
                .expect("restore planner hydration");
        }
    }
    store
        .hydration_checkpoints
        .insert(
            selected.hydration_id.as_bytes(),
            serde_json::to_vec(&selected).expect("encode reset checkpoint"),
        )
        .expect("reset checkpoint after planner terminal fuzzing");

    let failure = FailAgentHydrationCommand {
        hydration_id: command.hydration_id.clone(),
        attempt_epoch: command.attempt_epoch,
        lease_id: command.lease_id.clone(),
        expected_updated_at_seq: hydration.updated_at_seq,
        failed_at_seq: 8,
        error: "fuzz terminal failure".to_string(),
    };
    let mut current_checkpoint = selected.clone();
    match data[0] % 4 {
        0 => {
            let mut drifted = owner.directive.clone();
            drifted.text.push_str(" drift");
            store
                .directives
                .insert(
                    drifted.directive_id.as_bytes(),
                    serde_json::to_vec(&drifted).expect("encode drifted directive"),
                )
                .expect("persist drifted directive");
            assert!(store
                .fail_process_hydration_fenced(&failure, &owner, &selected)
                .is_err());
            store
                .directives
                .insert(
                    owner.directive.directive_id.as_bytes(),
                    serde_json::to_vec(&owner.directive).expect("encode restored directive"),
                )
                .expect("restore directive");
        }
        1 => {
            current_checkpoint.stage = AgentHydrationCheckpointStage::Attested;
            current_checkpoint.attestation_id = Some(format!("attestation-{suffix}"));
            current_checkpoint.updated_at_seq = 7;
            store
                .advance_hydration_checkpoint_cas_fenced(&selected, &current_checkpoint, &owner)
                .expect("advance fuzz checkpoint");
            assert!(store
                .fail_process_hydration_fenced(&failure, &owner, &selected)
                .is_err());
        }
        2 => {
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
            let writer_barrier = std::sync::Arc::clone(&barrier);
            let directive_tree = store.directives.clone();
            let mut drifted = owner.directive.clone();
            drifted.text.push_str(" concurrent drift");
            let writer = std::thread::spawn(move || {
                writer_barrier.wait();
                directive_tree
                    .insert(
                        drifted.directive_id.as_bytes(),
                        serde_json::to_vec(&drifted).expect("encode concurrent drift"),
                    )
                    .expect("persist concurrent drift");
            });
            barrier.wait();
            let outcome =
                store.fail_process_hydration_fenced(&failure, &owner, &current_checkpoint);
            writer.join().expect("join concurrent drift writer");
            let durable = store
                .get_process_hydration(&command.hydration_id)
                .expect("read concurrent hydration")
                .expect("concurrent hydration");
            assert_eq!(
                outcome.is_ok(),
                durable.status == AgentProcessHydrationStatus::Failed
            );
            store
                .directives
                .insert(
                    owner.directive.directive_id.as_bytes(),
                    serde_json::to_vec(&owner.directive).expect("encode restored directive"),
                )
                .expect("restore concurrent directive");
        }
        _ => {}
    }
    let failed = match store.get_process_hydration(&command.hydration_id) {
        Ok(Some(record)) if record.status == AgentProcessHydrationStatus::Failed => record,
        _ => store
            .fail_process_hydration_fenced(&failure, &owner, &current_checkpoint)
            .expect("commit fuzz terminal failure"),
    };
    let replayed = store
        .fail_process_hydration_fenced(&failure, &owner, &current_checkpoint)
        .expect("replay fuzz terminal failure");
    assert_eq!(replayed, failed);
    let failed_capability = store
        .hydration_activation_fence(&owner, &failed, &current_checkpoint)
        .expect("build failed fuzz capability");
    let failed_encoded = failed_capability
        .snapshot_bytes()
        .expect("encode failed fuzz capability");
    AgentHydrationFenceCapability::reopen(&store.db, &failed_encoded)
        .expect("reopen failed fuzz capability")
        .transaction_hydration_and_one(&store.activations, |_, _| Ok(()))
        .expect("verify reopened failed fuzz capability");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activation::BeliefActivationReceipt;
    use crate::belief::BranchScope;
    use crate::events::DomainObjectRef;
    use crate::world_state::graph::PerspectiveKey;

    #[test]
    fn complete_readiness_schema_replay_is_flush_free() {
        let store = AgentStore::new(
            sled::Config::new()
                .temporary(true)
                .open()
                .expect("temporary agent database"),
        )
        .expect("agent store");
        let state: AgentReadinessMigrationState = serde_json::from_slice(
            &store
                .readiness_schema
                .get(KEY_READINESS_SCHEMA_STATE)
                .expect("schema state read")
                .expect("schema state"),
        )
        .expect("schema state decode");
        assert!(state.complete);

        let before = store.flush_calls();
        store
            .migrate_legacy_readiness_proofs()
            .expect("complete schema replay");
        assert_eq!(store.flush_calls(), before);
    }

    #[test]
    fn hydration_capability_matches_only_its_exact_readiness_scope() {
        let (store, owner, command, checkpoint) = hydration_owner_fixture("scope-owner");
        let hydration = store
            .start_process_hydration_fenced(&command, &owner)
            .expect("start hydration");
        store
            .put_hydration_checkpoint_fenced(&checkpoint, &owner)
            .expect("persist checkpoint");
        let capability = store
            .hydration_activation_fence(&owner, &hydration, &checkpoint)
            .expect("hydration capability");
        assert!(capability.matches_readiness_scope(
            &owner.agent.agent_id,
            &owner.subscription.subscription_id,
            &owner.subscription.belief_key,
        ));

        let (_, other, _, _) = hydration_owner_fixture("scope-other");
        assert!(!capability.matches_readiness_scope(
            &other.agent.agent_id,
            &other.subscription.subscription_id,
            &other.subscription.belief_key,
        ));
    }

    #[test]
    fn exact_hydration_start_reflushes_after_indeterminate_flush() {
        let store = AgentStore::new(
            sled::Config::new()
                .temporary(true)
                .open()
                .expect("temporary agent database"),
        )
        .expect("agent store");
        let agent = AgentRecord {
            agent_id: "agent-a".to_string(),
            perspective_key: PerspectiveKey::new("agent", "a").expect("perspective"),
            subject: DomainObjectRef::new("workspace", "node", "a").expect("subject"),
            branch_scope: BranchScope::main(),
            observation_scope: "readiness".to_string(),
            directive_id: "directive-a".to_string(),
            seed_provenance: "test".to_string(),
            status: AgentStatus::Registered,
            created_at_seq: 1,
            updated_at_seq: 1,
        };
        store.put_agent(&agent).expect("persist agent");
        let command = StartAgentHydrationCommand {
            hydration_id: "hydration-a".to_string(),
            agent_id: agent.agent_id,
            expected_prior_attempt_epoch: 0,
            attempt_epoch: 1,
            lease_id: "lease-a".to_string(),
            started_at_seq: 2,
        };

        store.fail_next_flush();
        assert!(matches!(
            store.start_process_hydration(&command),
            Err(StorageError::DurabilityIndeterminate(_))
        ));
        let replayed = store
            .start_process_hydration(&command)
            .expect("exact start retry reflushes");
        assert_eq!(replayed.attempt_epoch, 1);
        assert_eq!(
            store
                .get_activation(&command.hydration_id)
                .expect("activation read")
                .expect("activation")
                .status,
            AgentActivationStatus::Started
        );
    }

    #[test]
    fn exact_checkpoint_advancement_reflushes_both_indeterminate_boundaries() {
        let store = AgentStore::new(
            sled::Config::new()
                .temporary(true)
                .open()
                .expect("temporary agent database"),
        )
        .expect("agent store");
        let agent = AgentRecord {
            agent_id: "agent-checkpoint".to_string(),
            perspective_key: PerspectiveKey::new("agent", "checkpoint").expect("perspective"),
            subject: DomainObjectRef::new("workspace", "node", "checkpoint").expect("subject"),
            branch_scope: BranchScope::main(),
            observation_scope: "readiness".to_string(),
            directive_id: "directive-checkpoint".to_string(),
            seed_provenance: "test".to_string(),
            status: AgentStatus::Registered,
            created_at_seq: 1,
            updated_at_seq: 1,
        };
        store.put_agent(&agent).expect("persist agent");
        let subscription = AgentSubscriptionRecord {
            subscription_id: "subscription-checkpoint".to_string(),
            agent_id: agent.agent_id.clone(),
            belief_key: crate::belief::BeliefKey {
                subject: agent.subject.clone(),
                dimension_id: "readiness".to_string(),
                predicate_id: "confidence".to_string(),
                perspective: agent.perspective_key.clone(),
                branch_scope: agent.branch_scope.clone(),
                evidence_policy_id: "policy-checkpoint".to_string(),
            },
            status: AgentSubscriptionStatus::Active,
            last_delivered_revision_id: None,
            last_delivered_seq: 0,
            created_at_seq: 2,
            updated_at_seq: 2,
        };
        store
            .put_subscription(&subscription)
            .expect("persist subscription");
        let command = StartAgentHydrationCommand {
            hydration_id: "hydration-checkpoint".to_string(),
            agent_id: agent.agent_id,
            expected_prior_attempt_epoch: 0,
            attempt_epoch: 1,
            lease_id: "lease-checkpoint".to_string(),
            started_at_seq: 3,
        };
        store
            .start_process_hydration(&command)
            .expect("start hydration");
        let selected = AgentHydrationCheckpoint {
            hydration_id: command.hydration_id,
            agent_id: command.agent_id,
            attempt_epoch: command.attempt_epoch,
            lease_id: command.lease_id,
            subscription_id: subscription.subscription_id,
            selected_revision_id: "revision-checkpoint".to_string(),
            selected_revision_seq: 2,
            stage: AgentHydrationCheckpointStage::Selected,
            attestation_id: None,
            planner_request_id: None,
            updated_at_seq: 4,
        };
        store
            .put_hydration_checkpoint(&selected)
            .expect("persist selected checkpoint");
        let mut attested = selected.clone();
        attested.stage = AgentHydrationCheckpointStage::Attested;
        attested.attestation_id = Some("attestation-checkpoint".to_string());
        attested.updated_at_seq = 6;
        store.fail_next_flush();
        assert!(matches!(
            store.advance_hydration_checkpoint_cas(&selected, &attested),
            Err(StorageError::DurabilityIndeterminate(_))
        ));
        assert_eq!(
            store
                .advance_hydration_checkpoint_cas(&selected, &attested)
                .expect("attested replay reflushes"),
            attested
        );

        let mut projected = attested.clone();
        projected.stage = AgentHydrationCheckpointStage::ProjectionRequested;
        projected.planner_request_id = Some("projection-checkpoint".to_string());
        projected.updated_at_seq = 8;
        store.fail_next_flush();
        assert!(matches!(
            store.advance_hydration_checkpoint_cas(&attested, &projected),
            Err(StorageError::DurabilityIndeterminate(_))
        ));
        assert_eq!(
            store
                .advance_hydration_checkpoint_cas(&attested, &projected)
                .expect("projection replay reflushes"),
            projected
        );
    }

    fn hydration_owner_fixture(
        suffix: &str,
    ) -> (
        AgentStore,
        AgentHydrationOwnerFence,
        StartAgentHydrationCommand,
        AgentHydrationCheckpoint,
    ) {
        let db = sled::Config::new()
            .temporary(true)
            .open()
            .expect("temporary agent database");
        hydration_owner_fixture_with_db(suffix, db)
    }

    fn hydration_owner_fixture_with_db(
        suffix: &str,
        db: Db,
    ) -> (
        AgentStore,
        AgentHydrationOwnerFence,
        StartAgentHydrationCommand,
        AgentHydrationCheckpoint,
    ) {
        let store = AgentStore::new(db).expect("agent store");
        let agent = AgentRecord {
            agent_id: format!("agent-{suffix}"),
            perspective_key: PerspectiveKey::new("agent", suffix).expect("perspective"),
            subject: DomainObjectRef::new("workspace", "node", suffix).expect("subject"),
            branch_scope: BranchScope::main(),
            observation_scope: format!("scope-{suffix}"),
            directive_id: format!("directive-{suffix}"),
            seed_provenance: "test".to_string(),
            status: AgentStatus::Registered,
            created_at_seq: 1,
            updated_at_seq: 1,
        };
        let rule = AgentCurationRuleRecord {
            rule_id: format!("rule-{suffix}"),
            agent_id: agent.agent_id.clone(),
            config: crate::agent::AgentCurationRuleConfig {
                dimension_id: format!("dimension-{suffix}"),
                threshold: 0.7,
                priority_urgency: 10,
                desired_summary: "ready".to_string(),
                source_kind: "belief".to_string(),
            },
        };
        let directive = DirectiveRecord {
            directive_id: agent.directive_id.clone(),
            text: "hydrate agent".to_string(),
        };
        let subscription = AgentSubscriptionRecord {
            subscription_id: format!("subscription-{suffix}"),
            agent_id: agent.agent_id.clone(),
            belief_key: crate::belief::BeliefKey {
                subject: agent.subject.clone(),
                dimension_id: rule.config.dimension_id.clone(),
                predicate_id: "confidence".to_string(),
                perspective: agent.perspective_key.clone(),
                branch_scope: agent.branch_scope.clone(),
                evidence_policy_id: format!("policy-{suffix}"),
            },
            status: AgentSubscriptionStatus::Active,
            last_delivered_revision_id: None,
            last_delivered_seq: 0,
            created_at_seq: 2,
            updated_at_seq: 2,
        };
        let bootstrap_id = format!("bootstrap-{suffix}");
        let receipt = AgentBootstrapReceipt {
            receipt_id: deterministic_id("agent-bootstrap-receipt", &bootstrap_id),
            bootstrap_id: bootstrap_id.clone(),
            activation_hash: "a".repeat(64),
            activation_id: format!("activation-{suffix}"),
            input_hash: "b".repeat(64),
            belief: BeliefActivationReceipt {
                family_id: format!("family-{suffix}"),
                config_snapshot_hash: "c".repeat(64),
                activation_hash: "a".repeat(64),
                activation_id: format!("activation-{suffix}"),
            },
            directive_id: agent.directive_id.clone(),
            agent_id: agent.agent_id.clone(),
            rule_id: rule.rule_id.clone(),
            subscription_id: subscription.subscription_id.clone(),
            completed_at_seq: 3,
        };
        store.put_agent(&agent).expect("persist agent");
        store
            .directives
            .insert(
                directive.directive_id.as_bytes(),
                serde_json::to_vec(&directive).expect("encode directive"),
            )
            .expect("persist directive");
        store
            .put_subscription(&subscription)
            .expect("persist subscription");
        store
            .curation_rules
            .insert(
                rule.rule_id.as_bytes(),
                serde_json::to_vec(&rule).expect("encode rule"),
            )
            .expect("persist rule");
        store
            .bootstrap_receipts
            .insert(
                receipt.bootstrap_id.as_bytes(),
                serde_json::to_vec(&receipt).expect("encode receipt"),
            )
            .expect("persist receipt");
        let progress = AgentBootstrapProgress {
            bootstrap_id,
            activation_id: receipt.activation_id.clone(),
            activation_hash: receipt.activation_hash.clone(),
            input_hash: receipt.input_hash.clone(),
            stage: AgentBootstrapStage::Completed,
            status: AgentBootstrapProgressStatus::Completed,
            updated_at_seq: receipt.completed_at_seq,
        };
        store
            .bootstrap_progress
            .insert(
                progress.bootstrap_id.as_bytes(),
                serde_json::to_vec(&progress).expect("encode progress"),
            )
            .expect("persist progress");
        let belief_config = BeliefConfigSnapshotFence::open(
            &store.db,
            receipt.belief.config_snapshot_hash.clone(),
            "{\"fixture\":true}".to_string(),
        )
        .expect("belief config fence");
        belief_config
            .tree
            .insert(belief_config.hash.as_bytes(), belief_config.json.as_bytes())
            .expect("persist belief config");
        let command = StartAgentHydrationCommand {
            hydration_id: format!("hydration-{suffix}"),
            agent_id: agent.agent_id.clone(),
            expected_prior_attempt_epoch: 0,
            attempt_epoch: 1,
            lease_id: format!("lease-{suffix}"),
            started_at_seq: 4,
        };
        let checkpoint = AgentHydrationCheckpoint {
            hydration_id: command.hydration_id.clone(),
            agent_id: command.agent_id.clone(),
            attempt_epoch: 1,
            lease_id: command.lease_id.clone(),
            subscription_id: subscription.subscription_id.clone(),
            selected_revision_id: format!("revision-{suffix}"),
            selected_revision_seq: 3,
            stage: AgentHydrationCheckpointStage::Selected,
            attestation_id: None,
            planner_request_id: None,
            updated_at_seq: 5,
        };
        (
            store,
            AgentHydrationOwnerFence {
                agent,
                receipt,
                directive,
                rule,
                subscription,
                belief_config,
            },
            command,
            checkpoint,
        )
    }

    #[test]
    fn concurrent_owner_drift_is_fenced_at_every_hydration_boundary() {
        let (store, owner, command, _) = hydration_owner_fixture("start-drift");
        let mut changed_directive = owner.directive.clone();
        changed_directive.text.push_str(" drift");
        let writer = store.clone();
        std::thread::spawn(move || {
            writer
                .directives
                .insert(
                    changed_directive.directive_id.as_bytes(),
                    serde_json::to_vec(&changed_directive).expect("encode changed directive"),
                )
                .expect("concurrent directive write");
        })
        .join()
        .expect("concurrent start writer");
        assert!(matches!(
            store.start_process_hydration_fenced(&command, &owner),
            Err(StorageError::Backpressure(_))
        ));
        assert!(store
            .get_process_hydration(&command.hydration_id)
            .expect("hydration read")
            .is_none());

        let (store, owner, command, selected) = hydration_owner_fixture("attestation-drift");
        store
            .start_process_hydration_fenced(&command, &owner)
            .expect("start hydration");
        store
            .put_hydration_checkpoint_fenced(&selected, &owner)
            .expect("persist selected checkpoint");
        let mut changed_rule = owner.rule.clone();
        changed_rule.config.priority_urgency += 1;
        let writer = store.clone();
        std::thread::spawn(move || {
            writer
                .curation_rules
                .insert(
                    changed_rule.rule_id.as_bytes(),
                    serde_json::to_vec(&changed_rule).expect("encode changed rule"),
                )
                .expect("concurrent rule write");
        })
        .join()
        .expect("concurrent attestation writer");
        let mut attested = selected.clone();
        attested.stage = AgentHydrationCheckpointStage::Attested;
        attested.attestation_id = Some("attestation-drift".to_string());
        attested.updated_at_seq = 7;
        assert!(matches!(
            store.advance_hydration_checkpoint_cas_fenced(&selected, &attested, &owner),
            Err(StorageError::Backpressure(_))
        ));
        assert_eq!(
            store
                .hydration_checkpoint(&selected.hydration_id)
                .expect("checkpoint read"),
            Some(selected)
        );

        let (store, owner, command, selected) = hydration_owner_fixture("projection-drift");
        store
            .start_process_hydration_fenced(&command, &owner)
            .expect("start hydration");
        store
            .put_hydration_checkpoint_fenced(&selected, &owner)
            .expect("persist selected checkpoint");
        let mut attested = selected.clone();
        attested.stage = AgentHydrationCheckpointStage::Attested;
        attested.attestation_id = Some("attestation-projection-drift".to_string());
        attested.updated_at_seq = 7;
        store
            .advance_hydration_checkpoint_cas_fenced(&selected, &attested, &owner)
            .expect("persist attested checkpoint");
        let config_tree = owner.belief_config.tree.clone();
        let config_hash = owner.belief_config.hash.clone();
        std::thread::spawn(move || {
            config_tree
                .insert(config_hash.as_bytes(), b"{\"fixture\":false}")
                .expect("concurrent belief config write");
        })
        .join()
        .expect("concurrent projection writer");
        let mut projected = attested.clone();
        projected.stage = AgentHydrationCheckpointStage::ProjectionRequested;
        projected.planner_request_id = Some("request-projection-drift".to_string());
        projected.updated_at_seq = 9;
        assert!(matches!(
            store.advance_hydration_checkpoint_cas_fenced(&attested, &projected, &owner),
            Err(StorageError::Backpressure(_))
        ));
        assert_eq!(
            store
                .hydration_checkpoint(&attested.hydration_id)
                .expect("checkpoint read"),
            Some(attested)
        );
    }

    #[test]
    fn terminal_failure_fences_owner_and_checkpoint_and_replays_after_reopen() {
        let temp = tempfile::tempdir().expect("temporary directory");
        let path = temp.path().join("terminal-failure");
        let db = sled::open(&path).expect("agent database");
        let (store, owner, command, selected) =
            hydration_owner_fixture_with_db("terminal-failure", db);
        let started = store
            .start_process_hydration_fenced(&command, &owner)
            .expect("start hydration");
        store
            .put_hydration_checkpoint_fenced(&selected, &owner)
            .expect("persist selected checkpoint");
        let failure = FailAgentHydrationCommand {
            hydration_id: command.hydration_id.clone(),
            attempt_epoch: command.attempt_epoch,
            lease_id: command.lease_id.clone(),
            expected_updated_at_seq: started.updated_at_seq,
            failed_at_seq: selected.updated_at_seq + 3,
            error: "planner unavailable".to_string(),
        };

        let mut changed_directive = owner.directive.clone();
        changed_directive.text.push_str(" drift");
        store
            .directives
            .insert(
                changed_directive.directive_id.as_bytes(),
                serde_json::to_vec(&changed_directive).expect("encode changed directive"),
            )
            .expect("persist changed directive");
        assert!(matches!(
            store.fail_process_hydration_fenced(&failure, &owner, &selected),
            Err(StorageError::Backpressure(_))
        ));
        assert_eq!(
            store
                .get_process_hydration(&command.hydration_id)
                .expect("hydration read")
                .expect("hydration")
                .status,
            AgentProcessHydrationStatus::Started
        );
        store
            .directives
            .insert(
                owner.directive.directive_id.as_bytes(),
                serde_json::to_vec(&owner.directive).expect("encode original directive"),
            )
            .expect("restore directive");

        let mut attested = selected.clone();
        attested.stage = AgentHydrationCheckpointStage::Attested;
        attested.attestation_id = Some("attestation-terminal-failure".to_string());
        attested.updated_at_seq += 2;
        store
            .advance_hydration_checkpoint_cas_fenced(&selected, &attested, &owner)
            .expect("advance checkpoint");
        assert!(matches!(
            store.fail_process_hydration_fenced(&failure, &owner, &selected),
            Err(StorageError::Backpressure(_))
        ));
        assert_eq!(
            store
                .get_process_hydration(&command.hydration_id)
                .expect("hydration read")
                .expect("hydration")
                .status,
            AgentProcessHydrationStatus::Started
        );

        store.fail_next_flush();
        assert!(matches!(
            store.fail_process_hydration_fenced(&failure, &owner, &attested),
            Err(StorageError::DurabilityIndeterminate(_))
        ));
        let failed = store
            .fail_process_hydration_fenced(&failure, &owner, &attested)
            .expect("exact failure retry reflushes");
        assert_eq!(failed.status, AgentProcessHydrationStatus::Failed);
        let owner_records = (
            owner.agent.clone(),
            owner.receipt.clone(),
            owner.directive.clone(),
            owner.rule.clone(),
            owner.subscription.clone(),
            owner.belief_config.hash.clone(),
            owner.belief_config.json.clone(),
        );
        store.flush().expect("flush terminal failure");
        drop(owner);
        drop(store);

        let reopened = AgentStore::new(sled::open(&path).expect("reopen database"))
            .expect("reopen agent store");
        let reopened_owner = AgentHydrationOwnerFence {
            agent: owner_records.0,
            receipt: owner_records.1,
            directive: owner_records.2,
            rule: owner_records.3,
            subscription: owner_records.4,
            belief_config: BeliefConfigSnapshotFence::reopen(
                &reopened.db,
                owner_records.5,
                owner_records.6,
            )
            .expect("reopen belief config fence"),
        };
        assert_eq!(
            reopened
                .fail_process_hydration_fenced(&failure, &reopened_owner, &attested)
                .expect("exact failure replay after reopen"),
            failed
        );
    }
}
