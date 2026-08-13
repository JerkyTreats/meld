//! Public contracts for bounded Strategy construction.

use meld_lang::{Bindings, Composition, Goal, Method, Operator, Proposition, WorldState};
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
    /// Exact problem identity supplied by the caller.
    pub problem_id: String,
    /// Ground proposed Goal.
    pub goal: Goal,
    /// Exact planner projection visible to Strategy.
    pub world_state: WorldState,
    /// Stable planner projection identity.
    pub planner_snapshot_id: String,
    /// Activated domain theory.
    pub theory: StrategyTheorySnapshot,
    /// Atomic construction vocabulary.
    pub capabilities: Vec<StrategyCapability>,
    /// Optional reusable solution templates.
    pub methods: Vec<Method>,
    /// Declared deterministic comparison policy.
    pub evaluation_policy: StrategyEvaluationPolicy,
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
pub enum StrategyCandidateOrigin {
    /// Constructed directly from Capability contracts.
    Direct,
    /// Seeded by one reusable Method.
    Method { method_id: String },
}

/// Minimal deterministic evaluation vector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyCandidateEvaluation {
    /// Number of atomic action steps.
    pub step_count: usize,
    /// Aggregate estimated time.
    pub time_ms: u64,
    /// Aggregate estimated provider calls.
    pub provider_calls: u32,
}

/// Ground candidate returned for independent verification and Agent judgment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyCandidate {
    /// Content-derived candidate identity.
    pub candidate_id: String,
    /// Problem against which this candidate was constructed.
    pub problem_id: String,
    /// Exact Goal identity.
    pub goal_id: String,
    /// Exact planner snapshot identity.
    pub planner_snapshot_id: String,
    /// Candidate construction origin.
    pub origin: StrategyCandidateOrigin,
    /// Ground semantic action graph.
    pub composition: Composition,
    /// Ground bindings used during construction.
    pub bindings: Bindings,
    /// Settlement obligation discharged by the root action.
    pub settlement_obligation: Proposition,
    /// Prospective evidence route preserved for later reconciliation.
    pub evidence_route: ProspectiveEvidenceRoute,
    /// Exact Capability contract identities selected by the candidate.
    pub capability_contract_ids: Vec<String>,
    /// Deterministic minimal evaluation.
    pub evaluation: StrategyCandidateEvaluation,
}

/// Exact Agent-owned authorization for one verified Strategy candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyAuthorization {
    /// Content-derived authorization identity.
    pub authorization_id: String,
    /// Agent decision that owns this authorization.
    pub agent_decision_id: String,
    /// Exact verified candidate selected by the Agent.
    pub candidate: StrategyCandidate,
    /// Evaluation policy applied before authorization.
    pub evaluation_policy_id: String,
    /// Exact complete Strategy theory revision used for construction.
    #[serde(default)]
    pub strategy_theory_revision: Option<crate::belief::TheoryRevisionRef>,
    /// Effective authority under which this exact candidate may execute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority_decision: Option<meld_lang::AuthorityDecision>,
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
    pub recommendation: Option<StrategyCandidate>,
    /// Typed grounds observed while rejecting branches.
    pub rejections: Vec<StrategyRejectionGround>,
    /// Deterministic work counts.
    pub statistics: StrategySearchStatistics,
}

/// Result of independently checking a candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CandidateVerification {
    /// Candidate is sound for the supplied problem.
    Valid {
        evaluation: StrategyCandidateEvaluation,
    },
    /// Candidate is unsound for the supplied problem.
    Invalid {
        grounds: Vec<StrategyRejectionGround>,
    },
}
