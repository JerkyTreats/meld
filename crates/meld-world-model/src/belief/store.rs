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

use std::io;
use std::sync::Arc;

use sled::transaction::{ConflictableTransactionError, TransactionError, Transactional};
use sled::{Db, Tree};

use crate::belief::contracts::{
    AssessmentLease, BeliefKey, BeliefProvenanceSummary, BeliefRevision, BeliefStatus, BeliefView,
    DirtyKeyState, DirtyReason, EvidenceAssignment, EvidenceItem, EvidenceRejection,
    FreshnessReason, HydrationRefs, LeaseStatus, ObservationOpportunity, ObservationReason,
};
use crate::belief::subscription::SourceSubscriptionAcceptanceV1;
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::PerspectiveKey;

const TREE_EVIDENCE: &str = "belief_evidence";
const TREE_ASSIGNMENTS: &str = "belief_assignments";
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
const TREE_SUBSCRIPTION_ACCEPTANCES: &str = "belief_subscription_acceptances_v1";

fn subscription_prefix(
    kind: &str,
    revision: &crate::belief::TheoryRevisionRef,
    perspective: &PerspectiveKey,
    branch: &crate::belief::BranchScope,
) -> Result<String, StorageError> {
    let bytes = serde_json::to_vec(&(revision, perspective, branch)).map_err(to_storage_data)?;
    Ok(format!("{kind}::{}::", blake3::hash(&bytes).to_hex()))
}

fn subscription_key(
    kind: &str,
    revision: &crate::belief::TheoryRevisionRef,
    key: &BeliefKey,
) -> Result<String, StorageError> {
    Ok(format!(
        "{}{}",
        subscription_prefix(kind, revision, &key.perspective, &key.branch_scope)?,
        key.index_key()
    ))
}

fn subscription_transaction_error(error: TransactionError<String>) -> StorageError {
    match error {
        TransactionError::Abort(message) => StorageError::InvalidPath(message),
        TransactionError::Storage(error) => to_storage_io(error),
    }
}

fn dirty_scope_prefix(
    dimension: &str,
    predicate: &str,
    policy: &str,
    perspective: &PerspectiveKey,
    branch: &crate::belief::BranchScope,
) -> Result<String, StorageError> {
    let bytes = serde_json::to_vec(&(dimension, predicate, policy, perspective, branch))
        .map_err(to_storage_data)?;
    Ok(format!(
        "dirty-scope-v1::{}::",
        blake3::hash(&bytes).to_hex()
    ))
}

fn dirty_scope_key(key: &BeliefKey) -> Result<String, StorageError> {
    Ok(format!(
        "{}{}",
        dirty_scope_prefix(
            &key.dimension_id,
            &key.predicate_id,
            &key.evidence_policy_id,
            &key.perspective,
            &key.branch_scope
        )?,
        key.index_key()
    ))
}

/// Sled-backed store for belief evidence, revisions, views, and leases.
#[derive(Clone)]
pub struct BeliefStore {
    resource_id: String,
    db: Db,
    evidence: Tree,
    assignments: Tree,
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
    subscription_acceptances: Tree,
}

impl BeliefStore {
    /// Exact durable world-model resource bound to this store instance.
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    /// Open all belief trees against a shared sled database.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        let store = Self {
            resource_id: crate::waiting::resource_identity(&db)?,
            evidence: db.open_tree(TREE_EVIDENCE).map_err(to_storage_io)?,
            assignments: db.open_tree(TREE_ASSIGNMENTS).map_err(to_storage_io)?,
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
            subscription_acceptances: db
                .open_tree(TREE_SUBSCRIPTION_ACCEPTANCES)
                .map_err(to_storage_io)?,
            db,
        };
        store.index_existing_dirty_scopes()?;
        Ok(store)
    }

    fn index_existing_dirty_scopes(&self) -> Result<(), StorageError> {
        const MARKER: &[u8] = b"dirty-scope-index-v1";
        if self
            .runtime_meta
            .contains_key(MARKER)
            .map_err(to_storage_io)?
        {
            return Ok(());
        }
        for entry in self.dirty_keys.iter() {
            let (key, value) = entry.map_err(to_storage_io)?;
            let state = decode_dirty_state(&value)?;
            let index_key = dirty_scope_key(&state.belief_key)?;
            (&self.dirty_keys, &self.runtime_meta)
                .transaction(|(dirty, index)| {
                    if dirty.get(&key)?.is_some() {
                        index.insert(index_key.as_bytes(), key.as_ref())?;
                    }
                    Ok(())
                })
                .map_err(subscription_transaction_error)?;
        }
        self.runtime_meta
            .insert(MARKER, b"complete")
            .map_err(to_storage_io)?;
        self.flush()
    }

    pub(super) fn put_subscription_acceptance(
        &self,
        acceptance: &SourceSubscriptionAcceptanceV1,
        request: &crate::agent::AgentSubscriptionRequestV1,
    ) -> Result<bool, StorageError> {
        let bytes = serde_json::to_vec(acceptance).map_err(to_storage_data)?;
        request.validate()?;
        let body = serde_json::to_vec(request).map_err(to_storage_data)?;
        let key = &request.belief_key;
        let pending_key = subscription_key("pending", &request.source_contract_revision, key)?;
        let binding_key = subscription_key("binding", &request.source_contract_revision, key)?;
        let request_key = format!("request::{}", request.request_id);
        let changed = (
            &self.subscription_acceptances,
            &self.revision_head,
            &self.revisions,
        )
            .transaction(|(subscriptions, heads, revisions)| {
                let previous = subscriptions.get(acceptance.request_id.as_bytes())?;
                if previous
                    .as_ref()
                    .is_some_and(|previous| previous.as_ref() != bytes.as_slice())
                {
                    return Err(ConflictableTransactionError::Abort(
                        "Belief subscription acceptance conflicts with its request".to_string(),
                    ));
                }
                if subscriptions
                    .get(request_key.as_bytes())?
                    .as_ref()
                    .is_some_and(|previous| previous.as_ref() != body.as_slice())
                {
                    return Err(ConflictableTransactionError::Abort(
                        "Belief subscription body conflicts with its identity".to_string(),
                    ));
                }
                subscriptions.insert(acceptance.request_id.as_bytes(), bytes.as_slice())?;
                subscriptions.insert(request_key.as_bytes(), body.as_slice())?;
                if subscriptions.get(binding_key.as_bytes())?.is_none() {
                    subscriptions.insert(binding_key.as_bytes(), request.request_id.as_bytes())?;
                }
                let current = heads
                    .get(key.index_key().as_bytes())?
                    .map(|head| {
                        let raw = revisions.get(head)?.ok_or_else(|| {
                            ConflictableTransactionError::Abort(
                                "Belief head names an absent revision".to_string(),
                            )
                        })?;
                        serde_json::from_slice::<BeliefRevision>(&raw)
                            .map_err(|error| ConflictableTransactionError::Abort(error.to_string()))
                    })
                    .transpose()?;
                if current.as_ref().is_none_or(|current| {
                    current.theory_revision.as_ref() != Some(&request.source_contract_revision)
                }) && subscriptions.get(pending_key.as_bytes())?.is_none()
                {
                    subscriptions.insert(pending_key.as_bytes(), request.request_id.as_bytes())?;
                }
                Ok(previous.is_none())
            })
            .map_err(subscription_transaction_error)?;
        self.db.flush().map_err(to_storage_io)?;
        Ok(changed)
    }

    /// Exact accepted source request retained independently of Agent storage.
    pub fn accepted_subscription(
        &self,
        revision: &crate::belief::TheoryRevisionRef,
        key: &BeliefKey,
    ) -> Result<Option<crate::agent::AgentSubscriptionRequestV1>, StorageError> {
        let Some(id) = self
            .subscription_acceptances
            .get(subscription_key("binding", revision, key)?)
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        self.subscription_request(&id).map(Some)
    }

    fn subscription_request(
        &self,
        id: &[u8],
    ) -> Result<crate::agent::AgentSubscriptionRequestV1, StorageError> {
        let id = std::str::from_utf8(id)
            .map_err(|error| StorageError::InvalidPath(error.to_string()))?;
        let request: crate::agent::AgentSubscriptionRequestV1 = decode_optional(
            self.subscription_acceptances
                .get(format!("request::{id}"))
                .map_err(to_storage_io)?,
        )?
        .ok_or_else(|| {
            StorageError::InvalidPath("accepted subscription has no durable request".into())
        })?;
        request.validate()?;
        if request.request_id != id || self.subscription_acceptance(id)?.is_none() {
            return Err(StorageError::InvalidPath(
                "subscription index has no matching acceptance".into(),
            ));
        }
        Ok(request)
    }

    pub(super) fn pending_subscriptions(
        &self,
        revision: &crate::belief::TheoryRevisionRef,
        perspective: &PerspectiveKey,
        branch: &crate::belief::BranchScope,
        limit: usize,
    ) -> Result<(Vec<crate::agent::AgentSubscriptionRequestV1>, bool), StorageError> {
        self.subscription_requests("pending", revision, perspective, branch, limit)
    }

    pub(super) fn bound_subscriptions(
        &self,
        revision: &crate::belief::TheoryRevisionRef,
        perspective: &PerspectiveKey,
        branch: &crate::belief::BranchScope,
    ) -> Result<Vec<crate::agent::AgentSubscriptionRequestV1>, StorageError> {
        Ok(self
            .subscription_requests("binding", revision, perspective, branch, usize::MAX)?
            .0)
    }

    pub(super) fn subscription_for_index_key(
        &self,
        revision: &crate::belief::TheoryRevisionRef,
        perspective: &PerspectiveKey,
        branch: &crate::belief::BranchScope,
        key: &str,
    ) -> Result<Option<crate::agent::AgentSubscriptionRequestV1>, StorageError> {
        let index = format!(
            "{}{key}",
            subscription_prefix("binding", revision, perspective, branch)?
        );
        let Some(id) = self
            .subscription_acceptances
            .get(index)
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let request = self.subscription_request(&id)?;
        if request.belief_key.index_key() != key
            || request.belief_key.perspective != *perspective
            || request.belief_key.branch_scope != *branch
            || request.source_contract_revision != *revision
        {
            return Err(StorageError::InvalidPath(
                "subscription binding differs from its index".into(),
            ));
        }
        Ok(Some(request))
    }

    fn subscription_requests(
        &self,
        kind: &str,
        revision: &crate::belief::TheoryRevisionRef,
        perspective: &PerspectiveKey,
        branch: &crate::belief::BranchScope,
        limit: usize,
    ) -> Result<(Vec<crate::agent::AgentSubscriptionRequestV1>, bool), StorageError> {
        let prefix = subscription_prefix(kind, revision, perspective, branch)?;
        let mut values = Vec::new();
        let mut rows = self.subscription_acceptances.scan_prefix(prefix);
        for row in rows.by_ref().take(limit) {
            let (_, id) = row.map_err(to_storage_io)?;
            let request = self.subscription_request(&id)?;
            if request.source_contract_revision != *revision
                || request.belief_key.perspective != *perspective
                || request.belief_key.branch_scope != *branch
            {
                return Err(StorageError::InvalidPath(
                    "subscription index crosses its family or observation scope".into(),
                ));
            }
            values.push(request);
        }
        let more = rows.next().transpose().map_err(to_storage_io)?.is_some();
        Ok((values, more))
    }

    pub(super) fn subscription_acceptance(
        &self,
        request_id: &str,
    ) -> Result<Option<SourceSubscriptionAcceptanceV1>, StorageError> {
        decode_optional(
            self.subscription_acceptances
                .get(request_id.as_bytes())
                .map_err(to_storage_io)?,
        )
    }

    /// Open the store behind an `Arc` for runtime assembly.
    pub fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        Ok(Arc::new(Self::new(db)?))
    }

    /// Append or replace a deterministic evidence record by id.
    pub fn put_evidence(&self, item: &EvidenceItem) -> Result<(), StorageError> {
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
        self.assignments
            .insert(
                assignment.assignment_id.as_bytes(),
                serde_json::to_vec(assignment).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.mark_dirty_for_assignment(assignment)?;
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
        if let Some(existing) = self.get_assignment(&assignment.assignment_id)? {
            if existing == *assignment {
                return Ok(false);
            }
            return Err(StorageError::InvalidPath(format!(
                "evidence assignment conflict for '{}'",
                assignment.assignment_id
            )));
        }

        self.assignments
            .insert(
                assignment.assignment_id.as_bytes(),
                serde_json::to_vec(assignment).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.mark_dirty_for_assignment(assignment)?;
        Ok(true)
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

    /// Store an audit record for unsupported evidence candidates.
    pub fn put_rejection(&self, rejection: &EvidenceRejection) -> Result<(), StorageError> {
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

    /// Store the serialized config snapshot used for replay.
    pub fn put_config_snapshot(&self, hash: &str, json: &str) -> Result<(), StorageError> {
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
        let key = lease.belief_key.index_key();
        if let Some(active_id) = self
            .active_lease
            .get(key.as_bytes())
            .map_err(to_storage_io)?
            .map(|raw| String::from_utf8(raw.to_vec()).map_err(to_storage_utf8))
            .transpose()?
        {
            if let Some(active) = self.get_lease(&active_id)? {
                if active.status == LeaseStatus::Leased {
                    return Err(StorageError::Backpressure(format!(
                        "active belief lease '{}'",
                        active.lease_id
                    )));
                }
            }
        }
        lease.status = LeaseStatus::Leased;
        self.put_lease(&lease)?;
        self.active_lease
            .insert(key.as_bytes(), lease.lease_id.as_bytes())
            .map_err(to_storage_io)?;
        Ok(lease)
    }

    /// Store a lease record in its current lifecycle state.
    pub fn put_lease(&self, lease: &AssessmentLease) -> Result<(), StorageError> {
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
        self.put_lease(&completed)?;
        self.active_lease
            .remove(lease.belief_key.index_key().as_bytes())
            .map_err(to_storage_io)?;
        Ok(())
    }

    /// Abandon expired leases and mark their keys dirty for recovery.
    ///
    /// Recovery iterates the active-lease index, which holds exactly the
    /// in-flight keys and shrinks on completion and abandonment. The
    /// append-forever leases tree stays an audit record and is never scanned,
    /// so recovery cost is bounded by concurrent work, not history.
    pub fn recover_expired_leases(
        &self,
        current_seq: u64,
    ) -> Result<Vec<AssessmentLease>, StorageError> {
        let mut recovered = Vec::new();
        for item in self.active_lease.iter() {
            let (index_key, lease_id) = item.map_err(to_storage_io)?;
            let lease_id = String::from_utf8(lease_id.to_vec()).map_err(to_storage_utf8)?;
            // A missing or non-Leased record is a stale index entry left by a
            // failed acquisition path; expiry recovery only owns Leased work.
            let Some(mut lease) = self.get_lease(&lease_id)? else {
                continue;
            };
            if lease.status != LeaseStatus::Leased || lease.expires_at_seq > current_seq {
                continue;
            }
            lease.status = LeaseStatus::Abandoned;
            self.put_lease(&lease)?;
            self.active_lease.remove(index_key).map_err(to_storage_io)?;
            self.mark_dirty_with_reason(
                &lease.belief_key,
                lease.input_cursor_end,
                lease.input_cursor_end,
                Some(lease.lease_id.clone()),
                DirtyReason::LeaseExpired,
            )?;
            recovered.push(lease);
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
        let active_id = self
            .active_lease
            .get(revision.belief_key.index_key().as_bytes())
            .map_err(to_storage_io)?
            .ok_or_else(|| StorageError::InvalidPath("missing active lease".to_string()))?;
        let active_id = String::from_utf8(active_id.to_vec()).map_err(to_storage_utf8)?;
        if active_id != lease.lease_id {
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
        self.revisions
            .insert(
                revision.revision_id.as_bytes(),
                serde_json::to_vec(revision).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        let pending_key = revision
            .theory_revision
            .as_ref()
            .map(|reference| subscription_key("pending", reference, &revision.belief_key))
            .transpose()?;
        (&self.revision_head, &self.subscription_acceptances)
            .transaction(|(heads, subscriptions)| {
                heads.insert(
                    revision.belief_key.index_key().as_bytes(),
                    revision.revision_id.as_bytes(),
                )?;
                if let Some(key) = &pending_key {
                    subscriptions.remove(key.as_bytes())?;
                }
                Ok(())
            })
            .map_err(subscription_transaction_error)?;
        match self.dirty_state(&revision.belief_key)? {
            Some(mut dirty) if dirty.latest_seq > lease.input_cursor_end => {
                dirty.dirty_since_seq = lease.input_cursor_end.saturating_add(1);
                dirty.active_lease_id = None;
                self.put_dirty_state(&dirty)?;
            }
            _ => self.clear_dirty(&revision.belief_key)?,
        }
        Ok(())
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
        self.views
            .insert(
                view.key.index_key().as_bytes(),
                serde_json::to_vec(view).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.view_by_subject
            .insert(
                format!(
                    "{}::{}::{}",
                    view.key.subject.index_key(),
                    view.key.perspective.index_key(),
                    view.key.index_key()
                )
                .as_bytes(),
                view.key.index_key().as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
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

    /// Recompute current view freshness from admitted evidence, config, and policy state.
    pub fn refresh_view_freshness(
        &self,
        key: &BeliefKey,
        active_config_hash: &str,
        active_policy_id: &str,
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
        let index_key = dirty_scope_key(key)?;
        (&self.dirty_keys, &self.runtime_meta)
            .transaction(|(dirty, index)| {
                dirty.remove(key.index_key().as_bytes())?;
                index.remove(index_key.as_bytes())?;
                Ok(())
            })
            .map_err(subscription_transaction_error)?;
        Ok(())
    }

    /// Read only dirty work belonging to one configured family and complete scope.
    pub fn scoped_dirty_key_states_bounded(
        &self,
        family: &crate::belief::BeliefFamilyRevision,
        perspective: &PerspectiveKey,
        branch: &crate::belief::BranchScope,
        max_items: usize,
    ) -> Result<(Vec<DirtyKeyState>, bool), StorageError> {
        let prefix = dirty_scope_prefix(
            &family.config.dimension_id,
            &family.config.predicate_id,
            &family.config.evidence_policy_id,
            perspective,
            branch,
        )?;
        let mut out = Vec::new();
        for entry in self.runtime_meta.scan_prefix(prefix.as_bytes()) {
            let (index_key, key) = entry.map_err(to_storage_io)?;
            let Some(raw) = self.dirty_keys.get(&key).map_err(to_storage_io)? else {
                continue;
            };
            if out.len() == max_items {
                return Ok((out, true));
            }
            let state = decode_dirty_state(&raw)?;
            if dirty_scope_key(&state.belief_key)?.as_bytes() != index_key.as_ref()
                || state.belief_key.index_key().as_bytes() != key.as_ref()
            {
                return Err(StorageError::InvalidPath(
                    "dirty scope index differs from its native record".into(),
                ));
            }
            out.push(state);
        }
        Ok((out, false))
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

    /// Read up to `max_items` durable dirty records plus a remainder flag.
    ///
    /// The dirty tree is keyed by [`BeliefKey::index_key`], so iterating the
    /// index yields deterministic key order without decoding the whole tree.
    pub fn dirty_key_states_bounded(
        &self,
        max_items: usize,
    ) -> Result<(Vec<DirtyKeyState>, bool), StorageError> {
        let mut out = Vec::new();
        let mut more_available = false;
        for item in self.dirty_keys.iter() {
            let (_, value) = item.map_err(to_storage_io)?;
            if out.len() == max_items {
                more_available = true;
                break;
            }
            out.push(decode_dirty_state(&value)?);
        }
        Ok((out, more_available))
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

    fn mark_dirty_for_assignment(
        &self,
        assignment: &EvidenceAssignment,
    ) -> Result<(), StorageError> {
        if let Some(active) = self.active_lease_for_key(&assignment.belief_key)? {
            let dirty_since = if assignment.source_cursor_end > active.input_cursor_end {
                assignment.source_cursor_end
            } else {
                active.input_cursor_end.saturating_add(1)
            };
            self.mark_dirty_with_reason(
                &assignment.belief_key,
                dirty_since,
                assignment.source_cursor_end,
                Some(active.lease_id),
                DirtyReason::ActiveLeaseCoalesced,
            )
        } else {
            self.mark_dirty_with_reason(
                &assignment.belief_key,
                assignment.source_cursor_end,
                assignment.source_cursor_end,
                None,
                DirtyReason::NewEvidence,
            )
        }
    }

    fn mark_dirty_with_reason(
        &self,
        key: &BeliefKey,
        dirty_since_seq: u64,
        latest_seq: u64,
        active_lease_id: Option<String>,
        reason: DirtyReason,
    ) -> Result<(), StorageError> {
        let state = match self.dirty_state(key)? {
            Some(existing) => DirtyKeyState {
                belief_key: key.clone(),
                dirty_since_seq: existing.dirty_since_seq.min(dirty_since_seq),
                latest_seq: existing.latest_seq.max(latest_seq),
                active_lease_id: active_lease_id.or(existing.active_lease_id),
                reason,
            },
            None => DirtyKeyState {
                belief_key: key.clone(),
                dirty_since_seq,
                latest_seq,
                active_lease_id,
                reason,
            },
        };
        self.put_dirty_state(&state)
    }

    fn put_dirty_state(&self, state: &DirtyKeyState) -> Result<(), StorageError> {
        let index_key = dirty_scope_key(&state.belief_key)?;
        let key = state.belief_key.index_key();
        let body = serde_json::to_vec(state).map_err(to_storage_data)?;
        (&self.dirty_keys, &self.runtime_meta)
            .transaction(|(dirty, index)| {
                dirty.insert(key.as_bytes(), body.as_slice())?;
                index.insert(index_key.as_bytes(), key.as_bytes())?;
                Ok(())
            })
            .map_err(subscription_transaction_error)?;
        Ok(())
    }

    pub(super) fn active_lease_for_key(
        &self,
        key: &BeliefKey,
    ) -> Result<Option<AssessmentLease>, StorageError> {
        let Some(active_id) = self
            .active_lease
            .get(key.index_key().as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        let active_id = String::from_utf8(active_id.to_vec()).map_err(to_storage_utf8)?;
        Ok(self
            .get_lease(&active_id)?
            .filter(|lease| lease.status == LeaseStatus::Leased))
    }

    /// Store small runtime metadata values.
    pub fn put_runtime_meta(&self, key: &str, value: &str) -> Result<(), StorageError> {
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
            theory_revision: revision.theory_revision.clone(),
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

    /// Flush the shared sled database.
    pub fn flush(&self) -> Result<(), StorageError> {
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }
}

fn decode_optional<T: serde::de::DeserializeOwned>(
    raw: Option<sled::IVec>,
) -> Result<Option<T>, StorageError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(&raw).map_err(to_storage_data)?))
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
