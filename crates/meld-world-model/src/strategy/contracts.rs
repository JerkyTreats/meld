//! Public contracts for bounded Strategy construction.

use meld_lang::{Bindings, Composition, Goal, Method, Operator, Proposition};
use serde::{Deserialize, Serialize};

/// Immutable domain theory needed to connect one Goal to prospective evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyTheorySnapshot {
    /// Stable content identity for the activated theory.
    pub theory_id: String,
    /// Rules that transform Goal targets into settlement obligations.
    pub settlement_rules: Vec<StrategySettlementRule>,
}

/// Declared settlement meaning for one class of Goals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategySettlementRule {
    /// Goal pattern to which this rule applies.
    pub goal_pattern: Proposition,
    /// Proposition that prospective action must contribute toward.
    pub settlement_obligation: Proposition,
    /// Evidence route required after action completes.
    pub evidence_route: ProspectiveEvidenceRoute,
    /// Whether the configured epistemic products prepare work or confirm its effects.
    #[serde(
        default,
        skip_serializing_if = "StrategyEpistemicPlacement::is_prerequisite"
    )]
    pub epistemic_placement: StrategyEpistemicPlacement,
}

/// Causal placement of configured epistemic work relative to executable realization.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyEpistemicPlacement {
    #[default]
    Prerequisite,
    Confirmation,
}

impl StrategyEpistemicPlacement {
    fn is_prerequisite(&self) -> bool {
        *self == Self::Prerequisite
    }
}

/// Prospective route from action outcome to later evidence admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectiveEvidenceRoute {
    /// Stable route identity.
    pub route_id: String,
    /// Belief dimension whose evidence this route may produce.
    pub dimension_id: String,
    /// Governed outcome contract expected from realization.
    pub outcome_contract_id: String,
    /// Evidence schema accepted by the owning belief domain.
    pub evidence_schema_id: String,
}

/// Neutral semantic view of one atomic executable contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyCapability {
    /// Exact contract content identity supplied by root assembly.
    pub contract_id: String,
    /// Semantic operator template exposed to Strategy.
    pub operator: Operator,
    /// Outcome contract produced by successful realization.
    pub outcome_contract_id: String,
}

/// Deterministic comparison policy for the minimal engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyEvaluationPolicy {
    /// Stable policy identity included in result identity.
    pub policy_id: String,
    /// Prefer candidates with fewer action steps before comparing cost.
    pub prefer_fewer_steps: bool,
}

/// Structural bounds for one deterministic search request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategySearchBounds {
    /// Maximum Capability positions expanded during the attempt.
    pub max_expansions: usize,
    /// Maximum producer depth used to close an artifact obligation.
    pub max_depth: usize,
}

/// Complete installed Strategy theory body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyTheoryPackage {
    /// Settlement and prospective evidence semantics.
    pub snapshot: StrategyTheorySnapshot,
    /// Atomic construction vocabulary.
    pub capabilities: Vec<StrategyCapability>,
    /// Deterministic candidate comparison policy.
    pub evaluation_policy: StrategyEvaluationPolicy,
    /// Structural limits for bounded search.
    pub search_bounds: StrategySearchBounds,
    /// Belief projection dimensions required by this package.
    pub requested_dimensions: Vec<String>,
    /// Capability type identities whose invocation the package requests.
    #[serde(default)]
    pub requested_authority: Vec<String>,
}

/// Complete immutable input to the pure Strategy function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyProblem {
    /// Exact frozen inputs available to the selected Task steps.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_inputs: Vec<meld_lang::TaskInput>,
    /// Exact problem identity supplied by the caller.
    pub problem_id: String,
    /// Ground proposed Goal.
    pub goal: Goal,
    /// Complete immutable reasoning cut visible to Strategy.
    pub planner_cut: crate::planner::PlannerCut,
    /// Activated domain theory.
    pub theory: StrategyTheorySnapshot,
    /// Atomic construction vocabulary.
    pub capabilities: Vec<StrategyCapability>,
    /// Optional reusable solution templates.
    pub methods: Vec<Method>,
    /// Declared deterministic comparison policy.
    pub evaluation_policy: StrategyEvaluationPolicy,
    /// Exact constructible Curation operations frozen beside the cut.
    pub curation_operations: Vec<crate::CurationOperation>,
}

/// One bounded invocation of Strategy construction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategySearchRequest {
    /// Immutable semantic problem.
    pub problem: StrategyProblem,
    /// Explicit structural bounds.
    pub bounds: StrategySearchBounds,
}

/// Origin of a constructed candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyPlanOrigin {
    /// Desired state is already established by the admitted Planner input.
    Satisfied,
    /// Remaining epistemic work confirms an already completed executable product.
    Confirmation,
    /// Constructed directly from Capability contracts.
    Direct,
    /// Seeded by one reusable Method.
    Method { method_id: String },
}

/// Minimal deterministic evaluation vector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyPlanEvaluation {
    /// Number of atomic action steps.
    pub step_count: usize,
    /// Aggregate estimated time.
    pub time_ms: u64,
    /// Aggregate estimated provider calls.
    pub provider_calls: u32,
}

/// Owner milestone that discharges one exact Plan dependency.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanMilestoneRequirement {
    CurationTerminal {
        operation_id: String,
    },
    GraphVisible {
        owner_id: String,
        revision_id: String,
    },
    BeliefRevision {
        belief_key: String,
        revision_id: String,
    },
    AgentAccepted {
        product_id: String,
    },
    ExecutionTerminal {
        task_id: String,
    },
}

/// One independently complete executable product retained by Agent only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyTask {
    /// Subject on which Execution acts, independent of the Goal's observation subject.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_subject: Option<meld_events::DomainObjectRef>,
    /// Exact frozen inputs available to the selected Task steps.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub initial_inputs: Vec<meld_lang::TaskInput>,
    pub task_id: String,
    pub composition: Composition,
    #[serde(default = "empty_bindings")]
    pub bindings: Bindings,
    pub capability_contract_ids: Vec<String>,
    pub expected_outcome_contract_id: String,
    pub authority_requirements: Vec<String>,
    pub idempotency_key: String,
    #[serde(default)]
    pub return_milestone: Option<PlanMilestoneRequirement>,
}

fn empty_bindings() -> Bindings {
    Bindings::empty()
}

/// One bounded epistemic product grounded from the Curation catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyEpistemicOperation {
    pub product_id: String,
    pub operation: crate::CurationOperation,
    pub authority_requirements: Vec<String>,
    pub idempotency_key: String,
}

/// Canonical complete product body constructed by Strategy and authorized by Agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyProduct {
    Epistemic(Box<StrategyEpistemicOperation>),
    Task(Box<StrategyTask>),
}

/// Exact causal or information dependency between Plan products.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyPlanDependency {
    pub dependency_id: String,
    pub producer_product_id: String,
    pub consumer_product_id: String,
    pub required_milestone: PlanMilestoneRequirement,
}

/// One completed causal fact that reconstruction must preserve exactly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyCompletedHistoryEntry {
    /// Plan revision under which the product reached its milestone.
    pub source_plan_revision_id: String,
    /// Exact semantic product whose history is retained.
    pub product_id: String,
    /// Owner milestone accepted for that product.
    pub accepted_milestone: PlanMilestoneRequirement,
    /// Exact durable owner position that justified acceptance.
    pub owner_position_id: String,
    /// Original product hydrated from its source Plan, absent in legacy identity-only history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product: Option<StrategyProduct>,
}

/// Ground immutable heterogeneous Plan returned for Agent judgment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyPlan {
    /// Content-derived immutable Plan revision identity.
    pub plan_revision_id: String,
    /// Stable lineage identity for one Agent Goal.
    pub plan_family_id: String,
    /// Problem against which this candidate was constructed.
    pub problem_id: String,
    /// Exact Goal identity.
    pub goal_id: String,
    /// Exact Planner consistency root.
    pub planner_cut_id: String,
    /// Candidate construction origin.
    pub origin: StrategyPlanOrigin,
    /// Ground semantic action graph.
    pub composition: Composition,
    /// Ground bindings used during construction.
    pub bindings: Bindings,
    /// Settlement obligation discharged by the root action.
    pub settlement_obligation: Proposition,
    /// Prospective evidence from executable work, absent when no work is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_route: Option<ProspectiveEvidenceRoute>,
    /// Exact Capability contract identities selected by the candidate.
    pub capability_contract_ids: Vec<String>,
    /// Independently complete executable products.
    pub tasks: Vec<StrategyTask>,
    /// Independently complete bounded epistemic products.
    pub epistemic_operations: Vec<StrategyEpistemicOperation>,
    /// Exact inter-product causal ordering.
    pub dependencies: Vec<StrategyPlanDependency>,
    /// Exact desired conditions and satisfaction meaning.
    pub conditions: Vec<Proposition>,
    /// Frozen construction context identity.
    pub frozen_context_id: String,
    /// Human-inspectable deterministic explanation.
    pub explanation: String,
    /// Named predecessor when this revision reconstructs an earlier Plan.
    pub predecessor_plan_revision_id: Option<String>,
    /// Deterministic minimal evaluation.
    pub evaluation: StrategyPlanEvaluation,
}

/// Complete immutable input for reconstructing one successor Plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategySuccessorRequest {
    /// Frozen construction request for the successor revision.
    pub search: StrategySearchRequest,
    /// Exact immutable predecessor Plan, including its content identity.
    pub predecessor_plan: Box<StrategyPlan>,
    /// Completed causal history that the successor must retain unchanged.
    pub completed_history: Vec<StrategyCompletedHistoryEntry>,
}

/// One successor Plan paired with the unchanged completed causal history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategySuccessorPlan {
    /// Newly constructed immutable Plan revision.
    pub plan: StrategyPlan,
    /// Completed predecessor history preserved byte-for-byte semantically.
    pub completed_history: Vec<StrategyCompletedHistoryEntry>,
}

/// Result of one pure bounded successor construction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategySuccessorResult {
    /// Exact problem identity.
    pub problem_id: String,
    /// Completion posture of the bounded construction attempt.
    pub completion: StrategySearchCompletion,
    /// Strongest successor retained by the minimal engine.
    pub recommendation: Option<StrategySuccessorPlan>,
    /// Typed grounds observed while rejecting construction.
    pub rejections: Vec<StrategyRejectionGround>,
    /// Deterministic work counts.
    pub statistics: StrategySearchStatistics,
}

/// Why a branch could not become an eligible candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategyRejectionGround {
    /// Goal is not ground and proposed.
    InvalidGoal,
    /// No theory rule applies to the Goal target.
    NoSettlementRule,
    /// A required world-state precondition is false.
    UnsatisfiedPrecondition { operator_id: String },
    /// A required world-state precondition is indeterminate.
    IndeterminatePrecondition { operator_id: String },
    /// No Capability can produce a required artifact.
    UnclosedArtifact { artifact_type: String },
    /// Substitution left a variable unbound.
    UnboundVariable { variable: String },
    /// Candidate graph failed structural validation.
    InvalidComposition,
    /// Candidate outcome does not support the required evidence route.
    InvalidEvidenceRoute,
    /// Candidate content differs from its identity or problem anchoring.
    IdentityMismatch,
    /// The supplied predecessor is corrupt or belongs to another Plan family.
    InvalidPredecessor,
    /// Explicit structural bounds stopped the attempt.
    BoundsExceeded,
}

/// Honest completion posture for bounded search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StrategySearchCompletion {
    /// The finite reachable space was exhausted.
    Exhaustive,
    /// Search stopped at an explicit structural bound.
    Bounded,
}

/// Structural work counts for deterministic inspection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategySearchStatistics {
    /// Capability positions expanded.
    pub expanded_positions: usize,
}

/// Result of one pure bounded search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategySearchResult {
    /// Exact problem identity.
    pub problem_id: String,
    /// Completion posture of the attempt.
    pub completion: StrategySearchCompletion,
    /// Strongest candidate retained by the minimal engine.
    pub recommendation: Option<StrategyPlan>,
    /// Typed grounds observed while rejecting branches.
    pub rejections: Vec<StrategyRejectionGround>,
    /// Deterministic work counts.
    pub statistics: StrategySearchStatistics,
}

/// Result of independently checking a candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlanVerification {
    /// Candidate is sound for the supplied problem.
    Valid { evaluation: StrategyPlanEvaluation },
    /// Candidate is unsound for the supplied problem.
    Invalid {
        grounds: Vec<StrategyRejectionGround>,
    },
}
