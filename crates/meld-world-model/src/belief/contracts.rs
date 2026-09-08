//! Public belief records and view contracts.
//!
//! This module defines the durable shape of belief state. It intentionally
//! stores runtime family names as data so a family such as `docs_freshness`
//! does not become a Rust subsystem.
//!
//! # Example
//!
//! ```rust
//! use meld_world_model::belief::{BeliefKey, BranchScope};
//! use meld_world_model::events::DomainObjectRef;
//! use meld_world_model::PerspectiveKey;
//!
//! let key = BeliefKey {
//!     subject: DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap(),
//!     dimension_id: "docs_freshness".to_string(),
//!     predicate_id: "confidence".to_string(),
//!     perspective: PerspectiveKey::new("default", "default").unwrap(),
//!     branch_scope: BranchScope::main(),
//!     evidence_policy_id: "default_policy".to_string(),
//! };
//!
//! key.validate().unwrap();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::belief::registry::TheoryRevisionRef;
use crate::error::StorageError;
use crate::events::{DomainObjectRef, EventRelation};
use crate::world_state::graph::PerspectiveKey;

/// Runtime-selected policy identity for evidence filtering and weighting.
pub type EvidencePolicyId = String;

/// Branch-local belief scope.
///
/// Belief keys carry branch scope explicitly so replay does not depend on
/// hidden process state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BranchScope {
    /// Stable branch identifier used in belief indexes.
    pub branch_id: String,
}

impl BranchScope {
    /// Build a validated branch scope.
    pub fn new(branch_id: impl Into<String>) -> Result<Self, StorageError> {
        let scope = Self {
            branch_id: branch_id.into(),
        };
        require_non_empty("branch scope", &scope.branch_id)?;
        Ok(scope)
    }

    /// Default branch scope used by the first belief slice.
    pub fn main() -> Self {
        Self {
            branch_id: "main".to_string(),
        }
    }
}

/// Stable identity for one assessed belief question.
///
/// The key names the subject, runtime dimension, predicate, perspective,
/// branch, and evidence policy. It is the unit used for revision heads,
/// active leases, and planner-facing views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeliefKey {
    /// Object whose state is being assessed.
    pub subject: DomainObjectRef,
    /// Runtime dimension id loaded from family configuration.
    pub dimension_id: String,
    /// Runtime predicate id loaded from family configuration.
    pub predicate_id: String,
    /// Perspective that owns this view over the shared graph substrate.
    pub perspective: PerspectiveKey,
    /// Branch scope for this belief view.
    pub branch_scope: BranchScope,
    /// Evidence policy used for assignment and assessment.
    pub evidence_policy_id: EvidencePolicyId,
}

impl BeliefKey {
    /// Validate fields needed for durable identity and replay.
    pub fn validate(&self) -> Result<(), StorageError> {
        self.subject.validate()?;
        require_non_empty("dimension id", &self.dimension_id)?;
        require_non_empty("predicate id", &self.predicate_id)?;
        self.perspective.validate()?;
        require_non_empty("branch scope", &self.branch_scope.branch_id)?;
        require_non_empty("evidence policy id", &self.evidence_policy_id)?;
        Ok(())
    }

    /// Deterministic storage key for sled indexes.
    pub fn index_key(&self) -> String {
        format!(
            "{}::{}::{}::{}::{}::{}",
            self.subject.index_key(),
            self.dimension_id,
            self.predicate_id,
            self.perspective.index_key(),
            self.branch_scope.branch_id,
            self.evidence_policy_id
        )
    }
}

/// Runtime-loaded family definition.
///
/// Family semantics live here as data. Core belief code reads generic ids,
/// schemas, mappings, and comparator settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeliefFamilyConfig {
    pub family_id: String,
    pub dimension_id: String,
    pub predicate_id: String,
    pub evidence_policy_id: EvidencePolicyId,
    pub evidence_schemas: Vec<EvidenceSchemaConfig>,
    pub source_mappings: Vec<EvidenceSourceMapping>,
    pub comparator: ComparatorConfig,
    pub default_prior: f64,
    pub planner_projection: PlannerProjectionConfig,
    pub config_version: String,
    /// Whether the family's dimension can be settled by observation.
    /// Declared by the owning family so the settlement transform never
    /// guesses; absent in older configs, defaulting to observational.
    #[serde(default)]
    pub observationality: DimensionObservationality,
    /// Whether a subject without curated evidence may receive a prior-only revision.
    // Preserve installed theory hashes. This wire name belongs to accepted family
    // revisions; runtime evidence admission no longer reads Graph anchors.
    #[serde(default, rename = "anchor_requirement")]
    pub initial_assessment: InitialAssessmentPolicy,
}

/// How a belief dimension can be settled.
///
/// The Strategy settlement transform consumes this declaration to decide
/// whether an unsettled question can become an observation obligation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DimensionObservationality {
    /// The dimension can be settled by direct observation of the subject.
    #[default]
    Observational,
    /// The dimension is derived from other evidence or beliefs and cannot
    /// be observed directly.
    Derived,
}

/// Evidence required before a family's first assessment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitialAssessmentPolicy {
    /// Wait for durable evidence admitted through Curation and ingestion.
    #[default]
    Required,
    /// Permit a prior-only revision until evidence arrives.
    #[serde(rename = "Unanchored")]
    PriorAllowed,
}

/// Evidence schema declared by runtime configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceSchemaConfig {
    pub schema_id: String,
    pub required: bool,
    pub role: EvidenceRole,
    pub reliability: f64,
    pub precision: f64,
}

/// Mapping from a source shape into one configured evidence schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceSourceMapping {
    pub mapping_id: String,
    pub source_kind: String,
    pub evidence_schema_id: String,
    pub subject_from: String,
    pub value_field: String,
    pub factor_id: String,
}

/// Comparator engine selection and factor configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparatorConfig {
    pub engine_id: String,
    pub engine_version: String,
    pub factors: Vec<ComparatorFactorConfig>,
    pub missing_evidence_uncertainty: f64,
    #[serde(default, skip_serializing_if = "ComparatorUpdatePolicy::is_default")]
    pub update_policy: ComparatorUpdatePolicy,
}

/// Whether evidence updates an accumulated estimate or replaces an observation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparatorUpdatePolicy {
    /// Blend the prior and this window's weighted evidence equally.
    #[default]
    BlendPrior,
    /// Use only the latest source position for each schema, without prior weight.
    LatestObservation,
}

impl ComparatorUpdatePolicy {
    fn is_default(&self) -> bool {
        *self == Self::BlendPrior
    }
}

/// One weighted factor consumed by the generic Bayesian comparator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparatorFactorConfig {
    pub factor_id: String,
    pub evidence_schema_id: String,
    pub weight: f64,
    pub polarity: EvidencePolarity,
}

/// Direction of a factor contribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidencePolarity {
    Supports,
    Contradicts,
}

/// Planner-facing projection fields declared by runtime configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannerProjectionConfig {
    pub confidence_field: String,
    pub threshold: f64,
    pub posterior_meaning: String,
    #[serde(default, skip_serializing_if = "ConfidenceProjection::is_default")]
    pub confidence_projection: ConfidenceProjection,
}

/// Meaning of the confidence proposition exposed to Planner.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfidenceProjection {
    #[default]
    Complement,
    Probability,
}

impl ConfidenceProjection {
    fn is_default(&self) -> bool {
        *self == Self::Complement
    }
}

/// Normalized belief input derived from graph state or promoted facts.
///
/// Evidence is append-only. Assignment may attach the same evidence to one or
/// more belief keys, but the evidence record itself remains a durable audit
/// record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// Exact owner publication record interpreted into this evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publication_record_id: Option<String>,
    pub evidence_id: String,
    pub candidate_key: BeliefKey,
    pub source_fact_ids: Vec<String>,
    pub graph_anchor_ids: Vec<String>,
    pub source_cursor_start: u64,
    pub source_cursor_end: u64,
    pub role: EvidenceRole,
    pub evidence_schema_id: String,
    pub typed_value: EvidenceValue,
    pub reliability: f64,
    pub precision: f64,
    #[serde(default)]
    pub reference_time: Option<String>,
    #[serde(default)]
    pub transaction_seq: u64,
    #[serde(default)]
    pub content_hash: Option<String>,
    /// Exact outcome-mapping revision that interpreted the source record.
    #[serde(default)]
    pub outcome_mapping_revision: Option<TheoryRevisionRef>,
    pub provenance: BeliefProvenanceSummary,
}

/// Generic promoted record accepted by evidence normalization.
///
/// The fields are runtime data rather than family-specific Rust types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotedEvidenceRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publication_record_id: Option<String>,
    pub source_kind: String,
    pub source_id: String,
    pub subject: DomainObjectRef,
    pub source_fact_ids: Vec<String>,
    pub graph_anchor_ids: Vec<String>,
    pub objects: Vec<DomainObjectRef>,
    pub relations: Vec<EventRelation>,
    pub source_cursor_start: u64,
    pub source_cursor_end: u64,
    pub reference_time: Option<String>,
    pub transaction_seq: u64,
    pub content_hash: Option<String>,
    /// Exact outcome-mapping revision that promoted this record.
    #[serde(default)]
    pub outcome_mapping_revision: Option<TheoryRevisionRef>,
    pub fields: BTreeMap<String, EvidenceValue>,
}

/// Role an evidence item plays in assessment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceRole {
    Support,
    Contradiction,
    Context,
    Calibration,
    Supersession,
}

/// Runtime-typed value carried by normalized evidence.
///
/// The variants are intentionally generic. Family-specific values belong in
/// runtime schemas and payload data, not in Rust enum variants.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EvidenceValue {
    Scalar(f64),
    Text(String),
    Map(BTreeMap<String, String>),
}

/// Durable edge from one evidence item to one belief key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceAssignment {
    pub assignment_id: String,
    pub evidence_id: String,
    pub belief_key: BeliefKey,
    pub role: EvidenceRole,
    pub source_cursor_start: u64,
    pub source_cursor_end: u64,
}

/// Audit record for a candidate that could not become evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRejection {
    pub rejection_id: String,
    pub source_id: String,
    pub reason: String,
    pub source_cursor_start: u64,
    pub source_cursor_end: u64,
    /// Exact outcome-mapping revision involved in rejection.
    #[serde(default)]
    pub outcome_mapping_revision: Option<Box<TheoryRevisionRef>>,
}

/// Compact posterior value exposed by a committed revision and view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PosteriorSummary {
    pub probability: f64,
    pub meaning: String,
}

/// Projection value intended for planner consumption.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannerProjectionSummary {
    pub confidence_field: String,
    pub confidence: f64,
    pub threshold: f64,
}

/// Append-only settlement for one bounded evidence window.
///
/// Revisions are never edited to supersede old belief state. The current head
/// moves to a newer revision while old revisions remain queryable for replay
/// and provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeliefRevision {
    pub revision_id: String,
    pub belief_key: BeliefKey,
    pub prior_revision_id: Option<String>,
    pub comparator_engine_id: String,
    pub comparator_engine_version: String,
    pub config_snapshot_hash: String,
    pub evidence_ids: Vec<String>,
    pub supporting_evidence_ids: Vec<String>,
    pub contradicted_evidence_ids: Vec<String>,
    pub source_cursor_start: u64,
    pub source_cursor_end: u64,
    pub posterior: PosteriorSummary,
    pub planner_projection: PlannerProjectionSummary,
    pub uncertainty: f64,
    pub precision: f64,
    pub freshness: FreshnessState,
    pub contradiction: ContradictionState,
    pub status: BeliefStatus,
    pub observation: Option<ObservationOpportunity>,
    pub provenance: BeliefProvenanceSummary,
    /// Installed theory revision that produced this revision.
    ///
    /// Additive lineage field: revisions committed before theory registries
    /// existed deserialize to `None` and stay loadable.
    #[serde(default)]
    pub theory_revision: Option<TheoryRevisionRef>,
}

/// High-level settlement state for a revision or view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeliefStatus {
    Settled,
    Stale,
    NeedsObservation,
    NeedsAssessment,
    AssessmentPending,
    Invalid,
}

/// Freshness metadata for the current belief view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FreshnessState {
    pub stale: bool,
    #[serde(default)]
    pub reasons: Vec<FreshnessReason>,
    pub high_water_seq: u64,
}

/// Typed cause for stale belief state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FreshnessReason {
    NewerEvidence,
    SupersededAnchor,
    ConfigSnapshotChanged,
    EvidencePolicyChanged,
}

/// Conflict metadata that keeps support and counterevidence separate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContradictionState {
    pub contradicted: bool,
    #[serde(default)]
    pub reasons: Vec<ContradictionReason>,
    pub supporting_evidence_ids: Vec<String>,
    pub contradicted_evidence_ids: Vec<String>,
}

/// Typed cause for contradiction or weak settlement state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContradictionReason {
    Counterevidence,
    WeakCoverage,
    Supersession,
    Invalidation,
    MissingComparatorState,
}

/// World-model output that names useful missing evidence.
///
/// Observation opportunities are not execution commands. The agent or planner
/// may later decide whether to act on them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationOpportunity {
    pub opportunity_id: String,
    pub belief_key: BeliefKey,
    pub target_evidence_schema_id: String,
    pub reason: ObservationReason,
    pub detail: String,
    pub source_revision_id: Option<String>,
    pub open: bool,
}

/// Typed reason for a future observation opportunity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservationReason {
    MissingRequiredEvidence,
    StaleEvidence,
    UnresolvedContradiction,
    MissingComparator,
}

/// Durable worker lease for assessing one belief key.
///
/// Leases serialize revision commits by key and give recovery enough state to
/// abandon expired work without relying on worker memory.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssessmentLease {
    pub lease_id: String,
    pub belief_key: BeliefKey,
    pub epoch: u64,
    pub owner_id: String,
    pub input_cursor_start: u64,
    pub input_cursor_end: u64,
    pub started_at_seq: u64,
    pub expires_at_seq: u64,
    pub comparator_engine_id: String,
    pub config_snapshot_hash: String,
    pub status: LeaseStatus,
}

/// Lifecycle state for assessment work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeaseStatus {
    Queued,
    Leased,
    Completed,
    Expired,
    Abandoned,
}

/// Durable dirty-key state used for recovery and storm coalescing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirtyKeyState {
    pub belief_key: BeliefKey,
    pub dirty_since_seq: u64,
    pub latest_seq: u64,
    pub active_lease_id: Option<String>,
    pub reason: DirtyReason,
}

/// Cause for a dirty belief key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirtyReason {
    NewEvidence,
    LeaseExpired,
    ActiveLeaseCoalesced,
    ConfigChanged,
    PolicyChanged,
}

/// Planner-safe projection of current belief state.
///
/// Views expose posterior, confidence, freshness, conflict, observation, and
/// provenance summaries. They deliberately omit raw evidence payloads,
/// comparator drafts, reducer internals, and active lease state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeliefView {
    pub view_id: String,
    pub key: BeliefKey,
    pub current_revision_id: Option<String>,
    pub status: BeliefStatus,
    pub posterior: PosteriorSummary,
    pub planner_projection: PlannerProjectionSummary,
    pub uncertainty: f64,
    pub precision: f64,
    pub freshness: FreshnessState,
    pub contradiction: ContradictionState,
    pub observation: Option<ObservationOpportunity>,
    pub assessment_state: String,
    pub advisory_posture: String,
    pub provenance: BeliefProvenanceSummary,
    pub hydration: HydrationRefs,
    /// Installed theory revision that produced the projected revision.
    ///
    /// Additive lineage field mirroring [`BeliefRevision::theory_revision`];
    /// views stored before theory registries existed deserialize to `None`.
    #[serde(default)]
    pub theory_revision: Option<TheoryRevisionRef>,
}

/// Compact provenance and hydration summary for belief records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeliefProvenanceSummary {
    pub evidence_ids: Vec<String>,
    pub source_fact_ids: Vec<String>,
    pub graph_anchor_ids: Vec<String>,
    pub objects: Vec<DomainObjectRef>,
    pub relations: Vec<EventRelation>,
    pub revision_ids: Vec<String>,
}

impl BeliefProvenanceSummary {
    /// Empty provenance value used when no evidence has been accepted.
    pub fn empty() -> Self {
        Self {
            evidence_ids: Vec::new(),
            source_fact_ids: Vec::new(),
            graph_anchor_ids: Vec::new(),
            objects: Vec::new(),
            relations: Vec::new(),
            revision_ids: Vec::new(),
        }
    }
}

/// Handles a caller can use to hydrate full records after planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HydrationRefs {
    pub evidence_ids: Vec<String>,
    pub source_fact_ids: Vec<String>,
    pub graph_anchor_ids: Vec<String>,
    pub revision_id: Option<String>,
}

/// Validate a finite scalar probability.
pub fn validate_probability(label: &str, value: f64) -> Result<(), StorageError> {
    if !(0.0..=1.0).contains(&value) || !value.is_finite() {
        return Err(StorageError::InvalidPath(format!(
            "{label} must be a finite probability"
        )));
    }
    Ok(())
}

/// Validate a non-empty runtime id or storage key component.
pub fn require_non_empty(label: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        return Err(StorageError::InvalidPath(format!(
            "{label} must be non-empty"
        )));
    }
    Ok(())
}
