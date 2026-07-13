//! Durable belief storage.
//!
//! The store keeps evidence and revisions append-only, while current heads and
//! views are indexes over those records. This mirrors graph storage: history is
//! preserved, and current state is rebuilt from durable records.
//!
//! # Example
//!
//! ```rust,no_run
//! use meld_world_model::belief::BeliefStore;
//!
//! let temp = tempfile::tempdir().unwrap();
//! let db = sled::open(temp.path()).unwrap();
//! let store = BeliefStore::new(db).unwrap();
//! store.flush().unwrap();
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::ops::Bound;
use std::sync::{Arc, OnceLock, Weak};

#[cfg(test)]
use parking_lot::Condvar;
use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use sled::transaction::Transactional;
use sled::{Db, Tree};
use uuid::Uuid;

use crate::agent::AgentHydrationFenceCapability;
use crate::belief::contracts::{
    AssessmentAssignmentCursor, AssessmentLease, AssessmentLeaseCasIntent,
    BeliefAuthorityMigrationIdentity, BeliefAuthorityMigrationMarker,
    BeliefAuthorityMigrationProgress, BeliefAuthorityParityReceipt, BeliefAuthoritySnapshot,
    BeliefCommitIntent, BeliefCommitRecoveryDisposition, BeliefKey, BeliefProvenanceSummary,
    BeliefRevision, BeliefStatus, BeliefView, DirtyKeyState, DirtyReason, EvidenceAssignment,
    EvidenceConsumerCursor, EvidenceIngestionReceipt, EvidenceIngestionReceiptWriteDisposition,
    EvidenceItem, EvidenceRejection, FreshnessReason, HydrationRefs, LeaseStatus,
    LegacyBeliefCompatibilityPosture, ObservationOpportunity, ObservationReason,
};
use crate::belief::readiness::{
    hash_readiness_view, BeliefReadinessAttestation, BeliefReadinessAttestationRequest,
    BeliefReadinessSnapshot,
};
use crate::belief::BeliefConfigSnapshotFence;
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::{PerspectiveKey, TraversalQuery};

const TREE_EVIDENCE: &str = "belief_evidence";
const TREE_ASSIGNMENTS: &str = "belief_assignments";
const TREE_ASSIGNMENT_GENERATIONS: &str = "belief_assignment_generations";
const TREE_ASSIGNMENTS_BY_KEY: &str = "belief_assignments_by_key";
const TREE_ASSIGNMENT_TAIL_BY_KEY: &str = "belief_assignment_tail_by_key";
const TREE_REVISIONS: &str = "belief_revisions";
const TREE_COMMITTED_EVIDENCE: &str = "belief_committed_evidence";
const TREE_REVISION_HEAD: &str = "belief_revision_head";
const TREE_VIEWS: &str = "belief_views";
const TREE_VIEW_BY_SUBJECT: &str = "belief_view_by_subject";
const TREE_LEASES: &str = "belief_leases";
const TREE_ACTIVE_LEASE: &str = "belief_active_lease";
const TREE_REJECTIONS: &str = "belief_rejections";
const TREE_CONFIG_SNAPSHOTS: &str = "belief_config_snapshots";
const TREE_DIRTY_KEYS: &str = "belief_dirty_keys";
const TREE_RUNTIME_META: &str = "belief_runtime_meta";
const TREE_EVIDENCE_CONSUMER_CURSORS: &str = "belief_evidence_consumer_cursors";
const TREE_EVIDENCE_INGESTION_RECEIPTS: &str = "belief_evidence_ingestion_receipts";
const TREE_AUTHORITY_META: &str = "belief_authority_meta";
const TREE_AUTHORITY_MIGRATION: &str = "belief_authority_migration";
const TREE_COMMIT_INTENTS: &str = "belief_commit_intents";
const TREE_COMMIT_INTENT_BY_LEASE: &str = "belief_commit_intent_by_lease";
const TREE_COMMIT_RECEIPTS: &str = "belief_commit_receipts";
const TREE_LEGACY_ASSESSMENT_RECEIPTS: &str = "belief_legacy_assessment_receipts";
const TREE_READINESS_ATTESTATIONS: &str = "belief_readiness_attestations";
const TREE_READINESS_ATTESTATION_INTENTS: &str = "belief_readiness_attestation_intents";
const TREE_READINESS_ATTESTATION_OWNER_FENCES: &str = "belief_readiness_attestation_owner_fences";
const TREE_READINESS_ATTESTATION_SNAPSHOTS: &str = "belief_readiness_attestation_snapshots";
const TREE_READINESS_SCHEMA: &str = "belief_readiness_schema";
const KEY_READINESS_SCHEMA_STATE: &[u8] = b"state";
const READINESS_SCHEMA_VERSION: u16 = 2;
const READINESS_MIGRATION_BATCH: usize = 128;
const KEY_PRODUCT_AUTHORITY_ID: &[u8] = b"product_authority_id";
const KEY_AUTHORITY_MIGRATION_MARKER: &[u8] = b"marker";
const KEY_LEGACY_WRITE_FENCE: &[u8] = b"legacy_write_fence";
const KEY_STORE_INSTANCE_ID: &[u8] = b"store_instance_id";
const KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE: &[u8] = b"legacy_assessment_migration_fence";
const KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE: &[u8] =
    b"\0legacy_assessment_migration_fence";
const KEY_ASSESSMENT_SOURCE_HIGH_WATER: &[u8] = b"assessment_source_high_water";
const KEY_ASSESSMENT_PROGRESS_SEQUENCE: &[u8] = b"assessment_progress_sequence";
const KEY_ASSESSMENT_LEASE_CLOCK: &[u8] = b"assessment_lease_clock";
const KEY_ASSESSMENT_DIRTY_CURSOR: &[u8] = b"assessment_dirty_cursor";
const LEGACY_ASSESSMENT_RECEIPT_SCHEMA_VERSION: u32 = 1;
const LEGACY_ASSESSMENT_MIGRATION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReadinessMigrationPhase {
    Intents,
    Visible,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadinessMigrationState {
    schema_version: u16,
    phase: ReadinessMigrationPhase,
    cursor: Option<Vec<u8>>,
}

impl ReadinessMigrationState {
    fn initial() -> Self {
        Self {
            schema_version: READINESS_SCHEMA_VERSION,
            phase: ReadinessMigrationPhase::Intents,
            cursor: None,
        }
    }

    fn complete() -> Self {
        Self {
            schema_version: READINESS_SCHEMA_VERSION,
            phase: ReadinessMigrationPhase::Complete,
            cursor: None,
        }
    }

    fn validate(&self) -> Result<(), StorageError> {
        if self.schema_version != READINESS_SCHEMA_VERSION
            || self.phase == ReadinessMigrationPhase::Complete && self.cursor.is_some()
        {
            return Err(StorageError::MigrationConflict(
                "belief readiness schema marker is invalid".to_string(),
            ));
        }
        Ok(())
    }
}

static WRITE_GATES: OnceLock<Mutex<BTreeMap<String, Weak<RwLock<()>>>>> = OnceLock::new();

#[cfg(any(test, feature = "test-support"))]
#[derive(Default)]
struct FlushProbe {
    calls: usize,
    fail_next: bool,
    fail_at_call: Option<usize>,
}

#[cfg(test)]
#[derive(Default)]
struct MigrationCopyProbe {
    state: Mutex<MigrationCopyProbeState>,
    changed: Condvar,
}

#[cfg(test)]
#[derive(Default)]
struct MigrationCopyProbeState {
    pause_after_gate: bool,
    entered: bool,
    released: bool,
}

#[cfg(test)]
#[derive(Default)]
struct CommitIntentProbe {
    state: Mutex<CommitIntentProbeState>,
    changed: Condvar,
}

#[cfg(test)]
#[derive(Default)]
struct CommitIntentProbeState {
    pause_after_prepare: bool,
    entered: bool,
    released: bool,
}

#[cfg(test)]
#[derive(Default)]
struct LegacyFenceProbe {
    state: Mutex<LegacyFenceProbeState>,
    changed: Condvar,
}

#[cfg(test)]
#[derive(Default)]
struct LegacyFenceProbeState {
    armed: bool,
    attempted: bool,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct LegacyAssessmentReceipt {
    schema_version: u32,
    revision: BeliefRevision,
    completed_lease: AssessmentLease,
    legacy_progress_sequence: u64,
}

/// Durable cutover proof for the pre-progress assessment receipt format.
// TODO compat-shim: remove this fence and legacy receipt reader after the
// minimum supported belief store version always carries canonical commit and
// progress receipts. Keep base-format reopen and post-cutover fail-closed
// proofs green before deleting the compatibility path.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct LegacyAssessmentMigrationFence {
    schema_version: u32,
    source_snapshot: BeliefAuthoritySnapshot,
    migrated_revision_ids: Vec<String>,
}

/// Sled-backed store for belief evidence, revisions, views, and leases.
#[derive(Clone)]
pub struct BeliefStore {
    db: Db,
    evidence: Tree,
    assignments: Tree,
    assignment_generations: Tree,
    assignments_by_key: Tree,
    assignment_tail_by_key: Tree,
    revisions: Tree,
    committed_evidence: Tree,
    revision_head: Tree,
    views: Tree,
    view_by_subject: Tree,
    leases: Tree,
    active_lease: Tree,
    rejections: Tree,
    config_snapshots: Tree,
    dirty_keys: Tree,
    runtime_meta: Tree,
    evidence_consumer_cursors: Tree,
    evidence_ingestion_receipts: Tree,
    authority_meta: Tree,
    authority_migration: Tree,
    commit_intents: Tree,
    commit_intent_by_lease: Tree,
    commit_receipts: Tree,
    legacy_assessment_receipts: Tree,
    readiness_attestations: Tree,
    readiness_attestation_intents: Tree,
    readiness_attestation_owner_fences: Tree,
    readiness_attestation_snapshots: Tree,
    readiness_schema: Tree,
    write_gate: Arc<RwLock<()>>,
    #[cfg(any(test, feature = "test-support"))]
    flush_probe: Arc<Mutex<FlushProbe>>,
    #[cfg(test)]
    migration_copy_probe: Arc<MigrationCopyProbe>,
    #[cfg(test)]
    commit_intent_probe: Arc<CommitIntentProbe>,
    #[cfg(test)]
    legacy_fence_probe: Arc<LegacyFenceProbe>,
}

/// Narrow capability for resuming one exact incomplete authority migration.
pub struct BeliefAuthorityMigrationRecovery {
    store: BeliefStore,
    identity: BeliefAuthorityMigrationIdentity,
}

impl BeliefAuthorityMigrationRecovery {
    /// Resume the bound migration until a terminal compatibility posture.
    pub fn resume(
        &self,
        legacy: &BeliefStore,
    ) -> Result<LegacyBeliefCompatibilityPosture, StorageError> {
        self.store
            .migrate_legacy_authority(legacy, self.identity.clone())
    }

    /// Flush recovery state before releasing the migration coordinator.
    pub fn flush(&self) -> Result<(), StorageError> {
        self.store.flush()
    }
}

/// One bounded cyclic page of durable dirty belief work.
pub(crate) struct BeliefDirtyWorkPage {
    /// Cursor value that must still be current before progress is acknowledged.
    pub expected_cursor: Option<Vec<u8>>,
    /// Dirty records selected in cyclic key order.
    pub items: Vec<DirtyKeyState>,
    /// True when another candidate existed beyond this page.
    pub has_more: bool,
}

/// One bounded evidence window selected for a dirty belief key.
pub(crate) struct BeliefEvidenceWindow {
    /// Uncommitted evidence admitted to this comparator run.
    pub evidence: Vec<EvidenceItem>,
    /// Highest complete source sequence inspected by this window.
    pub source_cursor_end: u64,
    /// Exact cursor before this selection.
    pub cursor_start: Option<AssessmentAssignmentCursor>,
    /// Last assignment durably inspected by this selection.
    pub cursor_end: Option<AssessmentAssignmentCursor>,
    /// True when every assignment admitted by this dirty generation was inspected.
    pub complete: bool,
}

struct LegacyCopyCheckpoint<'a> {
    marker: &'a BeliefAuthorityMigrationMarker,
    source: &'a BeliefAuthoritySnapshot,
    starting_count: u64,
}

impl BeliefStore {
    /// Open all belief trees against a shared sled database.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        let authority_meta = db.open_tree(TREE_AUTHORITY_META).map_err(to_storage_io)?;
        let authority_migration = db
            .open_tree(TREE_AUTHORITY_MIGRATION)
            .map_err(to_storage_io)?;
        require_product_authority_available(&authority_meta, &authority_migration)?;
        let store = Self::open_unreconciled(db, authority_meta, authority_migration)?;
        store.reconcile_exclusively_on_open()?;
        store.migrate_legacy_readiness_attestations()?;
        Ok(store)
    }

    /// Open only the exact incomplete migration bound to `identity`.
    ///
    /// The returned capability exposes migration recovery and durability only.
    /// Product readers and writers must continue to use `new`.
    pub fn open_for_authority_migration(
        db: Db,
        identity: BeliefAuthorityMigrationIdentity,
    ) -> Result<BeliefAuthorityMigrationRecovery, StorageError> {
        identity.validate()?;
        let authority_meta = db.open_tree(TREE_AUTHORITY_META).map_err(to_storage_io)?;
        let authority_migration = db
            .open_tree(TREE_AUTHORITY_MIGRATION)
            .map_err(to_storage_io)?;
        require_exact_preterminal_authority_migration(
            &authority_meta,
            &authority_migration,
            &identity,
        )?;
        let store = Self::open_unreconciled(db, authority_meta, authority_migration)?;
        {
            let _exclusive = store.write_gate.write();
            require_exact_preterminal_authority_migration(
                &store.authority_meta,
                &store.authority_migration,
                &identity,
            )?;
        }
        Ok(BeliefAuthorityMigrationRecovery { store, identity })
    }

    fn open_unreconciled(
        db: Db,
        authority_meta: Tree,
        authority_migration: Tree,
    ) -> Result<Self, StorageError> {
        let write_gate_identity = load_or_create_store_instance_id(&db, &authority_meta)?;
        let write_gate = shared_write_gate(&write_gate_identity);
        Ok(Self {
            evidence: db.open_tree(TREE_EVIDENCE).map_err(to_storage_io)?,
            assignments: db.open_tree(TREE_ASSIGNMENTS).map_err(to_storage_io)?,
            assignment_generations: db
                .open_tree(TREE_ASSIGNMENT_GENERATIONS)
                .map_err(to_storage_io)?,
            assignments_by_key: db
                .open_tree(TREE_ASSIGNMENTS_BY_KEY)
                .map_err(to_storage_io)?,
            assignment_tail_by_key: db
                .open_tree(TREE_ASSIGNMENT_TAIL_BY_KEY)
                .map_err(to_storage_io)?,
            revisions: db.open_tree(TREE_REVISIONS).map_err(to_storage_io)?,
            committed_evidence: db
                .open_tree(TREE_COMMITTED_EVIDENCE)
                .map_err(to_storage_io)?,
            revision_head: db.open_tree(TREE_REVISION_HEAD).map_err(to_storage_io)?,
            views: db.open_tree(TREE_VIEWS).map_err(to_storage_io)?,
            view_by_subject: db.open_tree(TREE_VIEW_BY_SUBJECT).map_err(to_storage_io)?,
            leases: db.open_tree(TREE_LEASES).map_err(to_storage_io)?,
            active_lease: db.open_tree(TREE_ACTIVE_LEASE).map_err(to_storage_io)?,
            rejections: db.open_tree(TREE_REJECTIONS).map_err(to_storage_io)?,
            config_snapshots: db.open_tree(TREE_CONFIG_SNAPSHOTS).map_err(to_storage_io)?,
            dirty_keys: db.open_tree(TREE_DIRTY_KEYS).map_err(to_storage_io)?,
            runtime_meta: db.open_tree(TREE_RUNTIME_META).map_err(to_storage_io)?,
            evidence_consumer_cursors: db
                .open_tree(TREE_EVIDENCE_CONSUMER_CURSORS)
                .map_err(to_storage_io)?,
            evidence_ingestion_receipts: db
                .open_tree(TREE_EVIDENCE_INGESTION_RECEIPTS)
                .map_err(to_storage_io)?,
            authority_meta,
            authority_migration,
            commit_intents: db.open_tree(TREE_COMMIT_INTENTS).map_err(to_storage_io)?,
            commit_intent_by_lease: db
                .open_tree(TREE_COMMIT_INTENT_BY_LEASE)
                .map_err(to_storage_io)?,
            commit_receipts: db.open_tree(TREE_COMMIT_RECEIPTS).map_err(to_storage_io)?,
            legacy_assessment_receipts: db
                .open_tree(TREE_LEGACY_ASSESSMENT_RECEIPTS)
                .map_err(to_storage_io)?,
            readiness_attestations: db
                .open_tree(TREE_READINESS_ATTESTATIONS)
                .map_err(to_storage_io)?,
            readiness_attestation_intents: db
                .open_tree(TREE_READINESS_ATTESTATION_INTENTS)
                .map_err(to_storage_io)?,
            readiness_attestation_owner_fences: db
                .open_tree(TREE_READINESS_ATTESTATION_OWNER_FENCES)
                .map_err(to_storage_io)?,
            readiness_attestation_snapshots: db
                .open_tree(TREE_READINESS_ATTESTATION_SNAPSHOTS)
                .map_err(to_storage_io)?,
            readiness_schema: db.open_tree(TREE_READINESS_SCHEMA).map_err(to_storage_io)?,
            write_gate,
            #[cfg(any(test, feature = "test-support"))]
            flush_probe: Arc::new(Mutex::new(FlushProbe::default())),
            #[cfg(test)]
            migration_copy_probe: Arc::new(MigrationCopyProbe::default()),
            #[cfg(test)]
            commit_intent_probe: Arc::new(CommitIntentProbe::default()),
            #[cfg(test)]
            legacy_fence_probe: Arc::new(LegacyFenceProbe::default()),
            db,
        })
    }

    /// Open the store behind an `Arc` for runtime assembly.
    pub fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        Ok(Arc::new(Self::new(db)?))
    }

    fn reconcile_exclusively_on_open(&self) -> Result<(), StorageError> {
        // Normal mutation takes a shared gate so migration can drain writers.
        // Opening another handle takes the exclusive side while scanning and
        // repairing cross-tree indexes. The private view prevents recursive
        // acquisition when existing reconciliation helpers perform writes.
        let reconciliation_view = Self {
            write_gate: Arc::new(RwLock::new(())),
            ..self.clone()
        };
        let _exclusive = self.write_gate.write();
        if let Some(marker) = preterminal_authority_migration(
            &reconciliation_view.authority_meta,
            &reconciliation_view.authority_migration,
        )? {
            return Err(incomplete_authority_unavailable(&marker));
        }
        reconciliation_view.reconcile_commit_intent_index()?;
        reconciliation_view.reconcile_open_commits()?;
        reconciliation_view.reconcile_assessment_indexes()?;
        Ok(())
    }

    fn writable_guard(&self) -> Result<RwLockReadGuard<'_, ()>, StorageError> {
        let guard = self.write_gate.read();
        if self
            .authority_meta
            .get(KEY_LEGACY_WRITE_FENCE)
            .map_err(to_storage_io)?
            .is_some()
        {
            return Err(StorageError::InvalidPath(
                "legacy belief authority is frozen for product cutover".to_string(),
            ));
        }
        if let Some(marker) =
            preterminal_authority_migration(&self.authority_meta, &self.authority_migration)?
        {
            return Err(incomplete_authority_unavailable(&marker));
        }
        Ok(guard)
    }

    fn migration_writable_guard(
        &self,
        identity: &BeliefAuthorityMigrationIdentity,
        allow_unbound_target: bool,
    ) -> Result<RwLockReadGuard<'_, ()>, StorageError> {
        let guard = self.write_gate.read();
        if self
            .authority_meta
            .get(KEY_LEGACY_WRITE_FENCE)
            .map_err(to_storage_io)?
            .is_some()
        {
            return Err(StorageError::InvalidPath(
                "legacy belief authority is frozen for product cutover".to_string(),
            ));
        }
        let bound = self
            .authority_meta
            .get(KEY_PRODUCT_AUTHORITY_ID)
            .map_err(to_storage_io)?;
        match bound {
            Some(bound) if bound.as_ref() != identity.target_authority_id().as_bytes() => {
                return Err(StorageError::MigrationConflict(
                    "belief migration coordinator authority binding changed".to_string(),
                ));
            }
            None if !allow_unbound_target => {
                return Err(StorageError::MigrationConflict(
                    "belief migration coordinator lost its target authority binding".to_string(),
                ));
            }
            _ => {}
        }
        if let Some(marker) = self.authority_migration_marker()? {
            marker.validate()?;
            if marker.migration() != identity {
                return Err(StorageError::MigrationConflict(
                    "belief migration coordinator identity changed".to_string(),
                ));
            }
        }
        Ok(guard)
    }

    fn fence_legacy_authority(
        &self,
        identity: &BeliefAuthorityMigrationIdentity,
    ) -> Result<BeliefAuthoritySnapshot, StorageError> {
        #[cfg(test)]
        self.signal_legacy_fence_attempt_for_test();
        let _guard = self.write_gate.write();
        let encoded = serde_json::to_vec(identity).map_err(to_storage_data)?;
        let current = self
            .authority_meta
            .get(KEY_LEGACY_WRITE_FENCE)
            .map_err(to_storage_io)?;
        match current.as_deref() {
            Some(existing) if existing == encoded.as_slice() => {
                self.require_no_staged_commit_authority()?;
                self.flush()?
            }
            Some(_) => {
                return Err(StorageError::InvalidPath(
                    "legacy belief authority is fenced by another migration".to_string(),
                ));
            }
            None => {
                // The exclusive live gate drains every write admitted before
                // migration. Finalizing staged commits before the fence makes
                // the source snapshot a closed prefix that cannot drift later.
                self.reconcile_before_legacy_fence_exclusively()?;
                match self
                    .authority_meta
                    .compare_and_swap(
                        KEY_LEGACY_WRITE_FENCE,
                        None as Option<&[u8]>,
                        Some(encoded.as_slice()),
                    )
                    .map_err(to_storage_io)?
                {
                    Ok(()) => self.flush()?,
                    Err(error) if error.current.as_deref() == Some(encoded.as_slice()) => {
                        self.flush()?
                    }
                    Err(_) => {
                        return Err(StorageError::InvalidPath(
                            "legacy belief authority is fenced by another migration".to_string(),
                        ));
                    }
                }
            }
        }
        self.authority_snapshot()
    }

    fn reconcile_before_legacy_fence_exclusively(&self) -> Result<(), StorageError> {
        // The caller owns the real exclusive gate. Recovery helpers use this
        // private gate so their ordinary admission checks cannot recursively
        // queue behind the migration writer.
        let reconciliation_view = Self {
            write_gate: Arc::new(RwLock::new(())),
            ..self.clone()
        };
        reconciliation_view.reconcile_commit_intent_index()?;
        reconciliation_view.reconcile_open_commits()?;
        if !reconciliation_view.commit_intents.is_empty() {
            // One pass can abandon an expired lease. The second pass removes
            // its now-abandoned intent without weakening lease ownership.
            reconciliation_view.reconcile_open_commits()?;
        }
        reconciliation_view.reconcile_assessment_indexes()?;
        reconciliation_view.require_no_staged_commit_authority()?;
        reconciliation_view.flush()
    }

    fn require_no_staged_commit_authority(&self) -> Result<(), StorageError> {
        if self.commit_intents.is_empty() && self.commit_intent_by_lease.is_empty() {
            return Ok(());
        }
        Err(StorageError::MigrationConflict(
            "legacy belief authority still has staged commits at its fence boundary".to_string(),
        ))
    }

    /// Return the durable product authority identity, when bound.
    pub fn product_authority_id(&self) -> Result<Option<String>, StorageError> {
        let Some(raw) = self
            .authority_meta
            .get(KEY_PRODUCT_AUTHORITY_ID)
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        Ok(Some(
            String::from_utf8(raw.to_vec()).map_err(to_storage_utf8)?,
        ))
    }

    /// Bind this database to exactly one canonical product authority identity.
    pub fn bind_product_authority(&self, authority_id: &str) -> Result<(), StorageError> {
        let _write = self.writable_guard()?;
        self.bind_product_authority_admitted(authority_id)
    }

    fn bind_product_authority_for_migration(
        &self,
        identity: &BeliefAuthorityMigrationIdentity,
    ) -> Result<(), StorageError> {
        let _write = self.migration_writable_guard(identity, true)?;
        self.bind_product_authority_admitted(identity.target_authority_id())
    }

    fn bind_product_authority_admitted(&self, authority_id: &str) -> Result<(), StorageError> {
        if authority_id.trim().is_empty() {
            return Err(StorageError::InvalidPath(
                "belief product authority id cannot be empty".to_string(),
            ));
        }
        match self
            .authority_meta
            .compare_and_swap(
                KEY_PRODUCT_AUTHORITY_ID,
                None as Option<&[u8]>,
                Some(authority_id.as_bytes()),
            )
            .map_err(to_storage_io)?
        {
            Ok(()) => self.flush(),
            Err(error) if error.current.as_deref() == Some(authority_id.as_bytes()) => self.flush(),
            Err(_) => Err(StorageError::InvalidPath(
                "belief database is already bound to a different product authority".to_string(),
            )),
        }
    }

    /// Read the durable legacy-to-product migration marker.
    pub fn authority_migration_marker(
        &self,
    ) -> Result<Option<BeliefAuthorityMigrationMarker>, StorageError> {
        decode_optional(
            self.authority_migration
                .get(KEY_AUTHORITY_MIGRATION_MARKER)
                .map_err(to_storage_io)?,
        )
    }

    /// Persist and flush one coordinator-owned migration marker checkpoint.
    fn put_authority_migration_marker(
        &self,
        marker: &BeliefAuthorityMigrationMarker,
    ) -> Result<(), StorageError> {
        let _write = self.migration_writable_guard(marker.migration(), false)?;
        marker.validate()?;
        let encoded = serde_json::to_vec(marker).map_err(to_storage_data)?;
        let current = self
            .authority_migration
            .get(KEY_AUTHORITY_MIGRATION_MARKER)
            .map_err(to_storage_io)?;
        if current.as_deref() == Some(encoded.as_slice()) {
            return self.flush();
        }
        if let Some(current) = &current {
            let current: BeliefAuthorityMigrationMarker =
                serde_json::from_slice(current).map_err(to_storage_data)?;
            validate_migration_marker_successor(&current, marker)?;
        } else if !matches!(
            marker.progress(),
            BeliefAuthorityMigrationProgress::Prepared { .. }
        ) {
            return Err(StorageError::InvalidPath(
                "belief migration must begin with a prepared marker".to_string(),
            ));
        }
        match self
            .authority_migration
            .compare_and_swap(
                KEY_AUTHORITY_MIGRATION_MARKER,
                current.as_deref(),
                Some(encoded.as_slice()),
            )
            .map_err(to_storage_io)?
        {
            Ok(()) => self.flush(),
            Err(_) => Err(StorageError::Backpressure(
                "belief migration marker changed concurrently".to_string(),
            )),
        }
    }

    /// Compute the canonical snapshot over every migrated belief record.
    pub fn authority_snapshot(&self) -> Result<BeliefAuthoritySnapshot, StorageError> {
        let mut hasher = blake3::Hasher::new();
        let mut record_count = 0u64;
        for (tree_name, tree) in self.migrated_trees() {
            for item in tree.iter() {
                let (key, value) = item.map_err(to_storage_io)?;
                hash_snapshot_part(&mut hasher, tree_name.as_bytes());
                hash_snapshot_part(&mut hasher, &key);
                hash_snapshot_part(&mut hasher, &value);
                record_count = record_count.saturating_add(1);
            }
        }
        BeliefAuthoritySnapshot::try_new(hasher.finalize().to_hex().to_string(), record_count)
    }

    /// Resume a frozen legacy authority migration into this product authority.
    pub fn migrate_legacy_authority(
        &self,
        legacy: &BeliefStore,
        identity: BeliefAuthorityMigrationIdentity,
    ) -> Result<LegacyBeliefCompatibilityPosture, StorageError> {
        loop {
            let marker = self.advance_legacy_authority_migration(legacy, identity.clone())?;
            match marker.progress() {
                BeliefAuthorityMigrationProgress::Cutover { .. } => {
                    return Ok(if !legacy.has_migrated_product_records()? {
                        LegacyBeliefCompatibilityPosture::NoLegacyState
                    } else {
                        LegacyBeliefCompatibilityPosture::ProductAuthoritative
                    });
                }
                BeliefAuthorityMigrationProgress::ForwardRepairOnly { .. } => {
                    return Ok(LegacyBeliefCompatibilityPosture::ForwardRepairOnly);
                }
                _ => {}
            }
        }
    }

    /// Advance one durable legacy migration state for bounded recovery.
    pub fn advance_legacy_authority_migration(
        &self,
        legacy: &BeliefStore,
        identity: BeliefAuthorityMigrationIdentity,
    ) -> Result<BeliefAuthorityMigrationMarker, StorageError> {
        identity.validate()?;
        if Arc::ptr_eq(&self.write_gate, &legacy.write_gate) {
            return Err(StorageError::InvalidPath(
                "belief authority migration requires distinct live databases".to_string(),
            ));
        }
        let source = legacy.fence_legacy_authority(&identity)?;
        let migration_view = Self {
            write_gate: Arc::new(RwLock::new(())),
            ..self.clone()
        };
        let _exclusive = self.write_gate.write();
        #[cfg(test)]
        self.pause_migration_copy_after_gate_for_test();
        migration_view.advance_legacy_authority_migration_exclusively(legacy, identity, source)
    }

    fn advance_legacy_authority_migration_exclusively(
        &self,
        legacy: &BeliefStore,
        identity: BeliefAuthorityMigrationIdentity,
        source: BeliefAuthoritySnapshot,
    ) -> Result<BeliefAuthorityMigrationMarker, StorageError> {
        self.bind_product_authority_for_migration(&identity)?;
        let current = match self.authority_migration_marker()? {
            Some(marker) if marker.migration() == &identity => marker,
            Some(_) => {
                return Err(StorageError::InvalidPath(
                    "belief authority migration identity conflict".to_string(),
                ));
            }
            None => {
                let prepared = BeliefAuthorityMigrationMarker::try_new(
                    identity,
                    BeliefAuthorityMigrationProgress::Prepared { source },
                )?;
                self.put_authority_migration_marker(&prepared)?;
                return Ok(prepared);
            }
        };
        let next = match current.progress().clone() {
            BeliefAuthorityMigrationProgress::Prepared {
                source: frozen_source,
            } => {
                require_frozen_source(legacy, &frozen_source)?;
                BeliefAuthorityMigrationMarker::try_new(
                    current.migration().clone(),
                    BeliefAuthorityMigrationProgress::Copying {
                        source: frozen_source,
                        verified_record_count: 0,
                    },
                )?
            }
            BeliefAuthorityMigrationProgress::Copying {
                source: frozen_source,
                ..
            } => {
                require_frozen_source(legacy, &frozen_source)?;
                self.copy_legacy_records(legacy, &current, &frozen_source)?;
                require_frozen_source(legacy, &frozen_source)?;
                let target = self.authority_snapshot()?;
                let parity = BeliefAuthorityParityReceipt::try_new(frozen_source, target)?;
                BeliefAuthorityMigrationMarker::try_new(
                    current.migration().clone(),
                    BeliefAuthorityMigrationProgress::Verified { parity },
                )?
            }
            BeliefAuthorityMigrationProgress::Verified { parity } => {
                require_migration_parity(legacy, self, &parity)?;
                BeliefAuthorityMigrationMarker::try_new(
                    current.migration().clone(),
                    BeliefAuthorityMigrationProgress::Cutover { parity },
                )?
            }
            BeliefAuthorityMigrationProgress::Cutover { parity } => {
                require_frozen_source(legacy, parity.source())?;
                if self.authority_snapshot()? == *parity.target() {
                    return Ok(current);
                }
                BeliefAuthorityMigrationMarker::try_new(
                    current.migration().clone(),
                    BeliefAuthorityMigrationProgress::ForwardRepairOnly { parity },
                )?
            }
            BeliefAuthorityMigrationProgress::ForwardRepairOnly { .. } => return Ok(current),
        };
        self.put_authority_migration_marker(&next)?;
        Ok(next)
    }

    /// Append or replace a deterministic evidence record by id.
    pub fn put_evidence(&self, item: &EvidenceItem) -> Result<(), StorageError> {
        let _write = self.writable_guard()?;
        self.evidence
            .insert(
                item.evidence_id.as_bytes(),
                serde_json::to_vec(item).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Store an evidence record only when its deterministic id is new.
    pub fn put_evidence_once(&self, item: &EvidenceItem) -> Result<bool, StorageError> {
        let _write = self.writable_guard()?;
        if let Some(existing) = self.get_evidence(&item.evidence_id)? {
            if existing == *item {
                return Ok(false);
            }
            return Err(StorageError::InvalidPath(format!(
                "evidence conflict for '{}'",
                item.evidence_id
            )));
        }

        self.evidence
            .insert(
                item.evidence_id.as_bytes(),
                serde_json::to_vec(item).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        Ok(true)
    }

    /// Read one evidence item by id.
    pub fn get_evidence(&self, evidence_id: &str) -> Result<Option<EvidenceItem>, StorageError> {
        decode_optional(
            self.evidence
                .get(evidence_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Store an evidence assignment and mark the key dirty.
    pub fn put_assignment(&self, assignment: &EvidenceAssignment) -> Result<(), StorageError> {
        self.put_assignment_once(assignment)?;
        Ok(())
    }

    /// Read one evidence assignment by id.
    pub fn get_assignment(
        &self,
        assignment_id: &str,
    ) -> Result<Option<EvidenceAssignment>, StorageError> {
        decode_optional(
            self.assignments
                .get(assignment_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Store an evidence assignment only when its deterministic id is new.
    pub fn put_assignment_once(
        &self,
        assignment: &EvidenceAssignment,
    ) -> Result<bool, StorageError> {
        let _write = self.writable_guard()?;
        let assignment_bytes = serde_json::to_vec(assignment).map_err(to_storage_data)?;
        let key = assignment.belief_key.index_key();
        let candidate_generation = if self
            .assignments
            .get(assignment.assignment_id.as_bytes())
            .map_err(to_storage_io)?
            .is_none()
        {
            self.db
                .generate_id()
                .map_err(to_storage_io)?
                .checked_add(1)
                .ok_or_else(|| {
                    StorageError::InvalidPath(
                        "belief assignment generation is exhausted".to_string(),
                    )
                })?
        } else {
            1
        };
        use sled::transaction::{ConflictableTransactionError, TransactionError};
        let inserted = (
            &self.assignments,
            &self.assignment_generations,
            &self.assignments_by_key,
            &self.assignment_tail_by_key,
            &self.dirty_keys,
            &self.active_lease,
            &self.leases,
        )
            .transaction(
                |(
                    assignments,
                    assignment_generations,
                    assignments_by_key,
                    assignment_tail_by_key,
                    dirty_keys,
                    active_leases,
                    leases,
                )| {
                    if let Some(existing) = assignments.get(assignment.assignment_id.as_bytes())? {
                        if existing.as_ref() == assignment_bytes.as_slice() {
                            let generation = decode_transaction_generation(
                                assignment_generations
                                    .get(assignment.assignment_id.as_bytes())?
                                    .as_deref(),
                            )
                            .map_err(ConflictableTransactionError::Abort)?;
                            let assignment_index_key =
                                assignment_by_key_index_key(assignment, generation);
                            match assignments_by_key.get(assignment_index_key.as_slice())? {
                                Some(indexed)
                                    if indexed.as_ref() == assignment.assignment_id.as_bytes() =>
                                {
                                    return Ok(false);
                                }
                                Some(_) => {
                                    return Err(ConflictableTransactionError::Abort(
                                        "belief assignment key index conflict".to_string(),
                                    ));
                                }
                                None => {
                                    assignments_by_key.insert(
                                        assignment_index_key.as_slice(),
                                        assignment.assignment_id.as_bytes(),
                                    )?;
                                    return Ok(false);
                                }
                            }
                        }
                        return Err(ConflictableTransactionError::Abort(format!(
                            "evidence assignment conflict for '{}'",
                            assignment.assignment_id
                        )));
                    }
                    let active = match active_leases.get(key.as_bytes())? {
                        Some(active_id) => leases
                            .get(active_id.as_ref())?
                            .map(|raw| {
                                serde_json::from_slice::<AssessmentLease>(&raw).map_err(|error| {
                                    ConflictableTransactionError::Abort(format!(
                                        "invalid active lease during assignment: {error}"
                                    ))
                                })
                            })
                            .transpose()?
                            .filter(|lease| lease.status == LeaseStatus::Leased),
                        None => None,
                    };
                    let existing = dirty_keys
                        .get(key.as_bytes())?
                        .map(|raw| {
                            serde_json::from_slice::<DirtyKeyState>(&raw).map_err(|error| {
                                ConflictableTransactionError::Abort(format!(
                                    "invalid dirty state during assignment: {error}"
                                ))
                            })
                        })
                        .transpose()?;
                    let dirty_since_seq = assignment.source_cursor_end;
                    let reason = if active.is_some() {
                        DirtyReason::ActiveLeaseCoalesced
                    } else {
                        DirtyReason::NewEvidence
                    };
                    let state = merge_dirty_state(
                        existing,
                        &assignment.belief_key,
                        dirty_since_seq,
                        assignment.source_cursor_end,
                        active.map(|lease| lease.lease_id),
                        reason,
                        Some(candidate_generation),
                    );
                    let state_bytes = serde_json::to_vec(&state).map_err(|error| {
                        ConflictableTransactionError::Abort(format!(
                            "cannot encode dirty state during assignment: {error}"
                        ))
                    })?;
                    assignments.insert(
                        assignment.assignment_id.as_bytes(),
                        assignment_bytes.as_slice(),
                    )?;
                    assignment_generations.insert(
                        assignment.assignment_id.as_bytes(),
                        &state.mutation_generation.to_be_bytes(),
                    )?;
                    let assignment_index_key =
                        assignment_by_key_index_key(assignment, state.mutation_generation);
                    if assignment_tail_by_key
                        .get(key.as_bytes())?
                        .is_some_and(|current| current.as_ref() >= assignment_index_key.as_slice())
                    {
                        return Err(ConflictableTransactionError::Abort(
                            "belief assignment generation did not advance the key tail".to_string(),
                        ));
                    }
                    assignments_by_key.insert(
                        assignment_index_key.as_slice(),
                        assignment.assignment_id.as_bytes(),
                    )?;
                    assignment_tail_by_key
                        .insert(key.as_bytes(), assignment_index_key.as_slice())?;
                    dirty_keys.insert(key.as_bytes(), state_bytes)?;
                    Ok(true)
                },
            )
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::InvalidPath(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        Ok(inserted)
    }

    /// Read assignments for one belief key in source order.
    pub fn assignments_for_key(
        &self,
        key: &BeliefKey,
    ) -> Result<Vec<EvidenceAssignment>, StorageError> {
        let mut out = Vec::new();
        for item in self
            .assignments_by_key
            .scan_prefix(belief_key_index_prefix(key))
        {
            let (index_key, assignment_id) = item.map_err(to_storage_io)?;
            let assignment_id =
                String::from_utf8(assignment_id.to_vec()).map_err(to_storage_utf8)?;
            let Some(assignment) = self.get_assignment(&assignment_id)? else {
                return Err(StorageError::InvalidPath(
                    "belief assignment key index references a missing assignment".to_string(),
                ));
            };
            let generation = self.assignment_generation(&assignment.assignment_id)?;
            if assignment.belief_key != *key
                || assignment_by_key_index_key(&assignment, generation).as_slice()
                    != index_key.as_ref()
            {
                return Err(StorageError::InvalidPath(
                    "belief assignment key index does not match its assignment".to_string(),
                ));
            }
            out.push(assignment);
        }
        Ok(out)
    }

    fn assignment_generation(&self, assignment_id: &str) -> Result<u64, StorageError> {
        decode_assignment_generation(
            self.assignment_generations
                .get(assignment_id.as_bytes())
                .map_err(to_storage_io)?
                .as_deref(),
        )
    }

    /// Hydrate assigned evidence for one belief key.
    pub fn evidence_for_key(&self, key: &BeliefKey) -> Result<Vec<EvidenceItem>, StorageError> {
        let mut out = Vec::new();
        for assignment in self.assignments_for_key(key)? {
            if let Some(item) = self.get_evidence(&assignment.evidence_id)? {
                out.push(item);
            }
        }
        out.sort_by_key(|item| item.source_cursor_end);
        Ok(out)
    }

    pub(crate) fn evidence_for_dirty_assessment(
        &self,
        key: &BeliefKey,
        dirty: &DirtyKeyState,
        admitted_generation: u64,
        source_cursor_end: u64,
        max_items: usize,
    ) -> Result<BeliefEvidenceWindow, StorageError> {
        if max_items == 0 {
            return Err(StorageError::InvalidPath(
                "belief evidence window limit must be positive".to_string(),
            ));
        }
        let prefix = belief_key_index_prefix(key);
        let tail_index_key = self
            .assignment_tail_by_key
            .get(key.index_key().as_bytes())
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "dirty belief key has no assignment stream tail".to_string(),
                )
            })?;
        if !tail_index_key.starts_with(&prefix)
            || self
                .assignments_by_key
                .get(&tail_index_key)
                .map_err(to_storage_io)?
                .is_none()
        {
            return Err(StorageError::InvalidPath(
                "belief assignment stream tail is missing or divergent".to_string(),
            ));
        }
        let cursor_start = dirty.assessment_cursor.clone();
        let mut iterator = if let Some(cursor) = &cursor_start {
            let cursor_key = assignment_cursor_index_key(&prefix, cursor);
            if self
                .assignments_by_key
                .get(&cursor_key)
                .map_err(to_storage_io)?
                .as_deref()
                != Some(cursor.assignment_id.as_bytes())
            {
                return Err(StorageError::InvalidPath(
                    "belief dirty assessment cursor is missing or divergent".to_string(),
                ));
            }
            self.assignments_by_key.range::<Vec<u8>, _>((
                std::ops::Bound::Excluded(cursor_key),
                std::ops::Bound::Unbounded,
            ))
        } else {
            self.assignments_by_key.range::<Vec<u8>, _>((
                std::ops::Bound::Included(prefix.clone()),
                std::ops::Bound::Unbounded,
            ))
        };
        let mut scanned = 0;
        let mut cursor_end = cursor_start.clone();
        let mut selected_evidence = None;
        while scanned < max_items {
            let Some(item) = iterator.next() else {
                break;
            };
            let (index_key, assignment_id) = item.map_err(to_storage_io)?;
            if !index_key.starts_with(&prefix) {
                break;
            }
            let assignment_id =
                String::from_utf8(assignment_id.to_vec()).map_err(to_storage_utf8)?;
            let Some(assignment) = self.get_assignment(&assignment_id)? else {
                return Err(StorageError::InvalidPath(
                    "belief assignment key index references a missing assignment".to_string(),
                ));
            };
            let generation = self.assignment_generation(&assignment.assignment_id)?;
            if generation > admitted_generation {
                break;
            }
            if assignment.belief_key != *key
                || assignment_by_key_index_key(&assignment, generation).as_slice()
                    != index_key.as_ref()
            {
                return Err(StorageError::InvalidPath(
                    "belief assignment key index does not match its assignment".to_string(),
                ));
            }
            if assignment.source_cursor_end > source_cursor_end {
                return Err(StorageError::InvalidPath(
                    "belief assignment exceeds its dirty source watermark".to_string(),
                ));
            }
            scanned += 1;
            cursor_end = Some(AssessmentAssignmentCursor {
                mutation_generation: generation,
                source_cursor_end: assignment.source_cursor_end,
                assignment_id: assignment.assignment_id.clone(),
            });
            let committed_index_key = committed_evidence_index_key(key, &assignment.evidence_id);
            if let Some(revision_id) = self
                .committed_evidence
                .get(committed_index_key)
                .map_err(to_storage_io)?
            {
                let revision_id =
                    String::from_utf8(revision_id.to_vec()).map_err(to_storage_utf8)?;
                let Some(revision) = self.get_revision(&revision_id)? else {
                    return Err(StorageError::InvalidPath(
                        "belief committed evidence index references a missing revision".to_string(),
                    ));
                };
                if revision.belief_key != *key
                    || !revision.evidence_ids.contains(&assignment.evidence_id)
                {
                    return Err(StorageError::InvalidPath(
                        "belief committed evidence index does not match its revision".to_string(),
                    ));
                }
                continue;
            }
            let Some(item) = self.get_evidence(&assignment.evidence_id)? else {
                return Err(StorageError::InvalidPath(
                    "belief assignment references missing evidence".to_string(),
                ));
            };
            if item.source_cursor_start != assignment.source_cursor_start
                || item.source_cursor_end != assignment.source_cursor_end
                || item.candidate_key != assignment.belief_key
            {
                return Err(StorageError::InvalidPath(
                    "belief assignment does not match its evidence record".to_string(),
                ));
            }
            selected_evidence = Some(item);
            break;
        }
        let selected_end = cursor_end
            .as_ref()
            .map_or(source_cursor_end, |cursor| cursor.source_cursor_end);
        let complete = cursor_end.as_ref().is_some_and(|cursor| {
            assignment_cursor_index_key(&prefix, cursor).as_slice() == tail_index_key.as_ref()
        });
        Ok(BeliefEvidenceWindow {
            evidence: selected_evidence.into_iter().collect(),
            source_cursor_end: selected_end,
            cursor_start,
            cursor_end,
            complete,
        })
    }

    /// Store an audit record for unsupported evidence candidates.
    pub fn put_rejection(&self, rejection: &EvidenceRejection) -> Result<(), StorageError> {
        let _write = self.writable_guard()?;
        self.rejections
            .insert(
                rejection.rejection_id.as_bytes(),
                serde_json::to_vec(rejection).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Read an audit rejection by id.
    pub fn get_rejection(
        &self,
        rejection_id: &str,
    ) -> Result<Option<EvidenceRejection>, StorageError> {
        decode_optional(
            self.rejections
                .get(rejection_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Read one owner-scoped evidence consumer cursor.
    pub fn evidence_consumer_cursor(
        &self,
        identity: &EvidenceConsumerCursor,
    ) -> Result<Option<EvidenceConsumerCursor>, StorageError> {
        decode_optional(
            self.evidence_consumer_cursors
                .get(evidence_cursor_key(identity)?)
                .map_err(to_storage_io)?,
        )
    }

    /// Read the durable receipt for one canonical source record.
    pub fn evidence_ingestion_receipt(
        &self,
        receipt: &EvidenceIngestionReceipt,
    ) -> Result<Option<EvidenceIngestionReceipt>, StorageError> {
        decode_optional(
            self.evidence_ingestion_receipts
                .get(evidence_receipt_key(receipt)?)
                .map_err(to_storage_io)?,
        )
    }

    /// Persist a source receipt and its consumer cursor as one durable unit.
    pub fn record_evidence_receipt_and_advance(
        &self,
        expected: Option<&EvidenceConsumerCursor>,
        receipt: &EvidenceIngestionReceipt,
        next: &EvidenceConsumerCursor,
    ) -> Result<EvidenceIngestionReceiptWriteDisposition, StorageError> {
        let _write = self.writable_guard()?;
        let cursor_key = evidence_cursor_key(next)?;
        let receipt_key = evidence_receipt_key(receipt)?;
        let receipt_bytes = serde_json::to_vec(receipt).map_err(to_storage_data)?;
        validate_receipt_next_identity(receipt, next)?;
        let expected_bytes = expected
            .map(serde_json::to_vec)
            .transpose()
            .map_err(to_storage_data)?;
        let next_bytes = serde_json::to_vec(next).map_err(to_storage_data)?;

        use sled::transaction::{ConflictableTransactionError, TransactionError};
        let disposition = (
            &self.evidence_ingestion_receipts,
            &self.evidence_consumer_cursors,
            &self.runtime_meta,
        )
            .transaction(|(receipts, cursors, runtime_meta)| {
                let stored_receipt = receipts.get(receipt_key.as_slice())?;
                let stored_cursor = cursors.get(cursor_key.as_slice())?;
                if stored_receipt.as_deref() == Some(receipt_bytes.as_slice()) {
                    let stored_cursor = stored_cursor.ok_or_else(|| {
                        ConflictableTransactionError::Abort(
                            "exact evidence receipt is missing its durable consumer cursor"
                                .to_string(),
                        )
                    })?;
                    let current: EvidenceConsumerCursor = serde_json::from_slice(&stored_cursor)
                        .map_err(|error| {
                            ConflictableTransactionError::Abort(format!(
                                "invalid evidence consumer cursor during exact replay: {error}"
                            ))
                        })?;
                    validate_exact_receipt_replay_cursor(receipt, next, &current)
                        .map_err(|error| ConflictableTransactionError::Abort(error.to_string()))?;
                    record_acknowledged_source_high_water(
                        runtime_meta,
                        receipt.identity.source_record.seq,
                    )?;
                    return Ok(EvidenceIngestionReceiptWriteDisposition::ExactReplay);
                }
                if stored_receipt.is_some() {
                    return Err(ConflictableTransactionError::Abort(
                        "evidence ingestion receipt conflict".to_string(),
                    ));
                }
                validate_receipt_cursor_transition(expected, receipt, next)
                    .map_err(|error| ConflictableTransactionError::Abort(error.to_string()))?;
                if stored_cursor.as_deref() != expected_bytes.as_deref() {
                    return Err(ConflictableTransactionError::Abort(
                        "evidence consumer cursor changed".to_string(),
                    ));
                }
                receipts.insert(receipt_key.as_slice(), receipt_bytes.as_slice())?;
                cursors.insert(cursor_key.as_slice(), next_bytes.as_slice())?;
                record_acknowledged_source_high_water(
                    runtime_meta,
                    receipt.identity.source_record.seq,
                )?;
                Ok(EvidenceIngestionReceiptWriteDisposition::Inserted)
            })
            .map_err(|error| match error {
                TransactionError::Abort(message)
                    if message == "evidence consumer cursor changed" =>
                {
                    StorageError::Backpressure(message)
                }
                TransactionError::Abort(message) => StorageError::InvalidPath(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.flush().map_err(|error| {
            StorageError::DurabilityIndeterminate(format!(
                "evidence receipt acknowledgement flush failed: {error}"
            ))
        })?;
        Ok(disposition)
    }

    /// Store the serialized config snapshot used for replay.
    pub fn put_config_snapshot(&self, hash: &str, json: &str) -> Result<(), StorageError> {
        let _write = self.writable_guard()?;
        self.config_snapshots
            .insert(hash.as_bytes(), json.as_bytes())
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Store an immutable config snapshot or accept an exact replay.
    pub fn put_config_snapshot_once(&self, hash: &str, json: &str) -> Result<bool, StorageError> {
        let _write = self.writable_guard()?;
        match self
            .config_snapshots
            .compare_and_swap(
                hash.as_bytes(),
                None as Option<&[u8]>,
                Some(json.as_bytes()),
            )
            .map_err(to_storage_io)?
        {
            Ok(()) => Ok(true),
            Err(error) if error.current.as_deref() == Some(json.as_bytes()) => Ok(false),
            Err(_) => Err(StorageError::InvalidPath(format!(
                "belief config snapshot conflict for '{hash}'"
            ))),
        }
    }

    /// Read a serialized config snapshot by hash.
    pub fn get_config_snapshot(&self, hash: &str) -> Result<Option<String>, StorageError> {
        let Some(raw) = self
            .config_snapshots
            .get(hash.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        Ok(Some(
            String::from_utf8(raw.to_vec()).map_err(to_storage_utf8)?,
        ))
    }

    /// Capture one exact immutable configuration snapshot for a cross-domain fence.
    pub(crate) fn config_snapshot_fence(
        &self,
        hash: &str,
    ) -> Result<Option<BeliefConfigSnapshotFence>, StorageError> {
        Ok(self
            .get_config_snapshot(hash)?
            .map(|json| BeliefConfigSnapshotFence {
                tree: self.config_snapshots.clone(),
                hash: hash.to_string(),
                json,
            }))
    }

    /// Acquire the only active assessment lease for a belief key.
    pub fn acquire_lease(
        &self,
        mut lease: AssessmentLease,
    ) -> Result<AssessmentLease, StorageError> {
        lease.status = LeaseStatus::Leased;
        let acquired = self.apply_assessment_lease_cas(&AssessmentLeaseCasIntent::Acquire {
            proposed_active_lease: lease,
        })?;
        self.flush().map_err(|error| {
            StorageError::DurabilityIndeterminate(format!(
                "belief assessment lease flush failed: {error}"
            ))
        })?;
        Ok(acquired)
    }

    /// Atomically apply one validated assessment lease transition.
    pub(crate) fn apply_assessment_lease_cas(
        &self,
        intent: &AssessmentLeaseCasIntent,
    ) -> Result<AssessmentLease, StorageError> {
        let _write = self.writable_guard()?;
        intent.validate()?;
        let (expected, proposed, retain_active) = match intent {
            AssessmentLeaseCasIntent::Acquire {
                proposed_active_lease,
            } => (None, proposed_active_lease, true),
            AssessmentLeaseCasIntent::Complete {
                expected_active_lease,
                completed_lease,
            } => (Some(expected_active_lease), completed_lease, false),
            AssessmentLeaseCasIntent::AbandonExpired {
                expected_active_lease,
                abandoned_lease,
            } => (Some(expected_active_lease), abandoned_lease, false),
        };
        let key = proposed.belief_key.index_key();
        let expected_bytes = expected
            .map(serde_json::to_vec)
            .transpose()
            .map_err(to_storage_data)?;
        let proposed_bytes = serde_json::to_vec(proposed).map_err(to_storage_data)?;

        use sled::transaction::{ConflictableTransactionError, TransactionError};
        (
            &self.leases,
            &self.active_lease,
            &self.dirty_keys,
            &self.commit_intents,
            &self.commit_intent_by_lease,
        )
            .transaction(
                |(leases, active, dirty_keys, commit_intents, intents_by_lease)| {
                    let stored_active = active.get(key.as_bytes())?;
                    let stored_proposed = leases.get(proposed.lease_id.as_bytes())?;
                    if retain_active
                        && stored_active.as_deref() == Some(proposed.lease_id.as_bytes())
                        && stored_proposed.as_deref() == Some(proposed_bytes.as_slice())
                    {
                        return Ok(());
                    }
                    if !retain_active
                        && stored_active.is_none()
                        && stored_proposed.as_deref() == Some(proposed_bytes.as_slice())
                    {
                        return Ok(());
                    }
                    if retain_active && stored_proposed.is_some() {
                        return Err(ConflictableTransactionError::Abort(
                            "belief lease id already belongs to a durable lease".to_string(),
                        ));
                    }
                    match expected {
                        None if stored_active.is_some() => {
                            return Err(ConflictableTransactionError::Abort(
                                "active belief lease already exists".to_string(),
                            ));
                        }
                        Some(expected)
                            if stored_active.as_deref() != Some(expected.lease_id.as_bytes())
                                || leases.get(expected.lease_id.as_bytes())?.as_deref()
                                    != expected_bytes.as_deref() =>
                        {
                            return Err(ConflictableTransactionError::Abort(
                                "active belief lease changed".to_string(),
                            ));
                        }
                        Some(_) => {}
                        None => {}
                    }
                    if matches!(intent, AssessmentLeaseCasIntent::AbandonExpired { .. }) {
                        let Some(expected) = expected else {
                            return Err(ConflictableTransactionError::Abort(
                                "belief lease abandonment is missing its expected lease"
                                    .to_string(),
                            ));
                        };
                        if let Some(intent_id) =
                            intents_by_lease.get(expected.lease_id.as_bytes())?
                        {
                            let intent_raw =
                                commit_intents.get(intent_id.as_ref())?.ok_or_else(|| {
                                    ConflictableTransactionError::Abort(
                                        "belief lease commit index references a missing intent"
                                            .to_string(),
                                    )
                                })?;
                            let commit_intent: BeliefCommitIntent =
                                serde_json::from_slice(&intent_raw).map_err(|error| {
                                    ConflictableTransactionError::Abort(format!(
                                    "invalid belief commit intent during lease recovery: {error}"
                                ))
                                })?;
                            if commit_intent.expected_active_lease() != expected {
                                return Err(ConflictableTransactionError::Abort(
                                    "belief lease commit index references another lease"
                                        .to_string(),
                                ));
                            }
                            commit_intents.remove(intent_id.as_ref())?;
                            intents_by_lease.remove(expected.lease_id.as_bytes())?;
                        }
                    }
                    leases.insert(proposed.lease_id.as_bytes(), proposed_bytes.as_slice())?;
                    if retain_active {
                        active.insert(key.as_bytes(), proposed.lease_id.as_bytes())?;
                    } else {
                        active.remove(key.as_bytes())?;
                    }
                    if let Some(raw) = dirty_keys.get(key.as_bytes())? {
                        let mut dirty: DirtyKeyState =
                            serde_json::from_slice(&raw).map_err(|error| {
                                ConflictableTransactionError::Abort(format!(
                                    "invalid dirty state during lease CAS: {error}"
                                ))
                            })?;
                        if retain_active {
                            if dirty.mutation_generation == 0 {
                                dirty.mutation_generation = 1;
                            }
                            if dirty.assessment_cursor != proposed.assignment_cursor_start
                                || proposed.epoch > dirty.mutation_generation
                            {
                                return Err(ConflictableTransactionError::Abort(
                                    "belief lease assignment window changed".to_string(),
                                ));
                            }
                            dirty.active_lease_id = Some(proposed.lease_id.clone());
                        } else if matches!(intent, AssessmentLeaseCasIntent::AbandonExpired { .. })
                        {
                            dirty.dirty_since_seq =
                                dirty.dirty_since_seq.min(proposed.input_cursor_end);
                            dirty.latest_seq = dirty.latest_seq.max(proposed.input_cursor_end);
                            dirty.active_lease_id = None;
                            dirty.reason = DirtyReason::LeaseExpired;
                        } else {
                            dirty.active_lease_id = None;
                        }
                        let encoded = serde_json::to_vec(&dirty).map_err(|error| {
                            ConflictableTransactionError::Abort(format!(
                                "cannot encode dirty state during lease CAS: {error}"
                            ))
                        })?;
                        dirty_keys.insert(key.as_bytes(), encoded)?;
                    } else if matches!(intent, AssessmentLeaseCasIntent::AbandonExpired { .. }) {
                        let dirty = DirtyKeyState {
                            belief_key: proposed.belief_key.clone(),
                            dirty_since_seq: proposed.input_cursor_end,
                            latest_seq: proposed.input_cursor_end,
                            mutation_generation: proposed.epoch.max(1),
                            assessment_cursor: proposed.assignment_cursor_start.clone(),
                            active_lease_id: None,
                            reason: DirtyReason::LeaseExpired,
                        };
                        let encoded = serde_json::to_vec(&dirty).map_err(|error| {
                            ConflictableTransactionError::Abort(format!(
                                "cannot encode dirty state during lease CAS: {error}"
                            ))
                        })?;
                        dirty_keys.insert(key.as_bytes(), encoded)?;
                    }
                    Ok(())
                },
            )
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::Backpressure(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        Ok(proposed.clone())
    }

    /// Read a lease by id.
    pub fn get_lease(&self, lease_id: &str) -> Result<Option<AssessmentLease>, StorageError> {
        decode_optional(
            self.leases
                .get(lease_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    pub(crate) fn active_lease_for_key(
        &self,
        key: &BeliefKey,
    ) -> Result<Option<AssessmentLease>, StorageError> {
        let Some(lease_id) = self
            .active_lease
            .get(key.index_key().as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let lease_id = String::from_utf8(lease_id.to_vec()).map_err(to_storage_utf8)?;
        let lease = self.get_lease(&lease_id)?.ok_or_else(|| {
            StorageError::InvalidPath(
                "active belief lease index references a missing lease".to_string(),
            )
        })?;
        if lease.belief_key != *key
            || lease.lease_id != lease_id
            || lease.status != LeaseStatus::Leased
        {
            return Err(StorageError::InvalidPath(
                "active belief lease index is inconsistent".to_string(),
            ));
        }
        Ok(Some(lease))
    }

    /// Complete a lease and release the active lease index.
    pub fn complete_lease(&self, lease: &AssessmentLease) -> Result<(), StorageError> {
        let mut completed = lease.clone();
        completed.status = LeaseStatus::Completed;
        self.apply_assessment_lease_cas(&AssessmentLeaseCasIntent::Complete {
            expected_active_lease: lease.clone(),
            completed_lease: completed,
        })?;
        Ok(())
    }

    /// Complete one empty bounded window and advance or clear its exact dirty state.
    pub(crate) fn complete_empty_assessment_window(
        &self,
        lease: &AssessmentLease,
    ) -> Result<u64, StorageError> {
        let _write = self.writable_guard()?;
        let key = lease.belief_key.index_key();
        let progress_receipt_key = assessment_progress_receipt_key("empty", &lease.lease_id);
        let expected_lease = serde_json::to_vec(lease).map_err(to_storage_data)?;
        let mut completed = lease.clone();
        completed.status = LeaseStatus::Completed;
        let completed_lease = serde_json::to_vec(&completed).map_err(to_storage_data)?;
        if let Some(progress) = self.exact_assessment_progress_receipt(&progress_receipt_key)? {
            if self
                .leases
                .get(lease.lease_id.as_bytes())
                .map_err(to_storage_io)?
                .as_deref()
                != Some(completed_lease.as_slice())
            {
                return Err(StorageError::InvalidPath(
                    "empty belief assessment receipt conflicts with its terminal lease".to_string(),
                ));
            }
            self.flush().map_err(|error| {
                StorageError::DurabilityIndeterminate(format!(
                    "empty belief assessment exact replay flush failed: {error}"
                ))
            })?;
            return Ok(progress);
        }
        use sled::transaction::{ConflictableTransactionError, TransactionError};
        let progress_sequence = (
            &self.leases,
            &self.active_lease,
            &self.dirty_keys,
            &self.runtime_meta,
        )
            .transaction(|(leases, active, dirty_keys, runtime_meta)| {
                if let Some(raw) = runtime_meta.get(progress_receipt_key.as_slice())? {
                    if leases.get(lease.lease_id.as_bytes())?.as_deref()
                        != Some(completed_lease.as_slice())
                    {
                        return Err(ConflictableTransactionError::Abort(
                            "empty belief assessment receipt conflicts with its terminal lease"
                                .to_string(),
                        ));
                    }
                    let bytes: [u8; 8] = raw.as_ref().try_into().map_err(|_| {
                        ConflictableTransactionError::Abort(
                            "empty belief assessment progress receipt has invalid width"
                                .to_string(),
                        )
                    })?;
                    return Ok(u64::from_be_bytes(bytes));
                }
                if active.get(key.as_bytes())?.as_deref() != Some(lease.lease_id.as_bytes())
                    || leases.get(lease.lease_id.as_bytes())?.as_deref()
                        != Some(expected_lease.as_slice())
                {
                    return Err(ConflictableTransactionError::Abort(
                        "empty belief assessment lease changed".to_string(),
                    ));
                }
                let dirty_raw = dirty_keys.get(key.as_bytes())?.ok_or_else(|| {
                    ConflictableTransactionError::Abort(
                        "empty belief assessment dirty state disappeared".to_string(),
                    )
                })?;
                let mut dirty: DirtyKeyState =
                    serde_json::from_slice(&dirty_raw).map_err(|error| {
                        ConflictableTransactionError::Abort(format!(
                            "invalid dirty state during empty assessment: {error}"
                        ))
                    })?;
                if dirty.belief_key != lease.belief_key
                    || dirty.active_lease_id.as_deref() != Some(lease.lease_id.as_str())
                    || dirty.mutation_generation < lease.epoch
                    || dirty.assessment_cursor != lease.assignment_cursor_start
                {
                    return Err(ConflictableTransactionError::Abort(
                        "empty belief assessment context changed".to_string(),
                    ));
                }
                leases.insert(lease.lease_id.as_bytes(), completed_lease.as_slice())?;
                active.remove(key.as_bytes())?;
                if dirty.mutation_generation > lease.epoch || !lease.assignment_window_complete {
                    if dirty.assessment_cursor < lease.assignment_cursor_end {
                        dirty.assessment_cursor = lease.assignment_cursor_end.clone();
                    }
                    dirty.active_lease_id = None;
                    dirty_keys.insert(
                        key.as_bytes(),
                        serde_json::to_vec(&dirty)
                            .map_err(|error| {
                                ConflictableTransactionError::Abort(format!(
                                    "cannot encode rescheduled empty assessment: {error}"
                                ))
                            })?
                            .as_slice(),
                    )?;
                } else {
                    dirty_keys.remove(key.as_bytes())?;
                }
                record_assessment_progress(runtime_meta, &progress_receipt_key)
            })
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::Backpressure(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.flush().map_err(|error| {
            StorageError::DurabilityIndeterminate(format!(
                "empty belief assessment apply flush failed: {error}"
            ))
        })?;
        Ok(progress_sequence)
    }

    /// Abandon expired leases and mark their keys dirty for recovery.
    pub fn recover_expired_leases(
        &self,
        current_seq: u64,
    ) -> Result<Vec<AssessmentLease>, StorageError> {
        let mut recovered = Vec::new();
        for item in self.leases.iter() {
            let (_, value) = item.map_err(to_storage_io)?;
            let mut lease: AssessmentLease =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            if lease.status == LeaseStatus::Leased && lease.expires_at_seq <= current_seq {
                let expected = lease.clone();
                lease.status = LeaseStatus::Abandoned;
                self.apply_assessment_lease_cas(&AssessmentLeaseCasIntent::AbandonExpired {
                    expected_active_lease: expected,
                    abandoned_lease: lease.clone(),
                })?;
                recovered.push(lease);
            }
        }
        Ok(recovered)
    }

    /// Recover the active lease referenced by one selected dirty key when expired.
    pub(crate) fn recover_expired_lease_for_key(
        &self,
        key: &BeliefKey,
        current_seq: u64,
    ) -> Result<Option<AssessmentLease>, StorageError> {
        let Some(dirty) = self.dirty_state(key)? else {
            return Ok(None);
        };
        let Some(active_lease_id) = dirty.active_lease_id else {
            return Ok(None);
        };
        let Some(mut lease) = self.get_lease(&active_lease_id)? else {
            return Err(StorageError::InvalidPath(
                "dirty belief key references a missing active lease".to_string(),
            ));
        };
        if lease.belief_key != *key || lease.lease_id != active_lease_id {
            return Err(StorageError::InvalidPath(
                "dirty belief key references an inconsistent active lease".to_string(),
            ));
        }
        if lease.status != LeaseStatus::Leased {
            return Err(StorageError::InvalidPath(
                "dirty belief key references a terminal active lease".to_string(),
            ));
        }
        if lease.expires_at_seq > current_seq {
            return Ok(None);
        }
        let expected = lease.clone();
        lease.status = LeaseStatus::Abandoned;
        self.apply_assessment_lease_cas(&AssessmentLeaseCasIntent::AbandonExpired {
            expected_active_lease: expected,
            abandoned_lease: lease.clone(),
        })?;
        self.flush()?;
        Ok(Some(lease))
    }

    /// Append a revision and move the current head.
    ///
    /// Commit validates active lease ownership and evidence membership so two
    /// workers cannot commit overlapping windows for the same key.
    pub fn commit_revision(
        &self,
        lease: &AssessmentLease,
        revision: &BeliefRevision,
    ) -> Result<(), StorageError> {
        let hydration = HydrationRefs {
            evidence_ids: revision.evidence_ids.clone(),
            source_fact_ids: revision.provenance.source_fact_ids.clone(),
            graph_anchor_ids: revision.provenance.graph_anchor_ids.clone(),
            revision_id: Some(revision.revision_id.clone()),
        };
        let view = self.project_view(revision, hydration);
        self.commit_belief_assessment(lease, revision.clone(), view)?;
        Ok(())
    }

    /// Durably stage a complete belief commit before applying its products.
    #[cfg(test)]
    pub(crate) fn prepare_belief_commit(
        &self,
        intent: &BeliefCommitIntent,
    ) -> Result<(), StorageError> {
        let _write = self.writable_guard()?;
        self.prepare_belief_commit_admitted(intent)
    }

    fn prepare_belief_commit_admitted(
        &self,
        intent: &BeliefCommitIntent,
    ) -> Result<(), StorageError> {
        let encoded = serde_json::to_vec(intent).map_err(to_storage_data)?;
        let lease_id = intent.expected_active_lease().lease_id.as_bytes();
        use sled::transaction::{ConflictableTransactionError, TransactionError};
        (&self.commit_intents, &self.commit_intent_by_lease)
            .transaction(|(intents, by_lease)| {
                if let Some(existing) = intents.get(intent.intent_id().as_bytes())? {
                    if existing.as_ref() != encoded.as_slice() {
                        return Err(ConflictableTransactionError::Abort(format!(
                            "belief commit intent conflict for '{}'",
                            intent.intent_id()
                        )));
                    }
                }
                if let Some(existing) = by_lease.get(lease_id)? {
                    if existing.as_ref() != intent.intent_id().as_bytes() {
                        return Err(ConflictableTransactionError::Abort(format!(
                            "belief lease commit intent conflict for '{}'",
                            intent.expected_active_lease().lease_id
                        )));
                    }
                }
                intents.insert(intent.intent_id().as_bytes(), encoded.as_slice())?;
                by_lease.insert(lease_id, intent.intent_id().as_bytes())?;
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::InvalidPath(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.flush()
    }

    /// Stage and atomically apply one complete assessment result.
    pub fn commit_belief_assessment(
        &self,
        lease: &AssessmentLease,
        revision: BeliefRevision,
        public_view: BeliefView,
    ) -> Result<BeliefCommitRecoveryDisposition, StorageError> {
        self.commit_belief_assessment_with_progress(lease, revision, public_view)
            .map(|(disposition, _)| disposition)
    }

    pub(crate) fn commit_belief_assessment_with_progress(
        &self,
        lease: &AssessmentLease,
        revision: BeliefRevision,
        public_view: BeliefView,
    ) -> Result<(BeliefCommitRecoveryDisposition, u64), StorageError> {
        // One admission spans durable intent preparation and final apply. The
        // migration writer therefore drains the whole admitted transition or
        // waits before fencing the source authority.
        let _write = self.writable_guard()?;
        let intent_id = format!("belief-commit-{}", revision.revision_id);
        if let Some(receipt_raw) = self
            .commit_receipts
            .get(intent_id.as_bytes())
            .map_err(to_storage_io)?
        {
            let receipt: BeliefCommitIntent =
                serde_json::from_slice(&receipt_raw).map_err(to_storage_data)?;
            verify_commit_retry(&receipt, lease, &revision, &public_view)?;
            let progress_receipt_key = assessment_progress_receipt_key("commit", &intent_id);
            let progress = self
                .exact_assessment_progress_receipt(&progress_receipt_key)?
                .ok_or_else(|| {
                    StorageError::InvalidPath(
                        "belief commit receipt is missing exact progress".to_string(),
                    )
                })?;
            self.flush().map_err(|error| {
                StorageError::DurabilityIndeterminate(format!(
                    "belief commit exact replay flush failed: {error}"
                ))
            })?;
            return Ok((BeliefCommitRecoveryDisposition::AlreadyConsistent, progress));
        }
        if let Some(intent_raw) = self
            .commit_intents
            .get(intent_id.as_bytes())
            .map_err(to_storage_io)?
        {
            let intent: BeliefCommitIntent =
                serde_json::from_slice(&intent_raw).map_err(to_storage_data)?;
            verify_commit_retry(&intent, lease, &revision, &public_view)?;
            return self.apply_belief_commit_with_progress_admitted(&intent_id);
        }
        self.validate_revision_commit(lease, &revision)?;
        let dirty = self.dirty_state(&revision.belief_key)?.ok_or_else(|| {
            StorageError::InvalidPath("belief commit requires durable dirty state".to_string())
        })?;
        let mut completed = lease.clone();
        completed.status = LeaseStatus::Completed;
        let intent = BeliefCommitIntent::try_new(
            intent_id,
            lease.clone(),
            completed,
            dirty,
            revision,
            public_view,
        )?;
        self.prepare_belief_commit_admitted(&intent)?;
        #[cfg(test)]
        self.pause_commit_after_prepare_for_test();
        self.apply_belief_commit_with_progress_admitted(intent.intent_id())
    }

    fn validate_revision_commit(
        &self,
        lease: &AssessmentLease,
        revision: &BeliefRevision,
    ) -> Result<(), StorageError> {
        let active_id = self
            .active_lease
            .get(revision.belief_key.index_key().as_bytes())
            .map_err(to_storage_io)?
            .ok_or_else(|| StorageError::InvalidPath("missing active lease".to_string()))?;
        let active_id = String::from_utf8(active_id.to_vec()).map_err(to_storage_utf8)?;
        if active_id != lease.lease_id || self.get_lease(&active_id)?.as_ref() != Some(lease) {
            return Err(StorageError::InvalidPath("stale lease owner".to_string()));
        }
        if revision.source_cursor_start < lease.input_cursor_start
            || revision.source_cursor_end > lease.input_cursor_end
        {
            return Err(StorageError::InvalidPath(
                "revision cursor outside lease window".to_string(),
            ));
        }
        for evidence_id in &revision.evidence_ids {
            if self.get_evidence(evidence_id)?.is_none() {
                return Err(StorageError::InvalidPath(format!(
                    "unknown evidence '{}'",
                    evidence_id
                )));
            }
        }
        Ok(())
    }

    /// Atomically apply one staged belief commit and remove its durable intent.
    pub(crate) fn apply_belief_commit(
        &self,
        intent_id: &str,
    ) -> Result<BeliefCommitRecoveryDisposition, StorageError> {
        let _write = self.writable_guard()?;
        self.apply_belief_commit_with_progress_admitted(intent_id)
            .map(|(disposition, _)| disposition)
    }

    fn apply_belief_commit_with_progress_admitted(
        &self,
        intent_id: &str,
    ) -> Result<(BeliefCommitRecoveryDisposition, u64), StorageError> {
        let progress_receipt_key = assessment_progress_receipt_key("commit", intent_id);
        let Some(intent_raw) = self
            .commit_intents
            .get(intent_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            if self
                .commit_receipts
                .get(intent_id.as_bytes())
                .map_err(to_storage_io)?
                .is_none()
            {
                return Err(StorageError::InvalidPath(format!(
                    "missing belief commit operation '{intent_id}'"
                )));
            }
            let progress = self
                .exact_assessment_progress_receipt(&progress_receipt_key)?
                .ok_or_else(|| {
                    StorageError::InvalidPath(
                        "belief commit receipt is missing exact progress".to_string(),
                    )
                })?;
            self.flush().map_err(|error| {
                StorageError::DurabilityIndeterminate(format!(
                    "belief commit exact apply replay flush failed: {error}"
                ))
            })?;
            return Ok((BeliefCommitRecoveryDisposition::AlreadyConsistent, progress));
        };
        let intent: BeliefCommitIntent =
            serde_json::from_slice(&intent_raw).map_err(to_storage_data)?;
        let lease = intent.expected_active_lease();
        let completed = intent.completed_lease();
        let dirty = intent.expected_dirty_state();
        let revision = intent.revision();
        let view = intent.public_view();
        let key = revision.belief_key.index_key();
        let subject_index_key = format!(
            "{}::{}::{}",
            view.key.subject.index_key(),
            view.key.perspective.index_key(),
            view.key.index_key()
        );
        let expected_lease_bytes = serde_json::to_vec(lease).map_err(to_storage_data)?;
        let completed_lease_bytes = serde_json::to_vec(completed).map_err(to_storage_data)?;
        let revision_bytes = serde_json::to_vec(revision).map_err(to_storage_data)?;
        let view_bytes = serde_json::to_vec(view).map_err(to_storage_data)?;
        let committed_evidence_keys = revision
            .evidence_ids
            .iter()
            .map(|evidence_id| committed_evidence_index_key(&revision.belief_key, evidence_id))
            .collect::<Vec<_>>();

        use sled::transaction::{ConflictableTransactionError, TransactionError};
        let disposition = (
            &self.revisions,
            &self.committed_evidence,
            &self.revision_head,
            &self.views,
            &self.view_by_subject,
            &self.leases,
            &self.active_lease,
            &self.dirty_keys,
            &self.commit_intents,
            &self.commit_intent_by_lease,
            &self.commit_receipts,
            &self.runtime_meta,
        )
            .transaction(
                |(
                    revisions,
                    committed_evidence,
                    heads,
                    views,
                    subject_views,
                    leases,
                    active,
                    dirty_keys,
                    intents,
                    intents_by_lease,
                    receipts,
                    runtime_meta,
                )| {
                    let products_committed =
                        revisions.get(revision.revision_id.as_bytes())?.as_deref()
                            == Some(revision_bytes.as_slice())
                            && heads.get(key.as_bytes())?.as_deref()
                                == Some(revision.revision_id.as_bytes())
                            && views.get(key.as_bytes())?.as_deref() == Some(view_bytes.as_slice())
                            && leases.get(lease.lease_id.as_bytes())?.as_deref()
                                == Some(completed_lease_bytes.as_slice())
                            && active.get(key.as_bytes())?.is_none();
                    if products_committed {
                        for index_key in &committed_evidence_keys {
                            if let Some(existing) = committed_evidence.get(index_key.as_slice())? {
                                if existing.as_ref() != revision.revision_id.as_bytes() {
                                    return Err(ConflictableTransactionError::Abort(
                                        "belief committed evidence index conflict".to_string(),
                                    ));
                                }
                            } else {
                                committed_evidence.insert(
                                    index_key.as_slice(),
                                    revision.revision_id.as_bytes(),
                                )?;
                            }
                        }
                        if intents_by_lease.get(lease.lease_id.as_bytes())?.as_deref()
                            != Some(intent_id.as_bytes())
                        {
                            return Err(ConflictableTransactionError::Abort(
                                "belief commit lease index changed".to_string(),
                            ));
                        }
                        intents.remove(intent_id.as_bytes())?;
                        intents_by_lease.remove(lease.lease_id.as_bytes())?;
                        receipts.insert(intent_id.as_bytes(), intent_raw.as_ref())?;
                        let progress =
                            record_assessment_progress(runtime_meta, &progress_receipt_key)?;
                        return Ok((BeliefCommitRecoveryDisposition::AlreadyConsistent, progress));
                    }
                    if active.get(key.as_bytes())?.as_deref() != Some(lease.lease_id.as_bytes())
                        || leases.get(lease.lease_id.as_bytes())?.as_deref()
                            != Some(expected_lease_bytes.as_slice())
                    {
                        return Err(ConflictableTransactionError::Abort(
                            "belief commit expected lease changed".to_string(),
                        ));
                    }
                    let current_dirty_raw = dirty_keys.get(key.as_bytes())?.ok_or_else(|| {
                        ConflictableTransactionError::Abort(
                            "belief commit dirty state disappeared".to_string(),
                        )
                    })?;
                    let current_dirty: DirtyKeyState = serde_json::from_slice(&current_dirty_raw)
                        .map_err(|error| {
                        ConflictableTransactionError::Abort(format!(
                            "invalid dirty state during belief commit: {error}"
                        ))
                    })?;
                    if current_dirty.belief_key != dirty.belief_key
                        || current_dirty.active_lease_id != dirty.active_lease_id
                        || current_dirty.latest_seq < dirty.latest_seq
                        || current_dirty.dirty_since_seq > dirty.dirty_since_seq
                        || current_dirty.mutation_generation < dirty.mutation_generation
                        || current_dirty.assessment_cursor < dirty.assessment_cursor
                        || dirty.assessment_cursor != lease.assignment_cursor_start
                    {
                        return Err(ConflictableTransactionError::Abort(
                            "belief commit dirty state changed non-monotonically".to_string(),
                        ));
                    }
                    let changed_after_lease = current_dirty.mutation_generation > lease.epoch;
                    let remaining_dirty =
                        if changed_after_lease || !lease.assignment_window_complete {
                            let mut remaining = current_dirty;
                            if remaining.assessment_cursor < lease.assignment_cursor_end {
                                remaining.assessment_cursor = lease.assignment_cursor_end.clone();
                            }
                            remaining.active_lease_id = None;
                            Some(serde_json::to_vec(&remaining).map_err(|error| {
                                ConflictableTransactionError::Abort(format!(
                                    "cannot encode rescheduled dirty state: {error}"
                                ))
                            })?)
                        } else {
                            None
                        };
                    let expected_head = revision.prior_revision_id.as_deref().map(str::as_bytes);
                    if heads.get(key.as_bytes())?.as_deref() != expected_head {
                        return Err(ConflictableTransactionError::Abort(
                            "belief revision head changed before commit".to_string(),
                        ));
                    }
                    if let Some(existing) = revisions.get(revision.revision_id.as_bytes())? {
                        if existing.as_ref() != revision_bytes.as_slice() {
                            return Err(ConflictableTransactionError::Abort(
                                "belief revision id conflict".to_string(),
                            ));
                        }
                    }
                    revisions.insert(revision.revision_id.as_bytes(), revision_bytes.as_slice())?;
                    for index_key in &committed_evidence_keys {
                        if let Some(existing) = committed_evidence.get(index_key.as_slice())? {
                            if existing.as_ref() != revision.revision_id.as_bytes() {
                                return Err(ConflictableTransactionError::Abort(
                                    "belief committed evidence index conflict".to_string(),
                                ));
                            }
                        } else {
                            committed_evidence
                                .insert(index_key.as_slice(), revision.revision_id.as_bytes())?;
                        }
                    }
                    heads.insert(key.as_bytes(), revision.revision_id.as_bytes())?;
                    views.insert(key.as_bytes(), view_bytes.as_slice())?;
                    subject_views.insert(subject_index_key.as_bytes(), key.as_bytes())?;
                    leases.insert(lease.lease_id.as_bytes(), completed_lease_bytes.as_slice())?;
                    active.remove(key.as_bytes())?;
                    if let Some(remaining) = &remaining_dirty {
                        dirty_keys.insert(key.as_bytes(), remaining.as_slice())?;
                    } else {
                        dirty_keys.remove(key.as_bytes())?;
                    }
                    if intents_by_lease.get(lease.lease_id.as_bytes())?.as_deref()
                        != Some(intent_id.as_bytes())
                    {
                        return Err(ConflictableTransactionError::Abort(
                            "belief commit lease index changed".to_string(),
                        ));
                    }
                    intents.remove(intent_id.as_bytes())?;
                    intents_by_lease.remove(lease.lease_id.as_bytes())?;
                    receipts.insert(intent_id.as_bytes(), intent_raw.as_ref())?;
                    let progress = record_assessment_progress(runtime_meta, &progress_receipt_key)?;
                    Ok((
                        if remaining_dirty.is_some() {
                            BeliefCommitRecoveryDisposition::RescheduledDirtyKey
                        } else {
                            BeliefCommitRecoveryDisposition::ResumedCommit
                        },
                        progress,
                    ))
                },
            )
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::InvalidPath(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.flush().map_err(|error| {
            StorageError::DurabilityIndeterminate(format!(
                "belief commit apply flush failed: {error}"
            ))
        })?;
        Ok(disposition)
    }

    /// Reconcile staged commits and missing current views from durable products.
    pub fn reconcile_open_commits(
        &self,
    ) -> Result<Vec<BeliefCommitRecoveryDisposition>, StorageError> {
        let mut dispositions = Vec::new();
        let intents = self
            .commit_intents
            .iter()
            .map(|item| {
                let (key, value) = item.map_err(to_storage_io)?;
                let intent_id = String::from_utf8(key.to_vec()).map_err(to_storage_utf8)?;
                let intent: BeliefCommitIntent =
                    serde_json::from_slice(&value).map_err(to_storage_data)?;
                if intent.intent_id() != intent_id {
                    return Err(StorageError::InvalidPath(
                        "belief commit intent tree key conflicts with its record".to_string(),
                    ));
                }
                Ok((intent_id, intent))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let lease_clock = self.assessment_lease_clock()?;
        for (intent_id, intent) in intents {
            let expected_lease = intent.expected_active_lease();
            let stored_lease = self.get_lease(&expected_lease.lease_id)?.ok_or_else(|| {
                StorageError::InvalidPath(
                    "belief commit intent references a missing lease".to_string(),
                )
            })?;
            if stored_lease.status == LeaseStatus::Abandoned {
                self.discard_abandoned_commit_intent(&intent)?;
                continue;
            }
            if stored_lease.status == LeaseStatus::Leased
                && expected_lease.expires_at_seq <= lease_clock
            {
                self.recover_expired_lease_for_key(&expected_lease.belief_key, lease_clock)?;
                continue;
            }
            dispositions.push(self.apply_belief_commit(&intent_id)?);
        }
        for item in self.revision_head.iter() {
            let (key, revision_id) = item.map_err(to_storage_io)?;
            let key = String::from_utf8(key.to_vec()).map_err(to_storage_utf8)?;
            let revision_id = String::from_utf8(revision_id.to_vec()).map_err(to_storage_utf8)?;
            let Some(revision) = self.get_revision(&revision_id)? else {
                return Err(StorageError::InvalidPath(
                    "belief revision head references a missing revision".to_string(),
                ));
            };
            let current: Option<BeliefView> =
                decode_optional(self.views.get(key.as_bytes()).map_err(to_storage_io)?)?;
            let current_matches = current
                .as_ref()
                .and_then(|view| view.current_revision_id.as_deref())
                == Some(revision_id.as_str());
            let indexed = match &current {
                Some(view) => self
                    .view_by_subject
                    .get(view_subject_index_key(view).as_bytes())
                    .map_err(to_storage_io)?
                    .is_some_and(|stored| stored.as_ref() == key.as_bytes()),
                None => false,
            };
            if !current_matches || !indexed {
                let hydration = HydrationRefs {
                    evidence_ids: revision.evidence_ids.clone(),
                    source_fact_ids: revision.provenance.source_fact_ids.clone(),
                    graph_anchor_ids: revision.provenance.graph_anchor_ids.clone(),
                    revision_id: Some(revision.revision_id.clone()),
                };
                let repaired = if current_matches {
                    current.expect("matching view must be present")
                } else {
                    self.project_view(&revision, hydration)
                };
                self.put_view(&repaired)?;
                dispositions.push(BeliefCommitRecoveryDisposition::RebuiltCurrentView);
            }
        }
        if !dispositions.is_empty() {
            self.flush()?;
        }
        Ok(dispositions)
    }

    fn reconcile_commit_intent_index(&self) -> Result<(), StorageError> {
        let _write = self.write_gate.read();
        let mut expected = BTreeMap::new();
        let mut changed = false;
        for item in self.commit_intents.iter() {
            let (intent_id, value) = item.map_err(to_storage_io)?;
            let intent: BeliefCommitIntent =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            if intent.intent_id().as_bytes() != intent_id.as_ref() {
                return Err(StorageError::InvalidPath(
                    "belief commit intent tree key conflicts with its record".to_string(),
                ));
            }
            let lease_id = intent.expected_active_lease().lease_id.as_bytes().to_vec();
            if expected
                .insert(lease_id.clone(), intent_id.to_vec())
                .is_some()
            {
                return Err(StorageError::InvalidPath(
                    "multiple belief commit intents reference one lease".to_string(),
                ));
            }
            match self
                .commit_intent_by_lease
                .compare_and_swap(lease_id, None as Option<&[u8]>, Some(intent_id.as_ref()))
                .map_err(to_storage_io)?
            {
                Ok(()) => changed = true,
                Err(error) if error.current.as_deref() == Some(intent_id.as_ref()) => {}
                Err(_) => {
                    return Err(StorageError::InvalidPath(
                        "belief commit lease index conflicts during reopen".to_string(),
                    ));
                }
            }
        }
        for item in self.commit_intent_by_lease.iter() {
            let (lease_id, intent_id) = item.map_err(to_storage_io)?;
            if expected.get(lease_id.as_ref()).map(Vec::as_slice) != Some(intent_id.as_ref()) {
                return Err(StorageError::InvalidPath(
                    "belief commit lease index is inconsistent during reopen".to_string(),
                ));
            }
        }
        if self.commit_intent_by_lease.len() != expected.len() {
            return Err(StorageError::InvalidPath(
                "belief commit lease index count diverges during reopen".to_string(),
            ));
        }
        if changed {
            self.flush()?;
        }
        Ok(())
    }

    fn discard_abandoned_commit_intent(
        &self,
        intent: &BeliefCommitIntent,
    ) -> Result<(), StorageError> {
        let _write = self.writable_guard()?;
        let intent_bytes = serde_json::to_vec(intent).map_err(to_storage_data)?;
        let lease_id = intent.expected_active_lease().lease_id.as_bytes();
        use sled::transaction::{ConflictableTransactionError, TransactionError};
        (
            &self.leases,
            &self.commit_intents,
            &self.commit_intent_by_lease,
        )
            .transaction(|(leases, intents, by_lease)| {
                let stored_lease = leases
                    .get(lease_id)?
                    .ok_or_else(|| {
                        ConflictableTransactionError::Abort(
                            "abandoned belief commit intent lost its lease".to_string(),
                        )
                    })
                    .and_then(|raw| {
                        serde_json::from_slice::<AssessmentLease>(&raw).map_err(|error| {
                            ConflictableTransactionError::Abort(format!(
                                "invalid abandoned belief commit lease: {error}"
                            ))
                        })
                    })?;
                if stored_lease.status != LeaseStatus::Abandoned
                    || intents.get(intent.intent_id().as_bytes())?.as_deref()
                        != Some(intent_bytes.as_slice())
                    || by_lease.get(lease_id)?.as_deref() != Some(intent.intent_id().as_bytes())
                {
                    return Err(ConflictableTransactionError::Abort(
                        "abandoned belief commit intent changed during recovery".to_string(),
                    ));
                }
                intents.remove(intent.intent_id().as_bytes())?;
                by_lease.remove(lease_id)?;
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::InvalidPath(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.flush()
    }

    fn reconcile_assessment_indexes(&self) -> Result<(), StorageError> {
        // Derived indexes remain repairable after a legacy semantic freeze.
        // They are excluded from authority snapshots and migration parity.
        let _write = self.write_gate.read();
        let mut changed = false;
        for item in self.assignments.iter() {
            let (_, value) = item.map_err(to_storage_io)?;
            let assignment: EvidenceAssignment =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            let generation = self.assignment_generation(&assignment.assignment_id)?;
            let index_key = assignment_by_key_index_key(&assignment, generation);
            match self
                .assignments_by_key
                .compare_and_swap(
                    index_key,
                    None as Option<&[u8]>,
                    Some(assignment.assignment_id.as_bytes()),
                )
                .map_err(to_storage_io)?
            {
                Ok(()) => changed = true,
                Err(error)
                    if error.current.as_deref() == Some(assignment.assignment_id.as_bytes()) => {}
                Err(_) => {
                    return Err(StorageError::InvalidPath(
                        "belief assignment key index conflicts during reopen".to_string(),
                    ));
                }
            }
        }
        let mut expected_assignment_tails = BTreeMap::new();
        for item in self.assignments_by_key.iter() {
            let (index_key, assignment_id) = item.map_err(to_storage_io)?;
            let assignment_id =
                String::from_utf8(assignment_id.to_vec()).map_err(to_storage_utf8)?;
            let Some(assignment) = self.get_assignment(&assignment_id)? else {
                return Err(StorageError::InvalidPath(
                    "belief assignment key index references a missing assignment".to_string(),
                ));
            };
            let generation = self.assignment_generation(&assignment.assignment_id)?;
            if assignment_by_key_index_key(&assignment, generation).as_slice() != index_key.as_ref()
            {
                return Err(StorageError::InvalidPath(
                    "belief assignment key index is inconsistent during reopen".to_string(),
                ));
            }
            expected_assignment_tails.insert(
                assignment.belief_key.index_key().into_bytes(),
                index_key.to_vec(),
            );
        }
        if self.assignments_by_key.len() != self.assignments.len() {
            return Err(StorageError::InvalidPath(
                "belief assignment key index count diverges during reopen".to_string(),
            ));
        }
        for (key, expected_tail) in &expected_assignment_tails {
            match self
                .assignment_tail_by_key
                .compare_and_swap(key, None as Option<&[u8]>, Some(expected_tail.as_slice()))
                .map_err(to_storage_io)?
            {
                Ok(()) => changed = true,
                Err(error) if error.current.as_deref() == Some(expected_tail.as_slice()) => {}
                Err(_) => {
                    return Err(StorageError::InvalidPath(
                        "belief assignment stream tail conflicts during reopen".to_string(),
                    ));
                }
            }
        }
        for item in self.assignment_tail_by_key.iter() {
            let (key, tail) = item.map_err(to_storage_io)?;
            if expected_assignment_tails
                .get(key.as_ref())
                .map(Vec::as_slice)
                != Some(tail.as_ref())
                || self
                    .assignments_by_key
                    .get(&tail)
                    .map_err(to_storage_io)?
                    .is_none()
            {
                return Err(StorageError::InvalidPath(
                    "belief assignment stream tail is inconsistent during reopen".to_string(),
                ));
            }
        }
        if self.assignment_tail_by_key.len() != expected_assignment_tails.len() {
            return Err(StorageError::InvalidPath(
                "belief assignment stream tail count diverges during reopen".to_string(),
            ));
        }

        let mut expected_committed = BTreeSet::new();
        for item in self.revisions.iter() {
            let (_, value) = item.map_err(to_storage_io)?;
            let revision: BeliefRevision =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            for evidence_id in &revision.evidence_ids {
                let index_key = committed_evidence_index_key(&revision.belief_key, evidence_id);
                if !expected_committed.insert(index_key.clone()) {
                    return Err(StorageError::InvalidPath(
                        "belief evidence appears in more than one committed revision".to_string(),
                    ));
                }
                match self
                    .committed_evidence
                    .compare_and_swap(
                        index_key,
                        None as Option<&[u8]>,
                        Some(revision.revision_id.as_bytes()),
                    )
                    .map_err(to_storage_io)?
                {
                    Ok(()) => changed = true,
                    Err(error)
                        if error.current.as_deref() == Some(revision.revision_id.as_bytes()) => {}
                    Err(_) => {
                        return Err(StorageError::InvalidPath(
                            "belief committed evidence index conflicts during reopen".to_string(),
                        ));
                    }
                }
            }
        }
        for item in self.committed_evidence.iter() {
            let (index_key, revision_id) = item.map_err(to_storage_io)?;
            let revision_id = String::from_utf8(revision_id.to_vec()).map_err(to_storage_utf8)?;
            let Some(revision) = self.get_revision(&revision_id)? else {
                return Err(StorageError::InvalidPath(
                    "belief committed evidence index references a missing revision".to_string(),
                ));
            };
            if !revision.evidence_ids.iter().any(|evidence_id| {
                committed_evidence_index_key(&revision.belief_key, evidence_id).as_slice()
                    == index_key.as_ref()
            }) {
                return Err(StorageError::InvalidPath(
                    "belief committed evidence index is inconsistent during reopen".to_string(),
                ));
            }
        }
        if self.committed_evidence.len() != expected_committed.len() {
            return Err(StorageError::InvalidPath(
                "belief committed evidence index count diverges during reopen".to_string(),
            ));
        }
        changed |= self.reconcile_evidence_ingestion_authority()?;
        changed |= self.reconcile_legacy_assessment_receipts()?;
        if changed {
            self.flush()?;
        }
        Ok(())
    }

    fn reconcile_evidence_ingestion_authority(&self) -> Result<bool, StorageError> {
        let mut acknowledged_high_water = 0u64;
        let mut receipt_sequences_by_cursor = BTreeMap::<Vec<u8>, BTreeSet<u64>>::new();
        for item in self.evidence_ingestion_receipts.iter() {
            let (receipt_key, raw) = item.map_err(to_storage_io)?;
            let receipt: EvidenceIngestionReceipt =
                serde_json::from_slice(&raw).map_err(to_storage_data)?;
            if evidence_receipt_key(&receipt)?.as_slice() != receipt_key.as_ref() {
                return Err(StorageError::InvalidPath(
                    "evidence ingestion receipt key conflicts with its authority".to_string(),
                ));
            }
            let next = receipt_cursor(&receipt);
            let current: EvidenceConsumerCursor = decode_required(
                self.evidence_consumer_cursors
                    .get(evidence_cursor_key(&next)?)
                    .map_err(to_storage_io)?,
                "evidence ingestion receipt is missing its durable consumer cursor",
            )?;
            validate_exact_receipt_replay_cursor(&receipt, &next, &current)?;
            receipt_sequences_by_cursor
                .entry(evidence_cursor_key(&next)?)
                .or_default()
                .insert(receipt.identity.source_record.seq);
            acknowledged_high_water =
                acknowledged_high_water.max(receipt.identity.source_record.seq);
        }
        for item in self.evidence_consumer_cursors.iter() {
            let (cursor_key, raw) = item.map_err(to_storage_io)?;
            let cursor: EvidenceConsumerCursor =
                serde_json::from_slice(&raw).map_err(to_storage_data)?;
            if evidence_cursor_key(&cursor)?.as_slice() != cursor_key.as_ref() {
                return Err(StorageError::InvalidPath(
                    "evidence consumer cursor key conflicts with its authority".to_string(),
                ));
            }
            let receipt_identity = receipt_identity_for_cursor(&cursor);
            let receipt: EvidenceIngestionReceipt = decode_required(
                self.evidence_ingestion_receipts
                    .get(serde_json::to_vec(&receipt_identity).map_err(to_storage_data)?)
                    .map_err(to_storage_io)?,
                "evidence consumer cursor is missing its exact ingestion receipt",
            )?;
            if receipt.identity != receipt_identity {
                return Err(StorageError::InvalidPath(
                    "evidence consumer cursor receipt identity changed".to_string(),
                ));
            }
            let sequences = receipt_sequences_by_cursor
                .get(cursor_key.as_ref())
                .ok_or_else(|| {
                    StorageError::InvalidPath(
                        "evidence consumer cursor has no ingestion receipt history".to_string(),
                    )
                })?;
            validate_contiguous_receipt_history(&cursor, sequences)?;
        }

        let current = self
            .runtime_meta
            .get(KEY_ASSESSMENT_SOURCE_HIGH_WATER)
            .map_err(to_storage_io)?;
        let current_sequence = current
            .as_deref()
            .map(decode_runtime_sequence)
            .transpose()?
            .unwrap_or(0);
        if current_sequence > acknowledged_high_water {
            return Err(StorageError::InvalidPath(
                "belief acknowledged source watermark exceeds receipt authority".to_string(),
            ));
        }
        if current_sequence == acknowledged_high_water || acknowledged_high_water == 0 {
            return Ok(false);
        }
        self.runtime_meta
            .insert(
                KEY_ASSESSMENT_SOURCE_HIGH_WATER,
                acknowledged_high_water.to_be_bytes().as_slice(),
            )
            .map_err(to_storage_io)?;
        Ok(true)
    }

    fn reconcile_legacy_assessment_receipts(&self) -> Result<bool, StorageError> {
        let mut revisions = BTreeMap::new();
        for item in self.revisions.iter() {
            let (revision_id, value) = item.map_err(to_storage_io)?;
            let revision_id = String::from_utf8(revision_id.to_vec()).map_err(to_storage_utf8)?;
            let revision: BeliefRevision =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            if revision.revision_id != revision_id {
                return Err(StorageError::InvalidPath(
                    "belief revision tree key conflicts with its record".to_string(),
                ));
            }
            revisions.insert(revision_id, revision);
        }
        let mut reachable = BTreeSet::new();
        for item in self.revision_head.iter() {
            let (key, revision_id) = item.map_err(to_storage_io)?;
            let key = String::from_utf8(key.to_vec()).map_err(to_storage_utf8)?;
            let mut revision_id =
                String::from_utf8(revision_id.to_vec()).map_err(to_storage_utf8)?;
            let mut chain = BTreeSet::new();
            loop {
                if !chain.insert(revision_id.clone()) {
                    return Err(StorageError::InvalidPath(
                        "belief revision chain is cyclic during legacy receipt migration"
                            .to_string(),
                    ));
                }
                let revision = revisions.get(&revision_id).ok_or_else(|| {
                    StorageError::InvalidPath(
                        "belief revision head chain references a missing revision".to_string(),
                    )
                })?;
                if revision.belief_key.index_key() != key {
                    return Err(StorageError::InvalidPath(
                        "belief revision head chain crosses belief keys".to_string(),
                    ));
                }
                reachable.insert(revision_id.clone());
                let Some(prior) = &revision.prior_revision_id else {
                    break;
                };
                revision_id = prior.clone();
            }
        }

        let local_fence = self
            .authority_meta
            .get(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE)
            .map_err(to_storage_io)?;
        let portable_fence = self
            .legacy_assessment_receipts
            .get(KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE)
            .map_err(to_storage_io)?;
        if local_fence.as_deref() != portable_fence.as_deref() {
            return Err(StorageError::InvalidPath(
                "legacy belief assessment migration fence copies diverge".to_string(),
            ));
        }
        let fence: Option<LegacyAssessmentMigrationFence> = decode_optional(local_fence)?;
        if let Some(fence) = fence {
            validate_legacy_assessment_migration_fence(&fence)?;
            let stored = self
                .legacy_assessment_receipts
                .iter()
                .map(|item| {
                    let (revision_id, _) = item.map_err(to_storage_io)?;
                    if revision_id.as_ref() == KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE {
                        return Ok(None);
                    }
                    String::from_utf8(revision_id.to_vec())
                        .map_err(to_storage_utf8)
                        .map(Some)
                })
                .collect::<Result<Vec<_>, StorageError>>()?
                .into_iter()
                .flatten()
                .collect::<BTreeSet<_>>();
            let migrated = fence
                .migrated_revision_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            if stored != migrated {
                return Err(StorageError::InvalidPath(
                    "legacy belief assessment receipt identities changed after cutover".to_string(),
                ));
            }
            let mut used_leases = BTreeSet::new();
            for revision_id in &reachable {
                let revision = revisions
                    .get(revision_id)
                    .expect("reachable revision must remain available");
                if let Some(lease_id) = self.verify_current_commit_receipt(revision)? {
                    if migrated.contains(revision_id) {
                        return Err(StorageError::InvalidPath(
                            "belief revision has both current and legacy commit authority"
                                .to_string(),
                        ));
                    }
                    if !used_leases.insert(lease_id) {
                        return Err(StorageError::InvalidPath(
                            "completed belief assessment lease authenticates multiple reachable revisions"
                                .to_string(),
                        ));
                    }
                    continue;
                }
                if !migrated.contains(revision_id) {
                    return Err(StorageError::InvalidPath(
                        "current-format belief revision lost its commit receipt after cutover"
                            .to_string(),
                    ));
                }
                let receipt = self.load_legacy_assessment_receipt(revision)?;
                if !used_leases.insert(receipt.completed_lease.lease_id.clone()) {
                    return Err(StorageError::InvalidPath(
                        "completed belief assessment lease authenticates multiple reachable revisions"
                            .to_string(),
                    ));
                }
            }
            if !migrated.is_subset(&reachable) {
                return Err(StorageError::InvalidPath(
                    "legacy belief assessment migration references an unreachable revision"
                        .to_string(),
                ));
            }
            return Ok(false);
        }

        let mut completed_leases = Vec::new();
        for item in self.leases.iter() {
            let (_, value) = item.map_err(to_storage_io)?;
            let lease: AssessmentLease = serde_json::from_slice(&value).map_err(to_storage_data)?;
            if lease.status == LeaseStatus::Completed {
                completed_leases.push(lease);
            }
        }
        completed_leases.sort_by(|left, right| left.lease_id.cmp(&right.lease_id));

        let source_snapshot = self.authority_snapshot()?;
        let mut migrated_receipts = Vec::new();
        let mut migrated_revision_ids = Vec::new();
        let mut used_leases = BTreeSet::new();
        for revision_id in &reachable {
            let revision = revisions
                .get(revision_id)
                .expect("reachable revision must remain available");
            if let Some(lease_id) = self.verify_current_commit_receipt(revision)? {
                if !used_leases.insert(lease_id) {
                    return Err(StorageError::InvalidPath(
                        "completed belief assessment lease authenticates multiple reachable revisions"
                            .to_string(),
                    ));
                }
                if self
                    .legacy_assessment_receipts
                    .get(revision_id.as_bytes())
                    .map_err(to_storage_io)?
                    .is_some()
                {
                    return Err(StorageError::InvalidPath(
                        "belief revision has both current and legacy commit authority".to_string(),
                    ));
                }
                continue;
            }
            let progress_receipt_key =
                assessment_progress_receipt_key("commit", &revision_id_to_intent_id(revision_id));
            if self
                .exact_assessment_progress_receipt(&progress_receipt_key)?
                .is_some()
            {
                return Err(StorageError::InvalidPath(
                    "current-format belief revision is missing its commit receipt".to_string(),
                ));
            }
            let matching_leases = completed_leases
                .iter()
                .filter(|lease| legacy_lease_matches_revision(lease, revision))
                .collect::<Vec<_>>();
            let [completed_lease] = matching_leases.as_slice() else {
                return Err(StorageError::InvalidPath(format!(
                    "legacy belief revision '{}' requires exactly one matching completed lease",
                    revision.revision_id
                )));
            };
            if !used_leases.insert(completed_lease.lease_id.clone()) {
                return Err(StorageError::InvalidPath(
                    "completed belief assessment lease authenticates multiple reachable revisions"
                        .to_string(),
                ));
            }
            for evidence_id in &revision.evidence_ids {
                let evidence = self.get_evidence(evidence_id)?.ok_or_else(|| {
                    StorageError::InvalidPath(
                        "legacy belief revision references missing evidence".to_string(),
                    )
                })?;
                if evidence.candidate_key != revision.belief_key {
                    return Err(StorageError::InvalidPath(
                        "legacy belief revision evidence crosses belief keys".to_string(),
                    ));
                }
            }
            let receipt = LegacyAssessmentReceipt {
                schema_version: LEGACY_ASSESSMENT_RECEIPT_SCHEMA_VERSION,
                revision: revision.clone(),
                completed_lease: (*completed_lease).clone(),
                // Zero is a durable migration sentinel. New assessment
                // transitions begin at one and retain their exact receipts.
                legacy_progress_sequence: 0,
            };
            let encoded = serde_json::to_vec(&receipt).map_err(to_storage_data)?;
            if let Some(existing) = self
                .legacy_assessment_receipts
                .get(revision_id.as_bytes())
                .map_err(to_storage_io)?
            {
                if existing.as_ref() != encoded.as_slice() {
                    return Err(StorageError::InvalidPath(
                        "legacy belief assessment receipt conflicts during migration".to_string(),
                    ));
                }
            }
            migrated_revision_ids.push(revision_id.clone());
            migrated_receipts.push((revision_id.clone(), encoded));
        }
        let fence = LegacyAssessmentMigrationFence {
            schema_version: LEGACY_ASSESSMENT_MIGRATION_SCHEMA_VERSION,
            source_snapshot,
            migrated_revision_ids,
        };
        validate_legacy_assessment_migration_fence(&fence)?;
        let fence_bytes = serde_json::to_vec(&fence).map_err(to_storage_data)?;
        use sled::transaction::{ConflictableTransactionError, TransactionError};
        (&self.legacy_assessment_receipts, &self.authority_meta)
            .transaction(|(receipts, authority_meta)| {
                if authority_meta
                    .get(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE)?
                    .is_some()
                    || receipts
                        .get(KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE)?
                        .is_some()
                {
                    return Err(ConflictableTransactionError::Abort(
                        "legacy belief assessment migration fence changed concurrently".to_string(),
                    ));
                }
                for (revision_id, encoded) in &migrated_receipts {
                    if let Some(existing) = receipts.get(revision_id.as_bytes())? {
                        if existing.as_ref() != encoded.as_slice() {
                            return Err(ConflictableTransactionError::Abort(
                                "legacy belief assessment receipt conflicts during migration"
                                    .to_string(),
                            ));
                        }
                    } else {
                        receipts.insert(revision_id.as_bytes(), encoded.as_slice())?;
                    }
                }
                authority_meta.insert(
                    KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE,
                    fence_bytes.as_slice(),
                )?;
                receipts.insert(
                    KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE,
                    fence_bytes.as_slice(),
                )?;
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::InvalidPath(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        Ok(true)
    }

    fn verify_current_commit_receipt(
        &self,
        revision: &BeliefRevision,
    ) -> Result<Option<String>, StorageError> {
        let intent_id = revision_id_to_intent_id(&revision.revision_id);
        let Some(raw) = self
            .commit_receipts
            .get(intent_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let receipt: BeliefCommitIntent = serde_json::from_slice(&raw).map_err(to_storage_data)?;
        if receipt.intent_id() != intent_id || receipt.revision() != revision {
            return Err(StorageError::InvalidPath(
                "belief commit receipt does not match its revision".to_string(),
            ));
        }
        if self
            .get_lease(&receipt.completed_lease().lease_id)?
            .as_ref()
            != Some(receipt.completed_lease())
        {
            return Err(StorageError::InvalidPath(
                "belief commit receipt lost its exact completed lease".to_string(),
            ));
        }
        let progress_receipt_key = assessment_progress_receipt_key("commit", &intent_id);
        if self
            .exact_assessment_progress_receipt(&progress_receipt_key)?
            .is_none()
        {
            return Err(StorageError::InvalidPath(
                "belief commit receipt is missing exact progress".to_string(),
            ));
        }
        Ok(Some(receipt.completed_lease().lease_id.clone()))
    }

    fn load_legacy_assessment_receipt(
        &self,
        revision: &BeliefRevision,
    ) -> Result<LegacyAssessmentReceipt, StorageError> {
        let raw = self
            .legacy_assessment_receipts
            .get(revision.revision_id.as_bytes())
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "legacy belief assessment migration lost its exact receipt".to_string(),
                )
            })?;
        let receipt: LegacyAssessmentReceipt =
            serde_json::from_slice(&raw).map_err(to_storage_data)?;
        validate_legacy_assessment_receipt(&receipt, revision)?;
        if self.get_lease(&receipt.completed_lease.lease_id)?.as_ref()
            != Some(&receipt.completed_lease)
        {
            return Err(StorageError::InvalidPath(
                "legacy belief assessment receipt lost its exact completed lease".to_string(),
            ));
        }
        Ok(receipt)
    }

    /// Read the current revision for one key.
    pub fn current_revision(
        &self,
        key: &BeliefKey,
    ) -> Result<Option<BeliefRevision>, StorageError> {
        let Some(raw) = self
            .revision_head
            .get(key.index_key().as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let revision_id = String::from_utf8(raw.to_vec()).map_err(to_storage_utf8)?;
        let revision = self.get_revision(&revision_id)?.ok_or_else(|| {
            StorageError::InvalidPath(format!(
                "belief revision head references missing revision '{revision_id}'"
            ))
        })?;
        if revision.revision_id != revision_id || revision.belief_key != *key {
            return Err(StorageError::InvalidPath(format!(
                "belief revision head conflicts with revision '{revision_id}'"
            )));
        }
        Ok(Some(revision))
    }

    /// Read one revision by id.
    pub fn get_revision(&self, revision_id: &str) -> Result<Option<BeliefRevision>, StorageError> {
        decode_optional(
            self.revisions
                .get(revision_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    pub(crate) fn committed_revision_for_evidence(
        &self,
        key: &BeliefKey,
        evidence_id: &str,
    ) -> Result<Option<BeliefRevision>, StorageError> {
        let index_key = committed_evidence_index_key(key, evidence_id);
        let Some(revision_id) = self
            .committed_evidence
            .get(index_key)
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let revision_id = String::from_utf8(revision_id.to_vec()).map_err(to_storage_utf8)?;
        let revision = self.get_revision(&revision_id)?.ok_or_else(|| {
            StorageError::InvalidPath(
                "belief committed evidence references a missing revision".to_string(),
            )
        })?;
        if revision.belief_key != *key
            || !revision
                .evidence_ids
                .iter()
                .any(|candidate| candidate == evidence_id)
        {
            return Err(StorageError::InvalidPath(
                "belief committed evidence does not match its revision".to_string(),
            ));
        }
        Ok(Some(revision))
    }

    pub(crate) fn exact_commit_progress_for_revision(
        &self,
        revision: &BeliefRevision,
    ) -> Result<Option<u64>, StorageError> {
        let intent_id = revision_id_to_intent_id(&revision.revision_id);
        let Some(receipt_raw) = self
            .commit_receipts
            .get(intent_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            if self
                .legacy_assessment_receipts
                .get(revision.revision_id.as_bytes())
                .map_err(to_storage_io)?
                .is_none()
            {
                return Ok(None);
            }
            let legacy = self.load_legacy_assessment_receipt(revision)?;
            self.flush().map_err(|error| {
                StorageError::DurabilityIndeterminate(format!(
                    "legacy belief subject replay flush failed: {error}"
                ))
            })?;
            return Ok(Some(legacy.legacy_progress_sequence));
        };
        let receipt: BeliefCommitIntent =
            serde_json::from_slice(&receipt_raw).map_err(to_storage_data)?;
        if receipt.intent_id() != intent_id || receipt.revision() != revision {
            return Err(StorageError::InvalidPath(
                "belief commit receipt does not match its revision".to_string(),
            ));
        }
        let progress_receipt_key = assessment_progress_receipt_key("commit", &intent_id);
        let progress = self
            .exact_assessment_progress_receipt(&progress_receipt_key)?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "belief commit receipt is missing exact progress".to_string(),
                )
            })?;
        self.flush().map_err(|error| {
            StorageError::DurabilityIndeterminate(format!(
                "belief subject assessment exact replay flush failed: {error}"
            ))
        })?;
        Ok(Some(progress))
    }

    /// Read the complete revision chain for one key in causal order.
    pub fn revision_history(&self, key: &BeliefKey) -> Result<Vec<BeliefRevision>, StorageError> {
        let mut by_id = BTreeMap::new();
        for item in self.revisions.iter() {
            let (_, value) = item.map_err(to_storage_io)?;
            let revision: BeliefRevision =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            if revision.belief_key == *key
                && by_id
                    .insert(revision.revision_id.clone(), revision)
                    .is_some()
            {
                return Err(StorageError::InvalidPath(
                    "belief revision history contains a duplicate identity".to_string(),
                ));
            }
        }
        let Some(mut current_id) = self
            .revision_head
            .get(key.index_key().as_bytes())
            .map_err(to_storage_io)?
            .map(|raw| String::from_utf8(raw.to_vec()).map_err(to_storage_utf8))
            .transpose()?
        else {
            if by_id.is_empty() {
                return Ok(Vec::new());
            }
            // TODO compat-shim: remove unheaded revision fallback after every
            // supported semantic fixture and migrated store persists revision
            // heads. It preserves the pre-head query shape. Keep agent
            // selection fixtures and causal late-evidence proofs green.
            let mut legacy = by_id.into_values().collect::<Vec<_>>();
            legacy.sort_by(|left, right| {
                left.source_cursor_end
                    .cmp(&right.source_cursor_end)
                    .then_with(|| left.revision_id.cmp(&right.revision_id))
            });
            return Ok(legacy);
        };
        let mut newest_first = Vec::with_capacity(by_id.len());
        loop {
            let revision = by_id.remove(&current_id).ok_or_else(|| {
                StorageError::InvalidPath(
                    "belief revision history has a missing or cyclic prior link".to_string(),
                )
            })?;
            let prior = revision.prior_revision_id.clone();
            newest_first.push(revision);
            let Some(prior) = prior else {
                break;
            };
            current_id = prior;
        }
        if !by_id.is_empty() {
            return Err(StorageError::InvalidPath(
                "belief revision history contains a divergent branch".to_string(),
            ));
        }
        newest_first.reverse();
        Ok(newest_first)
    }

    /// Store the current planner-safe view for one key.
    pub fn put_view(&self, view: &BeliefView) -> Result<(), StorageError> {
        let _write = self.writable_guard()?;
        let key = view.key.index_key();
        let subject_key = view_subject_index_key(view);
        let encoded = serde_json::to_vec(view).map_err(to_storage_data)?;
        use sled::transaction::TransactionError;
        (&self.views, &self.view_by_subject)
            .transaction(|(views, subject_views)| {
                views.insert(key.as_bytes(), encoded.as_slice())?;
                subject_views.insert(subject_key.as_bytes(), key.as_bytes())?;
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Abort(()) => {
                    StorageError::InvalidPath("belief view transaction aborted".to_string())
                }
                TransactionError::Storage(error) => to_storage_io(error),
            })
    }

    /// Read the current view for one key.
    pub fn current_view(&self, key: &BeliefKey) -> Result<Option<BeliefView>, StorageError> {
        decode_optional(
            self.views
                .get(key.index_key().as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Prepare an attestation intent bound to one exact agent owner snapshot.
    pub(crate) fn prepare_readiness_attestation_fenced(
        &self,
        request: &BeliefReadinessAttestationRequest,
        owner_fence_hash: &str,
    ) -> Result<BeliefReadinessAttestation, StorageError> {
        validate_readiness_owner_hash(owner_fence_hash)?;
        self.prepare_readiness_attestation_inner(request, owner_fence_hash)
    }

    fn prepare_readiness_attestation_inner(
        &self,
        request: &BeliefReadinessAttestationRequest,
        owner_fence_hash: &str,
    ) -> Result<BeliefReadinessAttestation, StorageError> {
        request.validate()?;
        let _write = self.writable_guard()?;
        let key = request.belief_key.index_key();
        let head = self
            .revision_head
            .get(key.as_bytes())
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "belief readiness attestation requires a current revision".to_string(),
                )
            })?;
        if head.as_ref() != request.expected_revision_id.as_bytes() {
            return Err(StorageError::Backpressure(
                "belief revision head changed before readiness attestation".to_string(),
            ));
        }
        let revision_raw = self
            .revisions
            .get(request.expected_revision_id.as_bytes())
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "belief readiness revision is missing from durable history".to_string(),
                )
            })?;
        let revision: BeliefRevision =
            serde_json::from_slice(&revision_raw).map_err(to_storage_data)?;
        if revision.belief_key != request.belief_key {
            return Err(StorageError::InvalidPath(
                "belief readiness revision belongs to another key".to_string(),
            ));
        }
        let view_raw = self
            .views
            .get(key.as_bytes())
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "belief readiness attestation requires a persisted current view".to_string(),
                )
            })?;
        let view: BeliefView = serde_json::from_slice(&view_raw).map_err(to_storage_data)?;
        if view.key != request.belief_key
            || view.current_revision_id.as_deref() != Some(request.expected_revision_id.as_str())
            || view.view_id.trim().is_empty()
        {
            return Err(StorageError::Backpressure(
                "persisted belief view does not match the readiness revision".to_string(),
            ));
        }
        let attestation = BeliefReadinessAttestation::identified(request, &revision, &view)?;
        let snapshot = BeliefReadinessSnapshot {
            attestation: attestation.clone(),
            revision,
            view,
        };
        snapshot.validate()?;
        let attestation_bytes = serde_json::to_vec(&attestation).map_err(to_storage_data)?;
        let snapshot_bytes = serde_json::to_vec(&snapshot).map_err(to_storage_data)?;

        use sled::transaction::{ConflictableTransactionError, TransactionError};
        (
            &self.revision_head,
            &self.revisions,
            &self.views,
            &self.readiness_attestation_intents,
            &self.readiness_attestation_owner_fences,
            &self.readiness_attestation_snapshots,
        )
            .transaction(
                |(heads, revisions, views, intents, owner_fences, snapshots)| {
                    require_belief_transaction_value(
                        heads,
                        key.as_bytes(),
                        request.expected_revision_id.as_bytes(),
                        "belief revision head",
                    )?;
                    require_belief_transaction_value(
                        revisions,
                        request.expected_revision_id.as_bytes(),
                        revision_raw.as_ref(),
                        "belief revision",
                    )?;
                    require_belief_transaction_value(
                        views,
                        key.as_bytes(),
                        view_raw.as_ref(),
                        "belief current view",
                    )?;
                    let intent_was_present = if let Some(existing) =
                        intents.get(attestation.attestation_id.as_bytes())?
                    {
                        if existing.as_ref() != attestation_bytes.as_slice() {
                            return Err(ConflictableTransactionError::Abort(
                                "belief readiness attestation identity conflict".to_string(),
                            ));
                        }
                        true
                    } else {
                        intents.insert(
                            attestation.attestation_id.as_bytes(),
                            attestation_bytes.as_slice(),
                        )?;
                        false
                    };
                    match owner_fences.get(attestation.attestation_id.as_bytes())? {
                        Some(existing) if existing.as_ref() == owner_fence_hash.as_bytes() => {}
                        Some(_) => {
                            return Err(ConflictableTransactionError::Abort(
                                "belief readiness attestation owner fence conflict".to_string(),
                            ));
                        }
                        None if intent_was_present => {
                            return Err(ConflictableTransactionError::Abort(
                                "readiness attestation is missing its owner fence".to_string(),
                            ));
                        }
                        None => {
                            owner_fences.insert(
                                attestation.attestation_id.as_bytes(),
                                owner_fence_hash.as_bytes(),
                            )?;
                        }
                    }
                    match snapshots.get(attestation.attestation_id.as_bytes())? {
                        Some(existing) if existing.as_ref() != snapshot_bytes.as_slice() => {
                            return Err(ConflictableTransactionError::Abort(
                                "belief readiness snapshot identity conflict".to_string(),
                            ));
                        }
                        Some(_) => {}
                        None => {
                            snapshots.insert(
                                attestation.attestation_id.as_bytes(),
                                snapshot_bytes.as_slice(),
                            )?;
                        }
                    }
                    Ok(())
                },
            )
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::Backpressure(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.flush().map_err(|error| {
            StorageError::DurabilityIndeterminate(format!(
                "belief readiness attestation intent flush failed: {error}"
            ))
        })?;
        Ok(attestation)
    }

    // TODO compat-shim: remove this rejected one-call surface after downstream
    // callers compile against capability-fenced hydration and the public API
    // compatibility test no longer requires the historical method to exist.
    /// Reject the legacy one-call surface because it carries no owner capability.
    pub fn attest_current_view(
        &self,
        request: &BeliefReadinessAttestationRequest,
    ) -> Result<BeliefReadinessAttestation, StorageError> {
        request.validate()?;
        Err(StorageError::InvalidPath(
            "belief readiness attestation requires an exact agent hydration capability".to_string(),
        ))
    }

    /// Make one prepared attestation visible under an exact agent owner fence.
    pub(crate) fn activate_prepared_readiness_attestation(
        &self,
        expected: &BeliefReadinessAttestation,
        fence: &AgentHydrationFenceCapability,
    ) -> Result<BeliefReadinessAttestation, StorageError> {
        expected.validate()?;
        let _write = self.writable_guard()?;
        let encoded = serde_json::to_vec(expected).map_err(to_storage_data)?;
        use sled::transaction::ConflictableTransactionError;
        fence.transaction_three(
            &self.readiness_attestation_intents,
            &self.readiness_attestation_owner_fences,
            &self.readiness_attestations,
            |intents, owner_fences, attestations| {
                if !fence.matches_readiness_scope(
                    &expected.agent_id,
                    &expected.subscription_id,
                    &expected.belief_key,
                ) {
                    return Err(ConflictableTransactionError::Abort(
                        "readiness attestation scope conflicts with its owner capability"
                            .to_string(),
                    ));
                }
                require_belief_transaction_value(
                    intents,
                    expected.attestation_id.as_bytes(),
                    &encoded,
                    "belief readiness attestation intent",
                )?;
                require_belief_transaction_value(
                    owner_fences,
                    expected.attestation_id.as_bytes(),
                    fence.owner_hash().as_bytes(),
                    "belief readiness attestation owner fence",
                )?;
                if let Some(existing) = attestations.get(expected.attestation_id.as_bytes())? {
                    if existing.as_ref() != encoded.as_slice() {
                        return Err(ConflictableTransactionError::Abort(
                            "belief readiness attestation identity conflict".to_string(),
                        ));
                    }
                } else {
                    attestations.insert(expected.attestation_id.as_bytes(), encoded.as_slice())?;
                }
                Ok(())
            },
        )?;
        self.flush().map_err(|error| {
            StorageError::DurabilityIndeterminate(format!(
                "belief readiness attestation activation flush failed: {error}"
            ))
        })?;
        Ok(expected.clone())
    }

    /// Claim one migrated or legacy-visible attestation under an exact owner capability.
    pub(crate) fn claim_readiness_attestation_fenced(
        &self,
        expected: &BeliefReadinessAttestation,
        fence: &AgentHydrationFenceCapability,
    ) -> Result<BeliefReadinessAttestation, StorageError> {
        expected.validate()?;
        let _write = self.writable_guard()?;
        if expected.requires_legacy_upgrade() {
            return Err(StorageError::MigrationConflict(
                "legacy readiness attestation was not upgraded before owner claim".to_string(),
            ));
        }
        let encoded = serde_json::to_vec(expected).map_err(to_storage_data)?;
        let snapshot = self
            .readiness_snapshot(&expected.attestation_id)?
            .ok_or_else(|| {
                StorageError::MigrationConflict(
                    "readiness owner claim requires an upgraded immutable snapshot".to_string(),
                )
            })?;
        let snapshot_bytes = serde_json::to_vec(&snapshot).map_err(to_storage_data)?;
        let visible_raw = self
            .readiness_attestations
            .get(expected.attestation_id.as_bytes())
            .map_err(to_storage_io)?;
        if visible_raw
            .as_ref()
            .is_some_and(|raw| raw.as_ref() != encoded.as_slice())
        {
            return Err(StorageError::MigrationConflict(
                "readiness owner claim found divergent visible content".to_string(),
            ));
        }
        let visible_expected = visible_raw.as_ref().map(|raw| raw.as_ref());
        use sled::transaction::ConflictableTransactionError;
        fence.transaction_four(
            &self.readiness_attestation_intents,
            &self.readiness_attestation_owner_fences,
            &self.readiness_attestation_snapshots,
            &self.readiness_attestations,
            |intents, owner_fences, snapshots, attestations| {
                if !fence.matches_readiness_scope(
                    &expected.agent_id,
                    &expected.subscription_id,
                    &expected.belief_key,
                ) {
                    return Err(ConflictableTransactionError::Abort(
                        "readiness attestation scope conflicts with its owner capability"
                            .to_string(),
                    ));
                }
                require_belief_transaction_value(
                    intents,
                    expected.attestation_id.as_bytes(),
                    &encoded,
                    "belief readiness attestation intent",
                )?;
                require_belief_transaction_value(
                    snapshots,
                    expected.attestation_id.as_bytes(),
                    &snapshot_bytes,
                    "belief readiness attestation snapshot",
                )?;
                require_optional_belief_transaction_value(
                    attestations,
                    expected.attestation_id.as_bytes(),
                    visible_expected,
                    "belief readiness attestation",
                )?;
                // TODO compat-shim: remove the sentinel claim branch after all
                // accepted W3A identities have exact capability claims and the
                // legacy reopen fixture remains green without it.
                match owner_fences.get(expected.attestation_id.as_bytes())? {
                    Some(existing)
                        if existing.as_ref() == b"legacy-unfenced"
                            || existing.as_ref() == fence.owner_hash().as_bytes() =>
                    {
                        if existing.as_ref() == b"legacy-unfenced"
                            && !expected.has_legacy_identity()
                        {
                            return Err(ConflictableTransactionError::Abort(
                                "current readiness attestation has no exact owner fence"
                                    .to_string(),
                            ));
                        }
                        owner_fences.insert(
                            expected.attestation_id.as_bytes(),
                            fence.owner_hash().as_bytes(),
                        )?;
                    }
                    Some(_) => {
                        return Err(ConflictableTransactionError::Abort(
                            "belief readiness attestation owner fence conflict".to_string(),
                        ));
                    }
                    None => {
                        return Err(ConflictableTransactionError::Abort(
                            "readiness attestation is missing its owner fence".to_string(),
                        ));
                    }
                }
                Ok(())
            },
        )?;
        self.flush().map_err(|error| {
            StorageError::DurabilityIndeterminate(format!(
                "belief readiness owner claim flush failed: {error}"
            ))
        })?;
        Ok(expected.clone())
    }

    /// Read one prepared but not necessarily visible readiness attestation.
    pub(crate) fn get_prepared_readiness_attestation(
        &self,
        attestation_id: &str,
    ) -> Result<Option<BeliefReadinessAttestation>, StorageError> {
        let mut attestation: Option<BeliefReadinessAttestation> = decode_optional(
            self.readiness_attestation_intents
                .get(attestation_id.as_bytes())
                .map_err(to_storage_io)?,
        )?;
        if attestation
            .as_ref()
            .is_some_and(BeliefReadinessAttestation::requires_legacy_upgrade)
        {
            // TODO compat-shim: remove this lazy v1 rewrite after late-inserted W3A
            // fixture coverage is retired and all supported stores carry a complete
            // readiness schema marker produced after the final v1-capable release.
            let _migration = self.write_gate.write();
            self.migrate_readiness_identity(attestation_id.as_bytes())?;
            attestation = decode_optional(
                self.readiness_attestation_intents
                    .get(attestation_id.as_bytes())
                    .map_err(to_storage_io)?,
            )?;
        }
        if let Some(attestation) = attestation.as_ref() {
            attestation.validate()?;
            if attestation.attestation_id != attestation_id {
                return Err(StorageError::MigrationConflict(
                    "prepared readiness attestation key conflicts with its identity".to_string(),
                ));
            }
            self.validate_readiness_products(attestation, false)?;
        }
        Ok(attestation)
    }

    /// Read one belief-owned readiness attestation by deterministic identity.
    pub fn get_readiness_attestation(
        &self,
        attestation_id: &str,
    ) -> Result<Option<BeliefReadinessAttestation>, StorageError> {
        let mut attestation: Option<BeliefReadinessAttestation> = decode_optional(
            self.readiness_attestations
                .get(attestation_id.as_bytes())
                .map_err(to_storage_io)?,
        )?;
        if attestation
            .as_ref()
            .is_some_and(BeliefReadinessAttestation::requires_legacy_upgrade)
        {
            // TODO compat-shim: remove this lazy v1 rewrite after late-inserted W3A
            // fixture coverage is retired and all supported stores carry a complete
            // readiness schema marker produced after the final v1-capable release.
            let _migration = self.write_gate.write();
            self.migrate_readiness_identity(attestation_id.as_bytes())?;
            attestation = decode_optional(
                self.readiness_attestations
                    .get(attestation_id.as_bytes())
                    .map_err(to_storage_io)?,
            )?;
        }
        if let Some(attestation) = attestation.as_ref() {
            attestation.validate()?;
            if attestation.attestation_id != attestation_id {
                return Err(StorageError::MigrationConflict(
                    "readiness attestation key conflicts with its identity".to_string(),
                ));
            }
            self.validate_readiness_products(attestation, true)?;
        }
        Ok(attestation)
    }

    fn validate_readiness_products(
        &self,
        expected: &BeliefReadinessAttestation,
        require_visible: bool,
    ) -> Result<(), StorageError> {
        expected.validate()?;
        let key = expected.attestation_id.as_bytes();
        let encoded = serde_json::to_vec(expected).map_err(to_storage_data)?;
        let intent = self
            .readiness_attestation_intents
            .get(key)
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::MigrationConflict(
                    "readiness attestation is missing its durable intent".to_string(),
                )
            })?;
        if intent.as_ref() != encoded.as_slice() {
            return Err(StorageError::MigrationConflict(
                "readiness attestation intent diverges from its visible product".to_string(),
            ));
        }
        let snapshot_raw = self
            .readiness_attestation_snapshots
            .get(key)
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::MigrationConflict(
                    "readiness attestation is missing its immutable snapshot".to_string(),
                )
            })?;
        let snapshot: BeliefReadinessSnapshot =
            serde_json::from_slice(&snapshot_raw).map_err(to_storage_data)?;
        snapshot.validate()?;
        if snapshot.attestation != *expected {
            return Err(StorageError::MigrationConflict(
                "readiness attestation snapshot diverges from its identity".to_string(),
            ));
        }
        let owner = self
            .readiness_attestation_owner_fences
            .get(key)
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::MigrationConflict(
                    "readiness attestation is missing its owner fence".to_string(),
                )
            })?;
        if owner.as_ref() == b"legacy-unfenced" {
            // TODO compat-shim: remove sentinel acceptance after all accepted W3A
            // identities have exact capability claims and reopen migration tests
            // remain green without legacy owner products.
            if !expected.has_legacy_identity() {
                return Err(StorageError::MigrationConflict(
                    "current readiness attestation has no exact owner fence".to_string(),
                ));
            }
        } else {
            let owner = std::str::from_utf8(owner.as_ref()).map_err(|error| {
                StorageError::MigrationConflict(format!(
                    "readiness owner fence is not UTF-8: {error}"
                ))
            })?;
            validate_readiness_owner_hash(owner)?;
        }
        if require_visible
            && self
                .readiness_attestations
                .get(key)
                .map_err(to_storage_io)?
                .as_deref()
                != Some(encoded.as_slice())
        {
            return Err(StorageError::MigrationConflict(
                "readiness attestation visible product diverges from its identity".to_string(),
            ));
        }
        Ok(())
    }

    /// Read and verify the exact immutable snapshot behind one attestation.
    pub fn readiness_snapshot(
        &self,
        attestation_id: &str,
    ) -> Result<Option<BeliefReadinessSnapshot>, StorageError> {
        let snapshot: Option<BeliefReadinessSnapshot> = decode_optional(
            self.readiness_attestation_snapshots
                .get(attestation_id.as_bytes())
                .map_err(to_storage_io)?,
        )?;
        if let Some(snapshot) = snapshot.as_ref() {
            snapshot.validate()?;
            if snapshot.attestation.attestation_id != attestation_id {
                return Err(StorageError::InvalidPath(
                    "readiness snapshot tree key conflicts with embedded attestation identity"
                        .to_string(),
                ));
            }
        }
        Ok(snapshot)
    }

    /// Verify that one exact immutable readiness attestation is durable.
    pub fn verify_readiness_attestation(
        &self,
        expected: &BeliefReadinessAttestation,
    ) -> Result<(), StorageError> {
        expected.validate()?;
        if self
            .get_readiness_attestation(&expected.attestation_id)?
            .as_ref()
            != Some(expected)
        {
            return Err(StorageError::Backpressure(format!(
                "belief readiness attestation '{}' changed or is missing",
                expected.attestation_id
            )));
        }
        let snapshot = self
            .readiness_snapshot(&expected.attestation_id)?
            .ok_or_else(|| {
                StorageError::Backpressure(format!(
                    "belief readiness snapshot '{}' is missing",
                    expected.attestation_id
                ))
            })?;
        if snapshot.attestation != *expected {
            return Err(StorageError::Backpressure(format!(
                "belief readiness snapshot '{}' changed",
                expected.attestation_id
            )));
        }
        Ok(())
    }

    fn migrate_legacy_readiness_attestations(&self) -> Result<(), StorageError> {
        let _migration = self.write_gate.write();
        let mut state = match self
            .readiness_schema
            .get(KEY_READINESS_SCHEMA_STATE)
            .map_err(to_storage_io)?
        {
            Some(raw) => serde_json::from_slice(&raw).map_err(to_storage_data)?,
            None => {
                let initial = if self.readiness_attestation_intents.is_empty()
                    && self.readiness_attestations.is_empty()
                {
                    ReadinessMigrationState::complete()
                } else {
                    ReadinessMigrationState::initial()
                };
                self.put_readiness_migration_state(&initial)?;
                initial
            }
        };
        state.validate()?;
        while state.phase != ReadinessMigrationPhase::Complete {
            state = self.advance_readiness_migration(state)?;
        }
        Ok(())
    }

    fn advance_readiness_migration(
        &self,
        mut state: ReadinessMigrationState,
    ) -> Result<ReadinessMigrationState, StorageError> {
        state.validate()?;
        let tree = match state.phase {
            ReadinessMigrationPhase::Intents => &self.readiness_attestation_intents,
            ReadinessMigrationPhase::Visible => &self.readiness_attestations,
            ReadinessMigrationPhase::Complete => return Ok(state),
        };
        let mut keys = Vec::with_capacity(READINESS_MIGRATION_BATCH);
        match state.cursor.as_deref() {
            Some(cursor) => {
                for item in tree
                    .range::<&[u8], _>((Bound::Excluded(cursor), Bound::Unbounded))
                    .take(READINESS_MIGRATION_BATCH)
                {
                    let (key, _) = item.map_err(to_storage_io)?;
                    keys.push(key.to_vec());
                }
            }
            None => {
                for item in tree.iter().take(READINESS_MIGRATION_BATCH) {
                    let (key, _) = item.map_err(to_storage_io)?;
                    keys.push(key.to_vec());
                }
            }
        }
        if keys.is_empty() {
            state.phase = match state.phase {
                ReadinessMigrationPhase::Intents => ReadinessMigrationPhase::Visible,
                ReadinessMigrationPhase::Visible => ReadinessMigrationPhase::Complete,
                ReadinessMigrationPhase::Complete => ReadinessMigrationPhase::Complete,
            };
            state.cursor = None;
        } else {
            for key in &keys {
                self.migrate_readiness_identity(key)?;
            }
            state.cursor = keys.last().cloned();
        }
        self.put_readiness_migration_state(&state)?;
        Ok(state)
    }

    fn put_readiness_migration_state(
        &self,
        state: &ReadinessMigrationState,
    ) -> Result<(), StorageError> {
        state.validate()?;
        self.readiness_schema
            .insert(
                KEY_READINESS_SCHEMA_STATE,
                serde_json::to_vec(state).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.flush().map_err(|error| {
            StorageError::DurabilityIndeterminate(format!(
                "belief readiness schema migration flush failed: {error}"
            ))
        })
    }

    fn migrate_readiness_identity(&self, key: &[u8]) -> Result<(), StorageError> {
        let intent_raw = self
            .readiness_attestation_intents
            .get(key)
            .map_err(to_storage_io)?;
        let visible_raw = self
            .readiness_attestations
            .get(key)
            .map_err(to_storage_io)?;
        let source_raw = visible_raw
            .as_ref()
            .or(intent_raw.as_ref())
            .ok_or_else(|| {
                StorageError::MigrationConflict(
                    "readiness migration identity disappeared during scan".to_string(),
                )
            })?;
        let legacy: BeliefReadinessAttestation =
            serde_json::from_slice(source_raw).map_err(to_storage_data)?;
        legacy.validate()?;
        if key != legacy.attestation_id.as_bytes() {
            return Err(StorageError::MigrationConflict(
                "readiness attestation tree key conflicts with its identity".to_string(),
            ));
        }
        if !legacy.requires_legacy_upgrade() {
            return self.validate_readiness_products(&legacy, visible_raw.is_some());
        }
        for raw in [intent_raw.as_ref(), visible_raw.as_ref()]
            .into_iter()
            .flatten()
        {
            let decoded: BeliefReadinessAttestation =
                serde_json::from_slice(raw).map_err(to_storage_data)?;
            if decoded != legacy {
                return Err(StorageError::MigrationConflict(
                    "legacy readiness intent and visible record diverge".to_string(),
                ));
            }
        }
        let revision_raw = self
            .revisions
            .get(legacy.belief_revision_id.as_bytes())
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::MigrationConflict(
                    "legacy readiness attestation references a missing revision".to_string(),
                )
            })?;
        let revision: BeliefRevision =
            serde_json::from_slice(&revision_raw).map_err(to_storage_data)?;
        let view = self.legacy_readiness_view(&legacy, &revision)?;
        let upgraded = legacy.clone().upgrade_legacy(&revision, &view)?;
        let snapshot = BeliefReadinessSnapshot {
            attestation: upgraded.clone(),
            revision,
            view,
        };
        snapshot.validate()?;
        let upgraded_bytes = serde_json::to_vec(&upgraded).map_err(to_storage_data)?;
        let snapshot_bytes = serde_json::to_vec(&snapshot).map_err(to_storage_data)?;
        let intent_expected = intent_raw.as_ref().map(|raw| raw.as_ref());
        let visible_expected = visible_raw.as_ref().map(|raw| raw.as_ref());
        use sled::transaction::{ConflictableTransactionError, TransactionError};
        (
            &self.revisions,
            &self.readiness_attestation_intents,
            &self.readiness_attestation_owner_fences,
            &self.readiness_attestation_snapshots,
            &self.readiness_attestations,
        )
            .transaction(
                |(revisions, intents, owner_fences, snapshots, attestations)| {
                    require_belief_transaction_value(
                        revisions,
                        legacy.belief_revision_id.as_bytes(),
                        revision_raw.as_ref(),
                        "legacy readiness revision",
                    )?;
                    require_optional_belief_transaction_value(
                        intents,
                        key,
                        intent_expected,
                        "legacy readiness intent",
                    )?;
                    require_optional_belief_transaction_value(
                        attestations,
                        key,
                        visible_expected,
                        "legacy readiness attestation",
                    )?;
                    match snapshots.get(key)? {
                        Some(existing) if existing.as_ref() != snapshot_bytes.as_slice() => {
                            return Err(ConflictableTransactionError::Abort(
                                "legacy readiness snapshot conflicts with migration".to_string(),
                            ));
                        }
                        Some(_) => {}
                        None => {
                            snapshots.insert(key, snapshot_bytes.as_slice())?;
                        }
                    }
                    match owner_fences.get(key)? {
                        Some(existing) if existing.as_ref() != b"legacy-unfenced" => {
                            validate_readiness_owner_hash_transaction(existing.as_ref())?;
                        }
                        Some(_) => {}
                        None => {
                            // TODO compat-shim: remove the v1 owner sentinel only after every
                            // accepted W3A record has an exact capability claim and migration,
                            // reopen, and stale-owner tests remain green without this branch.
                            owner_fences.insert(key, b"legacy-unfenced")?;
                        }
                    }
                    if intent_expected.is_some() || visible_expected.is_some() {
                        intents.insert(key, upgraded_bytes.as_slice())?;
                    }
                    if visible_expected.is_some() {
                        attestations.insert(key, upgraded_bytes.as_slice())?;
                    }
                    Ok(())
                },
            )
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::MigrationConflict(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.validate_readiness_products(&upgraded, visible_expected.is_some())
    }

    fn legacy_readiness_view(
        &self,
        legacy: &BeliefReadinessAttestation,
        revision: &BeliefRevision,
    ) -> Result<BeliefView, StorageError> {
        if let Some(view) = self.current_view(&legacy.belief_key)? {
            if view.view_id == legacy.belief_view_id
                && hash_readiness_view(&view)? == legacy.belief_view_hash
            {
                return Ok(view);
            }
        }
        let hydration = HydrationRefs {
            evidence_ids: revision.evidence_ids.clone(),
            source_fact_ids: revision.provenance.source_fact_ids.clone(),
            graph_anchor_ids: revision.provenance.graph_anchor_ids.clone(),
            revision_id: Some(revision.revision_id.clone()),
        };
        let projected = self.project_view(revision, hydration);
        if projected.view_id == legacy.belief_view_id
            && hash_readiness_view(&projected)? == legacy.belief_view_hash
        {
            Ok(projected)
        } else {
            Err(StorageError::MigrationConflict(
                "legacy readiness view cannot be reconstructed with hash parity".to_string(),
            ))
        }
    }

    /// Read current views for a subject and perspective.
    pub fn views_for_subject(
        &self,
        subject: &DomainObjectRef,
        perspective: &PerspectiveKey,
    ) -> Result<Vec<BeliefView>, StorageError> {
        let prefix = format!("{}::{}::", subject.index_key(), perspective.index_key());
        let mut out = Vec::new();
        for item in self.view_by_subject.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item.map_err(to_storage_io)?;
            let key = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(raw) = self.views.get(key.as_bytes()).map_err(to_storage_io)? {
                out.push(serde_json::from_slice(&raw).map_err(to_storage_data)?);
            }
        }
        Ok(out)
    }

    /// Hydrate evidence used by one revision.
    pub fn evidence_by_revision(
        &self,
        revision_id: &str,
    ) -> Result<Vec<EvidenceItem>, StorageError> {
        let Some(revision) = self.get_revision(revision_id)? else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for evidence_id in revision.evidence_ids {
            if let Some(item) = self.get_evidence(&evidence_id)? {
                out.push(item);
            }
        }
        Ok(out)
    }

    /// Read compact provenance for one revision.
    pub fn provenance_by_revision(
        &self,
        revision_id: &str,
    ) -> Result<Option<BeliefProvenanceSummary>, StorageError> {
        Ok(self
            .get_revision(revision_id)?
            .map(|revision| revision.provenance))
    }

    /// Read open observation opportunities for a subject and perspective.
    pub fn open_observation_opportunities(
        &self,
        subject: &DomainObjectRef,
        perspective: &PerspectiveKey,
    ) -> Result<Vec<ObservationOpportunity>, StorageError> {
        Ok(self
            .views_for_subject(subject, perspective)?
            .into_iter()
            .filter_map(|view| view.observation)
            .filter(|observation| observation.open)
            .collect())
    }

    /// Mark a current view stale when newer assigned evidence exists.
    pub fn mark_stale_if_newer_evidence(
        &self,
        key: &BeliefKey,
    ) -> Result<Option<BeliefView>, StorageError> {
        let Some(mut view) = self.current_view(key)? else {
            return Ok(None);
        };
        let high_water = view.freshness.high_water_seq;
        let newer = self
            .evidence_for_key(key)?
            .into_iter()
            .any(|item| item.source_cursor_end > high_water);
        if newer {
            view.status = BeliefStatus::Stale;
            view.freshness.stale = true;
            push_reason(&mut view.freshness.reasons, FreshnessReason::NewerEvidence);
            view.observation = Some(stale_observation(
                key,
                view.current_revision_id.clone(),
                FreshnessReason::NewerEvidence,
            ));
            self.put_view(&view)?;
        }
        Ok(Some(view))
    }

    /// Recompute current view freshness from evidence, graph, config, and policy state.
    pub fn refresh_view_freshness(
        &self,
        key: &BeliefKey,
        active_config_hash: &str,
        active_policy_id: &str,
        graph_query: Option<&TraversalQuery<'_>>,
    ) -> Result<Option<BeliefView>, StorageError> {
        let Some(mut view) = self.mark_stale_if_newer_evidence(key)? else {
            return Ok(None);
        };
        let revision = match view.current_revision_id.as_deref() {
            Some(revision_id) => self.get_revision(revision_id)?,
            None => None,
        };
        if let Some(revision) = revision {
            if revision.config_snapshot_hash != active_config_hash {
                mark_view_stale(
                    &mut view,
                    FreshnessReason::ConfigSnapshotChanged,
                    "active config snapshot changed",
                );
            }
            if key.evidence_policy_id != active_policy_id {
                mark_view_stale(
                    &mut view,
                    FreshnessReason::EvidencePolicyChanged,
                    "active evidence policy changed",
                );
            }
            if let Some(query) = graph_query {
                for anchor_id in &revision.provenance.graph_anchor_ids {
                    if query.supersession_for_anchor(anchor_id)?.is_some() {
                        mark_view_stale(
                            &mut view,
                            FreshnessReason::SupersededAnchor,
                            "graph anchor was superseded",
                        );
                        break;
                    }
                }
            }
        }
        if view.freshness.stale {
            self.put_view(&view)?;
        }
        Ok(Some(view))
    }

    /// Mark a key as needing assessment after the given source sequence.
    pub fn mark_dirty(&self, key: &BeliefKey, seq: u64) -> Result<(), StorageError> {
        self.mark_dirty_with_reason(key, seq, seq, None, DirtyReason::NewEvidence)
    }

    /// Clear dirty state after a successful commit.
    pub fn clear_dirty(&self, key: &BeliefKey) -> Result<(), StorageError> {
        let _write = self.writable_guard()?;
        self.dirty_keys
            .remove(key.index_key().as_bytes())
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Return dirty key index entries for recovery and tests.
    pub fn dirty_keys(&self) -> Result<Vec<String>, StorageError> {
        let mut out = Vec::new();
        for item in self.dirty_keys.iter() {
            let (key, _) = item.map_err(to_storage_io)?;
            out.push(String::from_utf8(key.to_vec()).map_err(to_storage_utf8)?);
        }
        Ok(out)
    }

    /// Return durable dirty key records in deterministic key order.
    pub fn dirty_key_states(&self) -> Result<Vec<DirtyKeyState>, StorageError> {
        let mut out = Vec::new();
        for item in self.dirty_keys.iter() {
            let (_, value) = item.map_err(to_storage_io)?;
            out.push(decode_dirty_state(&value)?);
        }
        out.sort_by(|left, right| {
            left.belief_key
                .index_key()
                .cmp(&right.belief_key.index_key())
        });
        Ok(out)
    }

    /// Read durable dirty state for one belief key.
    pub fn dirty_state(&self, key: &BeliefKey) -> Result<Option<DirtyKeyState>, StorageError> {
        let Some(raw) = self
            .dirty_keys
            .get(key.index_key().as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        Ok(Some(decode_dirty_state(&raw)?))
    }

    fn mark_dirty_with_reason(
        &self,
        key: &BeliefKey,
        dirty_since_seq: u64,
        latest_seq: u64,
        active_lease_id: Option<String>,
        reason: DirtyReason,
    ) -> Result<(), StorageError> {
        let _write = self.writable_guard()?;
        use sled::transaction::{ConflictableTransactionError, TransactionError};
        self.dirty_keys
            .transaction(|dirty_keys| {
                let existing = dirty_keys
                    .get(key.index_key().as_bytes())?
                    .map(|raw| {
                        serde_json::from_slice::<DirtyKeyState>(&raw).map_err(|error| {
                            ConflictableTransactionError::Abort(format!(
                                "invalid dirty state during update: {error}"
                            ))
                        })
                    })
                    .transpose()?;
                let state = merge_dirty_state(
                    existing,
                    key,
                    dirty_since_seq,
                    latest_seq,
                    active_lease_id.clone(),
                    reason.clone(),
                    None,
                );
                let encoded = serde_json::to_vec(&state).map_err(|error| {
                    ConflictableTransactionError::Abort(format!(
                        "cannot encode dirty state update: {error}"
                    ))
                })?;
                dirty_keys.insert(key.index_key().as_bytes(), encoded)?;
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::InvalidPath(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })
    }

    /// Read the source watermark acknowledged by canonical ingestion receipts.
    pub(crate) fn assessment_source_high_water(&self) -> Result<u64, StorageError> {
        self.runtime_meta
            .get(KEY_ASSESSMENT_SOURCE_HIGH_WATER)
            .map_err(to_storage_io)?
            .as_deref()
            .map(decode_runtime_sequence)
            .transpose()
            .map(|value| value.unwrap_or(0))
    }

    /// Advance the durable logical clock used only for recurring lease expiry.
    pub(crate) fn advance_assessment_lease_clock(&self) -> Result<u64, StorageError> {
        let _write = self.writable_guard()?;
        loop {
            let current = self
                .runtime_meta
                .get(KEY_ASSESSMENT_LEASE_CLOCK)
                .map_err(to_storage_io)?;
            let current_sequence = current
                .as_deref()
                .map(decode_assessment_lease_clock)
                .transpose()?
                .unwrap_or(0);
            let next_sequence = current_sequence.checked_add(1).ok_or_else(|| {
                StorageError::InvalidPath("belief assessment lease clock is exhausted".to_string())
            })?;
            let next = next_sequence.to_be_bytes();
            match self
                .runtime_meta
                .compare_and_swap(
                    KEY_ASSESSMENT_LEASE_CLOCK,
                    current.as_deref(),
                    Some(next.as_slice()),
                )
                .map_err(to_storage_io)?
            {
                Ok(()) => {
                    self.flush()?;
                    return Ok(next_sequence);
                }
                Err(_) => continue,
            }
        }
    }

    /// Read the durable logical clock used for recurring lease expiry.
    pub(crate) fn assessment_lease_clock(&self) -> Result<u64, StorageError> {
        self.runtime_meta
            .get(KEY_ASSESSMENT_LEASE_CLOCK)
            .map_err(to_storage_io)?
            .as_deref()
            .map(decode_assessment_lease_clock)
            .transpose()
            .map(|value| value.unwrap_or(0))
    }

    /// Read the monotonic sequence for durable assessment state transitions.
    pub(crate) fn assessment_progress_sequence(&self) -> Result<u64, StorageError> {
        self.runtime_meta
            .get(KEY_ASSESSMENT_PROGRESS_SEQUENCE)
            .map_err(to_storage_io)?
            .as_deref()
            .map(decode_assessment_progress_sequence)
            .transpose()
            .map(|value| value.unwrap_or(0))
    }

    fn exact_assessment_progress_receipt(
        &self,
        receipt_key: &[u8],
    ) -> Result<Option<u64>, StorageError> {
        self.runtime_meta
            .get(receipt_key)
            .map_err(to_storage_io)?
            .as_deref()
            .map(decode_assessment_progress_sequence)
            .transpose()
    }

    /// Select one bounded starvation-safe dirty-key page after the durable cursor.
    pub(crate) fn select_dirty_work_page(
        &self,
        max_items: usize,
    ) -> Result<BeliefDirtyWorkPage, StorageError> {
        if max_items == 0 {
            return Err(StorageError::InvalidPath(
                "belief dirty work page limit must be positive".to_string(),
            ));
        }
        let expected_cursor = self
            .runtime_meta
            .get(KEY_ASSESSMENT_DIRTY_CURSOR)
            .map_err(to_storage_io)?
            .map(|value| value.to_vec());
        let page_limit = max_items.saturating_add(1);
        let mut selected = Vec::with_capacity(page_limit);
        if let Some(cursor) = expected_cursor.as_deref() {
            for item in self
                .dirty_keys
                .range::<&[u8], _>((
                    std::ops::Bound::Excluded(cursor),
                    std::ops::Bound::Unbounded,
                ))
                .take(page_limit)
            {
                let (_, value) = item.map_err(to_storage_io)?;
                selected.push(decode_dirty_state(&value)?);
            }
            if selected.len() < page_limit {
                for item in self.dirty_keys.iter() {
                    let (key, value) = item.map_err(to_storage_io)?;
                    if key.as_ref() > cursor {
                        break;
                    }
                    selected.push(decode_dirty_state(&value)?);
                    if selected.len() == page_limit {
                        break;
                    }
                }
            }
        } else {
            for item in self.dirty_keys.iter().take(page_limit) {
                let (_, value) = item.map_err(to_storage_io)?;
                selected.push(decode_dirty_state(&value)?);
            }
        }
        let has_more = selected.len() > max_items;
        selected.truncate(max_items);
        Ok(BeliefDirtyWorkPage {
            expected_cursor,
            items: selected,
            has_more,
        })
    }

    /// Acknowledge one attempted dirty key with an exact cursor compare and swap.
    pub(crate) fn acknowledge_dirty_work_cursor(
        &self,
        expected_cursor: Option<&[u8]>,
        attempted_key: &BeliefKey,
    ) -> Result<Vec<u8>, StorageError> {
        let _write = self.writable_guard()?;
        let next = attempted_key.index_key().into_bytes();
        match self
            .runtime_meta
            .compare_and_swap(
                KEY_ASSESSMENT_DIRTY_CURSOR,
                expected_cursor,
                Some(next.as_slice()),
            )
            .map_err(to_storage_io)?
        {
            Ok(()) => self.flush()?,
            Err(error) if error.current.as_deref() == Some(next.as_slice()) => self.flush()?,
            Err(_) => {
                return Err(StorageError::Backpressure(
                    "belief assessment dirty cursor changed".to_string(),
                ));
            }
        }
        Ok(next)
    }

    /// Store small runtime metadata values.
    // TODO compat-shim: remove this generic writer after external migration
    // fixtures use explicit belief compatibility operations. Until then it
    // preserves non-authoritative legacy metadata while reserved actor state
    // remains writable only through narrow domain methods and their tests.
    pub fn put_runtime_meta(&self, key: &str, value: &str) -> Result<(), StorageError> {
        if reserved_runtime_meta_key(key) {
            return Err(StorageError::InvalidPath(
                "belief runtime metadata key is reserved for domain authority".to_string(),
            ));
        }
        self.put_runtime_meta_internal(key, value)
    }

    pub(crate) fn put_runtime_meta_internal(
        &self,
        key: &str,
        value: &str,
    ) -> Result<(), StorageError> {
        let _write = self.writable_guard()?;
        self.runtime_meta
            .insert(key.as_bytes(), value.as_bytes())
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Read a small runtime metadata value.
    pub fn get_runtime_meta(&self, key: &str) -> Result<Option<String>, StorageError> {
        let Some(raw) = self
            .runtime_meta
            .get(key.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        Ok(Some(
            String::from_utf8(raw.to_vec()).map_err(to_storage_utf8)?,
        ))
    }

    /// Project one committed revision into a planner-safe view.
    pub fn project_view(&self, revision: &BeliefRevision, hydration: HydrationRefs) -> BeliefView {
        BeliefView {
            view_id: format!("view-{}", revision.revision_id),
            key: revision.belief_key.clone(),
            current_revision_id: Some(revision.revision_id.clone()),
            status: revision.status.clone(),
            posterior: revision.posterior.clone(),
            planner_projection: revision.planner_projection.clone(),
            uncertainty: revision.uncertainty,
            precision: revision.precision,
            freshness: revision.freshness.clone(),
            contradiction: revision.contradiction.clone(),
            observation: revision.observation.clone(),
            assessment_state: "complete".to_string(),
            advisory_posture: if revision.planner_projection.confidence
                < revision.planner_projection.threshold
            {
                "observe_or_repair".to_string()
            } else {
                "ready".to_string()
            },
            provenance: revision.provenance.clone(),
            hydration,
        }
    }

    /// Rebuild the current view from durable revision state without using the view cache.
    pub fn rebuild_current_view_from_revision(
        &self,
        key: &BeliefKey,
    ) -> Result<Option<BeliefView>, StorageError> {
        let Some(revision) = self.current_revision(key)? else {
            return Ok(None);
        };
        let hydration = HydrationRefs {
            evidence_ids: revision.evidence_ids.clone(),
            source_fact_ids: revision.provenance.source_fact_ids.clone(),
            graph_anchor_ids: revision.provenance.graph_anchor_ids.clone(),
            revision_id: Some(revision.revision_id.clone()),
        };
        Ok(Some(self.project_view(&revision, hydration)))
    }

    fn migrated_trees(&self) -> Vec<(&'static str, &Tree)> {
        vec![
            (TREE_EVIDENCE, &self.evidence),
            (TREE_ASSIGNMENTS, &self.assignments),
            (TREE_ASSIGNMENT_GENERATIONS, &self.assignment_generations),
            (TREE_REVISIONS, &self.revisions),
            (TREE_REVISION_HEAD, &self.revision_head),
            (TREE_VIEWS, &self.views),
            (TREE_VIEW_BY_SUBJECT, &self.view_by_subject),
            (TREE_LEASES, &self.leases),
            (TREE_ACTIVE_LEASE, &self.active_lease),
            (TREE_REJECTIONS, &self.rejections),
            (TREE_CONFIG_SNAPSHOTS, &self.config_snapshots),
            (TREE_DIRTY_KEYS, &self.dirty_keys),
            (TREE_RUNTIME_META, &self.runtime_meta),
            (
                TREE_EVIDENCE_CONSUMER_CURSORS,
                &self.evidence_consumer_cursors,
            ),
            (
                TREE_EVIDENCE_INGESTION_RECEIPTS,
                &self.evidence_ingestion_receipts,
            ),
            (TREE_COMMIT_INTENTS, &self.commit_intents),
            (TREE_COMMIT_RECEIPTS, &self.commit_receipts),
            (
                TREE_LEGACY_ASSESSMENT_RECEIPTS,
                &self.legacy_assessment_receipts,
            ),
        ]
    }

    fn has_migrated_product_records(&self) -> Result<bool, StorageError> {
        for (tree_name, tree) in self.migrated_trees() {
            for item in tree.iter() {
                let (key, _) = item.map_err(to_storage_io)?;
                if tree_name != TREE_LEGACY_ASSESSMENT_RECEIPTS
                    || key.as_ref() != KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn copy_legacy_records(
        &self,
        legacy: &BeliefStore,
        marker: &BeliefAuthorityMigrationMarker,
        source: &BeliefAuthoritySnapshot,
    ) -> Result<(), StorageError> {
        let source_trees = legacy.migrated_trees();
        let target_trees = self.migrated_trees();
        let starting_count = match marker.progress() {
            BeliefAuthorityMigrationProgress::Copying {
                verified_record_count,
                ..
            } => *verified_record_count,
            _ => 0,
        };
        let may_replace_portable_fence = self.may_replace_empty_local_assessment_fence()?;
        let checkpoint = LegacyCopyCheckpoint {
            marker,
            source,
            starting_count,
        };
        let mut verified_record_count = 0u64;
        for ((source_name, source_tree), (target_name, target_tree)) in
            source_trees.into_iter().zip(target_trees)
        {
            if source_name != target_name {
                return Err(StorageError::InvalidPath(
                    "belief migration tree ordering changed".to_string(),
                ));
            }
            if source_name == TREE_LEGACY_ASSESSMENT_RECEIPTS {
                let mut receipt_payload = Vec::new();
                let mut portable_fence = None;
                for item in source_tree.iter() {
                    let (key, value) = item.map_err(to_storage_io)?;
                    verified_record_count = verified_record_count.saturating_add(1);
                    if key.as_ref() == KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE {
                        portable_fence = Some(value.to_vec());
                    } else {
                        receipt_payload.push((key.to_vec(), value.to_vec()));
                    }
                }
                let portable_fence = portable_fence.ok_or_else(|| {
                    StorageError::InvalidPath(
                        "belief migration source omitted its portable assessment fence".to_string(),
                    )
                })?;
                if verified_record_count <= starting_count {
                    self.require_matching_assessment_payload(&receipt_payload, &portable_fence)?;
                } else {
                    let checkpoint = BeliefAuthorityMigrationMarker::try_new(
                        marker.migration().clone(),
                        BeliefAuthorityMigrationProgress::Copying {
                            source: source.clone(),
                            verified_record_count,
                        },
                    )?;
                    self.install_assessment_payload_and_checkpoint(
                        &receipt_payload,
                        &portable_fence,
                        &checkpoint,
                        may_replace_portable_fence,
                    )?;
                }
                continue;
            }
            for item in source_tree.iter() {
                let (key, value) = item.map_err(to_storage_io)?;
                self.copy_legacy_record_and_checkpoint(
                    target_name,
                    target_tree,
                    &key,
                    &value,
                    &checkpoint,
                    &mut verified_record_count,
                )?;
            }
        }
        if verified_record_count != source.record_count() {
            return Err(StorageError::InvalidPath(
                "belief migration verified count diverged from its frozen source".to_string(),
            ));
        }
        self.reconcile_assessment_indexes()
    }

    fn copy_legacy_record_and_checkpoint(
        &self,
        target_name: &str,
        target_tree: &Tree,
        key: &[u8],
        value: &[u8],
        checkpoint: &LegacyCopyCheckpoint<'_>,
        verified_record_count: &mut u64,
    ) -> Result<(), StorageError> {
        match target_tree
            .compare_and_swap(key, None as Option<&[u8]>, Some(value))
            .map_err(to_storage_io)?
        {
            Ok(()) => {}
            Err(error) if error.current.as_deref() == Some(value) => {}
            Err(_) => {
                return Err(StorageError::InvalidPath(format!(
                    "belief migration target conflict in tree '{target_name}'"
                )));
            }
        }
        *verified_record_count = verified_record_count.saturating_add(1);
        if *verified_record_count <= checkpoint.starting_count {
            return Ok(());
        }
        let marker = BeliefAuthorityMigrationMarker::try_new(
            checkpoint.marker.migration().clone(),
            BeliefAuthorityMigrationProgress::Copying {
                source: checkpoint.source.clone(),
                verified_record_count: *verified_record_count,
            },
        )?;
        self.put_authority_migration_marker(&marker)
    }

    fn may_replace_empty_local_assessment_fence(&self) -> Result<bool, StorageError> {
        let Some(raw) = self
            .authority_meta
            .get(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE)
            .map_err(to_storage_io)?
        else {
            return Ok(false);
        };
        let fence: LegacyAssessmentMigrationFence =
            serde_json::from_slice(&raw).map_err(to_storage_data)?;
        validate_legacy_assessment_migration_fence(&fence)?;
        Ok(fence.source_snapshot.record_count() == 0 && fence.migrated_revision_ids.is_empty())
    }

    fn require_matching_assessment_fence_copies(
        &self,
        expected: &[u8],
    ) -> Result<(), StorageError> {
        let portable = self
            .legacy_assessment_receipts
            .get(KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE)
            .map_err(to_storage_io)?;
        let local = self
            .authority_meta
            .get(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE)
            .map_err(to_storage_io)?;
        if portable.as_deref() == Some(expected) && local.as_deref() == Some(expected) {
            return Ok(());
        }
        Err(StorageError::InvalidPath(
            "belief migration target assessment fence conflicts with source".to_string(),
        ))
    }

    fn require_matching_assessment_payload(
        &self,
        receipt_payload: &[(Vec<u8>, Vec<u8>)],
        portable_fence: &[u8],
    ) -> Result<(), StorageError> {
        for (key, value) in receipt_payload {
            if self
                .legacy_assessment_receipts
                .get(key)
                .map_err(to_storage_io)?
                .as_deref()
                != Some(value.as_slice())
            {
                return Err(StorageError::InvalidPath(
                    "belief migration target assessment receipt conflicts with source".to_string(),
                ));
            }
        }
        self.require_matching_assessment_fence_copies(portable_fence)
    }

    fn install_assessment_payload_and_checkpoint(
        &self,
        receipt_payload: &[(Vec<u8>, Vec<u8>)],
        portable_fence: &[u8],
        checkpoint: &BeliefAuthorityMigrationMarker,
        may_replace: bool,
    ) -> Result<(), StorageError> {
        let fence: LegacyAssessmentMigrationFence =
            serde_json::from_slice(portable_fence).map_err(to_storage_data)?;
        validate_legacy_assessment_migration_fence(&fence)?;
        let current_marker_raw = self
            .authority_migration
            .get(KEY_AUTHORITY_MIGRATION_MARKER)
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "belief migration checkpoint disappeared during assessment fence handoff"
                        .to_string(),
                )
            })?;
        let current_marker: BeliefAuthorityMigrationMarker =
            serde_json::from_slice(&current_marker_raw).map_err(to_storage_data)?;
        validate_migration_marker_successor(&current_marker, checkpoint)?;
        let checkpoint_bytes = serde_json::to_vec(checkpoint).map_err(to_storage_data)?;

        use sled::transaction::{ConflictableTransactionError, TransactionError};
        (
            &self.legacy_assessment_receipts,
            &self.authority_meta,
            &self.authority_migration,
        )
            .transaction(|(receipts, authority_meta, migration)| {
                for (key, value) in receipt_payload {
                    if let Some(existing) = receipts.get(key.as_slice())? {
                        if existing.as_ref() != value.as_slice() {
                            return Err(ConflictableTransactionError::Abort(
                                "belief migration target assessment receipt conflicts with source"
                                    .to_string(),
                            ));
                        }
                    } else {
                        receipts.insert(key.as_slice(), value.as_slice())?;
                    }
                }
                let current_portable =
                    receipts.get(KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE)?;
                let current_local = authority_meta.get(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE)?;
                let already_installed = current_portable.as_deref() == Some(portable_fence)
                    && current_local.as_deref() == Some(portable_fence);
                if !already_installed {
                    if !may_replace || current_portable.as_deref() != current_local.as_deref() {
                        return Err(ConflictableTransactionError::Abort(
                            "belief migration target assessment fence conflicts with source"
                                .to_string(),
                        ));
                    }
                    receipts.insert(
                        KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE,
                        portable_fence,
                    )?;
                    authority_meta.insert(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE, portable_fence)?;
                }
                if migration.get(KEY_AUTHORITY_MIGRATION_MARKER)?.as_deref()
                    != Some(current_marker_raw.as_ref())
                {
                    return Err(ConflictableTransactionError::Abort(
                        "belief migration checkpoint changed during assessment fence handoff"
                            .to_string(),
                    ));
                }
                migration.insert(KEY_AUTHORITY_MIGRATION_MARKER, checkpoint_bytes.as_slice())?;
                Ok(())
            })
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::InvalidPath(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.flush().map_err(|error| {
            StorageError::DurabilityIndeterminate(format!(
                "belief assessment payload handoff flush failed: {error}"
            ))
        })
    }

    /// Flush the shared sled database.
    pub fn flush(&self) -> Result<(), StorageError> {
        #[cfg(any(test, feature = "test-support"))]
        {
            let mut probe = self.flush_probe.lock();
            probe.calls = probe.calls.saturating_add(1);
            if probe.fail_next || probe.fail_at_call == Some(probe.calls) {
                probe.fail_next = false;
                probe.fail_at_call = None;
                return Err(StorageError::IoError(io::Error::other(
                    "injected indeterminate flush",
                )));
            }
        }
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    /// Inject one future flush failure for deterministic durability testing.
    #[doc(hidden)]
    #[cfg(any(test, feature = "test-support"))]
    pub fn fail_flush_after_for_test(&self, additional_calls: usize) {
        let mut probe = self.flush_probe.lock();
        probe.fail_at_call = Some(probe.calls.saturating_add(additional_calls));
    }

    #[cfg(test)]
    fn arm_migration_copy_pause_for_test(&self) {
        let mut state = self.migration_copy_probe.state.lock();
        state.pause_after_gate = true;
        state.entered = false;
        state.released = false;
    }

    #[cfg(test)]
    fn pause_migration_copy_after_gate_for_test(&self) {
        let mut state = self.migration_copy_probe.state.lock();
        if !state.pause_after_gate {
            return;
        }
        state.entered = true;
        self.migration_copy_probe.changed.notify_all();
        while !state.released {
            self.migration_copy_probe.changed.wait(&mut state);
        }
        state.pause_after_gate = false;
    }

    #[cfg(test)]
    fn wait_for_migration_copy_gate_for_test(&self) {
        let mut state = self.migration_copy_probe.state.lock();
        while !state.entered {
            self.migration_copy_probe.changed.wait(&mut state);
        }
    }

    #[cfg(test)]
    fn release_migration_copy_for_test(&self) {
        let mut state = self.migration_copy_probe.state.lock();
        state.released = true;
        self.migration_copy_probe.changed.notify_all();
    }

    #[cfg(test)]
    fn arm_commit_after_prepare_pause_for_test(&self) {
        let mut state = self.commit_intent_probe.state.lock();
        state.pause_after_prepare = true;
        state.entered = false;
        state.released = false;
    }

    #[cfg(test)]
    fn pause_commit_after_prepare_for_test(&self) {
        let mut state = self.commit_intent_probe.state.lock();
        if !state.pause_after_prepare {
            return;
        }
        state.entered = true;
        self.commit_intent_probe.changed.notify_all();
        while !state.released {
            self.commit_intent_probe.changed.wait(&mut state);
        }
        state.pause_after_prepare = false;
    }

    #[cfg(test)]
    fn wait_for_commit_prepare_for_test(&self) {
        let mut state = self.commit_intent_probe.state.lock();
        while !state.entered {
            self.commit_intent_probe.changed.wait(&mut state);
        }
    }

    #[cfg(test)]
    fn release_commit_after_prepare_for_test(&self) {
        let mut state = self.commit_intent_probe.state.lock();
        state.released = true;
        self.commit_intent_probe.changed.notify_all();
    }

    #[cfg(test)]
    fn arm_legacy_fence_attempt_for_test(&self) {
        let mut state = self.legacy_fence_probe.state.lock();
        state.armed = true;
        state.attempted = false;
    }

    #[cfg(test)]
    fn signal_legacy_fence_attempt_for_test(&self) {
        let mut state = self.legacy_fence_probe.state.lock();
        if !state.armed {
            return;
        }
        state.attempted = true;
        self.legacy_fence_probe.changed.notify_all();
    }

    #[cfg(test)]
    fn wait_for_legacy_fence_attempt_for_test(&self) {
        let mut state = self.legacy_fence_probe.state.lock();
        while !state.attempted {
            self.legacy_fence_probe.changed.wait(&mut state);
        }
        state.armed = false;
    }

    /// Read internal assessment source authority for test-support verification.
    #[doc(hidden)]
    #[cfg(feature = "test-support")]
    pub fn assessment_source_high_water_for_test(&self) -> Result<u64, StorageError> {
        self.assessment_source_high_water()
    }
}

pub(super) fn load_or_create_store_instance_id(
    db: &Db,
    meta: &Tree,
) -> Result<String, StorageError> {
    loop {
        let current = meta.get(KEY_STORE_INSTANCE_ID).map_err(to_storage_io)?;
        if let Some(raw) = current.as_deref() {
            let identity = String::from_utf8(raw.to_vec()).map_err(to_storage_utf8)?;
            if identity.trim().is_empty() {
                return Err(StorageError::InvalidPath(
                    "belief store identity must be non-empty".to_string(),
                ));
            }
            return Ok(write_gate_identity(db, &identity));
        }
        let candidate = format!("belief-store-{}", Uuid::new_v4());
        match meta
            .compare_and_swap(
                KEY_STORE_INSTANCE_ID,
                current.as_deref(),
                Some(candidate.as_bytes()),
            )
            .map_err(to_storage_io)?
        {
            Ok(()) => {
                db.flush().map_err(to_storage_io)?;
                return Ok(write_gate_identity(db, &candidate));
            }
            Err(_) => continue,
        }
    }
}

fn write_gate_identity(db: &Db, store_instance_id: &str) -> String {
    format!("{store_instance_id}::live-db-{:p}", &*db.context.pagecache)
}

pub(super) fn shared_write_gate(identity: &str) -> Arc<RwLock<()>> {
    let registry = WRITE_GATES.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut registry = registry.lock();
    registry.retain(|_, gate| gate.strong_count() > 0);
    if let Some(gate) = registry.get(identity).and_then(Weak::upgrade) {
        return gate;
    }
    let gate = Arc::new(RwLock::new(()));
    registry.insert(identity.to_string(), Arc::downgrade(&gate));
    gate
}

fn preterminal_authority_migration(
    authority_meta: &Tree,
    authority_migration: &Tree,
) -> Result<Option<BeliefAuthorityMigrationMarker>, StorageError> {
    let Some(marker) = decode_optional::<BeliefAuthorityMigrationMarker>(
        authority_migration
            .get(KEY_AUTHORITY_MIGRATION_MARKER)
            .map_err(to_storage_io)?,
    )?
    else {
        return Ok(None);
    };
    marker.validate()?;
    if !matches!(
        marker.progress(),
        BeliefAuthorityMigrationProgress::Prepared { .. }
            | BeliefAuthorityMigrationProgress::Copying { .. }
            | BeliefAuthorityMigrationProgress::Verified { .. }
    ) {
        return Ok(None);
    }
    let bound_authority = authority_meta
        .get(KEY_PRODUCT_AUTHORITY_ID)
        .map_err(to_storage_io)?
        .ok_or_else(|| {
            StorageError::MigrationConflict(
                "incomplete belief migration lost its product authority binding".to_string(),
            )
        })?;
    if bound_authority.as_ref() != marker.migration().target_authority_id().as_bytes() {
        return Err(StorageError::MigrationConflict(
            "incomplete belief migration product authority binding changed".to_string(),
        ));
    }
    Ok(Some(marker))
}

fn require_exact_preterminal_authority_migration(
    authority_meta: &Tree,
    authority_migration: &Tree,
    identity: &BeliefAuthorityMigrationIdentity,
) -> Result<BeliefAuthorityMigrationMarker, StorageError> {
    let marker =
        preterminal_authority_migration(authority_meta, authority_migration)?.ok_or_else(|| {
            StorageError::MigrationConflict(
                "belief migration recovery requires a preterminal checkpoint".to_string(),
            )
        })?;
    if marker.migration() != identity {
        return Err(StorageError::MigrationConflict(
            "belief migration recovery identity conflicts with durable authority".to_string(),
        ));
    }
    Ok(marker)
}

pub(super) fn require_product_authority_available(
    authority_meta: &Tree,
    authority_migration: &Tree,
) -> Result<(), StorageError> {
    if let Some(marker) = preterminal_authority_migration(authority_meta, authority_migration)? {
        return Err(incomplete_authority_unavailable(&marker));
    }
    Ok(())
}

fn incomplete_authority_unavailable(marker: &BeliefAuthorityMigrationMarker) -> StorageError {
    StorageError::Unavailable(format!(
        "belief product authority migration '{}' requires coordinator recovery",
        marker.migration().migration_id()
    ))
}

fn decode_optional<T: serde::de::DeserializeOwned>(
    raw: Option<sled::IVec>,
) -> Result<Option<T>, StorageError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&raw).map_err(to_storage_data)?))
}

fn decode_required<T: serde::de::DeserializeOwned>(
    raw: Option<sled::IVec>,
    missing: &str,
) -> Result<T, StorageError> {
    let raw = raw.ok_or_else(|| StorageError::InvalidPath(missing.to_string()))?;
    serde_json::from_slice(&raw).map_err(to_storage_data)
}

fn decode_runtime_sequence(raw: &[u8]) -> Result<u64, StorageError> {
    let bytes: [u8; 8] = raw.try_into().map_err(|_| {
        StorageError::InvalidPath(
            "belief assessment source watermark has invalid width".to_string(),
        )
    })?;
    Ok(u64::from_be_bytes(bytes))
}

fn decode_assessment_lease_clock(raw: &[u8]) -> Result<u64, StorageError> {
    let bytes: [u8; 8] = raw.try_into().map_err(|_| {
        StorageError::InvalidPath("belief assessment lease clock has invalid width".to_string())
    })?;
    Ok(u64::from_be_bytes(bytes))
}

fn decode_assessment_progress_sequence(raw: &[u8]) -> Result<u64, StorageError> {
    let bytes: [u8; 8] = raw.try_into().map_err(|_| {
        StorageError::InvalidPath(
            "belief assessment progress sequence has invalid width".to_string(),
        )
    })?;
    Ok(u64::from_be_bytes(bytes))
}

fn assessment_progress_receipt_key(kind: &str, operation_id: &str) -> Vec<u8> {
    format!("assessment_progress_receipt::{kind}::{operation_id}").into_bytes()
}

fn revision_id_to_intent_id(revision_id: &str) -> String {
    format!("belief-commit-{revision_id}")
}

fn legacy_lease_matches_revision(lease: &AssessmentLease, revision: &BeliefRevision) -> bool {
    lease.status == LeaseStatus::Completed
        && lease.belief_key == revision.belief_key
        && lease.input_cursor_start == revision.source_cursor_start
        && lease.input_cursor_end == revision.source_cursor_end
        && lease.comparator_engine_id == revision.comparator_engine_id
        && lease.config_snapshot_hash == revision.config_snapshot_hash
}

fn validate_legacy_assessment_receipt(
    receipt: &LegacyAssessmentReceipt,
    revision: &BeliefRevision,
) -> Result<(), StorageError> {
    if receipt.schema_version != LEGACY_ASSESSMENT_RECEIPT_SCHEMA_VERSION
        || receipt.revision != *revision
        || receipt.legacy_progress_sequence != 0
        || !legacy_lease_matches_revision(&receipt.completed_lease, revision)
    {
        return Err(StorageError::InvalidPath(
            "legacy belief assessment receipt does not match its revision".to_string(),
        ));
    }
    Ok(())
}

fn validate_legacy_assessment_migration_fence(
    fence: &LegacyAssessmentMigrationFence,
) -> Result<(), StorageError> {
    if fence.schema_version != LEGACY_ASSESSMENT_MIGRATION_SCHEMA_VERSION {
        return Err(StorageError::InvalidPath(
            "unsupported legacy belief assessment migration fence version".to_string(),
        ));
    }
    fence.source_snapshot.validate()?;
    let mut prior = None;
    for revision_id in &fence.migrated_revision_ids {
        if revision_id.trim().is_empty() || prior.is_some_and(|prior| prior >= revision_id) {
            return Err(StorageError::InvalidPath(
                "legacy belief assessment migration identities are not canonical".to_string(),
            ));
        }
        prior = Some(revision_id);
    }
    Ok(())
}

fn reserved_runtime_meta_key(key: &str) -> bool {
    key.starts_with("assessment_")
        || key.starts_with("active_config_hash::")
        || key.starts_with("active_policy_id::")
}

fn verify_commit_retry(
    recorded: &BeliefCommitIntent,
    lease: &AssessmentLease,
    revision: &BeliefRevision,
    public_view: &BeliefView,
) -> Result<(), StorageError> {
    if recorded.expected_active_lease() != lease
        || recorded.revision() != revision
        || recorded.public_view() != public_view
    {
        return Err(StorageError::InvalidPath(format!(
            "belief commit operation conflict for '{}'",
            recorded.intent_id()
        )));
    }
    Ok(())
}

fn record_assessment_progress(
    runtime_meta: &sled::transaction::TransactionalTree,
    receipt_key: &[u8],
) -> Result<u64, sled::transaction::ConflictableTransactionError<String>> {
    use sled::transaction::ConflictableTransactionError;

    if let Some(raw) = runtime_meta.get(receipt_key)? {
        let bytes: [u8; 8] = raw.as_ref().try_into().map_err(|_| {
            ConflictableTransactionError::Abort(
                "belief assessment progress receipt has invalid width".to_string(),
            )
        })?;
        return Ok(u64::from_be_bytes(bytes));
    }
    let current = runtime_meta
        .get(KEY_ASSESSMENT_PROGRESS_SEQUENCE)?
        .map(|raw| {
            let bytes: [u8; 8] = raw.as_ref().try_into().map_err(|_| {
                ConflictableTransactionError::Abort(
                    "belief assessment progress sequence has invalid width".to_string(),
                )
            })?;
            Ok::<u64, ConflictableTransactionError<String>>(u64::from_be_bytes(bytes))
        })
        .transpose()?
        .unwrap_or(0);
    let next = current.checked_add(1).ok_or_else(|| {
        ConflictableTransactionError::Abort(
            "belief assessment progress sequence overflow".to_string(),
        )
    })?;
    let encoded = next.to_be_bytes();
    runtime_meta.insert(KEY_ASSESSMENT_PROGRESS_SEQUENCE, encoded.as_slice())?;
    runtime_meta.insert(receipt_key, encoded.as_slice())?;
    Ok(next)
}

fn record_acknowledged_source_high_water(
    runtime_meta: &sled::transaction::TransactionalTree,
    sequence: u64,
) -> Result<(), sled::transaction::ConflictableTransactionError<String>> {
    use sled::transaction::ConflictableTransactionError;

    let current = runtime_meta
        .get(KEY_ASSESSMENT_SOURCE_HIGH_WATER)?
        .map(|raw| {
            let bytes: [u8; 8] = raw.as_ref().try_into().map_err(|_| {
                ConflictableTransactionError::Abort(
                    "belief acknowledged source watermark has invalid width".to_string(),
                )
            })?;
            Ok::<u64, ConflictableTransactionError<String>>(u64::from_be_bytes(bytes))
        })
        .transpose()?
        .unwrap_or(0);
    if current < sequence {
        runtime_meta.insert(
            KEY_ASSESSMENT_SOURCE_HIGH_WATER,
            sequence.to_be_bytes().as_slice(),
        )?;
    }
    Ok(())
}

fn belief_key_index_prefix(key: &BeliefKey) -> Vec<u8> {
    let encoded = key.index_key();
    let mut prefix = Vec::with_capacity(8 + encoded.len());
    prefix.extend_from_slice(&(encoded.len() as u64).to_be_bytes());
    prefix.extend_from_slice(encoded.as_bytes());
    prefix
}

fn assignment_by_key_index_key(assignment: &EvidenceAssignment, generation: u64) -> Vec<u8> {
    let mut key = belief_key_index_prefix(&assignment.belief_key);
    key.extend_from_slice(&generation.to_be_bytes());
    key.extend_from_slice(&assignment.source_cursor_end.to_be_bytes());
    key.extend_from_slice(assignment.assignment_id.as_bytes());
    key
}

fn assignment_cursor_index_key(prefix: &[u8], cursor: &AssessmentAssignmentCursor) -> Vec<u8> {
    let mut key = prefix.to_vec();
    key.extend_from_slice(&cursor.mutation_generation.to_be_bytes());
    key.extend_from_slice(&cursor.source_cursor_end.to_be_bytes());
    key.extend_from_slice(cursor.assignment_id.as_bytes());
    key
}

fn decode_assignment_generation(raw: Option<&[u8]>) -> Result<u64, StorageError> {
    decode_transaction_generation(raw).map_err(StorageError::InvalidPath)
}

fn decode_transaction_generation(raw: Option<&[u8]>) -> Result<u64, String> {
    match raw {
        Some(raw) if raw.len() == 8 => {
            Ok(u64::from_be_bytes(raw.try_into().map_err(|_| {
                "evidence assignment generation has invalid width".to_string()
            })?))
        }
        Some(_) => Err("evidence assignment generation has invalid width".to_string()),
        // TODO compat-shim: remove generation zero after every supported store
        // has passed assignment-index migration. This preserves assignments
        // written before generation records existed. Keep legacy fixture and
        // migration parity tests green before removal.
        None => Ok(0),
    }
}

fn committed_evidence_index_key(key: &BeliefKey, evidence_id: &str) -> Vec<u8> {
    let mut index_key = belief_key_index_prefix(key);
    index_key.extend_from_slice(evidence_id.as_bytes());
    index_key
}

fn hash_snapshot_part(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn require_frozen_source(
    legacy: &BeliefStore,
    expected: &BeliefAuthoritySnapshot,
) -> Result<(), StorageError> {
    if legacy.authority_snapshot()? == *expected {
        Ok(())
    } else {
        Err(StorageError::InvalidPath(
            "legacy belief authority mutated after migration preparation".to_string(),
        ))
    }
}

fn require_migration_parity(
    legacy: &BeliefStore,
    product: &BeliefStore,
    parity: &BeliefAuthorityParityReceipt,
) -> Result<(), StorageError> {
    require_frozen_source(legacy, parity.source())?;
    if product.authority_snapshot()? == *parity.target() {
        Ok(())
    } else {
        Err(StorageError::InvalidPath(
            "product belief authority no longer matches migration parity".to_string(),
        ))
    }
}

fn validate_migration_marker_successor(
    current: &BeliefAuthorityMigrationMarker,
    next: &BeliefAuthorityMigrationMarker,
) -> Result<(), StorageError> {
    if current.migration() != next.migration() {
        return Err(StorageError::InvalidPath(
            "belief migration marker identity changed".to_string(),
        ));
    }
    let legal = match (current.progress(), next.progress()) {
        (
            BeliefAuthorityMigrationProgress::Prepared { source: current },
            BeliefAuthorityMigrationProgress::Copying {
                source: next,
                verified_record_count: 0,
            },
        ) => current == next,
        (
            BeliefAuthorityMigrationProgress::Copying {
                source: current_source,
                verified_record_count: current_count,
            },
            BeliefAuthorityMigrationProgress::Copying {
                source: next_source,
                verified_record_count: next_count,
            },
        ) => current_source == next_source && next_count >= current_count,
        (
            BeliefAuthorityMigrationProgress::Copying {
                source: current_source,
                ..
            },
            BeliefAuthorityMigrationProgress::Verified { parity },
        ) => current_source == parity.source(),
        (
            BeliefAuthorityMigrationProgress::Verified { parity: current },
            BeliefAuthorityMigrationProgress::Cutover { parity: next },
        ) => current == next,
        (
            BeliefAuthorityMigrationProgress::Cutover { parity: current },
            BeliefAuthorityMigrationProgress::ForwardRepairOnly { parity: next },
        ) => current == next,
        _ => false,
    };
    if legal {
        Ok(())
    } else {
        Err(StorageError::InvalidPath(
            "illegal belief migration marker transition".to_string(),
        ))
    }
}

fn view_subject_index_key(view: &BeliefView) -> String {
    format!(
        "{}::{}::{}",
        view.key.subject.index_key(),
        view.key.perspective.index_key(),
        view.key.index_key()
    )
}

fn require_belief_transaction_value(
    tree: &sled::transaction::TransactionalTree,
    key: &[u8],
    expected: &[u8],
    product: &str,
) -> Result<(), sled::transaction::ConflictableTransactionError<String>> {
    if tree.get(key)?.as_deref() != Some(expected) {
        return Err(sled::transaction::ConflictableTransactionError::Abort(
            format!("{product} changed during readiness attestation"),
        ));
    }
    Ok(())
}

fn require_optional_belief_transaction_value(
    tree: &sled::transaction::TransactionalTree,
    key: &[u8],
    expected: Option<&[u8]>,
    product: &str,
) -> Result<(), sled::transaction::ConflictableTransactionError<String>> {
    if tree.get(key)?.as_deref() != expected {
        return Err(sled::transaction::ConflictableTransactionError::Abort(
            format!("{product} changed during legacy readiness migration"),
        ));
    }
    Ok(())
}

pub(super) fn validate_readiness_owner_hash(owner_fence_hash: &str) -> Result<(), StorageError> {
    if owner_fence_hash.len() != 64
        || !owner_fence_hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(StorageError::InvalidPath(
            "belief readiness owner fence must be a lowercase BLAKE3 digest".to_string(),
        ));
    }
    Ok(())
}

fn validate_readiness_owner_hash_transaction(
    owner_fence_hash: &[u8],
) -> Result<(), sled::transaction::ConflictableTransactionError<String>> {
    if owner_fence_hash.len() != 64
        || !owner_fence_hash
            .iter()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(sled::transaction::ConflictableTransactionError::Abort(
            "legacy readiness owner fence is not a lowercase BLAKE3 digest".to_string(),
        ));
    }
    Ok(())
}

fn evidence_cursor_key(cursor: &EvidenceConsumerCursor) -> Result<Vec<u8>, StorageError> {
    serde_json::to_vec(&(
        &cursor.consumer_id,
        &cursor.family_config_hash,
        &cursor.source_mapping_hash,
        &cursor.perspective,
        &cursor.branch_scope,
    ))
    .map_err(to_storage_data)
}

fn merge_dirty_state(
    existing: Option<DirtyKeyState>,
    key: &BeliefKey,
    dirty_since_seq: u64,
    latest_seq: u64,
    active_lease_id: Option<String>,
    reason: DirtyReason,
    mutation_generation: Option<u64>,
) -> DirtyKeyState {
    match existing {
        Some(existing) => DirtyKeyState {
            belief_key: key.clone(),
            dirty_since_seq: existing.dirty_since_seq.min(dirty_since_seq),
            latest_seq: existing.latest_seq.max(latest_seq),
            mutation_generation: mutation_generation.map_or_else(
                || existing.mutation_generation.saturating_add(1),
                |generation| {
                    existing
                        .mutation_generation
                        .saturating_add(1)
                        .max(generation)
                },
            ),
            assessment_cursor: existing.assessment_cursor,
            active_lease_id: active_lease_id.or(existing.active_lease_id),
            reason,
        },
        None => DirtyKeyState {
            belief_key: key.clone(),
            dirty_since_seq,
            latest_seq,
            mutation_generation: mutation_generation.unwrap_or(1).max(1),
            assessment_cursor: None,
            active_lease_id,
            reason,
        },
    }
}

fn evidence_receipt_key(receipt: &EvidenceIngestionReceipt) -> Result<Vec<u8>, StorageError> {
    serde_json::to_vec(&receipt.identity).map_err(to_storage_data)
}

fn receipt_cursor(receipt: &EvidenceIngestionReceipt) -> EvidenceConsumerCursor {
    EvidenceConsumerCursor {
        consumer_id: receipt.identity.consumer_id.clone(),
        ledger_cursor: crate::events::LedgerCursor {
            ledger_id: receipt.identity.source_record.ledger_id,
            after_seq: receipt.identity.source_record.seq,
        },
        family_config_hash: receipt.identity.family_config_hash.clone(),
        source_mapping_hash: receipt.identity.source_mapping_hash.clone(),
        perspective: receipt.identity.perspective.clone(),
        branch_scope: receipt.identity.branch_scope.clone(),
    }
}

fn receipt_identity_for_cursor(
    cursor: &EvidenceConsumerCursor,
) -> crate::belief::contracts::EvidenceIngestionReceiptIdentity {
    crate::belief::contracts::EvidenceIngestionReceiptIdentity {
        consumer_id: cursor.consumer_id.clone(),
        source_record: crate::events::EventRecordRef {
            ledger_id: cursor.ledger_cursor.ledger_id,
            seq: cursor.ledger_cursor.after_seq,
        },
        family_config_hash: cursor.family_config_hash.clone(),
        source_mapping_hash: cursor.source_mapping_hash.clone(),
        perspective: cursor.perspective.clone(),
        branch_scope: cursor.branch_scope.clone(),
    }
}

fn validate_receipt_cursor_transition(
    expected: Option<&EvidenceConsumerCursor>,
    receipt: &EvidenceIngestionReceipt,
    next: &EvidenceConsumerCursor,
) -> Result<(), StorageError> {
    validate_receipt_next_identity(receipt, next)?;
    if let Some(expected) = expected {
        if evidence_cursor_key(expected)? != evidence_cursor_key(next)?
            || expected.ledger_cursor.ledger_id != next.ledger_cursor.ledger_id
        {
            return Err(StorageError::InvalidPath(
                "evidence consumer cannot cross ledger identity".to_string(),
            ));
        }
        if next.ledger_cursor.after_seq != expected.ledger_cursor.after_seq.saturating_add(1) {
            return Err(StorageError::InvalidPath(
                "evidence consumer cursor must advance by exactly one record".to_string(),
            ));
        }
    } else if next.ledger_cursor.after_seq != 1 {
        return Err(StorageError::InvalidPath(
            "initial evidence consumer cursor must cover canonical sequence one".to_string(),
        ));
    }
    Ok(())
}

fn validate_receipt_next_identity(
    receipt: &EvidenceIngestionReceipt,
    next: &EvidenceConsumerCursor,
) -> Result<(), StorageError> {
    let identity = &receipt.identity;
    if identity.consumer_id != next.consumer_id
        || identity.family_config_hash != next.family_config_hash
        || identity.source_mapping_hash != next.source_mapping_hash
        || identity.perspective != next.perspective
        || identity.branch_scope != next.branch_scope
        || identity.source_record.ledger_id != next.ledger_cursor.ledger_id
        || identity.source_record.seq != next.ledger_cursor.after_seq
    {
        return Err(StorageError::InvalidPath(
            "evidence receipt and consumer cursor identity mismatch".to_string(),
        ));
    }
    Ok(())
}

fn validate_exact_receipt_replay_cursor(
    receipt: &EvidenceIngestionReceipt,
    next: &EvidenceConsumerCursor,
    current: &EvidenceConsumerCursor,
) -> Result<(), StorageError> {
    validate_receipt_next_identity(receipt, next)?;
    if evidence_cursor_key(current)? != evidence_cursor_key(next)?
        || current.ledger_cursor.ledger_id != next.ledger_cursor.ledger_id
    {
        return Err(StorageError::InvalidPath(
            "exact evidence receipt crosses durable consumer identity".to_string(),
        ));
    }
    if current.ledger_cursor.after_seq < next.ledger_cursor.after_seq {
        return Err(StorageError::InvalidPath(
            "exact evidence receipt is ahead of its durable consumer cursor".to_string(),
        ));
    }
    Ok(())
}

fn validate_contiguous_receipt_history(
    cursor: &EvidenceConsumerCursor,
    sequences: &BTreeSet<u64>,
) -> Result<(), StorageError> {
    let mut expected = 1u64;
    for sequence in sequences {
        if *sequence != expected {
            return Err(StorageError::InvalidPath(
                "evidence consumer cursor has a gap in ingestion receipt history".to_string(),
            ));
        }
        expected = expected.checked_add(1).ok_or_else(|| {
            StorageError::InvalidPath(
                "evidence consumer receipt history sequence is exhausted".to_string(),
            )
        })?;
    }
    if expected.saturating_sub(1) != cursor.ledger_cursor.after_seq {
        return Err(StorageError::InvalidPath(
            "evidence consumer cursor exceeds contiguous ingestion receipt history".to_string(),
        ));
    }
    Ok(())
}

fn decode_dirty_state(raw: &[u8]) -> Result<DirtyKeyState, StorageError> {
    if let Ok(state) = serde_json::from_slice(raw) {
        return Ok(state);
    }
    Err(StorageError::InvalidPath(
        "dirty key state requires structured records".to_string(),
    ))
}

fn push_reason(reasons: &mut Vec<FreshnessReason>, reason: FreshnessReason) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

fn mark_view_stale(view: &mut BeliefView, reason: FreshnessReason, detail: &str) {
    view.status = BeliefStatus::Stale;
    view.freshness.stale = true;
    push_reason(&mut view.freshness.reasons, reason.clone());
    view.observation = Some(stale_observation(
        &view.key,
        view.current_revision_id.clone(),
        reason,
    ));
    view.assessment_state = detail.to_string();
}

fn stale_observation(
    key: &BeliefKey,
    revision_id: Option<String>,
    reason: FreshnessReason,
) -> ObservationOpportunity {
    ObservationOpportunity {
        opportunity_id: format!(
            "observation-{}",
            crate::belief::config::stable_hash_hex(
                format!("{}::{reason:?}::{revision_id:?}", key.index_key()).as_bytes()
            )
        ),
        belief_key: key.clone(),
        target_evidence_schema_id: String::new(),
        reason: ObservationReason::StaleEvidence,
        detail: format!("belief view is stale due to {reason:?}"),
        source_revision_id: revision_id,
        open: true,
    }
}

fn to_storage_io(err: sled::Error) -> StorageError {
    StorageError::IoError(io::Error::other(err.to_string()))
}

fn to_storage_data(err: serde_json::Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

fn to_storage_utf8(err: std::string::FromUtf8Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::belief::contracts::{
        BranchScope, ContradictionState, EvidenceIngestionReceiptDisposition,
        EvidenceIngestionReceiptIdentity, EvidenceRole, EvidenceValue, FreshnessState,
        PlannerProjectionSummary, PosteriorSummary,
    };
    use crate::belief::{BeliefConfigLoader, BeliefRuntime};
    use crate::events::{EventRecordRef, LedgerCursor};
    use crate::world_state::graph::store::TraversalStore;

    type DatabaseSnapshot = Vec<(Vec<u8>, Vec<(Vec<u8>, Vec<u8>)>)>;

    fn key() -> BeliefKey {
        BeliefKey {
            subject: DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap(),
            dimension_id: "docs_freshness".to_string(),
            predicate_id: "confidence".to_string(),
            perspective: PerspectiveKey::new("agent", "default").unwrap(),
            branch_scope: BranchScope::main(),
            evidence_policy_id: "policy-a".to_string(),
        }
    }

    fn queued_lease() -> AssessmentLease {
        AssessmentLease {
            lease_id: "lease-a".to_string(),
            belief_key: key(),
            epoch: 1,
            owner_id: "worker-a".to_string(),
            input_cursor_start: 1,
            input_cursor_end: 1,
            assignment_cursor_start: None,
            assignment_cursor_end: None,
            assignment_window_complete: true,
            started_at_seq: 1,
            expires_at_seq: 2,
            comparator_engine_id: "weighted_bayesian".to_string(),
            config_snapshot_hash: "config-a".to_string(),
            status: LeaseStatus::Queued,
        }
    }

    fn runtime_config() -> crate::belief::ConfigSnapshot {
        BeliefConfigLoader::load_json(
            r#"{
              "family_id":"docs_freshness",
              "dimension_id":"docs_freshness",
              "predicate_id":"confidence",
              "evidence_policy_id":"policy-a",
              "evidence_schemas":[{"schema_id":"signal","required":false,"role":"Support","reliability":1.0,"precision":1.0}],
              "source_mappings":[{"mapping_id":"signal","source_kind":"signal","evidence_schema_id":"signal","subject_from":"record.subject","value_field":"value","factor_id":"signal"}],
              "comparator":{"engine_id":"weighted_bayesian","engine_version":"1","factors":[{"factor_id":"signal","evidence_schema_id":"signal","weight":1.0,"polarity":"Supports"}],"missing_evidence_uncertainty":0.9},
              "default_prior":0.5,
              "planner_projection":{"confidence_field":"confidence","threshold":0.5,"posterior_meaning":"probability"},
              "config_version":"1"
            }"#,
        )
        .unwrap()
    }

    fn migration_identity() -> BeliefAuthorityMigrationIdentity {
        BeliefAuthorityMigrationIdentity::try_new("migration-a", "legacy-a", "product-a", 1)
            .unwrap()
    }

    fn fail_next_flush(store: &BeliefStore) {
        store.flush_probe.lock().fail_next = true;
    }

    fn flush_calls(store: &BeliefStore) -> usize {
        store.flush_probe.lock().calls
    }

    fn copy_directory(source: &std::path::Path, target: &std::path::Path) {
        std::fs::create_dir_all(target).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let source_path = entry.path();
            let target_path = target.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_directory(&source_path, &target_path);
            } else {
                std::fs::copy(source_path, target_path).unwrap();
            }
        }
    }

    fn open_test_database(path: &std::path::Path) -> sled::Db {
        for attempt in 0..100 {
            match sled::open(path) {
                Ok(db) => return db,
                Err(error)
                    if error.to_string().contains("could not acquire lock") && attempt < 99 =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(error) => panic!("open belief test database: {error}"),
            }
        }
        unreachable!("the final database open attempt returns")
    }

    fn database_snapshot(db: &Db) -> DatabaseSnapshot {
        let mut tree_names = db.tree_names();
        tree_names.sort();
        tree_names
            .into_iter()
            .map(|tree_name| {
                let tree = db.open_tree(&tree_name).unwrap();
                let records = tree
                    .iter()
                    .map(|item| {
                        let (key, value) = item.unwrap();
                        (key.to_vec(), value.to_vec())
                    })
                    .collect::<Vec<_>>();
                (tree_name.to_vec(), records)
            })
            .collect()
    }

    fn seed_readiness_request(store: &BeliefStore) -> BeliefReadinessAttestationRequest {
        let belief_key = key();
        store.mark_dirty(&belief_key, 1).unwrap();
        let lease = store.acquire_lease(queued_lease()).unwrap();
        let revision = revision(&belief_key, "revision-readiness", 1);
        store.commit_revision(&lease, &revision).unwrap();
        BeliefReadinessAttestationRequest {
            agent_id: "agent-a".to_string(),
            subscription_id: "subscription-a".to_string(),
            belief_key,
            expected_revision_id: revision.revision_id,
            attested_at_seq: 2,
        }
    }
    #[test]
    fn readiness_fenced_prepare_exact_retry_reflushes_and_reopens() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-readiness");
        let request;
        let attestation;
        let owner_hash = "a".repeat(64);
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            let belief_key = key();
            store.mark_dirty(&belief_key, 1).unwrap();
            let lease = store.acquire_lease(queued_lease()).unwrap();
            let revision = revision(&belief_key, "revision-readiness", 1);
            store.commit_revision(&lease, &revision).unwrap();
            request = BeliefReadinessAttestationRequest {
                agent_id: "agent-a".to_string(),
                subscription_id: "subscription-a".to_string(),
                belief_key,
                expected_revision_id: revision.revision_id.clone(),
                attested_at_seq: 2,
            };
            let expected = BeliefReadinessAttestation::identified(
                &request,
                &revision,
                &store.current_view(&request.belief_key).unwrap().unwrap(),
            )
            .unwrap();
            let calls_before_attestation = flush_calls(&store);
            fail_next_flush(&store);
            assert!(matches!(
                store.prepare_readiness_attestation_fenced(&request, &owner_hash),
                Err(StorageError::DurabilityIndeterminate(_))
            ));
            assert_eq!(
                store
                    .get_prepared_readiness_attestation(&expected.attestation_id)
                    .unwrap(),
                Some(expected.clone())
            );
            assert!(store
                .get_readiness_attestation(&expected.attestation_id)
                .unwrap()
                .is_none());
            attestation = store
                .prepare_readiness_attestation_fenced(&request, &owner_hash)
                .unwrap();
            assert_eq!(attestation, expected);
            assert_eq!(flush_calls(&store), calls_before_attestation + 2);
        }
        let reopened = BeliefStore::new(open_test_database(&path)).unwrap();
        assert_eq!(
            reopened
                .get_prepared_readiness_attestation(&attestation.attestation_id)
                .unwrap(),
            Some(attestation.clone())
        );
        let calls_before_verification = flush_calls(&reopened);
        assert!(reopened.verify_readiness_attestation(&attestation).is_err());
        assert_eq!(flush_calls(&reopened), calls_before_verification);

        let mut missing_request = request;
        missing_request.subscription_id = "subscription-missing".to_string();
        let view = reopened
            .current_view(&missing_request.belief_key)
            .unwrap()
            .unwrap();
        let revision = reopened
            .get_revision(&missing_request.expected_revision_id)
            .unwrap()
            .unwrap();
        let missing =
            BeliefReadinessAttestation::identified(&missing_request, &revision, &view).unwrap();
        assert!(reopened.verify_readiness_attestation(&missing).is_err());
    }

    #[test]
    fn assessment_lease_exact_retry_reflushes_and_reopens() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-lease");
        let acquired;
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            store.mark_dirty(&key(), 1).unwrap();
            let baseline_flushes = flush_calls(&store);
            fail_next_flush(&store);
            assert!(matches!(
                store.acquire_lease(queued_lease()),
                Err(StorageError::DurabilityIndeterminate(_))
            ));
            assert_eq!(flush_calls(&store), baseline_flushes + 1);
            acquired = store.acquire_lease(queued_lease()).unwrap();
            assert_eq!(flush_calls(&store), baseline_flushes + 2);
            assert_eq!(
                store.get_lease(&acquired.lease_id).unwrap(),
                Some(acquired.clone())
            );
            assert_eq!(
                store.dirty_state(&key()).unwrap().unwrap().active_lease_id,
                Some(acquired.lease_id.clone())
            );
        }
        let reopened = BeliefStore::new(open_test_database(&path)).unwrap();
        assert_eq!(
            reopened.get_lease(&acquired.lease_id).unwrap(),
            Some(acquired.clone())
        );
        assert_eq!(
            reopened
                .dirty_state(&key())
                .unwrap()
                .unwrap()
                .active_lease_id,
            Some(acquired.lease_id)
        );
    }

    #[test]
    fn completed_lease_id_cannot_be_reused_for_another_belief_key() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-completed-lease-reuse");
        let first_revision;
        let completed_lease;
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            store.mark_dirty(&key(), 1).unwrap();
            let acquired = store.acquire_lease(queued_lease()).unwrap();
            first_revision = revision(&key(), "revision-lease-owner", 1);
            store.commit_revision(&acquired, &first_revision).unwrap();
            completed_lease = store.get_lease("lease-a").unwrap().unwrap();
            assert_eq!(completed_lease.status, LeaseStatus::Completed);

            let mut second_key = key();
            second_key.subject = DomainObjectRef::new("workspace_fs", "node", "node-b").unwrap();
            store.mark_dirty(&second_key, 2).unwrap();
            let mut reused = queued_lease();
            reused.belief_key = second_key;
            reused.input_cursor_start = 2;
            reused.input_cursor_end = 2;
            reused.started_at_seq = 2;
            reused.expires_at_seq = 3;

            let snapshot = |tree: &Tree| {
                tree.iter()
                    .map(|item| {
                        let (key, value) = item.unwrap();
                        (key.to_vec(), value.to_vec())
                    })
                    .collect::<Vec<_>>()
            };
            let before = [
                snapshot(&store.leases),
                snapshot(&store.active_lease),
                snapshot(&store.dirty_keys),
                snapshot(&store.commit_intents),
                snapshot(&store.commit_intent_by_lease),
            ];
            assert!(matches!(
                store.acquire_lease(reused),
                Err(StorageError::Backpressure(message))
                    if message.contains("lease id already belongs")
            ));
            let after = [
                snapshot(&store.leases),
                snapshot(&store.active_lease),
                snapshot(&store.dirty_keys),
                snapshot(&store.commit_intents),
                snapshot(&store.commit_intent_by_lease),
            ];
            assert_eq!(after, before);
            assert_eq!(
                store.get_lease("lease-a").unwrap(),
                Some(completed_lease.clone())
            );
            assert_eq!(
                store
                    .exact_commit_progress_for_revision(&first_revision)
                    .unwrap(),
                Some(1)
            );
        }

        let reopened = BeliefStore::new(open_test_database(&path)).unwrap();
        assert_eq!(
            reopened.get_lease("lease-a").unwrap(),
            Some(completed_lease)
        );
        assert_eq!(
            reopened.current_revision(&key()).unwrap(),
            Some(first_revision.clone())
        );
        assert_eq!(
            reopened
                .exact_commit_progress_for_revision(&first_revision)
                .unwrap(),
            Some(1)
        );
    }

    #[test]
    fn indeterminate_assessment_acquire_resumes_exact_window_after_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-indeterminate-runtime");
        let config = runtime_config();
        let perspective = PerspectiveKey::new("agent", "default").unwrap();
        let branch_scope = BranchScope::main();
        let active_lease_id;
        {
            let db = open_test_database(&path);
            let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
            let traversal = Arc::new(TraversalStore::new(db).unwrap());
            let (item, assignment) = evidence_and_assignment("indeterminate", 1);
            store.put_evidence_once(&item).unwrap();
            store.put_assignment_once(&assignment).unwrap();
            let runtime = BeliefRuntime::new(
                Arc::clone(&store),
                traversal,
                config.clone(),
                perspective.clone(),
                branch_scope.clone(),
            );
            let lease_clock = store.advance_assessment_lease_clock().unwrap();
            fail_next_flush(&store);

            assert!(matches!(
                runtime.assess_dirty_key_bounded_outcome(
                    &key(),
                    "exact-replay-owner",
                    1024,
                    lease_clock,
                ),
                Err(StorageError::DurabilityIndeterminate(_))
            ));
            let active = store.active_lease_for_key(&key()).unwrap().unwrap();
            active_lease_id = active.lease_id;
            assert!(store.current_revision(&key()).unwrap().is_none());
            // Resolve the persisted branch of the indeterminate write before
            // simulating process loss. The reopened runtime must reuse it.
            store.flush().unwrap();
        }

        let db = open_test_database(&path);
        let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        let traversal = Arc::new(TraversalStore::new(db).unwrap());
        let runtime = BeliefRuntime::new(
            Arc::clone(&store),
            traversal,
            config,
            perspective,
            branch_scope,
        );
        let lease_clock = store.advance_assessment_lease_clock().unwrap();
        let outcome = runtime
            .assess_dirty_key_bounded_outcome(&key(), "exact-replay-owner", 1024, lease_clock)
            .unwrap();

        assert!(outcome.assessment.is_some());
        assert!(store.active_lease_for_key(&key()).unwrap().is_none());
        assert_eq!(
            store.get_lease(&active_lease_id).unwrap().unwrap().status,
            LeaseStatus::Completed
        );
        assert!(store.dirty_state(&key()).unwrap().is_none());
    }

    #[test]
    fn pre_progress_base_format_migrates_to_exact_legacy_assessment_receipt() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-base-format");
        let expected_revision;
        let expected_completed_lease;
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            let (item, assignment) = evidence_and_assignment("legacy-base", 1);
            store.put_evidence_once(&item).unwrap();
            store.put_assignment_once(&assignment).unwrap();
            let lease = store.acquire_lease(queued_lease()).unwrap();
            let mut revision = revision(&key(), "revision-legacy-base", 1);
            revision.evidence_ids = vec![item.evidence_id];
            let view = store.project_view(
                &revision,
                HydrationRefs {
                    evidence_ids: revision.evidence_ids.clone(),
                    source_fact_ids: revision.provenance.source_fact_ids.clone(),
                    graph_anchor_ids: revision.provenance.graph_anchor_ids.clone(),
                    revision_id: Some(revision.revision_id.clone()),
                },
            );
            store
                .commit_belief_assessment(&lease, revision.clone(), view)
                .unwrap();
            expected_revision = revision;
            expected_completed_lease = store.get_lease("lease-a").unwrap().unwrap();

            // Install the exact durable shape written by base d729e1f. The
            // completed lease lacks bounded cursor fields and the newer
            // derived indexes, commit receipts, and progress keys do not exist.
            let mut legacy_lease = serde_json::to_value(&expected_completed_lease).unwrap();
            let legacy_object = legacy_lease.as_object_mut().unwrap();
            legacy_object.remove("assignment_cursor_start");
            legacy_object.remove("assignment_cursor_end");
            legacy_object.remove("assignment_window_complete");
            let legacy_lease = serde_json::to_vec(&legacy_lease).unwrap();
            assert!(!String::from_utf8_lossy(&legacy_lease).contains("assignment_cursor"));
            store.leases.insert(b"lease-a", legacy_lease).unwrap();
            store.assignments_by_key.clear().unwrap();
            store.assignment_tail_by_key.clear().unwrap();
            store.committed_evidence.clear().unwrap();
            store.commit_receipts.clear().unwrap();
            store.commit_intent_by_lease.clear().unwrap();
            store.legacy_assessment_receipts.clear().unwrap();
            store
                .authority_meta
                .remove(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE)
                .unwrap();
            let assessment_keys = store
                .runtime_meta
                .scan_prefix(b"assessment_")
                .map(|item| item.unwrap().0)
                .collect::<Vec<_>>();
            for key in assessment_keys {
                store.runtime_meta.remove(key).unwrap();
            }
            store.flush().unwrap();
        }

        let reopened = BeliefStore::new(open_test_database(&path)).unwrap();
        let migrated = reopened
            .legacy_assessment_receipts
            .get(expected_revision.revision_id.as_bytes())
            .unwrap()
            .unwrap();
        let migrated: LegacyAssessmentReceipt = serde_json::from_slice(&migrated).unwrap();
        let fence: LegacyAssessmentMigrationFence = serde_json::from_slice(
            &reopened
                .authority_meta
                .get(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE)
                .unwrap()
                .unwrap(),
        )
        .unwrap();

        assert_eq!(
            migrated.schema_version,
            LEGACY_ASSESSMENT_RECEIPT_SCHEMA_VERSION
        );
        assert_eq!(
            fence.schema_version,
            LEGACY_ASSESSMENT_MIGRATION_SCHEMA_VERSION
        );
        assert_eq!(
            fence.migrated_revision_ids,
            vec![expected_revision.revision_id.clone()]
        );
        assert!(fence.source_snapshot.record_count() > 0);
        assert_eq!(migrated.revision, expected_revision);
        assert_eq!(migrated.completed_lease, expected_completed_lease);
        assert_eq!(migrated.legacy_progress_sequence, 0);
        assert_eq!(
            reopened
                .exact_commit_progress_for_revision(&migrated.revision)
                .unwrap(),
            Some(0)
        );
        assert_eq!(
            reopened
                .committed_revision_for_evidence(&key(), "evidence-legacy-base")
                .unwrap(),
            Some(migrated.revision)
        );
    }

    #[test]
    fn pre_assessment_handoff_checkpoint_reopens_without_partial_receipts_and_resumes() {
        let source_temp = tempfile::tempdir().unwrap();
        let product_temp = tempfile::tempdir().unwrap();
        let source_path = source_temp.path().join("belief-source-pre-handoff");
        let expected_revision = prepare_pre_progress_source(&source_path);
        let source = BeliefStore::new(open_test_database(&source_path)).unwrap();
        let product_path = product_temp.path().join("belief-product-pre-handoff");
        let product = BeliefStore::new(open_test_database(&product_path)).unwrap();
        product
            .advance_legacy_authority_migration(&source, migration_identity())
            .unwrap();
        product
            .advance_legacy_authority_migration(&source, migration_identity())
            .unwrap();

        let count_before_handoff = record_count_before_assessment_payload(&source);
        assert!(count_before_handoff > 0);
        product.fail_flush_after_for_test(
            usize::try_from(count_before_handoff)
                .unwrap()
                .saturating_add(1),
        );
        assert!(product
            .advance_legacy_authority_migration(&source, migration_identity())
            .is_err());
        assert_eq!(
            product
                .legacy_assessment_receipts
                .iter()
                .filter(|item| {
                    item.as_ref().unwrap().0.as_ref()
                        != KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE
                })
                .count(),
            0
        );
        drop(product);

        let product_db = open_test_database(&product_path);
        let before_open = database_snapshot(&product_db);
        assert!(matches!(
            BeliefStore::new(product_db.clone()),
            Err(StorageError::Unavailable(message))
                if message.contains("requires coordinator recovery")
        ));
        assert_eq!(database_snapshot(&product_db), before_open);
        let recovery =
            BeliefStore::open_for_authority_migration(product_db.clone(), migration_identity())
                .unwrap();
        assert_eq!(
            recovery.resume(&source).unwrap(),
            LegacyBeliefCompatibilityPosture::ProductAuthoritative
        );
        recovery.flush().unwrap();
        drop(recovery);

        let product = BeliefStore::new(product_db).unwrap();
        assert_eq!(product.assignments_by_key.len(), 1);
        assert_eq!(product.assignment_tail_by_key.len(), 1);
        assert_eq!(
            product
                .exact_commit_progress_for_revision(&expected_revision)
                .unwrap(),
            Some(0)
        );
    }

    #[test]
    fn predecessor_partial_assessment_payload_reopens_and_resumes_atomically() {
        let source_temp = tempfile::tempdir().unwrap();
        let product_temp = tempfile::tempdir().unwrap();
        let source_path = source_temp.path().join("belief-source-partial-payload");
        let expected_revision = prepare_pre_progress_source(&source_path);
        let source = BeliefStore::new(open_test_database(&source_path)).unwrap();
        let product_path = product_temp.path().join("belief-product-partial-payload");
        let product = BeliefStore::new(open_test_database(&product_path)).unwrap();
        product
            .advance_legacy_authority_migration(&source, migration_identity())
            .unwrap();
        product
            .advance_legacy_authority_migration(&source, migration_identity())
            .unwrap();

        let mut verified_record_count = copy_records_before_assessment_payload(&source, &product);
        let (receipt_key, receipt_value) = source
            .legacy_assessment_receipts
            .iter()
            .map(|item| item.unwrap())
            .find(|(key, _)| key.as_ref() != KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE)
            .unwrap();
        product
            .legacy_assessment_receipts
            .insert(receipt_key, receipt_value)
            .unwrap();
        verified_record_count = verified_record_count.saturating_add(1);
        let partial = BeliefAuthorityMigrationMarker::try_new(
            migration_identity(),
            BeliefAuthorityMigrationProgress::Copying {
                source: source.authority_snapshot().unwrap(),
                verified_record_count,
            },
        )
        .unwrap();
        product.put_authority_migration_marker(&partial).unwrap();
        product.flush().unwrap();
        drop(product);

        let product_db = open_test_database(&product_path);
        let before_open = database_snapshot(&product_db);
        assert!(matches!(
            BeliefStore::new(product_db.clone()),
            Err(StorageError::Unavailable(_))
        ));
        assert_eq!(database_snapshot(&product_db), before_open);
        let recovery =
            BeliefStore::open_for_authority_migration(product_db.clone(), migration_identity())
                .unwrap();
        assert_eq!(
            recovery.resume(&source).unwrap(),
            LegacyBeliefCompatibilityPosture::ProductAuthoritative
        );
        drop(recovery);

        let product = BeliefStore::new(product_db).unwrap();
        assert!(matches!(
            product
                .authority_migration_marker()
                .unwrap()
                .unwrap()
                .progress(),
            BeliefAuthorityMigrationProgress::Cutover { .. }
        ));
        assert_eq!(
            product
                .exact_commit_progress_for_revision(&expected_revision)
                .unwrap(),
            Some(0)
        );
    }

    #[test]
    fn conflicting_partial_assessment_payload_aborts_atomic_handoff_without_mutation() {
        let source_temp = tempfile::tempdir().unwrap();
        let product_temp = tempfile::tempdir().unwrap();
        let source_path = source_temp.path().join("belief-source-conflicting-payload");
        let expected_revision = prepare_pre_progress_source(&source_path);
        let source = BeliefStore::new(open_test_database(&source_path)).unwrap();
        let product_path = product_temp
            .path()
            .join("belief-product-conflicting-payload");
        let product = BeliefStore::new(open_test_database(&product_path)).unwrap();
        product
            .advance_legacy_authority_migration(&source, migration_identity())
            .unwrap();
        product
            .advance_legacy_authority_migration(&source, migration_identity())
            .unwrap();

        let verified_record_count = copy_records_before_assessment_payload(&source, &product);
        let partial = BeliefAuthorityMigrationMarker::try_new(
            migration_identity(),
            BeliefAuthorityMigrationProgress::Copying {
                source: source.authority_snapshot().unwrap(),
                verified_record_count,
            },
        )
        .unwrap();
        product.put_authority_migration_marker(&partial).unwrap();
        product
            .legacy_assessment_receipts
            .insert(
                expected_revision.revision_id.as_bytes(),
                b"conflicting-receipt",
            )
            .unwrap();
        product.flush().unwrap();
        let portable_before = product
            .legacy_assessment_receipts
            .get(KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE)
            .unwrap();
        let local_before = product
            .authority_meta
            .get(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE)
            .unwrap();

        assert!(matches!(
            product.advance_legacy_authority_migration(&source, migration_identity()),
            Err(StorageError::InvalidPath(message))
                if message.contains("assessment receipt conflicts")
        ));
        assert_eq!(
            product.authority_migration_marker().unwrap(),
            Some(partial.clone())
        );
        assert_eq!(
            product
                .legacy_assessment_receipts
                .get(KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE)
                .unwrap(),
            portable_before
        );
        assert_eq!(
            product
                .authority_meta
                .get(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE)
                .unwrap(),
            local_before
        );
        assert_eq!(
            product
                .legacy_assessment_receipts
                .get(expected_revision.revision_id.as_bytes())
                .unwrap()
                .as_deref(),
            Some(b"conflicting-receipt".as_slice())
        );
        drop(product);

        let product_db = open_test_database(&product_path);
        let before_open = database_snapshot(&product_db);
        assert!(matches!(
            BeliefStore::new(product_db.clone()),
            Err(StorageError::Unavailable(_))
        ));
        assert_eq!(database_snapshot(&product_db), before_open);
        let recovery =
            BeliefStore::open_for_authority_migration(product_db, migration_identity()).unwrap();
        assert!(matches!(
            recovery.resume(&source),
            Err(StorageError::InvalidPath(message))
                if message.contains("assessment receipt conflicts")
        ));
    }

    #[test]
    fn cross_authority_pre_progress_fence_handoff_reopens_atomically_and_serves_product() {
        let source_temp = tempfile::tempdir().unwrap();
        let product_temp = tempfile::tempdir().unwrap();
        let source_path = source_temp.path().join("belief-source-base-format");
        let expected_revision;
        {
            let store = BeliefStore::new(open_test_database(&source_path)).unwrap();
            let (item, assignment) = evidence_and_assignment("legacy-cross-authority", 1);
            store.put_evidence_once(&item).unwrap();
            store.put_assignment_once(&assignment).unwrap();
            let lease = store.acquire_lease(queued_lease()).unwrap();
            let mut revision = revision(&key(), "revision-legacy-cross-authority", 1);
            revision.evidence_ids = vec![item.evidence_id];
            let view = store.project_view(
                &revision,
                HydrationRefs {
                    evidence_ids: revision.evidence_ids.clone(),
                    source_fact_ids: revision.provenance.source_fact_ids.clone(),
                    graph_anchor_ids: revision.provenance.graph_anchor_ids.clone(),
                    revision_id: Some(revision.revision_id.clone()),
                },
            );
            store
                .commit_belief_assessment(&lease, revision.clone(), view)
                .unwrap();
            expected_revision = revision;

            let completed = store.get_lease("lease-a").unwrap().unwrap();
            let mut legacy_lease = serde_json::to_value(completed).unwrap();
            let legacy_object = legacy_lease.as_object_mut().unwrap();
            legacy_object.remove("assignment_cursor_start");
            legacy_object.remove("assignment_cursor_end");
            legacy_object.remove("assignment_window_complete");
            store
                .leases
                .insert(b"lease-a", serde_json::to_vec(&legacy_lease).unwrap())
                .unwrap();
            store.assignments_by_key.clear().unwrap();
            store.assignment_tail_by_key.clear().unwrap();
            store.committed_evidence.clear().unwrap();
            store.commit_receipts.clear().unwrap();
            store.commit_intent_by_lease.clear().unwrap();
            store.legacy_assessment_receipts.clear().unwrap();
            store
                .authority_meta
                .remove(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE)
                .unwrap();
            let assessment_keys = store
                .runtime_meta
                .scan_prefix(b"assessment_")
                .map(|item| item.unwrap().0)
                .collect::<Vec<_>>();
            for key in assessment_keys {
                store.runtime_meta.remove(key).unwrap();
            }
            store.flush().unwrap();
        }

        let source = BeliefStore::new(open_test_database(&source_path)).unwrap();
        assert_eq!(
            source
                .exact_commit_progress_for_revision(&expected_revision)
                .unwrap(),
            Some(0)
        );
        let product_path = product_temp.path().join("belief-product");
        let product = BeliefStore::new(open_test_database(&product_path)).unwrap();
        product
            .advance_legacy_authority_migration(&source, migration_identity())
            .unwrap();
        product
            .advance_legacy_authority_migration(&source, migration_identity())
            .unwrap();
        let records_through_fence = source.authority_snapshot().unwrap().record_count();
        let count_before_handoff = record_count_before_assessment_payload(&source);
        product.fail_flush_after_for_test(
            usize::try_from(count_before_handoff)
                .unwrap()
                .saturating_add(2),
        );
        assert!(product
            .advance_legacy_authority_migration(&source, migration_identity())
            .is_err());
        drop(product);
        let product_db = open_test_database(&product_path);
        let authority_meta = product_db.open_tree(TREE_AUTHORITY_META).unwrap();
        let legacy_receipts = product_db
            .open_tree(TREE_LEGACY_ASSESSMENT_RECEIPTS)
            .unwrap();
        let migration = product_db.open_tree(TREE_AUTHORITY_MIGRATION).unwrap();
        let local_after_handoff = authority_meta
            .get(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE)
            .unwrap()
            .unwrap();
        assert_eq!(
            legacy_receipts
                .get(KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE)
                .unwrap()
                .as_deref(),
            Some(local_after_handoff.as_ref())
        );
        let checkpoint: BeliefAuthorityMigrationMarker = serde_json::from_slice(
            &migration
                .get(KEY_AUTHORITY_MIGRATION_MARKER)
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        assert!(matches!(
            checkpoint.progress(),
            BeliefAuthorityMigrationProgress::Copying {
                verified_record_count,
                ..
            } if *verified_record_count == records_through_fence
        ));
        assert!(matches!(
            BeliefStore::new(product_db.clone()),
            Err(StorageError::Unavailable(_))
        ));
        let recovery =
            BeliefStore::open_for_authority_migration(product_db.clone(), migration_identity())
                .unwrap();
        assert_eq!(
            recovery.resume(&source).unwrap(),
            LegacyBeliefCompatibilityPosture::ProductAuthoritative
        );
        drop(recovery);
        drop(authority_meta);
        drop(legacy_receipts);
        drop(migration);

        let product = BeliefStore::new(product_db).unwrap();
        let product_fence = product
            .authority_meta
            .get(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE)
            .unwrap()
            .unwrap();
        assert_eq!(
            product
                .legacy_assessment_receipts
                .get(KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE)
                .unwrap()
                .as_deref(),
            Some(product_fence.as_ref())
        );
        assert_eq!(
            product
                .exact_commit_progress_for_revision(&expected_revision)
                .unwrap(),
            Some(0)
        );
        assert_eq!(
            product
                .committed_revision_for_evidence(
                    &expected_revision.belief_key,
                    "evidence-legacy-cross-authority",
                )
                .unwrap(),
            Some(expected_revision)
        );
    }

    #[test]
    fn reopen_rejects_interior_evidence_receipt_loss() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-interior-receipt-loss");
        let ledger_id = "00000000-0000-0000-0000-000000000123".parse().unwrap();
        let mut expected = None;
        let mut middle_receipt_key = None;
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            for sequence in 1..=3 {
                let cursor = EvidenceConsumerCursor {
                    consumer_id: "consumer-history".to_string(),
                    ledger_cursor: LedgerCursor {
                        ledger_id,
                        after_seq: sequence,
                    },
                    family_config_hash: "family-history".to_string(),
                    source_mapping_hash: "mapping-history".to_string(),
                    perspective: PerspectiveKey::new("agent", "default").unwrap(),
                    branch_scope: BranchScope::main(),
                };
                let receipt = EvidenceIngestionReceipt {
                    identity: EvidenceIngestionReceiptIdentity {
                        consumer_id: cursor.consumer_id.clone(),
                        source_record: EventRecordRef {
                            ledger_id,
                            seq: sequence,
                        },
                        family_config_hash: cursor.family_config_hash.clone(),
                        source_mapping_hash: cursor.source_mapping_hash.clone(),
                        perspective: cursor.perspective.clone(),
                        branch_scope: cursor.branch_scope.clone(),
                    },
                    disposition: EvidenceIngestionReceiptDisposition::Irrelevant,
                    evidence_ids: Vec::new(),
                };
                store
                    .record_evidence_receipt_and_advance(expected.as_ref(), &receipt, &cursor)
                    .unwrap();
                if sequence == 2 {
                    middle_receipt_key = Some(evidence_receipt_key(&receipt).unwrap());
                }
                expected = Some(cursor);
            }
            store
                .evidence_ingestion_receipts
                .remove(middle_receipt_key.unwrap())
                .unwrap();
            store.flush().unwrap();
        }

        assert!(matches!(
            BeliefStore::new(open_test_database(&path)),
            Err(StorageError::InvalidPath(message))
                if message == "evidence consumer cursor has a gap in ingestion receipt history"
        ));
    }

    #[test]
    fn reopen_rejects_completed_lease_reuse_across_current_and_legacy_receipts() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-cross-format-lease-reuse");
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            let belief_key = key();
            store.mark_dirty(&belief_key, 1).unwrap();
            let lease = store.acquire_lease(queued_lease()).unwrap();
            let current_revision = revision(&belief_key, "revision-a-current", 1);
            let view = store.project_view(
                &current_revision,
                HydrationRefs {
                    evidence_ids: Vec::new(),
                    source_fact_ids: Vec::new(),
                    graph_anchor_ids: Vec::new(),
                    revision_id: Some(current_revision.revision_id.clone()),
                },
            );
            store
                .commit_belief_assessment(&lease, current_revision.clone(), view)
                .unwrap();
            let completed = store.get_lease(&lease.lease_id).unwrap().unwrap();
            let mut legacy_revision = revision(&belief_key, "revision-b-legacy", 1);
            legacy_revision.prior_revision_id = Some(current_revision.revision_id);
            store
                .revisions
                .insert(
                    legacy_revision.revision_id.as_bytes(),
                    serde_json::to_vec(&legacy_revision).unwrap(),
                )
                .unwrap();
            store
                .revision_head
                .insert(
                    belief_key.index_key().as_bytes(),
                    legacy_revision.revision_id.as_bytes(),
                )
                .unwrap();
            let legacy_receipt = LegacyAssessmentReceipt {
                schema_version: LEGACY_ASSESSMENT_RECEIPT_SCHEMA_VERSION,
                revision: legacy_revision.clone(),
                completed_lease: completed,
                legacy_progress_sequence: 0,
            };
            store
                .legacy_assessment_receipts
                .insert(
                    legacy_revision.revision_id.as_bytes(),
                    serde_json::to_vec(&legacy_receipt).unwrap(),
                )
                .unwrap();
            let fence = LegacyAssessmentMigrationFence {
                schema_version: LEGACY_ASSESSMENT_MIGRATION_SCHEMA_VERSION,
                source_snapshot: BeliefAuthoritySnapshot::try_new("a".repeat(64), 1).unwrap(),
                migrated_revision_ids: vec![legacy_revision.revision_id],
            };
            let fence = serde_json::to_vec(&fence).unwrap();
            store
                .authority_meta
                .insert(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE, fence.as_slice())
                .unwrap();
            store
                .legacy_assessment_receipts
                .insert(
                    KEY_PORTABLE_LEGACY_ASSESSMENT_MIGRATION_FENCE,
                    fence.as_slice(),
                )
                .unwrap();
            store.flush().unwrap();
        }

        assert!(matches!(
            BeliefStore::new(open_test_database(&path)),
            Err(StorageError::InvalidPath(message))
                if message == "completed belief assessment lease authenticates multiple reachable revisions"
        ));
    }

    #[test]
    fn post_cutover_current_receipt_loss_fails_closed_on_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-current-receipt-loss");
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            let belief_key = key();
            store.mark_dirty(&belief_key, 1).unwrap();
            let lease = store.acquire_lease(queued_lease()).unwrap();
            let revision = revision(&belief_key, "revision-current-loss", 1);
            let view = store.project_view(
                &revision,
                HydrationRefs {
                    evidence_ids: Vec::new(),
                    source_fact_ids: Vec::new(),
                    graph_anchor_ids: Vec::new(),
                    revision_id: Some(revision.revision_id.clone()),
                },
            );
            store
                .commit_belief_assessment(&lease, revision.clone(), view)
                .unwrap();
            store
                .commit_receipts
                .remove(revision_id_to_intent_id(&revision.revision_id))
                .unwrap();
            store.flush().unwrap();
        }

        assert!(matches!(
            BeliefStore::new(open_test_database(&path)),
            Err(StorageError::InvalidPath(message))
                if message == "current-format belief revision lost its commit receipt after cutover"
        ));
    }

    #[test]
    fn pre_progress_migration_rejects_ambiguous_completed_lease_authority() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-ambiguous-legacy-lease");
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            let belief_key = key();
            store.mark_dirty(&belief_key, 1).unwrap();
            let lease = store.acquire_lease(queued_lease()).unwrap();
            let revision = revision(&belief_key, "revision-ambiguous-legacy", 1);
            let view = store.project_view(
                &revision,
                HydrationRefs {
                    evidence_ids: Vec::new(),
                    source_fact_ids: Vec::new(),
                    graph_anchor_ids: Vec::new(),
                    revision_id: Some(revision.revision_id.clone()),
                },
            );
            store
                .commit_belief_assessment(&lease, revision, view)
                .unwrap();
            let mut duplicate = store.get_lease(&lease.lease_id).unwrap().unwrap();
            duplicate.lease_id = "lease-ambiguous-duplicate".to_string();
            store
                .leases
                .insert(
                    duplicate.lease_id.as_bytes(),
                    serde_json::to_vec(&duplicate).unwrap(),
                )
                .unwrap();
            store.commit_receipts.clear().unwrap();
            store.legacy_assessment_receipts.clear().unwrap();
            store
                .authority_meta
                .remove(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE)
                .unwrap();
            let assessment_keys = store
                .runtime_meta
                .scan_prefix(b"assessment_")
                .map(|item| item.unwrap().0)
                .collect::<Vec<_>>();
            for key in assessment_keys {
                store.runtime_meta.remove(key).unwrap();
            }
            store.flush().unwrap();
        }

        assert!(matches!(
            BeliefStore::new(open_test_database(&path)),
            Err(StorageError::InvalidPath(message))
                if message.contains("requires exactly one matching completed lease")
        ));
    }

    #[test]
    fn empty_assessment_exact_retry_reflushes_after_final_apply_failure() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-empty-window");
        let completed_lease;
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            store.mark_dirty(&key(), 1).unwrap();
            completed_lease = store.acquire_lease(queued_lease()).unwrap();
            let baseline_flushes = flush_calls(&store);
            fail_next_flush(&store);
            assert!(matches!(
                store.complete_empty_assessment_window(&completed_lease),
                Err(StorageError::DurabilityIndeterminate(_))
            ));
            assert_eq!(flush_calls(&store), baseline_flushes + 1);
            assert_eq!(store.assessment_progress_sequence().unwrap(), 1);
            assert_eq!(
                store
                    .complete_empty_assessment_window(&completed_lease)
                    .unwrap(),
                1
            );
            assert_eq!(flush_calls(&store), baseline_flushes + 2);
            assert_eq!(store.assessment_progress_sequence().unwrap(), 1);

            let mut conflicting = completed_lease.clone();
            conflicting.owner_id = "conflicting-worker".to_string();
            assert!(store
                .complete_empty_assessment_window(&conflicting)
                .is_err());
            assert_eq!(store.assessment_progress_sequence().unwrap(), 1);
        }

        let reopened = BeliefStore::new(open_test_database(&path)).unwrap();
        assert_eq!(
            reopened
                .complete_empty_assessment_window(&completed_lease)
                .unwrap(),
            1
        );
        assert_eq!(reopened.assessment_progress_sequence().unwrap(), 1);
        assert!(reopened.dirty_state(&key()).unwrap().is_none());
        assert!(reopened
            .active_lease
            .get(key().index_key().as_bytes())
            .unwrap()
            .is_none());
        let mut terminal = completed_lease;
        terminal.status = LeaseStatus::Completed;
        assert_eq!(
            reopened.get_lease(&terminal.lease_id).unwrap(),
            Some(terminal)
        );
    }

    #[test]
    fn stale_subject_writer_cannot_commit_after_logical_clock_recovery() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-subject-lease");
        let belief_key = key();
        let stale_lease;
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            store.mark_dirty(&belief_key, 1).unwrap();
            let lease_clock = store.advance_assessment_lease_clock().unwrap();
            let mut lease = queued_lease();
            lease.lease_id = format!(
                "lease-{lease_clock}-{}-{}",
                lease.input_cursor_end,
                belief_key.index_key()
            );
            lease.started_at_seq = lease_clock;
            lease.expires_at_seq = lease_clock + 100;
            stale_lease = store.acquire_lease(lease).unwrap();
            for _ in 0..100 {
                store.advance_assessment_lease_clock().unwrap();
            }
        }

        let reopened = BeliefStore::new(open_test_database(&path)).unwrap();
        let recovery_clock = reopened.assessment_lease_clock().unwrap();
        assert_eq!(recovery_clock, stale_lease.expires_at_seq);
        let abandoned = reopened
            .recover_expired_lease_for_key(&belief_key, recovery_clock)
            .unwrap()
            .unwrap();
        assert_eq!(abandoned.status, LeaseStatus::Abandoned);

        let retry_clock = reopened.advance_assessment_lease_clock().unwrap();
        let mut replacement = queued_lease();
        replacement.lease_id = format!(
            "lease-{retry_clock}-{}-{}",
            replacement.input_cursor_end,
            belief_key.index_key()
        );
        replacement.owner_id = "worker-recovered".to_string();
        replacement.started_at_seq = retry_clock;
        replacement.expires_at_seq = retry_clock + 100;
        let replacement = reopened.acquire_lease(replacement).unwrap();
        assert_ne!(replacement.lease_id, stale_lease.lease_id);

        let stale_revision = revision(&belief_key, "revision-stale-subject", 1);
        assert!(reopened
            .commit_revision(&stale_lease, &stale_revision)
            .is_err());
        assert_eq!(
            reopened
                .dirty_state(&belief_key)
                .unwrap()
                .unwrap()
                .active_lease_id,
            Some(replacement.lease_id)
        );
    }

    #[test]
    fn expired_prepared_subject_commit_is_revoked_before_reopen_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-prepared-subject");
        let belief_key = key();
        let revision = revision(&belief_key, "revision-prepared-subject", 1);
        let view;
        let stale_lease;
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            store.mark_dirty(&belief_key, 1).unwrap();
            let lease_clock = store.advance_assessment_lease_clock().unwrap();
            let mut lease = queued_lease();
            lease.lease_id = format!(
                "lease-{lease_clock}-{}-{}",
                lease.input_cursor_end,
                belief_key.index_key()
            );
            lease.started_at_seq = lease_clock;
            lease.expires_at_seq = lease_clock + 1;
            stale_lease = store.acquire_lease(lease).unwrap();
            view = store.project_view(
                &revision,
                HydrationRefs {
                    evidence_ids: Vec::new(),
                    source_fact_ids: Vec::new(),
                    graph_anchor_ids: Vec::new(),
                    revision_id: Some(revision.revision_id.clone()),
                },
            );
            let mut completed = stale_lease.clone();
            completed.status = LeaseStatus::Completed;
            let intent = BeliefCommitIntent::try_new(
                format!("belief-commit-{}", revision.revision_id),
                stale_lease.clone(),
                completed,
                store.dirty_state(&belief_key).unwrap().unwrap(),
                revision.clone(),
                view.clone(),
            )
            .unwrap();
            store.prepare_belief_commit(&intent).unwrap();
            assert_eq!(store.advance_assessment_lease_clock().unwrap(), 2);
        }

        let reopened = BeliefStore::new(open_test_database(&path)).unwrap();
        assert!(reopened.current_revision(&belief_key).unwrap().is_none());
        assert_eq!(
            reopened
                .get_lease(&stale_lease.lease_id)
                .unwrap()
                .unwrap()
                .status,
            LeaseStatus::Abandoned
        );
        assert!(reopened
            .commit_intents
            .get(format!("belief-commit-{}", revision.revision_id))
            .unwrap()
            .is_none());

        let retry_clock = reopened.advance_assessment_lease_clock().unwrap();
        let mut replacement = queued_lease();
        replacement.lease_id = format!(
            "lease-{retry_clock}-{}-{}",
            replacement.input_cursor_end,
            belief_key.index_key()
        );
        replacement.owner_id = "worker-replacement".to_string();
        replacement.started_at_seq = retry_clock;
        replacement.expires_at_seq = retry_clock + 100;
        let replacement = reopened.acquire_lease(replacement).unwrap();
        assert_eq!(
            reopened
                .commit_belief_assessment(&replacement, revision.clone(), view)
                .unwrap(),
            BeliefCommitRecoveryDisposition::ResumedCommit
        );
        assert_eq!(
            reopened.current_revision(&belief_key).unwrap(),
            Some(revision)
        );
    }

    #[test]
    fn readiness_public_one_call_surface_rejects_without_writes() {
        let store = BeliefStore::new(
            sled::Config::new()
                .temporary(true)
                .open()
                .expect("temporary belief database"),
        )
        .unwrap();
        let request = seed_readiness_request(&store);
        let revision = store
            .get_revision(&request.expected_revision_id)
            .unwrap()
            .unwrap();
        let view = store.current_view(&request.belief_key).unwrap().unwrap();
        let expected = BeliefReadinessAttestation::identified(&request, &revision, &view).unwrap();

        assert!(matches!(
            store.attest_current_view(&request),
            Err(StorageError::InvalidPath(message))
                if message.contains("exact agent hydration capability")
        ));
        for tree in [
            &store.readiness_attestation_intents,
            &store.readiness_attestation_owner_fences,
            &store.readiness_attestation_snapshots,
            &store.readiness_attestations,
        ] {
            assert!(tree
                .get(expected.attestation_id.as_bytes())
                .unwrap()
                .is_none());
        }
    }

    #[test]
    fn readiness_fenced_prepare_is_atomic_and_rejects_stale_owner() {
        let store = BeliefStore::new(
            sled::Config::new()
                .temporary(true)
                .open()
                .expect("temporary belief database"),
        )
        .unwrap();
        let request = seed_readiness_request(&store);
        let owner_hash = "b".repeat(64);
        let stale_owner_hash = "c".repeat(64);
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let first_store = store.clone();
        let first_request = request.clone();
        let first_owner_hash = owner_hash.clone();
        let first_barrier = Arc::clone(&barrier);
        let first = std::thread::spawn(move || {
            first_barrier.wait();
            first_store.prepare_readiness_attestation_fenced(&first_request, &first_owner_hash)
        });
        let second_store = store.clone();
        let second_request = request.clone();
        let second_owner_hash = stale_owner_hash.clone();
        let second_barrier = Arc::clone(&barrier);
        let second = std::thread::spawn(move || {
            second_barrier.wait();
            second_store.prepare_readiness_attestation_fenced(&second_request, &second_owner_hash)
        });
        barrier.wait();
        let first = first.join().unwrap();
        let second = second.join().unwrap();
        assert_ne!(first.is_ok(), second.is_ok());
        let winner = first.as_ref().or(second.as_ref()).unwrap();
        let winning_owner = if first.is_ok() {
            owner_hash.as_bytes()
        } else {
            stale_owner_hash.as_bytes()
        };
        assert_eq!(
            store
                .readiness_attestation_owner_fences
                .get(winner.attestation_id.as_bytes())
                .unwrap()
                .as_deref(),
            Some(winning_owner)
        );
        let loser = first.err().or_else(|| second.err()).unwrap();
        assert!(matches!(
            loser,
            StorageError::Backpressure(message) if message.contains("owner fence conflict")
        ));
    }

    #[test]
    fn readiness_current_owner_sentinel_fails_closed() {
        let store = BeliefStore::new(
            sled::Config::new()
                .temporary(true)
                .open()
                .expect("temporary belief database"),
        )
        .unwrap();
        let request = seed_readiness_request(&store);
        let owner_hash = "d".repeat(64);
        let attestation = store
            .prepare_readiness_attestation_fenced(&request, &owner_hash)
            .unwrap();
        store
            .readiness_attestation_owner_fences
            .insert(attestation.attestation_id.as_bytes(), b"legacy-unfenced")
            .unwrap();
        assert!(matches!(
            store.get_prepared_readiness_attestation(&attestation.attestation_id),
            Err(StorageError::MigrationConflict(message))
                if message.contains("no exact owner fence")
        ));
        assert!(matches!(
            store.prepare_readiness_attestation_fenced(&request, &owner_hash),
            Err(StorageError::Backpressure(message))
                if message.contains("owner fence conflict")
        ));
    }

    #[test]
    fn readiness_getters_reject_alias_keys_and_malformed_current_content() {
        let store = BeliefStore::new(
            sled::Config::new()
                .temporary(true)
                .open()
                .expect("temporary belief database"),
        )
        .unwrap();
        let request = seed_readiness_request(&store);
        let attestation = store
            .prepare_readiness_attestation_fenced(&request, &"f".repeat(64))
            .unwrap();
        let encoded = serde_json::to_vec(&attestation).unwrap();
        store
            .readiness_attestation_intents
            .insert(b"prepared-alias", encoded.clone())
            .unwrap();
        assert!(matches!(
            store.get_prepared_readiness_attestation("prepared-alias"),
            Err(StorageError::MigrationConflict(message))
                if message.contains("key conflicts with its identity")
        ));

        store
            .readiness_attestations
            .insert(attestation.attestation_id.as_bytes(), encoded.clone())
            .unwrap();
        store
            .readiness_attestations
            .insert(b"visible-alias", encoded)
            .unwrap();
        assert!(matches!(
            store.get_readiness_attestation("visible-alias"),
            Err(StorageError::MigrationConflict(message))
                if message.contains("key conflicts with its identity")
        ));

        let mut malformed = attestation;
        malformed.belief_view_hash = "malformed-view-hash".to_string();
        store
            .readiness_attestations
            .insert(
                malformed.attestation_id.as_bytes(),
                serde_json::to_vec(&malformed).unwrap(),
            )
            .unwrap();
        assert!(store
            .get_readiness_attestation(&malformed.attestation_id)
            .is_err());
    }

    #[test]
    fn readiness_snapshot_getter_rejects_alias_keys_and_malformed_content() {
        let store = BeliefStore::new(
            sled::Config::new()
                .temporary(true)
                .open()
                .expect("temporary belief database"),
        )
        .unwrap();
        let request = seed_readiness_request(&store);
        let attestation = store
            .prepare_readiness_attestation_fenced(&request, &"b".repeat(64))
            .unwrap();
        let snapshot = store
            .readiness_snapshot(&attestation.attestation_id)
            .unwrap()
            .unwrap();
        store
            .readiness_attestation_snapshots
            .insert(b"snapshot-alias", serde_json::to_vec(&snapshot).unwrap())
            .unwrap();
        fail_next_flush(&store);
        assert!(matches!(
            store.readiness_snapshot("snapshot-alias"),
            Err(StorageError::InvalidPath(message))
                if message.contains("key conflicts with embedded attestation identity")
        ));

        let mut malformed_snapshot = snapshot;
        malformed_snapshot.view.view_id = "malformed-view".to_string();
        store
            .readiness_attestation_snapshots
            .insert(
                attestation.attestation_id.as_bytes(),
                serde_json::to_vec(&malformed_snapshot).unwrap(),
            )
            .unwrap();
        assert!(store
            .readiness_snapshot(&attestation.attestation_id)
            .is_err());
        assert!(store.flush().is_err());
    }

    #[test]
    fn readiness_fenced_prepare_rejects_missing_current_owner_without_repair() {
        let store = BeliefStore::new(
            sled::Config::new()
                .temporary(true)
                .open()
                .expect("temporary belief database"),
        )
        .unwrap();
        let request = seed_readiness_request(&store);
        let owner_hash = "e".repeat(64);
        let attestation = store
            .prepare_readiness_attestation_fenced(&request, &owner_hash)
            .unwrap();
        store
            .readiness_attestation_owner_fences
            .remove(attestation.attestation_id.as_bytes())
            .unwrap();

        assert!(matches!(
            store.prepare_readiness_attestation_fenced(&request, &owner_hash),
            Err(StorageError::Backpressure(message))
                if message.contains("missing its owner fence")
        ));
        assert!(store
            .readiness_attestation_owner_fences
            .get(attestation.attestation_id.as_bytes())
            .unwrap()
            .is_none());
    }

    #[test]
    fn readiness_schema_migration_is_batched_once_and_complete_replay_is_flush_free() {
        let store = BeliefStore::new(
            sled::Config::new()
                .temporary(true)
                .open()
                .expect("temporary belief database"),
        )
        .unwrap();
        let request = seed_readiness_request(&store);
        let revision = store
            .get_revision(&request.expected_revision_id)
            .unwrap()
            .unwrap();
        let view = store.current_view(&request.belief_key).unwrap().unwrap();
        for index in 0..300_u64 {
            let mut request = request.clone();
            request.agent_id = format!("migration-agent-{index:03}");
            request.subscription_id = format!("migration-subscription-{index:03}");
            request.attested_at_seq = 10 + index;
            let attestation =
                BeliefReadinessAttestation::identified(&request, &revision, &view).unwrap();
            let snapshot = BeliefReadinessSnapshot {
                attestation: attestation.clone(),
                revision: revision.clone(),
                view: view.clone(),
            };
            let encoded = serde_json::to_vec(&attestation).unwrap();
            store
                .readiness_attestation_intents
                .insert(attestation.attestation_id.as_bytes(), encoded.clone())
                .unwrap();
            store
                .readiness_attestations
                .insert(attestation.attestation_id.as_bytes(), encoded)
                .unwrap();
            store
                .readiness_attestation_snapshots
                .insert(
                    attestation.attestation_id.as_bytes(),
                    serde_json::to_vec(&snapshot).unwrap(),
                )
                .unwrap();
            store
                .readiness_attestation_owner_fences
                .insert(
                    attestation.attestation_id.as_bytes(),
                    "a".repeat(64).as_bytes(),
                )
                .unwrap();
        }
        store
            .readiness_schema
            .remove(KEY_READINESS_SCHEMA_STATE)
            .unwrap();
        let before = flush_calls(&store);
        store.migrate_legacy_readiness_attestations().unwrap();
        let migrated = flush_calls(&store);
        assert_eq!(migrated - before, 9);
        let state: ReadinessMigrationState = serde_json::from_slice(
            &store
                .readiness_schema
                .get(KEY_READINESS_SCHEMA_STATE)
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(state.phase, ReadinessMigrationPhase::Complete);
        assert!(state.cursor.is_none());

        store.migrate_legacy_readiness_attestations().unwrap();
        assert_eq!(flush_calls(&store), migrated);
    }

    fn revision(key: &BeliefKey, revision_id: &str, end: u64) -> BeliefRevision {
        BeliefRevision {
            revision_id: revision_id.to_string(),
            belief_key: key.clone(),
            prior_revision_id: None,
            comparator_engine_id: "weighted_bayesian".to_string(),
            comparator_engine_version: "1".to_string(),
            config_snapshot_hash: "config-a".to_string(),
            evidence_ids: Vec::new(),
            supporting_evidence_ids: Vec::new(),
            contradicted_evidence_ids: Vec::new(),
            source_cursor_start: end,
            source_cursor_end: end,
            posterior: PosteriorSummary {
                probability: 0.5,
                meaning: "stale_probability".to_string(),
            },
            planner_projection: PlannerProjectionSummary {
                confidence_field: "confidence".to_string(),
                confidence: 0.5,
                threshold: 0.7,
            },
            uncertainty: 0.5,
            precision: 0.5,
            freshness: FreshnessState {
                stale: false,
                reasons: Vec::new(),
                high_water_seq: end,
            },
            contradiction: ContradictionState {
                contradicted: false,
                reasons: Vec::new(),
                supporting_evidence_ids: Vec::new(),
                contradicted_evidence_ids: Vec::new(),
            },
            status: BeliefStatus::Settled,
            observation: None,
            provenance: BeliefProvenanceSummary::empty(),
        }
    }

    fn evidence_and_assignment(id: &str, seq: u64) -> (EvidenceItem, EvidenceAssignment) {
        let item = EvidenceItem {
            evidence_id: format!("evidence-{id}"),
            candidate_key: key(),
            source_fact_ids: vec![format!("fact-{id}")],
            graph_anchor_ids: Vec::new(),
            source_cursor_start: seq,
            source_cursor_end: seq,
            role: EvidenceRole::Support,
            evidence_schema_id: "signal".to_string(),
            typed_value: EvidenceValue::Scalar(1.0),
            reliability: 1.0,
            precision: 1.0,
            reference_time: None,
            transaction_seq: seq,
            content_hash: None,
            provenance: BeliefProvenanceSummary::empty(),
        };
        let assignment = EvidenceAssignment {
            assignment_id: format!("assignment-{id}"),
            evidence_id: item.evidence_id.clone(),
            belief_key: item.candidate_key.clone(),
            role: item.role.clone(),
            source_cursor_start: seq,
            source_cursor_end: seq,
        };
        (item, assignment)
    }

    fn prepare_pre_progress_source(path: &std::path::Path) -> BeliefRevision {
        let store = BeliefStore::new(open_test_database(path)).unwrap();
        let (item, assignment) = evidence_and_assignment("legacy-cross-authority", 1);
        store.put_evidence_once(&item).unwrap();
        store.put_assignment_once(&assignment).unwrap();
        let lease = store.acquire_lease(queued_lease()).unwrap();
        let mut expected_revision = revision(&key(), "revision-legacy-cross-authority", 1);
        expected_revision.evidence_ids = vec![item.evidence_id];
        let view = store.project_view(
            &expected_revision,
            HydrationRefs {
                evidence_ids: expected_revision.evidence_ids.clone(),
                source_fact_ids: expected_revision.provenance.source_fact_ids.clone(),
                graph_anchor_ids: expected_revision.provenance.graph_anchor_ids.clone(),
                revision_id: Some(expected_revision.revision_id.clone()),
            },
        );
        store
            .commit_belief_assessment(&lease, expected_revision.clone(), view)
            .unwrap();

        let completed = store.get_lease("lease-a").unwrap().unwrap();
        let mut legacy_lease = serde_json::to_value(completed).unwrap();
        let legacy_object = legacy_lease.as_object_mut().unwrap();
        legacy_object.remove("assignment_cursor_start");
        legacy_object.remove("assignment_cursor_end");
        legacy_object.remove("assignment_window_complete");
        store
            .leases
            .insert(b"lease-a", serde_json::to_vec(&legacy_lease).unwrap())
            .unwrap();
        store.assignments_by_key.clear().unwrap();
        store.assignment_tail_by_key.clear().unwrap();
        store.committed_evidence.clear().unwrap();
        store.commit_receipts.clear().unwrap();
        store.commit_intent_by_lease.clear().unwrap();
        store.legacy_assessment_receipts.clear().unwrap();
        store
            .authority_meta
            .remove(KEY_LEGACY_ASSESSMENT_MIGRATION_FENCE)
            .unwrap();
        let assessment_keys = store
            .runtime_meta
            .scan_prefix(b"assessment_")
            .map(|item| item.unwrap().0)
            .collect::<Vec<_>>();
        for key in assessment_keys {
            store.runtime_meta.remove(key).unwrap();
        }
        store.flush().unwrap();
        expected_revision
    }

    fn record_count_before_assessment_payload(store: &BeliefStore) -> u64 {
        store
            .migrated_trees()
            .into_iter()
            .filter(|(tree_name, _)| *tree_name != TREE_LEGACY_ASSESSMENT_RECEIPTS)
            .map(|(_, tree)| u64::try_from(tree.len()).unwrap())
            .sum()
    }

    fn copy_records_before_assessment_payload(source: &BeliefStore, target: &BeliefStore) -> u64 {
        let mut copied = 0u64;
        for ((source_name, source_tree), (target_name, target_tree)) in source
            .migrated_trees()
            .into_iter()
            .zip(target.migrated_trees())
        {
            assert_eq!(source_name, target_name);
            if source_name == TREE_LEGACY_ASSESSMENT_RECEIPTS {
                break;
            }
            for item in source_tree.iter() {
                let (key, value) = item.unwrap();
                target_tree.insert(key, value).unwrap();
                copied = copied.saturating_add(1);
            }
        }
        copied
    }

    fn prepared_marker() -> BeliefAuthorityMigrationMarker {
        BeliefAuthorityMigrationMarker::try_new(
            migration_identity(),
            BeliefAuthorityMigrationProgress::Prepared {
                source: BeliefAuthoritySnapshot::try_new("a".repeat(64), 1).unwrap(),
            },
        )
        .unwrap()
    }

    #[test]
    fn independent_databases_have_distinct_migration_drain_gates() {
        let first = BeliefStore::new(
            sled::Config::new()
                .temporary(true)
                .open()
                .expect("first temporary belief database"),
        )
        .unwrap();
        let second = BeliefStore::new(
            sled::Config::new()
                .temporary(true)
                .open()
                .expect("second temporary belief database"),
        )
        .unwrap();

        assert!(!Arc::ptr_eq(&first.write_gate, &second.write_gate));
    }

    #[test]
    fn independent_handles_share_the_migration_drain_gate() {
        let source_temp = tempfile::tempdir().unwrap();
        let product_temp = tempfile::tempdir().unwrap();
        let db = sled::open(source_temp.path().join("belief")).unwrap();
        let source_a = BeliefStore::new(db.clone()).unwrap();
        let source_b = BeliefStore::new(db).unwrap();
        assert!(Arc::ptr_eq(&source_a.write_gate, &source_b.write_gate));
        source_a.put_runtime_meta("before", "1").unwrap();
        let admitted_writer = source_b.writable_guard().unwrap();
        let product =
            BeliefStore::new(sled::open(product_temp.path().join("belief")).unwrap()).unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let migration = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            product
                .advance_legacy_authority_migration(&source_a, migration_identity())
                .unwrap()
        });
        started_rx.recv().unwrap();
        source_b
            .runtime_meta
            .insert(b"admitted", b"before-fence")
            .unwrap();
        source_b.flush().unwrap();
        drop(admitted_writer);
        let marker = migration.join().unwrap();
        let frozen = match marker.progress() {
            BeliefAuthorityMigrationProgress::Prepared { source } => source,
            other => panic!("expected prepared marker, got {other:?}"),
        };
        assert_eq!(&source_b.authority_snapshot().unwrap(), frozen);
        assert!(source_b.put_runtime_meta("late", "after-fence").is_err());
    }

    #[test]
    fn staged_commit_drains_before_fence_and_recovers_from_every_marker() {
        for (checkpoint, additional_advances) in [
            ("prepared", 0),
            ("copying", 1),
            ("verified", 2),
            ("cutover", 3),
        ] {
            let source_temp = tempfile::tempdir().unwrap();
            let product_temp = tempfile::tempdir().unwrap();
            let source_path = source_temp
                .path()
                .join(format!("belief-source-{checkpoint}"));
            let product_path = product_temp
                .path()
                .join(format!("belief-product-{checkpoint}"));
            let source = BeliefStore::new(open_test_database(&source_path)).unwrap();
            source.mark_dirty(&key(), 1).unwrap();
            let lease = source.acquire_lease(queued_lease()).unwrap();
            let revision = revision(&key(), &format!("revision-{checkpoint}"), 1);
            let view = source.project_view(
                &revision,
                HydrationRefs {
                    evidence_ids: Vec::new(),
                    source_fact_ids: Vec::new(),
                    graph_anchor_ids: Vec::new(),
                    revision_id: Some(revision.revision_id.clone()),
                },
            );
            source.arm_commit_after_prepare_pause_for_test();
            source.arm_legacy_fence_attempt_for_test();
            let commit_store = source.clone();
            let commit_lease = lease.clone();
            let commit_revision = revision.clone();
            let commit_view = view.clone();
            let commit = std::thread::spawn(move || {
                commit_store.commit_belief_assessment(&commit_lease, commit_revision, commit_view)
            });
            source.wait_for_commit_prepare_for_test();
            assert_eq!(source.commit_intents.len(), 1);
            assert_eq!(source.commit_intent_by_lease.len(), 1);

            let product = BeliefStore::new(open_test_database(&product_path)).unwrap();
            let migration_product = product.clone();
            let migration_source = source.clone();
            let migration = std::thread::spawn(move || {
                migration_product
                    .advance_legacy_authority_migration(&migration_source, migration_identity())
            });
            source.wait_for_legacy_fence_attempt_for_test();
            assert!(source
                .authority_meta
                .get(KEY_LEGACY_WRITE_FENCE)
                .unwrap()
                .is_none());
            assert!(product.authority_migration_marker().unwrap().is_none());
            assert_eq!(source.commit_intents.len(), 1);

            source.release_commit_after_prepare_for_test();
            commit.join().unwrap().unwrap();
            let mut marker = migration.join().unwrap().unwrap();
            assert!(matches!(
                marker.progress(),
                BeliefAuthorityMigrationProgress::Prepared { .. }
            ));
            assert!(source.commit_intents.is_empty());
            assert!(source.commit_intent_by_lease.is_empty());
            assert!(source
                .commit_receipts
                .get(format!("belief-commit-{}", revision.revision_id).as_bytes())
                .unwrap()
                .is_some());

            for _ in 0..additional_advances {
                marker = product
                    .advance_legacy_authority_migration(&source, migration_identity())
                    .unwrap();
            }
            let actual_checkpoint = match marker.progress() {
                BeliefAuthorityMigrationProgress::Prepared { .. } => "prepared",
                BeliefAuthorityMigrationProgress::Copying { .. } => "copying",
                BeliefAuthorityMigrationProgress::Verified { .. } => "verified",
                BeliefAuthorityMigrationProgress::Cutover { .. } => "cutover",
                BeliefAuthorityMigrationProgress::ForwardRepairOnly { .. } => "forward-repair",
            };
            assert_eq!(actual_checkpoint, checkpoint);
            let frozen_source = match marker.progress() {
                BeliefAuthorityMigrationProgress::Prepared { source }
                | BeliefAuthorityMigrationProgress::Copying { source, .. } => source.clone(),
                BeliefAuthorityMigrationProgress::Verified { parity }
                | BeliefAuthorityMigrationProgress::Cutover { parity }
                | BeliefAuthorityMigrationProgress::ForwardRepairOnly { parity } => {
                    parity.source().clone()
                }
            };
            assert_eq!(source.authority_snapshot().unwrap(), frozen_source);

            let source_before_rejected_write = database_snapshot(&source.db);
            assert!(source
                .put_runtime_meta("post-fence-write", checkpoint)
                .is_err());
            assert_eq!(database_snapshot(&source.db), source_before_rejected_write);
            if checkpoint != "cutover" {
                let product_before_rejected_write = database_snapshot(&product.db);
                assert!(matches!(
                    product.put_runtime_meta("pre-cutover-write", checkpoint),
                    Err(StorageError::Unavailable(_))
                ));
                assert_eq!(
                    database_snapshot(&product.db),
                    product_before_rejected_write
                );
            }

            drop(product);
            drop(source);
            let reopened_source = BeliefStore::new(open_test_database(&source_path)).unwrap();
            assert_eq!(reopened_source.authority_snapshot().unwrap(), frozen_source);
            let product_db = open_test_database(&product_path);
            let product = if checkpoint == "cutover" {
                BeliefStore::new(product_db).unwrap()
            } else {
                let before_open = database_snapshot(&product_db);
                assert!(matches!(
                    BeliefStore::new(product_db.clone()),
                    Err(StorageError::Unavailable(_))
                ));
                assert_eq!(database_snapshot(&product_db), before_open);
                let recovery = BeliefStore::open_for_authority_migration(
                    product_db.clone(),
                    migration_identity(),
                )
                .unwrap();
                assert_eq!(
                    recovery.resume(&reopened_source).unwrap(),
                    LegacyBeliefCompatibilityPosture::ProductAuthoritative
                );
                drop(recovery);
                BeliefStore::new(product_db).unwrap()
            };
            assert!(matches!(
                product
                    .authority_migration_marker()
                    .unwrap()
                    .unwrap()
                    .progress(),
                BeliefAuthorityMigrationProgress::Cutover { .. }
            ));
            assert_eq!(product.authority_snapshot().unwrap(), frozen_source);
            assert_eq!(
                product.current_revision(&key()).unwrap(),
                Some(revision.clone())
            );
            assert_eq!(
                product
                    .exact_commit_progress_for_revision(&revision)
                    .unwrap(),
                Some(1)
            );
            assert!(product.commit_intents.is_empty());
            assert!(product.commit_intent_by_lease.is_empty());
        }
    }

    #[test]
    fn conflicting_staged_commit_rejects_fence_byte_clean() {
        let source_temp = tempfile::tempdir().unwrap();
        let product_temp = tempfile::tempdir().unwrap();
        let source = BeliefStore::new(open_test_database(
            &source_temp.path().join("belief-source-conflicting-intent"),
        ))
        .unwrap();
        source.mark_dirty(&key(), 1).unwrap();
        let lease = source.acquire_lease(queued_lease()).unwrap();
        let revision = revision(&key(), "revision-conflicting-intent", 1);
        let view = source.project_view(
            &revision,
            HydrationRefs {
                evidence_ids: Vec::new(),
                source_fact_ids: Vec::new(),
                graph_anchor_ids: Vec::new(),
                revision_id: Some(revision.revision_id.clone()),
            },
        );
        let mut completed = lease.clone();
        completed.status = LeaseStatus::Completed;
        let intent = BeliefCommitIntent::try_new(
            format!("belief-commit-{}", revision.revision_id),
            lease,
            completed,
            source.dirty_state(&key()).unwrap().unwrap(),
            revision,
            view,
        )
        .unwrap();
        source.prepare_belief_commit(&intent).unwrap();
        source
            .active_lease
            .insert(key().index_key().as_bytes(), b"conflicting-lease")
            .unwrap();
        source.flush().unwrap();

        let product = BeliefStore::new(open_test_database(
            &product_temp
                .path()
                .join("belief-product-conflicting-intent"),
        ))
        .unwrap();
        let source_before = database_snapshot(&source.db);
        let product_before = database_snapshot(&product.db);
        assert!(matches!(
            product.advance_legacy_authority_migration(&source, migration_identity()),
            Err(StorageError::InvalidPath(message))
                if message.contains("expected lease changed")
        ));
        assert_eq!(database_snapshot(&source.db), source_before);
        assert_eq!(database_snapshot(&product.db), product_before);
        assert!(source
            .authority_meta
            .get(KEY_LEGACY_WRITE_FENCE)
            .unwrap()
            .is_none());
        assert!(product.authority_migration_marker().unwrap().is_none());
        assert_eq!(source.commit_intents.len(), 1);
        assert_eq!(source.commit_intent_by_lease.len(), 1);

        source
            .authority_meta
            .insert(
                KEY_LEGACY_WRITE_FENCE,
                serde_json::to_vec(&migration_identity()).unwrap(),
            )
            .unwrap();
        source.flush().unwrap();
        let fenced_source_before = database_snapshot(&source.db);
        let fenced_product_before = database_snapshot(&product.db);
        assert!(matches!(
            product.advance_legacy_authority_migration(&source, migration_identity()),
            Err(StorageError::MigrationConflict(message))
                if message.contains("staged commits at its fence boundary")
        ));
        assert_eq!(database_snapshot(&source.db), fenced_source_before);
        assert_eq!(database_snapshot(&product.db), fenced_product_before);
        assert!(product.authority_migration_marker().unwrap().is_none());
    }

    #[test]
    fn target_constructor_rejects_incomplete_copy_then_opens_coherent_checkpoint() {
        let source_temp = tempfile::tempdir().unwrap();
        let product_temp = tempfile::tempdir().unwrap();
        let source_path = source_temp.path().join("belief-source-copy-gate");
        let source = BeliefStore::new(open_test_database(&source_path)).unwrap();
        let (item, assignment) = evidence_and_assignment("copy-gate", 1);
        source.put_evidence_once(&item).unwrap();
        source.put_assignment_once(&assignment).unwrap();

        let product_path = product_temp.path().join("belief-product-copy-gate");
        let product_db = open_test_database(&product_path);
        let product = BeliefStore::new(product_db.clone()).unwrap();
        product
            .advance_legacy_authority_migration(&source, migration_identity())
            .unwrap();
        product
            .advance_legacy_authority_migration(&source, migration_identity())
            .unwrap();
        product.arm_migration_copy_pause_for_test();
        let controller = product.clone();
        let migration_source = source.clone();
        let migration = std::thread::spawn(move || {
            product
                .advance_legacy_authority_migration(&migration_source, migration_identity())
                .unwrap()
        });
        controller.wait_for_migration_copy_gate_for_test();

        assert!(controller
            .get_assignment(&assignment.assignment_id)
            .unwrap()
            .is_none());
        assert!(matches!(
            controller
                .authority_migration_marker()
                .unwrap()
                .unwrap()
                .progress(),
            BeliefAuthorityMigrationProgress::Copying {
                verified_record_count: 0,
                ..
            }
        ));
        let before_open = database_snapshot(&product_db);
        assert!(matches!(
            BeliefStore::new(product_db.clone()),
            Err(StorageError::Unavailable(_))
        ));
        assert_eq!(database_snapshot(&product_db), before_open);

        controller.release_migration_copy_for_test();
        let marker = migration.join().unwrap();
        assert!(matches!(
            marker.progress(),
            BeliefAuthorityMigrationProgress::Verified { .. }
        ));
        let before_verified_open = database_snapshot(&product_db);
        assert!(matches!(
            BeliefStore::new(product_db.clone()),
            Err(StorageError::Unavailable(_))
        ));
        assert_eq!(database_snapshot(&product_db), before_verified_open);
        let recovery =
            BeliefStore::open_for_authority_migration(product_db.clone(), migration_identity())
                .unwrap();
        assert_eq!(
            recovery.resume(&source).unwrap(),
            LegacyBeliefCompatibilityPosture::ProductAuthoritative
        );
        drop(recovery);
        let opened = BeliefStore::new(product_db.clone()).unwrap();
        assert_eq!(
            opened.get_assignment(&assignment.assignment_id).unwrap(),
            Some(assignment.clone())
        );
        assert_eq!(
            opened.assignments_for_key(&key()).unwrap(),
            vec![assignment]
        );
        assert_eq!(opened.assignments_by_key.len(), 1);
        assert_eq!(opened.assignment_tail_by_key.len(), 1);
        assert_eq!(
            opened.authority_snapshot().unwrap(),
            source.authority_snapshot().unwrap()
        );

        drop(opened);
        drop(controller);
        drop(product_db);
        let reopened = BeliefStore::new(open_test_database(&product_path)).unwrap();
        assert_eq!(reopened.assignments_for_key(&key()).unwrap().len(), 1);
        assert_eq!(reopened.assignments_by_key.len(), 1);
        assert_eq!(reopened.assignment_tail_by_key.len(), 1);
        assert_eq!(
            reopened.authority_snapshot().unwrap(),
            source.authority_snapshot().unwrap()
        );
    }

    #[test]
    fn concurrent_handle_open_stress_is_exclusive_with_assignment_writers() {
        const ASSIGNMENT_COUNT: usize = 128;
        const OPEN_COUNT: usize = 128;
        let temp = tempfile::tempdir().unwrap();
        let db = sled::open(temp.path().join("belief")).unwrap();
        let writer_store = BeliefStore::new(db.clone()).unwrap();
        let opener_db = db.clone();
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let writer_barrier = Arc::clone(&barrier);
        let writer = std::thread::spawn(move || {
            writer_barrier.wait();
            for index in 0..ASSIGNMENT_COUNT {
                let (item, assignment) =
                    evidence_and_assignment(&format!("open-race-{index}"), index as u64 + 1);
                writer_store.put_evidence_once(&item).unwrap();
                writer_store.put_assignment_once(&assignment).unwrap();
            }
        });
        let opener_barrier = Arc::clone(&barrier);
        let opener = std::thread::spawn(move || {
            opener_barrier.wait();
            for _ in 0..OPEN_COUNT {
                BeliefStore::new(opener_db.clone()).unwrap();
            }
        });

        barrier.wait();
        writer.join().unwrap();
        opener.join().unwrap();

        let reopened = BeliefStore::new(db).unwrap();
        assert_eq!(
            reopened.assignments_for_key(&key()).unwrap().len(),
            ASSIGNMENT_COUNT
        );
        assert_eq!(reopened.assignments_by_key.len(), ASSIGNMENT_COUNT);
        assert_eq!(reopened.assignment_tail_by_key.len(), 1);
    }

    #[test]
    fn independent_authorities_get_unique_write_gates_under_parallel_open() {
        const STORE_COUNT: usize = 32;
        let barrier = Arc::new(std::sync::Barrier::new(STORE_COUNT));
        let handles = (0..STORE_COUNT)
            .map(|index| {
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    let temp = tempfile::tempdir().unwrap();
                    let store = BeliefStore::new(
                        sled::open(temp.path().join(format!("belief-{index}"))).unwrap(),
                    )
                    .unwrap();
                    barrier.wait();
                    (temp, store)
                })
            })
            .collect::<Vec<_>>();
        let stores = handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>();

        for left in 0..stores.len() {
            for right in left + 1..stores.len() {
                assert!(!Arc::ptr_eq(
                    &stores[left].1.write_gate,
                    &stores[right].1.write_gate
                ));
            }
        }
    }

    #[test]
    fn copied_database_metadata_does_not_alias_live_write_gates() {
        let temp = tempfile::tempdir().unwrap();
        let source_path = temp.path().join("source");
        let copied_path = temp.path().join("copied");
        let persisted_identity;
        {
            let source = BeliefStore::new(open_test_database(&source_path)).unwrap();
            source.put_runtime_meta("copy-proof", "present").unwrap();
            source.flush().unwrap();
            persisted_identity = source
                .authority_meta
                .get(KEY_STORE_INSTANCE_ID)
                .unwrap()
                .unwrap();
        }
        copy_directory(&source_path, &copied_path);

        let source_db = open_test_database(&source_path);
        let source = BeliefStore::new(source_db.clone()).unwrap();
        let source_peer = BeliefStore::new(source_db).unwrap();
        let copied = BeliefStore::new(open_test_database(&copied_path)).unwrap();
        assert_eq!(
            copied
                .authority_meta
                .get(KEY_STORE_INSTANCE_ID)
                .unwrap()
                .unwrap(),
            persisted_identity
        );
        assert!(Arc::ptr_eq(&source.write_gate, &source_peer.write_gate));
        assert!(!Arc::ptr_eq(&source.write_gate, &copied.write_gate));

        let held_source_fence = source.write_gate.write();
        let copied_gate = Arc::clone(&copied.write_gate);
        let concurrent = std::thread::spawn(move || copied_gate.try_write().is_some());
        assert!(concurrent.join().unwrap());
        drop(held_source_fence);
    }

    #[test]
    fn legacy_process_local_identities_are_scoped_to_live_database_authority() {
        let first_temp = tempfile::tempdir().unwrap();
        let second_temp = tempfile::tempdir().unwrap();
        let first_db = sled::open(first_temp.path().join("belief")).unwrap();
        let second_db = sled::open(second_temp.path().join("belief")).unwrap();
        for db in [&first_db, &second_db] {
            let meta = db.open_tree(TREE_AUTHORITY_META).unwrap();
            meta.insert(KEY_STORE_INSTANCE_ID, b"belief-store-0")
                .unwrap();
            db.flush().unwrap();
        }

        let first_peer = BeliefStore::new(first_db.clone()).unwrap();
        let first = BeliefStore::new(first_db).unwrap();
        let second = BeliefStore::new(second_db).unwrap();
        assert!(Arc::ptr_eq(&first.write_gate, &first_peer.write_gate));
        assert!(!Arc::ptr_eq(&first.write_gate, &second.write_gate));
        for store in [&first, &second] {
            assert_eq!(
                store
                    .authority_meta
                    .get(KEY_STORE_INSTANCE_ID)
                    .unwrap()
                    .unwrap()
                    .as_ref(),
                b"belief-store-0"
            );
        }
    }

    #[test]
    fn exact_fence_retry_flushes_before_target_preparation() {
        let source_temp = tempfile::tempdir().unwrap();
        let product_temp = tempfile::tempdir().unwrap();
        let source_path = source_temp.path().join("belief");
        let product_path = product_temp.path().join("belief");
        let source = BeliefStore::new(sled::open(&source_path).unwrap()).unwrap();
        source.put_runtime_meta("legacy", "1").unwrap();
        let product = BeliefStore::new(sled::open(&product_path).unwrap()).unwrap();
        let baseline_flushes = flush_calls(&source);
        fail_next_flush(&source);
        assert!(product
            .advance_legacy_authority_migration(&source, migration_identity())
            .is_err());
        assert_eq!(flush_calls(&source), baseline_flushes + 1);
        assert!(product.authority_migration_marker().unwrap().is_none());
        product
            .advance_legacy_authority_migration(&source, migration_identity())
            .unwrap();
        assert_eq!(flush_calls(&source), baseline_flushes + 3);
        drop(source);
        drop(product);
        let reopened_source = BeliefStore::new(sled::open(&source_path).unwrap()).unwrap();
        let product_db = sled::open(&product_path).unwrap();
        assert!(matches!(
            BeliefStore::new(product_db.clone()),
            Err(StorageError::Unavailable(_))
        ));
        let _recovery =
            BeliefStore::open_for_authority_migration(product_db, migration_identity()).unwrap();
        assert!(reopened_source.put_runtime_meta("late", "2").is_err());
    }

    #[test]
    fn exact_marker_retry_flushes_and_survives_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief");
        let store = BeliefStore::new(open_test_database(&path)).unwrap();
        let marker = prepared_marker();
        store
            .bind_product_authority(marker.migration().target_authority_id())
            .unwrap();
        let baseline_flushes = flush_calls(&store);
        fail_next_flush(&store);
        assert!(store.put_authority_migration_marker(&marker).is_err());
        assert_eq!(flush_calls(&store), baseline_flushes + 1);
        store.put_authority_migration_marker(&marker).unwrap();
        assert_eq!(flush_calls(&store), baseline_flushes + 2);
        drop(store);
        let database = open_test_database(&path);
        assert!(matches!(
            BeliefStore::new(database.clone()),
            Err(StorageError::Unavailable(_))
        ));
        BeliefStore::open_for_authority_migration(database.clone(), marker.migration().clone())
            .unwrap();
        let reopened_marker: BeliefAuthorityMigrationMarker = serde_json::from_slice(
            &database
                .open_tree(TREE_AUTHORITY_MIGRATION)
                .unwrap()
                .get(KEY_AUTHORITY_MIGRATION_MARKER)
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(reopened_marker, marker);
    }

    #[test]
    fn private_marker_writer_rejects_snapshot_replacement_and_regression() {
        let store = BeliefStore::new(sled::Config::new().temporary(true).open().unwrap()).unwrap();
        let prepared = prepared_marker();
        store
            .bind_product_authority(prepared.migration().target_authority_id())
            .unwrap();
        store.put_authority_migration_marker(&prepared).unwrap();

        let replacement = BeliefAuthorityMigrationMarker::try_new(
            migration_identity(),
            BeliefAuthorityMigrationProgress::Prepared {
                source: BeliefAuthoritySnapshot::try_new("b".repeat(64), 1).unwrap(),
            },
        )
        .unwrap();
        assert!(store.put_authority_migration_marker(&replacement).is_err());

        let source = match prepared.progress() {
            BeliefAuthorityMigrationProgress::Prepared { source } => source.clone(),
            _ => unreachable!(),
        };
        let copying = BeliefAuthorityMigrationMarker::try_new(
            migration_identity(),
            BeliefAuthorityMigrationProgress::Copying {
                source,
                verified_record_count: 0,
            },
        )
        .unwrap();
        store.put_authority_migration_marker(&copying).unwrap();
        assert!(store.put_authority_migration_marker(&prepared).is_err());
    }

    #[test]
    fn malformed_migration_marker_blocks_every_opener_without_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-malformed-migration-marker");
        let database = open_test_database(&path);
        let store = BeliefStore::new(database.clone()).unwrap();
        drop(store);
        database
            .open_tree(TREE_AUTHORITY_MIGRATION)
            .unwrap()
            .insert(KEY_AUTHORITY_MIGRATION_MARKER, b"not-json")
            .unwrap();
        database.flush().unwrap();
        let before_open = database_snapshot(&database);

        assert!(BeliefStore::new(database.clone()).is_err());
        assert_eq!(database_snapshot(&database), before_open);
        assert!(
            BeliefStore::open_for_authority_migration(database.clone(), migration_identity())
                .is_err()
        );
        assert_eq!(database_snapshot(&database), before_open);
    }

    #[test]
    fn exact_evidence_receipt_retry_flushes_and_survives_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief");
        let store = BeliefStore::new(open_test_database(&path)).unwrap();
        let ledger_id = "00000000-0000-0000-0000-000000000123".parse().unwrap();
        let cursor = EvidenceConsumerCursor {
            consumer_id: "consumer-a".to_string(),
            ledger_cursor: LedgerCursor {
                ledger_id,
                after_seq: 1,
            },
            family_config_hash: "family-a".to_string(),
            source_mapping_hash: "mapping-a".to_string(),
            perspective: PerspectiveKey::new("agent", "default").unwrap(),
            branch_scope: BranchScope::main(),
        };
        let receipt = EvidenceIngestionReceipt {
            identity: EvidenceIngestionReceiptIdentity {
                consumer_id: cursor.consumer_id.clone(),
                source_record: EventRecordRef { ledger_id, seq: 1 },
                family_config_hash: cursor.family_config_hash.clone(),
                source_mapping_hash: cursor.source_mapping_hash.clone(),
                perspective: cursor.perspective.clone(),
                branch_scope: cursor.branch_scope.clone(),
            },
            disposition: EvidenceIngestionReceiptDisposition::Irrelevant,
            evidence_ids: Vec::new(),
        };
        let baseline_flushes = flush_calls(&store);
        fail_next_flush(&store);
        assert!(matches!(
            store.record_evidence_receipt_and_advance(None, &receipt, &cursor),
            Err(StorageError::DurabilityIndeterminate(_))
        ));
        assert_eq!(flush_calls(&store), baseline_flushes + 1);
        assert_eq!(store.assessment_source_high_water().unwrap(), 1);
        assert_eq!(
            store
                .record_evidence_receipt_and_advance(None, &receipt, &cursor)
                .unwrap(),
            EvidenceIngestionReceiptWriteDisposition::ExactReplay
        );
        assert_eq!(flush_calls(&store), baseline_flushes + 2);
        assert_eq!(store.assessment_source_high_water().unwrap(), 1);
        drop(store);
        let reopened = BeliefStore::new(open_test_database(&path)).unwrap();
        assert_eq!(
            reopened.evidence_consumer_cursor(&cursor).unwrap(),
            Some(cursor)
        );
        assert_eq!(
            reopened.evidence_ingestion_receipt(&receipt).unwrap(),
            Some(receipt)
        );
        assert_eq!(reopened.assessment_source_high_water().unwrap(), 1);
    }

    #[test]
    fn public_dirty_and_assignment_writes_do_not_forge_acknowledged_source_high_water() {
        let temp = tempfile::tempdir().unwrap();
        let store = BeliefStore::new(open_test_database(&temp.path().join("belief"))).unwrap();
        store.mark_dirty(&key(), 999).unwrap();
        assert_eq!(store.assessment_source_high_water().unwrap(), 0);

        let (item, assignment) = evidence_and_assignment("unreceipted", 777);
        store.put_evidence_once(&item).unwrap();
        store.put_assignment_once(&assignment).unwrap();
        assert_eq!(store.assessment_source_high_water().unwrap(), 0);
    }

    #[test]
    fn reopen_repairs_acknowledged_high_water_only_from_exact_receipts() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-receipt-repair");
        let ledger_id = "00000000-0000-0000-0000-000000000123".parse().unwrap();
        let cursor = EvidenceConsumerCursor {
            consumer_id: "consumer-a".to_string(),
            ledger_cursor: LedgerCursor {
                ledger_id,
                after_seq: 1,
            },
            family_config_hash: "family-a".to_string(),
            source_mapping_hash: "mapping-a".to_string(),
            perspective: PerspectiveKey::new("agent", "default").unwrap(),
            branch_scope: BranchScope::main(),
        };
        let receipt = EvidenceIngestionReceipt {
            identity: EvidenceIngestionReceiptIdentity {
                consumer_id: cursor.consumer_id.clone(),
                source_record: EventRecordRef { ledger_id, seq: 1 },
                family_config_hash: cursor.family_config_hash.clone(),
                source_mapping_hash: cursor.source_mapping_hash.clone(),
                perspective: cursor.perspective.clone(),
                branch_scope: cursor.branch_scope.clone(),
            },
            disposition: EvidenceIngestionReceiptDisposition::Irrelevant,
            evidence_ids: Vec::new(),
        };
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            store
                .record_evidence_receipt_and_advance(None, &receipt, &cursor)
                .unwrap();
            store
                .runtime_meta
                .remove(KEY_ASSESSMENT_SOURCE_HIGH_WATER)
                .unwrap();
            store.flush().unwrap();
        }

        let reopened = BeliefStore::new(open_test_database(&path)).unwrap();
        assert_eq!(reopened.assessment_source_high_water().unwrap(), 1);
        assert_eq!(
            reopened.evidence_ingestion_receipt(&receipt).unwrap(),
            Some(receipt)
        );
        assert_eq!(
            reopened.evidence_consumer_cursor(&cursor).unwrap(),
            Some(cursor)
        );
    }

    #[test]
    fn exact_evidence_receipt_rejects_changed_next_without_authority_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let store = BeliefStore::new(open_test_database(&temp.path().join("belief"))).unwrap();
        let ledger_id = "00000000-0000-0000-0000-000000000123".parse().unwrap();
        let cursor = EvidenceConsumerCursor {
            consumer_id: "consumer-a".to_string(),
            ledger_cursor: LedgerCursor {
                ledger_id,
                after_seq: 1,
            },
            family_config_hash: "family-a".to_string(),
            source_mapping_hash: "mapping-a".to_string(),
            perspective: PerspectiveKey::new("agent", "default").unwrap(),
            branch_scope: BranchScope::main(),
        };
        let receipt = EvidenceIngestionReceipt {
            identity: EvidenceIngestionReceiptIdentity {
                consumer_id: cursor.consumer_id.clone(),
                source_record: EventRecordRef { ledger_id, seq: 1 },
                family_config_hash: cursor.family_config_hash.clone(),
                source_mapping_hash: cursor.source_mapping_hash.clone(),
                perspective: cursor.perspective.clone(),
                branch_scope: cursor.branch_scope.clone(),
            },
            disposition: EvidenceIngestionReceiptDisposition::Irrelevant,
            evidence_ids: Vec::new(),
        };
        store
            .record_evidence_receipt_and_advance(None, &receipt, &cursor)
            .unwrap();
        let mut forged_next = cursor.clone();
        forged_next.ledger_cursor.after_seq = 99;

        assert!(store
            .record_evidence_receipt_and_advance(None, &receipt, &forged_next)
            .is_err());
        assert_eq!(store.assessment_source_high_water().unwrap(), 1);
        assert_eq!(
            store.evidence_consumer_cursor(&cursor).unwrap(),
            Some(cursor)
        );
    }

    #[test]
    fn exact_prepared_commit_retry_flushes_and_recovers_on_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief");
        let store = BeliefStore::new(open_test_database(&path)).unwrap();
        let key = key();
        store.mark_dirty(&key, 1).unwrap();
        let leased = store.acquire_lease(queued_lease()).unwrap();
        let revision = revision(&key, "revision-flush", 1);
        let view = store.project_view(
            &revision,
            HydrationRefs {
                evidence_ids: Vec::new(),
                source_fact_ids: Vec::new(),
                graph_anchor_ids: Vec::new(),
                revision_id: Some(revision.revision_id.clone()),
            },
        );
        let mut completed = leased.clone();
        completed.status = LeaseStatus::Completed;
        let intent = BeliefCommitIntent::try_new(
            format!("belief-commit-{}", revision.revision_id),
            leased,
            completed,
            store.dirty_state(&key).unwrap().unwrap(),
            revision,
            view,
        )
        .unwrap();
        let baseline_flushes = flush_calls(&store);
        fail_next_flush(&store);
        assert!(store.prepare_belief_commit(&intent).is_err());
        assert_eq!(flush_calls(&store), baseline_flushes + 1);
        store.prepare_belief_commit(&intent).unwrap();
        assert_eq!(flush_calls(&store), baseline_flushes + 2);
        drop(store);
        let reopened = BeliefStore::new(open_test_database(&path)).unwrap();
        assert_eq!(
            reopened
                .current_revision(&key)
                .unwrap()
                .unwrap()
                .revision_id,
            "revision-flush"
        );
        assert_eq!(reopened.assessment_progress_sequence().unwrap(), 1);
        reopened.prepare_belief_commit(&intent).unwrap();
        assert_eq!(
            reopened.apply_belief_commit(intent.intent_id()).unwrap(),
            BeliefCommitRecoveryDisposition::AlreadyConsistent
        );
        assert_eq!(reopened.assessment_progress_sequence().unwrap(), 1);
    }

    #[test]
    fn terminal_commit_exact_retry_reflushes_after_final_apply_failure() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief-final-apply");
        let belief_key = key();
        let leased;
        let revision = revision(&belief_key, "revision-final-apply", 1);
        let view;
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            store.mark_dirty(&belief_key, 1).unwrap();
            leased = store.acquire_lease(queued_lease()).unwrap();
            view = store.project_view(
                &revision,
                HydrationRefs {
                    evidence_ids: Vec::new(),
                    source_fact_ids: Vec::new(),
                    graph_anchor_ids: Vec::new(),
                    revision_id: Some(revision.revision_id.clone()),
                },
            );
            let mut completed = leased.clone();
            completed.status = LeaseStatus::Completed;
            let intent = BeliefCommitIntent::try_new(
                format!("belief-commit-{}", revision.revision_id),
                leased.clone(),
                completed,
                store.dirty_state(&belief_key).unwrap().unwrap(),
                revision.clone(),
                view.clone(),
            )
            .unwrap();
            store.prepare_belief_commit(&intent).unwrap();
            let baseline_flushes = flush_calls(&store);
            fail_next_flush(&store);
            assert!(matches!(
                store.apply_belief_commit(intent.intent_id()),
                Err(StorageError::DurabilityIndeterminate(_))
            ));
            assert_eq!(flush_calls(&store), baseline_flushes + 1);
            assert!(store
                .active_lease
                .get(belief_key.index_key().as_bytes())
                .unwrap()
                .is_none());

            assert_eq!(
                store
                    .commit_belief_assessment(&leased, revision.clone(), view.clone())
                    .unwrap(),
                BeliefCommitRecoveryDisposition::AlreadyConsistent
            );
            assert_eq!(flush_calls(&store), baseline_flushes + 2);

            let mut conflicting_view = view.clone();
            conflicting_view.advisory_posture = "conflict".to_string();
            assert!(store
                .commit_belief_assessment(&leased, revision.clone(), conflicting_view)
                .is_err());
        }

        let reopened = BeliefStore::new(open_test_database(&path)).unwrap();
        assert_eq!(
            reopened
                .commit_belief_assessment(&leased, revision, view)
                .unwrap(),
            BeliefCommitRecoveryDisposition::AlreadyConsistent
        );
        assert_eq!(reopened.assessment_progress_sequence().unwrap(), 1);
    }

    #[test]
    fn concurrent_assignments_merge_dirty_state_without_lost_sequence() {
        let temp = tempfile::tempdir().unwrap();
        let db = sled::open(temp.path().join("belief")).unwrap();
        let first_store = BeliefStore::new(db.clone()).unwrap();
        let second_store = BeliefStore::new(db).unwrap();
        let (first_item, first_assignment) = evidence_and_assignment("a", 1);
        let (second_item, second_assignment) = evidence_and_assignment("b", 2);
        first_store.put_evidence_once(&first_item).unwrap();
        first_store.put_evidence_once(&second_item).unwrap();
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let first_barrier = barrier.clone();
        let first = std::thread::spawn(move || {
            first_barrier.wait();
            first_store.put_assignment_once(&first_assignment).unwrap();
        });
        let second_barrier = barrier.clone();
        let second = std::thread::spawn(move || {
            second_barrier.wait();
            second_store
                .put_assignment_once(&second_assignment)
                .unwrap();
        });
        barrier.wait();
        first.join().unwrap();
        second.join().unwrap();
        let reopened = BeliefStore::new(sled::open(temp.path().join("belief")).unwrap()).unwrap();
        let dirty = reopened.dirty_state(&key()).unwrap().unwrap();
        assert_eq!(dirty.dirty_since_seq, 1);
        assert_eq!(dirty.latest_seq, 2);
        let first_generation = reopened.assignment_generation("assignment-a").unwrap();
        let second_generation = reopened.assignment_generation("assignment-b").unwrap();
        assert_ne!(first_generation, second_generation);
        assert_eq!(
            dirty.mutation_generation,
            first_generation.max(second_generation)
        );
    }

    #[test]
    fn late_in_window_assignment_reschedules_without_synthetic_source_sequence() {
        let temp = tempfile::tempdir().unwrap();
        let store = BeliefStore::new(sled::open(temp.path().join("belief")).unwrap()).unwrap();
        let key = key();
        store.mark_dirty(&key, 1).unwrap();
        let mut legacy_dirty = store.dirty_state(&key).unwrap().unwrap();
        legacy_dirty.mutation_generation = 0;
        let legacy_bytes = serde_json::to_vec(&legacy_dirty).unwrap();
        assert!(serde_json::from_slice::<serde_json::Value>(&legacy_bytes)
            .unwrap()
            .get("mutation_generation")
            .is_none());
        store
            .dirty_keys
            .insert(key.index_key().as_bytes(), legacy_bytes)
            .unwrap();
        let leased = store.acquire_lease(queued_lease()).unwrap();
        assert_eq!(
            store
                .dirty_state(&key)
                .unwrap()
                .unwrap()
                .mutation_generation,
            1
        );
        assert!(store.evidence_for_key(&key).unwrap().is_empty());
        let (item, assignment) = evidence_and_assignment("late", 1);
        store.put_evidence_once(&item).unwrap();
        store.put_assignment_once(&assignment).unwrap();
        let dirty_after_assignment = store.dirty_state(&key).unwrap().unwrap();
        assert!(store
            .evidence_for_dirty_assessment(&key, &dirty_after_assignment, leased.epoch, 1, 1024,)
            .unwrap()
            .evidence
            .is_empty());
        let initial_revision = revision(&key, "revision-before-late", 1);
        let view = store.project_view(
            &initial_revision,
            HydrationRefs {
                evidence_ids: Vec::new(),
                source_fact_ids: Vec::new(),
                graph_anchor_ids: Vec::new(),
                revision_id: Some(initial_revision.revision_id.clone()),
            },
        );
        assert_eq!(
            store
                .commit_belief_assessment(&leased, initial_revision, view)
                .unwrap(),
            BeliefCommitRecoveryDisposition::RescheduledDirtyKey
        );
        let dirty = store.dirty_state(&key).unwrap().unwrap();
        assert_eq!(dirty.dirty_since_seq, 1);
        assert_eq!(dirty.latest_seq, 1);
        assert_eq!(dirty.mutation_generation, 2);
        assert!(dirty.active_lease_id.is_none());
        let pending = store
            .evidence_for_dirty_assessment(&key, &dirty, dirty.mutation_generation, 1, 1024)
            .unwrap();
        assert_eq!(pending.evidence.len(), 1);
        assert_eq!(pending.evidence[0].evidence_id, item.evidence_id);
        let mut next_lease = queued_lease();
        next_lease.lease_id = "lease-b".to_string();
        next_lease.epoch = dirty.mutation_generation;
        let next_lease = store.acquire_lease(next_lease).unwrap();
        let mut next_revision = revision(&key, "revision-with-late", 1);
        next_revision.prior_revision_id = Some("revision-before-late".to_string());
        next_revision.evidence_ids = vec![item.evidence_id.clone()];
        let next_view = store.project_view(
            &next_revision,
            HydrationRefs {
                evidence_ids: next_revision.evidence_ids.clone(),
                source_fact_ids: Vec::new(),
                graph_anchor_ids: Vec::new(),
                revision_id: Some(next_revision.revision_id.clone()),
            },
        );
        store
            .commit_belief_assessment(&next_lease, next_revision, next_view)
            .unwrap();
        assert!(store.dirty_state(&key).unwrap().is_none());
        assert_eq!(
            store
                .revision_history(&key)
                .unwrap()
                .into_iter()
                .flat_map(|revision| revision.evidence_ids)
                .filter(|evidence_id| evidence_id == &item.evidence_id)
                .count(),
            1
        );
    }

    #[test]
    fn prepared_commit_merges_newer_dirty_generation_on_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief");
        let key = key();
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            store.mark_dirty(&key, 1).unwrap();
            let leased = store.acquire_lease(queued_lease()).unwrap();
            let revision = revision(&key, "revision-recovered", 1);
            let view = store.project_view(
                &revision,
                HydrationRefs {
                    evidence_ids: Vec::new(),
                    source_fact_ids: Vec::new(),
                    graph_anchor_ids: Vec::new(),
                    revision_id: Some(revision.revision_id.clone()),
                },
            );
            let mut completed = leased.clone();
            completed.status = LeaseStatus::Completed;
            let intent = BeliefCommitIntent::try_new(
                format!("belief-commit-{}", revision.revision_id),
                leased,
                completed,
                store.dirty_state(&key).unwrap().unwrap(),
                revision,
                view,
            )
            .unwrap();
            store.prepare_belief_commit(&intent).unwrap();
            let (item, assignment) = evidence_and_assignment("after-prepare", 1);
            store.put_evidence_once(&item).unwrap();
            store.put_assignment_once(&assignment).unwrap();
            store.flush().unwrap();
        }
        let reopened = BeliefStore::new(open_test_database(&path)).unwrap();
        assert_eq!(
            reopened
                .current_revision(&key)
                .unwrap()
                .unwrap()
                .revision_id,
            "revision-recovered"
        );
        let dirty = reopened.dirty_state(&key).unwrap().unwrap();
        assert_eq!(dirty.latest_seq, 1);
        assert_eq!(dirty.mutation_generation, 2);
        assert!(dirty.active_lease_id.is_none());
    }

    #[test]
    fn coalesced_recovery_excludes_evidence_from_all_prior_revisions() {
        let temp = tempfile::tempdir().unwrap();
        let store = BeliefStore::new(sled::open(temp.path().join("belief")).unwrap()).unwrap();
        let key = key();
        for id in ["old-a", "old-b", "late-c"] {
            let (item, assignment) = evidence_and_assignment(id, 1);
            store.put_evidence_once(&item).unwrap();
            store.put_assignment_once(&assignment).unwrap();
        }
        let mut first = revision(&key, "revision-old-a", 1);
        first.evidence_ids = vec!["evidence-old-a".to_string()];
        let mut second = revision(&key, "revision-old-b", 1);
        second.prior_revision_id = Some(first.revision_id.clone());
        second.evidence_ids = vec!["evidence-old-b".to_string()];
        store
            .revisions
            .insert(
                first.revision_id.as_bytes(),
                serde_json::to_vec(&first).unwrap(),
            )
            .unwrap();
        store
            .revisions
            .insert(
                second.revision_id.as_bytes(),
                serde_json::to_vec(&second).unwrap(),
            )
            .unwrap();
        store.reconcile_assessment_indexes().unwrap();
        let dirty = store.dirty_state(&key).unwrap().unwrap();
        let pending = store
            .evidence_for_dirty_assessment(&key, &dirty, dirty.mutation_generation, 1, 1024)
            .unwrap();
        assert_eq!(pending.evidence.len(), 1);
        assert_eq!(pending.evidence[0].evidence_id, "evidence-late-c");
    }

    #[test]
    fn lease_dirty_transactions_abort_without_partial_acquire_or_abandon() {
        let temp = tempfile::tempdir().unwrap();
        let store = BeliefStore::new(sled::open(temp.path().join("belief")).unwrap()).unwrap();
        let key = key();
        store
            .dirty_keys
            .insert(key.index_key().as_bytes(), b"invalid-json")
            .unwrap();

        assert!(store.acquire_lease(queued_lease()).is_err());
        assert!(store.get_lease("lease-a").unwrap().is_none());
        assert!(store
            .active_lease
            .get(key.index_key().as_bytes())
            .unwrap()
            .is_none());

        let dirty = DirtyKeyState {
            belief_key: key.clone(),
            dirty_since_seq: 1,
            latest_seq: 1,
            mutation_generation: 1,
            assessment_cursor: None,
            active_lease_id: None,
            reason: DirtyReason::NewEvidence,
        };
        store
            .dirty_keys
            .insert(
                key.index_key().as_bytes(),
                serde_json::to_vec(&dirty).unwrap(),
            )
            .unwrap();
        let leased = store.acquire_lease(queued_lease()).unwrap();
        store
            .dirty_keys
            .insert(key.index_key().as_bytes(), b"invalid-json")
            .unwrap();
        let mut abandoned = leased.clone();
        abandoned.status = LeaseStatus::Abandoned;
        assert!(store
            .apply_assessment_lease_cas(&AssessmentLeaseCasIntent::AbandonExpired {
                expected_active_lease: leased.clone(),
                abandoned_lease: abandoned,
            })
            .is_err());
        assert_eq!(store.get_lease("lease-a").unwrap(), Some(leased.clone()));
        assert_eq!(
            store
                .active_lease
                .get(key.index_key().as_bytes())
                .unwrap()
                .as_deref(),
            Some(leased.lease_id.as_bytes())
        );
    }

    #[test]
    fn open_repair_restores_missing_subject_view_index() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief");
        let view;
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            let key = key();
            store.mark_dirty(&key, 1).unwrap();
            let lease = store.acquire_lease(queued_lease()).unwrap();
            let revision = revision(&key, "revision-a", 1);
            view = store.project_view(
                &revision,
                HydrationRefs {
                    evidence_ids: Vec::new(),
                    source_fact_ids: Vec::new(),
                    graph_anchor_ids: Vec::new(),
                    revision_id: Some(revision.revision_id.clone()),
                },
            );
            store
                .commit_belief_assessment(&lease, revision, view.clone())
                .unwrap();
            store
                .view_by_subject
                .remove(view_subject_index_key(&view).as_bytes())
                .unwrap();
            store.flush().unwrap();
        }
        let reopened = BeliefStore::new(open_test_database(&path)).unwrap();
        assert_eq!(
            reopened
                .view_by_subject
                .get(view_subject_index_key(&view).as_bytes())
                .unwrap()
                .as_deref(),
            Some(view.key.index_key().as_bytes())
        );
    }

    #[test]
    fn reopen_rebuilds_exact_bounded_assessment_indexes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief");
        let key = key();
        let assignment;
        let stored_revision;
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            let products = evidence_and_assignment("indexed", 7);
            assignment = products.1.clone();
            store.put_evidence_once(&products.0).unwrap();
            store.put_assignment_once(&assignment).unwrap();
            stored_revision = {
                let mut value = revision(&key, "revision-indexed", 7);
                value.evidence_ids = vec![products.0.evidence_id];
                value
            };
            store
                .revisions
                .insert(
                    stored_revision.revision_id.as_bytes(),
                    serde_json::to_vec(&stored_revision).unwrap(),
                )
                .unwrap();
            store.assignments_by_key.clear().unwrap();
            store.assignment_tail_by_key.clear().unwrap();
            store.committed_evidence.clear().unwrap();
            store.flush().unwrap();
        }

        let reopened = BeliefStore::new(open_test_database(&path)).unwrap();

        assert_eq!(
            reopened.assignments_for_key(&key).unwrap(),
            vec![assignment]
        );
        assert!(reopened
            .assignment_tail_by_key
            .get(key.index_key().as_bytes())
            .unwrap()
            .is_some());
        assert_eq!(
            reopened
                .committed_evidence
                .get(committed_evidence_index_key(
                    &key,
                    &stored_revision.evidence_ids[0]
                ))
                .unwrap()
                .as_deref(),
            Some(stored_revision.revision_id.as_bytes())
        );
    }

    #[test]
    fn reopen_rejects_divergent_bounded_assessment_indexes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief");
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            let (item, assignment) = evidence_and_assignment("corrupt-index", 1);
            store.put_evidence_once(&item).unwrap();
            store.put_assignment_once(&assignment).unwrap();
            store
                .assignments_by_key
                .insert(
                    assignment_by_key_index_key(
                        &assignment,
                        store
                            .assignment_generation(&assignment.assignment_id)
                            .unwrap(),
                    ),
                    b"other-assignment",
                )
                .unwrap();
            store.flush().unwrap();
        }

        assert!(matches!(
            BeliefStore::new(open_test_database(&path)),
            Err(StorageError::InvalidPath(message))
                if message == "belief assignment key index conflicts during reopen"
        ));
    }

    #[test]
    fn reopen_rejects_divergent_assignment_stream_tail() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief");
        {
            let store = BeliefStore::new(open_test_database(&path)).unwrap();
            let (item, assignment) = evidence_and_assignment("corrupt-tail", 1);
            store.put_evidence_once(&item).unwrap();
            store.put_assignment_once(&assignment).unwrap();
            store
                .assignment_tail_by_key
                .insert(
                    assignment.belief_key.index_key().as_bytes(),
                    b"foreign-tail",
                )
                .unwrap();
            store.flush().unwrap();
        }

        assert!(matches!(
            BeliefStore::new(open_test_database(&path)),
            Err(StorageError::InvalidPath(message))
                if message == "belief assignment stream tail conflicts during reopen"
        ));
    }

    #[test]
    fn corrupt_assessment_source_watermark_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let store = BeliefStore::new(sled::open(temp.path().join("belief")).unwrap()).unwrap();
        store
            .runtime_meta
            .insert(KEY_ASSESSMENT_SOURCE_HIGH_WATER, b"bad")
            .unwrap();

        assert!(matches!(
            store.assessment_source_high_water(),
            Err(StorageError::InvalidPath(message))
                if message == "belief assessment source watermark has invalid width"
        ));
    }

    #[test]
    fn corrupt_assessment_progress_sequence_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let store = BeliefStore::new(sled::open(temp.path().join("belief")).unwrap()).unwrap();
        store
            .runtime_meta
            .insert(KEY_ASSESSMENT_PROGRESS_SEQUENCE, b"bad")
            .unwrap();

        assert!(matches!(
            store.assessment_progress_sequence(),
            Err(StorageError::InvalidPath(message))
                if message == "belief assessment progress sequence has invalid width"
        ));
    }

    #[test]
    fn corrupt_assessment_lease_clock_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let store = BeliefStore::new(sled::open(temp.path().join("belief")).unwrap()).unwrap();
        store
            .runtime_meta
            .insert(KEY_ASSESSMENT_LEASE_CLOCK, b"bad")
            .unwrap();

        assert!(matches!(
            store.assessment_lease_clock(),
            Err(StorageError::InvalidPath(message))
                if message == "belief assessment lease clock has invalid width"
        ));
    }
}
