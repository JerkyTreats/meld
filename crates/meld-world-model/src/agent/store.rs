//! Durable agent storage.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
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
use crate::agent::bootstrap::AgentBootstrapProgress;
use crate::agent::contracts::{
    AgentActivationRecord, AgentActivationStatus, AgentAuthoredCommand, AgentCurationDecision,
    AgentCurationDedupeKey, AgentCurationOutcome, AgentDecisionKind, AgentDecisionOutboxRecord,
    AgentDeliverySelection, AgentHydrationCheckpoint, AgentHydrationCheckpointStage, AgentRecord,
    AgentSatisfactionCursorCasIntent, AgentSatisfactionCursorIdentity, AgentSatisfactionReview,
    AgentSatisfactionReviewCursor, AgentSatisfactionReviewSelection, AgentSemanticEnablementAudit,
    AgentSinkReceipt, AgentSinkReceiptKind, AgentStatus, AgentSubscriptionCursorCasIntent,
    AgentSubscriptionRecord, AgentSubscriptionStatus,
};
use crate::agent::hydration::{
    AgentProcessHydrationRecord, AgentProcessHydrationStatus, AgentReadinessProof,
    FailAgentHydrationCommand, MarkAgentOperationalCommand, StartAgentHydrationCommand,
};
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
const TREE_LEGACY_MIGRATION_RECEIPTS: &str = "agent_legacy_directive_migration_receipts";
const TREE_PROCESS_HYDRATIONS: &str = "agent_process_hydrations";
const TREE_PROCESS_HYDRATIONS_BY_AGENT: &str = "agent_process_hydrations_by_agent";
const TREE_HYDRATION_EPOCHS: &str = "agent_hydration_epochs";
const TREE_CURRENT_HYDRATIONS: &str = "agent_current_hydrations";
const TREE_READINESS_PROOFS: &str = "agent_readiness_proofs";
const RUNTIME_SCHEMA_VERSION_KEY: &[u8] = b"semantic_runtime";
const RUNTIME_SCHEMA_VERSION: u16 = 1;
const KEY_PAD: usize = 20;

#[cfg(test)]
#[derive(Default)]
struct FlushProbe {
    fail_next: bool,
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
    legacy_migration_receipts: Tree,
    process_hydrations: Tree,
    process_hydrations_by_agent: Tree,
    hydration_epochs: Tree,
    current_hydrations: Tree,
    readiness_proofs: Tree,
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
            legacy_migration_receipts: db
                .open_tree(TREE_LEGACY_MIGRATION_RECEIPTS)
                .map_err(to_storage_io)?,
            process_hydrations: db
                .open_tree(TREE_PROCESS_HYDRATIONS)
                .map_err(to_storage_io)?,
            process_hydrations_by_agent: db
                .open_tree(TREE_PROCESS_HYDRATIONS_BY_AGENT)
                .map_err(to_storage_io)?,
            hydration_epochs: db.open_tree(TREE_HYDRATION_EPOCHS).map_err(to_storage_io)?,
            current_hydrations: db
                .open_tree(TREE_CURRENT_HYDRATIONS)
                .map_err(to_storage_io)?,
            readiness_proofs: db.open_tree(TREE_READINESS_PROOFS).map_err(to_storage_io)?,
            #[cfg(test)]
            flush_probe: Arc::new(Mutex::new(FlushProbe::default())),
            db,
        };
        store.migrate_runtime_schema()?;
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
    pub(super) fn start_process_hydration(
        &self,
        command: &StartAgentHydrationCommand,
    ) -> Result<AgentProcessHydrationRecord, StorageError> {
        command
            .validate()
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        let agent = self.get_agent(&command.agent_id)?.ok_or_else(|| {
            StorageError::InvalidPath(format!("unknown agent '{}'", command.agent_id))
        })?;
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
            self.verify_hydration_replay_fences(&agent, &started, &activation)?;
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

        (
            &self.agents,
            &self.hydration_epochs,
            &self.current_hydrations,
            &self.process_hydrations,
            &self.process_hydrations_by_agent,
            &self.activations,
            &self.activations_by_agent,
        )
            .transaction(
                |(
                    agents,
                    epochs,
                    current,
                    hydrations,
                    hydration_index_tree,
                    activations,
                    activation_index_tree,
                )| {
                    require_transaction_value(
                        agents,
                        command.agent_id.as_bytes(),
                        &expected_agent,
                        "agent",
                    )?;
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
                        activations,
                        command.hydration_id.as_bytes(),
                        None,
                        "new activation",
                    )?;
                    hydrations.insert(command.hydration_id.as_bytes(), started_bytes.as_slice())?;
                    hydration_index_tree
                        .insert(hydration_index.as_bytes(), command.hydration_id.as_bytes())?;
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
    pub(super) fn fail_process_hydration(
        &self,
        command: &FailAgentHydrationCommand,
    ) -> Result<AgentProcessHydrationRecord, StorageError> {
        command
            .validate()
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        let (hydration, activation) = self.require_hydration_pair(&command.hydration_id)?;
        if self.current_hydration_epoch(&hydration.agent_id)? != command.attempt_epoch
            || self.current_hydration_id(&hydration.agent_id)?.as_deref()
                != Some(command.hydration_id.as_str())
            || hydration.attempt_epoch != command.attempt_epoch
            || hydration.lease_id != command.lease_id
            || activation.attempt_epoch != command.attempt_epoch
            || activation.lease_id.as_deref() != Some(command.lease_id.as_str())
        {
            return Err(StorageError::Backpressure(
                "hydration failure command lost its epoch or lease fence".to_string(),
            ));
        }
        if hydration.status == AgentProcessHydrationStatus::Failed {
            if hydration.updated_at_seq == command.failed_at_seq
                && hydration.last_error.as_deref() == Some(command.error.as_str())
                && activation.status == AgentActivationStatus::Failed
                && activation.updated_at_seq == command.failed_at_seq
                && activation.last_error.as_deref() == Some(command.error.as_str())
            {
                self.verify_current_hydration_products(&hydration, &activation)?;
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
        let epoch_bytes = encode_epoch(command.attempt_epoch);
        (
            &self.hydration_epochs,
            &self.current_hydrations,
            &self.process_hydrations,
            &self.activations,
        )
            .transaction(|(epochs, current, hydrations, activations)| {
                require_transaction_value(
                    epochs,
                    hydration.agent_id.as_bytes(),
                    &epoch_bytes,
                    "hydration epoch",
                )?;
                require_transaction_value(
                    current,
                    hydration.agent_id.as_bytes(),
                    command.hydration_id.as_bytes(),
                    "current hydration",
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
                hydrations.insert(
                    command.hydration_id.as_bytes(),
                    failed_hydration_bytes.as_slice(),
                )?;
                activations.insert(
                    command.hydration_id.as_bytes(),
                    failed_activation_bytes.as_slice(),
                )?;
                Ok(())
            })
            .map_err(to_agent_transition_error)?;
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

    /// Read one durable operational-readiness proof.
    pub fn get_readiness_proof(
        &self,
        proof_id: &str,
    ) -> Result<Option<AgentReadinessProof>, StorageError> {
        decode_optional(
            self.readiness_proofs
                .get(proof_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Atomically accept one exact readiness proof and mark its agent operational.
    pub(super) fn mark_agent_operational(
        &self,
        command: &MarkAgentOperationalCommand,
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
        if agent.status == AgentStatus::Operational
            && self
                .get_process_hydration(&command.hydration_id)?
                .is_some_and(|hydration| hydration.status == AgentProcessHydrationStatus::Ready)
        {
            return self.validate_operational_replay(command, agent);
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
            &self.subscriptions,
            &self.process_hydrations,
            &self.activations,
            &self.readiness_proofs,
            &self.hydration_epochs,
            &self.current_hydrations,
        )
            .transaction(
                |(
                    agents,
                    status,
                    subscriptions,
                    hydrations,
                    activations,
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
    pub fn put_hydration_checkpoint(
        &self,
        checkpoint: &AgentHydrationCheckpoint,
    ) -> Result<AgentHydrationCheckpoint, StorageError> {
        checkpoint.validate()?;
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
        {
            return Err(StorageError::Backpressure(
                "hydration checkpoint lost its active subscription fence".to_string(),
            ));
        }
        let key = checkpoint.hydration_id.as_bytes();
        let encoded = serde_json::to_vec(checkpoint).map_err(to_storage_data)?;
        let hydration_bytes = serde_json::to_vec(&hydration).map_err(to_storage_data)?;
        let subscription_bytes = serde_json::to_vec(&subscription).map_err(to_storage_data)?;
        (
            &self.process_hydrations,
            &self.subscriptions,
            &self.hydration_checkpoints,
        )
            .transaction(|(hydrations, subscriptions, checkpoints)| {
                require_transaction_value(hydrations, key, &hydration_bytes, "process hydration")?;
                require_transaction_value(
                    subscriptions,
                    checkpoint.subscription_id.as_bytes(),
                    &subscription_bytes,
                    "hydration subscription",
                )?;
                insert_exact_transaction_value(checkpoints, key, &encoded, "hydration checkpoint")
            })
            .map_err(to_agent_transition_error)?;
        self.flush_durable("hydration checkpoint creation")?;
        Ok(checkpoint.clone())
    }

    /// Advance one hydration checkpoint under exact stage and lease fencing.
    pub fn advance_hydration_checkpoint_cas(
        &self,
        expected: &AgentHydrationCheckpoint,
        advanced: &AgentHydrationCheckpoint,
    ) -> Result<AgentHydrationCheckpoint, StorageError> {
        expected.validate()?;
        advanced.validate()?;
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
        {
            return Err(StorageError::Backpressure(
                "hydration checkpoint lost its active subscription fence".to_string(),
            ));
        }
        let key = expected.hydration_id.as_bytes();
        let expected_bytes = serde_json::to_vec(expected).map_err(to_storage_data)?;
        let advanced_bytes = serde_json::to_vec(advanced).map_err(to_storage_data)?;
        let hydration_bytes = serde_json::to_vec(&hydration).map_err(to_storage_data)?;
        let subscription_bytes = serde_json::to_vec(&subscription).map_err(to_storage_data)?;
        (
            &self.process_hydrations,
            &self.subscriptions,
            &self.hydration_checkpoints,
        )
            .transaction(|(hydrations, subscriptions, checkpoints)| {
                require_transaction_value(hydrations, key, &hydration_bytes, "process hydration")?;
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
            })
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

        repair_exact_tree_value(
            &self.runtime_schema,
            RUNTIME_SCHEMA_VERSION_KEY,
            &RUNTIME_SCHEMA_VERSION.to_be_bytes(),
            "agent runtime schema version",
        )?;
        self.flush_durable("agent runtime schema migration")
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
        Ok((hydration, activation))
    }

    fn verify_hydration_replay_fences(
        &self,
        agent: &AgentRecord,
        hydration: &AgentProcessHydrationRecord,
        activation: &AgentActivationRecord,
    ) -> Result<(), StorageError> {
        let agent_bytes = serde_json::to_vec(agent).map_err(to_storage_data)?;
        let hydration_bytes = serde_json::to_vec(hydration).map_err(to_storage_data)?;
        let activation_bytes = serde_json::to_vec(activation).map_err(to_storage_data)?;
        let epoch_bytes = encode_epoch(hydration.attempt_epoch);
        (
            &self.agents,
            &self.hydration_epochs,
            &self.current_hydrations,
            &self.process_hydrations,
            &self.activations,
        )
            .transaction(|(agents, epochs, current, hydrations, activations)| {
                require_transaction_value(
                    agents,
                    agent.agent_id.as_bytes(),
                    &agent_bytes,
                    "agent",
                )?;
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
                    activations,
                    activation.activation_id.as_bytes(),
                    &activation_bytes,
                    "activation",
                )?;
                Ok(())
            })
            .map_err(to_agent_transition_error)
    }

    fn verify_current_hydration_products(
        &self,
        hydration: &AgentProcessHydrationRecord,
        activation: &AgentActivationRecord,
    ) -> Result<(), StorageError> {
        let hydration_bytes = serde_json::to_vec(hydration).map_err(to_storage_data)?;
        let activation_bytes = serde_json::to_vec(activation).map_err(to_storage_data)?;
        let epoch_bytes = encode_epoch(hydration.attempt_epoch);
        (
            &self.hydration_epochs,
            &self.current_hydrations,
            &self.process_hydrations,
            &self.activations,
        )
            .transaction(|(epochs, current, hydrations, activations)| {
                require_transaction_value(
                    epochs,
                    hydration.agent_id.as_bytes(),
                    &epoch_bytes,
                    "hydration epoch",
                )?;
                require_transaction_value(
                    current,
                    hydration.agent_id.as_bytes(),
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
                    activations,
                    activation.activation_id.as_bytes(),
                    &activation_bytes,
                    "activation",
                )?;
                Ok(())
            })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belief::BranchScope;
    use crate::events::DomainObjectRef;
    use crate::world_state::graph::PerspectiveKey;

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
}
