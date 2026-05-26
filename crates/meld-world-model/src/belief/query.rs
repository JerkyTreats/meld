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
    BeliefKey, BeliefProvenanceSummary, BeliefRevision, BeliefView, EvidenceItem,
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
}
