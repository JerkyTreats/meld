//! One durable Agent reconciliation participant.

use std::sync::Arc;

use meld_lang::{evaluate_authority, Goal};

use crate::agent::{
    AgentAuthorizationFence, AgentAuthorizedProduct, AgentConsumerReceipt, AgentCurrentnessCheck,
    AgentExecutionAdmissionDecision, AgentExecutionPosition, AgentExecutionReceipt,
    AgentMilestoneAcceptance, AgentPlanJudgment, AgentPlanJudgmentKind, AgentProductAuthorization,
    AgentProductProgress, AgentProductState, AgentReconciliationGoal, AgentStore,
    AgentStrategyRuntimeConfig,
};
use crate::curation::{
    CurationAcceptanceRecord, CurationAdmissionDecision, CurationAuthority, CurationOperation,
    CurationPlannedAuthorization, CurationResult,
};
use crate::error::StorageError;
use crate::planner::{PlannerAssemblyOutcome, PlannerCut, PlannerRefusal};
use crate::strategy::{
    search, search_successor, verify_plan, verify_successor_plan, PlanMilestoneRequirement,
    PlanVerification, StrategyPlan, StrategySearchRequest, StrategySuccessorRequest,
};
use crate::waiting::{StructuralWakeAddress, WaitingOnDeclaration};

/// Planner-owned current source assembly used by Agent for construction and freshness.
pub trait AgentPlannerPort: Send + Sync {
    fn assemble(&self) -> PlannerAssemblyOutcome;

    /// Assemble against the Agent-owned Goal and current admission authority.
    fn assemble_for(
        &self,
        _goal_id: &str,
        _fence: &AgentAuthorizationFence,
    ) -> PlannerAssemblyOutcome {
        self.assemble()
    }

    /// Explicit observation selection; old adapters cannot silently ignore it.
    fn assemble_epoch(
        &self,
        products: &crate::agent::AgentEpochProducts,
    ) -> PlannerAssemblyOutcome {
        PlannerAssemblyOutcome::Refused(PlannerRefusal {
            request_context_id: format!("agent-context::{}", products.specification.goal_id),
            grounds: vec![crate::planner::PlannerRefusalGround::UnsupportedObservationSelection],
        })
    }
}

/// Read-only root observation of the currently active Agent authority fence.
pub trait AgentAuthorityPort: Send + Sync {
    fn observe(&self) -> Result<Option<AgentAuthorizationFence>, StorageError>;
}

/// Curation-owned planned intake and result query boundary.
pub trait AgentCurationPort: Send + Sync {
    fn resolve_operation(
        &self,
        candidate: CurationOperation,
    ) -> Result<CurationOperation, StorageError>;
    fn submit(&self, operation: CurationOperation) -> Result<(), StorageError>;
    fn acceptance(
        &self,
        operation_id: &str,
    ) -> Result<Option<CurationAcceptanceRecord>, StorageError>;
    fn result(&self, operation_id: &str) -> Result<Option<CurationResult>, StorageError>;
}

/// Execution consumer and owner-return boundary used by Agent progression.
pub trait AgentExecutionPort: Send + Sync {
    fn submit(
        &self,
        authorization: &AgentProductAuthorization,
    ) -> Result<AgentExecutionPosition, StorageError>;

    fn advance(
        &self,
        authorization: &AgentProductAuthorization,
    ) -> Result<AgentExecutionPosition, StorageError>;

    /// Read an existing consumer position without offering or admitting work.
    fn observe(
        &self,
        _authorization: &AgentProductAuthorization,
    ) -> Result<Option<AgentExecutionPosition>, StorageError> {
        Ok(None)
    }
}

/// Truthful bounded transition report for one reconciliation tick.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AgentReconciliationReport {
    pub input_position: u64,
    pub output_position: u64,
    pub items_attempted: usize,
    pub budget_exhausted: bool,
    pub records_persisted: usize,
    pub products_authorized: usize,
    pub milestones_accepted: usize,
    pub eligible_task_ids: Vec<String>,
    pub waiting_on: Vec<WaitingOnDeclaration>,
    pub retryable_errors: Vec<String>,
    pub fatal_errors: Vec<String>,
}

/// Canonical Agent owner for Goal, Plan, authority, receipt, and milestone history.
pub struct AgentReconciliationActor {
    actor_id: String,
    intent: crate::agent::AgentReconciliationIntent,
    store: Arc<AgentStore>,
    planner: Arc<dyn AgentPlannerPort>,
    authority: Arc<dyn AgentAuthorityPort>,
    frozen_authority: AgentAuthorizationFence,
    curation: Arc<dyn AgentCurationPort>,
    execution: Arc<dyn AgentExecutionPort>,
    strategy: AgentStrategyRuntimeConfig,
    curation_authority: CurationAuthority,
    preparation: crate::agent::AgentPreparation,
    lifecycle: crate::lifecycle::NativeLifecycle,
    work_lock: parking_lot::Mutex<()>,
}

impl AgentReconciliationActor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actor_id: impl Into<String>,
        intent: impl Into<crate::agent::AgentReconciliationIntent>,
        store: Arc<AgentStore>,
        planner: Arc<dyn AgentPlannerPort>,
        authority: Arc<dyn AgentAuthorityPort>,
        frozen_authority: AgentAuthorizationFence,
        curation: Arc<dyn AgentCurationPort>,
        execution: Arc<dyn AgentExecutionPort>,
        strategy: AgentStrategyRuntimeConfig,
        curation_authority: CurationAuthority,
        preparation: impl Into<crate::agent::AgentPreparation>,
    ) -> Result<Self, StorageError> {
        let actor_id = actor_id.into();
        let intent = intent.into();
        match &intent {
            crate::agent::AgentReconciliationIntent::Goal(goal)
                if goal.agent_id != strategy.agent_id =>
            {
                return Err(StorageError::InvalidPath("Goal names another Agent".into()));
            }
            crate::agent::AgentReconciliationIntent::MaintainedCondition(binding) => {
                binding.validate()?
            }
            _ => {}
        }
        if actor_id.trim().is_empty()
            || strategy.agent_id != curation_authority.agent_id
            || strategy.subject != curation_authority.subject
        {
            return Err(StorageError::InvalidPath(
                "Agent reconciliation actor requires one exact Agent identity".to_string(),
            ));
        }
        curation_authority.validate()?;
        let preparation = preparation.into();
        if let crate::agent::AgentReconciliationIntent::MaintainedCondition(binding) = &intent {
            let declared_epoch = binding.condition.observation_scope
                == crate::agent::AgentObservationScope::AdmissionEpoch;
            if declared_epoch != matches!(preparation, crate::agent::AgentPreparation::Epoch { .. })
            {
                return Err(StorageError::InvalidPath(
                    "Agent preparation differs from its declared observation scope".into(),
                ));
            }
        }
        match &preparation {
            crate::agent::AgentPreparation::InstalledRule(rule) => rule.validate()?,
            crate::agent::AgentPreparation::Epoch { .. }
                if !matches!(
                    intent,
                    crate::agent::AgentReconciliationIntent::MaintainedCondition(_)
                ) =>
            {
                return Err(StorageError::InvalidPath(
                    "epoch preparation requires a native maintained-condition intent".into(),
                ));
            }
            _ => {}
        }
        if frozen_authority.activation_generation != curation_authority.activation_generation
            || frozen_authority.admission_epoch != curation_authority.admission_epoch
            || frozen_authority
                .authority_policy_content_hash
                .trim()
                .is_empty()
        {
            return Err(StorageError::InvalidPath(
                "Agent reconciliation authority fence is incomplete".to_string(),
            ));
        }
        Ok(Self {
            lifecycle: crate::lifecycle::NativeLifecycle::new(actor_id.clone()),
            work_lock: parking_lot::Mutex::new(()),
            actor_id,
            intent,
            store,
            planner,
            authority,
            frozen_authority,
            curation,
            execution,
            strategy,
            curation_authority,
            preparation,
        })
    }

    fn prepare_epoch(
        &self,
        authority: &CurationAuthority,
        fence: &AgentAuthorizationFence,
    ) -> Result<Option<Arc<crate::agent::AgentEpochProducts>>, StorageError> {
        let crate::agent::AgentPreparation::Epoch { products: port, .. } = &self.preparation else {
            return Ok(None);
        };
        let genesis = self
            .store
            .genesis_intent_for_agent(&self.strategy.agent_id)?
            .ok_or_else(|| {
                StorageError::InvalidPath("epoch preparation requires native Agent genesis".into())
            })?;
        let specification = crate::agent::AgentEpochSpecification::new(
            self.intent.clone(),
            authority.clone(),
            fence.clone(),
            genesis,
        )?;
        if let Some(products) = self.store.epoch_products(&specification.goal_id)? {
            if products.specification != specification {
                return Err(StorageError::InvalidPath(
                    "persisted epoch products name another specification".into(),
                ));
            }
            return Ok(Some(Arc::new(products)));
        }
        if self.authority.observe()?.as_ref() != Some(fence) {
            return Err(StorageError::InvalidPath(
                "Agent authority changed before epoch preparation".into(),
            ));
        }
        let products = port.prepare(&specification)?;
        products.validate()?;
        if products.specification != specification {
            return Err(StorageError::InvalidPath(
                "owner preparation returned a foreign specification".into(),
            ));
        }
        if self.authority.observe()?.as_ref() != Some(fence) {
            return Err(StorageError::InvalidPath(
                "Agent authority changed during epoch preparation".into(),
            ));
        }
        self.store.put_epoch_products(&products)?;
        Ok(Some(Arc::new(products)))
    }

    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Return the native durable reconciliation position used by lifecycle recovery.
    pub fn lifecycle_position(&self) -> u64 {
        self.store.reconciliation_position()
    }

    /// Resolve this Agent's Planner and product waits through its bound native ports.
    pub fn resolves_wake(&self, wake: &StructuralWakeAddress) -> Result<bool, String> {
        let value = match wake {
            StructuralWakeAddress::OwnerRevision(value)
            | StructuralWakeAddress::DurableOperation(value)
            | StructuralWakeAddress::BindingRecovery(value) => value,
            _ => return Ok(false),
        };
        let Some(value) = crate::waiting::bound_address(value, self.store.resource_id()) else {
            return Ok(false);
        };
        let fence = self
            .authority
            .observe()
            .map_err(|error| error.to_string())?
            .unwrap_or_else(|| self.frozen_authority.clone());
        let goal_id = self
            .intent
            .goal_id(&self.strategy.agent_id, &fence.reconciliation_scope());
        let mut goal_ids = vec![goal_id.clone()];
        goal_ids.extend(
            self.store
                .reconciliation_goals_for_agent(&self.strategy.agent_id)
                .map_err(|error| error.to_string())?
                .into_iter()
                .map(|record| record.goal.goal_id),
        );
        goal_ids.sort();
        goal_ids.dedup();
        let mut authorizations = Vec::new();
        for id in &goal_ids {
            authorizations.extend(
                self.store
                    .product_authorizations_for_goal(id)
                    .map_err(|error| error.to_string())?,
            );
        }
        if let StructuralWakeAddress::DurableOperation(_) = wake {
            if let Some(id) = value
                .strip_prefix("curation-operation::")
                .and_then(|tail| tail.strip_suffix("::completion"))
            {
                return Ok(authorizations.iter().any(|authorization| matches!(&authorization.product, AgentAuthorizedProduct::Epistemic(product) if product.operation.operation_id == id)));
            }
            if let Some(id) = value
                .strip_prefix("execution-outcome::")
                .and_then(|tail| tail.strip_suffix("::completion"))
            {
                return Ok(self
                    .store
                    .product_progress(id)
                    .map_err(|error| error.to_string())?
                    .is_some_and(|progress| {
                        progress.agent_id == self.strategy.agent_id
                            && goal_ids.contains(&progress.goal_id)
                    }));
            }
        }
        let key = match wake {
            StructuralWakeAddress::OwnerRevision(_) => value
                .strip_prefix("planner-input::")
                .and_then(|tail| tail.strip_suffix("::successor")),
            StructuralWakeAddress::BindingRecovery(_) => value.strip_prefix("agent-authority::"),
            _ => None,
        };
        if let Some(key) = key {
            if key == goal_id
                || authorizations.iter().any(|authorization| {
                    authorization.authorization_id == key || authorization.product_id == key
                })
            {
                return Ok(true);
            }
            if self
                .store
                .product_progress(key)
                .map_err(|error| error.to_string())?
                .is_some_and(|progress| {
                    progress.agent_id == self.strategy.agent_id
                        && goal_ids.contains(&progress.goal_id)
                })
                || self
                    .store
                    .reconciliation_plan(key)
                    .map_err(|error| error.to_string())?
                    .is_some_and(|plan| goal_ids.contains(&plan.goal_id))
            {
                return Ok(true);
            }
            let assembled = match self
                .store
                .epoch_products(&goal_id)
                .map_err(|error| error.to_string())?
            {
                Some(products) => self.planner.assemble_epoch(&products),
                None => self.planner.assemble_for(&goal_id, &fence),
            };
            return Ok(match assembled {
                PlannerAssemblyOutcome::Complete(cut) => {
                    key == cut.cut_id || key == cut.context.context_id
                }
                PlannerAssemblyOutcome::Refused(refusal) => {
                    key == refusal.request_context_id
                        && !refusal.grounds.contains(
                            &crate::planner::PlannerRefusalGround::UnsupportedObservationSelection,
                        )
                }
            });
        }
        // Rejected intake and frozen policy changes require a new authority or
        // construction result. They are not live work in this instance.
        Ok(false)
    }

    /// Read lifecycle evidence from this owner's bound stores and installed inputs.
    pub fn lifecycle_evidence(&self) -> Result<crate::lifecycle::NativeLifecycleEvidence, String> {
        let _guard = self.work_lock.lock();
        self.lifecycle_evidence_inner()
    }

    fn lifecycle_evidence_inner(
        &self,
    ) -> Result<crate::lifecycle::NativeLifecycleEvidence, String> {
        let position = self.store.reconciliation_position();
        let goals = self
            .store
            .reconciliation_goals_for_agent(&self.strategy.agent_id)
            .map_err(|error| error.to_string())?;
        let mut authorizations = Vec::new();
        let mut history = Vec::new();
        for goal in goals {
            authorizations.extend(
                self.store
                    .product_authorizations_for_goal(&goal.goal.goal_id)
                    .map_err(|error| error.to_string())?,
            );
            history.extend(
                self.store
                    .completed_history_for_goal(&goal.goal.goal_id)
                    .map_err(|error| error.to_string())?,
            );
        }
        let pending: Vec<_> = authorizations
            .iter()
            .filter(|authorization| {
                !history.iter().any(|entry| {
                    entry.source_plan_revision_id == authorization.plan_revision_id
                        && entry.product_id == authorization.product_id
                })
            })
            .collect();
        let checkpoint_ref = format!(
            "agent-reconciliation::{}::{position}",
            self.strategy.agent_id
        );
        let current_authority = self
            .authority
            .observe()
            .map_err(|error| error.to_string())?;
        let current_products = match &current_authority {
            Some(fence) => self
                .store
                .epoch_products(
                    &self
                        .intent
                        .goal_id(&self.strategy.agent_id, &fence.reconciliation_scope()),
                )
                .map_err(|error| error.to_string())?,
            None => None,
        };
        Ok(crate::lifecycle::NativeLifecycleEvidence {
            checkpoint_ref: checkpoint_ref.clone(),
            installed_revision_refs: vec![
                crate::lifecycle::evidence_ref("agent-intent", &self.intent)?,
                crate::lifecycle::evidence_ref("agent-strategy", &self.strategy)?,
                crate::lifecycle::evidence_ref(
                    "curation-rule",
                    &match &self.preparation {
                        crate::agent::AgentPreparation::InstalledRule(rule) => {
                            Some(rule.revision_ref())
                        }
                        crate::agent::AgentPreparation::Epoch { .. } => None,
                    },
                )?,
            ],
            binding_refs: vec![
                crate::lifecycle::evidence_ref("agent-prepared-authority", &self.frozen_authority)?,
                crate::lifecycle::evidence_ref("agent-current-admission", &current_authority)?,
                crate::lifecycle::evidence_ref("agent-epoch-products", &current_products)?,
            ],
            subscription_refs: vec![format!("agent-plan-products::{}", self.strategy.agent_id)],
            proof_position_ref: checkpoint_ref,
            unresolved_operation_summary_ref: crate::lifecycle::evidence_ref(
                "agent-unresolved-products",
                &pending,
            )?,
        })
    }

    /// Author native start evidence with bounded work excluded.
    pub fn lifecycle_start(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.work_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .start(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native safe point evidence with bounded work excluded.
    pub fn lifecycle_safe_point(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.work_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .safe_point(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native stop evidence with bounded work excluded.
    pub fn lifecycle_stop(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.work_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .stop(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native release evidence with bounded work excluded.
    pub fn lifecycle_release(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let _guard = self.work_lock.lock();
        let evidence = self.lifecycle_evidence_inner()?;
        let transition = self
            .lifecycle
            .release(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Advance eligible transitions through durable, replay-resumable boundaries.
    pub fn bounded_step(&self, max_items: usize) -> AgentReconciliationReport {
        let _guard = self.work_lock.lock();
        let input_position = self.store.reconciliation_position();
        let mut report = self.bounded_step_inner(max_items);
        crate::waiting::bind_waits(&mut report.waiting_on, self.store.resource_id());
        report.input_position = input_position;
        report.output_position = self.store.reconciliation_position();
        report.records_persisted = report.output_position.saturating_sub(input_position) as usize;
        report
    }

    fn subscribe_epoch(
        &self,
        products: &crate::agent::AgentEpochProducts,
        max_items: usize,
    ) -> Result<(bool, usize), StorageError> {
        let crate::agent::AgentPreparation::Epoch { subscriptions, .. } = &self.preparation else {
            return Err(StorageError::InvalidPath(
                "epoch subscription source is not bound".into(),
            ));
        };
        let mut consumed = 0;
        for request in products.subscription_requests()? {
            if let Some(receipt) = self.store.epoch_subscription_receipt(&request.request_id)? {
                if receipt.agent_id != request.agent_id || receipt.belief_key != request.belief_key
                {
                    return Err(StorageError::InvalidPath(
                        "subscription receipt belongs to another Agent observation".into(),
                    ));
                }
                continue;
            }
            if consumed == max_items
                || self.authority.observe()?.as_ref() != Some(&products.specification.fence)
            {
                return Ok((false, consumed));
            }
            let proof = subscriptions.subscribe(&request)?;
            self.store.receive_epoch_subscription(&request, &proof)?;
            consumed += 1;
        }
        Ok((true, consumed))
    }

    fn observe_prior_returns(
        &self,
        current: Option<&AgentAuthorizationFence>,
        max_items: usize,
        report: &mut AgentReconciliationReport,
    ) -> Result<usize, StorageError> {
        let mut returned = 0;
        for record in self
            .store
            .reconciliation_goals_for_agent(&self.strategy.agent_id)?
        {
            let history = self
                .store
                .completed_history_for_goal(&record.goal.goal_id)?;
            for authorization in self
                .store
                .product_authorizations_for_goal(&record.goal.goal_id)?
            {
                if authorization.admission_epoch.is_none()
                    || current.is_some_and(|fence| {
                        fence.activation_generation == authorization.activation_generation
                            && fence.admission_epoch == authorization.admission_epoch
                    })
                    || history.iter().any(|entry| {
                        entry.source_plan_revision_id == authorization.plan_revision_id
                            && entry.product_id == authorization.product_id
                    })
                {
                    continue;
                }
                let plan = self
                    .store
                    .reconciliation_plan(&authorization.plan_revision_id)?
                    .ok_or_else(|| {
                        StorageError::InvalidPath("authorized predecessor Plan is absent".into())
                    })?;
                let cut = self
                    .store
                    .reconciliation_cut(&plan.planner_cut_id)?
                    .ok_or_else(|| {
                        StorageError::InvalidPath(
                            "epoch-bound Plan has no retained admitted cut".into(),
                        )
                    })?;
                let fence = AgentAuthorizationFence {
                    activation_generation: authorization.activation_generation.clone(),
                    admission_epoch: authorization.admission_epoch.clone(),
                    authority_policy_content_hash: authorization
                        .authority_policy_content_hash
                        .clone(),
                };
                let mut curation_authority = self.curation_authority.clone();
                curation_authority.activation_generation = fence.activation_generation.clone();
                curation_authority.admission_epoch = fence.admission_epoch.clone();
                let epoch = ReconciliationEpoch {
                    owner: self,
                    planner: EpochPlanner {
                        port: self.planner.as_ref(),
                        goal_id: record.goal.goal_id.clone(),
                        fence: fence.clone(),
                        products: None,
                    },
                    frozen_authority: fence,
                    curation_authority,
                    products: None,
                };
                let reconciliation = GoalReconciliation {
                    historical_return: true,
                    epoch: &epoch,
                    goal: record.goal.clone(),
                };
                match &authorization.product {
                    AgentAuthorizedProduct::Task(task) => {
                        let Some(position) = self.execution.observe(&authorization)? else {
                            continue;
                        };
                        if position.outcome_id.is_none() {
                            continue;
                        }
                        reconciliation.persist_execution_position(&authorization, &position)?;
                        reconciliation
                            .project_execution_wait(&cut, &plan, task, &position, report)?;
                    }
                    AgentAuthorizedProduct::Epistemic(operation) => {
                        if self
                            .curation
                            .result(&operation.operation.operation_id)?
                            .is_none()
                        {
                            continue;
                        }
                        reconciliation.reconcile_curation(
                            &cut,
                            &plan,
                            operation,
                            usize::MAX,
                            false,
                            report,
                        )?;
                    }
                }
                returned += 1;
                if returned == max_items {
                    return Ok(returned);
                }
            }
        }
        Ok(returned)
    }

    fn bounded_step_inner(&self, max_items: usize) -> AgentReconciliationReport {
        let mut report = AgentReconciliationReport::default();
        if max_items == 0 {
            report
                .fatal_errors
                .push("Agent reconciliation budget must be greater than zero".to_string());
            return report;
        }
        report.items_attempted = 1;
        let observed = match self.authority.observe() {
            Ok(observed) => observed,
            Err(error) => {
                report.retryable_errors.push(error.to_string());
                return report;
            }
        };
        let consumed = match self.observe_prior_returns(observed.as_ref(), max_items, &mut report) {
            Ok(consumed) => consumed,
            Err(error) => {
                report.retryable_errors.push(error.to_string());
                return report;
            }
        };
        let mut max_items = max_items.saturating_sub(consumed);
        if max_items == 0 {
            report.budget_exhausted = true;
            return report;
        }
        if observed.is_none() {
            report.waiting_on.push(waiting(
                "agent_authority_changed",
                self.intent.goal_id(
                    &self.strategy.agent_id,
                    &self.frozen_authority.reconciliation_scope(),
                ),
                "Agent has no open admission epoch; only existing owner returns may be recorded",
            ));
            return report;
        }
        let frozen_authority = match observed {
            Some(fence)
                if fence.admission_epoch.is_some()
                    && fence.authority_policy_content_hash
                        == self.frozen_authority.authority_policy_content_hash =>
            {
                fence
            }
            _ => self.frozen_authority.clone(),
        };
        let goal_id = self.intent.goal_id(
            &self.strategy.agent_id,
            &frozen_authority.reconciliation_scope(),
        );
        let mut curation_authority = self.curation_authority.clone();
        curation_authority.activation_generation = frozen_authority.activation_generation.clone();
        curation_authority.admission_epoch = frozen_authority.admission_epoch.clone();
        let products = match self.prepare_epoch(&curation_authority, &frozen_authority) {
            Ok(products) => products,
            Err(error) => {
                report.retryable_errors.push(error.to_string());
                return report;
            }
        };
        if let Some(products) = &products {
            match self.subscribe_epoch(products, max_items) {
                Ok((_, consumed)) if consumed == max_items => {
                    report.budget_exhausted = true;
                    return report;
                }
                Ok((true, consumed)) => {
                    max_items -= consumed;
                }
                Ok((false, _)) => {
                    report.waiting_on.push(waiting(
                        "agent_authority_changed",
                        &products.specification.goal_id,
                        "epoch subscriptions require current Agent authority",
                    ));
                    return report;
                }
                Err(error) => {
                    report.retryable_errors.push(error.to_string());
                    return report;
                }
            }
        }
        let epoch = ReconciliationEpoch {
            owner: self,
            planner: EpochPlanner {
                port: self.planner.as_ref(),
                goal_id,
                fence: frozen_authority.clone(),
                products: products.clone(),
            },
            frozen_authority,
            curation_authority,
            products,
        };
        epoch.advance(max_items, report)
    }
}

struct EpochPlanner<'a> {
    port: &'a dyn AgentPlannerPort,
    goal_id: String,
    fence: AgentAuthorizationFence,
    products: Option<Arc<crate::agent::AgentEpochProducts>>,
}

impl AgentPlannerPort for EpochPlanner<'_> {
    fn assemble(&self) -> PlannerAssemblyOutcome {
        match &self.products {
            Some(products) => self.port.assemble_epoch(products),
            None => self.port.assemble_for(&self.goal_id, &self.fence),
        }
    }
}

/// Native decision context bound once per tick and rechecked before authority is granted.
struct ReconciliationEpoch<'a> {
    owner: &'a AgentReconciliationActor,
    planner: EpochPlanner<'a>,
    frozen_authority: AgentAuthorizationFence,
    curation_authority: CurationAuthority,
    products: Option<Arc<crate::agent::AgentEpochProducts>>,
}

impl std::ops::Deref for ReconciliationEpoch<'_> {
    type Target = AgentReconciliationActor;
    fn deref(&self) -> &Self::Target {
        self.owner
    }
}

impl ReconciliationEpoch<'_> {
    fn advance(
        &self,
        max_items: usize,
        mut report: AgentReconciliationReport,
    ) -> AgentReconciliationReport {
        let cut = match self.planner.assemble() {
            PlannerAssemblyOutcome::Complete(cut) => *cut,
            PlannerAssemblyOutcome::Refused(refusal)
                if refusal.grounds.contains(
                    &crate::planner::PlannerRefusalGround::UnsupportedObservationSelection,
                ) =>
            {
                report
                    .fatal_errors
                    .push("Planner adapter does not support the selected epoch observation".into());
                return report;
            }
            PlannerAssemblyOutcome::Refused(refusal) => {
                report.waiting_on.push(planner_wait(&refusal));
                return report;
            }
        };
        if cut.context.activation_generation != self.frozen_authority.activation_generation
            || cut.context.admission_epoch != self.frozen_authority.admission_epoch
        {
            report
                .fatal_errors
                .push("Planner returned a cut outside the Agent admission fence".into());
            return report;
        }
        if let Some(products) = &self.products {
            if cut.context.subject != self.strategy.subject
                || cut.context.observation_subject() != &products.observation_subject
                || cut.context.goal_id != products.specification.goal_id
                || cut.context.agent_id != self.strategy.agent_id
                || cut.traversal_request != products.curation_rule.rule.traversal_request()
            {
                report
                    .fatal_errors
                    .push("Planner returned evidence outside the epoch specification".into());
                return report;
            }
        }
        let goal = match self.judge_intent(&cut, &mut report) {
            Ok(Some(goal)) => goal,
            Ok(None) => return report,
            Err(error) => {
                report.retryable_errors.push(error.to_string());
                return report;
            }
        };
        GoalReconciliation {
            epoch: self,
            goal,
            historical_return: false,
        }
        .advance(cut, max_items, report)
    }

    fn judge_intent(
        &self,
        cut: &PlannerCut,
        report: &mut AgentReconciliationReport,
    ) -> Result<Option<Goal>, StorageError> {
        use crate::agent::{AgentConditionJudgment, AgentReconciliationIntent};
        let binding = match &self.intent {
            AgentReconciliationIntent::Goal(goal) => return Ok(Some(goal.clone())),
            AgentReconciliationIntent::MaintainedCondition(binding) => binding,
        };
        let goal_id = self.intent.goal_id(
            &self.strategy.agent_id,
            &self.frozen_authority.reconciliation_scope(),
        );
        if let Some(record) = self.store.reconciliation_goal(&goal_id)? {
            return Ok(Some(record.goal));
        }
        if self.authority.observe()?.as_ref() != Some(&self.frozen_authority) {
            report.waiting_on.push(waiting(
                "agent_authority_changed",
                &goal_id,
                "maintained-condition judgment requires current Agent authority",
            ));
            return Ok(None);
        }
        match self.planner.assemble() {
            PlannerAssemblyOutcome::Complete(current) if current.cut_id == cut.cut_id => {}
            PlannerAssemblyOutcome::Complete(_) => {
                report.waiting_on.push(waiting(
                    "planner_cut_changed",
                    &cut.context.context_id,
                    "condition judgment waits for the changed admitted evidence",
                ));
                return Ok(None);
            }
            PlannerAssemblyOutcome::Refused(refusal) => {
                report.waiting_on.push(planner_wait(&refusal));
                return Ok(None);
            }
        }
        let condition = &binding.condition;
        let target = condition.target_for(
            self.products
                .as_ref()
                .map(|products| &products.observation_subject)
                .unwrap_or(&self.strategy.subject)
                .clone(),
        )?;
        let evaluation = meld_lang::evaluate(&cut.world_model_view.world_state, &target);
        self.store.put_condition_judgment(&AgentConditionJudgment {
            judgment_id: stable_id(
                "agent-condition-judgment-v1",
                &(&self.strategy.agent_id, &binding.revision, &cut.cut_id),
            ),
            agent_id: self.strategy.agent_id.clone(),
            condition_revision: binding.revision.clone(),
            planner_cut_id: cut.cut_id.clone(),
            activation_generation: self.frozen_authority.activation_generation.clone(),
            evaluation: evaluation.clone(),
        })?;
        let source = match evaluation {
            meld_lang::EvalResult::Satisfied => {
                report.waiting_on.push(waiting(
                    "goal_disposition",
                    &cut.context.context_id,
                    "maintained condition is established; no Goal or work is required",
                ));
                return Ok(None);
            }
            meld_lang::EvalResult::Unsatisfied { .. } => {
                meld_lang::GoalSource::MaintainedConditionBreach {
                    maintained_condition_id: condition.condition_id.clone(),
                    dimension: condition.dimension_id.clone(),
                    observed: serde_json::to_string(&evaluation)
                        .map_err(|error| StorageError::InvalidPath(error.to_string()))?,
                    desired: condition.desired_summary.clone(),
                }
            }
            meld_lang::EvalResult::Indeterminate { .. } => meld_lang::GoalSource::Maintenance {
                invariant_description: format!(
                    "establish {} from admitted evidence for {}",
                    condition.desired_summary, condition.condition_id,
                ),
            },
        };
        Ok(Some(Goal {
            goal_id,
            agent_id: self.strategy.agent_id.clone(),
            target,
            priority: condition.goal_priority.clone(),
            source,
            lifecycle: meld_lang::GoalLifecycle::Proposed,
        }))
    }
}

/// One borrowed progression context after the native Agent has selected a Goal.
struct GoalReconciliation<'a, 'owner> {
    historical_return: bool,
    epoch: &'a ReconciliationEpoch<'owner>,
    goal: Goal,
}

impl<'owner> std::ops::Deref for GoalReconciliation<'_, 'owner> {
    type Target = ReconciliationEpoch<'owner>;

    fn deref(&self) -> &Self::Target {
        self.epoch
    }
}

impl GoalReconciliation<'_, '_> {
    fn advance(
        &self,
        cut: PlannerCut,
        max_items: usize,
        mut report: AgentReconciliationReport,
    ) -> AgentReconciliationReport {
        let goal_inserted = match self.persist_goal(&cut) {
            Ok(inserted) => inserted,
            Err(error) => {
                report.fatal_errors.push(error.to_string());
                return report;
            }
        };
        let authorizations = match self
            .store
            .product_authorizations_for_goal(&self.goal.goal_id)
        {
            Ok(records) => records,
            Err(error) => {
                report.fatal_errors.push(error.to_string());
                return report;
            }
        };
        let current = match self.store.current_reconciliation_plan(&self.goal.goal_id) {
            Ok(plan) => plan,
            Err(error) => {
                report.fatal_errors.push(error.to_string());
                return report;
            }
        };
        let (plan, judgment_inserted) =
            match self.select_plan(&cut, current, &authorizations, &mut report) {
                Ok(Some(selection)) => selection,
                Ok(None) => return report,
                Err(error) => {
                    report.fatal_errors.push(error.to_string());
                    return report;
                }
            };
        if plan.origin == crate::strategy::StrategyPlanOrigin::Satisfied {
            if let Err(error) = self.accept_satisfied_plan(&plan, &mut report) {
                report.retryable_errors.push(error.to_string());
            }
            return report;
        }
        // Only the admitted leaf supplies eligibility. Completed products from
        // the entire Goal lineage discharge dependencies without reauthorization.
        let history = match self.store.completed_history_for_goal(&self.goal.goal_id) {
            Ok(history) => history,
            Err(error) => {
                report.fatal_errors.push(error.to_string());
                return report;
            }
        };
        let current_authorizations: Vec<_> = authorizations
            .iter()
            .filter(|record| record.plan_revision_id == plan.plan_revision_id)
            .collect();
        if judgment_inserted && max_items == 1 {
            report.budget_exhausted = true;
            return report;
        }
        for authorization in &current_authorizations {
            if product_completed(&plan, &authorization.product_id, &history) {
                continue;
            }
            if !dependencies_satisfied(&plan, &authorization.product_id, &history) {
                continue;
            }
            match &authorization.product {
                AgentAuthorizedProduct::Task(_) => {
                    if let Err(error) =
                        self.reconcile_execution(&cut, &plan, authorization, &mut report)
                    {
                        report.retryable_errors.push(error.to_string());
                    }
                }
                AgentAuthorizedProduct::Epistemic(epistemic) => {
                    if goal_inserted && max_items == 1 {
                        report.budget_exhausted = true;
                        return report;
                    }
                    match self.curation.acceptance(&epistemic.operation.operation_id) {
                        Ok(None) => {
                            match self.authorize_epistemic(&cut, &plan, epistemic, &mut report) {
                                Ok(Some(_)) => {}
                                Ok(None) => return report,
                                Err(error) => {
                                    report.retryable_errors.push(error.to_string());
                                    return report;
                                }
                            }
                        }
                        Ok(Some(_)) => {}
                        Err(error) => {
                            report.retryable_errors.push(error.to_string());
                            return report;
                        }
                    }
                    if let Err(error) = self.reconcile_curation(
                        &cut,
                        &plan,
                        epistemic,
                        max_items - usize::from(goal_inserted),
                        true,
                        &mut report,
                    ) {
                        report.retryable_errors.push(error.to_string());
                    }
                }
            }
            return report;
        }
        let mut used = usize::from(goal_inserted || judgment_inserted);
        if used == max_items {
            report.budget_exhausted = true;
            return report;
        }
        for epistemic in &plan.epistemic_operations {
            if product_completed(&plan, &epistemic.product_id, &history)
                || !dependencies_satisfied(&plan, &epistemic.product_id, &history)
            {
                continue;
            }
            match self.authorize_epistemic(&cut, &plan, epistemic, &mut report) {
                Ok(Some(inserted)) => used += usize::from(inserted),
                Ok(None) => return report,
                Err(error) => {
                    report.fatal_errors.push(error.to_string());
                    return report;
                }
            }
            if used == max_items {
                report.budget_exhausted = true;
                return report;
            }
            if let Err(error) =
                self.reconcile_curation(&cut, &plan, epistemic, max_items - used, true, &mut report)
            {
                report.retryable_errors.push(error.to_string());
            }
            return report;
        }
        for task in &plan.tasks {
            if product_completed(&plan, &task.task_id, &history)
                || !dependencies_satisfied(&plan, &task.task_id, &history)
            {
                continue;
            }
            if let Err(error) = self.advance_task(&cut, &plan, task, &mut report) {
                report.retryable_errors.push(error.to_string());
            }
            return report;
        }
        report.waiting_on.push(waiting(
            "goal_disposition", &self.goal.goal_id,
            "completed products are retained; further reconciliation requires admitted owner evidence",
        ));
        report
    }

    fn select_plan(
        &self,
        cut: &PlannerCut,
        current: Option<StrategyPlan>,
        authorizations: &[AgentProductAuthorization],
        report: &mut AgentReconciliationReport,
    ) -> Result<Option<(StrategyPlan, bool)>, StorageError> {
        if let Some(plan) = current.as_ref() {
            if plan.planner_cut_id == cut.cut_id {
                let history = self.store.completed_history_for_goal(&self.goal.goal_id)?;
                let confirmation_returned =
                    plan.origin == crate::strategy::StrategyPlanOrigin::Confirmation
                        && !plan.epistemic_operations.is_empty()
                        && plan.epistemic_operations.iter().all(|operation| {
                            product_completed(plan, &operation.product_id, &history)
                        });
                if !confirmation_returned
                    || !matches!(
                        meld_lang::evaluate(&cut.world_model_view.world_state, &self.goal.target),
                        meld_lang::EvalResult::Satisfied
                    )
                {
                    return Ok(Some((plan.clone(), false)));
                }
            }
            for product_id in plan.tasks.iter().map(|task| &task.task_id).chain(
                plan.epistemic_operations
                    .iter()
                    .map(|operation| &operation.product_id),
            ) {
                if plan.planner_cut_id == cut.cut_id {
                    continue;
                }
                self.put_progress(
                    cut,
                    plan,
                    product_id,
                    AgentCurrentnessCheck {
                        frozen_cut_id: plan.planner_cut_id.clone(),
                        observed_cut_id: Some(cut.cut_id.clone()),
                        refusal: None,
                    },
                    AgentProductState::Blocked {
                        reason: "Plan premises changed; successor required".into(),
                    },
                )?;
            }
            // Already admitted effects keep their original authority and history.
            // Finish observing them before constructing replacement work, so a
            // successor cannot accidentally duplicate an unresolved effect.
            for authorization in authorizations
                .iter()
                .filter(|record| record.plan_revision_id == plan.plan_revision_id)
            {
                match &authorization.product {
                    AgentAuthorizedProduct::Task(_) => {
                        self.reconcile_execution(cut, plan, authorization, report)?;
                        if report
                            .waiting_on
                            .iter()
                            .any(|wait| wait.condition != "execution_admission_rejected")
                        {
                            return Ok(None);
                        }
                    }
                    AgentAuthorizedProduct::Epistemic(epistemic) => {
                        if self
                            .curation
                            .result(&epistemic.operation.operation_id)?
                            .is_some()
                        {
                            self.reconcile_curation(
                                cut,
                                plan,
                                epistemic,
                                usize::MAX,
                                true,
                                report,
                            )?;
                        }
                    }
                }
            }
        }
        let operations = if current.is_none()
            && matches!(
                meld_lang::evaluate(&cut.world_model_view.world_state, &self.goal.target,),
                meld_lang::EvalResult::Satisfied
            ) {
            Vec::new()
        } else {
            let operation = CurationOperation::reconstruct(
                self.curation_authority.clone(),
                match &self.products {
                    Some(products) => products.curation_rule.revision_ref(),
                    None => match &self.preparation {
                        crate::agent::AgentPreparation::InstalledRule(rule) => rule.revision_ref(),
                        crate::agent::AgentPreparation::Epoch { .. } => {
                            return Err(StorageError::InvalidPath(
                                "Agent epoch products were not prepared".into(),
                            ))
                        }
                    },
                },
                cut.traversal_cut.clone(),
                cut.traversal_request.clone(),
            )?;
            let confirmation = self
                .strategy
                .package
                .snapshot
                .settlement_rules
                .iter()
                .find(|rule| meld_lang::unify(&rule.goal_pattern, &self.goal.target).is_some())
                .is_some_and(|rule| {
                    rule.epistemic_placement
                        == crate::strategy::StrategyEpistemicPlacement::Confirmation
                });
            let operation = if confirmation {
                operation.for_request(stable_id(
                    "agent-curation-request-v1",
                    &(&self.goal.agent_id, &self.goal.goal_id),
                ))?
            } else {
                operation
            };
            vec![self.curation.resolve_operation(operation)?]
        };
        let mut problem = self
            .strategy
            .problem(self.goal.clone(), cut.clone(), operations);
        if let Some(products) = &self.products {
            problem.task_inputs = products.task_inputs.clone();
        }
        let request = StrategySearchRequest {
            problem: problem.clone(),
            bounds: self.strategy.bounds(),
        };
        if let Some(predecessor) = current {
            if self.authority.observe()?.as_ref() != Some(&self.frozen_authority) {
                report.waiting_on.push(waiting(
                    "agent_authority_changed",
                    &predecessor.plan_revision_id,
                    "successor judgment requires the current Agent authority",
                ));
                return Ok(None);
            }
            let request = StrategySuccessorRequest {
                search: request,
                predecessor_plan: Box::new(predecessor.clone()),
                completed_history: self.store.completed_history_for_goal(&self.goal.goal_id)?,
            };
            let result = search_successor(&request);
            let Some(successor) = result.recommendation else {
                report.waiting_on.push(waiting(
                    "strategy_refusal",
                    &cut.cut_id,
                    format!(
                        "Strategy cannot reconstruct a complete successor: {:?}",
                        result.rejections
                    ),
                ));
                return Ok(None);
            };
            if let PlanVerification::Invalid { grounds } =
                verify_successor_plan(&request, &successor)
            {
                return Err(StorageError::InvalidPath(format!(
                    "invalid successor Plan: {grounds:?}"
                )));
            }
            let admitted = self.plan_judgment(cut, &successor.plan);
            let mut superseded = self.plan_judgment(cut, &predecessor);
            superseded.judgment_id =
                stable_id("agent-plan-supersession-v1", &predecessor.plan_revision_id);
            superseded.kind = AgentPlanJudgmentKind::Superseded {
                successor_plan_revision_id: successor.plan.plan_revision_id.clone(),
            };
            if cut.context.admission_epoch.is_some() {
                self.store.put_reconciliation_cut(cut)?;
            }
            self.store
                .admit_successor(&successor, &admitted, &superseded)?;
            report.waiting_on.clear();
            return Ok(Some((successor.plan, true)));
        }
        let result = search(&request);
        let Some(plan) = result.recommendation else {
            report.waiting_on.push(waiting(
                "strategy_refusal",
                &cut.cut_id,
                format!(
                    "Strategy produced no complete Plan: {:?}",
                    result.rejections
                ),
            ));
            return Ok(None);
        };
        if let PlanVerification::Invalid { grounds } = verify_plan(&problem, &plan) {
            return Err(StorageError::InvalidPath(format!(
                "invalid Strategy Plan: {grounds:?}"
            )));
        }
        let inserted = self.persist_plan_and_judgment(cut, &plan)?;
        Ok(Some((plan, inserted)))
    }

    fn accept_satisfied_plan(
        &self,
        plan: &StrategyPlan,
        report: &mut AgentReconciliationReport,
    ) -> Result<(), StorageError> {
        if self.authority.observe()?.as_ref() != Some(&self.frozen_authority) {
            report.waiting_on.push(waiting(
                "agent_authority_changed",
                &self.goal.goal_id,
                "Goal judgment requires the current Agent authority",
            ));
            return Ok(());
        }
        let cut = match self.planner.assemble() {
            PlannerAssemblyOutcome::Complete(cut) if cut.cut_id == plan.planner_cut_id => cut,
            PlannerAssemblyOutcome::Complete(_) => {
                report.waiting_on.push(waiting(
                    "planner_cut_changed",
                    &plan.plan_revision_id,
                    "Goal judgment waits for reconstruction from the changed evidence",
                ));
                return Ok(());
            }
            PlannerAssemblyOutcome::Refused(refusal) => {
                report.waiting_on.push(planner_wait(&refusal));
                return Ok(());
            }
        };
        if !matches!(
            meld_lang::evaluate(&cut.world_model_view.world_state, &self.goal.target),
            meld_lang::EvalResult::Satisfied
        ) {
            return Err(StorageError::InvalidPath(
                "Goal satisfaction lacks admitted evidence".into(),
            ));
        }
        let accepted_milestone_ids: Vec<_> = self
            .store
            .milestones_for_goal(&self.goal.goal_id)?
            .into_iter()
            .filter(|milestone| {
                milestone.agent_id == self.goal.agent_id
                    && milestone.activation_generation == cut.context.activation_generation
            })
            .map(|milestone| milestone.milestone_id)
            .collect();
        self.store
            .put_goal_disposition(&crate::agent::AgentGoalDisposition {
                disposition_id: stable_id(
                    "agent-goal-disposition-v2",
                    &(
                        &self.goal.goal_id,
                        &plan.plan_revision_id,
                        &cut.cut_id,
                        &accepted_milestone_ids,
                    ),
                ),
                accepted_milestone_ids,
                agent_id: self.goal.agent_id.clone(),
                goal_id: self.goal.goal_id.clone(),
                plan_revision_id: plan.plan_revision_id.clone(),
                planner_cut_id: cut.cut_id.clone(),
                activation_generation: cut.context.activation_generation.clone(),
                lifecycle: meld_lang::GoalLifecycle::Satisfied {
                    at_seq: cut.traversal_cut.event_position.after_seq,
                },
            })?;
        report.waiting_on.push(waiting(
            "goal_disposition",
            &self.goal.goal_id,
            "desired condition is satisfied; awaiting changed admitted evidence",
        ));
        Ok(())
    }

    fn persist_goal(&self, cut: &PlannerCut) -> Result<bool, StorageError> {
        let created_at_seq = self
            .store
            .reconciliation_goal(&self.goal.goal_id)?
            .map_or(cut.traversal_cut.event_position.after_seq, |record| {
                record.created_at_seq
            });
        self.store
            .put_reconciliation_goal(&AgentReconciliationGoal {
                goal: self.goal.clone(),
                context_id: cut.context.context_id.clone(),
                authority_scope_id: cut.context.authority_scope_id.clone(),
                activation_generation: cut.context.activation_generation.clone(),
                created_at_seq,
            })
    }

    fn persist_plan_and_judgment(
        &self,
        cut: &PlannerCut,
        plan: &StrategyPlan,
    ) -> Result<bool, StorageError> {
        if cut.context.admission_epoch.is_some() {
            self.store.put_reconciliation_cut(cut)?;
        }
        let plan_inserted = self.store.put_reconciliation_plan(plan)?;
        let judgment_inserted = self
            .store
            .put_plan_judgment(&self.plan_judgment(cut, plan))?;
        Ok(plan_inserted || judgment_inserted)
    }

    fn plan_judgment(&self, cut: &PlannerCut, plan: &StrategyPlan) -> AgentPlanJudgment {
        AgentPlanJudgment {
            judgment_id: stable_id(
                "agent-plan-judgment-v1",
                &(
                    &cut.context.agent_id,
                    &self.goal.goal_id,
                    &plan.plan_revision_id,
                    &cut.context.context_id,
                    &cut.context.authority_scope_id,
                    &cut.context.activation_generation,
                    "admitted",
                ),
            ),
            agent_id: cut.context.agent_id.clone(),
            goal_id: self.goal.goal_id.clone(),
            plan_revision_id: plan.plan_revision_id.clone(),
            context_id: cut.context.context_id.clone(),
            authority_scope_id: cut.context.authority_scope_id.clone(),
            activation_generation: cut.context.activation_generation.clone(),
            kind: AgentPlanJudgmentKind::Admitted,
        }
    }

    fn authorize_epistemic(
        &self,
        cut: &PlannerCut,
        plan: &StrategyPlan,
        epistemic: &crate::strategy::StrategyEpistemicOperation,
        report: &mut AgentReconciliationReport,
    ) -> Result<Option<bool>, StorageError> {
        let observed = match self.planner.assemble() {
            PlannerAssemblyOutcome::Complete(current) => Some(current.cut_id),
            PlannerAssemblyOutcome::Refused(refusal) => {
                let (_, progress_id) = self.put_progress(
                    cut,
                    plan,
                    &epistemic.product_id,
                    AgentCurrentnessCheck {
                        frozen_cut_id: plan.planner_cut_id.clone(),
                        observed_cut_id: None,
                        refusal: Some(refusal),
                    },
                    AgentProductState::Blocked {
                        reason: "Planner currentness refused".to_string(),
                    },
                )?;
                report.waiting_on.push(waiting(
                    "planner_currentness_refused",
                    &progress_id,
                    "wake requires a complete Planner cut for this exact product",
                ));
                return Ok(None);
            }
        };
        let currentness = AgentCurrentnessCheck {
            frozen_cut_id: plan.planner_cut_id.clone(),
            observed_cut_id: observed.clone(),
            refusal: None,
        };
        if observed.as_deref() != Some(plan.planner_cut_id.as_str()) {
            let (_, progress_id) = self.put_progress(
                cut,
                plan,
                &epistemic.product_id,
                currentness,
                AgentProductState::Blocked {
                    reason: "Planner cut changed before authorization".to_string(),
                },
            )?;
            report.waiting_on.push(waiting(
                "planner_cut_changed",
                &progress_id,
                "wake requires a successor reconciliation request for the observed cut",
            ));
            return Ok(None);
        }
        let observed_authority = self.authority.observe()?;
        if observed_authority.as_ref() != Some(&self.frozen_authority) {
            let (_, progress_id) = self.put_progress(
                cut,
                plan,
                &epistemic.product_id,
                currentness,
                AgentProductState::Blocked {
                    reason: "Agent activation or authority policy changed".to_string(),
                },
            )?;
            report.waiting_on.push(waiting(
                "agent_authority_changed",
                &progress_id,
                "wake requires reconstruction under the live activation and authority revision",
            ));
            return Ok(None);
        }
        let authorization_id = stable_id(
            "agent-product-authorization-v1",
            &(
                &cut.context.agent_id,
                &self.goal.goal_id,
                &plan.plan_revision_id,
                &epistemic.product_id,
                &epistemic.operation.operation_id,
                &cut.context.context_id,
                &cut.context.authority_scope_id,
                &cut.context.activation_generation,
                &epistemic.idempotency_key,
            ),
        );
        let curation_authorization = CurationPlannedAuthorization {
            authorization_id: authorization_id.clone(),
            agent_id: cut.context.agent_id.clone(),
            goal_id: self.goal.goal_id.clone(),
            plan_revision_id: plan.plan_revision_id.clone(),
            product_id: epistemic.product_id.clone(),
            operation_id: epistemic.operation.operation_id.clone(),
            context_id: cut.context.context_id.clone(),
            authority_scope_id: cut.context.authority_scope_id.clone(),
            activation_generation: cut.context.activation_generation.clone(),
            admission_epoch: cut.context.admission_epoch.clone(),
            idempotency_key: epistemic.idempotency_key.clone(),
        };
        let inserted = self
            .store
            .put_product_authorization(&AgentProductAuthorization {
                authorization_id: authorization_id.clone(),
                agent_id: cut.context.agent_id.clone(),
                goal_id: self.goal.goal_id.clone(),
                plan_revision_id: plan.plan_revision_id.clone(),
                product_id: epistemic.product_id.clone(),
                context_id: cut.context.context_id.clone(),
                authority_scope_id: cut.context.authority_scope_id.clone(),
                authority_policy_content_hash: self
                    .frozen_authority
                    .authority_policy_content_hash
                    .clone(),
                authority_decision: None,
                activation_generation: cut.context.activation_generation.clone(),
                admission_epoch: cut.context.admission_epoch.clone(),
                idempotency_key: epistemic.idempotency_key.clone(),
                product: AgentAuthorizedProduct::Epistemic(Box::new(epistemic.clone())),
                curation_authorization: Some(curation_authorization.clone()),
            })?;
        self.put_progress(
            cut,
            plan,
            &epistemic.product_id,
            currentness,
            AgentProductState::Authorized {
                authorization_id: authorization_id.clone(),
            },
        )?;
        let operation = epistemic
            .operation
            .clone()
            .with_planned_authorization(curation_authorization)?;
        self.curation.submit(operation)?;
        report.products_authorized = usize::from(inserted);
        Ok(Some(inserted))
    }

    fn reconcile_curation(
        &self,
        cut: &PlannerCut,
        plan: &StrategyPlan,
        epistemic: &crate::strategy::StrategyEpistemicOperation,
        budget: usize,
        advance_products: bool,
        report: &mut AgentReconciliationReport,
    ) -> Result<(), StorageError> {
        let Some(acceptance) = self
            .curation
            .acceptance(&epistemic.operation.operation_id)?
        else {
            report.waiting_on.push(waiting(
                "curation_acceptance",
                &epistemic.operation.operation_id,
                format!(
                    "awaiting Curation acceptance for '{}'",
                    epistemic.operation.operation_id
                ),
            ));
            return Ok(());
        };
        let authorization_id = stable_id(
            "agent-product-authorization-v1",
            &(
                &cut.context.agent_id,
                &self.goal.goal_id,
                &plan.plan_revision_id,
                &epistemic.product_id,
                &epistemic.operation.operation_id,
                &cut.context.context_id,
                &cut.context.authority_scope_id,
                &cut.context.activation_generation,
                &epistemic.idempotency_key,
            ),
        );
        let acceptance_receipt_id = stable_id(
            "agent-consumer-receipt-v1",
            &(
                &authorization_id,
                &acceptance.acceptance_id,
                Option::<&String>::None,
            ),
        );
        let acceptance_inserted = self.store.put_consumer_receipt(&AgentConsumerReceipt {
            receipt_id: acceptance_receipt_id,
            authorization_id: authorization_id.clone(),
            operation_id: epistemic.operation.operation_id.clone(),
            acceptance_id: acceptance.acceptance_id.clone(),
            result_id: None,
        })?;
        let mut used = usize::from(acceptance_inserted);
        if used == budget {
            report.budget_exhausted = true;
            return Ok(());
        }
        if acceptance.decision != CurationAdmissionDecision::Admitted {
            report.waiting_on.push(waiting(
                "curation_rejected",
                &acceptance.acceptance_id,
                acceptance.reason,
            ));
            return Ok(());
        }
        let result = self.curation.result(&epistemic.operation.operation_id)?;
        let Some(result) = result else {
            report.waiting_on.push(waiting(
                "curation_terminal_result",
                &epistemic.operation.operation_id,
                "Curation accepted the operation but has no terminal result",
            ));
            return Ok(());
        };
        let terminal_receipt_id = stable_id(
            "agent-consumer-receipt-v1",
            &(
                &authorization_id,
                &acceptance.acceptance_id,
                Some(&result.result_id),
            ),
        );
        let terminal_inserted = self.store.put_consumer_receipt(&AgentConsumerReceipt {
            receipt_id: terminal_receipt_id,
            authorization_id,
            operation_id: epistemic.operation.operation_id.clone(),
            acceptance_id: acceptance.acceptance_id.clone(),
            result_id: Some(result.result_id.clone()),
        })?;
        used += usize::from(terminal_inserted);
        if used == budget {
            report.budget_exhausted = true;
            return Ok(());
        }
        let milestone_id = stable_id(
            "agent-milestone-acceptance-v1",
            &(
                &cut.context.agent_id,
                &self.goal.goal_id,
                &plan.plan_revision_id,
                &epistemic.product_id,
                &result.result_id,
                &cut.context.context_id,
                &cut.context.activation_generation,
            ),
        );
        let requirement = PlanMilestoneRequirement::CurationTerminal {
            operation_id: epistemic.operation.operation_id.clone(),
        };
        let milestone_inserted = self.store.put_milestone(&AgentMilestoneAcceptance {
            milestone_id: milestone_id.clone(),
            agent_id: cut.context.agent_id.clone(),
            goal_id: self.goal.goal_id.clone(),
            plan_revision_id: plan.plan_revision_id.clone(),
            product_id: epistemic.product_id.clone(),
            requirement: requirement.clone(),
            owner_position_id: result.result_id.clone(),
            context_id: cut.context.context_id.clone(),
            activation_generation: cut.context.activation_generation.clone(),
        })?;
        let (milestone_progress_inserted, _) = self.put_progress(
            cut,
            plan,
            &epistemic.product_id,
            AgentCurrentnessCheck {
                frozen_cut_id: plan.planner_cut_id.clone(),
                observed_cut_id: (!self.historical_return).then(|| cut.cut_id.clone()),
                refusal: None,
            },
            AgentProductState::MilestoneAccepted { milestone_id },
        )?;
        let milestone_advanced = milestone_inserted || milestone_progress_inserted;
        report.milestones_accepted += usize::from(milestone_inserted);
        used += usize::from(milestone_advanced);
        if used == budget {
            report.budget_exhausted = true;
            return Ok(());
        }
        if let Some(route) = &epistemic.return_evidence {
            let Some(products) = &self.products else {
                return Err(StorageError::InvalidPath(
                    "confirmation requires an exact Belief source relationship".into(),
                ));
            };
            let crate::agent::AgentPreparation::Epoch { subscriptions, .. } = &self.preparation
            else {
                unreachable!("epoch products require epoch preparation");
            };
            let mut returned = None;
            for subscription in products
                .subscription_requests()?
                .into_iter()
                .filter(|request| request.belief_key.dimension_id == route.dimension_id)
            {
                returned = subscriptions.returned_evidence(
                    &crate::belief::BeliefEvidenceReturnRequest {
                        subscription,
                        revision_ids: cut.world_model_view.hydration_refs.revision_ids.clone(),
                        publication_record_id: result.event_record_id(),
                        evidence_schema_id: route.evidence_schema_id.clone(),
                        mapping_revisions: products
                            .specification
                            .genesis
                            .installed_owner_revisions
                            .iter()
                            .filter(|reference| reference.registry == "outcome_mapping")
                            .cloned()
                            .collect(),
                    },
                )?;
                if returned.is_some() {
                    break;
                }
            }
            let Some(returned) = returned else {
                report.waiting_on.push(waiting(
                    "planner_cut_changed",
                    &plan.plan_revision_id,
                    "awaiting Belief evidence from this exact planned confirmation",
                ));
                return Ok(());
            };
            let requirement = PlanMilestoneRequirement::BeliefRevision {
                belief_key: returned.belief_key().index_key(),
                revision_id: returned.revision_id().to_string(),
            };
            let milestone_id = stable_id(
                "agent-belief-return-v1",
                &(
                    &self.goal.goal_id,
                    &plan.plan_revision_id,
                    &epistemic.product_id,
                    &requirement,
                ),
            );
            let inserted = self.store.put_milestone(&AgentMilestoneAcceptance {
                milestone_id: milestone_id.clone(),
                agent_id: cut.context.agent_id.clone(),
                goal_id: self.goal.goal_id.clone(),
                plan_revision_id: plan.plan_revision_id.clone(),
                product_id: epistemic.product_id.clone(),
                requirement,
                owner_position_id: returned.revision_id().to_string(),
                context_id: cut.context.context_id.clone(),
                activation_generation: cut.context.activation_generation.clone(),
            })?;
            self.put_progress(
                cut,
                plan,
                &epistemic.product_id,
                AgentCurrentnessCheck {
                    frozen_cut_id: plan.planner_cut_id.clone(),
                    observed_cut_id: (!self.historical_return).then(|| cut.cut_id.clone()),
                    refusal: None,
                },
                AgentProductState::MilestoneAccepted { milestone_id },
            )?;
            report.milestones_accepted += usize::from(inserted);
        }
        if advance_products {
            self.advance_after_curation(cut, plan, epistemic, &requirement, report)
        } else {
            Ok(())
        }
    }

    fn advance_after_curation(
        &self,
        cut: &PlannerCut,
        plan: &StrategyPlan,
        epistemic: &crate::strategy::StrategyEpistemicOperation,
        requirement: &PlanMilestoneRequirement,
        report: &mut AgentReconciliationReport,
    ) -> Result<(), StorageError> {
        if plan.planner_cut_id != cut.cut_id {
            return Ok(());
        }
        let history = self.store.completed_history_for_goal(&self.goal.goal_id)?;
        if !history.iter().any(|entry| {
            entry.product_id == epistemic.product_id && &entry.accepted_milestone == requirement
        }) {
            return Ok(());
        }
        if let Some(task) = plan.tasks.iter().find(|task| {
            !product_completed(plan, &task.task_id, &history)
                && dependencies_satisfied(plan, &task.task_id, &history)
        }) {
            self.advance_task(cut, plan, task, report)?;
        }
        Ok(())
    }

    fn advance_task(
        &self,
        cut: &PlannerCut,
        plan: &StrategyPlan,
        task: &crate::strategy::StrategyTask,
        report: &mut AgentReconciliationReport,
    ) -> Result<(), StorageError> {
        let (_, progress_id) = self.put_progress(
            cut,
            plan,
            &task.task_id,
            AgentCurrentnessCheck {
                frozen_cut_id: plan.planner_cut_id.clone(),
                observed_cut_id: Some(cut.cut_id.clone()),
                refusal: None,
            },
            AgentProductState::Eligible,
        )?;
        report.eligible_task_ids.push(task.task_id.clone());
        self.authorize_task(cut, plan, task, &progress_id, report)
    }

    fn authorize_task(
        &self,
        cut: &PlannerCut,
        plan: &StrategyPlan,
        task: &crate::strategy::StrategyTask,
        eligible_progress_id: &str,
        report: &mut AgentReconciliationReport,
    ) -> Result<(), StorageError> {
        if task.return_milestone.is_none() {
            report.waiting_on.push(waiting(
                "task_return_milestone_missing",
                eligible_progress_id,
                "Task requires a successor Plan with an exact return milestone",
            ));
            return Ok(());
        }
        let observed = match self.planner.assemble() {
            PlannerAssemblyOutcome::Complete(current) => Some(current.cut_id),
            PlannerAssemblyOutcome::Refused(refusal) => {
                let (_, progress_id) = self.put_progress(
                    cut,
                    plan,
                    &task.task_id,
                    AgentCurrentnessCheck {
                        frozen_cut_id: plan.planner_cut_id.clone(),
                        observed_cut_id: None,
                        refusal: Some(refusal),
                    },
                    AgentProductState::Blocked {
                        reason: "Planner currentness refused before Task authorization".to_string(),
                    },
                )?;
                report.waiting_on.push(waiting(
                    "planner_currentness_refused",
                    progress_id,
                    "wake requires the complete frozen Planner cut",
                ));
                return Ok(());
            }
        };
        let currentness = AgentCurrentnessCheck {
            frozen_cut_id: plan.planner_cut_id.clone(),
            observed_cut_id: observed.clone(),
            refusal: None,
        };
        if observed.as_deref() != Some(plan.planner_cut_id.as_str()) {
            let (_, progress_id) = self.put_progress(
                cut,
                plan,
                &task.task_id,
                currentness,
                AgentProductState::Blocked {
                    reason: "Planner cut changed before Task authorization".to_string(),
                },
            )?;
            report.waiting_on.push(waiting(
                "planner_cut_changed",
                progress_id,
                "wake requires a successor Plan for the observed cut",
            ));
            return Ok(());
        }
        if self.authority.observe()?.as_ref() != Some(&self.frozen_authority) {
            let (_, progress_id) = self.put_progress(
                cut,
                plan,
                &task.task_id,
                currentness,
                AgentProductState::Blocked {
                    reason: "Agent activation or authority policy changed".to_string(),
                },
            )?;
            report.waiting_on.push(waiting(
                "agent_authority_changed",
                progress_id,
                "wake requires reconstruction under the live Agent fence",
            ));
            return Ok(());
        }
        let authorization_id = stable_id(
            "agent-task-authorization-v1",
            &(
                &cut.context.agent_id,
                &self.goal.goal_id,
                &plan.plan_revision_id,
                &task.task_id,
                &cut.context.context_id,
                &cut.context.authority_scope_id,
                &cut.context.activation_generation,
                &task.idempotency_key,
            ),
        );
        let authority_decision = self
            .strategy
            .authority_policy
            .as_ref()
            .map(|policy| {
                evaluate_authority(
                    policy,
                    &self.strategy.package.requested_authority,
                    &task.composition,
                    &self.strategy.subject,
                )
                .map_err(|error| {
                    StorageError::InvalidPath(format!(
                        "Task authority evaluation was denied: {error}"
                    ))
                })
            })
            .transpose()?;
        let authorization = AgentProductAuthorization {
            authorization_id: authorization_id.clone(),
            agent_id: cut.context.agent_id.clone(),
            goal_id: self.goal.goal_id.clone(),
            plan_revision_id: plan.plan_revision_id.clone(),
            product_id: task.task_id.clone(),
            context_id: cut.context.context_id.clone(),
            authority_scope_id: cut.context.authority_scope_id.clone(),
            authority_policy_content_hash: self
                .frozen_authority
                .authority_policy_content_hash
                .clone(),
            authority_decision,
            activation_generation: cut.context.activation_generation.clone(),
            admission_epoch: cut.context.admission_epoch.clone(),
            idempotency_key: task.idempotency_key.clone(),
            product: AgentAuthorizedProduct::Task(Box::new(task.clone())),
            curation_authorization: None,
        };
        let inserted = self.store.put_product_authorization(&authorization)?;
        self.put_progress(
            cut,
            plan,
            &task.task_id,
            currentness,
            AgentProductState::Authorized {
                authorization_id: authorization_id.clone(),
            },
        )?;
        report.products_authorized += usize::from(inserted);
        let position = self.execution.submit(&authorization)?;
        self.persist_execution_position(&authorization, &position)?;
        self.project_execution_wait(cut, plan, task, &position, report)?;
        Ok(())
    }

    fn reconcile_execution(
        &self,
        cut: &PlannerCut,
        plan: &StrategyPlan,
        authorization: &AgentProductAuthorization,
        report: &mut AgentReconciliationReport,
    ) -> Result<(), StorageError> {
        let AgentAuthorizedProduct::Task(task) = &authorization.product else {
            return Err(StorageError::InvalidPath(
                "Execution reconciliation requires one Task authorization".to_string(),
            ));
        };
        if self.authority.observe()?.as_ref() != Some(&self.frozen_authority) {
            report.waiting_on.push(waiting(
                "agent_authority_changed",
                &authorization.authorization_id,
                "wake requires a successor Task authorization under the live fence",
            ));
            return Ok(());
        }
        let position = if cut.cut_id != plan.planner_cut_id {
            // An old authorization may never have reached its consumer. Observing
            // predecessor work must not create its first admission after invalidation.
            let Some(position) = self.execution.observe(authorization)? else {
                return Ok(());
            };
            position
        } else {
            self.execution.advance(authorization)?
        };
        self.persist_execution_position(authorization, &position)?;
        self.project_execution_wait(cut, plan, task, &position, report)
    }

    fn persist_execution_position(
        &self,
        authorization: &AgentProductAuthorization,
        position: &AgentExecutionPosition,
    ) -> Result<bool, StorageError> {
        if position.authorization_id != authorization.authorization_id {
            return Err(StorageError::InvalidPath(
                "Execution position belongs to another Agent authorization".to_string(),
            ));
        }
        let receipt_id = stable_id(
            "agent-execution-receipt-v1",
            &(
                &authorization.authorization_id,
                position,
                &authorization.activation_generation,
            ),
        );
        self.store.put_execution_receipt(&AgentExecutionReceipt {
            receipt_id,
            agent_id: authorization.agent_id.clone(),
            goal_id: authorization.goal_id.clone(),
            plan_revision_id: authorization.plan_revision_id.clone(),
            product_id: authorization.product_id.clone(),
            position: position.clone(),
        })
    }

    fn project_execution_wait(
        &self,
        cut: &PlannerCut,
        plan: &StrategyPlan,
        task: &crate::strategy::StrategyTask,
        position: &AgentExecutionPosition,
        report: &mut AgentReconciliationReport,
    ) -> Result<(), StorageError> {
        match &position.admission_decision {
            AgentExecutionAdmissionDecision::Rejected { grounds } => {
                report.waiting_on.push(waiting(
                    "execution_admission_rejected",
                    &position.admission_id,
                    grounds.join("; "),
                ));
                return Ok(());
            }
            AgentExecutionAdmissionDecision::StaleFence => {
                report.waiting_on.push(waiting(
                    "execution_admission_stale_fence",
                    &position.admission_id,
                    "wake requires a successor Task authorization under the live generation",
                ));
                return Ok(());
            }
            AgentExecutionAdmissionDecision::Admitted => {}
        }
        let state = if let Some(outcome_id) = &position.outcome_id {
            AgentProductState::ExecutionTerminal {
                outcome_id: outcome_id.clone(),
            }
        } else {
            AgentProductState::ExecutionAdmitted {
                admission_id: position.admission_id.clone(),
            }
        };
        let (_, progress_id) = self.put_progress(
            cut,
            plan,
            &task.task_id,
            AgentCurrentnessCheck {
                frozen_cut_id: plan.planner_cut_id.clone(),
                observed_cut_id: (!self.historical_return).then(|| cut.cut_id.clone()),
                refusal: None,
            },
            state,
        )?;
        let Some(requirement) = task.return_milestone.as_ref() else {
            report.waiting_on.push(waiting(
                "task_return_milestone_missing",
                progress_id,
                "wake requires a successor Plan with an exact return milestone",
            ));
            return Ok(());
        };
        let return_position = match requirement {
            PlanMilestoneRequirement::ExecutionTerminal { task_id } if task_id == &task.task_id => {
                position.outcome_id.as_ref()
            }
            _ => None,
        };
        let Some(return_position_id) = return_position else {
            report.waiting_on.push(waiting(
                "execution_terminal_outcome",
                progress_id,
                "awaiting the exact Plan-declared Task return position",
            ));
            return Ok(());
        };
        let milestone_id = stable_id(
            "agent-task-milestone-acceptance-v1",
            &(
                &cut.context.agent_id,
                &self.goal.goal_id,
                &plan.plan_revision_id,
                &task.task_id,
                requirement,
                return_position_id,
                &cut.context.activation_generation,
            ),
        );
        let inserted = self.store.put_milestone(&AgentMilestoneAcceptance {
            milestone_id: milestone_id.clone(),
            agent_id: cut.context.agent_id.clone(),
            goal_id: self.goal.goal_id.clone(),
            plan_revision_id: plan.plan_revision_id.clone(),
            product_id: task.task_id.clone(),
            requirement: requirement.clone(),
            owner_position_id: return_position_id.clone(),
            context_id: cut.context.context_id.clone(),
            activation_generation: cut.context.activation_generation.clone(),
        })?;
        self.put_progress(
            cut,
            plan,
            &task.task_id,
            AgentCurrentnessCheck {
                frozen_cut_id: plan.planner_cut_id.clone(),
                observed_cut_id: (!self.historical_return).then(|| cut.cut_id.clone()),
                refusal: None,
            },
            AgentProductState::MilestoneAccepted { milestone_id },
        )?;
        report.milestones_accepted += usize::from(inserted);
        Ok(())
    }

    fn put_progress(
        &self,
        cut: &PlannerCut,
        plan: &StrategyPlan,
        product_id: &str,
        currentness: AgentCurrentnessCheck,
        state: AgentProductState,
    ) -> Result<(bool, String), StorageError> {
        let progress_id = stable_id(
            "agent-product-progress-v1",
            &(
                &cut.context.agent_id,
                &self.goal.goal_id,
                &plan.plan_revision_id,
                product_id,
                &currentness,
                &state,
            ),
        );
        let inserted = self.store.put_product_progress(&AgentProductProgress {
            progress_id: progress_id.clone(),
            agent_id: cut.context.agent_id.clone(),
            goal_id: self.goal.goal_id.clone(),
            plan_revision_id: plan.plan_revision_id.clone(),
            product_id: product_id.to_string(),
            context_id: cut.context.context_id.clone(),
            authority_scope_id: cut.context.authority_scope_id.clone(),
            activation_generation: cut.context.activation_generation.clone(),
            currentness,
            state,
        })?;
        Ok((inserted, progress_id))
    }
}

fn product_completed(
    plan: &StrategyPlan,
    product_id: &str,
    history: &[crate::strategy::StrategyCompletedHistoryEntry],
) -> bool {
    history.iter().any(|entry| {
        entry.product_id == product_id
            && (plan.tasks.iter().any(|task| {
                task.task_id == product_id
                    && task.return_milestone.as_ref() == Some(&entry.accepted_milestone)
            }) || plan.epistemic_operations.iter().any(|operation| {
                operation.product_id == product_id
                    && operation.accepts_return(&entry.accepted_milestone)
            }))
    })
}

fn dependencies_satisfied(
    plan: &StrategyPlan,
    product_id: &str,
    history: &[crate::strategy::StrategyCompletedHistoryEntry],
) -> bool {
    plan.dependencies.iter().filter(|dependency| dependency.consumer_product_id == product_id)
        .all(|dependency| history.iter().any(|entry|
            entry.product_id == dependency.producer_product_id
                && entry.accepted_milestone == dependency.required_milestone
                // A confirmation must be reconstructed from a successor cut,
                // never authorized against the pre-execution frozen selection.
                && !(matches!(dependency.required_milestone, PlanMilestoneRequirement::ExecutionTerminal { .. })
                    && plan.epistemic_operations.iter().any(|operation| operation.product_id == product_id)
                    && entry.source_plan_revision_id == plan.plan_revision_id)))
}

fn planner_wait(refusal: &PlannerRefusal) -> WaitingOnDeclaration {
    waiting(
        "planner_refusal",
        &refusal.request_context_id,
        format!("Planner refused exact context: {:?}", refusal.grounds),
    )
}

fn waiting(
    condition: impl Into<String>,
    subject_key: impl Into<String>,
    detail: impl Into<String>,
) -> WaitingOnDeclaration {
    let condition = condition.into();
    let subject_key = subject_key.into();
    let wake_addresses = match condition.as_str() {
        "planner_refusal"
        | "goal_disposition"
        | "strategy_refusal"
        | "plan_has_no_epistemic_product"
        | "planner_currentness_refused"
        | "planner_cut_changed"
        | "task_return_milestone_missing" => vec![StructuralWakeAddress::OwnerRevision(format!(
            "planner-input::{subject_key}::successor"
        ))],
        "curation_acceptance" | "curation_terminal_result" => {
            vec![StructuralWakeAddress::DurableOperation(format!(
                "curation-operation::{subject_key}::completion"
            ))]
        }
        "curation_rejected" => vec![StructuralWakeAddress::OwnerRevision(format!(
            "curation-policy::{subject_key}::successor"
        ))],
        "agent_authority_changed" => vec![StructuralWakeAddress::BindingRecovery(format!(
            "agent-authority::{subject_key}"
        ))],
        "execution_admission_rejected" => {
            vec![StructuralWakeAddress::OwnerRevision(format!(
                "execution-admission::{subject_key}::successor"
            ))]
        }
        "execution_admission_stale_fence" => {
            vec![StructuralWakeAddress::BindingRecovery(format!(
                "execution-admission::{subject_key}"
            ))]
        }
        "execution_terminal_outcome" => {
            vec![StructuralWakeAddress::DurableOperation(format!(
                "execution-outcome::{subject_key}::completion"
            ))]
        }
        _ => unreachable!("Agent wait condition lacks a native wake mapping"),
    };
    WaitingOnDeclaration {
        condition,
        subject_key: Some(subject_key),
        detail: detail.into(),
        wake_addresses,
    }
}

fn stable_id(namespace: &str, value: &impl serde::Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("Agent reconciliation identity is serializable");
    format!("{namespace}::{}", blake3::hash(&bytes).to_hex())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

    use meld_events::{DomainObjectRef, LedgerCursor, LedgerIdentity};
    use meld_lang::{
        Condition, GoalLifecycle, GoalPriority, GoalSource, Literal, Proposition, Term, WorldState,
    };

    use super::*;
    use crate::agent::AGENT_RECONCILIATION_RUNTIME_ID;
    use crate::belief::{BranchScope, TheoryRevisionRef};
    use crate::curation::{
        CurationStore, CurationTerminalDisposition, StandingCurationRule,
        StandingCurationRuleRevision, CURATION_OWNER_ID, CURATION_RULE_REGISTRY_ID,
    };
    use crate::planner::{
        PlannerAssemblyPolicy, PlannerDecisionContext, PlannerHydrationRefs,
        PlannerProjectionWarning, PlannerSourceKind, PlannerSourceRef, WorldModelView,
        PLANNER_PROJECTION_VERSION,
    };
    use crate::strategy::{search, StrategySearchRequest, StrategyTheoryPackage};
    use crate::world_state::graph::contracts::*;
    use crate::world_state::graph::PerspectiveKey;

    #[test]
    fn agent_wait_conditions_publish_exact_native_wake_addresses() {
        let cases = [
            (
                "planner_refusal",
                StructuralWakeAddress::OwnerRevision("planner-input::subject::successor".into()),
            ),
            (
                "curation_acceptance",
                StructuralWakeAddress::DurableOperation(
                    "curation-operation::subject::completion".into(),
                ),
            ),
            (
                "curation_rejected",
                StructuralWakeAddress::OwnerRevision("curation-policy::subject::successor".into()),
            ),
            (
                "agent_authority_changed",
                StructuralWakeAddress::BindingRecovery("agent-authority::subject".into()),
            ),
            (
                "execution_admission_rejected",
                StructuralWakeAddress::OwnerRevision(
                    "execution-admission::subject::successor".into(),
                ),
            ),
            (
                "execution_admission_stale_fence",
                StructuralWakeAddress::BindingRecovery("execution-admission::subject".into()),
            ),
            (
                "execution_terminal_outcome",
                StructuralWakeAddress::DurableOperation(
                    "execution-outcome::subject::completion".into(),
                ),
            ),
        ];
        for (condition, expected) in cases {
            let declaration = waiting(condition, "subject", "detail");
            assert_eq!(declaration.wake_addresses, vec![expected]);
        }
        assert!(std::panic::catch_unwind(|| waiting("unknown", "subject", "detail")).is_err());
    }

    struct PlannerSequence {
        outcomes: Mutex<VecDeque<PlannerAssemblyOutcome>>,
        fallback: PlannerAssemblyOutcome,
    }

    impl AgentPlannerPort for PlannerSequence {
        fn assemble(&self) -> PlannerAssemblyOutcome {
            self.outcomes
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| self.fallback.clone())
        }
    }

    struct FixedAuthority(AgentAuthorizationFence);

    impl AgentAuthorityPort for FixedAuthority {
        fn observe(&self) -> Result<Option<AgentAuthorizationFence>, StorageError> {
            Ok(Some(self.0.clone()))
        }
    }

    struct PendingExecution;

    impl PendingExecution {
        fn position(
            authorization: &AgentProductAuthorization,
        ) -> Result<AgentExecutionPosition, StorageError> {
            if !matches!(authorization.product, AgentAuthorizedProduct::Task(_)) {
                return Err(StorageError::InvalidPath(
                    "Execution test port accepts only Task products".to_string(),
                ));
            }
            Ok(AgentExecutionPosition {
                authorization_id: authorization.authorization_id.clone(),
                admission_id: format!("admission::{}", authorization.authorization_id),
                admission_decision: AgentExecutionAdmissionDecision::Admitted,
                admission_revision: 1,
                network_commit_revision: None,
                outcome_id: None,
                execution_publication_position_id: None,
            })
        }
    }

    impl AgentExecutionPort for PendingExecution {
        fn observe(
            &self,
            authorization: &AgentProductAuthorization,
        ) -> Result<Option<AgentExecutionPosition>, StorageError> {
            Self::position(authorization).map(Some)
        }

        fn submit(
            &self,
            authorization: &AgentProductAuthorization,
        ) -> Result<AgentExecutionPosition, StorageError> {
            Self::position(authorization)
        }

        fn advance(
            &self,
            authorization: &AgentProductAuthorization,
        ) -> Result<AgentExecutionPosition, StorageError> {
            Self::position(authorization)
        }
    }

    struct TerminalExecution {
        submissions: Mutex<Vec<String>>,
    }

    impl AgentExecutionPort for TerminalExecution {
        fn submit(
            &self,
            authorization: &AgentProductAuthorization,
        ) -> Result<AgentExecutionPosition, StorageError> {
            self.submissions
                .lock()
                .unwrap()
                .push(authorization.authorization_id.clone());
            PendingExecution::position(authorization)
        }
        fn observe(
            &self,
            authorization: &AgentProductAuthorization,
        ) -> Result<Option<AgentExecutionPosition>, StorageError> {
            self.advance(authorization).map(Some)
        }
        fn advance(
            &self,
            authorization: &AgentProductAuthorization,
        ) -> Result<AgentExecutionPosition, StorageError> {
            let mut position = PendingExecution::position(authorization)?;
            position.outcome_id = Some(format!("outcome::{}", authorization.authorization_id));
            position.execution_publication_position_id = Some("published-task-outcome".into());
            Ok(position)
        }
    }

    #[test]
    fn completed_task_advances_to_confirmation_in_a_live_successor_without_reexecution() {
        let fixture = Fixture::new();
        let (mut actor, curation) = fixture.actor(
            vec![PlannerAssemblyOutcome::Complete(Box::new(
                fixture.cut.clone(),
            ))],
            true,
        );
        for rule in &mut actor.strategy.package.snapshot.settlement_rules {
            rule.epistemic_placement = crate::strategy::StrategyEpistemicPlacement::Confirmation;
        }
        let execution = Arc::new(TerminalExecution {
            submissions: Mutex::new(Vec::new()),
        });
        actor.execution = execution.clone();
        let initial = actor.bounded_step(8);
        assert!(initial.fatal_errors.is_empty(), "{initial:?}");
        assert_eq!(initial.products_authorized, 1);
        assert!(curation.operation.lock().unwrap().is_none());
        assert_eq!(execution.submissions.lock().unwrap().len(), 1);
        let predecessor = fixture
            .store
            .current_reconciliation_plan(&fixture.goal.goal_id)
            .unwrap()
            .unwrap();
        let completed = actor.bounded_step(8);
        assert!(completed.retryable_errors.is_empty(), "{completed:?}");
        assert_eq!(completed.milestones_accepted, 1);
        assert!(curation.operation.lock().unwrap().is_none());
        let frozen = actor.bounded_step(8);
        assert_eq!(frozen.products_authorized, 0);
        assert!(curation.operation.lock().unwrap().is_none());
        change_planner(&mut actor, &fixture.cut);
        let confirmation = actor.bounded_step(8);
        assert!(confirmation.fatal_errors.is_empty(), "{confirmation:?}");
        assert!(confirmation
            .retryable_errors
            .iter()
            .any(|error| error.contains("Belief source relationship")));
        assert_eq!(confirmation.products_authorized, 1);
        assert_eq!(confirmation.milestones_accepted, 1);
        let successor = fixture
            .store
            .current_reconciliation_plan(&fixture.goal.goal_id)
            .unwrap()
            .unwrap();
        assert!(successor.tasks.is_empty());
        assert_eq!(
            successor.predecessor_plan_revision_id.as_ref(),
            Some(&predecessor.plan_revision_id)
        );
        assert!(curation.operation.lock().unwrap().is_some());
        let history = fixture
            .store
            .completed_history_for_goal(&fixture.goal.goal_id)
            .unwrap();
        assert_eq!(history.len(), 2);
        assert!(history.iter().any(|entry| entry.source_plan_revision_id
            == predecessor.plan_revision_id
            && entry.product
                == Some(AgentAuthorizedProduct::Task(Box::new(
                    predecessor.tasks[0].clone()
                )))));
        actor.store = Arc::new(AgentStore::new(fixture.db.clone()).unwrap());
        let replay = actor.bounded_step(8);
        assert_eq!(replay.products_authorized, 0);
        assert_eq!(execution.submissions.lock().unwrap().len(), 1);
        assert!(replay
            .retryable_errors
            .iter()
            .any(|error| error.contains("Belief source relationship")));
        assert!(fixture
            .store
            .goal_disposition_for_plan(&successor.plan_revision_id)
            .unwrap()
            .is_none());
    }

    struct ImmediateCuration {
        rule: StandingCurationRuleRevision,
        operation: Mutex<Option<CurationOperation>>,
        terminal: bool,
    }

    impl AgentCurationPort for ImmediateCuration {
        fn resolve_operation(
            &self,
            candidate: CurationOperation,
        ) -> Result<CurationOperation, StorageError> {
            Ok(self
                .operation
                .lock()
                .unwrap()
                .as_ref()
                .filter(|prior| prior.selection_id == candidate.selection_id)
                .cloned()
                .unwrap_or(candidate)
                .semantic_operation())
        }
        fn submit(&self, operation: CurationOperation) -> Result<(), StorageError> {
            let mut current = self.operation.lock().unwrap();
            if let Some(existing) = current.as_ref() {
                if existing != &operation {
                    return Err(StorageError::InvalidPath(
                        "Curation replay changed the authorized operation".to_string(),
                    ));
                }
            } else {
                *current = Some(operation);
            }
            Ok(())
        }

        fn acceptance(
            &self,
            operation_id: &str,
        ) -> Result<Option<CurationAcceptanceRecord>, StorageError> {
            self.operation
                .lock()
                .unwrap()
                .as_ref()
                .filter(|operation| operation.operation_id == operation_id)
                .map(|operation| CurationAcceptanceRecord::for_operation(operation, &self.rule))
                .transpose()
        }

        fn result(&self, operation_id: &str) -> Result<Option<CurationResult>, StorageError> {
            if !self.terminal {
                return Ok(None);
            }
            self.operation
                .lock()
                .unwrap()
                .as_ref()
                .filter(|operation| operation.operation_id == operation_id)
                .map(|operation| {
                    CurationResult::new(
                        operation,
                        CurationTerminalDisposition::Abstained,
                        "canonical Curation declined semantic authorship",
                        None,
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                    )
                })
                .transpose()
        }
    }

    struct DurableCuration {
        store: Arc<CurationStore>,
    }

    impl AgentCurationPort for DurableCuration {
        fn resolve_operation(
            &self,
            candidate: CurationOperation,
        ) -> Result<CurationOperation, StorageError> {
            self.store.resolve_operation(candidate)
        }
        fn submit(&self, operation: CurationOperation) -> Result<(), StorageError> {
            self.store.submit_planned(&operation)
        }

        fn acceptance(
            &self,
            operation_id: &str,
        ) -> Result<Option<CurationAcceptanceRecord>, StorageError> {
            self.store.acceptance_for_planned_operation(operation_id)
        }

        fn result(&self, operation_id: &str) -> Result<Option<CurationResult>, StorageError> {
            self.store.result_for_operation(operation_id)
        }
    }

    struct Fixture {
        _temp: tempfile::TempDir,
        db: sled::Db,
        store: Arc<AgentStore>,
        cut: PlannerCut,
        goal: Goal,
        strategy: AgentStrategyRuntimeConfig,
        authority: CurationAuthority,
        rule: StandingCurationRuleRevision,
    }

    impl Fixture {
        fn new() -> Self {
            let temp = tempfile::tempdir().unwrap();
            let db = sled::open(temp.path().join("world_model.sled")).unwrap();
            let store = Arc::new(AgentStore::new(db.clone()).unwrap());
            let authority = authority();
            let rule_body = rule();
            let rule = CurationStore::new(db.clone())
                .unwrap()
                .install_rule(rule_body.clone(), 1)
                .unwrap();
            let cut = planner_cut(&rule_body);
            let goal = goal();
            let package: StrategyTheoryPackage = serde_json::from_str(include_str!(
                "../../../../theory/docs_freshness/strategy_theory.docs_freshness.json"
            ))
            .unwrap();
            let strategy =
                AgentStrategyRuntimeConfig::activate_installed(package, subject(), "agent-docs")
                    .unwrap();
            Self {
                _temp: temp,
                db,
                store,
                cut,
                goal,
                strategy,
                authority,
                rule,
            }
        }

        fn actor(
            &self,
            outcomes: Vec<PlannerAssemblyOutcome>,
            terminal: bool,
        ) -> (AgentReconciliationActor, Arc<ImmediateCuration>) {
            self.actor_with_fence(
                outcomes,
                terminal,
                AgentAuthorizationFence {
                    activation_generation: self.authority.activation_generation.clone(),
                    admission_epoch: None,
                    authority_policy_content_hash: "authority-docs-v1".to_string(),
                },
            )
        }

        fn actor_with_fence(
            &self,
            outcomes: Vec<PlannerAssemblyOutcome>,
            terminal: bool,
            observed: AgentAuthorizationFence,
        ) -> (AgentReconciliationActor, Arc<ImmediateCuration>) {
            let fallback = outcomes.last().cloned().unwrap();
            let planner = Arc::new(PlannerSequence {
                outcomes: Mutex::new(outcomes.into()),
                fallback,
            });
            let curation = Arc::new(ImmediateCuration {
                rule: self.rule.clone(),
                operation: Mutex::new(None),
                terminal,
            });
            let actor = AgentReconciliationActor::new(
                AGENT_RECONCILIATION_RUNTIME_ID,
                self.goal.clone(),
                Arc::clone(&self.store),
                planner,
                Arc::new(FixedAuthority(observed)),
                AgentAuthorizationFence {
                    activation_generation: self.authority.activation_generation.clone(),
                    admission_epoch: None,
                    authority_policy_content_hash: "authority-docs-v1".to_string(),
                },
                Arc::clone(&curation) as Arc<dyn AgentCurationPort>,
                Arc::new(PendingExecution),
                self.strategy.clone(),
                self.authority.clone(),
                self.rule.clone(),
            )
            .unwrap();
            (actor, curation)
        }

        fn expected_plan(&self) -> StrategyPlan {
            let operation = CurationOperation::reconstruct(
                self.authority.clone(),
                self.rule.revision_ref(),
                self.cut.traversal_cut.clone(),
                self.cut.traversal_request.clone(),
            )
            .unwrap();
            let problem =
                self.strategy
                    .problem(self.goal.clone(), self.cut.clone(), vec![operation]);
            search(&StrategySearchRequest {
                problem,
                bounds: self.strategy.bounds(),
            })
            .recommendation
            .unwrap()
        }
    }

    #[test]
    fn reconciliation_authorizes_task_and_persists_execution_admission() {
        let fixture = Fixture::new();
        let plan = fixture.expected_plan();
        let (actor, _) = fixture.actor(
            vec![
                PlannerAssemblyOutcome::Complete(Box::new(fixture.cut.clone())),
                PlannerAssemblyOutcome::Complete(Box::new(fixture.cut.clone())),
            ],
            true,
        );

        let report = actor.bounded_step(8);

        assert!(report.fatal_errors.is_empty(), "{:?}", report.fatal_errors);
        assert_eq!(report.products_authorized, 2);
        assert_eq!(report.milestones_accepted, 1, "{report:?}");
        assert_eq!(
            report.eligible_task_ids,
            vec![plan.tasks[0].task_id.clone()]
        );
        assert_eq!(
            report.waiting_on.last().unwrap().condition,
            "execution_terminal_outcome"
        );
        assert!(fixture
            .store
            .reconciliation_goal(&fixture.goal.goal_id)
            .unwrap()
            .is_some());
        assert_eq!(
            fixture
                .store
                .reconciliation_plan(&plan.plan_revision_id)
                .unwrap(),
            Some(plan.clone())
        );
        let judgment_id = stable_id(
            "agent-plan-judgment-v1",
            &(
                "agent-docs",
                &fixture.goal.goal_id,
                &plan.plan_revision_id,
                &fixture.cut.context.context_id,
                &fixture.cut.context.authority_scope_id,
                &fixture.cut.context.activation_generation,
                "admitted",
            ),
        );
        let judgment = fixture.store.plan_judgment(&judgment_id).unwrap().unwrap();
        assert_eq!(judgment.kind, AgentPlanJudgmentKind::Admitted);
        let epistemic = &plan.epistemic_operations[0];
        let authorization_id = stable_id(
            "agent-product-authorization-v1",
            &(
                "agent-docs",
                &fixture.goal.goal_id,
                &plan.plan_revision_id,
                &epistemic.product_id,
                &epistemic.operation.operation_id,
                &fixture.cut.context.context_id,
                &fixture.cut.context.authority_scope_id,
                &fixture.cut.context.activation_generation,
                &epistemic.idempotency_key,
            ),
        );
        let authorization = fixture
            .store
            .product_authorization(&authorization_id)
            .unwrap()
            .unwrap();
        assert_ne!(judgment.judgment_id, authorization.authorization_id);
        let result = CurationResult::new(
            &epistemic.operation,
            CurationTerminalDisposition::Abstained,
            "terminal",
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let milestone_id = stable_id(
            "agent-milestone-acceptance-v1",
            &(
                "agent-docs",
                &fixture.goal.goal_id,
                &plan.plan_revision_id,
                &epistemic.product_id,
                &result.result_id,
                &fixture.cut.context.context_id,
                &fixture.cut.context.activation_generation,
            ),
        );
        let milestone = fixture.store.milestone(&milestone_id).unwrap().unwrap();
        assert_eq!(
            milestone.requirement,
            PlanMilestoneRequirement::CurationTerminal {
                operation_id: epistemic.operation.operation_id.clone(),
            }
        );
        assert!(fixture
            .db
            .open_tree("execution_goal_records")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn pending_curation_wait_names_the_operation_observed_by_the_bound_agent() {
        let fixture = Fixture::new();
        let (actor, _) = fixture.actor(
            vec![PlannerAssemblyOutcome::Complete(Box::new(
                fixture.cut.clone(),
            ))],
            false,
        );
        let report = actor.bounded_step(8);
        let wait = report
            .waiting_on
            .iter()
            .find(|wait| wait.condition == "curation_terminal_result")
            .unwrap();
        assert!(actor.resolves_wake(&wait.wake_addresses[0]).unwrap());
        let foreign = StructuralWakeAddress::DurableOperation(format!(
            "world-model::{}::curation-operation::foreign-operation::completion",
            fixture.store.resource_id(),
        ));
        assert!(!actor.resolves_wake(&foreign).unwrap());
        assert_eq!(report.milestones_accepted, 0);
    }

    #[test]
    fn changed_current_cut_blocks_authorization_before_curation_submission() {
        let fixture = Fixture::new();
        let mut changed = fixture.cut.clone();
        changed.cut_id = "newer-planner-cut".to_string();
        let plan = fixture.expected_plan();
        let (actor, curation) = fixture.actor(
            vec![
                PlannerAssemblyOutcome::Complete(Box::new(fixture.cut.clone())),
                PlannerAssemblyOutcome::Complete(Box::new(changed)),
            ],
            true,
        );

        let report = actor.bounded_step(8);

        assert_eq!(report.products_authorized, 0);
        assert!(curation.operation.lock().unwrap().is_none());
        assert!(fixture
            .store
            .reconciliation_plan(&plan.plan_revision_id)
            .unwrap()
            .is_some());
        let epistemic = &plan.epistemic_operations[0];
        let state = AgentProductState::Blocked {
            reason: "Planner cut changed before authorization".to_string(),
        };
        let currentness = AgentCurrentnessCheck {
            frozen_cut_id: fixture.cut.cut_id.clone(),
            observed_cut_id: Some("newer-planner-cut".to_string()),
            refusal: None,
        };
        let progress_id = stable_id(
            "agent-product-progress-v1",
            &(
                "agent-docs",
                &fixture.goal.goal_id,
                &plan.plan_revision_id,
                &epistemic.product_id,
                &currentness,
                &state,
            ),
        );
        assert_eq!(
            fixture
                .store
                .product_progress(&progress_id)
                .unwrap()
                .unwrap()
                .state,
            state
        );
    }

    struct MultiCuration {
        rule: StandingCurationRuleRevision,
        operations: Mutex<std::collections::BTreeMap<String, CurationOperation>>,
    }

    impl AgentCurationPort for MultiCuration {
        fn resolve_operation(
            &self,
            candidate: CurationOperation,
        ) -> Result<CurationOperation, StorageError> {
            Ok(self
                .operations
                .lock()
                .unwrap()
                .values()
                .find(|prior| prior.selection_id == candidate.selection_id)
                .cloned()
                .unwrap_or(candidate)
                .semantic_operation())
        }
        fn submit(&self, operation: CurationOperation) -> Result<(), StorageError> {
            let mut operations = self.operations.lock().unwrap();
            if let Some(prior) = operations.get(&operation.operation_id) {
                assert_eq!(prior, &operation);
            } else {
                operations.insert(operation.operation_id.clone(), operation);
            }
            Ok(())
        }
        fn acceptance(&self, id: &str) -> Result<Option<CurationAcceptanceRecord>, StorageError> {
            self.operations
                .lock()
                .unwrap()
                .get(id)
                .map(|operation| CurationAcceptanceRecord::for_operation(operation, &self.rule))
                .transpose()
        }
        fn result(&self, id: &str) -> Result<Option<CurationResult>, StorageError> {
            self.operations
                .lock()
                .unwrap()
                .get(id)
                .map(|operation| {
                    CurationResult::new(
                        operation,
                        CurationTerminalDisposition::Abstained,
                        "bounded terminal observation",
                        None,
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                    )
                })
                .transpose()
        }
    }

    fn change_planner(actor: &mut AgentReconciliationActor, original: &PlannerCut) -> PlannerCut {
        let mut changed = original.clone();
        changed.traversal_cut.receipts[0].revision_id = "workspace-v2".into();
        changed.traversal_cut.event_position.after_seq += 1;
        changed.traversal_cut.graph_position.after_seq += 1;
        changed.traversal_cut.cut_id = traversal_cut_identity(&changed.traversal_cut).unwrap();
        changed.cut_id = format!("successor::{}", original.cut_id);
        actor.planner = Arc::new(PlannerSequence {
            outcomes: Mutex::new(VecDeque::new()),
            fallback: PlannerAssemblyOutcome::Complete(Box::new(changed.clone())),
        });
        changed
    }

    #[test]
    fn changed_knowledge_reconstructs_the_running_agent_and_preserves_completed_history() {
        let fixture = Fixture::new();
        let (mut actor, _) = fixture.actor(
            vec![PlannerAssemblyOutcome::Complete(Box::new(
                fixture.cut.clone(),
            ))],
            true,
        );
        let curation = Arc::new(MultiCuration {
            rule: fixture.rule.clone(),
            operations: Mutex::new(Default::default()),
        });
        actor.curation = curation.clone();
        // Stop after accepting Curation's milestone, before Task authorization.
        let first = actor.bounded_step(5);
        assert!(first.fatal_errors.is_empty(), "{first:?}");
        assert_eq!(first.products_authorized, 1);
        assert_eq!(first.milestones_accepted, 1);
        let predecessor = fixture
            .store
            .current_reconciliation_plan(&fixture.goal.goal_id)
            .unwrap()
            .unwrap();
        let history = fixture
            .store
            .completed_history_for_goal(&fixture.goal.goal_id)
            .unwrap();
        assert_eq!(history.len(), 1);
        let changed = change_planner(&mut actor, &fixture.cut);
        let result = actor.bounded_step(8);
        assert!(result.fatal_errors.is_empty(), "{result:?}");
        assert!(result.retryable_errors.is_empty(), "{result:?}");
        let successor = fixture
            .store
            .current_reconciliation_plan(&fixture.goal.goal_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            successor.predecessor_plan_revision_id.as_ref(),
            Some(&predecessor.plan_revision_id)
        );
        assert_eq!(successor.planner_cut_id, changed.cut_id);
        assert_eq!(
            fixture
                .store
                .reconciliation_plan_history(&successor.plan_revision_id)
                .unwrap(),
            history
        );
        assert_eq!(
            fixture
                .store
                .reconciliation_plan(&predecessor.plan_revision_id)
                .unwrap(),
            Some(predecessor.clone())
        );
        let authorizations = fixture
            .store
            .product_authorizations_for_goal(&fixture.goal.goal_id)
            .unwrap();
        let tasks: Vec<_> = authorizations
            .iter()
            .filter(|record| matches!(record.product, AgentAuthorizedProduct::Task(_)))
            .collect();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].plan_revision_id, successor.plan_revision_id);
        // Restart the owner over its durable state. Hash-sorted predecessor
        // authorizations cannot seize the current Plan or cause new work.
        actor.store = Arc::new(AgentStore::new(fixture.db.clone()).unwrap());
        let replay = actor.bounded_step(8);
        assert!(replay.fatal_errors.is_empty(), "{replay:?}");
        assert_eq!(replay.products_authorized, 0);
        assert_eq!(replay.records_persisted, 0);
        assert_eq!(curation.operations.lock().unwrap().len(), 2);
        let goal_id = fixture.goal.goal_id.clone();
        drop(actor);
        drop(curation);
        let Fixture {
            _temp, db, store, ..
        } = fixture;
        drop(store);
        drop(db);
        let reopened =
            AgentStore::new(sled::open(_temp.path().join("world_model.sled")).unwrap()).unwrap();
        assert_eq!(
            reopened.current_reconciliation_plan(&goal_id).unwrap(),
            Some(successor.clone())
        );
        assert_eq!(
            reopened
                .reconciliation_plan_history(&successor.plan_revision_id)
                .unwrap(),
            history
        );
    }

    #[test]
    fn changed_knowledge_does_not_duplicate_an_unresolved_execution_effect() {
        let fixture = Fixture::new();
        let (mut actor, _) = fixture.actor(
            vec![PlannerAssemblyOutcome::Complete(Box::new(
                fixture.cut.clone(),
            ))],
            true,
        );
        assert_eq!(actor.bounded_step(8).products_authorized, 2);
        let predecessor = fixture
            .store
            .current_reconciliation_plan(&fixture.goal.goal_id)
            .unwrap()
            .unwrap();
        change_planner(&mut actor, &fixture.cut);
        let result = actor.bounded_step(8);
        assert_eq!(result.products_authorized, 0);
        assert!(result
            .waiting_on
            .iter()
            .any(|wait| wait.condition == "execution_terminal_outcome"));
        assert_eq!(
            fixture
                .store
                .current_reconciliation_plan(&fixture.goal.goal_id)
                .unwrap(),
            Some(predecessor)
        );
    }

    #[test]
    fn successor_refusal_keeps_old_evidence_without_authorizing_old_products() {
        let fixture = Fixture::new();
        let (mut actor, _) = fixture.actor(
            vec![PlannerAssemblyOutcome::Complete(Box::new(
                fixture.cut.clone(),
            ))],
            true,
        );
        actor.bounded_step(5);
        let predecessor = fixture
            .store
            .current_reconciliation_plan(&fixture.goal.goal_id)
            .unwrap()
            .unwrap();
        change_planner(&mut actor, &fixture.cut);
        actor.strategy.package.snapshot.settlement_rules.clear();
        let result = actor.bounded_step(8);
        assert_eq!(result.products_authorized, 0);
        assert!(result
            .waiting_on
            .iter()
            .any(|wait| wait.condition == "strategy_refusal"));
        assert_eq!(
            fixture
                .store
                .current_reconciliation_plan(&fixture.goal.goal_id)
                .unwrap(),
            Some(predecessor)
        );
    }

    fn satisfied_cut(original: &PlannerCut) -> PlannerCut {
        let mut cut = original.clone();
        cut.cut_id = "planner-satisfied-docs".into();
        cut.world_model_view.world_state = WorldState::new(vec![Proposition::Holds {
            subject: Term::Object(subject()),
            dimension: Term::Dimension("docs_freshness".into()),
            condition: Condition::Equals(Term::Literal(Literal::Number(1.0))),
        }])
        .unwrap();
        cut
    }

    struct MutableEpochAuthority(Mutex<Option<AgentAuthorizationFence>>);
    impl AgentAuthorityPort for MutableEpochAuthority {
        fn observe(&self) -> Result<Option<AgentAuthorizationFence>, StorageError> {
            Ok(self.0.lock().unwrap().clone())
        }
    }

    struct EpochTestPlanner(PlannerCut);
    impl AgentPlannerPort for EpochTestPlanner {
        fn assemble(&self) -> PlannerAssemblyOutcome {
            PlannerAssemblyOutcome::Complete(Box::new(self.0.clone()))
        }
        fn assemble_for(
            &self,
            goal_id: &str,
            fence: &AgentAuthorizationFence,
        ) -> PlannerAssemblyOutcome {
            let mut cut = self.0.clone();
            cut.context.goal_id = goal_id.into();
            cut.context.context_id = format!("context::{goal_id}");
            cut.context.activation_generation = fence.activation_generation.clone();
            cut.context.admission_epoch = fence.admission_epoch.clone();
            cut.cut_id = stable_id("test-epoch-cut", &cut.context);
            PlannerAssemblyOutcome::Complete(Box::new(cut))
        }
    }

    struct EpochPreparedPlanner(PlannerCut);
    impl AgentPlannerPort for EpochPreparedPlanner {
        fn assemble(&self) -> PlannerAssemblyOutcome {
            panic!("epoch selection is required")
        }
        fn assemble_epoch(
            &self,
            products: &crate::agent::AgentEpochProducts,
        ) -> PlannerAssemblyOutcome {
            let mut cut = self.0.clone();
            let specification = &products.specification;
            cut.context.goal_id = specification.goal_id.clone();
            cut.context.context_id = format!("agent-context::{}", specification.goal_id);
            cut.context.activation_generation = specification.fence.activation_generation.clone();
            cut.context.admission_epoch = specification.fence.admission_epoch.clone();
            cut.context.observation_subject = Some(products.observation_subject.clone());
            cut.traversal_request = products.curation_rule.rule.traversal_request();
            cut.cut_id = stable_id("epoch-prepared-test-cut", &cut.context);
            PlannerAssemblyOutcome::Complete(Box::new(cut))
        }
    }

    struct PreparedEpochSource {
        calls: Mutex<usize>,
        curation: Arc<CurationStore>,
        rule: StandingCurationRule,
    }
    impl crate::agent::AgentEpochPreparationPort for PreparedEpochSource {
        fn prepare(
            &self,
            specification: &crate::agent::AgentEpochSpecification,
        ) -> Result<crate::agent::AgentEpochProducts, StorageError> {
            let sequence = {
                let mut calls = self.calls.lock().unwrap();
                *calls += 1;
                *calls as u64 + 1
            };
            let mut rule = self.rule.clone();
            rule.expected_object_id = specification.specification_id.clone();
            rule.rule_id = format!("rule::{}", specification.specification_id);
            let rule = self.curation.install_rule(rule, sequence)?;
            Ok(crate::agent::AgentEpochProducts {
                specification: specification.clone(),
                observation_subject: rule.rule.expected_object()?,
                curation_rule: rule,
                task_inputs: vec![meld_lang::TaskInput {
                    step_id: "inspect-docs-scope".into(),
                    slot_id: "epoch_input".into(),
                    artifact_type_id: "epoch_input".into(),
                    schema_version: 1,
                    content: serde_json::json!({"specification": specification.specification_id}),
                }],
            })
        }
    }

    #[test]
    fn native_epoch_specification_freezes_inputs_and_separates_observation_from_authority() {
        use crate::agent::*;
        let fixture = Fixture::new();
        let (mut actor, _) = fixture.actor(
            vec![PlannerAssemblyOutcome::Complete(Box::new(
                fixture.cut.clone(),
            ))],
            false,
        );
        let (_, condition) = AgentMaintainedConditionRegistryStore::new(fixture.db.clone())
            .unwrap()
            .install(
                AgentMaintainedCondition {
                    condition_id: "epoch-condition".into(),
                    dimension_id: "docs_freshness".into(),
                    desired: Condition::Above(Term::Literal(Literal::Number(0.7))),
                    goal_priority: fixture.goal.priority.clone(),
                    desired_summary: "current epoch evidence".into(),
                    observation_scope: crate::agent::AgentObservationScope::AdmissionEpoch,
                },
                1,
            )
            .unwrap();
        let condition = condition.binding().unwrap();
        actor.intent = AgentReconciliationIntent::MaintainedCondition(condition.clone());
        use crate::belief::BeliefFamilyRegistry;
        let mut registry =
            crate::belief::BeliefFamilyRegistryStore::new(fixture.db.clone()).unwrap();
        let family = registry
            .install(
                serde_json::from_str(include_str!(
                    "../../../../theory/docs_freshness/belief_family.docs_freshness.json"
                ))
                .unwrap(),
                1,
            )
            .unwrap()
            .1;
        let belief_key = crate::belief::configured_belief_key(
            &family,
            &subject(),
            &fixture.authority.perspective,
            &fixture.authority.branch_scope,
        );
        let subscription = AgentSubscriptionRequestV1::new(
            actor.strategy.agent_id.clone(),
            "belief".into(),
            family.revision_ref(),
            belief_key,
            "from_genesis".into(),
        )
        .unwrap();
        let genesis = AgentGenesisIntentV1::new(
            "assignment".into(),
            "compilation".into(),
            "steward".into(),
            vec![
                condition.revision.clone(),
                subscription.source_contract_revision.clone(),
            ],
            SeedAgentRegistration {
                agent_id: actor.strategy.agent_id.clone(),
                perspective_key: fixture.authority.perspective.clone(),
                subject: subject(),
                branch_scope: fixture.authority.branch_scope.clone(),
                observation_scope: "meld".into(),
                directive: "maintain current epoch".into(),
                seed_provenance: "test".into(),
                curation_rule: None,
                curation_rule_revision: None,
                maintained_condition: Some(condition.clone()),
                maintained_condition_revision: Some(condition.revision.clone()),
                created_at_seq: 1,
            },
            vec![subscription],
        )
        .unwrap();
        fixture
            .store
            .claim_genesis_lineage(&actor.strategy.agent_id, &genesis.intent_id)
            .unwrap();
        fixture.store.put_genesis_intent(&genesis).unwrap();
        crate::agent::registration::AgentRegistration::new(&fixture.store)
            .register_seed_agent(genesis.registration.clone())
            .unwrap();
        let source = Arc::new(PreparedEpochSource {
            calls: Mutex::new(0),
            curation: Arc::new(CurationStore::new(fixture.db.clone()).unwrap()),
            rule: fixture.rule.rule.clone(),
        });
        actor.strategy.package.capabilities[0]
            .operator
            .resolution
            .requires_inputs
            .push(meld_lang::SlotConstraint {
                artifact_type: Term::ArtifactType("epoch_input".into()),
                required: true,
            });
        for rule in &mut actor.strategy.package.snapshot.settlement_rules {
            rule.epistemic_placement = crate::strategy::StrategyEpistemicPlacement::Confirmation;
        }
        let mut fence = actor.frozen_authority.clone();
        fence.admission_epoch = Some("epoch-one".into());
        let observer = Arc::new(MutableEpochAuthority(Mutex::new(Some(fence.clone()))));
        actor.authority = observer.clone();
        actor.planner = Arc::new(EpochPreparedPlanner(fixture.cut.clone()));
        actor.curation = Arc::new(DurableCuration {
            store: source.curation.clone(),
        });
        actor.preparation = AgentPreparation::Epoch {
            products: source.clone(),
            subscriptions: Arc::new(crate::belief::BeliefSubscriptionSource::new(
                Arc::new(crate::belief::BeliefStore::new(fixture.db.clone()).unwrap()),
                Arc::new(registry),
            )),
        };
        let report = actor.bounded_step(8);
        assert!(
            report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.products_authorized, 1, "{report:?}");
        let goal_id = actor
            .intent
            .goal_id(&actor.strategy.agent_id, &fence.reconciliation_scope());
        let products = fixture.store.epoch_products(&goal_id).unwrap().unwrap();
        assert_eq!(products.specification.genesis, genesis);
        assert_eq!(
            products.specification.authority.subject,
            actor.strategy.subject
        );
        assert_ne!(products.observation_subject, actor.strategy.subject);
        let goal = fixture
            .store
            .reconciliation_goal(&goal_id)
            .unwrap()
            .unwrap()
            .goal;
        assert_eq!(
            goal.target,
            condition
                .condition
                .target_for(products.observation_subject.clone())
                .unwrap()
        );
        let plan = fixture
            .store
            .current_reconciliation_plan(&goal_id)
            .unwrap()
            .unwrap();
        assert_eq!(plan.tasks[0].initial_inputs, products.task_inputs);
        let mut substituted = products.clone();
        substituted.task_inputs[0].content = serde_json::json!({"foreign": true});
        assert!(fixture.store.put_epoch_products(&substituted).is_err());
        let mut foreign = products.clone();
        foreign.specification.genesis.product_compilation_receipt_id = "foreign".into();
        assert!(foreign.validate().is_err());

        actor.store = Arc::new(AgentStore::new(fixture.db.clone()).unwrap());
        assert_eq!(actor.bounded_step(8).products_authorized, 0);
        assert_eq!(*source.calls.lock().unwrap(), 1);
        *observer.0.lock().unwrap() = None;
        assert_eq!(actor.bounded_step(8).products_authorized, 0);
        assert_eq!(*source.calls.lock().unwrap(), 1);
        fence.admission_epoch = Some("epoch-two".into());
        *observer.0.lock().unwrap() = Some(fence.clone());
        let next = actor.bounded_step(8);
        assert!(
            next.fatal_errors.is_empty() && next.retryable_errors.is_empty(),
            "{next:?}"
        );
        assert_eq!(next.products_authorized, 1);
        let next_goal = actor
            .intent
            .goal_id(&actor.strategy.agent_id, &fence.reconciliation_scope());
        let next_products = actor.store.epoch_products(&next_goal).unwrap().unwrap();
        assert_ne!(
            products.observation_subject,
            next_products.observation_subject
        );
        assert_ne!(products.task_inputs, next_products.task_inputs);
        assert_eq!(
            actor.store.epoch_products(&goal_id).unwrap().unwrap(),
            products
        );
        assert_eq!(*source.calls.lock().unwrap(), 2);
    }

    #[test]
    fn closed_epoch_absorbs_task_return_without_authorizing_more_work() {
        let fixture = Fixture::new();
        let (mut actor, _) = fixture.actor(
            vec![PlannerAssemblyOutcome::Complete(Box::new(
                fixture.cut.clone(),
            ))],
            true,
        );
        let fence = AgentAuthorizationFence {
            activation_generation: "actual-generation".into(),
            admission_epoch: Some("closed-epoch".into()),
            authority_policy_content_hash: actor
                .frozen_authority
                .authority_policy_content_hash
                .clone(),
        };
        let observer = Arc::new(MutableEpochAuthority(Mutex::new(Some(fence))));
        actor.authority = observer.clone();
        actor.planner = Arc::new(EpochTestPlanner(fixture.cut.clone()));
        for _ in 0..8 {
            actor.bounded_step(8);
        }
        let offered = fixture
            .store
            .product_authorizations_for_goal(&fixture.goal.goal_id)
            .unwrap();
        assert!(offered
            .iter()
            .any(|authorization| matches!(authorization.product, AgentAuthorizedProduct::Task(_))));
        let execution = Arc::new(TerminalExecution {
            submissions: Mutex::new(Vec::new()),
        });
        actor.execution = execution.clone();
        *observer.0.lock().unwrap() = None;
        actor.store = Arc::new(AgentStore::new(fixture.db.clone()).unwrap());
        let returned = actor.bounded_step(8);
        assert!(returned.fatal_errors.is_empty(), "{returned:?}");
        assert!(returned.retryable_errors.is_empty(), "{returned:?}");
        assert_eq!(returned.products_authorized, 0);
        assert_eq!(returned.milestones_accepted, 1);
        assert!(execution.submissions.lock().unwrap().is_empty());
        assert_eq!(
            fixture
                .store
                .product_authorizations_for_goal(&fixture.goal.goal_id)
                .unwrap(),
            offered
        );
        assert!(fixture
            .store
            .completed_history_for_goal(&fixture.goal.goal_id)
            .unwrap()
            .iter()
            .any(|entry| matches!(entry.product, Some(AgentAuthorizedProduct::Task(_)))));
        assert_eq!(actor.bounded_step(8).milestones_accepted, 0);
    }

    #[test]
    fn admission_epochs_create_distinct_goals_and_preserve_predecessor_authority() {
        let fixture = Fixture::new();
        let (mut actor, _) = fixture.actor(
            vec![PlannerAssemblyOutcome::Complete(Box::new(
                fixture.cut.clone(),
            ))],
            false,
        );
        let condition = crate::agent::AgentMaintainedCondition {
            condition_id: "docs-epoch-condition".into(),
            dimension_id: "docs_freshness".into(),
            desired: Condition::Above(Term::Literal(Literal::Number(0.7))),
            goal_priority: fixture.goal.priority.clone(),
            desired_summary: "current docs".into(),
            observation_scope: crate::agent::AgentObservationScope::AssignedSubject,
        };
        let (_, revision) =
            crate::agent::AgentMaintainedConditionRegistryStore::new(fixture.db.clone())
                .unwrap()
                .install(condition, 1)
                .unwrap();
        actor.intent = crate::agent::AgentReconciliationIntent::MaintainedCondition(
            revision.binding().unwrap(),
        );
        let first_fence = AgentAuthorizationFence {
            activation_generation: "actual-generation".into(),
            admission_epoch: Some("epoch-1".into()),
            authority_policy_content_hash: actor
                .frozen_authority
                .authority_policy_content_hash
                .clone(),
        };
        let observer = Arc::new(MutableEpochAuthority(Mutex::new(Some(first_fence.clone()))));
        actor.authority = observer.clone();
        actor.planner = Arc::new(EpochTestPlanner(fixture.cut.clone()));
        let curation_store = Arc::new(CurationStore::new(fixture.db.clone()).unwrap());
        actor.curation = Arc::new(DurableCuration {
            store: curation_store,
        });
        let first = actor.bounded_step(8);
        assert!(first.fatal_errors.is_empty(), "{first:?}");
        assert!(first.retryable_errors.is_empty(), "{first:?}");
        assert_eq!(first.products_authorized, 1);
        let first_goal = actor.intent.goal_id(
            &actor.strategy.agent_id,
            &first_fence.reconciliation_scope(),
        );
        let first_authorizations = fixture
            .store
            .product_authorizations_for_goal(&first_goal)
            .unwrap();
        assert_eq!(first_authorizations.len(), 1);
        assert_eq!(
            first_authorizations[0].activation_generation,
            "actual-generation"
        );
        assert_eq!(
            first_authorizations[0].admission_epoch.as_deref(),
            Some("epoch-1")
        );
        *observer.0.lock().unwrap() = None;
        let closed = actor.bounded_step(8);
        assert_eq!(closed.products_authorized, 0);
        assert_eq!(closed.records_persisted, 0);
        let mut next = first_fence.clone();
        next.admission_epoch = Some("epoch-2".into());
        *observer.0.lock().unwrap() = Some(next.clone());
        let second = actor.bounded_step(8);
        assert!(second.fatal_errors.is_empty(), "{second:?}");
        assert!(second.retryable_errors.is_empty(), "{second:?}");
        assert_eq!(second.products_authorized, 1);
        let second_goal = actor
            .intent
            .goal_id(&actor.strategy.agent_id, &next.reconciliation_scope());
        assert_ne!(first_goal, second_goal);
        assert_eq!(
            fixture
                .store
                .reconciliation_goals_for_agent(&actor.strategy.agent_id)
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            fixture
                .store
                .product_authorizations_for_goal(&first_goal)
                .unwrap(),
            first_authorizations
        );
        let second_authorizations = fixture
            .store
            .product_authorizations_for_goal(&second_goal)
            .unwrap();
        assert_eq!(
            second_authorizations[0].admission_epoch.as_deref(),
            Some("epoch-2")
        );
        assert_ne!(
            first_authorizations[0].authorization_id,
            second_authorizations[0].authorization_id
        );
        actor.store = Arc::new(AgentStore::new(fixture.db.clone()).unwrap());
        assert_eq!(actor.bounded_step(8).products_authorized, 0);
        assert_eq!(
            actor
                .store
                .reconciliation_goals_for_agent(&actor.strategy.agent_id)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn maintained_condition_does_not_create_a_goal_until_work_is_needed() {
        let fixture = Fixture::new();
        let (mut actor, curation) = fixture.actor(
            vec![PlannerAssemblyOutcome::Complete(Box::new(satisfied_cut(
                &fixture.cut,
            )))],
            true,
        );
        let condition = crate::agent::AgentMaintainedCondition {
            condition_id: "docs-maintained".into(),
            dimension_id: "docs_freshness".into(),
            desired: Condition::Above(Term::Literal(Literal::Number(0.7))),
            goal_priority: fixture.goal.priority.clone(),
            desired_summary: "current documentation is established".into(),
            observation_scope: crate::agent::AgentObservationScope::AssignedSubject,
        };
        let (_, revision) =
            crate::agent::AgentMaintainedConditionRegistryStore::new(fixture.db.clone())
                .unwrap()
                .install(condition, 1)
                .unwrap();
        actor.intent = crate::agent::AgentReconciliationIntent::MaintainedCondition(
            revision.binding().unwrap(),
        );
        let goal_id = actor.intent.goal_id(
            &actor.strategy.agent_id,
            &actor.frozen_authority.activation_generation,
        );

        let satisfied = actor.bounded_step(8);
        assert!(satisfied.fatal_errors.is_empty(), "{satisfied:?}");
        assert!(satisfied.retryable_errors.is_empty(), "{satisfied:?}");
        assert_eq!(fixture.store.condition_judgments().unwrap().len(), 1);
        assert!(fixture
            .store
            .reconciliation_goal(&goal_id)
            .unwrap()
            .is_none());
        assert!(fixture
            .store
            .current_reconciliation_plan(&goal_id)
            .unwrap()
            .is_none());
        assert!(curation.operation.lock().unwrap().is_none());
        assert_eq!(actor.bounded_step(8).records_persisted, 0);

        let mut changed = change_planner(&mut actor, &fixture.cut);
        changed.context.goal_id = goal_id.clone();
        actor.planner = Arc::new(PlannerSequence {
            outcomes: Mutex::new(VecDeque::new()),
            fallback: PlannerAssemblyOutcome::Complete(Box::new(changed)),
        });
        let needs_knowledge = actor.bounded_step(8);
        assert!(
            needs_knowledge.fatal_errors.is_empty(),
            "{needs_knowledge:?}"
        );
        assert!(
            needs_knowledge.retryable_errors.is_empty(),
            "{needs_knowledge:?}"
        );
        let goal = fixture
            .store
            .reconciliation_goal(&goal_id)
            .unwrap()
            .unwrap();
        assert!(matches!(
            goal.goal.source,
            meld_lang::GoalSource::Maintenance { .. }
        ));
        assert_eq!(
            needs_knowledge.products_authorized, 2,
            "{needs_knowledge:?}"
        );
        assert!(fixture
            .store
            .current_reconciliation_plan(&goal_id)
            .unwrap()
            .is_some());
        assert_eq!(fixture.store.condition_judgments().unwrap().len(), 2);
        let reopened = AgentStore::new(fixture.db.clone()).unwrap();
        assert_eq!(
            reopened.condition_judgments().unwrap(),
            fixture.store.condition_judgments().unwrap()
        );
    }

    #[test]
    fn admitted_satisfaction_creates_no_products_and_changed_evidence_resumes_work() {
        let fixture = Fixture::new();
        let satisfied = satisfied_cut(&fixture.cut);
        let (mut actor, curation) = fixture.actor(
            vec![PlannerAssemblyOutcome::Complete(Box::new(satisfied))],
            true,
        );
        let report = actor.bounded_step(8);
        assert!(report.fatal_errors.is_empty(), "{report:?}");
        assert!(report.retryable_errors.is_empty(), "{report:?}");
        assert_eq!(report.products_authorized, 0);
        assert!(curation.operation.lock().unwrap().is_none());
        let plan = fixture
            .store
            .current_reconciliation_plan(&fixture.goal.goal_id)
            .unwrap()
            .unwrap();
        assert!(plan.tasks.is_empty());
        let disposition = fixture
            .store
            .goal_disposition_for_plan(&plan.plan_revision_id)
            .unwrap()
            .unwrap();
        assert!(matches!(
            disposition.lifecycle,
            GoalLifecycle::Satisfied { .. }
        ));
        assert_eq!(disposition.planner_cut_id, plan.planner_cut_id);
        assert_eq!(actor.bounded_step(8).records_persisted, 0);
        change_planner(&mut actor, &fixture.cut);
        let changed = actor.bounded_step(8);
        assert_eq!(changed.products_authorized, 2, "{changed:?}");
        let successor = fixture
            .store
            .current_reconciliation_plan(&fixture.goal.goal_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            successor.predecessor_plan_revision_id,
            Some(plan.plan_revision_id)
        );
        assert!(fixture
            .store
            .goal_disposition_for_plan(&successor.plan_revision_id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn changed_evidence_blocks_satisfaction_at_the_final_agent_judgment() {
        let fixture = Fixture::new();
        let (actor, _) = fixture.actor(
            vec![
                PlannerAssemblyOutcome::Complete(Box::new(satisfied_cut(&fixture.cut))),
                PlannerAssemblyOutcome::Complete(Box::new(fixture.cut.clone())),
            ],
            true,
        );
        let result = actor.bounded_step(8);
        let plan = fixture
            .store
            .current_reconciliation_plan(&fixture.goal.goal_id)
            .unwrap()
            .unwrap();
        assert!(fixture
            .store
            .goal_disposition_for_plan(&plan.plan_revision_id)
            .unwrap()
            .is_none());
        assert_eq!(result.products_authorized, 0);
        assert!(result
            .waiting_on
            .iter()
            .any(|wait| wait.condition == "planner_cut_changed"));
    }

    #[test]
    fn stale_generation_or_authority_blocks_authorization() {
        for observed in [
            AgentAuthorizationFence {
                activation_generation: "activation-docs-v2".to_string(),
                admission_epoch: None,
                authority_policy_content_hash: "authority-docs-v1".to_string(),
            },
            AgentAuthorizationFence {
                activation_generation: "activation-docs".to_string(),
                admission_epoch: None,
                authority_policy_content_hash: "authority-docs-v2".to_string(),
            },
        ] {
            let fixture = Fixture::new();
            let (actor, curation) = fixture.actor_with_fence(
                vec![
                    PlannerAssemblyOutcome::Complete(Box::new(fixture.cut.clone())),
                    PlannerAssemblyOutcome::Complete(Box::new(fixture.cut.clone())),
                ],
                true,
                observed,
            );
            let report = actor.bounded_step(8);
            assert_eq!(report.products_authorized, 0);
            assert!(curation.operation.lock().unwrap().is_none());
            let wait = report
                .waiting_on
                .iter()
                .find(|wait| wait.condition == "agent_authority_changed")
                .unwrap();
            assert!(wait.subject_key.is_some());
        }
    }

    #[test]
    fn each_durable_agent_boundary_resumes_with_monotonic_zero_replay() {
        let temp = tempfile::tempdir().unwrap();
        let store_path = temp.path().join("world-model.sled");
        let authority = authority();
        let rule_body = rule();
        let rule = {
            let db = sled::open(&store_path).unwrap();
            let store = CurationStore::new(db).unwrap();
            let revision = store.install_rule(rule_body.clone(), 1).unwrap();
            store.flush().unwrap();
            revision
        };
        let cut = planner_cut(&rule_body);
        let goal = goal();
        let package: StrategyTheoryPackage = serde_json::from_str(include_str!(
            "../../../../theory/docs_freshness/strategy_theory.docs_freshness.json"
        ))
        .unwrap();
        let strategy =
            AgentStrategyRuntimeConfig::activate_installed(package, subject(), "agent-docs")
                .unwrap();
        let operation = CurationOperation::reconstruct(
            authority.clone(),
            rule.revision_ref(),
            cut.traversal_cut.clone(),
            cut.traversal_request.clone(),
        )
        .unwrap();
        let plan = search(&StrategySearchRequest {
            problem: strategy.problem(goal.clone(), cut.clone(), vec![operation.clone()]),
            bounds: strategy.bounds(),
        })
        .recommendation
        .unwrap();
        let epistemic = plan.epistemic_operations[0].clone();
        let acceptance = CurationAcceptanceRecord::for_operation(&operation, &rule).unwrap();
        let result = CurationResult::new(
            &operation,
            CurationTerminalDisposition::Abstained,
            "canonical Curation declined semantic authorship",
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .unwrap();
        let mut last = 0;
        for boundary in 0..6 {
            let report = {
                let db = sled::open(&store_path).unwrap();
                let store = Arc::new(AgentStore::new(db.clone()).unwrap());
                let curation_store = Arc::new(CurationStore::new(db).unwrap());
                let actor = AgentReconciliationActor::new(
                    AGENT_RECONCILIATION_RUNTIME_ID,
                    goal.clone(),
                    Arc::clone(&store),
                    Arc::new(PlannerSequence {
                        outcomes: Mutex::new(VecDeque::new()),
                        fallback: PlannerAssemblyOutcome::Complete(Box::new(cut.clone())),
                    }),
                    Arc::new(FixedAuthority(AgentAuthorizationFence {
                        activation_generation: "activation-docs".to_string(),
                        admission_epoch: None,
                        authority_policy_content_hash: "authority-docs-v1".to_string(),
                    })),
                    AgentAuthorizationFence {
                        activation_generation: "activation-docs".to_string(),
                        admission_epoch: None,
                        authority_policy_content_hash: "authority-docs-v1".to_string(),
                    },
                    Arc::new(DurableCuration {
                        store: Arc::clone(&curation_store),
                    }),
                    Arc::new(PendingExecution),
                    strategy.clone(),
                    authority.clone(),
                    rule.clone(),
                )
                .unwrap();
                let report = actor.bounded_step(1);
                if boundary == 1 {
                    curation_store.put_acceptance(&acceptance).unwrap();
                    curation_store.put_result(&result).unwrap();
                }
                store.flush().unwrap();
                curation_store.flush().unwrap();
                report
            };
            assert_eq!(report.input_position, last, "boundary {boundary}");
            assert!(
                report.output_position > last,
                "boundary {boundary}: {report:?}"
            );
            last = report.output_position;
        }

        let db = sled::open(&store_path).unwrap();
        let store = Arc::new(AgentStore::new(db.clone()).unwrap());
        let curation_store = Arc::new(CurationStore::new(db).unwrap());
        let replay = AgentReconciliationActor::new(
            AGENT_RECONCILIATION_RUNTIME_ID,
            goal.clone(),
            Arc::clone(&store),
            Arc::new(PlannerSequence {
                outcomes: Mutex::new(VecDeque::new()),
                fallback: PlannerAssemblyOutcome::Complete(Box::new(cut.clone())),
            }),
            Arc::new(FixedAuthority(AgentAuthorizationFence {
                activation_generation: "activation-docs".to_string(),
                admission_epoch: None,
                authority_policy_content_hash: "authority-docs-v1".to_string(),
            })),
            AgentAuthorizationFence {
                activation_generation: "activation-docs".to_string(),
                admission_epoch: None,
                authority_policy_content_hash: "authority-docs-v1".to_string(),
            },
            Arc::new(DurableCuration {
                store: Arc::clone(&curation_store),
            }),
            Arc::new(PendingExecution),
            strategy,
            authority,
            rule,
        )
        .unwrap()
        .bounded_step(8);
        assert_eq!(replay.input_position, last);
        assert_eq!(replay.output_position, last);
        assert_eq!(replay.records_persisted, 0);
        assert_eq!(replay.products_authorized, 0);
        assert_eq!(replay.milestones_accepted, 0);
        assert!(replay
            .waiting_on
            .iter()
            .all(|wait| wait.subject_key.is_some()));
        assert_eq!(
            store.reconciliation_plan(&plan.plan_revision_id).unwrap(),
            Some(plan.clone())
        );
        let judgment_id = stable_id(
            "agent-plan-judgment-v1",
            &(
                "agent-docs",
                &goal.goal_id,
                &plan.plan_revision_id,
                &cut.context.context_id,
                &cut.context.authority_scope_id,
                &cut.context.activation_generation,
                "admitted",
            ),
        );
        assert!(store.plan_judgment(&judgment_id).unwrap().is_some());
        assert_eq!(
            curation_store
                .acceptance_for_planned_operation(&operation.operation_id)
                .unwrap(),
            Some(acceptance)
        );
        assert_eq!(
            curation_store
                .result_for_operation(&operation.operation_id)
                .unwrap(),
            Some(result.clone())
        );
        assert_eq!(
            store
                .product_authorizations_for_goal(&goal.goal_id)
                .unwrap()
                .len(),
            2
        );
        let milestone_id = stable_id(
            "agent-milestone-acceptance-v1",
            &(
                "agent-docs",
                &goal.goal_id,
                &plan.plan_revision_id,
                &epistemic.product_id,
                &result.result_id,
                &cut.context.context_id,
                &cut.context.activation_generation,
            ),
        );
        assert!(store.milestone(&milestone_id).unwrap().is_some());
    }

    #[test]
    fn reopening_shared_database_replays_without_identity_drift() {
        let fixture = Fixture::new();
        let plan = fixture.expected_plan();
        let (actor, _) = fixture.actor(
            vec![
                PlannerAssemblyOutcome::Complete(Box::new(fixture.cut.clone())),
                PlannerAssemblyOutcome::Complete(Box::new(fixture.cut.clone())),
            ],
            true,
        );
        let first = actor.bounded_step(8);
        assert!(first.fatal_errors.is_empty());
        fixture.store.flush().unwrap();
        drop(actor);

        let reopened = Arc::new(AgentStore::new(fixture.db.clone()).unwrap());
        assert_eq!(
            reopened
                .reconciliation_plan(&plan.plan_revision_id)
                .unwrap(),
            Some(plan.clone())
        );
        let curation = Arc::new(ImmediateCuration {
            rule: fixture.rule.clone(),
            operation: Mutex::new(None),
            terminal: true,
        });
        let replay = AgentReconciliationActor::new(
            AGENT_RECONCILIATION_RUNTIME_ID,
            fixture.goal.clone(),
            Arc::clone(&reopened),
            Arc::new(PlannerSequence {
                outcomes: Mutex::new(VecDeque::from(vec![
                    PlannerAssemblyOutcome::Complete(Box::new(fixture.cut.clone())),
                    PlannerAssemblyOutcome::Complete(Box::new(fixture.cut.clone())),
                ])),
                fallback: PlannerAssemblyOutcome::Complete(Box::new(fixture.cut.clone())),
            }),
            Arc::new(FixedAuthority(AgentAuthorizationFence {
                activation_generation: fixture.authority.activation_generation.clone(),
                admission_epoch: None,
                authority_policy_content_hash: "authority-docs-v1".to_string(),
            })),
            AgentAuthorizationFence {
                activation_generation: fixture.authority.activation_generation.clone(),
                admission_epoch: None,
                authority_policy_content_hash: "authority-docs-v1".to_string(),
            },
            curation,
            Arc::new(PendingExecution),
            fixture.strategy.clone(),
            fixture.authority.clone(),
            fixture.rule.clone(),
        )
        .unwrap()
        .bounded_step(8);
        assert!(replay.fatal_errors.is_empty(), "{:?}", replay.fatal_errors);
        assert_eq!(
            reopened
                .reconciliation_plan(&plan.plan_revision_id)
                .unwrap(),
            Some(plan)
        );
    }

    fn subject() -> DomainObjectRef {
        DomainObjectRef::new("workspace_fs", "node", "meld").unwrap()
    }

    fn authority() -> CurationAuthority {
        CurationAuthority {
            agent_id: "agent-docs".to_string(),
            perspective: PerspectiveKey::new("frame", "default").unwrap(),
            branch_scope: BranchScope::main(),
            activation_generation: "activation-docs".to_string(),
            admission_epoch: None,
            subject: subject(),
        }
    }

    fn scope() -> OwnerPublicationScope {
        OwnerPublicationScope {
            scope_id: "meld".to_string(),
            branch_id: Some("main".to_string()),
            perspective_id: Some("default".to_string()),
            valid_at: None,
        }
    }

    fn rule() -> StandingCurationRule {
        StandingCurationRule {
            source_event_route: None,
            judgment_scope: None,
            rule_id: "rule-docs".to_string(),
            agent_id: "agent-docs".to_string(),
            source_owner_id: "workspace_fs".to_string(),
            scope: scope(),
            roots: vec![subject()],
            traversal_direction: TraversalDirection::Incoming,
            bounds: TraversalBounds {
                max_depth: 1,
                max_objects: 8,
                max_occurrences: 8,
                max_paths: 8,
            },
            expected_object_kind: "assessment".to_string(),
            expected_object_id: "meld::expected".to_string(),
            relation_type: "curation_assesses".to_string(),
            output_policy_revision: "output-policy-v1".to_string(),
            realization: None,
        }
    }

    fn goal() -> Goal {
        Goal {
            goal_id: "goal-docs".to_string(),
            agent_id: "agent-docs".to_string(),
            target: Proposition::Holds {
                subject: Term::Object(subject()),
                dimension: Term::Dimension("docs_freshness".to_string()),
                condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
            },
            priority: GoalPriority {
                urgency: 1,
                cost_ceiling: None,
            },
            source: GoalSource::Maintenance {
                invariant_description: "documentation remains fresh".to_string(),
            },
            lifecycle: GoalLifecycle::Proposed,
        }
    }

    fn planner_cut(rule: &StandingCurationRule) -> PlannerCut {
        let ledger_id = LedgerIdentity::new();
        let mut cut = TraversalCut {
            cut_id: String::new(),
            owners: vec![
                TraversalOwnerRequirement {
                    event_source: None,
                    owner_id: CURATION_OWNER_ID.to_string(),
                    scope: scope(),
                    required: false,
                },
                TraversalOwnerRequirement {
                    event_source: None,
                    owner_id: "workspace_fs".to_string(),
                    scope: scope(),
                    required: true,
                },
            ],
            receipts: vec![OwnerGraphRevisionReceipt {
                event_coverage: None,
                owner_id: "workspace_fs".to_string(),
                revision_id: "workspace-v1".to_string(),
                scope: scope(),
                completeness: OwnerCompletenessReceipt {
                    receipt_id: "workspace-complete-v1".to_string(),
                    scope: scope(),
                    included_ids: Vec::new(),
                    exclusions: Vec::new(),
                    failures: Vec::new(),
                    status: OwnerCompletenessStatus::Complete,
                },
                source_event: Some(meld_events::EventRecordRef { ledger_id, seq: 7 }),
                projection_position: LedgerCursor {
                    ledger_id,
                    after_seq: 7,
                },
            }],
            scope: scope(),
            currentness: OwnerCurrentnessPolicy::LatestComplete,
            event_position: LedgerCursor {
                ledger_id,
                after_seq: 7,
            },
            graph_position: LedgerCursor {
                ledger_id,
                after_seq: 7,
            },
            status: TraversalCutStatus::Complete,
            issues: Vec::new(),
        };
        cut.cut_id = traversal_cut_identity(&cut).unwrap();
        PlannerCut {
            cut_id: "planner-cut-docs-v1".to_string(),
            context: PlannerDecisionContext {
                observation_subject: None,
                context_id: "context-docs-v1".to_string(),
                agent_id: "agent-docs".to_string(),
                goal_id: "goal-docs".to_string(),
                subject: subject(),
                scope_id: "meld".to_string(),
                branch_id: "main".to_string(),
                perspective_id: "default".to_string(),
                authority_scope_id: "authority-docs".to_string(),
                activation_generation: "activation-docs".to_string(),
                admission_epoch: None,
            },
            policy: PlannerAssemblyPolicy {
                policy_revision_id: "policy-v1".to_string(),
                required_sources: vec![PlannerSourceKind::Graph, PlannerSourceKind::Belief],
                explicitly_not_required: vec![
                    PlannerSourceKind::Causation,
                    PlannerSourceKind::Regime,
                ],
            },
            traversal_cut: cut,
            traversal_request: rule.traversal_request(),
            traversal_result: TraversalResult {
                absent_roots: Vec::new(),
                result_id: "traversal-result-v1".to_string(),
                cut_id: String::new(),
                objects: Vec::new(),
                occurrences: Vec::new(),
                paths: Vec::new(),
                receipts: Vec::new(),
                frontier: Vec::new(),
                truncation: TraversalTruncation::default(),
            },
            source_positions: Vec::new(),
            world_model_view: WorldModelView {
                world_state: WorldState::new(Vec::new()).unwrap(),
                projection_version: PLANNER_PROJECTION_VERSION.to_string(),
                source_refs: vec![PlannerSourceRef::ProjectionRule {
                    rule_id: "projection-v1".to_string(),
                }],
                hydration_refs: PlannerHydrationRefs::default(),
                warnings: Vec::<PlannerProjectionWarning>::new(),
                theory_revision: Some(TheoryRevisionRef {
                    registry: CURATION_RULE_REGISTRY_ID.to_string(),
                    id: "belief-theory".to_string(),
                    content_hash: "belief-theory-hash".to_string(),
                }),
            },
        }
    }
}
