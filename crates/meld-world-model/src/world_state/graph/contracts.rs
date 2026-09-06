//! Public graph traversal contracts.
//!
//! These records describe facts extracted from the event ledger and anchors that
//! identify the current object for a subject under a perspective. Anchors are
//! generic graph contracts, not belief or planner decisions.
//!
//! # Example
//!
//! ```rust
//! use meld_world_model::PerspectiveKey;
//!
//! let key = PerspectiveKey::new("frame_type", "analysis").unwrap();
//! assert_eq!(key.index_key(), "frame_type::analysis");
//! ```

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::StorageError;
use crate::events::{DomainObjectRef, EventRecordRef, EventRelation, LedgerCursor};

/// Event type for one producer-owned, exact owner publication.
pub const OWNER_PUBLICATION_EVENT_TYPE: &str = "world_state.owner_publication.v1";

/// Durable anchor identifier.
pub type AnchorId = String;
/// Durable traversal fact identifier.
pub type TraversalFactId = String;
/// Durable provenance identifier.
pub type ProvenanceId = String;

/// Exact owner-local source scope covered by one publication.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OwnerPublicationScope {
    pub scope_id: String,
    pub branch_id: Option<String>,
    pub perspective_id: Option<String>,
    pub valid_at: Option<String>,
}

impl OwnerPublicationScope {
    /// Validate durable scope components.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("owner publication scope_id", &self.scope_id)?;
        for value in [
            self.branch_id.as_deref(),
            self.perspective_id.as_deref(),
            self.valid_at.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            require_non_empty("owner publication scope qualifier", value)?;
        }
        Ok(())
    }
}

/// Owner-shaped lifecycle state for one address publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerPublicationState {
    Observed,
    Withdrawn,
    Archived,
}

/// Stable route back to the semantic owner's material.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HydrationReference {
    pub owner_id: String,
    pub product_kind: String,
    pub product_id: String,
    pub revision_id: String,
    pub role: String,
}

impl HydrationReference {
    fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("hydration owner_id", &self.owner_id)?;
        require_non_empty("hydration product_kind", &self.product_kind)?;
        require_non_empty("hydration product_id", &self.product_id)?;
        require_non_empty("hydration revision_id", &self.revision_id)?;
        require_non_empty("hydration role", &self.role)
    }
}

/// One owner-qualified statement that an address is present in a revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerObjectPublication {
    pub publication_id: String,
    pub object_ref: DomainObjectRef,
    pub state: OwnerPublicationState,
    pub source_product_ref: String,
    pub hydration: HydrationReference,
    pub provenance_refs: Vec<String>,
    pub qualifications: BTreeMap<String, String>,
}

impl OwnerObjectPublication {
    fn normalize(&mut self) {
        self.provenance_refs.sort();
        self.provenance_refs.dedup();
    }

    fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("object publication_id", &self.publication_id)?;
        self.object_ref.validate()?;
        require_non_empty("object source_product_ref", &self.source_product_ref)?;
        self.hydration.validate()
    }
}

/// One independently identified owner relation occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerRelationOccurrence {
    pub occurrence_id: String,
    pub relation_type: String,
    pub src: DomainObjectRef,
    pub dst: DomainObjectRef,
    pub source_product_ref: String,
    pub hydration: HydrationReference,
    pub qualifications: BTreeMap<String, String>,
    pub provenance_refs: Vec<String>,
}

impl OwnerRelationOccurrence {
    fn normalize(&mut self) {
        self.provenance_refs.sort();
        self.provenance_refs.dedup();
    }

    fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("relation occurrence_id", &self.occurrence_id)?;
        require_non_empty("relation relation_type", &self.relation_type)?;
        self.src.validate()?;
        self.dst.validate()?;
        require_non_empty("relation source_product_ref", &self.source_product_ref)?;
        self.hydration.validate()
    }

    /// Neutral Event relation hint for this occurrence.
    pub fn event_relation(&self) -> Result<EventRelation, StorageError> {
        EventRelation::new(
            self.relation_type.clone(),
            self.src.clone(),
            self.dst.clone(),
        )
    }
}

/// A source item intentionally outside one observation set.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OwnerPublicationExclusion {
    pub source_ref: String,
    pub reason: String,
}

/// A source item that could not be observed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OwnerPublicationFailure {
    pub source_ref: String,
    pub reason: String,
}

/// Terminal state of an owner completeness account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerCompletenessStatus {
    Complete,
    Incomplete,
    Failed,
    InProgress,
}

/// Owner-issued account of one bounded observation set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerCompletenessReceipt {
    pub receipt_id: String,
    pub scope: OwnerPublicationScope,
    pub included_ids: Vec<String>,
    pub exclusions: Vec<OwnerPublicationExclusion>,
    pub failures: Vec<OwnerPublicationFailure>,
    pub status: OwnerCompletenessStatus,
}

impl OwnerCompletenessReceipt {
    fn normalize(&mut self) {
        self.included_ids.sort();
        self.included_ids.dedup();
        self.exclusions.sort();
        self.exclusions.dedup();
        self.failures.sort();
        self.failures.dedup();
    }

    fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("completeness receipt_id", &self.receipt_id)?;
        self.scope.validate()?;
        for exclusion in &self.exclusions {
            require_non_empty("completeness exclusion source_ref", &exclusion.source_ref)?;
            require_non_empty("completeness exclusion reason", &exclusion.reason)?;
        }
        for failure in &self.failures {
            require_non_empty("completeness failure source_ref", &failure.source_ref)?;
            require_non_empty("completeness failure reason", &failure.reason)?;
        }
        if self.status == OwnerCompletenessStatus::Complete && !self.failures.is_empty() {
            return invalid("a complete owner receipt cannot contain failures");
        }
        if self.status == OwnerCompletenessStatus::Incomplete
            && self.exclusions.is_empty()
            && self.failures.is_empty()
        {
            return invalid("an incomplete owner receipt must explain its gap");
        }
        if self.status == OwnerCompletenessStatus::Failed && self.failures.is_empty() {
            return invalid("a failed owner receipt must contain a failure");
        }
        Ok(())
    }
}

/// Producer-owned typed batch carried intact through Events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerPublicationBatch {
    pub owner_id: String,
    pub revision_id: String,
    pub scope: OwnerPublicationScope,
    pub objects: Vec<OwnerObjectPublication>,
    pub relations: Vec<OwnerRelationOccurrence>,
    pub completeness: OwnerCompletenessReceipt,
}

impl OwnerPublicationBatch {
    /// Canonical ordering used by operation and cut identity.
    pub fn normalized(&self) -> Self {
        let mut batch = self.clone();
        for object in &mut batch.objects {
            object.normalize();
        }
        for relation in &mut batch.relations {
            relation.normalize();
        }
        batch
            .objects
            .sort_by(|left, right| left.publication_id.cmp(&right.publication_id));
        batch
            .relations
            .sort_by(|left, right| left.occurrence_id.cmp(&right.occurrence_id));
        batch.completeness.normalize();
        batch
    }

    /// Validate semantic ownership, receipt coverage, and unique identities.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("owner publication owner_id", &self.owner_id)?;
        require_non_empty("owner publication revision_id", &self.revision_id)?;
        self.scope.validate()?;
        self.completeness.validate()?;
        if self.completeness.scope != self.scope {
            return invalid("owner completeness scope must equal publication scope");
        }
        let mut identities = BTreeSet::new();
        for object in &self.objects {
            object.validate()?;
            if object.object_ref.domain_id != self.owner_id
                || object.hydration.owner_id != self.owner_id
                || object.hydration.revision_id != self.revision_id
            {
                return invalid("object publication ownership does not match its batch");
            }
            if !identities.insert(object.publication_id.clone()) {
                return invalid("duplicate owner publication identity");
            }
        }
        for relation in &self.relations {
            relation.validate()?;
            if relation.hydration.owner_id != self.owner_id
                || relation.hydration.revision_id != self.revision_id
            {
                return invalid("relation hydration does not match its batch");
            }
            if !identities.insert(relation.occurrence_id.clone()) {
                return invalid("duplicate owner publication identity");
            }
        }
        let included = self
            .completeness
            .included_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if included != identities {
            return invalid("owner completeness must name every publication exactly");
        }
        Ok(())
    }
}

/// Retryable operation reconstructed from one exact source revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerPublicationOperation {
    pub operation_id: String,
    pub enumeration_rule_revision: String,
    pub batch: OwnerPublicationBatch,
}

impl OwnerPublicationOperation {
    /// Reconstruct deterministic operation identity without process or clock state.
    pub fn reconstruct(
        enumeration_rule_revision: impl Into<String>,
        batch: OwnerPublicationBatch,
    ) -> Result<Self, StorageError> {
        let enumeration_rule_revision = enumeration_rule_revision.into();
        require_non_empty(
            "owner enumeration rule revision",
            &enumeration_rule_revision,
        )?;
        batch.validate()?;
        let batch = batch.normalized();
        let operation_id = stable_identity(
            "owner-publication-operation-v1",
            &(&enumeration_rule_revision, &batch),
        )?;
        Ok(Self {
            operation_id,
            enumeration_rule_revision,
            batch,
        })
    }

    /// Validate a deserialized operation before Graph admission.
    pub fn validate(&self) -> Result<(), StorageError> {
        if Self::reconstruct(self.enumeration_rule_revision.clone(), self.batch.clone())? != *self {
            return invalid("owner publication operation is not canonical");
        }
        Ok(())
    }

    /// Stable Event record identity for every retry.
    pub fn event_record_id(&self) -> String {
        format!("owner-publication::{}", self.operation_id)
    }
}

/// Graph-owned projection of an intact producer operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedOwnerPublication {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_route: Option<super::admission::OwnerEventSourceRef>,
    pub operation: OwnerPublicationOperation,
    pub source_event: EventRecordRef,
}

/// Owner requirement for a latest complete cut.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TraversalOwnerRequirement {
    /// Opt-in exhaustive Event source, distinct from a published owner revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_source: Option<super::admission::OwnerEventSourceRef>,
    pub owner_id: String,
    pub scope: OwnerPublicationScope,
    pub required: bool,
}

/// Explicit owner currentness selection policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerCurrentnessPolicy {
    LatestComplete,
}

/// Immutable cut request at independent Event and Graph positions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalCutRequest {
    pub owners: Vec<TraversalOwnerRequirement>,
    pub scope: OwnerPublicationScope,
    pub currentness: OwnerCurrentnessPolicy,
    pub event_position: LedgerCursor,
}

impl TraversalCutRequest {
    /// Normalize owner ordering for deterministic identity.
    pub fn normalized(&self) -> Self {
        let mut request = self.clone();
        request.owners.sort();
        request.owners.dedup();
        request
    }

    /// Validate explicit owner requirements.
    pub fn validate(&self) -> Result<(), StorageError> {
        self.scope.validate()?;
        if self.owners.is_empty() {
            return invalid("a traversal cut requires at least one owner");
        }
        let mut owner_scopes = BTreeSet::new();
        for owner in &self.owners {
            if let Some(source) = &owner.event_source {
                source.validate()?;
            }
            require_non_empty("cut owner_id", &owner.owner_id)?;
            owner.scope.validate()?;
            if !owner_scopes.insert((owner.owner_id.clone(), owner.scope.clone())) {
                return invalid("duplicate cut owner scope");
            }
        }
        Ok(())
    }
}

/// Exact owner revision selected into one immutable cut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerGraphRevisionReceipt {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_coverage: Option<OwnerEventCoverageReceipt>,
    pub owner_id: String,
    pub revision_id: String,
    pub scope: OwnerPublicationScope,
    pub completeness: OwnerCompletenessReceipt,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_event: Option<EventRecordRef>,
    pub projection_position: LedgerCursor,
}

impl OwnerGraphRevisionReceipt {
    /// Material identity excludes a later proof boundary over unchanged source contents.
    pub fn semantic_basis(&self) -> Self {
        let mut basis = self.clone();
        if let Some(coverage) = &mut basis.event_coverage {
            coverage.through.after_seq = 0;
            if basis.source_event.is_none() {
                basis.projection_position.after_seq = 0;
            }
        }
        basis
    }
}

/// Native coverage of an exact owner route from ledger genesis through the cut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerEventCoverageReceipt {
    pub source: super::admission::OwnerEventSourceRef,
    pub through: LedgerCursor,
}

/// Reason a cut cannot claim complete owner coverage.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraversalCutIssue {
    ProjectionLag { event_seq: u64, graph_seq: u64 },
    MissingRequiredOwner { owner_id: String, scope_id: String },
}

/// Completeness state of an immutable cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraversalCutStatus {
    Complete,
    Incomplete,
}

/// Immutable selection of complete owner revisions and durable positions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalCut {
    pub cut_id: String,
    pub owners: Vec<TraversalOwnerRequirement>,
    pub receipts: Vec<OwnerGraphRevisionReceipt>,
    pub scope: OwnerPublicationScope,
    pub currentness: OwnerCurrentnessPolicy,
    pub event_position: LedgerCursor,
    pub graph_position: LedgerCursor,
    pub status: TraversalCutStatus,
    pub issues: Vec<TraversalCutIssue>,
}

impl TraversalCut {
    /// Verify content-derived cut identity.
    pub fn validate_identity(&self) -> Result<(), StorageError> {
        if traversal_cut_identity(self)? != self.cut_id {
            return invalid("traversal cut identity does not match its inputs");
        }
        Ok(())
    }
}

/// Independent resource bounds for occurrence-rich traversal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalBounds {
    pub max_depth: usize,
    pub max_objects: usize,
    pub max_occurrences: usize,
    pub max_paths: usize,
}

impl TraversalBounds {
    fn validate(&self) -> Result<(), StorageError> {
        if self.max_depth == 0
            || self.max_objects == 0
            || self.max_occurrences == 0
            || self.max_paths == 0
        {
            return invalid("all traversal bounds must be greater than zero");
        }
        Ok(())
    }
}

/// Deterministic bounded traversal request against one exact cut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedTraversalRequest {
    pub roots: Vec<DomainObjectRef>,
    pub direction: TraversalDirection,
    pub relation_types: Option<Vec<String>>,
    pub bounds: TraversalBounds,
}

impl BoundedTraversalRequest {
    /// Normalize unordered request fields.
    pub fn normalized(&self) -> Self {
        let mut request = self.clone();
        request.roots.sort();
        request.roots.dedup();
        if let Some(types) = &mut request.relation_types {
            types.sort();
            types.dedup();
        }
        request
    }

    /// Validate query roots and independent bounds.
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.roots.is_empty() {
            return invalid("a bounded traversal requires at least one root");
        }
        for root in &self.roots {
            root.validate()?;
        }
        self.bounds.validate()
    }
}

/// One deterministic occurrence-qualified path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TraversalPath {
    pub objects: Vec<DomainObjectRef>,
    pub occurrence_ids: Vec<String>,
}

/// Why material remains on the unexplored frontier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraversalFrontierReason {
    DepthBound,
    ObjectBound,
    OccurrenceBound,
    PathBound,
    UnresolvedObject,
}

/// Explicit explored and unexplored boundary.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TraversalFrontierEntry {
    pub object_ref: DomainObjectRef,
    pub depth: usize,
    pub reason: TraversalFrontierReason,
}

/// Resource bounds that truncated one result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalTruncation {
    pub depth: bool,
    pub objects: bool,
    pub occurrences: bool,
    pub paths: bool,
}

impl TraversalTruncation {
    pub fn is_truncated(&self) -> bool {
        self.depth || self.objects || self.occurrences || self.paths
    }
}

/// Deterministic occurrence-rich result tied to one immutable cut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalResult {
    /// Roots proved absent within the complete declared Event-source scopes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub absent_roots: Vec<DomainObjectRef>,
    pub result_id: String,
    pub cut_id: String,
    pub objects: Vec<OwnerObjectPublication>,
    pub occurrences: Vec<OwnerRelationOccurrence>,
    pub paths: Vec<TraversalPath>,
    pub receipts: Vec<OwnerGraphRevisionReceipt>,
    pub frontier: Vec<TraversalFrontierEntry>,
    pub truncation: TraversalTruncation,
}

pub(crate) fn traversal_cut_identity(cut: &TraversalCut) -> Result<String, StorageError> {
    stable_identity(
        "traversal-cut-v1",
        &(
            &cut.owners,
            &cut.receipts,
            &cut.scope,
            cut.currentness,
            cut.event_position,
            cut.graph_position,
            cut.status,
            &cut.issues,
        ),
    )
}

pub(crate) fn traversal_result_identity(
    cut_id: &str,
    request: &BoundedTraversalRequest,
) -> Result<String, StorageError> {
    stable_identity("traversal-result-v1", &(cut_id, request))
}

fn stable_identity(namespace: &str, value: &impl Serialize) -> Result<String, StorageError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| StorageError::InvalidPath(format!("cannot encode identity: {error}")))?;
    Ok(format!("{namespace}::{}", blake3::hash(&bytes).to_hex()))
}

fn require_non_empty(label: &str, value: &str) -> Result<(), StorageError> {
    if value.trim().is_empty() {
        invalid(&format!("{label} must be non-empty"))
    } else {
        Ok(())
    }
}

fn invalid<T>(message: &str) -> Result<T, StorageError> {
    Err(StorageError::InvalidPath(message.to_string()))
}

/// Perspective namespace for current-anchor selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerspectiveKey {
    /// Perspective family such as `frame_type`.
    pub perspective_kind: String,
    /// Perspective member such as `analysis`.
    pub perspective_id: String,
}

impl PerspectiveKey {
    /// Build and validate a perspective key.
    pub fn new(
        perspective_kind: impl Into<String>,
        perspective_id: impl Into<String>,
    ) -> Result<Self, StorageError> {
        let key = Self {
            perspective_kind: perspective_kind.into(),
            perspective_id: perspective_id.into(),
        };
        key.validate()?;
        Ok(key)
    }

    /// Reject empty fields before using the key in durable indexes.
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.perspective_kind.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "perspective kind must be non-empty".to_string(),
            ));
        }
        if self.perspective_id.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "perspective id must be non-empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Deterministic storage key for perspective indexes.
    pub fn index_key(&self) -> String {
        format!("{}::{}", self.perspective_kind, self.perspective_id)
    }
}

/// Durable record for one anchor selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorSelectionRecord {
    /// Stable anchor id.
    pub anchor_id: AnchorId,
    /// Logical anchor slot being selected.
    pub anchor_ref: DomainObjectRef,
    /// Subject whose current state this anchor describes.
    pub subject: DomainObjectRef,
    /// Perspective that owns this current selection.
    pub perspective: PerspectiveKey,
    /// Current target object for the subject and perspective.
    pub target: DomainObjectRef,
    /// Source fact ids that justify this anchor.
    pub source_fact_ids: Vec<String>,
    /// Fact that created this anchor record.
    pub created_by_fact_id: String,
    /// Runtime sequence where this anchor became current.
    pub selected_at_seq: u64,
    /// Runtime sequence where this anchor stopped being current.
    pub ended_at_seq: Option<u64>,
    /// Replacement anchor id when superseded by another anchor.
    pub ended_by_anchor_id: Option<AnchorId>,
    #[serde(default)]
    /// Fact that ended this anchor when known.
    pub ended_by_fact_id: Option<String>,
}

/// Reducer input for selecting a new current anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorSelectionInput {
    pub anchor_ref: DomainObjectRef,
    pub subject: DomainObjectRef,
    pub perspective: PerspectiveKey,
    pub target: DomainObjectRef,
    pub source_fact_id: String,
}

/// Reducer input for ending a current anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorEndInput {
    pub anchor_ref: DomainObjectRef,
    pub ended_at_seq: u64,
}

/// Graph mutation intent derived from a source event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum TraversalIntent {
    /// Select a new current anchor.
    SelectAnchor(AnchorSelectionInput),
    /// End an existing current anchor.
    EndAnchor(AnchorEndInput),
}

/// Graph-readable fact copied from the event ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraversalFactRecord {
    pub fact_id: TraversalFactId,
    pub source_spine_fact_id: String,
    pub seq: u64,
    pub event_type: String,
    pub objects: Vec<DomainObjectRef>,
    pub relations: Vec<EventRelation>,
}

/// Provenance bundle for a selected anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorProvenanceRecord {
    pub anchor_id: AnchorId,
    pub source_fact_ids: Vec<String>,
    #[serde(default)]
    pub derived_fact_ids: Vec<String>,
    pub objects: Vec<DomainObjectRef>,
    pub relations: Vec<EventRelation>,
}

/// Direction used by neighbor and walk queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraversalDirection {
    Outgoing,
    Incoming,
    Both,
}

/// Bounded graph walk request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphWalkSpec {
    pub direction: TraversalDirection,
    pub relation_types: Option<Vec<String>>,
    pub max_depth: usize,
    pub current_only: bool,
    pub include_facts: bool,
}

impl GraphWalkSpec {
    /// Validate walk bounds before querying indexes.
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.max_depth == 0 {
            return Err(StorageError::InvalidPath(
                "graph walk max_depth must be at least 1".to_string(),
            ));
        }
        Ok(())
    }
}

/// Objects, facts, and relations reached by a graph walk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphWalkResult {
    pub visited_objects: Vec<DomainObjectRef>,
    pub visited_facts: Vec<TraversalFactRecord>,
    pub traversed_relations: Vec<EventRelation>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn anchor_selection_record_round_trips() {
        let record = AnchorSelectionRecord {
            anchor_id: "anchor_a".to_string(),
            anchor_ref: DomainObjectRef::new("context", "head", "node_a::analysis").unwrap(),
            subject: DomainObjectRef::new("workspace_fs", "node", "node_a").unwrap(),
            perspective: PerspectiveKey::new("frame_type", "analysis").unwrap(),
            target: DomainObjectRef::new("context", "frame", "frame_a").unwrap(),
            source_fact_ids: vec!["spine::1".to_string()],
            created_by_fact_id: "fact_a".to_string(),
            selected_at_seq: 1,
            ended_at_seq: None,
            ended_by_anchor_id: None,
            ended_by_fact_id: None,
        };

        let serialized = serde_json::to_string(&record).unwrap();
        let parsed: AnchorSelectionRecord = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed.anchor_id, "anchor_a");
        assert_eq!(parsed.perspective.perspective_kind, "frame_type");
    }

    #[test]
    fn perspective_key_rejects_empty_fields() {
        assert!(PerspectiveKey::new("", "analysis").is_err());
        assert!(PerspectiveKey::new("frame_type", "").is_err());
    }

    #[test]
    fn graph_walk_spec_requires_positive_depth() {
        let spec = GraphWalkSpec {
            direction: TraversalDirection::Both,
            relation_types: None,
            max_depth: 0,
            current_only: true,
            include_facts: false,
        };
        assert!(spec.validate().is_err());
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn owner_operation_identity_is_invariant_to_publication_order(
            values in proptest::collection::btree_set(any::<u16>(), 1..24)
        ) {
            let scope = OwnerPublicationScope {
                scope_id: "scope-a".to_string(),
                branch_id: None,
                perspective_id: None,
                valid_at: None,
            };
            let mut objects = values
                .iter()
                .map(|value| {
                    let id = format!("object-{value}");
                    OwnerObjectPublication {
                        publication_id: id.clone(),
                        object_ref: DomainObjectRef::new("owner-a", "specimen", &id).unwrap(),
                        state: OwnerPublicationState::Observed,
                        source_product_ref: id.clone(),
                        hydration: HydrationReference {
                            owner_id: "owner-a".to_string(),
                            product_kind: "specimen".to_string(),
                            product_id: id,
                            revision_id: "revision-a".to_string(),
                            role: "published_material".to_string(),
                        },
                        provenance_refs: Vec::new(),
                        qualifications: BTreeMap::new(),
                    }
                })
                .collect::<Vec<_>>();
            let included_ids = objects
                .iter()
                .map(|object| object.publication_id.clone())
                .collect::<Vec<_>>();
            let batch = |objects| OwnerPublicationBatch {
                owner_id: "owner-a".to_string(),
                revision_id: "revision-a".to_string(),
                scope: scope.clone(),
                objects,
                relations: Vec::new(),
                completeness: OwnerCompletenessReceipt {
                    receipt_id: "receipt-a".to_string(),
                    scope: scope.clone(),
                    included_ids: included_ids.clone(),
                    exclusions: Vec::new(),
                    failures: Vec::new(),
                    status: OwnerCompletenessStatus::Complete,
                },
            };
            let first = OwnerPublicationOperation::reconstruct("rule-a", batch(objects.clone())).unwrap();
            objects.reverse();
            let reversed = OwnerPublicationOperation::reconstruct("rule-a", batch(objects)).unwrap();
            prop_assert_eq!(first, reversed);
        }
    }
}
