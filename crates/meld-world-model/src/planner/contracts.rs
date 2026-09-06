//! Public planner projection contracts.

use std::fmt;

use meld_lang::{world_state::GroundingError, WorldState};
use serde::{Deserialize, Serialize};

use crate::belief::{BeliefView, BranchScope, TheoryRevisionRef};
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::contracts::{
    BoundedTraversalRequest, TraversalCut, TraversalCutRequest, TraversalResult,
};
use crate::world_state::graph::{AnchorId, PerspectiveKey};

/// Static projection version for the first planner-facing world state slice.
pub const PLANNER_PROJECTION_VERSION: &str = "world_model.planner.v1";

/// Native owner position that may be required by one installed Planner policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerSourceKind {
    Graph,
    Belief,
    Directive,
    MaintainedCondition,
    CapabilityCatalog,
    CurationCatalog,
    StrategyPolicy,
    Causation,
    Regime,
}

/// Exact immutable native-owner revision supplied to Planner.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PlannerSourcePosition {
    pub kind: PlannerSourceKind,
    pub owner_id: String,
    pub source_id: String,
    pub revision_id: String,
    pub content_hash: String,
    pub scope_id: String,
    pub branch_id: String,
    pub perspective_id: String,
    pub authority_scope_id: String,
    pub invalidated_by_revision_id: Option<String>,
}

/// Deliberately limited installed policy for one Planner decision context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannerAssemblyPolicy {
    pub policy_revision_id: String,
    pub required_sources: Vec<PlannerSourceKind>,
    pub explicitly_not_required: Vec<PlannerSourceKind>,
}

/// Exact Agent and authority fence for one Planner assembly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannerDecisionContext {
    pub context_id: String,
    pub agent_id: String,
    pub goal_id: String,
    pub subject: DomainObjectRef,
    /// Explicit evidence target when it differs from the authorized runtime subject.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation_subject: Option<DomainObjectRef>,
    pub scope_id: String,
    pub branch_id: String,
    pub perspective_id: String,
    pub authority_scope_id: String,
    pub activation_generation: String,
    /// Exact admission epoch, absent only in legacy records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission_epoch: Option<String>,
}

impl PlannerDecisionContext {
    pub fn observation_subject(&self) -> &DomainObjectRef {
        self.observation_subject.as_ref().unwrap_or(&self.subject)
    }
}

/// Complete immutable input to Planner cut assembly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannerAssemblyRequest {
    pub context: PlannerDecisionContext,
    pub policy: PlannerAssemblyPolicy,
    pub traversal_cut: TraversalCut,
    pub traversal_request: BoundedTraversalRequest,
    pub traversal_result: TraversalResult,
    pub source_positions: Vec<PlannerSourcePosition>,
    pub view_input: PlannerProjectionInput,
}

/// Store-backed request that resolves the exact Graph and Belief positions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannerCurrentAssemblyRequest {
    pub context: PlannerDecisionContext,
    pub policy: PlannerAssemblyPolicy,
    pub traversal_cut_request: TraversalCutRequest,
    pub traversal_request: BoundedTraversalRequest,
    pub belief_key: crate::belief::BeliefKey,
    pub unanchored_belief: bool,
    pub source_positions: Vec<PlannerSourcePosition>,
}

/// Typed reason that Planner refused an incomplete or inconsistent request.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerRefusalGround {
    Missing {
        kind: PlannerSourceKind,
    },
    Unexpected {
        kind: PlannerSourceKind,
    },
    Duplicate {
        kind: PlannerSourceKind,
    },
    Invalidated {
        kind: PlannerSourceKind,
        revision_id: String,
    },
    ScopeMismatch {
        kind: PlannerSourceKind,
    },
    BranchMismatch {
        kind: PlannerSourceKind,
    },
    PerspectiveMismatch {
        kind: PlannerSourceKind,
    },
    Unauthorized {
        kind: PlannerSourceKind,
    },
    IncompleteTraversal,
    TraversalResultMismatch,
    InvalidInput {
        detail: String,
    },
    UnsupportedObservationSelection,
}

/// Exhaustive refusal returned before Strategy is invoked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannerRefusal {
    pub request_context_id: String,
    pub grounds: Vec<PlannerRefusalGround>,
}

/// Canonical immutable reasoning consistency root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannerCut {
    pub cut_id: String,
    pub context: PlannerDecisionContext,
    pub policy: PlannerAssemblyPolicy,
    pub traversal_cut: TraversalCut,
    pub traversal_request: BoundedTraversalRequest,
    pub traversal_result: TraversalResult,
    pub source_positions: Vec<PlannerSourcePosition>,
    pub world_model_view: WorldModelView,
}

/// Complete-or-refused Planner outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerAssemblyOutcome {
    Complete(Box<PlannerCut>),
    Refused(PlannerRefusal),
}

/// Decision context for one projection request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannerProjectionContext {
    pub subject: DomainObjectRef,
    pub perspective: PerspectiveKey,
    pub branch_scope: BranchScope,
    pub projection_version: String,
}

impl PlannerProjectionContext {
    /// Build the default first-slice context for one subject.
    pub fn first_slice(subject: DomainObjectRef) -> Self {
        Self {
            subject,
            perspective: PerspectiveKey::new("default", "default")
                .expect("static default perspective is valid"),
            branch_scope: BranchScope::main(),
            projection_version: PLANNER_PROJECTION_VERSION.to_string(),
        }
    }
}

/// Data-driven field naming rules for planner propositions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannerFieldProjectionConfig {
    pub confidence_field: String,
    pub stale_field_suffix: String,
    pub observation_needed_field_suffix: String,
    pub emit_accessible: bool,
}

impl PlannerFieldProjectionConfig {
    /// Build field rules from the planner-safe belief view metadata.
    pub fn from_belief_view(view: &BeliefView) -> Self {
        Self {
            confidence_field: view.planner_projection.confidence_field.clone(),
            ..Self::default()
        }
    }

    /// Validate configured field fragments before projection.
    pub fn validate(&self) -> Result<(), PlannerProjectionError> {
        validate_field_id(&self.confidence_field)?;
        validate_field_id(&self.stale_field_suffix)?;
        validate_field_id(&self.observation_needed_field_suffix)?;
        Ok(())
    }

    /// Derive the freshness field from a runtime dimension id.
    pub fn stale_dimension(&self, dimension_id: &str) -> Result<String, PlannerProjectionError> {
        derived_dimension(dimension_id, &self.stale_field_suffix)
    }

    /// Derive the observation-needed field from a runtime dimension id.
    pub fn observation_needed_dimension(
        &self,
        dimension_id: &str,
    ) -> Result<String, PlannerProjectionError> {
        derived_dimension(dimension_id, &self.observation_needed_field_suffix)
    }
}

impl Default for PlannerFieldProjectionConfig {
    fn default() -> Self {
        Self {
            confidence_field: "confidence".to_string(),
            stale_field_suffix: "stale".to_string(),
            observation_needed_field_suffix: "observation_needed".to_string(),
            emit_accessible: true,
        }
    }
}

/// Input to the pure planner projection function.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannerProjectionInput {
    pub context: PlannerProjectionContext,
    pub belief_view: Option<BeliefView>,
    pub graph_scope: Option<PlannerGraphScope>,
    pub field_config: PlannerFieldProjectionConfig,
}

/// Graph scope facts visible to the planner projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannerGraphScope {
    pub accessible: bool,
    pub anchor_ids: Vec<AnchorId>,
    pub source_fact_ids: Vec<String>,
}

/// Output envelope for a projected ground world state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldModelView {
    pub world_state: WorldState,
    pub projection_version: String,
    pub source_refs: Vec<PlannerSourceRef>,
    pub hydration_refs: PlannerHydrationRefs,
    pub warnings: Vec<PlannerProjectionWarning>,
    /// Installed theory revision cited by the projected belief view.
    ///
    /// Additive lineage field carried from the belief revision so planning
    /// records can answer which theory produced the consumed belief.
    #[serde(default)]
    pub theory_revision: Option<TheoryRevisionRef>,
}

/// Traceable origin of one projection input or rule.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlannerSourceRef {
    BeliefRevision { revision_id: String },
    Evidence { evidence_id: String },
    SourceFact { source_fact_id: String },
    GraphAnchor { anchor_id: AnchorId },
    ProjectionRule { rule_id: String },
}

/// Handles that let callers hydrate detailed lower-layer records later.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannerHydrationRefs {
    pub evidence_ids: Vec<String>,
    pub source_fact_ids: Vec<String>,
    pub graph_anchor_ids: Vec<AnchorId>,
    pub revision_ids: Vec<String>,
}

impl PlannerHydrationRefs {
    pub(crate) fn sort_and_dedup(&mut self) {
        sort_dedup(&mut self.evidence_ids);
        sort_dedup(&mut self.source_fact_ids);
        sort_dedup(&mut self.graph_anchor_ids);
        sort_dedup(&mut self.revision_ids);
    }
}

/// Non-fatal projection condition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlannerProjectionWarning {
    MissingBelief { subject: DomainObjectRef },
    MissingGraphScope { subject: DomainObjectRef },
    InvalidConfidence { dimension_id: String },
    InvalidProjectionField { field_id: String },
}

/// Error returned when planner projection cannot safely produce a world state.
#[derive(Debug)]
pub enum PlannerProjectionError {
    Storage(StorageError),
    SubjectMismatch {
        expected: Box<DomainObjectRef>,
        actual: Box<DomainObjectRef>,
    },
    PerspectiveMismatch {
        expected: Box<PerspectiveKey>,
        actual: Box<PerspectiveKey>,
    },
    BranchScopeMismatch {
        expected: Box<BranchScope>,
        actual: Box<BranchScope>,
    },
    InvalidProjectionField {
        field_id: String,
    },
    InvalidConfidence {
        dimension_id: String,
        confidence: f64,
    },
    Grounding(GroundingError),
    Serde(Box<serde_json::Error>),
}

impl fmt::Display for PlannerProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(err) => write!(formatter, "storage error: {err}"),
            Self::SubjectMismatch { expected, actual } => {
                write!(
                    formatter,
                    "subject mismatch: expected {}, got {}",
                    expected.index_key(),
                    actual.index_key()
                )
            }
            Self::PerspectiveMismatch { expected, actual } => {
                write!(
                    formatter,
                    "perspective mismatch: expected {}, got {}",
                    expected.index_key(),
                    actual.index_key()
                )
            }
            Self::BranchScopeMismatch { expected, actual } => {
                write!(
                    formatter,
                    "branch scope mismatch: expected {}, got {}",
                    expected.branch_id, actual.branch_id
                )
            }
            Self::InvalidProjectionField { field_id } => {
                write!(formatter, "invalid projection field: {field_id}")
            }
            Self::InvalidConfidence {
                dimension_id,
                confidence,
            } => write!(
                formatter,
                "invalid confidence for dimension {dimension_id}: {confidence}"
            ),
            Self::Grounding(err) => write!(
                formatter,
                "grounding error at index {}: {}",
                err.index, err.variable
            ),
            Self::Serde(err) => write!(formatter, "serialization error: {err}"),
        }
    }
}

impl std::error::Error for PlannerProjectionError {}

impl From<StorageError> for PlannerProjectionError {
    fn from(err: StorageError) -> Self {
        Self::Storage(err)
    }
}

impl From<GroundingError> for PlannerProjectionError {
    fn from(err: GroundingError) -> Self {
        Self::Grounding(err)
    }
}

impl From<serde_json::Error> for PlannerProjectionError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serde(Box::new(err))
    }
}

pub(crate) fn validate_field_id(field_id: &str) -> Result<(), PlannerProjectionError> {
    if field_id.trim().is_empty()
        || field_id.contains(char::is_whitespace)
        || field_id.contains("..")
    {
        return Err(PlannerProjectionError::InvalidProjectionField {
            field_id: field_id.to_string(),
        });
    }
    Ok(())
}

pub(crate) fn derived_dimension(
    dimension_id: &str,
    suffix: &str,
) -> Result<String, PlannerProjectionError> {
    validate_field_id(dimension_id)?;
    validate_field_id(suffix)?;
    Ok(format!("{dimension_id}.{suffix}"))
}

pub(crate) fn sort_dedup<T: Ord>(items: &mut Vec<T>) {
    items.sort();
    items.dedup();
}
