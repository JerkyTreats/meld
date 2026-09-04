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
    CurationPlannedAuthorization, CurationResult, StandingCurationRuleRevision,
};
use crate::error::StorageError;
use crate::planner::{PlannerAssemblyOutcome, PlannerCut, PlannerRefusal};
use crate::strategy::{
    search, verify_plan, PlanMilestoneRequirement, PlanVerification, StrategyPlan,
    StrategySearchRequest,
};
use crate::waiting::WaitingOnDeclaration;

/// Planner-owned current source assembly used by Agent for construction and freshness.
pub trait AgentPlannerPort: Send + Sync {
    fn assemble(&self) -> PlannerAssemblyOutcome;
}

/// Read-only root observation of the currently active Agent authority fence.
pub trait AgentAuthorityPort: Send + Sync {
    fn observe(&self) -> Result<Option<AgentAuthorizationFence>, StorageError>;
}

/// Curation-owned planned intake and result query boundary.
pub trait AgentCurationPort: Send + Sync {
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
    goal: Goal,
    store: Arc<AgentStore>,
    planner: Arc<dyn AgentPlannerPort>,
    authority: Arc<dyn AgentAuthorityPort>,
    frozen_authority: AgentAuthorizationFence,
    curation: Arc<dyn AgentCurationPort>,
    execution: Arc<dyn AgentExecutionPort>,
    strategy: AgentStrategyRuntimeConfig,
    curation_authority: CurationAuthority,
    curation_rule: StandingCurationRuleRevision,
}

impl AgentReconciliationActor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actor_id: impl Into<String>,
        goal: Goal,
        store: Arc<AgentStore>,
        planner: Arc<dyn AgentPlannerPort>,
        authority: Arc<dyn AgentAuthorityPort>,
        frozen_authority: AgentAuthorizationFence,
        curation: Arc<dyn AgentCurationPort>,
        execution: Arc<dyn AgentExecutionPort>,
        strategy: AgentStrategyRuntimeConfig,
        curation_authority: CurationAuthority,
        curation_rule: StandingCurationRuleRevision,
    ) -> Result<Self, StorageError> {
        let actor_id = actor_id.into();
        if actor_id.trim().is_empty()
            || goal.agent_id != strategy.agent_id
            || goal.agent_id != curation_authority.agent_id
        {
            return Err(StorageError::InvalidPath(
                "Agent reconciliation actor requires one exact Agent identity".to_string(),
            ));
        }
        curation_authority.validate()?;
        curation_rule.validate()?;
        if frozen_authority.activation_generation != curation_authority.activation_generation
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
            actor_id,
            goal,
            store,
            planner,
            authority,
            frozen_authority,
            curation,
            execution,
            strategy,
            curation_authority,
            curation_rule,
        })
    }

    pub fn actor_id(&self) -> &str {
        &self.actor_id
    }

    /// Advance eligible transitions through durable, replay-resumable boundaries.
    pub fn bounded_step(&self, max_items: usize) -> AgentReconciliationReport {
        let input_position = self.store.reconciliation_position();
        let mut report = self.bounded_step_inner(max_items);
        report.input_position = input_position;
        report.output_position = self.store.reconciliation_position();
        report.records_persisted = report.output_position.saturating_sub(input_position) as usize;
        report
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
        let cut = match self.planner.assemble() {
            PlannerAssemblyOutcome::Complete(cut) => *cut,
            PlannerAssemblyOutcome::Refused(refusal) => {
                report.waiting_on.push(planner_wait(&refusal));
                return report;
            }
        };
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
        if let Some(authorization) = authorizations
            .iter()
            .find(|record| matches!(record.product, AgentAuthorizedProduct::Task(_)))
        {
            let Some(plan) = (match self
                .store
                .reconciliation_plan(&authorization.plan_revision_id)
            {
                Ok(plan) => plan,
                Err(error) => {
                    report.fatal_errors.push(error.to_string());
                    return report;
                }
            }) else {
                report
                    .fatal_errors
                    .push("authorized Agent product has no durable Plan".to_string());
                return report;
            };
            if let Err(error) = self.reconcile_execution(&cut, &plan, authorization, &mut report) {
                report.retryable_errors.push(error.to_string());
            }
            return report;
        }
        if let Some(authorization) = authorizations
            .iter()
            .find(|record| matches!(record.product, AgentAuthorizedProduct::Epistemic(_)))
        {
            let Some(plan) = (match self
                .store
                .reconciliation_plan(&authorization.plan_revision_id)
            {
                Ok(plan) => plan,
                Err(error) => {
                    report.fatal_errors.push(error.to_string());
                    return report;
                }
            }) else {
                report
                    .fatal_errors
                    .push("authorized Agent product has no durable Plan".to_string());
                return report;
            };
            let AgentAuthorizedProduct::Epistemic(epistemic) = &authorization.product else {
                unreachable!("authorization selection fixes the product variant");
            };
            if goal_inserted && max_items == 1 {
                report.budget_exhausted = true;
                return report;
            }
            if let Err(error) = self.reconcile_curation(
                &cut,
                &plan,
                epistemic,
                max_items - usize::from(goal_inserted),
                &mut report,
            ) {
                report.retryable_errors.push(error.to_string());
            }
            return report;
        }
        let operation = match CurationOperation::reconstruct(
            self.curation_authority.clone(),
            self.curation_rule.revision_ref(),
            cut.traversal_cut.clone(),
            cut.traversal_request.clone(),
        ) {
            Ok(operation) => operation,
            Err(error) => {
                report.fatal_errors.push(error.to_string());
                return report;
            }
        };
        let problem = self
            .strategy
            .problem(self.goal.clone(), cut.clone(), vec![operation]);
        let result = search(&StrategySearchRequest {
            problem: problem.clone(),
            bounds: self.strategy.bounds(),
        });
        let Some(plan) = result.recommendation else {
            report.waiting_on.push(waiting(
                "strategy_refusal",
                &cut.cut_id,
                format!(
                    "Strategy produced no complete Plan: {:?}",
                    result.rejections
                ),
            ));
            return report;
        };
        if let PlanVerification::Invalid { grounds } = verify_plan(&problem, &plan) {
            report.fatal_errors.push(format!(
                "Strategy verification rejected its Plan: {grounds:?}"
            ));
            return report;
        }
        let judgment_inserted = match self.persist_plan_and_judgment(&cut, &plan) {
            Ok(inserted) => inserted,
            Err(error) => {
                report.fatal_errors.push(error.to_string());
                return report;
            }
        };
        let mut used = usize::from(goal_inserted || judgment_inserted);
        if used == max_items {
            report.budget_exhausted = true;
            return report;
        }
        let Some(epistemic) = plan.epistemic_operations.first() else {
            report.waiting_on.push(waiting(
                "plan_has_no_epistemic_product",
                &plan.plan_revision_id,
                "mixed Plan requires one bounded Curation operation",
            ));
            return report;
        };
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
            self.reconcile_curation(&cut, &plan, epistemic, max_items - used, &mut report)
        {
            report.retryable_errors.push(error.to_string());
        }
        report
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
        let plan_inserted = self.store.put_reconciliation_plan(plan)?;
        let judgment_id = stable_id(
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
        );
        let judgment_inserted = self.store.put_plan_judgment(&AgentPlanJudgment {
            judgment_id,
            agent_id: cut.context.agent_id.clone(),
            goal_id: self.goal.goal_id.clone(),
            plan_revision_id: plan.plan_revision_id.clone(),
            context_id: cut.context.context_id.clone(),
            authority_scope_id: cut.context.authority_scope_id.clone(),
            activation_generation: cut.context.activation_generation.clone(),
            kind: AgentPlanJudgmentKind::Admitted,
        })?;
        Ok(plan_inserted || judgment_inserted)
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
                        frozen_cut_id: cut.cut_id.clone(),
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
            frozen_cut_id: cut.cut_id.clone(),
            observed_cut_id: observed.clone(),
            refusal: None,
        };
        if observed.as_deref() != Some(cut.cut_id.as_str()) {
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
                &acceptance.acceptance_id,
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
            owner_position_id: result.result_id,
            context_id: cut.context.context_id.clone(),
            activation_generation: cut.context.activation_generation.clone(),
        })?;
        let (milestone_progress_inserted, _) = self.put_progress(
            cut,
            plan,
            &epistemic.product_id,
            AgentCurrentnessCheck {
                frozen_cut_id: cut.cut_id.clone(),
                observed_cut_id: Some(cut.cut_id.clone()),
                refusal: None,
            },
            AgentProductState::MilestoneAccepted { milestone_id },
        )?;
        let milestone_advanced = milestone_inserted || milestone_progress_inserted;
        report.milestones_accepted = usize::from(milestone_inserted);
        used += usize::from(milestone_advanced);
        if used == budget {
            report.budget_exhausted = true;
            return Ok(());
        }
        if let Some(task) = plan.tasks.first() {
            let dependency_satisfied = plan.dependencies.iter().any(|dependency| {
                dependency.producer_product_id == epistemic.product_id
                    && dependency.consumer_product_id == task.task_id
                    && dependency.required_milestone == requirement
            });
            if dependency_satisfied {
                if self.authority.observe()?.as_ref() != Some(&self.frozen_authority) {
                    report.waiting_on.push(waiting(
                        "agent_authority_changed",
                        &task.task_id,
                        "wake requires successor Task eligibility under the live authority fence",
                    ));
                    return Ok(());
                }
                let (_, progress_id) = self.put_progress(
                    cut,
                    plan,
                    &task.task_id,
                    AgentCurrentnessCheck {
                        frozen_cut_id: cut.cut_id.clone(),
                        observed_cut_id: Some(cut.cut_id.clone()),
                        refusal: None,
                    },
                    AgentProductState::Eligible,
                )?;
                report.eligible_task_ids.push(task.task_id.clone());
                self.authorize_task(cut, plan, task, &progress_id, report)?;
            }
        }
        Ok(())
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
                        frozen_cut_id: cut.cut_id.clone(),
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
            frozen_cut_id: cut.cut_id.clone(),
            observed_cut_id: observed.clone(),
            refusal: None,
        };
        if observed.as_deref() != Some(cut.cut_id.as_str()) {
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
        let position = self.execution.advance(authorization)?;
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
                frozen_cut_id: cut.cut_id.clone(),
                observed_cut_id: Some(cut.cut_id.clone()),
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
                frozen_cut_id: cut.cut_id.clone(),
                observed_cut_id: Some(cut.cut_id.clone()),
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
    WaitingOnDeclaration {
        condition: condition.into(),
        subject_key: Some(subject_key.into()),
        detail: detail.into(),
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
        CurationStore, CurationTerminalDisposition, StandingCurationRule, CURATION_OWNER_ID,
        CURATION_RULE_REGISTRY_ID,
    };
    use crate::planner::{
        PlannerAssemblyPolicy, PlannerDecisionContext, PlannerHydrationRefs,
        PlannerProjectionWarning, PlannerSourceKind, PlannerSourceRef, WorldModelView,
        PLANNER_PROJECTION_VERSION,
    };
    use crate::strategy::{search, StrategySearchRequest, StrategyTheoryPackage};
    use crate::world_state::graph::contracts::*;
    use crate::world_state::graph::PerspectiveKey;

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

    struct ImmediateCuration {
        rule: StandingCurationRuleRevision,
        operation: Mutex<Option<CurationOperation>>,
        terminal: bool,
    }

    impl AgentCurationPort for ImmediateCuration {
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

    #[test]
    fn stale_generation_or_authority_blocks_authorization() {
        for observed in [
            AgentAuthorizationFence {
                activation_generation: "activation-docs-v2".to_string(),
                authority_policy_content_hash: "authority-docs-v1".to_string(),
            },
            AgentAuthorizationFence {
                activation_generation: "activation-docs".to_string(),
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
                        authority_policy_content_hash: "authority-docs-v1".to_string(),
                    })),
                    AgentAuthorizationFence {
                        activation_generation: "activation-docs".to_string(),
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
                authority_policy_content_hash: "authority-docs-v1".to_string(),
            })),
            AgentAuthorizationFence {
                activation_generation: "activation-docs".to_string(),
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
                authority_policy_content_hash: "authority-docs-v1".to_string(),
            })),
            AgentAuthorizationFence {
                activation_generation: fixture.authority.activation_generation.clone(),
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
                    owner_id: CURATION_OWNER_ID.to_string(),
                    scope: scope(),
                    required: false,
                },
                TraversalOwnerRequirement {
                    owner_id: "workspace_fs".to_string(),
                    scope: scope(),
                    required: true,
                },
            ],
            receipts: vec![OwnerGraphRevisionReceipt {
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
                source_event: meld_events::EventRecordRef { ledger_id, seq: 7 },
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
                context_id: "context-docs-v1".to_string(),
                agent_id: "agent-docs".to_string(),
                goal_id: "goal-docs".to_string(),
                subject: subject(),
                scope_id: "meld".to_string(),
                branch_id: "main".to_string(),
                perspective_id: "default".to_string(),
                authority_scope_id: "authority-docs".to_string(),
                activation_generation: "activation-docs".to_string(),
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
