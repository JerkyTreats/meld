//! Public agent records and command contracts.

use meld_lang::{Condition, Goal, GoalLifecycle, Literal, Proposition, Term};
use serde::{Deserialize, Serialize};

use crate::belief::{BeliefKey, BranchScope};
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::planner::PlannerProjectionOutput;
use crate::world_state::graph::PerspectiveKey;

/// Stable durable identifier for an agent.
pub type AgentId = String;
/// Stable durable identifier for an agent subscription.
pub type AgentSubscriptionId = String;
/// Stable durable identifier for an activation lease record.
pub type AgentActivationId = String;
/// Stable durable identifier for a curation decision.
pub type AgentDecisionId = String;
/// Stable durable identifier for a goal command emitted by an agent.
pub type AgentGoalCommandId = String;
/// Stable durable identifier for a goal mutation command emitted by an agent.
pub type AgentGoalMutationCommandId = String;
/// Stable durable identifier for an accepted agent sink receipt.
pub type AgentSinkReceiptId = String;

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

/// Classification of a persisted curation decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentDecisionKind {
    /// The decision produced a goal command boundary object.
    GoalCommand,
    /// The decision produced a goal mutation boundary object.
    GoalMutationCommand,
    /// The input was understood but no new command was needed.
    Absorbed,
    /// The input could not produce a determinate command.
    Indeterminate,
}

/// Execution boundary that accepted or replayed an agent-authored command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentSinkReceiptKind {
    /// Receipt for a proposed goal command.
    GoalCommand,
    /// Receipt for a goal lifecycle mutation command.
    GoalMutationCommand,
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
    /// Durable directive that explains agent intent.
    pub directive_id: String,
    /// Provenance string for seed created agents.
    pub seed_provenance: String,
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
        require_non_empty("directive id", &self.directive_id)?;
        require_non_empty("seed provenance", &self.seed_provenance)?;
        Ok(())
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

/// Stable key used to suppress duplicate curation output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCurationDedupeKey {
    /// Agent that owns the decision family.
    pub agent_id: AgentId,
    /// Stable subject index key.
    pub subject_key: String,
    /// Branch identifier for the belief stream.
    pub branch_id: String,
    /// Runtime configured dimension identifier.
    pub dimension_id: String,
    /// Serialized target condition used by the generated goal.
    pub target_condition_key: String,
    /// Runtime configured source family for the goal.
    pub source_kind: String,
}

impl AgentCurationDedupeKey {
    /// Build the dedupe key for a configured threshold rule.
    pub fn threshold_rule(
        agent_id: impl Into<String>,
        subject: &DomainObjectRef,
        branch_scope: &BranchScope,
        rule: &AgentCurationRuleConfig,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            subject_key: subject.index_key(),
            branch_id: branch_scope.branch_id.clone(),
            dimension_id: rule.dimension_id.clone(),
            target_condition_key: rule.target_condition_key(),
            source_kind: rule.source_kind.clone(),
        }
    }

    /// Return the canonical storage key for dedupe indexes.
    pub fn index_key(&self) -> String {
        format!(
            "{}::{}::{}::{}::{}::{}",
            self.agent_id,
            self.subject_key,
            self.branch_id,
            self.dimension_id,
            self.target_condition_key,
            self.source_kind
        )
    }

    /// Validate every dedupe component used in storage indexes.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("dedupe agent id", &self.agent_id)?;
        require_non_empty("dedupe subject key", &self.subject_key)?;
        require_non_empty("dedupe branch id", &self.branch_id)?;
        require_non_empty("dedupe dimension id", &self.dimension_id)?;
        require_non_empty("dedupe target condition key", &self.target_condition_key)?;
        require_non_empty("dedupe source kind", &self.source_kind)?;
        Ok(())
    }
}

/// Runtime configuration for a threshold based curation rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCurationRuleConfig {
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

/// Durable references to the state used for one curation decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCurationInputRefs {
    /// Belief revision supplied by delivery, when available.
    pub belief_revision_id: Option<String>,
    /// Belief stream read for the decision.
    pub belief_key: BeliefKey,
    /// Planner projection version read for the decision.
    pub planner_projection_version: String,
    /// Planner source references captured as stable debug strings.
    pub planner_source_refs: Vec<String>,
    /// Planner warnings captured as stable debug strings.
    pub planner_warnings: Vec<String>,
}

impl AgentCurationInputRefs {
    /// Validate references required to replay or inspect a decision.
    pub fn validate(&self) -> Result<(), StorageError> {
        self.belief_key.validate()?;
        require_non_empty(
            "planner projection version",
            &self.planner_projection_version,
        )?;
        Ok(())
    }
}

/// Durable output of one curation pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCurationDecision {
    /// Stable decision identifier.
    pub decision_id: AgentDecisionId,
    /// Agent that made the decision.
    pub agent_id: AgentId,
    /// Subscription that delivered the input.
    pub subscription_id: AgentSubscriptionId,
    /// Decision classification.
    pub decision: AgentDecisionKind,
    /// Goal command emitted by the decision, when present.
    pub goal_command_id: Option<AgentGoalCommandId>,
    /// Goal mutation command emitted by the decision, when present.
    #[serde(default)]
    pub goal_mutation_command_id: Option<AgentGoalMutationCommandId>,
    /// Dedupe key that defines the command family.
    pub dedupe_key: AgentCurationDedupeKey,
    /// References to belief and planner inputs used by the decision.
    pub input_refs: AgentCurationInputRefs,
    /// Human readable reason for the decision.
    pub reason: String,
    /// Sequence assigned when the decision was created.
    pub created_at_seq: u64,
}

impl AgentCurationDecision {
    /// Validate decision identifiers, indexes, references, and reason.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("decision id", &self.decision_id)?;
        require_non_empty("agent id", &self.agent_id)?;
        require_non_empty("subscription id", &self.subscription_id)?;
        self.dedupe_key.validate()?;
        self.input_refs.validate()?;
        require_non_empty("decision reason", &self.reason)?;
        Ok(())
    }
}

/// Stable identity returned by an execution-owned sink after command submit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSinkSubmission {
    /// Command id accepted or absorbed by the execution boundary.
    pub command_id: String,
    /// Goal id that execution associated with the submitted command.
    pub goal_id: String,
    /// Sink-specific outcome label such as applied, duplicate, or recovered.
    pub outcome: String,
}

impl AgentSinkSubmission {
    /// Build a sink submission identity.
    pub fn new(
        command_id: impl Into<String>,
        goal_id: impl Into<String>,
        outcome: impl Into<String>,
    ) -> Self {
        Self {
            command_id: command_id.into(),
            goal_id: goal_id.into(),
            outcome: outcome.into(),
        }
    }

    /// Validate identifiers returned by the sink.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("sink command id", &self.command_id)?;
        require_non_empty("sink goal id", &self.goal_id)?;
        require_non_empty("sink outcome", &self.outcome)?;
        if !matches!(self.outcome.as_str(), "applied" | "duplicate" | "recovered") {
            return Err(StorageError::InvalidPath(format!(
                "sink outcome '{}' is not an accepted command outcome",
                self.outcome
            )));
        }
        Ok(())
    }
}

/// Durable receipt that lets an agent retry advance without resubmitting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSinkReceipt {
    /// Stable receipt identifier.
    pub receipt_id: AgentSinkReceiptId,
    /// Decision whose command crossed the execution boundary.
    pub decision_id: AgentDecisionId,
    /// Type of command accepted by execution.
    pub kind: AgentSinkReceiptKind,
    /// Accepted sink identity.
    pub submission: AgentSinkSubmission,
    /// Sequence assigned when the receipt was recorded.
    pub recorded_at_seq: u64,
}

impl AgentSinkReceipt {
    /// Build a receipt from a persisted decision and accepted submission.
    pub fn new(
        decision: &AgentCurationDecision,
        kind: AgentSinkReceiptKind,
        submission: AgentSinkSubmission,
    ) -> Self {
        Self {
            receipt_id: deterministic_id(
                "agent-sink-receipt",
                &format!("{}::{}", decision.decision_id, submission.command_id),
            ),
            decision_id: decision.decision_id.clone(),
            kind,
            submission,
            recorded_at_seq: decision.created_at_seq,
        }
    }

    /// Validate receipt identity and sink submission.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("sink receipt id", &self.receipt_id)?;
        require_non_empty("sink receipt decision id", &self.decision_id)?;
        self.submission.validate()?;
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
    /// Durable directive referenced by the agent record.
    pub directive_id: String,
    /// Provenance stored on the agent record.
    pub seed_provenance: String,
    /// Sequence used for create and update timestamps.
    pub created_at_seq: u64,
}

impl SeedAgentRegistration {
    /// Validate required seed registration fields.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("agent id", &self.agent_id)?;
        self.perspective_key.validate()?;
        self.subject.validate()?;
        require_non_empty("branch scope", &self.branch_scope.branch_id)?;
        require_non_empty("observation scope", &self.observation_scope)?;
        require_non_empty("directive id", &self.directive_id)?;
        require_non_empty("seed provenance", &self.seed_provenance)?;
        Ok(())
    }
}

/// Command to bind an agent to one belief stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscribeAgentCommand {
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

/// Command to advance a subscription delivery cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvanceSubscriptionCommand {
    /// Agent that owns the subscription.
    pub agent_id: AgentId,
    /// Subscription whose cursor is advanced.
    pub subscription_id: AgentSubscriptionId,
    /// Delivered belief revision persisted after the decision.
    pub delivered_revision_id: String,
    /// Delivered sequence persisted as the cursor.
    pub delivered_seq: u64,
}

impl AdvanceSubscriptionCommand {
    /// Validate identifiers required for cursor advancement.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("agent id", &self.agent_id)?;
        require_non_empty("subscription id", &self.subscription_id)?;
        require_non_empty("delivered revision id", &self.delivered_revision_id)?;
        Ok(())
    }
}

/// Compare-and-swap intent for one agent subscription cursor advancement.
///
/// The command remains the canonical requested transition. Expected cursor
/// fields are operational fencing metadata that prevent concurrent deliveries
/// from overwriting a newer durable position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "AgentSubscriptionCursorCasIntentWire")]
pub struct AgentSubscriptionCursorCasIntent {
    /// Revision id observed before the attempted advancement.
    expected_delivered_revision_id: Option<String>,
    /// Sequence observed before the attempted advancement.
    expected_delivered_seq: u64,
    /// Complete canonical cursor advancement command.
    command: AdvanceSubscriptionCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct AgentSubscriptionCursorCasIntentWire {
    expected_delivered_revision_id: Option<String>,
    expected_delivered_seq: u64,
    command: AdvanceSubscriptionCommand,
}

impl AgentSubscriptionCursorCasIntent {
    /// Bind a strictly advancing command to the exact expected cursor.
    pub fn try_new(
        expected_delivered_revision_id: Option<String>,
        expected_delivered_seq: u64,
        command: AdvanceSubscriptionCommand,
    ) -> Result<Self, StorageError> {
        command.validate()?;
        match expected_delivered_revision_id.as_deref() {
            None if expected_delivered_seq != 0 => {
                return Err(StorageError::InvalidPath(
                    "empty subscription cursor must have sequence zero".to_string(),
                ));
            }
            Some(revision_id) => require_non_empty("expected delivered revision id", revision_id)?,
            None => {}
        }
        if expected_delivered_revision_id.is_some() && expected_delivered_seq == 0 {
            return Err(StorageError::InvalidPath(
                "non-empty subscription cursor must have a positive sequence".to_string(),
            ));
        }
        if command.delivered_seq <= expected_delivered_seq {
            return Err(StorageError::InvalidPath(
                "subscription cursor advancement must increase sequence".to_string(),
            ));
        }
        if expected_delivered_revision_id.as_deref() == Some(command.delivered_revision_id.as_str())
        {
            return Err(StorageError::InvalidPath(
                "subscription cursor advancement must name a newer revision".to_string(),
            ));
        }
        Ok(Self {
            expected_delivered_revision_id,
            expected_delivered_seq,
            command,
        })
    }

    /// Borrow the expected revision id.
    pub fn expected_delivered_revision_id(&self) -> Option<&str> {
        self.expected_delivered_revision_id.as_deref()
    }

    /// Return the expected durable sequence.
    pub fn expected_delivered_seq(&self) -> u64 {
        self.expected_delivered_seq
    }

    /// Borrow the canonical advancement command.
    pub fn command(&self) -> &AdvanceSubscriptionCommand {
        &self.command
    }
}

impl TryFrom<AgentSubscriptionCursorCasIntentWire> for AgentSubscriptionCursorCasIntent {
    type Error = StorageError;

    fn try_from(value: AgentSubscriptionCursorCasIntentWire) -> Result<Self, Self::Error> {
        Self::try_new(
            value.expected_delivered_revision_id,
            value.expected_delivered_seq,
            value.command,
        )
    }
}

/// Command wrapper for persisting a curation decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordCurationDecisionCommand {
    /// Decision to write to durable storage.
    pub decision: AgentCurationDecision,
}

/// World-model curation output that proposes one execution goal.
///
/// Execution does not interpret this producer-specific object directly.
/// Integration maps it into a neutral execution goal acceptance request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentGoalCommand {
    /// Stable command identifier reused for execution idempotency.
    pub command_id: AgentGoalCommandId,
    /// Ground proposed goal owned by the emitting agent.
    pub goal: Goal,
    /// Dedupe key that must match the goal target and agent.
    pub dedupe_key: AgentCurationDedupeKey,
}

impl AgentGoalCommand {
    /// Validate producer invariants before crossing into execution.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("goal command id", &self.command_id)?;
        self.dedupe_key.validate()?;
        if let Some(variable) = self.goal.target.grounding_issue() {
            return Err(StorageError::InvalidPath(format!(
                "agent goal command target must be ground: {variable}"
            )));
        }
        if !matches!(self.goal.lifecycle, GoalLifecycle::Proposed) {
            return Err(StorageError::InvalidPath(
                "agent goal command lifecycle must be proposed".to_string(),
            ));
        }
        require_goal_matches_dedupe(&self.goal, &self.dedupe_key)?;
        Ok(())
    }
}

/// Agent-authored mutation against an existing execution goal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentGoalMutationKind {
    /// Mark an active goal satisfied at the supplied review sequence.
    Satisfy {
        /// Sequence where the agent established satisfaction.
        at_seq: u64,
    },
}

/// World-model curation output that requests a goal lifecycle mutation.
///
/// Execution owns durable lifecycle state. Integration validates and maps this
/// command into execution's public goal set API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentGoalMutationCommand {
    /// Stable command identifier reused for execution idempotency.
    pub command_id: AgentGoalMutationCommandId,
    /// Agent that authored the mutation.
    pub agent_id: AgentId,
    /// Existing execution goal to mutate.
    pub goal_id: String,
    /// Mutation kind and mutation-specific data.
    pub kind: AgentGoalMutationKind,
    /// Dedupe key for the target goal family.
    pub dedupe_key: AgentCurationDedupeKey,
    /// Caller supplied review sequence used for idempotency and timestamps.
    pub review_seq: u64,
    /// Planner projection version used for the satisfaction review.
    pub projection_version: String,
    /// Planner source references copied from the review input.
    pub planner_source_refs: Vec<String>,
    /// Planner warnings copied from the review input.
    pub planner_warnings: Vec<String>,
}

impl AgentGoalMutationCommand {
    /// Validate invariants before crossing into the execution boundary.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("agent goal mutation command id", &self.command_id)?;
        require_non_empty("agent id", &self.agent_id)?;
        require_non_empty("goal id", &self.goal_id)?;
        if self.review_seq == 0 {
            return Err(StorageError::InvalidPath(
                "review seq must be greater than zero".to_string(),
            ));
        }
        require_non_empty("projection version", &self.projection_version)?;
        self.dedupe_key.validate()?;
        if self.dedupe_key.agent_id != self.agent_id {
            return Err(StorageError::InvalidPath(
                "agent goal mutation dedupe key agent id mismatch".to_string(),
            ));
        }
        match &self.kind {
            AgentGoalMutationKind::Satisfy { at_seq } if *at_seq == self.review_seq => Ok(()),
            AgentGoalMutationKind::Satisfy { .. } => Err(StorageError::InvalidPath(
                "satisfy at seq must equal review seq".to_string(),
            )),
        }
    }
}

/// Snapshot of execution goals visible to curation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActiveGoalSummary {
    /// Active or proposed goals supplied by the execution boundary.
    pub goals: Vec<Goal>,
}

impl ActiveGoalSummary {
    /// Return the first active or proposed goal that matches the dedupe key.
    pub fn first_open_matching_goal(&self, dedupe_key: &AgentCurationDedupeKey) -> Option<&Goal> {
        self.goals.iter().find(|goal| {
            matches!(
                goal.lifecycle,
                GoalLifecycle::Active | GoalLifecycle::Proposed
            ) && goal.agent_id == dedupe_key.agent_id
                && goal_matches_dedupe(goal, dedupe_key)
        })
    }

    /// Return the first visible goal that matches the dedupe key.
    pub fn first_matching_goal(&self, dedupe_key: &AgentCurationDedupeKey) -> Option<&Goal> {
        self.goals.iter().find(|goal| {
            goal.agent_id == dedupe_key.agent_id && goal_matches_dedupe(goal, dedupe_key)
        })
    }

    /// Return whether an active or proposed goal already matches the dedupe key.
    pub fn has_matching_goal(&self, dedupe_key: &AgentCurationDedupeKey) -> bool {
        self.first_open_matching_goal(dedupe_key).is_some()
    }
}

/// Complete input to the pure curation rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCurationInput {
    /// Agent record used for scope and ownership checks.
    pub agent: AgentRecord,
    /// Subscription that delivered the input.
    pub subscription: AgentSubscriptionRecord,
    /// Delivery sequence used for decision ordering.
    pub delivered_seq: u64,
    /// Runtime rule configuration applied to the input.
    pub rule_config: AgentCurationRuleConfig,
    /// Current belief view, if the belief store has one.
    pub belief_view: Option<crate::belief::BeliefView>,
    /// Planner projection read for the same subject and branch.
    pub planner_projection: PlannerProjectionOutput,
    /// Execution goal snapshot used for absorption.
    pub active_goals: ActiveGoalSummary,
    /// Durable references to the curation inputs.
    pub input_refs: AgentCurationInputRefs,
}

/// Complete input to pure agent-owned satisfaction curation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentGoalSatisfactionInput {
    /// Agent record used for scope and ownership checks.
    pub agent: AgentRecord,
    /// Subscription that anchors the review to one belief stream.
    pub subscription: AgentSubscriptionRecord,
    /// Caller supplied review sequence used for decision identity.
    pub review_seq: u64,
    /// Planner projection evaluated against active goal targets.
    pub planner_projection: PlannerProjectionOutput,
    /// Execution goal snapshot used to select active owned goals.
    pub active_goals: ActiveGoalSummary,
    /// Durable references to the curation inputs.
    pub input_refs: AgentCurationInputRefs,
}

/// Durable review envelope for agent-owned goal satisfaction checks.
///
/// The review identity is assigned by the runtime caller and is distinct from
/// belief revision identity, since several reviews may inspect the same belief
/// state while execution retries or checkpoint recovery are in progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSatisfactionReview {
    /// Agent that owns the satisfaction judgment.
    pub agent_id: AgentId,
    /// Subscription anchoring the review to one belief stream.
    pub subscription_id: AgentSubscriptionId,
    /// Caller supplied review sequence used for decision identity.
    pub review_seq: u64,
}

impl AgentSatisfactionReview {
    /// Validate identifiers and the monotonic review sequence.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("agent id", &self.agent_id)?;
        require_non_empty("subscription id", &self.subscription_id)?;
        if self.review_seq == 0 {
            return Err(StorageError::InvalidPath(
                "review seq must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }

    /// Return the stable storage key for one satisfaction review attempt.
    pub fn index_key(&self) -> String {
        format!(
            "{}::{}::{}",
            self.agent_id, self.subscription_id, self.review_seq
        )
    }
}

/// Delivery envelope supplied by a runtime subscription driver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDelivery {
    /// Agent that should receive the delivery.
    pub agent_id: AgentId,
    /// Subscription that produced the delivery.
    pub subscription_id: AgentSubscriptionId,
    /// Belief revision being delivered.
    pub belief_revision_id: String,
    /// Monotonic sequence for cursor checks.
    pub revision_seq: u64,
}

impl AgentDelivery {
    /// Validate delivery identifiers.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("agent id", &self.agent_id)?;
        require_non_empty("subscription id", &self.subscription_id)?;
        require_non_empty("belief revision id", &self.belief_revision_id)?;
        Ok(())
    }
}

/// Result of one curation pass after persistence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCurationOutcome {
    /// Persisted decision for the delivered input.
    pub decision: AgentCurationDecision,
    /// Optional goal command returned to the execution boundary.
    pub goal_command: Option<AgentGoalCommand>,
    /// Optional goal mutation command returned to the execution boundary.
    #[serde(default)]
    pub goal_mutation_command: Option<AgentGoalMutationCommand>,
}

pub(crate) fn require_non_empty(label: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(StorageError::InvalidPath(format!(
            "{label} must be non-empty"
        )));
    }
    Ok(())
}

pub(crate) fn threshold_target(
    subject: DomainObjectRef,
    dimension_id: String,
    condition: Condition,
) -> Proposition {
    Proposition::Holds {
        subject: Term::Object(subject),
        dimension: Term::Dimension(dimension_id),
        condition,
    }
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

fn goal_matches_dedupe(goal: &Goal, dedupe_key: &AgentCurationDedupeKey) -> bool {
    match &goal.target {
        Proposition::Holds {
            subject,
            dimension,
            condition,
        } => {
            let Term::Object(subject) = subject else {
                return false;
            };
            let Term::Dimension(dimension) = dimension else {
                return false;
            };
            subject.index_key() == dedupe_key.subject_key
                && dimension == &dedupe_key.dimension_id
                && condition_key(condition) == dedupe_key.target_condition_key
        }
        _ => false,
    }
}

fn require_goal_matches_dedupe(
    goal: &Goal,
    dedupe_key: &AgentCurationDedupeKey,
) -> Result<(), StorageError> {
    if goal.agent_id != dedupe_key.agent_id {
        return dedupe_mismatch("agent id");
    }

    let Proposition::Holds {
        subject,
        dimension,
        condition,
    } = &goal.target
    else {
        return dedupe_mismatch("goal target kind");
    };

    let Term::Object(subject) = subject else {
        return dedupe_mismatch("goal target subject");
    };
    if subject.index_key() != dedupe_key.subject_key {
        return dedupe_mismatch("goal target subject");
    }

    let Term::Dimension(dimension) = dimension else {
        return dedupe_mismatch("goal target dimension");
    };
    if dimension != &dedupe_key.dimension_id {
        return dedupe_mismatch("goal target dimension");
    }

    if condition_key(condition) != dedupe_key.target_condition_key {
        return dedupe_mismatch("goal target condition");
    }

    Ok(())
}

fn dedupe_mismatch(field: &str) -> Result<(), StorageError> {
    Err(StorageError::InvalidPath(format!(
        "agent goal command dedupe key does not match {field}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn advance_command() -> AdvanceSubscriptionCommand {
        AdvanceSubscriptionCommand {
            agent_id: "agent-a".to_string(),
            subscription_id: "subscription-a".to_string(),
            delivered_revision_id: "revision-b".to_string(),
            delivered_seq: 12,
        }
    }

    #[test]
    fn subscription_cursor_cas_intent_round_trips_canonical_command() {
        let intent = AgentSubscriptionCursorCasIntent::try_new(
            Some("revision-a".to_string()),
            11,
            advance_command(),
        )
        .unwrap();

        let value = serde_json::to_value(&intent).unwrap();
        assert_eq!(value["expected_delivered_seq"], 11);
        assert_eq!(value["command"]["delivered_revision_id"], "revision-b");
        assert_eq!(
            serde_json::from_value::<AgentSubscriptionCursorCasIntent>(value).unwrap(),
            intent
        );
        assert!(AgentSubscriptionCursorCasIntent::try_new(
            Some("revision-b".to_string()),
            12,
            advance_command(),
        )
        .is_err());
    }

    #[test]
    fn legacy_subscription_advance_command_shape_remains_unchanged() {
        let legacy = serde_json::json!({
            "agent_id": "agent-a",
            "subscription_id": "subscription-a",
            "delivered_revision_id": "revision-b",
            "delivered_seq": 12
        });
        let decoded: AdvanceSubscriptionCommand = serde_json::from_value(legacy.clone()).unwrap();
        let encoded = serde_json::to_value(decoded).unwrap();

        assert_eq!(encoded, legacy);
        assert!(encoded.get("expected_delivered_seq").is_none());
        assert!(encoded.get("command").is_none());
    }
}
