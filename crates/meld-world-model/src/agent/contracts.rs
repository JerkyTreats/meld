//! Public agent records and command contracts.

use meld_lang::{Condition, Goal, Literal, Term};
use serde::{Deserialize, Serialize};

use crate::belief::{BeliefKey, BranchScope};
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::PerspectiveKey;

/// Desired state supplied to native Agent judgment, without inventing a breach.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentReconciliationIntent {
    Goal(Goal),
    MaintainedCondition(super::AgentMaintainedConditionBinding),
}

impl From<Goal> for AgentReconciliationIntent {
    fn from(goal: Goal) -> Self {
        Self::Goal(goal)
    }
}

impl AgentReconciliationIntent {
    /// Stable reasoning identity, available before a transient Goal exists.
    pub fn goal_id(&self, agent_id: &str, activation_generation: &str) -> String {
        match self {
            Self::Goal(goal) => goal.goal_id.clone(),
            Self::MaintainedCondition(binding) => format!(
                "agent-goal::{agent_id}::{}::{activation_generation}",
                binding.condition.condition_id
            ),
        }
    }
}

/// Native judgment of one standing condition against an exact admitted cut.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentConditionJudgment {
    pub judgment_id: String,
    pub agent_id: String,
    pub condition_revision: crate::belief::TheoryRevisionRef,
    pub planner_cut_id: String,
    pub activation_generation: String,
    pub evaluation: meld_lang::EvalResult,
}

/// Durable Agent-owned Goal with immutable inception context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentReconciliationGoal {
    /// Exact standing intent that caused this transient Goal, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintained_condition_revision: Option<crate::belief::TheoryRevisionRef>,
    pub goal: Goal,
    pub context_id: String,
    pub authority_scope_id: String,
    pub activation_generation: String,
    pub created_at_seq: u64,
}

/// Agent's Goal judgment over exact admitted evidence, separate from product completion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentGoalDisposition {
    /// Exact native returns accepted before this separate Goal judgment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accepted_milestone_ids: Vec<String>,
    pub disposition_id: String,
    pub agent_id: String,
    pub goal_id: String,
    pub plan_revision_id: String,
    pub planner_cut_id: String,
    pub activation_generation: String,
    pub lifecycle: meld_lang::GoalLifecycle,
}

/// Immutable Agent judgment over one complete Plan revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentPlanJudgmentKind {
    Admitted,
    Rejected { reason: String },
    Superseded { successor_plan_revision_id: String },
}

/// Durable Agent decision that is deliberately distinct from product authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentPlanJudgment {
    pub judgment_id: String,
    pub agent_id: String,
    pub goal_id: String,
    pub plan_revision_id: String,
    pub context_id: String,
    pub authority_scope_id: String,
    pub activation_generation: String,
    pub kind: AgentPlanJudgmentKind,
}

/// Product-specific authority granted only after fresh eligibility.
pub type AgentAuthorizedProduct = crate::strategy::StrategyProduct;

/// Exact currentness evidence retained with one product progression position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCurrentnessCheck {
    pub frozen_cut_id: String,
    pub observed_cut_id: Option<String>,
    pub refusal: Option<crate::planner::PlannerRefusal>,
}

/// Exact live fence that must still hold when Agent grants product authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentAuthorizationFence {
    pub activation_generation: String,
    /// Exact admission epoch, absent only in legacy records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission_epoch: Option<String>,
    pub authority_policy_content_hash: String,
}

impl AgentAuthorizationFence {
    /// Reconciliation scope for transient Goals under one admission epoch.
    pub fn reconciliation_scope(&self) -> String {
        Self::scope_for(&self.activation_generation, self.admission_epoch.as_deref())
    }

    pub fn scope_for(generation: &str, epoch: Option<&str>) -> String {
        match epoch {
            Some(epoch) => format!("{generation}::epoch::{epoch}"),
            None => generation.to_string(),
        }
    }
}

/// Durable state of one Plan product.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentProductState {
    Blocked { reason: String },
    Eligible,
    Authorized { authorization_id: String },
    ConsumerAccepted { acceptance_id: String },
    Terminal { result_id: String },
    MilestoneAccepted { milestone_id: String },
    ExecutionAdmitted { admission_id: String },
    ExecutionTerminal { outcome_id: String },
}

/// Durable progression and currentness for one exact Plan product.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentProductProgress {
    pub progress_id: String,
    pub agent_id: String,
    pub goal_id: String,
    pub plan_revision_id: String,
    pub product_id: String,
    pub context_id: String,
    pub authority_scope_id: String,
    pub activation_generation: String,
    pub currentness: AgentCurrentnessCheck,
    pub state: AgentProductState,
}

/// Immutable per-product authorization persisted before publication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentProductAuthorization {
    pub authorization_id: String,
    pub agent_id: String,
    pub goal_id: String,
    pub plan_revision_id: String,
    pub product_id: String,
    pub context_id: String,
    pub authority_scope_id: String,
    pub authority_policy_content_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_decision: Option<meld_lang::AuthorityDecision>,
    pub activation_generation: String,
    /// Exact admission epoch, absent only in legacy records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission_epoch: Option<String>,
    pub idempotency_key: String,
    pub product: AgentAuthorizedProduct,
    pub curation_authorization: Option<crate::CurationPlannedAuthorization>,
}

/// Exact Curation consumer receipt retained by Agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentConsumerReceipt {
    pub receipt_id: String,
    pub authorization_id: String,
    pub operation_id: String,
    pub acceptance_id: String,
    pub result_id: Option<String>,
}

/// Exact Execution positions observed by Agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentExecutionPosition {
    pub authorization_id: String,
    pub admission_id: String,
    pub admission_decision: AgentExecutionAdmissionDecision,
    pub admission_revision: u64,
    pub network_commit_revision: Option<u64>,
    pub outcome_id: Option<String>,
    pub execution_publication_position_id: Option<String>,
}

/// Execution consumer decision projected without importing Execution types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecutionAdmissionDecision {
    Admitted,
    Rejected { grounds: Vec<String> },
    StaleFence,
}

/// Immutable Agent receipt for one distinct Execution return position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentExecutionReceipt {
    pub receipt_id: String,
    pub agent_id: String,
    pub goal_id: String,
    pub plan_revision_id: String,
    pub product_id: String,
    pub position: AgentExecutionPosition,
}

/// Exact owner milestone accepted by Agent for one Plan dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentMilestoneAcceptance {
    pub milestone_id: String,
    pub agent_id: String,
    pub goal_id: String,
    pub plan_revision_id: String,
    pub product_id: String,
    pub requirement: crate::strategy::PlanMilestoneRequirement,
    pub owner_position_id: String,
    pub context_id: String,
    pub activation_generation: String,
}

/// Stable durable identifier for an agent.
pub type AgentId = String;
/// Stable durable identifier for an agent subscription.
pub type AgentSubscriptionId = String;
/// Stable durable identifier for an activation lease record.
pub type AgentActivationId = String;

/// Lifecycle status for an agent record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// Stored but not yet ready for delivery.
    Registered,
    /// Ready to receive subscription deliveries.
    Operational,
    /// Disabled without deleting durable state.
    Suspended,
}

impl AgentStatus {
    /// Return the stable storage index fragment for this status.
    pub fn index_key(&self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::Operational => "operational",
            Self::Suspended => "suspended",
        }
    }
}

/// Lifecycle status for a subscription binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentSubscriptionStatus {
    /// Delivery is enabled for the bound belief key.
    Active,
    /// Delivery is paused while retaining the cursor.
    Suspended,
}

impl AgentSubscriptionStatus {
    /// Return the stable storage index fragment for this status.
    pub fn index_key(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
        }
    }
}

/// Runtime state reported by an activation worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentActivationStatus {
    /// Activation has started but is not yet ready.
    Started,
    /// Activation is ready to process work.
    Activated,
    /// Activation failed and may carry an error string.
    Failed,
}

/// Durable description of an agent and the world scope it observes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentRecord {
    /// Stable agent identifier.
    pub agent_id: AgentId,
    /// Perspective used for belief and planner reads.
    pub perspective_key: PerspectiveKey,
    /// Subject observed by the agent.
    pub subject: DomainObjectRef,
    /// Branch whose belief stream belongs to the agent.
    pub branch_scope: BranchScope,
    /// Named scope used by the seed or runtime binder.
    pub observation_scope: String,
    /// Human or system directive that explains agent intent.
    pub directive: String,
    /// Provenance string for seed created agents.
    pub seed_provenance: String,
    /// Curation rule installed on this agent as durable theory.
    ///
    /// The durable home for the rule per initialization stage 3: callers
    /// resolve the rule from the record instead of re-supplying it per
    /// call. Absent on records created before the binding existed.
    #[serde(default)]
    pub curation_rule: Option<AgentCurationRuleBinding>,
    /// Exact curation-rule revision selected for newly elevated records.
    #[serde(default)]
    pub curation_rule_revision: Option<crate::belief::TheoryRevisionRef>,
    /// Standing desired state that survives transient Goal lifecycles.
    #[serde(default)]
    pub maintained_condition: Option<super::AgentMaintainedConditionBinding>,
    /// Exact maintained-condition revision selected for this Agent.
    #[serde(default)]
    pub maintained_condition_revision: Option<crate::belief::TheoryRevisionRef>,
    /// Current agent lifecycle status.
    pub status: AgentStatus,
    /// Sequence assigned when the record was first stored.
    pub created_at_seq: u64,
    /// Last sequence that changed the record.
    pub updated_at_seq: u64,
}

impl AgentRecord {
    /// Validate stable identifiers and required scope fields.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("agent id", &self.agent_id)?;
        self.perspective_key.validate()?;
        self.subject.validate()?;
        require_non_empty("branch scope", &self.branch_scope.branch_id)?;
        require_non_empty("observation scope", &self.observation_scope)?;
        require_non_empty("directive", &self.directive)?;
        require_non_empty("seed provenance", &self.seed_provenance)?;
        if let Some(binding) = &self.curation_rule {
            binding.validate()?;
        }
        if let Some(reference) = &self.curation_rule_revision {
            reference.validate_for_registry("agent_curation_rule")?;
        }
        if let Some(binding) = &self.maintained_condition {
            binding.validate()?;
        }
        if let Some(reference) = &self.maintained_condition_revision {
            reference.validate_for_registry("agent_maintained_condition")?;
        }
        match (
            &self.maintained_condition,
            &self.maintained_condition_revision,
        ) {
            (Some(binding), Some(reference)) if &binding.revision == reference => {}
            (None, None) => {}
            _ => {
                return Err(StorageError::InvalidPath(
                    "agent maintained condition body and revision must match".to_string(),
                ))
            }
        }
        Ok(())
    }

    /// Resolve the curation rule installed on this record.
    ///
    /// The record is the durable home for the rule: the actor path resolves
    /// it here instead of accepting caller-supplied rule configuration, and
    /// fails truthfully when genesis never installed one.
    pub fn installed_curation_rule(&self) -> Result<&AgentCurationRuleConfig, StorageError> {
        let binding = self.curation_rule.as_ref().ok_or_else(|| {
            StorageError::InvalidPath(format!(
                "agent '{}' has no installed curation rule",
                self.agent_id
            ))
        })?;
        binding.validate()?;
        Ok(&binding.rule)
    }

    /// Resolve the installed standing condition or lower the legacy rule.
    pub fn effective_maintained_condition(
        &self,
    ) -> Result<super::AgentMaintainedCondition, StorageError> {
        if let Some(binding) = &self.maintained_condition {
            binding.validate()?;
            return Ok(binding.condition.clone());
        }
        super::AgentMaintainedCondition::from_legacy_rule(self.installed_curation_rule()?)
    }
}

/// Durable binding between an agent and one belief stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSubscriptionRecord {
    /// Stable subscription identifier.
    pub subscription_id: AgentSubscriptionId,
    /// Agent that owns the binding.
    pub agent_id: AgentId,
    /// Belief stream delivered to the agent.
    pub belief_key: BeliefKey,
    /// Current subscription lifecycle status.
    pub status: AgentSubscriptionStatus,
    /// Last revision persisted after successful decision storage.
    pub last_delivered_revision_id: Option<String>,
    /// Monotonic delivery cursor for replay safety.
    pub last_delivered_seq: u64,
    /// Sequence assigned when the binding was first stored.
    pub created_at_seq: u64,
    /// Last sequence that changed the binding.
    pub updated_at_seq: u64,
}

impl AgentSubscriptionRecord {
    /// Validate identifiers and the bound belief key.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("subscription id", &self.subscription_id)?;
        require_non_empty("agent id", &self.agent_id)?;
        self.belief_key.validate()?;
        Ok(())
    }

    /// Build the idempotency key for one agent and belief stream.
    pub fn natural_key(agent_id: &str, belief_key: &BeliefKey) -> String {
        format!("{agent_id}::{}", belief_key.index_key())
    }
}

/// Durable record for a runtime activation attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentActivationRecord {
    /// Stable activation identifier.
    pub activation_id: AgentActivationId,
    /// Agent activated by this runtime attempt.
    pub agent_id: AgentId,
    /// Sequence assigned when activation began.
    pub started_at_seq: u64,
    /// Current activation state.
    pub status: AgentActivationStatus,
    /// Last activation failure message, when present.
    pub last_error: Option<String>,
    /// Runtime lease identifier, when leased.
    pub lease_id: Option<String>,
}

impl AgentActivationRecord {
    /// Validate identifiers for the activation record.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("activation id", &self.activation_id)?;
        require_non_empty("agent id", &self.agent_id)?;
        Ok(())
    }
}

/// Read-only compatibility shape for a decision written before reconciliation cutover.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyAgentDecisionInputRefs {
    pub belief_revision_id: Option<String>,
}

/// Read-only compatibility classification for a pre-cutover decision record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegacyAgentDecisionKind {
    GoalCommand,
    GoalMutationCommand,
    Absorbed,
    Indeterminate,
}

/// Read-only compatibility record for historical traversal evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyAgentDecisionRecord {
    pub decision_id: String,
    pub agent_id: String,
    pub decision: LegacyAgentDecisionKind,
    pub input_refs: LegacyAgentDecisionInputRefs,
}

/// Read-only compatibility receipt for a pre-cutover Execution command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyAgentSinkReceiptRecord {
    pub decision_id: String,
}

/// Runtime configuration for a threshold based curation rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCurationRuleConfig {
    /// Standing condition selected by elevated curation theory.
    #[serde(default)]
    pub maintained_condition_id: Option<String>,
    /// Belief dimension the rule evaluates.
    pub dimension_id: String,
    /// Confidence threshold below which a goal may be proposed.
    pub threshold: f64,
    /// Urgency copied into emitted goal priority.
    pub priority_urgency: u32,
    /// Desired state summary copied into emitted goal source.
    pub desired_summary: String,
    /// Source family copied into dedupe state.
    pub source_kind: String,
}

impl AgentCurationRuleConfig {
    /// Validate the dimension, finite threshold, and source metadata.
    pub fn validate(&self) -> Result<(), StorageError> {
        if let Some(condition_id) = &self.maintained_condition_id {
            require_non_empty("rule maintained condition id", condition_id)?;
        }
        require_non_empty("rule dimension id", &self.dimension_id)?;
        if !(0.0..=1.0).contains(&self.threshold) || !self.threshold.is_finite() {
            return Err(StorageError::InvalidPath(
                "rule threshold must be a finite probability".to_string(),
            ));
        }
        require_non_empty("rule desired summary", &self.desired_summary)?;
        require_non_empty("rule source kind", &self.source_kind)?;
        Ok(())
    }

    /// Build the goal target condition represented by this rule.
    pub fn target_condition(&self) -> Condition {
        Condition::Above(Term::Literal(Literal::Number(self.threshold)))
    }

    /// Return the stable condition fragment for dedupe keys.
    pub fn target_condition_key(&self) -> String {
        condition_key(&self.target_condition())
    }
}

/// Curation rule bound to an agent record as installed theory.
///
/// The content hash pins the exact installed rule revision so decisions
/// can cite which rule produced them, on the same pattern as the belief
/// family registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCurationRuleBinding {
    /// Installed rule configuration.
    pub rule: AgentCurationRuleConfig,
    /// Content hash over the serialized rule, the revision identity.
    pub content_hash: String,
    /// Exact registry reference for elevated Agent records.
    #[serde(default)]
    pub revision: Option<crate::belief::TheoryRevisionRef>,
}

impl AgentCurationRuleBinding {
    /// Build a binding whose content hash pins the exact rule revision.
    pub fn for_rule(rule: AgentCurationRuleConfig) -> Result<Self, StorageError> {
        rule.validate()?;
        let content_hash = Self::content_hash_for(&rule)?;
        Ok(Self {
            rule,
            content_hash,
            revision: None,
        })
    }

    /// Build a compatibility binding that also cites its exact owner revision.
    pub fn for_revision(
        rule_id: &str,
        revision_content_hash: &str,
        rule: AgentCurationRuleConfig,
    ) -> Result<Self, StorageError> {
        let mut binding = Self::for_rule(rule)?;
        binding.revision = Some(crate::belief::TheoryRevisionRef {
            registry: "agent_curation_rule".to_string(),
            id: rule_id.to_string(),
            content_hash: revision_content_hash.to_string(),
        });
        binding.validate()?;
        Ok(binding)
    }

    /// Compute the canonical content hash for a rule configuration.
    ///
    /// Uses the crate's stable-hash path over the serialized rule so the
    /// same rule content always yields the same revision identity.
    pub fn content_hash_for(rule: &AgentCurationRuleConfig) -> Result<String, StorageError> {
        let bytes =
            serde_json::to_vec(rule).map_err(|err| StorageError::InvalidPath(err.to_string()))?;
        Ok(stable_hash_hex(&bytes))
    }

    /// Validate the bound rule and verify hash integrity against its content.
    pub fn validate(&self) -> Result<(), StorageError> {
        self.rule.validate()?;
        require_non_empty("curation rule content hash", &self.content_hash)?;
        if self.content_hash != Self::content_hash_for(&self.rule)? {
            return Err(StorageError::InvalidPath(
                "curation rule content hash does not match rule content".to_string(),
            ));
        }
        if let Some(revision) = &self.revision {
            revision.validate_for_registry("agent_curation_rule")?;
        }
        Ok(())
    }
}

/// Command to create an idempotent seed agent record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeedAgentRegistration {
    /// Stable seed agent identifier.
    pub agent_id: AgentId,
    /// Perspective used by the seed agent.
    pub perspective_key: PerspectiveKey,
    /// Subject observed by the seed agent.
    pub subject: DomainObjectRef,
    /// Branch observed by the seed agent.
    pub branch_scope: BranchScope,
    /// Named observation scope for the seed.
    pub observation_scope: String,
    /// Directive stored on the agent record.
    pub directive: String,
    /// Provenance stored on the agent record.
    pub seed_provenance: String,
    /// Curation rule installed with the seed registration.
    #[serde(default)]
    pub curation_rule: Option<AgentCurationRuleBinding>,
    /// Exact curation-rule revision installed with the seed registration.
    #[serde(default)]
    pub curation_rule_revision: Option<crate::belief::TheoryRevisionRef>,
    /// Standing condition installed with the seed registration.
    #[serde(default)]
    pub maintained_condition: Option<super::AgentMaintainedConditionBinding>,
    /// Exact maintained-condition revision installed with the seed registration.
    #[serde(default)]
    pub maintained_condition_revision: Option<crate::belief::TheoryRevisionRef>,
    /// Sequence used for create and update timestamps.
    pub created_at_seq: u64,
}

impl SeedAgentRegistration {
    /// Validate required seed registration fields and any rule binding.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("agent id", &self.agent_id)?;
        self.perspective_key.validate()?;
        self.subject.validate()?;
        require_non_empty("branch scope", &self.branch_scope.branch_id)?;
        require_non_empty("observation scope", &self.observation_scope)?;
        require_non_empty("directive", &self.directive)?;
        require_non_empty("seed provenance", &self.seed_provenance)?;
        if let Some(binding) = &self.curation_rule {
            binding.validate()?;
        }
        if let Some(reference) = &self.curation_rule_revision {
            reference.validate_for_registry("agent_curation_rule")?;
        }
        if let Some(binding) = &self.maintained_condition {
            binding.validate()?;
        }
        if let Some(reference) = &self.maintained_condition_revision {
            reference.validate_for_registry("agent_maintained_condition")?;
        }
        match (
            &self.maintained_condition,
            &self.maintained_condition_revision,
        ) {
            (Some(binding), Some(reference)) if &binding.revision == reference => {}
            (None, None) => {}
            _ => {
                return Err(StorageError::InvalidPath(
                    "seed maintained condition body and revision must match".to_string(),
                ))
            }
        }
        Ok(())
    }
}

/// Command to bind an agent to one belief stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SubscribeAgentCommand {
    /// Agent that owns the subscription.
    pub agent_id: AgentId,
    /// Belief stream to deliver.
    pub belief_key: BeliefKey,
    /// Sequence assigned when creating a new binding.
    pub created_at_seq: u64,
}

impl SubscribeAgentCommand {
    /// Validate the agent identifier and belief key.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("agent id", &self.agent_id)?;
        self.belief_key.validate()?;
        Ok(())
    }
}

pub(crate) fn require_non_empty(label: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(StorageError::InvalidPath(format!(
            "{label} must be non-empty"
        )));
    }
    Ok(())
}

pub(crate) fn condition_key(condition: &Condition) -> String {
    serde_json::to_string(condition).expect("condition serialization is infallible")
}

pub(crate) fn stable_hash_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

pub(crate) fn deterministic_id(prefix: &str, key: &str) -> String {
    format!("{prefix}-{}", stable_hash_hex(key.as_bytes()))
}
