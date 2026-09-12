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

/// Declared settlement meaning and local product relationships for one class of Goals.
#[derive(Debug, Clone, PartialEq)]
pub struct StrategySettlementRule {
    pub(crate) historical_wire: Option<Box<super::rule_history::HistoricalRule>>,
    pub repeat_on_changed_owners: Vec<String>,
    pub goal_pattern: Proposition,
    pub settlement_obligation: Proposition,
    pub evidence_route: ProspectiveEvidenceRoute,
    /// The bounded Curation products this rule selects from the frozen problem.
    pub epistemic_selections: Vec<StrategyEpistemicSelection>,
    /// Local prerequisites between selected complete products.
    pub product_ordering: Vec<StrategyProductOrdering>,
    pub construction: StrategyConstruction,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyConstruction {
    #[default]
    Executable,
    /// Select only bounded evidence acquisition while Goal knowledge is indeterminate.
    ObserveUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyEpistemicSelection {
    /// Absent selects every supplied bounded operation; a value names an exact Curation rule.
    pub rule_revision: Option<crate::belief::TheoryRevisionRef>,
    /// A semantic return must pass through Belief before Agent accepts it.
    pub evidence_return: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StrategyProductSelector {
    Task { contract_id: String },
    Epistemic { rule_id: String },
    AllTasks,
    AllEpistemic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyDependencyMilestone {
    ExecutionTerminal,
    EffectVisible,
    CurationVisible,
}

/// Whether a rule requires its producer, or only orders products selected together.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyOrderingCondition {
    #[default]
    ConsumerSelected,
    BothSelected,
}

impl StrategyOrderingCondition {
    fn is_default(&self) -> bool {
        *self == Self::ConsumerSelected
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StrategyProductOrdering {
    #[serde(default, skip_serializing_if = "StrategyOrderingCondition::is_default")]
    pub condition: StrategyOrderingCondition,
    pub before: StrategyProductSelector,
    pub after: StrategyProductSelector,
    pub milestone: StrategyDependencyMilestone,
}

impl StrategyProductSelector {
    pub(crate) fn selects_task(&self, task: &StrategyTask) -> bool {
        match self {
            Self::AllTasks => true,
            Self::Task { contract_id } => task.capability_contract_ids.contains(contract_id),
            _ => false,
        }
    }
    pub(crate) fn selects_epistemic(&self, operation: &StrategyEpistemicOperation) -> bool {
        match self {
            Self::AllEpistemic => true,
            Self::Epistemic { rule_id } => &operation.operation.rule_revision.id == rule_id,
            _ => false,
        }
    }
    pub(crate) fn is_exact(&self) -> bool {
        matches!(self, Self::Task { .. } | Self::Epistemic { .. })
    }
}

impl StrategySettlementRule {
    pub fn has_evidence_returns(&self) -> bool {
        self.epistemic_selections
            .iter()
            .any(|selection| selection.evidence_return)
    }
    pub fn requires_effect_visibility(&self) -> bool {
        self.product_ordering
            .iter()
            .any(|ordering| ordering.milestone == StrategyDependencyMilestone::EffectVisible)
    }
    pub(crate) fn task_requires_visibility(&self, task: &StrategyTask) -> bool {
        self.product_ordering.iter().any(|ordering| {
            ordering.milestone == StrategyDependencyMilestone::EffectVisible
                && ordering.before.selects_task(task)
        })
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
    /// Installed reusable templates, subject to the same capability contracts as direct search.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<Method>,
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
    /// Exact producer publication that may enable confirmation before operational return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect_visibility:
        Option<crate::world_state::graph::contracts::OwnerPublicationExpectation>,
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
    /// Previously admitted operations whose unchanged input has no successful return.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unavailable_curation_operation_ids: Vec<String>,
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
    /// Bounded evidence acquisition for an indeterminate Goal, with no executable Task.
    Epistemic,
    /// Desired state is already established by the admitted Planner input.
    Satisfied,
    /// Remaining epistemic work follows an accepted effect milestone from executable work.
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

/// Owner return or visibility milestone retained by Agent.
/// Negative returns account for a product without discharging positive dependencies.
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
    /// Successful Curation publication consumed by the declared Graph selection.
    CurationVisible {
        operation_id: String,
    },
    /// The operation returned unsuccessfully; it discharges no positive dependency.
    CurationUnsuccessful {
        operation_id: String,
    },
    /// Curation rejected intake; no accepted-operation result or semantic effect exists.
    CurationRejected {
        operation_id: String,
    },
    /// Execution refused intake; the Task has no admitted operational realization.
    ExecutionNotAdmitted {
        task_id: String,
    },
    /// Remaining admitted work was refused without invocation. No complete effect is proved.
    ExecutionInterrupted {
        task_id: String,
    },
}

/// One independently complete executable product retained by Agent only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyTask {
    /// Non-Curation owner knowledge against which this work was constructed.
    /// Confirmation of an old effect does not complete work over changed source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_basis_id: Option<String>,
    /// Exact producer publication that may enable confirmation before operational return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect_visibility:
        Option<crate::world_state::graph::contracts::OwnerPublicationExpectation>,
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

impl StrategyTask {
    pub(crate) fn accounts_for_return(&self, milestone: &PlanMilestoneRequirement) -> bool {
        self.return_milestone.as_ref() == Some(milestone)
            || matches!(milestone, PlanMilestoneRequirement::ExecutionNotAdmitted { task_id }
                | PlanMilestoneRequirement::ExecutionInterrupted { task_id }
                if task_id == &self.task_id)
    }

    pub fn confirmation_milestone(&self) -> PlanMilestoneRequirement {
        self.effect_visibility.as_ref().map_or_else(
            || PlanMilestoneRequirement::ExecutionTerminal {
                task_id: self.task_id.clone(),
            },
            |expected| PlanMilestoneRequirement::GraphVisible {
                owner_id: expected.owner_id.clone(),
                revision_id: expected.revision_id.clone(),
            },
        )
    }
}

fn empty_bindings() -> Bindings {
    Bindings::empty()
}

/// One bounded epistemic product grounded from the Curation catalog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategyEpistemicOperation {
    /// Configured Belief return; absence requires the native Curation Graph visibility return.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_evidence: Option<ProspectiveEvidenceRoute>,
    pub product_id: String,
    pub operation: crate::CurationOperation,
    pub authority_requirements: Vec<String>,
    pub idempotency_key: String,
}

impl StrategyEpistemicOperation {
    /// A completed named request survives its own publication changing the source cut.
    pub fn same_request_as(&self, other: &Self) -> bool {
        if self.return_evidence != other.return_evidence {
            return false;
        }
        match (&self.operation.request_id, &other.operation.request_id) {
            (Some(left), Some(right)) => {
                left == right
                    && self.operation.authority == other.operation.authority
                    && self.same_source_as(other)
            }
            _ => self.operation.selection_id == other.operation.selection_id,
        }
    }

    pub(super) fn same_source_as(&self, other: &Self) -> bool {
        self.return_evidence == other.return_evidence
            && self.operation.rule_revision == other.operation.rule_revision
            && self.operation.traversal_request == other.operation.traversal_request
            && self.source_basis() == other.source_basis()
    }

    fn source_basis(&self) -> Vec<crate::world_state::graph::contracts::OwnerGraphRevisionReceipt> {
        self.operation
            .source_cut
            .receipts
            .iter()
            .filter(|receipt| {
                receipt.owner_id != crate::curation::CURATION_OWNER_ID
                    || Some(&receipt.scope) != self.operation.publication_scope()
            })
            .map(|receipt| receipt.semantic_basis())
            .collect()
    }

    /// A negative return ends the product's debt without establishing its required evidence.
    pub(crate) fn accounts_for_return(&self, milestone: &PlanMilestoneRequirement) -> bool {
        self.accepts_return(milestone)
            || matches!(milestone, PlanMilestoneRequirement::CurationUnsuccessful { operation_id }
                | PlanMilestoneRequirement::CurationRejected { operation_id }
                if operation_id == &self.operation.operation_id)
    }

    pub fn accepts_return(&self, milestone: &PlanMilestoneRequirement) -> bool {
        match (&self.return_evidence, milestone) {
            (
                Some(_),
                PlanMilestoneRequirement::BeliefRevision {
                    belief_key,
                    revision_id,
                },
            ) => !belief_key.is_empty() && !revision_id.is_empty(),
            (None, PlanMilestoneRequirement::CurationVisible { operation_id }) => {
                operation_id == &self.operation.operation_id
            }
            _ => false,
        }
    }
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

/// Selected method hierarchy and its exact primitive derivation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyDecomposition {
    /// Abstract and primitive occurrences in the selected refinement traversal.
    pub expansion_order: Vec<String>,
    /// Selected compound refinements in parent-before-child order.
    pub refinements: Vec<StrategyRefinement>,
    /// Exact local bindings for each primitive occurrence in the resulting Tasks.
    pub primitives: Vec<StrategyPrimitiveBinding>,
}

/// One justified reduction of an abstract occurrence against projected state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StrategyRefinement {
    Satisfied {
        path: String,
        target: Proposition,
    },
    Method {
        path: String,
        target: Proposition,
        method_id: String,
        bindings: Bindings,
    },
}

/// A primitive occurrence retains its own scope of method variables.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategyPrimitiveBinding {
    pub step_id: String,
    pub bindings: Bindings,
}

/// Ground immutable heterogeneous Plan returned for Agent judgment.
#[derive(Debug, Clone, PartialEq)]
pub struct StrategyPlan {
    pub(super) historical_identity: Option<Box<super::history::HistoricalPlanIdentity>>,
    /// Exact installed settlement rule selected during construction; empty for satisfied Plans
    /// and historical records that predate explicit rule identity.
    pub settlement_rule_id: String,

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
    /// Absent on historical and direct Plans; selected hierarchy on refined Plans.
    pub decomposition: Option<StrategyDecomposition>,
    /// Ground bindings used during construction.
    pub bindings: Bindings,
    /// Settlement obligation discharged by the root action.
    pub settlement_obligation: Proposition,
    /// Prospective evidence from executable work, absent when no work is required.
    pub evidence_route: Option<ProspectiveEvidenceRoute>,
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
    /// A reusable template does not apply to the admitted world state.
    UnsatisfiedMethodPrecondition { method_id: String },
    /// A reusable template needs evidence absent from the admitted world state.
    IndeterminateMethodPrecondition { method_id: String },
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
    /// Repeating the same completed work has no new confirmed source basis.
    UnchangedCompletedWork,
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
