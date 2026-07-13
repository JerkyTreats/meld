//! Explicit query boundary from belief into graph-owned state.

use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::graph::store::TraversalStore;
use crate::world_state::graph::{AnchorProvenanceRecord, AnchorSelectionRecord};

/// Read-only graph behavior required by belief assessment.
pub trait BeliefGraphQuery: Send + Sync {
    /// Read the current anchor for one subject and perspective.
    fn current_anchor_for_subject(
        &self,
        subject: &DomainObjectRef,
        perspective_kind: &str,
        perspective_id: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError>;

    /// Read compact provenance for one selected anchor.
    fn provenance_for_anchor(
        &self,
        anchor_id: &str,
    ) -> Result<AnchorProvenanceRecord, StorageError>;
}

impl BeliefGraphQuery for TraversalStore {
    fn current_anchor_for_subject(
        &self,
        subject: &DomainObjectRef,
        perspective_kind: &str,
        perspective_id: &str,
    ) -> Result<Option<AnchorSelectionRecord>, StorageError> {
        TraversalStore::current_anchor_for_subject(self, subject, perspective_kind, perspective_id)
    }

    fn provenance_for_anchor(
        &self,
        anchor_id: &str,
    ) -> Result<AnchorProvenanceRecord, StorageError> {
        self.anchor_provenance(anchor_id)
    }
}
