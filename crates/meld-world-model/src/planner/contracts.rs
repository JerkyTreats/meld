//! Public planner projection contracts.

use std::fmt;

use meld_lang::{world_state::GroundingError, WorldState};
use serde::{Deserialize, Serialize};

use crate::belief::{BeliefView, BranchScope};
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::{AnchorId, PerspectiveKey};

/// Static projection version for the first planner-facing world state slice.
pub const PLANNER_PROJECTION_VERSION: &str = "world_model.planner.v1";

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
pub struct PlannerProjectionOutput {
    pub world_state: WorldState,
    pub projection_version: String,
    pub source_refs: Vec<PlannerSourceRef>,
    pub hydration_refs: PlannerHydrationRefs,
    pub warnings: Vec<PlannerProjectionWarning>,
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
