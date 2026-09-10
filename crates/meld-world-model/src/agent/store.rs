//! Durable agent storage.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::sync::Arc;

use sled::transaction::{ConflictableTransactionError, TransactionError, Transactional};
use sled::{Db, Tree};

use crate::agent::contracts::{
    AgentActivationRecord, AgentConsumerReceipt, AgentExecutionReceipt, AgentMilestoneAcceptance,
    AgentPlanJudgment, AgentProductAuthorization, AgentProductProgress, AgentReconciliationGoal,
    AgentRecord, AgentStatus, AgentSubscriptionRecord, AgentSubscriptionStatus,
    LegacyAgentDecisionRecord, LegacyAgentSinkReceiptRecord,
};
use crate::agent::genesis::{
    AgentGenesisIntentV1, AgentGenesisPublicationV1, AgentGenesisReceiptV1,
    AgentSubscriptionRequestV1,
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
const TREE_RECONCILIATION_REQUESTS: &str = "agent_reconciliation_requests_v1";
const TREE_RECONCILIATION_PLANS: &str = "agent_reconciliation_plans";
const TREE_RECONCILIATION_JUDGMENTS: &str = "agent_reconciliation_plan_judgments";
const TREE_RECONCILIATION_PROGRESS: &str = "agent_reconciliation_product_progress";
const TREE_RECONCILIATION_RECEIPTS: &str = "agent_reconciliation_consumer_receipts";
const TREE_RECONCILIATION_MILESTONES: &str = "agent_reconciliation_milestones";
const TREE_GENESIS_INTENTS: &str = "agent_genesis_intents_v1";
const TREE_GENESIS_SUBSCRIPTION_REQUESTS: &str = "agent_genesis_subscription_requests_v1";
const TREE_GENESIS_PUBLICATIONS: &str = "agent_genesis_publications_v1";
const TREE_GENESIS_RECEIPTS: &str = "agent_genesis_receipts_v1";
const TREE_GENESIS_BY_AGENT: &str = "agent_genesis_by_agent_v1";
const KEY_PAD: usize = 20;

/// Sled backed storage for durable agent records and indexes.
#[derive(Clone)]
#[allow(dead_code)]
pub struct AgentStore {
    resource_id: String,
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
    reconciliation_requests: Tree,
    reconciliation_plans: Tree,
    reconciliation_judgments: Tree,
    reconciliation_progress: Tree,
    reconciliation_receipts: Tree,
    reconciliation_milestones: Tree,
    genesis_intents: Tree,
    genesis_subscription_requests: Tree,
    genesis_publications: Tree,
    genesis_receipts: Tree,
    genesis_by_agent: Tree,
}

#[allow(dead_code)]
impl AgentStore {
    /// Exact durable world-model resource bound to this store instance.
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    /// Open all agent trees from the shared world model database.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        Ok(Self {
            resource_id: crate::waiting::resource_identity(&db)?,
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
            reconciliation_requests: db
                .open_tree(TREE_RECONCILIATION_REQUESTS)
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
            genesis_intents: db.open_tree(TREE_GENESIS_INTENTS).map_err(to_storage_io)?,
            genesis_subscription_requests: db
                .open_tree(TREE_GENESIS_SUBSCRIPTION_REQUESTS)
                .map_err(to_storage_io)?,
            genesis_publications: db
                .open_tree(TREE_GENESIS_PUBLICATIONS)
                .map_err(to_storage_io)?,
            genesis_receipts: db.open_tree(TREE_GENESIS_RECEIPTS).map_err(to_storage_io)?,
            genesis_by_agent: db.open_tree(TREE_GENESIS_BY_AGENT).map_err(to_storage_io)?,
            db,
        })
    }

    pub(crate) fn claim_genesis_lineage(
        &self,
        agent_id: &str,
        intent_id: &str,
    ) -> Result<bool, StorageError> {
        match self
            .genesis_by_agent
            .compare_and_swap(
                agent_id.as_bytes(),
                None as Option<&[u8]>,
                Some(intent_id.as_bytes()),
            )
            .map_err(to_storage_io)?
        {
            Ok(()) => {
                self.genesis_by_agent.flush().map_err(to_storage_io)?;
                Ok(true)
            }
            Err(conflict) if conflict.current.as_deref() == Some(intent_id.as_bytes()) => Ok(false),
            Err(_) => Err(StorageError::InvalidPath(format!(
                "Agent '{agent_id}' already belongs to a different genesis lineage"
            ))),
        }
    }

    pub(crate) fn put_genesis_intent(
        &self,
        intent: &AgentGenesisIntentV1,
    ) -> Result<bool, StorageError> {
        put_immutable(&self.genesis_intents, &intent.intent_id, intent)
    }

    /// Resolve this Agent's canonical genesis, without choosing among assignments.
    pub fn genesis_intent_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentGenesisIntentV1>, StorageError> {
        let Some(id) = self.genesis_by_agent.get(agent_id).map_err(to_storage_io)? else {
            return Ok(None);
        };
        let id = std::str::from_utf8(&id)
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        get_immutable(&self.genesis_intents, id)
    }

    /// Read native genesis identities in Agent-id order without consulting profiles.
    pub fn list_genesis_intents(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<AgentGenesisIntentV1>, StorageError> {
        self.genesis_by_agent
            .iter()
            .filter_map(|entry| match entry {
                Ok((key, _)) if after.is_some_and(|after| key.as_ref() <= after.as_bytes()) => None,
                other => Some(other),
            })
            .take(limit.min(1001))
            .map(|entry| {
                let (key, _) = entry.map_err(to_storage_io)?;
                let id = std::str::from_utf8(&key)
                    .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
                self.genesis_intent_for_agent(id)?.ok_or_else(|| {
                    StorageError::InvalidPath("native Agent genesis index is unresolved".into())
                })
            })
            .collect()
    }

    pub(crate) fn put_epoch_products(
        &self,
        products: &crate::agent::AgentEpochProducts,
    ) -> Result<bool, StorageError> {
        products.validate()?;
        put_immutable(
            &self.reconciliation_plans,
            &format!("epoch-products::{}", products.specification.goal_id),
            products,
        )
    }

    /// Durably request another reconciliation of this Agent's installed intent.
    /// Intake does not judge a Goal or confer current execution authority.
    pub fn request_reconciliation(
        &self,
        agent_id: &str,
        request_key: impl Into<String>,
    ) -> Result<super::AgentReconciliationRequest, StorageError> {
        let genesis = self.genesis_intent_for_agent(agent_id)?.ok_or_else(|| {
            StorageError::InvalidPath("reconciliation request has no native Agent genesis".into())
        })?;
        let request = super::AgentReconciliationRequest::new(&genesis, request_key.into())?;
        put_immutable(&self.reconciliation_requests, &request.request_id, &request)?;
        Ok(request)
    }

    pub fn reconciliation_requests(
        &self,
        agent_id: &str,
    ) -> Result<Vec<super::AgentReconciliationRequest>, StorageError> {
        let Some(genesis) = self.genesis_intent_for_agent(agent_id)? else {
            return Ok(Vec::new());
        };
        let mut requests = Vec::new();
        for entry in self.reconciliation_requests.iter() {
            let (_, bytes) = entry.map_err(to_storage_io)?;
            let request: super::AgentReconciliationRequest =
                serde_json::from_slice(&bytes).map_err(to_storage_data)?;
            if request.agent_id == agent_id {
                request.validate_for(&genesis)?;
                requests.push(request);
            }
        }
        Ok(requests)
    }

    /// Completion names a native condition or Goal judgment, never Task success.
    pub fn reconciliation_request_completed(
        &self,
        request: &super::AgentReconciliationRequest,
    ) -> Result<bool, StorageError> {
        let genesis = self
            .genesis_intent_for_agent(&request.agent_id)?
            .ok_or_else(|| {
                StorageError::InvalidPath("request completion has no native genesis".into())
            })?;
        request.validate_for(&genesis)?;
        let intent = super::AgentReconciliationIntent::MaintainedCondition(
            genesis.registration.maintained_condition.unwrap(),
        );
        let goal_id = request.goal_id(&intent);
        if let Some(plan) = self.current_reconciliation_plan(&goal_id)? {
            return Ok(self
                .goal_disposition_for_plan(&plan.plan_revision_id)?
                .is_some_and(|disposition| {
                    matches!(
                        disposition.lifecycle,
                        meld_lang::GoalLifecycle::Satisfied { .. }
                    )
                }));
        }
        for judgment in self.condition_judgments()? {
            if judgment.agent_id == request.agent_id
                && judgment.evaluation == meld_lang::EvalResult::Satisfied
                && self
                    .reconciliation_cut(&judgment.planner_cut_id)?
                    .is_some_and(|cut| cut.context.goal_id == goal_id)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn epoch_products_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Vec<super::AgentEpochProducts>, StorageError> {
        let mut products = Vec::new();
        for entry in self.reconciliation_plans.scan_prefix("epoch-products::") {
            let (_, bytes) = entry.map_err(to_storage_io)?;
            let product: super::AgentEpochProducts =
                serde_json::from_slice(&bytes).map_err(to_storage_data)?;
            if product.specification.authority.agent_id == agent_id {
                product.validate()?;
                products.push(product);
            }
        }
        Ok(products)
    }

    /// Intake source position advances independently of Goal and Plan progress.
    pub fn reconciliation_request_position(&self) -> u64 {
        self.reconciliation_requests.len() as u64
    }

    pub fn epoch_subscription_receipt(
        &self,
        request_id: &str,
    ) -> Result<Option<crate::agent::AgentEpochSubscriptionReceipt>, StorageError> {
        get_immutable(
            &self.reconciliation_receipts,
            &format!("subscription::{request_id}"),
        )
    }

    pub(crate) fn receive_epoch_subscription(
        &self,
        request: &crate::agent::AgentSubscriptionRequestV1,
        proof: &crate::belief::BeliefSubscriptionAcceptanceProof,
    ) -> Result<bool, StorageError> {
        request.validate()?;
        if proof.request_id() != request.request_id {
            return Err(StorageError::InvalidPath(
                "Belief acceptance names another subscription request".into(),
            ));
        }
        self.put_subscription_request(request)?;
        super::subscription::AgentSubscription::new(self).subscribe(
            super::SubscribeAgentCommand {
                agent_id: request.agent_id.clone(),
                belief_key: request.belief_key.clone(),
                created_at_seq: self.reconciliation_position(),
            },
        )?;
        put_immutable(
            &self.reconciliation_receipts,
            &format!("subscription::{}", request.request_id),
            &crate::agent::AgentEpochSubscriptionReceipt {
                request_id: request.request_id.clone(),
                acceptance_id: proof.acceptance_id().into(),
                agent_id: request.agent_id.clone(),
                belief_key: request.belief_key.clone(),
            },
        )
    }

    /// Retain exact owner inputs across restart and later admission epochs.
    pub fn epoch_products(
        &self,
        goal_id: &str,
    ) -> Result<Option<crate::agent::AgentEpochProducts>, StorageError> {
        let products: Option<crate::agent::AgentEpochProducts> = get_immutable(
            &self.reconciliation_plans,
            &format!("epoch-products::{goal_id}"),
        )?;
        if let Some(products) = &products {
            products.validate()?;
        }
        Ok(products)
    }

    pub(crate) fn put_subscription_request(
        &self,
        request: &AgentSubscriptionRequestV1,
    ) -> Result<bool, StorageError> {
        put_immutable(
            &self.genesis_subscription_requests,
            &request.request_id,
            request,
        )
    }

    pub(crate) fn put_genesis_publication(
        &self,
        publication: &AgentGenesisPublicationV1,
    ) -> Result<bool, StorageError> {
        put_immutable(
            &self.genesis_publications,
            &publication.publication_id,
            publication,
        )
    }

    pub(crate) fn put_genesis_receipt(
        &self,
        receipt: &AgentGenesisReceiptV1,
    ) -> Result<bool, StorageError> {
        put_immutable(&self.genesis_receipts, &receipt.genesis_receipt_id, receipt)
    }

    pub fn genesis_receipt(
        &self,
        receipt_id: &str,
    ) -> Result<Option<AgentGenesisReceiptV1>, StorageError> {
        get_immutable(&self.genesis_receipts, receipt_id)
    }

    pub fn genesis_receipts_for_assignment(
        &self,
        assignment_id: &str,
    ) -> Result<Vec<AgentGenesisReceiptV1>, StorageError> {
        let mut receipts = Vec::new();
        for row in &self.genesis_receipts {
            let (_, raw) = row.map_err(to_storage_io)?;
            let receipt: AgentGenesisReceiptV1 =
                serde_json::from_slice(&raw).map_err(to_storage_data)?;
            if receipt.assignment_id == assignment_id {
                receipts.push(receipt);
            }
        }
        receipts.sort_by(|left, right| left.topology_position_id.cmp(&right.topology_position_id));
        Ok(receipts)
    }

    /// Persist one Agent-owned Goal without rewriting an existing identity.
    pub fn put_reconciliation_goal(
        &self,
        record: &AgentReconciliationGoal,
    ) -> Result<bool, StorageError> {
        put_immutable(&self.reconciliation_goals, &record.goal.goal_id, record)
    }

    /// Retain every transient Goal owned by this Agent, including closed epochs.
    pub fn reconciliation_goals_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Vec<AgentReconciliationGoal>, StorageError> {
        let mut goals = Vec::new();
        for row in &self.reconciliation_goals {
            let (key, raw) = row.map_err(to_storage_io)?;
            if key.starts_with(b"disposition::") {
                continue;
            }
            let goal: AgentReconciliationGoal =
                serde_json::from_slice(&raw).map_err(to_storage_data)?;
            if goal.goal.agent_id == agent_id {
                goals.push(goal);
            }
        }
        Ok(goals)
    }

    pub fn reconciliation_goal(
        &self,
        goal_id: &str,
    ) -> Result<Option<AgentReconciliationGoal>, StorageError> {
        get_immutable(&self.reconciliation_goals, goal_id)
    }

    pub(crate) fn put_condition_judgment(
        &self,
        judgment: &crate::agent::AgentConditionJudgment,
    ) -> Result<bool, StorageError> {
        put_immutable(
            &self.reconciliation_judgments,
            &format!("condition::{}", judgment.judgment_id),
            judgment,
        )
    }

    pub fn condition_judgments(
        &self,
    ) -> Result<Vec<crate::agent::AgentConditionJudgment>, StorageError> {
        self.reconciliation_judgments
            .scan_prefix("condition::")
            .map(|entry| {
                let (_, value) = entry.map_err(to_storage_io)?;
                serde_json::from_slice(&value).map_err(to_storage_data)
            })
            .collect()
    }

    /// Persist a separate Agent decision about the Goal's desired state.
    pub(crate) fn put_goal_disposition(
        &self,
        disposition: &crate::agent::AgentGoalDisposition,
    ) -> Result<bool, StorageError> {
        put_immutable(
            &self.reconciliation_goals,
            &format!("disposition::{}", disposition.plan_revision_id),
            disposition,
        )
    }

    pub fn goal_disposition_for_plan(
        &self,
        plan_id: &str,
    ) -> Result<Option<crate::agent::AgentGoalDisposition>, StorageError> {
        get_immutable(
            &self.reconciliation_goals,
            &format!("disposition::{plan_id}"),
        )
    }

    /// Retain the exact admitted cut for return attribution after an epoch closes.
    pub(crate) fn put_reconciliation_cut(
        &self,
        cut: &crate::planner::PlannerCut,
    ) -> Result<bool, StorageError> {
        put_immutable(
            &self.reconciliation_plans,
            &format!("cut::{}", cut.cut_id),
            cut,
        )
    }

    /// Read the immutable native cut retained for a judgment or return attribution.
    pub fn reconciliation_cut(
        &self,
        cut_id: &str,
    ) -> Result<Option<crate::planner::PlannerCut>, StorageError> {
        get_immutable(&self.reconciliation_plans, &format!("cut::{cut_id}"))
    }

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

    /// Resolve the admitted leaf of a Goal's immutable Plan lineage.
    /// Hash ordering and historical product authorizations never select the current Plan.
    pub fn current_reconciliation_plan(
        &self,
        goal_id: &str,
    ) -> Result<Option<crate::strategy::StrategyPlan>, StorageError> {
        let mut admitted = BTreeMap::new();
        let mut superseded = BTreeSet::new();
        for row in &self.reconciliation_judgments {
            let (key, raw) = row.map_err(to_storage_io)?;
            if key.starts_with(b"condition::") {
                continue;
            }
            let judgment: AgentPlanJudgment =
                serde_json::from_slice(&raw).map_err(to_storage_data)?;
            if judgment.goal_id != goal_id {
                continue;
            }
            match judgment.kind {
                crate::agent::AgentPlanJudgmentKind::Admitted => {
                    admitted.insert(judgment.plan_revision_id.clone(), judgment);
                }
                crate::agent::AgentPlanJudgmentKind::Superseded { .. } => {
                    superseded.insert(judgment.plan_revision_id);
                }
                crate::agent::AgentPlanJudgmentKind::Rejected { .. } => {}
            }
        }
        admitted.retain(|id, _| !superseded.contains(id));
        if admitted.len() > 1 {
            return Err(StorageError::InvalidPath(
                "Agent Goal has conflicting admitted Plan heads".to_string(),
            ));
        }
        let Some((id, _)) = admitted.into_iter().next() else {
            return Ok(None);
        };
        self.reconciliation_plan(&id)?
            .map(Some)
            .ok_or_else(|| StorageError::InvalidPath("admitted Agent Plan is absent".to_string()))
    }

    /// Completed facts remain attributed to the Plan that accepted them.
    pub fn completed_history_for_goal(
        &self,
        goal_id: &str,
    ) -> Result<Vec<crate::strategy::StrategyCompletedHistoryEntry>, StorageError> {
        let mut history = Vec::new();
        for milestone in self.milestones_for_goal(goal_id)? {
            if milestone.goal_id == goal_id {
                let plan = self
                    .reconciliation_plan(&milestone.plan_revision_id)?
                    .filter(|plan| plan.goal_id == goal_id)
                    .ok_or_else(|| {
                        StorageError::InvalidPath(
                            "completed milestone has no original Goal Plan".into(),
                        )
                    })?;
                let product = plan
                    .tasks
                    .iter()
                    .find(|task| task.task_id == milestone.product_id)
                    .map(|task| crate::strategy::StrategyProduct::Task(Box::new(task.clone())))
                    .or_else(|| {
                        plan.epistemic_operations
                            .iter()
                            .find(|operation| operation.product_id == milestone.product_id)
                            .map(|operation| {
                                crate::strategy::StrategyProduct::Epistemic(Box::new(
                                    operation.clone(),
                                ))
                            })
                    })
                    .ok_or_else(|| {
                        StorageError::InvalidPath(
                            "completed milestone has no original Plan product".into(),
                        )
                    })?;
                history.push(crate::strategy::StrategyCompletedHistoryEntry {
                    source_plan_revision_id: milestone.plan_revision_id,
                    product_id: milestone.product_id,
                    accepted_milestone: milestone.requirement,
                    owner_position_id: milestone.owner_position_id,
                    product: Some(product),
                });
            }
        }
        history
            .sort_by_cached_key(|entry| serde_json::to_string(entry).expect("history serializes"));
        history.dedup();
        Ok(history)
    }

    pub fn reconciliation_plan_history(
        &self,
        plan_id: &str,
    ) -> Result<Vec<crate::strategy::StrategyCompletedHistoryEntry>, StorageError> {
        Ok(
            get_immutable(&self.reconciliation_plans, &format!("history::{plan_id}"))?
                .unwrap_or_default(),
        )
    }

    /// Publish successor admission and predecessor supersession atomically.
    /// The fixed predecessor decision key makes competing successors conflict.
    pub(crate) fn admit_successor(
        &self,
        successor: &crate::strategy::StrategySuccessorPlan,
        admitted: &AgentPlanJudgment,
        superseded: &AgentPlanJudgment,
    ) -> Result<(), StorageError> {
        let plan_rows = [
            (
                successor.plan.plan_revision_id.clone(),
                serde_json::to_vec(&successor.plan).map_err(to_storage_data)?,
            ),
            (
                format!("history::{}", successor.plan.plan_revision_id),
                serde_json::to_vec(&successor.completed_history).map_err(to_storage_data)?,
            ),
        ];
        let judgment_rows = [
            (
                admitted.judgment_id.clone(),
                serde_json::to_vec(admitted).map_err(to_storage_data)?,
            ),
            (
                superseded.judgment_id.clone(),
                serde_json::to_vec(superseded).map_err(to_storage_data)?,
            ),
        ];
        (&self.reconciliation_plans, &self.reconciliation_judgments)
            .transaction(|(plans, judgments)| {
                for (tree, rows) in [(plans, &plan_rows), (judgments, &judgment_rows)] {
                    for (key, value) in rows {
                        if let Some(existing) = tree.get(key.as_bytes())? {
                            if existing.as_ref() != value.as_slice() {
                                return Err(ConflictableTransactionError::Abort(format!(
                                    "Agent successor conflicts at '{key}'"
                                )));
                            }
                        } else {
                            tree.insert(key.as_bytes(), value.as_slice())?;
                        }
                    }
                }
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Abort(reason) => StorageError::InvalidPath(reason),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
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
        if record
            .request_ref
            .as_ref()
            .is_some_and(|request| request != &record.goal_id)
        {
            return Err(StorageError::InvalidPath(
                "product request differs from its native Goal".into(),
            ));
        }
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

    pub fn milestones_for_goal(
        &self,
        goal_id: &str,
    ) -> Result<Vec<AgentMilestoneAcceptance>, StorageError> {
        let mut milestones = Vec::new();
        for row in &self.reconciliation_milestones {
            let (_, raw) = row.map_err(to_storage_io)?;
            let milestone: AgentMilestoneAcceptance =
                serde_json::from_slice(&raw).map_err(to_storage_data)?;
            if milestone.goal_id == goal_id {
                milestones.push(milestone);
            }
        }
        milestones.sort_by(|left, right| left.milestone_id.cmp(&right.milestone_id));
        Ok(milestones)
    }

    /// Monotonic durable position across the append-only reconciliation family.
    pub fn reconciliation_position(&self) -> u64 {
        [
            &self.reconciliation_goals,
            &self.reconciliation_requests,
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
    pub(crate) fn put_agent(&self, record: &AgentRecord) -> Result<(), StorageError> {
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
    pub(crate) fn put_subscription(
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
