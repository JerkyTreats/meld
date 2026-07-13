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
use std::sync::{Arc, OnceLock, Weak};

use parking_lot::{Mutex, RwLock, RwLockReadGuard};
use sled::transaction::Transactional;
use sled::{Db, Tree};

use crate::belief::contracts::{
    AssessmentLease, AssessmentLeaseCasIntent, BeliefAuthorityMigrationIdentity,
    BeliefAuthorityMigrationMarker, BeliefAuthorityMigrationProgress, BeliefAuthorityParityReceipt,
    BeliefAuthoritySnapshot, BeliefCommitIntent, BeliefCommitRecoveryDisposition, BeliefKey,
    BeliefProvenanceSummary, BeliefRevision, BeliefStatus, BeliefView, DirtyKeyState, DirtyReason,
    EvidenceAssignment, EvidenceConsumerCursor, EvidenceIngestionReceipt,
    EvidenceIngestionReceiptWriteDisposition, EvidenceItem, EvidenceRejection, FreshnessReason,
    HydrationRefs, LeaseStatus, LegacyBeliefCompatibilityPosture, ObservationOpportunity,
    ObservationReason,
};
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::{PerspectiveKey, TraversalQuery};

const TREE_EVIDENCE: &str = "belief_evidence";
const TREE_ASSIGNMENTS: &str = "belief_assignments";
const TREE_ASSIGNMENT_GENERATIONS: &str = "belief_assignment_generations";
const TREE_REVISIONS: &str = "belief_revisions";
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
const KEY_PRODUCT_AUTHORITY_ID: &[u8] = b"product_authority_id";
const KEY_AUTHORITY_MIGRATION_MARKER: &[u8] = b"marker";
const KEY_LEGACY_WRITE_FENCE: &[u8] = b"legacy_write_fence";
const KEY_STORE_INSTANCE_ID: &[u8] = b"store_instance_id";

static WRITE_GATES: OnceLock<Mutex<BTreeMap<String, Weak<RwLock<()>>>>> = OnceLock::new();

#[cfg(test)]
#[derive(Default)]
struct FlushProbe {
    calls: usize,
    fail_next: bool,
}

/// Sled-backed store for belief evidence, revisions, views, and leases.
#[derive(Clone)]
pub struct BeliefStore {
    db: Db,
    evidence: Tree,
    assignments: Tree,
    assignment_generations: Tree,
    revisions: Tree,
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
    write_gate: Arc<RwLock<()>>,
    #[cfg(test)]
    flush_probe: Arc<Mutex<FlushProbe>>,
}

impl BeliefStore {
    /// Open all belief trees against a shared sled database.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        let authority_meta = db.open_tree(TREE_AUTHORITY_META).map_err(to_storage_io)?;
        let store_instance_id = load_or_create_store_instance_id(&db, &authority_meta)?;
        let write_gate = shared_write_gate(&store_instance_id);
        let store = Self {
            evidence: db.open_tree(TREE_EVIDENCE).map_err(to_storage_io)?,
            assignments: db.open_tree(TREE_ASSIGNMENTS).map_err(to_storage_io)?,
            assignment_generations: db
                .open_tree(TREE_ASSIGNMENT_GENERATIONS)
                .map_err(to_storage_io)?,
            revisions: db.open_tree(TREE_REVISIONS).map_err(to_storage_io)?,
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
            authority_migration: db
                .open_tree(TREE_AUTHORITY_MIGRATION)
                .map_err(to_storage_io)?,
            commit_intents: db.open_tree(TREE_COMMIT_INTENTS).map_err(to_storage_io)?,
            write_gate,
            #[cfg(test)]
            flush_probe: Arc::new(Mutex::new(FlushProbe::default())),
            db,
        };
        store.reconcile_open_commits()?;
        Ok(store)
    }

    /// Open the store behind an `Arc` for runtime assembly.
    pub fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        Ok(Arc::new(Self::new(db)?))
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
        Ok(guard)
    }

    fn fence_legacy_authority(
        &self,
        identity: &BeliefAuthorityMigrationIdentity,
    ) -> Result<BeliefAuthoritySnapshot, StorageError> {
        let _guard = self.write_gate.write();
        let encoded = serde_json::to_vec(identity).map_err(to_storage_data)?;
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
            Err(error) if error.current.as_deref() == Some(encoded.as_slice()) => self.flush()?,
            Err(_) => {
                return Err(StorageError::InvalidPath(
                    "legacy belief authority is fenced by another migration".to_string(),
                ));
            }
        }
        self.authority_snapshot()
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

    /// Persist and flush one migration marker checkpoint.
    pub fn put_authority_migration_marker(
        &self,
        marker: &BeliefAuthorityMigrationMarker,
    ) -> Result<(), StorageError> {
        let _write = self.writable_guard()?;
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
                BeliefAuthorityMigrationProgress::Cutover { parity } => {
                    return Ok(if parity.source().record_count() == 0 {
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
        self.bind_product_authority(identity.target_authority_id())?;
        let source = legacy.fence_legacy_authority(&identity)?;
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
        use sled::transaction::{ConflictableTransactionError, TransactionError};
        (
            &self.assignments,
            &self.assignment_generations,
            &self.dirty_keys,
            &self.active_lease,
            &self.leases,
        )
            .transaction(
                |(assignments, assignment_generations, dirty_keys, active_leases, leases)| {
                    if let Some(existing) = assignments.get(assignment.assignment_id.as_bytes())? {
                        if existing.as_ref() == assignment_bytes.as_slice() {
                            return Ok(false);
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
                    let dirty_since_seq = active
                        .as_ref()
                        .map(|lease| {
                            if assignment.source_cursor_end > lease.input_cursor_end {
                                assignment.source_cursor_end
                            } else {
                                lease.input_cursor_end.saturating_add(1)
                            }
                        })
                        .unwrap_or(assignment.source_cursor_end);
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
                    dirty_keys.insert(key.as_bytes(), state_bytes)?;
                    Ok(true)
                },
            )
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::InvalidPath(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })
    }

    /// Read assignments for one belief key in source order.
    pub fn assignments_for_key(
        &self,
        key: &BeliefKey,
    ) -> Result<Vec<EvidenceAssignment>, StorageError> {
        let mut out = Vec::new();
        for item in self.assignments.iter() {
            let (_, value) = item.map_err(to_storage_io)?;
            let assignment: EvidenceAssignment =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            if assignment.belief_key == *key {
                out.push(assignment);
            }
        }
        out.sort_by_key(|assignment| assignment.source_cursor_end);
        Ok(out)
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
        source_cursor_start: u64,
        source_cursor_end: u64,
    ) -> Result<Vec<EvidenceItem>, StorageError> {
        let committed_evidence_ids = self
            .revision_history(key)?
            .into_iter()
            .flat_map(|revision| revision.evidence_ids)
            .collect::<BTreeSet<_>>();
        let mut evidence = Vec::new();
        for assignment in self.assignments_for_key(key)? {
            let generation = match self
                .assignment_generations
                .get(assignment.assignment_id.as_bytes())
                .map_err(to_storage_io)?
            {
                Some(raw) if raw.len() == 8 => {
                    u64::from_be_bytes(raw.as_ref().try_into().map_err(|_| {
                        StorageError::InvalidPath(
                            "evidence assignment generation has invalid width".to_string(),
                        )
                    })?)
                }
                Some(_) => {
                    return Err(StorageError::InvalidPath(
                        "evidence assignment generation has invalid width".to_string(),
                    ));
                }
                None => 0,
            };
            if generation > admitted_generation {
                continue;
            }
            let Some(item) = self.get_evidence(&assignment.evidence_id)? else {
                continue;
            };
            let in_cursor_window = item.source_cursor_end >= source_cursor_start
                && item.source_cursor_end <= source_cursor_end;
            let admitted_coalesced = dirty.reason == DirtyReason::ActiveLeaseCoalesced
                && item.source_cursor_end <= source_cursor_end
                && !committed_evidence_ids.contains(&item.evidence_id);
            if (dirty.reason == DirtyReason::ActiveLeaseCoalesced && admitted_coalesced)
                || (dirty.reason != DirtyReason::ActiveLeaseCoalesced && in_cursor_window)
            {
                evidence.push(item);
            }
        }
        evidence.sort_by_key(|item| item.source_cursor_end);
        Ok(evidence)
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
        if let Some(existing) = self
            .evidence_ingestion_receipts
            .get(receipt_key.as_slice())
            .map_err(to_storage_io)?
        {
            return if existing.as_ref() == receipt_bytes.as_slice() {
                self.flush()?;
                Ok(EvidenceIngestionReceiptWriteDisposition::ExactReplay)
            } else {
                Err(StorageError::InvalidPath(
                    "evidence ingestion receipt conflict".to_string(),
                ))
            };
        }
        validate_receipt_cursor_transition(expected, receipt, next)?;
        let expected_bytes = expected
            .map(serde_json::to_vec)
            .transpose()
            .map_err(to_storage_data)?;
        let next_bytes = serde_json::to_vec(next).map_err(to_storage_data)?;

        use sled::transaction::{ConflictableTransactionError, TransactionError};
        let disposition = (
            &self.evidence_ingestion_receipts,
            &self.evidence_consumer_cursors,
        )
            .transaction(|(receipts, cursors)| {
                let stored_receipt = receipts.get(receipt_key.as_slice())?;
                let stored_cursor = cursors.get(cursor_key.as_slice())?;
                if stored_receipt.as_deref() == Some(receipt_bytes.as_slice()) {
                    return Ok(EvidenceIngestionReceiptWriteDisposition::ExactReplay);
                }
                if stored_receipt.is_some() {
                    return Err(ConflictableTransactionError::Abort(
                        "evidence ingestion receipt conflict".to_string(),
                    ));
                }
                if stored_cursor.as_deref() != expected_bytes.as_deref() {
                    return Err(ConflictableTransactionError::Abort(
                        "evidence consumer cursor changed".to_string(),
                    ));
                }
                receipts.insert(receipt_key.as_slice(), receipt_bytes.as_slice())?;
                cursors.insert(cursor_key.as_slice(), next_bytes.as_slice())?;
                Ok(EvidenceIngestionReceiptWriteDisposition::Inserted)
            })
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::InvalidPath(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.flush()?;
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

    /// Acquire the only active assessment lease for a belief key.
    pub fn acquire_lease(
        &self,
        mut lease: AssessmentLease,
    ) -> Result<AssessmentLease, StorageError> {
        lease.status = LeaseStatus::Leased;
        self.apply_assessment_lease_cas(&AssessmentLeaseCasIntent::Acquire {
            proposed_active_lease: lease,
        })
    }

    /// Atomically apply one validated assessment lease transition.
    pub fn apply_assessment_lease_cas(
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
        (&self.leases, &self.active_lease, &self.dirty_keys)
            .transaction(|(leases, active, dirty_keys)| {
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
                        dirty.active_lease_id = Some(proposed.lease_id.clone());
                    } else if matches!(intent, AssessmentLeaseCasIntent::AbandonExpired { .. }) {
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
            })
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::Backpressure(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        Ok(proposed.clone())
    }

    /// Store a lease record in its current lifecycle state.
    pub fn put_lease(&self, lease: &AssessmentLease) -> Result<(), StorageError> {
        let _write = self.writable_guard()?;
        self.leases
            .insert(
                lease.lease_id.as_bytes(),
                serde_json::to_vec(lease).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Read a lease by id.
    pub fn get_lease(&self, lease_id: &str) -> Result<Option<AssessmentLease>, StorageError> {
        decode_optional(
            self.leases
                .get(lease_id.as_bytes())
                .map_err(to_storage_io)?,
        )
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
    pub fn prepare_belief_commit(&self, intent: &BeliefCommitIntent) -> Result<(), StorageError> {
        let _write = self.writable_guard()?;
        let encoded = serde_json::to_vec(intent).map_err(to_storage_data)?;
        match self
            .commit_intents
            .compare_and_swap(
                intent.intent_id().as_bytes(),
                None as Option<&[u8]>,
                Some(encoded.as_slice()),
            )
            .map_err(to_storage_io)?
        {
            Ok(()) => self.flush(),
            Err(error) if error.current.as_deref() == Some(encoded.as_slice()) => self.flush(),
            Err(_) => Err(StorageError::InvalidPath(format!(
                "belief commit intent conflict for '{}'",
                intent.intent_id()
            ))),
        }
    }

    /// Stage and atomically apply one complete assessment result.
    pub fn commit_belief_assessment(
        &self,
        lease: &AssessmentLease,
        revision: BeliefRevision,
        public_view: BeliefView,
    ) -> Result<BeliefCommitRecoveryDisposition, StorageError> {
        self.validate_revision_commit(lease, &revision)?;
        let dirty = self.dirty_state(&revision.belief_key)?.ok_or_else(|| {
            StorageError::InvalidPath("belief commit requires durable dirty state".to_string())
        })?;
        let mut completed = lease.clone();
        completed.status = LeaseStatus::Completed;
        let intent = BeliefCommitIntent::try_new(
            format!("belief-commit-{}", revision.revision_id),
            lease.clone(),
            completed,
            dirty,
            revision,
            public_view,
        )?;
        self.prepare_belief_commit(&intent)?;
        self.apply_belief_commit(intent.intent_id())
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
    pub fn apply_belief_commit(
        &self,
        intent_id: &str,
    ) -> Result<BeliefCommitRecoveryDisposition, StorageError> {
        let _write = self.writable_guard()?;
        let Some(intent_raw) = self
            .commit_intents
            .get(intent_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(BeliefCommitRecoveryDisposition::AlreadyConsistent);
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

        use sled::transaction::{ConflictableTransactionError, TransactionError};
        let disposition = (
            &self.revisions,
            &self.revision_head,
            &self.views,
            &self.view_by_subject,
            &self.leases,
            &self.active_lease,
            &self.dirty_keys,
            &self.commit_intents,
        )
            .transaction(
                |(revisions, heads, views, subject_views, leases, active, dirty_keys, intents)| {
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
                        intents.remove(intent_id.as_bytes())?;
                        return Ok(BeliefCommitRecoveryDisposition::AlreadyConsistent);
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
                    {
                        return Err(ConflictableTransactionError::Abort(
                            "belief commit dirty state changed non-monotonically".to_string(),
                        ));
                    }
                    let changed_after_lease = current_dirty.mutation_generation > lease.epoch;
                    let remaining_dirty = if changed_after_lease
                        || current_dirty.latest_seq > lease.input_cursor_end
                    {
                        let mut remaining = current_dirty;
                        if !changed_after_lease {
                            remaining.dirty_since_seq = lease.input_cursor_end.saturating_add(1);
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
                    intents.remove(intent_id.as_bytes())?;
                    Ok(if remaining_dirty.is_some() {
                        BeliefCommitRecoveryDisposition::RescheduledDirtyKey
                    } else {
                        BeliefCommitRecoveryDisposition::ResumedCommit
                    })
                },
            )
            .map_err(|error| match error {
                TransactionError::Abort(message) => StorageError::InvalidPath(message),
                TransactionError::Storage(error) => to_storage_io(error),
            })?;
        self.flush()?;
        Ok(disposition)
    }

    /// Reconcile staged commits and missing current views from durable products.
    pub fn reconcile_open_commits(
        &self,
    ) -> Result<Vec<BeliefCommitRecoveryDisposition>, StorageError> {
        let mut dispositions = Vec::new();
        let intent_ids = self
            .commit_intents
            .iter()
            .map(|item| {
                let (key, _) = item.map_err(to_storage_io)?;
                String::from_utf8(key.to_vec()).map_err(to_storage_utf8)
            })
            .collect::<Result<Vec<_>, _>>()?;
        for intent_id in intent_ids {
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
        self.get_revision(&revision_id)
    }

    /// Read one revision by id.
    pub fn get_revision(&self, revision_id: &str) -> Result<Option<BeliefRevision>, StorageError> {
        decode_optional(
            self.revisions
                .get(revision_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Read all revisions for one key in source order.
    pub fn revision_history(&self, key: &BeliefKey) -> Result<Vec<BeliefRevision>, StorageError> {
        let mut out = Vec::new();
        for item in self.revisions.iter() {
            let (_, value) = item.map_err(to_storage_io)?;
            let revision: BeliefRevision =
                serde_json::from_slice(&value).map_err(to_storage_data)?;
            if revision.belief_key == *key {
                out.push(revision);
            }
        }
        out.sort_by_key(|revision| revision.source_cursor_end);
        Ok(out)
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
        let _write = self.writable_guard()?;
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

    /// Store small runtime metadata values.
    pub fn put_runtime_meta(&self, key: &str, value: &str) -> Result<(), StorageError> {
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
        ]
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
        let mut verified_record_count = 0u64;
        for ((source_name, source_tree), (target_name, target_tree)) in
            source_trees.into_iter().zip(target_trees)
        {
            if source_name != target_name {
                return Err(StorageError::InvalidPath(
                    "belief migration tree ordering changed".to_string(),
                ));
            }
            for item in source_tree.iter() {
                let (key, value) = item.map_err(to_storage_io)?;
                match target_tree
                    .compare_and_swap(&key, None as Option<&[u8]>, Some(value.as_ref()))
                    .map_err(to_storage_io)?
                {
                    Ok(()) => {}
                    Err(error) if error.current.as_deref() == Some(value.as_ref()) => {}
                    Err(_) => {
                        return Err(StorageError::InvalidPath(format!(
                            "belief migration target conflict in tree '{target_name}'"
                        )));
                    }
                }
                verified_record_count = verified_record_count.saturating_add(1);
                if verified_record_count <= starting_count {
                    continue;
                }
                let checkpoint = BeliefAuthorityMigrationMarker::try_new(
                    marker.migration().clone(),
                    BeliefAuthorityMigrationProgress::Copying {
                        source: source.clone(),
                        verified_record_count,
                    },
                )?;
                self.put_authority_migration_marker(&checkpoint)?;
            }
        }
        Ok(())
    }

    /// Flush the shared sled database.
    pub fn flush(&self) -> Result<(), StorageError> {
        #[cfg(test)]
        {
            let mut probe = self.flush_probe.lock();
            probe.calls = probe.calls.saturating_add(1);
            if probe.fail_next {
                probe.fail_next = false;
                return Err(StorageError::IoError(io::Error::other(
                    "injected indeterminate flush",
                )));
            }
        }
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }
}

fn load_or_create_store_instance_id(db: &Db, meta: &Tree) -> Result<String, StorageError> {
    if let Some(raw) = meta.get(KEY_STORE_INSTANCE_ID).map_err(to_storage_io)? {
        return String::from_utf8(raw.to_vec()).map_err(to_storage_utf8);
    }
    let candidate = format!("belief-store-{}", db.generate_id().map_err(to_storage_io)?);
    let identity = match meta
        .compare_and_swap(
            KEY_STORE_INSTANCE_ID,
            None as Option<&[u8]>,
            Some(candidate.as_bytes()),
        )
        .map_err(to_storage_io)?
    {
        Ok(()) => {
            db.flush().map_err(to_storage_io)?;
            candidate
        }
        Err(error) => String::from_utf8(
            error
                .current
                .ok_or_else(|| {
                    StorageError::InvalidPath(
                        "belief store identity initialization raced without a winner".to_string(),
                    )
                })?
                .to_vec(),
        )
        .map_err(to_storage_utf8)?,
    };
    Ok(identity)
}

fn shared_write_gate(identity: &str) -> Arc<RwLock<()>> {
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

fn decode_optional<T: serde::de::DeserializeOwned>(
    raw: Option<sled::IVec>,
) -> Result<Option<T>, StorageError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&raw).map_err(to_storage_data)?))
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
) -> DirtyKeyState {
    match existing {
        Some(existing) => DirtyKeyState {
            belief_key: key.clone(),
            dirty_since_seq: existing.dirty_since_seq.min(dirty_since_seq),
            latest_seq: existing.latest_seq.max(latest_seq),
            mutation_generation: existing.mutation_generation.saturating_add(1),
            active_lease_id: active_lease_id.or(existing.active_lease_id),
            reason,
        },
        None => DirtyKeyState {
            belief_key: key.clone(),
            dirty_since_seq,
            latest_seq,
            mutation_generation: 1,
            active_lease_id,
            reason,
        },
    }
}

fn evidence_receipt_key(receipt: &EvidenceIngestionReceipt) -> Result<Vec<u8>, StorageError> {
    serde_json::to_vec(&receipt.identity).map_err(to_storage_data)
}

fn validate_receipt_cursor_transition(
    expected: Option<&EvidenceConsumerCursor>,
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
    use crate::events::{EventRecordRef, LedgerCursor};

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
            started_at_seq: 1,
            expires_at_seq: 2,
            comparator_engine_id: "weighted_bayesian".to_string(),
            config_snapshot_hash: "config-a".to_string(),
            status: LeaseStatus::Queued,
        }
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
    fn exact_fence_retry_flushes_before_target_preparation() {
        let source_temp = tempfile::tempdir().unwrap();
        let product_temp = tempfile::tempdir().unwrap();
        let source_path = source_temp.path().join("belief");
        let product_path = product_temp.path().join("belief");
        let source = BeliefStore::new(sled::open(&source_path).unwrap()).unwrap();
        source.put_runtime_meta("legacy", "1").unwrap();
        let product = BeliefStore::new(sled::open(&product_path).unwrap()).unwrap();
        fail_next_flush(&source);
        assert!(product
            .advance_legacy_authority_migration(&source, migration_identity())
            .is_err());
        assert_eq!(flush_calls(&source), 1);
        assert!(product.authority_migration_marker().unwrap().is_none());
        product
            .advance_legacy_authority_migration(&source, migration_identity())
            .unwrap();
        assert_eq!(flush_calls(&source), 2);
        drop(source);
        drop(product);
        let reopened_source = BeliefStore::new(sled::open(&source_path).unwrap()).unwrap();
        let reopened_product = BeliefStore::new(sled::open(&product_path).unwrap()).unwrap();
        assert!(reopened_source.put_runtime_meta("late", "2").is_err());
        assert!(reopened_product
            .authority_migration_marker()
            .unwrap()
            .is_some());
    }

    #[test]
    fn exact_marker_retry_flushes_and_survives_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief");
        let store = BeliefStore::new(sled::open(&path).unwrap()).unwrap();
        let marker = prepared_marker();
        fail_next_flush(&store);
        assert!(store.put_authority_migration_marker(&marker).is_err());
        assert_eq!(flush_calls(&store), 1);
        store.put_authority_migration_marker(&marker).unwrap();
        assert_eq!(flush_calls(&store), 2);
        drop(store);
        let reopened = BeliefStore::new(sled::open(&path).unwrap()).unwrap();
        assert_eq!(reopened.authority_migration_marker().unwrap(), Some(marker));
    }

    #[test]
    fn exact_evidence_receipt_retry_flushes_and_survives_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief");
        let store = BeliefStore::new(sled::open(&path).unwrap()).unwrap();
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
        fail_next_flush(&store);
        assert!(store
            .record_evidence_receipt_and_advance(None, &receipt, &cursor)
            .is_err());
        assert_eq!(flush_calls(&store), 1);
        assert_eq!(
            store
                .record_evidence_receipt_and_advance(None, &receipt, &cursor)
                .unwrap(),
            EvidenceIngestionReceiptWriteDisposition::ExactReplay
        );
        assert_eq!(flush_calls(&store), 2);
        drop(store);
        let reopened = BeliefStore::new(sled::open(&path).unwrap()).unwrap();
        assert_eq!(
            reopened.evidence_consumer_cursor(&cursor).unwrap(),
            Some(cursor)
        );
        assert_eq!(
            reopened.evidence_ingestion_receipt(&receipt).unwrap(),
            Some(receipt)
        );
    }

    #[test]
    fn exact_prepared_commit_retry_flushes_and_recovers_on_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("belief");
        let store = BeliefStore::new(sled::open(&path).unwrap()).unwrap();
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
            "commit-flush",
            leased,
            completed,
            store.dirty_state(&key).unwrap().unwrap(),
            revision,
            view,
        )
        .unwrap();
        fail_next_flush(&store);
        assert!(store.prepare_belief_commit(&intent).is_err());
        assert_eq!(flush_calls(&store), 1);
        store.prepare_belief_commit(&intent).unwrap();
        assert_eq!(flush_calls(&store), 2);
        drop(store);
        let reopened = BeliefStore::new(sled::open(&path).unwrap()).unwrap();
        assert_eq!(
            reopened
                .current_revision(&key)
                .unwrap()
                .unwrap()
                .revision_id,
            "revision-flush"
        );
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
        assert_eq!(dirty.mutation_generation, 2);
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
            .evidence_for_dirty_assessment(&key, &dirty_after_assignment, leased.epoch, 1, 1,)
            .unwrap()
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
            .evidence_for_dirty_assessment(&key, &dirty, dirty.mutation_generation, 1, 1)
            .unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].evidence_id, item.evidence_id);
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
            let store = BeliefStore::new(sled::open(&path).unwrap()).unwrap();
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
                "commit-recovered",
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
        let reopened = BeliefStore::new(sled::open(&path).unwrap()).unwrap();
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
        let dirty = DirtyKeyState {
            belief_key: key.clone(),
            dirty_since_seq: 1,
            latest_seq: 1,
            mutation_generation: 3,
            active_lease_id: None,
            reason: DirtyReason::ActiveLeaseCoalesced,
        };
        let pending = store
            .evidence_for_dirty_assessment(&key, &dirty, dirty.mutation_generation, 1, 1)
            .unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].evidence_id, "evidence-late-c");
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
            let store = BeliefStore::new(sled::open(&path).unwrap()).unwrap();
            let key = key();
            let revision = BeliefRevision {
                revision_id: "revision-a".to_string(),
                belief_key: key.clone(),
                prior_revision_id: None,
                comparator_engine_id: "weighted_bayesian".to_string(),
                comparator_engine_version: "1".to_string(),
                config_snapshot_hash: "config-a".to_string(),
                evidence_ids: Vec::new(),
                supporting_evidence_ids: Vec::new(),
                contradicted_evidence_ids: Vec::new(),
                source_cursor_start: 1,
                source_cursor_end: 1,
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
                    high_water_seq: 1,
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
            };
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
                .revisions
                .insert(
                    revision.revision_id.as_bytes(),
                    serde_json::to_vec(&revision).unwrap(),
                )
                .unwrap();
            store
                .revision_head
                .insert(key.index_key().as_bytes(), revision.revision_id.as_bytes())
                .unwrap();
            store
                .views
                .insert(
                    key.index_key().as_bytes(),
                    serde_json::to_vec(&view).unwrap(),
                )
                .unwrap();
            store.flush().unwrap();
        }
        let reopened = BeliefStore::new(sled::open(&path).unwrap()).unwrap();
        assert_eq!(
            reopened
                .view_by_subject
                .get(view_subject_index_key(&view).as_bytes())
                .unwrap()
                .as_deref(),
            Some(view.key.index_key().as_bytes())
        );
    }
}
