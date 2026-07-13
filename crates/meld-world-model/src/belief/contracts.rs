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

use crate::error::StorageError;
use crate::events::{DomainObjectRef, EventRecordRef, EventRelation, LedgerCursor};
use crate::world_state::graph::{AnchorId, PerspectiveKey};

/// Runtime-selected policy identity for evidence filtering and weighting.
pub type EvidencePolicyId = String;

/// Wire schema for durable belief authority migration markers.
pub const BELIEF_AUTHORITY_MIGRATION_SCHEMA_VERSION: u32 = 1;

/// Stable identity for moving legacy belief state into one product authority.
///
/// The identity names logical authorities rather than filesystem paths. Root
/// assembly owns path resolution and must prove that source and target are
/// physically distinct before handing this product to the belief domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "BeliefAuthorityMigrationIdentityWire")]
pub struct BeliefAuthorityMigrationIdentity {
    /// Stable id reused for every retry of the same migration.
    migration_id: String,
    /// Persisted identity established for the frozen legacy source.
    source_authority_id: String,
    /// Persisted identity of the product belief authority.
    target_authority_id: String,
    /// Product binding generation that authorized this migration.
    generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct BeliefAuthorityMigrationIdentityWire {
    migration_id: String,
    source_authority_id: String,
    target_authority_id: String,
    generation: u64,
}

impl BeliefAuthorityMigrationIdentity {
    /// Build one validated authority migration identity.
    pub fn try_new(
        migration_id: impl Into<String>,
        source_authority_id: impl Into<String>,
        target_authority_id: impl Into<String>,
        generation: u64,
    ) -> Result<Self, StorageError> {
        let identity = Self {
            migration_id: migration_id.into(),
            source_authority_id: source_authority_id.into(),
            target_authority_id: target_authority_id.into(),
            generation,
        };
        identity.validate()?;
        Ok(identity)
    }

    /// Validate authority identity and cutover generation.
    pub fn validate(&self) -> Result<(), StorageError> {
        require_non_empty("belief migration id", &self.migration_id)?;
        require_non_empty("belief source authority id", &self.source_authority_id)?;
        require_non_empty("belief target authority id", &self.target_authority_id)?;
        if self.source_authority_id == self.target_authority_id {
            return Err(StorageError::InvalidPath(
                "belief migration source and target authorities must differ".to_string(),
            ));
        }
        if self.generation == 0 {
            return Err(StorageError::InvalidPath(
                "belief migration generation must be positive".to_string(),
            ));
        }
        Ok(())
    }

    /// Borrow the stable migration id.
    pub fn migration_id(&self) -> &str {
        &self.migration_id
    }

    /// Borrow the frozen source authority id.
    pub fn source_authority_id(&self) -> &str {
        &self.source_authority_id
    }

    /// Borrow the product target authority id.
    pub fn target_authority_id(&self) -> &str {
        &self.target_authority_id
    }

    /// Return the product binding generation.
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

impl TryFrom<BeliefAuthorityMigrationIdentityWire> for BeliefAuthorityMigrationIdentity {
    type Error = StorageError;

    fn try_from(value: BeliefAuthorityMigrationIdentityWire) -> Result<Self, Self::Error> {
        Self::try_new(
            value.migration_id,
            value.source_authority_id,
            value.target_authority_id,
            value.generation,
        )
    }
}

/// Identity and record count for one frozen belief authority snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "BeliefAuthoritySnapshotWire")]
pub struct BeliefAuthoritySnapshot {
    /// BLAKE3 hash over ordered authority records.
    snapshot_hash: String,
    /// Total records covered by the snapshot hash.
    record_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct BeliefAuthoritySnapshotWire {
    snapshot_hash: String,
    record_count: u64,
}

impl BeliefAuthoritySnapshot {
    /// Build validated frozen authority snapshot metadata.
    pub fn try_new(
        snapshot_hash: impl Into<String>,
        record_count: u64,
    ) -> Result<Self, StorageError> {
        let snapshot = Self {
            snapshot_hash: snapshot_hash.into(),
            record_count,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    /// Validate the canonical lowercase BLAKE3 digest.
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.snapshot_hash.len() != 64
            || !self
                .snapshot_hash
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(StorageError::InvalidPath(
                "belief authority snapshot hash must be a lowercase BLAKE3 hex digest".to_string(),
            ));
        }
        Ok(())
    }

    /// Borrow the canonical snapshot digest.
    pub fn snapshot_hash(&self) -> &str {
        &self.snapshot_hash
    }

    /// Return the number of records covered by the snapshot.
    pub fn record_count(&self) -> u64 {
        self.record_count
    }
}

impl TryFrom<BeliefAuthoritySnapshotWire> for BeliefAuthoritySnapshot {
    type Error = StorageError;

    fn try_from(value: BeliefAuthoritySnapshotWire) -> Result<Self, Self::Error> {
        Self::try_new(value.snapshot_hash, value.record_count)
    }
}

/// Durable proof that source and target authority snapshots are equal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "BeliefAuthorityParityReceiptWire")]
pub struct BeliefAuthorityParityReceipt {
    /// Frozen legacy source snapshot.
    source: BeliefAuthoritySnapshot,
    /// Product target snapshot verified against the source.
    target: BeliefAuthoritySnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct BeliefAuthorityParityReceiptWire {
    source: BeliefAuthoritySnapshot,
    target: BeliefAuthoritySnapshot,
}

impl BeliefAuthorityParityReceipt {
    /// Bind equal source and target authority snapshots as durable parity proof.
    pub fn try_new(
        source: BeliefAuthoritySnapshot,
        target: BeliefAuthoritySnapshot,
    ) -> Result<Self, StorageError> {
        source.validate()?;
        target.validate()?;
        if source != target {
            return Err(StorageError::InvalidPath(
                "belief authority parity requires equal source and target snapshots".to_string(),
            ));
        }
        Ok(Self { source, target })
    }

    /// Borrow the frozen source snapshot.
    pub fn source(&self) -> &BeliefAuthoritySnapshot {
        &self.source
    }

    /// Borrow the verified product target snapshot.
    pub fn target(&self) -> &BeliefAuthoritySnapshot {
        &self.target
    }
}

impl TryFrom<BeliefAuthorityParityReceiptWire> for BeliefAuthorityParityReceipt {
    type Error = StorageError;

    fn try_from(value: BeliefAuthorityParityReceiptWire) -> Result<Self, Self::Error> {
        Self::try_new(value.source, value.target)
    }
}

/// State-specific durable progress for belief authority migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum BeliefAuthorityMigrationProgress {
    /// Source identity and immutable snapshot metadata are established.
    Prepared {
        /// Frozen legacy source snapshot.
        source: BeliefAuthoritySnapshot,
    },
    /// Source records are being copied into the product target.
    Copying {
        /// Frozen legacy source snapshot.
        source: BeliefAuthoritySnapshot,
        /// Source records durably copied and verified so far.
        verified_record_count: u64,
    },
    /// Source and target snapshots have equal durable content.
    Verified {
        /// Durable source and target parity proof.
        parity: BeliefAuthorityParityReceipt,
    },
    /// Product reads have cut over after durable parity proof.
    Cutover {
        /// Durable source and target parity proof retained at cutover.
        parity: BeliefAuthorityParityReceipt,
    },
    /// Product writes require forward repair after durable cutover.
    ForwardRepairOnly {
        /// Last durable source and target parity proof before product writes.
        parity: BeliefAuthorityParityReceipt,
    },
}

/// Durable checkpoint for resumable belief authority migration.
///
/// `migration` is the canonical identity product. Counts and the source hash
/// are operational verification metadata and must never substitute for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "BeliefAuthorityMigrationMarkerWire")]
pub struct BeliefAuthorityMigrationMarker {
    /// Marker wire schema.
    schema_version: u32,
    /// Canonical migration identity shared by every checkpoint.
    migration: BeliefAuthorityMigrationIdentity,
    /// State-specific recoverable migration progress.
    progress: BeliefAuthorityMigrationProgress,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct BeliefAuthorityMigrationMarkerWire {
    schema_version: u32,
    migration: BeliefAuthorityMigrationIdentity,
    progress: BeliefAuthorityMigrationProgress,
}

impl BeliefAuthorityMigrationMarker {
    /// Build one validated durable migration checkpoint.
    pub fn try_new(
        migration: BeliefAuthorityMigrationIdentity,
        progress: BeliefAuthorityMigrationProgress,
    ) -> Result<Self, StorageError> {
        let marker = Self {
            schema_version: BELIEF_AUTHORITY_MIGRATION_SCHEMA_VERSION,
            migration,
            progress,
        };
        marker.validate()?;
        Ok(marker)
    }

    /// Validate the marker schema and state-specific progress.
    pub fn validate(&self) -> Result<(), StorageError> {
        if self.schema_version != BELIEF_AUTHORITY_MIGRATION_SCHEMA_VERSION {
            return Err(StorageError::InvalidPath(format!(
                "unsupported belief migration marker schema {}",
                self.schema_version
            )));
        }
        self.migration.validate()?;
        match &self.progress {
            BeliefAuthorityMigrationProgress::Prepared { source } => source.validate(),
            BeliefAuthorityMigrationProgress::Copying {
                source,
                verified_record_count,
            } => {
                source.validate()?;
                if *verified_record_count > source.record_count() {
                    return Err(StorageError::InvalidPath(
                        "belief migration verified count exceeds frozen source count".to_string(),
                    ));
                }
                Ok(())
            }
            BeliefAuthorityMigrationProgress::Verified { parity }
            | BeliefAuthorityMigrationProgress::Cutover { parity }
            | BeliefAuthorityMigrationProgress::ForwardRepairOnly { parity } => {
                parity.source().validate()?;
                parity.target().validate()
            }
        }
    }

    /// Return the durable marker schema version.
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Borrow the migration identity.
    pub fn migration(&self) -> &BeliefAuthorityMigrationIdentity {
        &self.migration
    }

    /// Borrow the state-specific migration progress.
    pub fn progress(&self) -> &BeliefAuthorityMigrationProgress {
        &self.progress
    }
}

impl TryFrom<BeliefAuthorityMigrationMarkerWire> for BeliefAuthorityMigrationMarker {
    type Error = StorageError;

    fn try_from(value: BeliefAuthorityMigrationMarkerWire) -> Result<Self, Self::Error> {
        let marker = Self {
            schema_version: value.schema_version,
            migration: value.migration,
            progress: value.progress,
        };
        marker.validate()?;
        Ok(marker)
    }
}

/// Compatibility posture for legacy belief state during authority cutover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyBeliefCompatibilityPosture {
    /// No legacy belief records require migration.
    NoLegacyState,
    /// Legacy state is frozen and readable only for migration or parity checks.
    LegacyReadOnly,
    /// Product state has passed full characterization and parity verification.
    ParityVerified,
    /// Product belief state is the only authority used by runtime readers.
    ProductAuthoritative,
    /// Product writes prevent safe rollback to the legacy source.
    ForwardRepairOnly,
}

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
}

/// Normalized belief input derived from graph state or promoted facts.
///
/// Evidence is append-only. Assignment may attach the same evidence to one or
/// more belief keys, but the evidence record itself remains a durable audit
/// record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceItem {
    pub evidence_id: String,
    pub candidate_key: BeliefKey,
    pub source_fact_ids: Vec<String>,
    pub graph_anchor_ids: Vec<AnchorId>,
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
    pub provenance: BeliefProvenanceSummary,
}

/// Generic promoted record accepted by evidence normalization.
///
/// The fields are runtime data rather than family-specific Rust types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromotedEvidenceRecord {
    pub source_kind: String,
    pub source_id: String,
    pub subject: DomainObjectRef,
    pub source_fact_ids: Vec<String>,
    pub graph_anchor_ids: Vec<AnchorId>,
    pub objects: Vec<DomainObjectRef>,
    pub relations: Vec<EventRelation>,
    pub source_cursor_start: u64,
    pub source_cursor_end: u64,
    pub reference_time: Option<String>,
    pub transaction_seq: u64,
    pub content_hash: Option<String>,
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
}

/// World-model-owned durable position for one evidence consumer.
///
/// The contained ledger cursor preserves canonical event authority identity.
/// Configuration and scope fields distinguish consumers that may traverse the
/// same event stream but produce different evidence products.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceConsumerCursor {
    /// Stable consumer identity owned by the belief domain.
    pub consumer_id: String,
    /// Identity-bearing canonical event position already processed.
    pub ledger_cursor: LedgerCursor,
    /// Hash of the complete belief family configuration used by the consumer.
    pub family_config_hash: String,
    /// Hash of the ordered source-mapping configuration used for promotion.
    pub source_mapping_hash: String,
    /// Perspective applied while assigning promoted evidence.
    pub perspective: PerspectiveKey,
    /// Branch applied while assigning promoted evidence.
    pub branch_scope: BranchScope,
}

/// Durable result classification for one evidence consumer receipt.
///
/// Every canonical source record receives one disposition, including records
/// that intentionally produce no evidence, so cursor recovery never depends on
/// hidden worker memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceIngestionReceiptDisposition {
    /// Source mapping produced one or more normalized evidence items.
    Promoted,
    /// Source mapping recognized the record but rejected its content.
    Rejected,
    /// The source record is outside the configured evidence mapping.
    Irrelevant,
}

/// Persistence result for one canonical evidence ingestion receipt.
///
/// Replay is write metadata rather than receipt truth. An exact replay retains
/// the original promoted, rejected, or irrelevant semantic disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceIngestionReceiptWriteDisposition {
    /// The canonical receipt was inserted for the first time.
    Inserted,
    /// An exactly equal canonical receipt was already durable.
    ExactReplay,
}

/// Stable identity for one evidence ingestion receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceIngestionReceiptIdentity {
    /// Belief-owned consumer that interpreted the source record.
    pub consumer_id: String,
    /// Canonical source event record interpreted by the consumer.
    pub source_record: EventRecordRef,
    /// Belief family configuration used for interpretation.
    pub family_config_hash: String,
    /// Source mapping configuration used for interpretation.
    pub source_mapping_hash: String,
    /// Perspective used for evidence assignment.
    pub perspective: PerspectiveKey,
    /// Branch used for evidence assignment.
    pub branch_scope: BranchScope,
}

/// Durable semantic result for one canonical source event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceIngestionReceipt {
    /// Complete immutable receipt identity.
    pub identity: EvidenceIngestionReceiptIdentity,
    /// Semantic mapping result retained across exact write replay.
    pub disposition: EvidenceIngestionReceiptDisposition,
    /// Sorted evidence ids produced by this source record.
    pub evidence_ids: Vec<String>,
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

/// Complete compare-and-swap intent for one assessment lease transition.
///
/// Each variant carries complete canonical lease products. This prevents
/// callers from validating only a lease id while changing ownership, epoch,
/// cursor, configuration, or lifecycle state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "snake_case",
    try_from = "AssessmentLeaseCasIntentWire"
)]
pub enum AssessmentLeaseCasIntent {
    /// Install the proposed leased product only when no active lease exists.
    Acquire {
        /// Complete active lease product with `LeaseStatus::Leased`.
        proposed_active_lease: AssessmentLease,
    },
    /// Persist a completed product and clear the matching active lease.
    Complete {
        /// Exact active leased product required before the transition.
        expected_active_lease: AssessmentLease,
        /// Complete terminal product with `LeaseStatus::Completed`.
        completed_lease: AssessmentLease,
    },
    /// Persist an abandoned product and clear the matching expired active lease.
    AbandonExpired {
        /// Exact active leased product required before the transition.
        expected_active_lease: AssessmentLease,
        /// Complete terminal product with `LeaseStatus::Abandoned`.
        abandoned_lease: AssessmentLease,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum AssessmentLeaseCasIntentWire {
    Acquire {
        proposed_active_lease: AssessmentLease,
    },
    Complete {
        expected_active_lease: AssessmentLease,
        completed_lease: AssessmentLease,
    },
    AbandonExpired {
        expected_active_lease: AssessmentLease,
        abandoned_lease: AssessmentLease,
    },
}

impl AssessmentLeaseCasIntent {
    /// Validate the complete legal lease transition carried by this intent.
    pub fn validate(&self) -> Result<(), StorageError> {
        match self {
            Self::Acquire {
                proposed_active_lease,
            } if proposed_active_lease.status == LeaseStatus::Leased => {
                validate_assessment_lease(proposed_active_lease)
            }
            Self::Complete {
                expected_active_lease,
                completed_lease,
            } => validate_terminal_lease_transition(
                expected_active_lease,
                completed_lease,
                LeaseStatus::Completed,
            ),
            Self::AbandonExpired {
                expected_active_lease,
                abandoned_lease,
            } => validate_terminal_lease_transition(
                expected_active_lease,
                abandoned_lease,
                LeaseStatus::Abandoned,
            ),
            Self::Acquire { .. } => Err(StorageError::InvalidPath(
                "assessment lease acquire requires leased status".to_string(),
            )),
        }
    }
}

impl TryFrom<AssessmentLeaseCasIntentWire> for AssessmentLeaseCasIntent {
    type Error = StorageError;

    fn try_from(value: AssessmentLeaseCasIntentWire) -> Result<Self, Self::Error> {
        let intent = match value {
            AssessmentLeaseCasIntentWire::Acquire {
                proposed_active_lease,
            } => Self::Acquire {
                proposed_active_lease,
            },
            AssessmentLeaseCasIntentWire::Complete {
                expected_active_lease,
                completed_lease,
            } => Self::Complete {
                expected_active_lease,
                completed_lease,
            },
            AssessmentLeaseCasIntentWire::AbandonExpired {
                expected_active_lease,
                abandoned_lease,
            } => Self::AbandonExpired {
                expected_active_lease,
                abandoned_lease,
            },
        };
        intent.validate()?;
        Ok(intent)
    }
}

fn validate_terminal_lease_transition(
    expected: &AssessmentLease,
    terminal: &AssessmentLease,
    terminal_status: LeaseStatus,
) -> Result<(), StorageError> {
    validate_assessment_lease(expected)?;
    validate_assessment_lease(terminal)?;
    if expected.status != LeaseStatus::Leased {
        return Err(StorageError::InvalidPath(
            "assessment lease terminal transition requires leased active status".to_string(),
        ));
    }
    let mut expected_terminal = expected.clone();
    expected_terminal.status = terminal_status;
    if terminal == &expected_terminal {
        Ok(())
    } else {
        Err(StorageError::InvalidPath(
            "assessment lease terminal product changed immutable lease identity".to_string(),
        ))
    }
}

fn validate_assessment_lease(lease: &AssessmentLease) -> Result<(), StorageError> {
    require_non_empty("assessment lease id", &lease.lease_id)?;
    require_non_empty("assessment lease owner id", &lease.owner_id)?;
    require_non_empty(
        "assessment lease comparator engine id",
        &lease.comparator_engine_id,
    )?;
    require_non_empty(
        "assessment lease config snapshot hash",
        &lease.config_snapshot_hash,
    )?;
    lease.belief_key.validate()?;
    if lease.epoch == 0 {
        return Err(StorageError::InvalidPath(
            "assessment lease epoch must be positive".to_string(),
        ));
    }
    if lease.input_cursor_start > lease.input_cursor_end {
        return Err(StorageError::InvalidPath(
            "assessment lease input cursor range is reversed".to_string(),
        ));
    }
    if lease.expires_at_seq <= lease.started_at_seq {
        return Err(StorageError::InvalidPath(
            "assessment lease expiry must follow its start".to_string(),
        ));
    }
    Ok(())
}

/// Recovery result after reconciling an interrupted atomic belief commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeliefCommitRecoveryDisposition {
    /// Revision head, public view, dirty state, and lease state already agree.
    AlreadyConsistent,
    /// A durable revision was used to rebuild its missing or stale public view.
    RebuiltCurrentView,
    /// Durable commit intent was resumed to complete all atomic products.
    ResumedCommit,
    /// Recovery preserved evidence and rescheduled the key for reassessment.
    RescheduledDirtyKey,
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
}

/// Durable intent binding every product in one atomic belief commit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "BeliefCommitIntentWire")]
pub struct BeliefCommitIntent {
    /// Stable id reused while recovering the same commit attempt.
    intent_id: String,
    /// Exact active lease required before commit.
    expected_active_lease: AssessmentLease,
    /// Complete terminal lease persisted by commit.
    completed_lease: AssessmentLease,
    /// Exact dirty-key state consumed by commit.
    expected_dirty_state: DirtyKeyState,
    /// Canonical append-only revision product.
    revision: BeliefRevision,
    /// Receiver-owned public view projected from the revision.
    public_view: BeliefView,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct BeliefCommitIntentWire {
    intent_id: String,
    expected_active_lease: AssessmentLease,
    completed_lease: AssessmentLease,
    expected_dirty_state: DirtyKeyState,
    revision: BeliefRevision,
    public_view: BeliefView,
}

impl BeliefCommitIntent {
    /// Bind every product in one legal leased-to-completed belief commit.
    pub fn try_new(
        intent_id: impl Into<String>,
        expected_active_lease: AssessmentLease,
        completed_lease: AssessmentLease,
        expected_dirty_state: DirtyKeyState,
        revision: BeliefRevision,
        public_view: BeliefView,
    ) -> Result<Self, StorageError> {
        let intent_id = intent_id.into();
        require_non_empty("belief commit intent id", &intent_id)?;
        validate_terminal_lease_transition(
            &expected_active_lease,
            &completed_lease,
            LeaseStatus::Completed,
        )?;
        if expected_dirty_state.dirty_since_seq > expected_dirty_state.latest_seq {
            return Err(StorageError::InvalidPath(
                "belief commit dirty-key sequence range is reversed".to_string(),
            ));
        }
        let key = &expected_active_lease.belief_key;
        if expected_dirty_state.belief_key != *key
            || expected_dirty_state.active_lease_id.as_deref()
                != Some(expected_active_lease.lease_id.as_str())
            || revision.belief_key != *key
            || public_view.key != *key
            || public_view.current_revision_id.as_deref() != Some(revision.revision_id.as_str())
        {
            return Err(StorageError::InvalidPath(
                "belief commit products must share one key, active lease, and revision".to_string(),
            ));
        }
        Ok(Self {
            intent_id,
            expected_active_lease,
            completed_lease,
            expected_dirty_state,
            revision,
            public_view,
        })
    }

    /// Borrow the stable commit intent id.
    pub fn intent_id(&self) -> &str {
        &self.intent_id
    }

    /// Borrow the exact active lease required before commit.
    pub fn expected_active_lease(&self) -> &AssessmentLease {
        &self.expected_active_lease
    }

    /// Borrow the terminal lease persisted by commit.
    pub fn completed_lease(&self) -> &AssessmentLease {
        &self.completed_lease
    }

    /// Borrow the dirty-key state consumed by commit.
    pub fn expected_dirty_state(&self) -> &DirtyKeyState {
        &self.expected_dirty_state
    }

    /// Borrow the canonical revision product.
    pub fn revision(&self) -> &BeliefRevision {
        &self.revision
    }

    /// Borrow the receiver-owned public view.
    pub fn public_view(&self) -> &BeliefView {
        &self.public_view
    }
}

impl TryFrom<BeliefCommitIntentWire> for BeliefCommitIntent {
    type Error = StorageError;

    fn try_from(value: BeliefCommitIntentWire) -> Result<Self, Self::Error> {
        Self::try_new(
            value.intent_id,
            value.expected_active_lease,
            value.completed_lease,
            value.expected_dirty_state,
            value.revision,
            value.public_view,
        )
    }
}

/// Compact provenance and hydration summary for belief records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeliefProvenanceSummary {
    pub evidence_ids: Vec<String>,
    pub source_fact_ids: Vec<String>,
    pub graph_anchor_ids: Vec<AnchorId>,
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
    pub graph_anchor_ids: Vec<AnchorId>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::LedgerIdentity;

    fn belief_key() -> BeliefKey {
        BeliefKey {
            subject: DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap(),
            dimension_id: "docs_freshness".to_string(),
            predicate_id: "confidence".to_string(),
            perspective: PerspectiveKey::new("agent", "default").unwrap(),
            branch_scope: BranchScope::main(),
            evidence_policy_id: "policy-a".to_string(),
        }
    }

    fn legacy_lease_json() -> serde_json::Value {
        serde_json::json!({
            "lease_id": "lease-a",
            "belief_key": belief_key(),
            "epoch": 9,
            "owner_id": "worker-a",
            "input_cursor_start": 4,
            "input_cursor_end": 8,
            "started_at_seq": 4,
            "expires_at_seq": 20,
            "comparator_engine_id": "weighted_bayesian",
            "config_snapshot_hash": "config-a",
            "status": "Leased"
        })
    }

    #[test]
    fn belief_authority_migration_contracts_round_trip() {
        let marker = BeliefAuthorityMigrationMarker::try_new(
            BeliefAuthorityMigrationIdentity::try_new(
                "belief-migration-a",
                "legacy-belief-a",
                "product-belief-a",
                3,
            )
            .unwrap(),
            BeliefAuthorityMigrationProgress::Verified {
                parity: BeliefAuthorityParityReceipt::try_new(
                    BeliefAuthoritySnapshot::try_new("a".repeat(64), 12).unwrap(),
                    BeliefAuthoritySnapshot::try_new("a".repeat(64), 12).unwrap(),
                )
                .unwrap(),
            },
        )
        .unwrap();

        let value = serde_json::to_value(&marker).unwrap();
        assert_eq!(value["progress"]["state"], "verified");
        assert_eq!(
            serde_json::from_value::<BeliefAuthorityMigrationMarker>(value).unwrap(),
            marker
        );
        let invalid = serde_json::json!({
            "source": {"snapshot_hash": "a", "record_count": 1},
            "target": {"snapshot_hash": "b", "record_count": 1}
        });
        assert!(serde_json::from_value::<BeliefAuthorityParityReceipt>(invalid).is_err());
    }

    #[test]
    fn belief_authority_migration_rejects_invalid_durable_states() {
        assert!(BeliefAuthorityMigrationIdentity::try_new("id", "same", "same", 1).is_err());
        assert!(BeliefAuthoritySnapshot::try_new("not-a-digest", 1).is_err());

        let marker = serde_json::json!({
            "schema_version": BELIEF_AUTHORITY_MIGRATION_SCHEMA_VERSION + 1,
            "migration": {
                "migration_id": "migration-a",
                "source_authority_id": "legacy-a",
                "target_authority_id": "product-a",
                "generation": 1
            },
            "progress": {
                "state": "copying",
                "source": {
                    "snapshot_hash": "a".repeat(64),
                    "record_count": 2
                },
                "verified_record_count": 3
            }
        });
        assert!(serde_json::from_value::<BeliefAuthorityMigrationMarker>(marker).is_err());

        let excessive_progress = serde_json::json!({
            "schema_version": BELIEF_AUTHORITY_MIGRATION_SCHEMA_VERSION,
            "migration": {
                "migration_id": "migration-a",
                "source_authority_id": "legacy-a",
                "target_authority_id": "product-a",
                "generation": 1
            },
            "progress": {
                "state": "copying",
                "source": {
                    "snapshot_hash": "a".repeat(64),
                    "record_count": 2
                },
                "verified_record_count": 3
            }
        });
        assert!(
            serde_json::from_value::<BeliefAuthorityMigrationMarker>(excessive_progress).is_err()
        );
    }

    #[test]
    fn evidence_cursor_preserves_ledger_and_consumer_identity() {
        let ledger_id: LedgerIdentity = "00000000-0000-0000-0000-000000000123".parse().unwrap();
        let cursor = EvidenceConsumerCursor {
            consumer_id: "world_model.evidence.default".to_string(),
            ledger_cursor: LedgerCursor {
                ledger_id,
                after_seq: 42,
            },
            family_config_hash: "family-a".to_string(),
            source_mapping_hash: "mapping-a".to_string(),
            perspective: PerspectiveKey::new("agent", "default").unwrap(),
            branch_scope: BranchScope::main(),
        };

        let value = serde_json::to_value(&cursor).unwrap();
        assert_eq!(value["ledger_cursor"]["ledger_id"], ledger_id.to_string());
        assert_eq!(value["ledger_cursor"]["after_seq"], 42);
        assert_eq!(
            serde_json::from_value::<EvidenceConsumerCursor>(value).unwrap(),
            cursor
        );
    }

    #[test]
    fn evidence_receipt_and_recovery_dispositions_have_stable_wire_names() {
        assert_eq!(
            serde_json::to_value(EvidenceIngestionReceiptWriteDisposition::ExactReplay).unwrap(),
            "exact_replay"
        );
        assert_eq!(
            serde_json::to_value(BeliefCommitRecoveryDisposition::RebuiltCurrentView).unwrap(),
            "rebuilt_current_view"
        );
        assert_eq!(
            serde_json::to_value(LegacyBeliefCompatibilityPosture::ForwardRepairOnly).unwrap(),
            "forward_repair_only"
        );
    }

    #[test]
    fn evidence_receipt_identity_binds_source_and_mapping_inputs() {
        let ledger_id: LedgerIdentity = "00000000-0000-0000-0000-000000000123".parse().unwrap();
        let receipt = EvidenceIngestionReceipt {
            identity: EvidenceIngestionReceiptIdentity {
                consumer_id: "world_model.evidence.default".to_string(),
                source_record: EventRecordRef { ledger_id, seq: 42 },
                family_config_hash: "family-a".to_string(),
                source_mapping_hash: "mapping-a".to_string(),
                perspective: PerspectiveKey::new("agent", "default").unwrap(),
                branch_scope: BranchScope::main(),
            },
            disposition: EvidenceIngestionReceiptDisposition::Promoted,
            evidence_ids: vec!["evidence-a".to_string()],
        };

        let encoded = serde_json::to_vec(&receipt).unwrap();
        let decoded: EvidenceIngestionReceipt = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, receipt);
    }

    #[test]
    fn assessment_lease_cas_intent_carries_complete_expected_product() {
        let active: AssessmentLease = serde_json::from_value(legacy_lease_json()).unwrap();
        let mut completed = active.clone();
        completed.status = LeaseStatus::Completed;
        let intent = AssessmentLeaseCasIntent::Complete {
            expected_active_lease: active.clone(),
            completed_lease: completed,
        };

        let value = serde_json::to_value(&intent).unwrap();
        assert_eq!(value["action"], "complete");
        assert_eq!(
            value["expected_active_lease"]["belief_key"]["subject"]["domain_id"],
            "workspace_fs"
        );
        assert_eq!(
            serde_json::from_value::<AssessmentLeaseCasIntent>(value).unwrap(),
            intent
        );
    }

    #[test]
    fn assessment_lease_acquire_carries_the_active_leased_product() {
        let mut leased: AssessmentLease = serde_json::from_value(legacy_lease_json()).unwrap();
        leased.status = LeaseStatus::Leased;
        let intent = AssessmentLeaseCasIntent::Acquire {
            proposed_active_lease: leased.clone(),
        };

        let value = serde_json::to_value(&intent).unwrap();
        assert_eq!(value["action"], "acquire");
        assert_eq!(value["proposed_active_lease"]["status"], "Leased");
        assert_eq!(
            serde_json::from_value::<AssessmentLeaseCasIntent>(value).unwrap(),
            intent
        );
    }

    #[test]
    fn assessment_lease_cas_rejects_malformed_active_products() {
        let mut invalid = legacy_lease_json();
        invalid["owner_id"] = serde_json::json!("");
        invalid["input_cursor_start"] = serde_json::json!(9);
        invalid["input_cursor_end"] = serde_json::json!(8);
        invalid["expires_at_seq"] = serde_json::json!(4);
        let intent = serde_json::json!({
            "action": "acquire",
            "proposed_active_lease": invalid
        });

        assert!(serde_json::from_value::<AssessmentLeaseCasIntent>(intent).is_err());
    }

    #[test]
    fn legacy_assessment_lease_shape_remains_unchanged() {
        let legacy = legacy_lease_json();
        let decoded: AssessmentLease = serde_json::from_value(legacy.clone()).unwrap();
        let encoded = serde_json::to_value(decoded).unwrap();

        assert_eq!(encoded, legacy);
        assert!(encoded.get("expected_active_lease").is_none());
        assert!(encoded.get("action").is_none());
    }
}
