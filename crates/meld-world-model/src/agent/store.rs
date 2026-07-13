//! Durable agent storage.

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
    AgentActivationRecord, AgentActivationStatus, AgentCurationDecision, AgentCurationDedupeKey,
    AgentRecord, AgentSatisfactionReview, AgentSinkReceipt, AgentStatus,
    AgentSubscriptionCursorCasIntent, AgentSubscriptionRecord, AgentSubscriptionStatus,
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
        Ok(Self {
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
        })
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
        for item in self.agent_by_status.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.map_err(to_storage_io)?;
            let agent_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(agent) = self.get_agent(&agent_id)? {
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
            || subscription.last_delivered_revision_id.as_deref()
                != Some(attestation.belief_revision_id.as_str())
            || subscription.last_delivered_seq != attestation.attested_at_seq
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
                    if let Some(existing) = proofs.get(command.readiness.proof_id.as_bytes())? {
                        if existing.as_ref() != proof_bytes.as_slice() {
                            return Err(ConflictableTransactionError::Abort(format!(
                                "readiness proof '{}' conflicts with durable state",
                                command.readiness.proof_id
                            )));
                        }
                    }
                    agents.insert(agent_id.as_bytes(), operational_bytes.clone())?;
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

    /// Persist a curation decision unless the dedupe and revision key already exists.
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
            self.index_satisfaction_dedupe(review, decision)?;
            self.index_satisfaction_review(review, &existing.decision_id)?;
            return Ok(existing);
        }
        let dedupe_index_key =
            decision_dedupe_satisfaction_review_key(&decision.dedupe_key, review);
        let persisted = self.insert_decision_record(decision, &dedupe_index_key)?;
        self.index_satisfaction_review(review, &persisted.decision_id)?;
        Ok(persisted)
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

    /// Persist a sink receipt unless the decision already has one.
    pub fn put_sink_receipt(
        &self,
        receipt: &AgentSinkReceipt,
    ) -> Result<AgentSinkReceipt, StorageError> {
        receipt.validate()?;
        if let Some(existing) = self.sink_receipt_by_decision(&receipt.decision_id)? {
            if existing.submission.command_id != receipt.submission.command_id
                || existing.submission.goal_id != receipt.submission.goal_id
            {
                return Err(StorageError::InvalidPath(
                    "sink receipt decision conflict".to_string(),
                ));
            }
            return Ok(existing);
        }
        self.sink_receipts
            .insert(
                receipt.decision_id.as_bytes(),
                serde_json::to_vec(receipt).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.sink_receipts_by_command
            .insert(
                sink_receipt_command_key(&receipt.submission.command_id, &receipt.decision_id)
                    .as_bytes(),
                receipt.decision_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
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
        self.decisions
            .insert(
                decision.decision_id.as_bytes(),
                serde_json::to_vec(decision).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.decisions_by_agent
            .insert(
                decision_agent_key(
                    &decision.agent_id,
                    decision.created_at_seq,
                    &decision.decision_id,
                )
                .as_bytes(),
                decision.decision_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        self.decisions_by_dedupe
            .insert(dedupe_index_key.as_bytes(), decision.decision_id.as_bytes())
            .map_err(to_storage_io)?;
        if let Some(revision_id) = &decision.input_refs.belief_revision_id {
            self.decisions_by_revision
                .insert(
                    decision_revision_key(revision_id, &decision.decision_id).as_bytes(),
                    decision.decision_id.as_bytes(),
                )
                .map_err(to_storage_io)?;
        }
        Ok(decision.clone())
    }

    fn index_satisfaction_review(
        &self,
        review: &AgentSatisfactionReview,
        decision_id: &str,
    ) -> Result<(), StorageError> {
        self.satisfaction_decisions_by_review
            .insert(review.index_key().as_bytes(), decision_id.as_bytes())
            .map_err(to_storage_io)?;
        Ok(())
    }

    fn index_satisfaction_dedupe(
        &self,
        review: &AgentSatisfactionReview,
        decision: &AgentCurationDecision,
    ) -> Result<(), StorageError> {
        self.decisions_by_dedupe
            .insert(
                decision_dedupe_satisfaction_review_key(&decision.dedupe_key, review).as_bytes(),
                decision.decision_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
    }
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

fn require_optional_transaction_value(
    tree: &sled::transaction::TransactionalTree,
    key: &[u8],
    expected: Option<&[u8]>,
    product: &str,
) -> Result<(), ConflictableTransactionError<String>> {
    if tree.get(key)?.as_deref() != expected {
        return Err(ConflictableTransactionError::Abort(format!(
            "{product} changed during hydration transition"
        )));
    }
    Ok(())
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
