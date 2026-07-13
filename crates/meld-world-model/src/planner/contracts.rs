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

const PROJECTION_REQUEST_HASH_DOMAIN: &[u8] = b"meld.planner-projection-request.v1";
const PROJECTION_FRAME_HASH_DOMAIN: &[u8] = b"meld.planner-projection-frame.v1";
const PLANNER_WORLD_STATE_HASH_DOMAIN: &[u8] = b"meld.planner-world-state.v1";

/// Immutable attested belief boundary frozen by one projection request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerAttestedBeliefSnapshot {
    /// Belief-owned readiness attestation identity.
    pub attestation_id: String,
    /// Exact append-only belief revision identity.
    pub revision_id: String,
    /// Canonical digest of the complete belief revision.
    pub revision_hash: String,
    /// Exact planner-safe view identity.
    pub view_id: String,
    /// Canonical digest of the complete planner-safe view.
    pub view_hash: String,
    /// Inclusive lower source sequence consumed by the revision.
    pub source_cursor_start: u64,
    /// Inclusive upper source sequence consumed by the revision.
    pub source_cursor_end: u64,
}

impl PlannerAttestedBeliefSnapshot {
    /// Build the planner boundary from a belief-owned readiness attestation.
    pub fn from_attestation(attestation: &crate::belief::BeliefReadinessAttestation) -> Self {
        Self {
            attestation_id: attestation.attestation_id.clone(),
            revision_id: attestation.belief_revision_id.clone(),
            revision_hash: attestation.belief_revision_hash.clone(),
            view_id: attestation.belief_view_id.clone(),
            view_hash: attestation.belief_view_hash.clone(),
            source_cursor_start: attestation.source_cursor_start,
            source_cursor_end: attestation.source_cursor_end,
        }
    }

    /// Validate exact identities, content digests, and the source boundary.
    pub fn validate(&self) -> Result<(), PlannerProjectionContractError> {
        for (field, value) in [
            ("planner attestation id", self.attestation_id.as_str()),
            ("planner belief revision id", self.revision_id.as_str()),
            ("planner belief revision hash", self.revision_hash.as_str()),
            ("planner belief view id", self.view_id.as_str()),
            ("planner belief view hash", self.view_hash.as_str()),
        ] {
            projection_required(field, value)?;
        }
        if self.source_cursor_start == 0 || self.source_cursor_end < self.source_cursor_start {
            return Err(PlannerProjectionContractError::SequenceRegression);
        }
        Ok(())
    }
}

/// Durable world-model request for one planner-facing projection frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerProjectionRequest {
    /// Deterministic world-model request identity.
    pub request_id: String,
    /// Opaque hash of the complete execution-owned source request.
    pub source_request_hash: String,
    /// Agent perspective requesting projection.
    pub agent_id: String,
    /// Subject projected into planner world state.
    pub subject: DomainObjectRef,
    /// Full perspective identity rather than one unscoped id.
    pub perspective: PerspectiveKey,
    /// Branch-local world-model scope.
    pub branch_scope: BranchScope,
    /// Canonically sorted requested dimensions.
    pub requested_dimensions: Vec<String>,
    /// Canonically ordered extra planner preconditions.
    pub required_preconditions: Vec<meld_lang::Proposition>,
    /// Exact belief snapshot for attested hydration projections.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attested_belief: Option<PlannerAttestedBeliefSnapshot>,
}

impl PlannerProjectionRequest {
    /// Canonicalize and identify a complete projection request.
    #[allow(clippy::too_many_arguments)]
    pub fn identified(
        source_request_hash: impl Into<String>,
        agent_id: impl Into<String>,
        subject: DomainObjectRef,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
        requested_dimensions: Vec<String>,
        required_preconditions: Vec<meld_lang::Proposition>,
    ) -> Result<Self, PlannerProjectionContractError> {
        Self::identified_inner(
            source_request_hash,
            agent_id,
            subject,
            perspective,
            branch_scope,
            requested_dimensions,
            required_preconditions,
            None,
        )
    }

    /// Canonicalize and identify a projection over one exact attested view.
    #[allow(clippy::too_many_arguments)]
    pub fn identified_attested(
        source_request_hash: impl Into<String>,
        agent_id: impl Into<String>,
        subject: DomainObjectRef,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
        requested_dimensions: Vec<String>,
        required_preconditions: Vec<meld_lang::Proposition>,
        attested_belief: PlannerAttestedBeliefSnapshot,
    ) -> Result<Self, PlannerProjectionContractError> {
        Self::identified_inner(
            source_request_hash,
            agent_id,
            subject,
            perspective,
            branch_scope,
            requested_dimensions,
            required_preconditions,
            Some(attested_belief),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn identified_inner(
        source_request_hash: impl Into<String>,
        agent_id: impl Into<String>,
        subject: DomainObjectRef,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
        mut requested_dimensions: Vec<String>,
        mut required_preconditions: Vec<meld_lang::Proposition>,
        attested_belief: Option<PlannerAttestedBeliefSnapshot>,
    ) -> Result<Self, PlannerProjectionContractError> {
        requested_dimensions.sort();
        requested_dimensions.dedup();
        canonicalize_propositions(&mut required_preconditions)?;
        let mut request = Self {
            request_id: String::new(),
            source_request_hash: source_request_hash.into(),
            agent_id: agent_id.into(),
            subject,
            perspective,
            branch_scope,
            requested_dimensions,
            required_preconditions,
            attested_belief,
        };
        request.request_id = request.derive_id()?;
        request.validate()?;
        Ok(request)
    }

    /// Validate scope, canonical ordering, and deterministic identity.
    pub fn validate(&self) -> Result<(), PlannerProjectionContractError> {
        projection_required("source request hash", &self.source_request_hash)?;
        projection_required("projection agent id", &self.agent_id)?;
        self.subject
            .validate()
            .map_err(|error| PlannerProjectionContractError::Invalid(error.to_string()))?;
        self.perspective
            .validate()
            .map_err(|error| PlannerProjectionContractError::Invalid(error.to_string()))?;
        projection_required("projection branch id", &self.branch_scope.branch_id)?;
        if let Some(snapshot) = self.attested_belief.as_ref() {
            snapshot.validate()?;
        }
        if self.requested_dimensions.is_empty()
            || self
                .requested_dimensions
                .iter()
                .any(|dimension| dimension.trim().is_empty())
        {
            return Err(PlannerProjectionContractError::Invalid(
                "projection request dimensions must be non-empty".to_string(),
            ));
        }
        let mut dimensions = self.requested_dimensions.clone();
        dimensions.sort();
        dimensions.dedup();
        let mut preconditions = self.required_preconditions.clone();
        canonicalize_propositions(&mut preconditions)?;
        if dimensions != self.requested_dimensions || preconditions != self.required_preconditions {
            return Err(PlannerProjectionContractError::NonCanonicalRequest);
        }
        if self.request_id != self.derive_id()? {
            return Err(PlannerProjectionContractError::IdentityMismatch(
                "projection request id".to_string(),
            ));
        }
        Ok(())
    }

    fn derive_id(&self) -> Result<String, PlannerProjectionContractError> {
        #[derive(Serialize)]
        struct LegacyIdentity<'a> {
            source_request_hash: &'a str,
            agent_id: &'a str,
            subject: &'a DomainObjectRef,
            perspective: &'a PerspectiveKey,
            branch_scope: &'a BranchScope,
            requested_dimensions: &'a [String],
            required_preconditions: &'a [meld_lang::Proposition],
        }
        let legacy = LegacyIdentity {
            source_request_hash: &self.source_request_hash,
            agent_id: &self.agent_id,
            subject: &self.subject,
            perspective: &self.perspective,
            branch_scope: &self.branch_scope,
            requested_dimensions: &self.requested_dimensions,
            required_preconditions: &self.required_preconditions,
        };
        let Some(attested_belief) = self.attested_belief.as_ref() else {
            return planner_contract_hash(PROJECTION_REQUEST_HASH_DOMAIN, &legacy);
        };
        #[derive(Serialize)]
        struct AttestedIdentity<'a> {
            #[serde(flatten)]
            legacy: LegacyIdentity<'a>,
            attested_belief: &'a PlannerAttestedBeliefSnapshot,
        }
        planner_contract_hash(
            PROJECTION_REQUEST_HASH_DOMAIN,
            &AttestedIdentity {
                legacy,
                attested_belief,
            },
        )
    }
}

/// Durable lifecycle of one planner projection request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerProjectionRequestStatus {
    /// Request is awaiting bounded projection work.
    Pending,
    /// One durable frame completed the request.
    Completed,
    /// Projection failed with a bounded diagnostic.
    Failed,
}

/// Durable projection request plus its terminal products.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerProjectionRequestRecord {
    /// Complete immutable request.
    pub request: PlannerProjectionRequest,
    /// Current durable request state.
    pub status: PlannerProjectionRequestStatus,
    /// Completed frame identity when successful.
    pub frame_id: Option<String>,
    /// Bounded failure detail when failed.
    pub last_error: Option<String>,
    /// Sequence assigned on first persistence.
    pub created_at_seq: u64,
    /// Last durable transition sequence.
    pub updated_at_seq: u64,
}

/// Crate-private non-visible hydration request intent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreparedPlannerProjectionRequest {
    pub(crate) request: PlannerProjectionRequest,
    pub(crate) created_at_seq: u64,
    pub(crate) owner_fence_hash: String,
}

impl PlannerProjectionRequestRecord {
    /// Validate request identity, monotonic sequence, and status products.
    pub fn validate(&self) -> Result<(), PlannerProjectionContractError> {
        self.request.validate()?;
        if self.created_at_seq == 0 || self.updated_at_seq < self.created_at_seq {
            return Err(PlannerProjectionContractError::SequenceRegression);
        }
        let products_valid = match self.status {
            PlannerProjectionRequestStatus::Pending => {
                self.frame_id.is_none() && self.last_error.is_none()
            }
            PlannerProjectionRequestStatus::Completed => {
                self.frame_id
                    .as_deref()
                    .is_some_and(|value| !value.is_empty())
                    && self.last_error.is_none()
            }
            PlannerProjectionRequestStatus::Failed => {
                self.frame_id.is_none()
                    && self
                        .last_error
                        .as_deref()
                        .is_some_and(|value| !value.is_empty())
            }
        };
        if !products_valid {
            return Err(PlannerProjectionContractError::InvalidStatusProducts);
        }
        Ok(())
    }
}

/// Deterministic identity inputs for one completed world-model planner frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerProjectionFrameIdentity {
    /// Frame id derived from every following field.
    pub frame_id: String,
    /// World-model request completed by this frame.
    pub request_id: String,
    /// Opaque execution source request retained for adapter validation.
    pub source_request_hash: String,
    /// Projection schema or algorithm version.
    pub projection_version: String,
    /// Canonical digest of the complete projection output.
    pub projection_hash: String,
    /// Canonical digest of the projected world state for execution verification.
    pub world_state_hash: String,
    /// Canonically sorted source provenance.
    pub source_refs: Vec<PlannerSourceRef>,
}

impl PlannerProjectionFrameIdentity {
    /// Identify one completed output for one exact request.
    pub fn identified(
        request: &PlannerProjectionRequest,
        output: &PlannerProjectionOutput,
    ) -> Result<Self, PlannerProjectionContractError> {
        request.validate()?;
        let mut source_refs = output.source_refs.clone();
        source_refs.sort();
        source_refs.dedup();
        if source_refs != output.source_refs {
            return Err(PlannerProjectionContractError::NonCanonicalFrame);
        }
        let projection_hash = planner_contract_hash(b"meld.planner-projection-output.v1", output)?;
        let world_state_hash = hash_planner_world_state(&output.world_state)?;
        let mut identity = Self {
            frame_id: String::new(),
            request_id: request.request_id.clone(),
            source_request_hash: request.source_request_hash.clone(),
            projection_version: output.projection_version.clone(),
            projection_hash,
            world_state_hash,
            source_refs,
        };
        identity.frame_id = identity.derive_id()?;
        identity.validate()?;
        Ok(identity)
    }

    /// Validate canonical provenance and deterministic frame identity.
    pub fn validate(&self) -> Result<(), PlannerProjectionContractError> {
        for (field, value) in [
            ("frame request id", self.request_id.as_str()),
            (
                "frame source request hash",
                self.source_request_hash.as_str(),
            ),
            ("frame projection version", self.projection_version.as_str()),
            ("frame projection hash", self.projection_hash.as_str()),
            ("frame world state hash", self.world_state_hash.as_str()),
        ] {
            projection_required(field, value)?;
        }
        let mut refs = self.source_refs.clone();
        refs.sort();
        refs.dedup();
        if refs != self.source_refs {
            return Err(PlannerProjectionContractError::NonCanonicalFrame);
        }
        if self.frame_id != self.derive_id()? {
            return Err(PlannerProjectionContractError::IdentityMismatch(
                "planner frame id".to_string(),
            ));
        }
        Ok(())
    }

    fn derive_id(&self) -> Result<String, PlannerProjectionContractError> {
        #[derive(Serialize)]
        struct Identity<'a> {
            request_id: &'a str,
            source_request_hash: &'a str,
            projection_version: &'a str,
            projection_hash: &'a str,
            world_state_hash: &'a str,
            source_refs: &'a [PlannerSourceRef],
        }
        planner_contract_hash(
            PROJECTION_FRAME_HASH_DOMAIN,
            &Identity {
                request_id: &self.request_id,
                source_request_hash: &self.source_request_hash,
                projection_version: &self.projection_version,
                projection_hash: &self.projection_hash,
                world_state_hash: &self.world_state_hash,
                source_refs: &self.source_refs,
            },
        )
    }
}

/// Durable planner frame owned by the world-model projection domain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerProjectionFrame {
    /// Deterministic identity and cross-domain correlation fields.
    pub identity: PlannerProjectionFrameIdentity,
    /// Complete projected world state and provenance.
    pub output: PlannerProjectionOutput,
    /// Durable completion sequence.
    pub completed_at_seq: u64,
}

impl PlannerProjectionFrame {
    /// Validate output digest, version, provenance, and frame identity.
    pub fn validate(&self) -> Result<(), PlannerProjectionContractError> {
        self.identity.validate()?;
        if self.completed_at_seq == 0 {
            return Err(PlannerProjectionContractError::SequenceRegression);
        }
        if self.output.projection_version != self.identity.projection_version
            || self.output.source_refs != self.identity.source_refs
            || hash_planner_world_state(&self.output.world_state)? != self.identity.world_state_hash
            || planner_contract_hash(b"meld.planner-projection-output.v1", &self.output)?
                != self.identity.projection_hash
        {
            return Err(PlannerProjectionContractError::FrameOutputMismatch);
        }
        Ok(())
    }
}

/// Hash one projected world state for cross-domain execution verification.
///
/// Execution mirrors this algorithm over serde JSON and carries the
/// world-model-owned frame id unchanged.
pub fn hash_planner_world_state(
    world_state: &WorldState,
) -> Result<String, PlannerProjectionContractError> {
    planner_contract_hash(PLANNER_WORLD_STATE_HASH_DOMAIN, world_state)
}

/// Invalid durable planner projection contract.
#[derive(Debug, thiserror::Error)]
pub enum PlannerProjectionContractError {
    /// Required identity or scope content is absent.
    #[error("invalid planner projection contract: {0}")]
    Invalid(String),
    /// Request fields were not stored in canonical order.
    #[error("planner projection request is not canonical")]
    NonCanonicalRequest,
    /// Frame provenance was not stored in canonical order.
    #[error("planner projection frame is not canonical")]
    NonCanonicalFrame,
    /// A deterministic id diverged from its fields.
    #[error("planner projection identity mismatch for {0}")]
    IdentityMismatch(String),
    /// A record sequence moved backwards.
    #[error("planner projection record sequence would regress")]
    SequenceRegression,
    /// Request status does not match its frame or failure products.
    #[error("planner projection request status products are inconsistent")]
    InvalidStatusProducts,
    /// Durable frame content diverged from its identity.
    #[error("planner projection frame output does not match its identity")]
    FrameOutputMismatch,
    /// A deterministic identity projection could not be encoded.
    #[error("planner projection identity encoding failed: {0}")]
    Encoding(String),
}

fn projection_required(field: &str, value: &str) -> Result<(), PlannerProjectionContractError> {
    if value.trim().is_empty() {
        return Err(PlannerProjectionContractError::Invalid(format!(
            "{field} must be non-empty"
        )));
    }
    Ok(())
}

fn canonicalize_propositions(
    propositions: &mut Vec<meld_lang::Proposition>,
) -> Result<(), PlannerProjectionContractError> {
    let mut keyed = propositions
        .drain(..)
        .map(|proposition| {
            serde_json::to_string(&proposition)
                .map(|key| (key, proposition))
                .map_err(|error| PlannerProjectionContractError::Encoding(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    keyed.dedup_by(|left, right| left.0 == right.0);
    propositions.extend(keyed.into_iter().map(|(_, proposition)| proposition));
    Ok(())
}

fn planner_contract_hash(
    domain: &[u8],
    value: &impl Serialize,
) -> Result<String, PlannerProjectionContractError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| PlannerProjectionContractError::Encoding(error.to_string()))?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(&encoded);
    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod durable_contract_tests {
    use super::*;

    fn request() -> PlannerProjectionRequest {
        PlannerProjectionRequest::identified(
            "execution-request-hash",
            "agent-docs",
            DomainObjectRef::new("workspace_fs", "node", "readme").unwrap(),
            PerspectiveKey::new("agent", "docs").unwrap(),
            BranchScope::main(),
            vec!["freshness".to_string(), "docs".to_string()],
            Vec::new(),
        )
        .unwrap()
    }

    fn output() -> PlannerProjectionOutput {
        PlannerProjectionOutput {
            world_state: WorldState::new(Vec::new()).unwrap(),
            projection_version: PLANNER_PROJECTION_VERSION.to_string(),
            source_refs: vec![PlannerSourceRef::ProjectionRule {
                rule_id: "readiness".to_string(),
            }],
            hydration_refs: PlannerHydrationRefs::default(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn durable_projection_request_and_frame_bind_complete_identity() {
        let request = request();
        assert_eq!(request.requested_dimensions, vec!["docs", "freshness"]);
        let identity = PlannerProjectionFrameIdentity::identified(&request, &output()).unwrap();
        let frame = PlannerProjectionFrame {
            identity: identity.clone(),
            output: output(),
            completed_at_seq: 7,
        };

        assert!(frame.validate().is_ok());
        assert_eq!(identity.request_id, request.request_id);
        assert_eq!(identity.source_request_hash, "execution-request-hash");
        let encoded = serde_json::to_vec(&frame).unwrap();
        let decoded: PlannerProjectionFrame = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn durable_projection_frame_rejects_content_drift_under_one_id() {
        let request = request();
        let output = output();
        let identity = PlannerProjectionFrameIdentity::identified(&request, &output).unwrap();
        let mut frame = PlannerProjectionFrame {
            identity,
            output,
            completed_at_seq: 7,
        };
        frame
            .output
            .warnings
            .push(PlannerProjectionWarning::MissingGraphScope {
                subject: request.subject,
            });

        assert!(matches!(
            frame.validate(),
            Err(PlannerProjectionContractError::FrameOutputMismatch)
        ));
    }

    #[test]
    fn projection_request_status_requires_exact_terminal_products() {
        let completed = PlannerProjectionRequestRecord {
            request: request(),
            status: PlannerProjectionRequestStatus::Completed,
            frame_id: Some("frame-1".to_string()),
            last_error: None,
            created_at_seq: 1,
            updated_at_seq: 2,
        };
        assert!(completed.validate().is_ok());

        let mut invalid = completed;
        invalid.last_error = Some("failed".to_string());
        assert!(matches!(
            invalid.validate(),
            Err(PlannerProjectionContractError::InvalidStatusProducts)
        ));
    }
}
