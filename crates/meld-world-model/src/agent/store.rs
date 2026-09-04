//! Durable agent storage.

use std::io;
use std::sync::Arc;

use sled::{Db, Tree};

use crate::agent::contracts::{
    AgentActivationRecord, AgentConsumerReceipt, AgentExecutionReceipt, AgentMilestoneAcceptance,
    AgentPlanJudgment, AgentProductAuthorization, AgentProductProgress, AgentReconciliationGoal,
    AgentRecord, AgentStatus, AgentSubscriptionRecord, AgentSubscriptionStatus,
    LegacyAgentDecisionRecord, LegacyAgentSinkReceiptRecord,
};
use crate::error::StorageError;

const TREE_AGENT_RECORDS: &str = "agent_records";
const TREE_AGENT_BY_STATUS: &str = "agent_by_status";
const TREE_SUBSCRIPTIONS: &str = "agent_subscriptions";
const TREE_SUBSCRIPTIONS_BY_AGENT: &str = "agent_subscriptions_by_agent";
const TREE_SUBSCRIPTIONS_BY_KEY: &str = "agent_subscriptions_by_key";
const TREE_ACTIVATIONS: &str = "agent_activations";
const TREE_ACTIVATIONS_BY_AGENT: &str = "agent_activations_by_agent";
const TREE_LEGACY_DECISIONS: &str = "agent_curation_decisions";
const TREE_LEGACY_SINK_RECEIPTS: &str = "agent_sink_receipts";
const TREE_LEGACY_SINK_RECEIPTS_BY_COMMAND: &str = "agent_sink_receipts_by_command";
const TREE_RECONCILIATION_GOALS: &str = "agent_reconciliation_goals";
const TREE_RECONCILIATION_PLANS: &str = "agent_reconciliation_plans";
const TREE_RECONCILIATION_JUDGMENTS: &str = "agent_reconciliation_plan_judgments";
const TREE_RECONCILIATION_PROGRESS: &str = "agent_reconciliation_product_progress";
const TREE_RECONCILIATION_RECEIPTS: &str = "agent_reconciliation_consumer_receipts";
const TREE_RECONCILIATION_MILESTONES: &str = "agent_reconciliation_milestones";
const KEY_PAD: usize = 20;

/// Sled backed storage for durable agent records and indexes.
#[derive(Clone)]
#[allow(dead_code)]
pub struct AgentStore {
    db: Db,
    agents: Tree,
    agent_by_status: Tree,
    subscriptions: Tree,
    subscriptions_by_agent: Tree,
    subscriptions_by_key: Tree,
    activations: Tree,
    activations_by_agent: Tree,
    legacy_decisions: Tree,
    legacy_sink_receipts: Tree,
    legacy_sink_receipts_by_command: Tree,
    reconciliation_goals: Tree,
    reconciliation_plans: Tree,
    reconciliation_judgments: Tree,
    reconciliation_progress: Tree,
    reconciliation_receipts: Tree,
    reconciliation_milestones: Tree,
}

#[allow(dead_code)]
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
            legacy_decisions: db.open_tree(TREE_LEGACY_DECISIONS).map_err(to_storage_io)?,
            legacy_sink_receipts: db
                .open_tree(TREE_LEGACY_SINK_RECEIPTS)
                .map_err(to_storage_io)?,
            legacy_sink_receipts_by_command: db
                .open_tree(TREE_LEGACY_SINK_RECEIPTS_BY_COMMAND)
                .map_err(to_storage_io)?,
            reconciliation_goals: db
                .open_tree(TREE_RECONCILIATION_GOALS)
                .map_err(to_storage_io)?,
            reconciliation_plans: db
                .open_tree(TREE_RECONCILIATION_PLANS)
                .map_err(to_storage_io)?,
            reconciliation_judgments: db
                .open_tree(TREE_RECONCILIATION_JUDGMENTS)
                .map_err(to_storage_io)?,
            reconciliation_progress: db
                .open_tree(TREE_RECONCILIATION_PROGRESS)
                .map_err(to_storage_io)?,
            reconciliation_receipts: db
                .open_tree(TREE_RECONCILIATION_RECEIPTS)
                .map_err(to_storage_io)?,
            reconciliation_milestones: db
                .open_tree(TREE_RECONCILIATION_MILESTONES)
                .map_err(to_storage_io)?,
            db,
        })
    }

    /// Persist one Agent-owned Goal without rewriting an existing identity.
    pub fn put_reconciliation_goal(
        &self,
        record: &AgentReconciliationGoal,
    ) -> Result<bool, StorageError> {
        put_immutable(&self.reconciliation_goals, &record.goal.goal_id, record)
    }

    pub fn reconciliation_goal(
        &self,
        goal_id: &str,
    ) -> Result<Option<AgentReconciliationGoal>, StorageError> {
        get_immutable(&self.reconciliation_goals, goal_id)
    }

    /// Persist one immutable Strategy Plan body as Agent history.
    pub fn put_reconciliation_plan(
        &self,
        plan: &crate::strategy::StrategyPlan,
    ) -> Result<bool, StorageError> {
        put_immutable(&self.reconciliation_plans, &plan.plan_revision_id, plan)
    }

    pub fn reconciliation_plan(
        &self,
        plan_id: &str,
    ) -> Result<Option<crate::strategy::StrategyPlan>, StorageError> {
        get_immutable(&self.reconciliation_plans, plan_id)
    }

    pub fn put_plan_judgment(&self, record: &AgentPlanJudgment) -> Result<bool, StorageError> {
        put_immutable(&self.reconciliation_judgments, &record.judgment_id, record)
    }

    pub fn plan_judgment(
        &self,
        judgment_id: &str,
    ) -> Result<Option<AgentPlanJudgment>, StorageError> {
        get_immutable(&self.reconciliation_judgments, judgment_id)
    }

    pub fn put_product_progress(
        &self,
        record: &AgentProductProgress,
    ) -> Result<bool, StorageError> {
        put_immutable(
            &self.reconciliation_progress,
            &format!("progress::{}", record.progress_id),
            record,
        )
    }

    pub fn product_progress(
        &self,
        progress_id: &str,
    ) -> Result<Option<AgentProductProgress>, StorageError> {
        get_immutable(
            &self.reconciliation_progress,
            &format!("progress::{progress_id}"),
        )
    }

    pub fn put_product_authorization(
        &self,
        record: &AgentProductAuthorization,
    ) -> Result<bool, StorageError> {
        put_immutable(
            &self.reconciliation_progress,
            &format!("authorization::{}", record.authorization_id),
            record,
        )
    }

    pub fn product_authorization(
        &self,
        authorization_id: &str,
    ) -> Result<Option<AgentProductAuthorization>, StorageError> {
        get_immutable(
            &self.reconciliation_progress,
            &format!("authorization::{authorization_id}"),
        )
    }

    /// Read immutable product authorizations retained for one Agent Goal.
    pub fn product_authorizations_for_goal(
        &self,
        goal_id: &str,
    ) -> Result<Vec<AgentProductAuthorization>, StorageError> {
        let mut records = Vec::new();
        for row in self.reconciliation_progress.scan_prefix(b"authorization::") {
            let (_, raw) = row.map_err(to_storage_io)?;
            let record: AgentProductAuthorization =
                serde_json::from_slice(&raw).map_err(to_storage_data)?;
            if record.goal_id == goal_id {
                records.push(record);
            }
        }
        records.sort_by(|left, right| left.authorization_id.cmp(&right.authorization_id));
        Ok(records)
    }

    pub fn put_consumer_receipt(
        &self,
        record: &AgentConsumerReceipt,
    ) -> Result<bool, StorageError> {
        put_immutable(&self.reconciliation_receipts, &record.receipt_id, record)
    }

    pub fn consumer_receipt(
        &self,
        receipt_id: &str,
    ) -> Result<Option<AgentConsumerReceipt>, StorageError> {
        get_immutable(&self.reconciliation_receipts, receipt_id)
    }

    pub fn put_execution_receipt(
        &self,
        record: &AgentExecutionReceipt,
    ) -> Result<bool, StorageError> {
        put_immutable(
            &self.reconciliation_receipts,
            &format!("execution::{}", record.receipt_id),
            record,
        )
    }

    pub fn execution_receipt(
        &self,
        receipt_id: &str,
    ) -> Result<Option<AgentExecutionReceipt>, StorageError> {
        get_immutable(
            &self.reconciliation_receipts,
            &format!("execution::{receipt_id}"),
        )
    }

    pub fn put_milestone(&self, record: &AgentMilestoneAcceptance) -> Result<bool, StorageError> {
        put_immutable(
            &self.reconciliation_milestones,
            &record.milestone_id,
            record,
        )
    }

    pub fn milestone(
        &self,
        milestone_id: &str,
    ) -> Result<Option<AgentMilestoneAcceptance>, StorageError> {
        get_immutable(&self.reconciliation_milestones, milestone_id)
    }

    /// Monotonic durable position across the append-only reconciliation family.
    pub fn reconciliation_position(&self) -> u64 {
        [
            &self.reconciliation_goals,
            &self.reconciliation_plans,
            &self.reconciliation_judgments,
            &self.reconciliation_progress,
            &self.reconciliation_receipts,
            &self.reconciliation_milestones,
        ]
        .into_iter()
        .map(|tree| tree.len() as u64)
        .sum()
    }

    /// Open the store behind a shared pointer for facade wiring.
    pub fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        Ok(Arc::new(Self::new(db)?))
    }

    /// Write an agent record and its status index.
    pub fn put_agent(&self, record: &AgentRecord) -> Result<(), StorageError> {
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
        decode_optional(
            self.agents
                .get(agent_id.as_bytes())
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
    pub fn put_subscription(&self, record: &AgentSubscriptionRecord) -> Result<(), StorageError> {
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

    /// Write an activation record and its agent index.
    pub fn put_activation(&self, record: &AgentActivationRecord) -> Result<(), StorageError> {
        record.validate()?;
        self.activations
            .insert(
                record.activation_id.as_bytes(),
                serde_json::to_vec(record).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.activations_by_agent
            .insert(
                activation_agent_key(
                    &record.agent_id,
                    record.started_at_seq,
                    &record.activation_id,
                )
                .as_bytes(),
                record.activation_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
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

    /// Read one historical pre-cutover decision without exposing a writer.
    pub fn legacy_decision(
        &self,
        decision_id: &str,
    ) -> Result<Option<LegacyAgentDecisionRecord>, StorageError> {
        decode_optional(
            self.legacy_decisions
                .get(decision_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Read historical receipts for one pre-cutover command without exposing a writer.
    pub fn legacy_sink_receipts_by_command(
        &self,
        command_id: &str,
    ) -> Result<Vec<LegacyAgentSinkReceiptRecord>, StorageError> {
        let prefix = format!("{command_id}::");
        let mut out: Vec<LegacyAgentSinkReceiptRecord> = Vec::new();
        for item in self
            .legacy_sink_receipts_by_command
            .scan_prefix(prefix.as_bytes())
        {
            let (_, value) = item.map_err(to_storage_io)?;
            let decision_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(receipt) = decode_optional(
                self.legacy_sink_receipts
                    .get(decision_id.as_bytes())
                    .map_err(to_storage_io)?,
            )? {
                out.push(receipt);
            }
        }
        out.sort_by(|left, right| left.decision_id.cmp(&right.decision_id));
        Ok(out)
    }

    /// Flush all sled writes for this store.
    pub fn flush(&self) -> Result<(), StorageError> {
        self.db.flush().map_err(to_storage_io)?;
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

fn decode_optional<T: serde::de::DeserializeOwned>(
    raw: Option<sled::IVec>,
) -> Result<Option<T>, StorageError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&raw).map_err(to_storage_data)?))
}

fn put_immutable<T>(tree: &Tree, key: &str, value: &T) -> Result<bool, StorageError>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    if let Some(raw) = tree.get(key.as_bytes()).map_err(to_storage_io)? {
        let existing: T = serde_json::from_slice(&raw).map_err(to_storage_data)?;
        if existing != *value {
            return Err(StorageError::InvalidPath(format!(
                "Agent reconciliation identity '{key}' has divergent content"
            )));
        }
        return Ok(false);
    }
    tree.insert(
        key.as_bytes(),
        serde_json::to_vec(value).map_err(to_storage_data)?,
    )
    .map_err(to_storage_io)?;
    tree.flush().map_err(to_storage_io)?;
    Ok(true)
}

fn get_immutable<T>(tree: &Tree, key: &str) -> Result<Option<T>, StorageError>
where
    T: serde::de::DeserializeOwned,
{
    decode_optional(tree.get(key.as_bytes()).map_err(to_storage_io)?)
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
