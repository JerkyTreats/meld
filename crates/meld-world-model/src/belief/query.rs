//! Read-only belief query facade.
//!
//! Planners and agents should use this module instead of raw belief store
//! trees. The query surface exposes views and provenance while hiding
//! comparator work state and lease internals.
//!
//! # Example
//!
//! ```rust,no_run
//! use meld_world_model::belief::{BeliefQuery, BeliefStore};
//!
//! let temp = tempfile::tempdir().unwrap();
//! let store = BeliefStore::new(sled::open(temp.path()).unwrap()).unwrap();
//! let query = BeliefQuery::new(&store);
//! let dirty = query.dirty_keys().unwrap();
//! assert!(dirty.is_empty());
//! ```

use crate::belief::contracts::{
    BeliefKey, BeliefProvenanceSummary, BeliefRevision, BeliefView, DirtyKeyState, EvidenceItem,
    ObservationOpportunity,
};
use crate::belief::store::BeliefStore;
use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::PerspectiveKey;

/// Planner-safe read facade over belief storage.
pub struct BeliefQuery<'a> {
    store: &'a BeliefStore,
}

impl<'a> BeliefQuery<'a> {
    /// Create a query facade over an existing store.
    pub fn new(store: &'a BeliefStore) -> Self {
        Self { store }
    }

    /// Read the current view for one belief key.
    pub fn current_view(&self, key: &BeliefKey) -> Result<Option<BeliefView>, StorageError> {
        self.store.current_view(key)
    }

    /// Read the current committed revision for one exact belief key.
    pub fn current_revision(
        &self,
        key: &BeliefKey,
    ) -> Result<Option<BeliefRevision>, StorageError> {
        self.store.current_revision(key)
    }

    /// Read the current revision and its planner-safe view for one exact key.
    ///
    /// The revision head is the durable authority: the returned view is
    /// rebuilt from that revision when the view cache is missing, and both
    /// halves always cite the same revision identity.
    pub fn current_revision_and_view(
        &self,
        key: &BeliefKey,
    ) -> Result<Option<(BeliefRevision, BeliefView)>, StorageError> {
        let Some(revision) = self.store.current_revision(key)? else {
            return Ok(None);
        };
        let view = match self.store.current_view(key)? {
            Some(view) if view.current_revision_id.as_deref() == Some(&revision.revision_id) => {
                view
            }
            // Cache miss or a stale cache entry: durable revision state wins.
            _ => self
                .store
                .rebuild_current_view_from_revision(key)?
                .ok_or_else(|| {
                    StorageError::InvalidPath(
                        "revision head vanished while rebuilding its view".to_string(),
                    )
                })?,
        };
        Ok(Some((revision, view)))
    }

    /// Read current views for one subject and perspective.
    pub fn current_views_for_subject(
        &self,
        subject: &DomainObjectRef,
        perspective: &PerspectiveKey,
    ) -> Result<Vec<BeliefView>, StorageError> {
        self.store.views_for_subject(subject, perspective)
    }

    /// Read committed revisions for one key in source order.
    pub fn revision_history(&self, key: &BeliefKey) -> Result<Vec<BeliefRevision>, StorageError> {
        self.store.revision_history(key)
    }

    /// Hydrate evidence used by a revision.
    pub fn evidence_by_revision(
        &self,
        revision_id: &str,
    ) -> Result<Vec<EvidenceItem>, StorageError> {
        self.store.evidence_by_revision(revision_id)
    }

    /// Read provenance for a revision without exposing raw payloads.
    pub fn provenance_by_revision(
        &self,
        revision_id: &str,
    ) -> Result<Option<BeliefProvenanceSummary>, StorageError> {
        self.store.provenance_by_revision(revision_id)
    }

    /// Read open observation opportunities for a subject and perspective.
    pub fn open_observation_opportunities(
        &self,
        subject: &DomainObjectRef,
        perspective: &PerspectiveKey,
    ) -> Result<Vec<ObservationOpportunity>, StorageError> {
        self.store
            .open_observation_opportunities(subject, perspective)
    }

    /// Read dirty key index entries for recovery and diagnostics.
    pub fn dirty_keys(&self) -> Result<Vec<String>, StorageError> {
        self.store.dirty_keys()
    }

    /// Read structured dirty key state for recovery and storm coalescing.
    pub fn dirty_key_states(&self) -> Result<Vec<DirtyKeyState>, StorageError> {
        self.store.dirty_key_states()
    }

    /// Rebuild the current view from durable revision state.
    pub fn rebuild_current_view_from_revision(
        &self,
        key: &BeliefKey,
    ) -> Result<Option<BeliefView>, StorageError> {
        self.store.rebuild_current_view_from_revision(key)
    }
}
