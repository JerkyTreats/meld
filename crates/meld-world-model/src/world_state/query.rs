//! Read-only query facade for legacy claim state.
//!
//! Callers should use this facade instead of reading store trees directly. It
//! returns claim records and compact provenance while keeping storage indexes
//! private to the world-state domain.
//!
//! # Example
//!
//! ```rust,no_run
//! use meld_world_model::world_state::store::WorldStateStore;
//! use meld_world_model::WorldStateQuery;
//!
//! let temp = tempfile::tempdir().unwrap();
//! let store = WorldStateStore::new(sled::open(temp.path()).unwrap()).unwrap();
//! let query = WorldStateQuery::new(&store);
//! let subject = meld_world_model::events::DomainObjectRef::new(
//!     "workspace_fs",
//!     "node",
//!     "node-a",
//! )
//! .unwrap();
//! assert!(query.current_claims_for_object(&subject).unwrap().is_empty());
//! ```

use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::contracts::{ClaimRecord, ProvenanceRecord};
use crate::world_state::store::WorldStateStore;

/// Planner-safe read facade for claim records.
pub struct WorldStateQuery<'a> {
    store: &'a WorldStateStore,
}

impl<'a> WorldStateQuery<'a> {
    /// Create a query facade over an existing store.
    pub fn new(store: &'a WorldStateStore) -> Self {
        Self { store }
    }

    /// Read active claims for one subject.
    pub fn current_claims_for_object(
        &self,
        subject: &DomainObjectRef,
    ) -> Result<Vec<ClaimRecord>, StorageError> {
        self.store.current_claims_for_object(subject)
    }

    /// Read active and superseded claims for one subject.
    pub fn claim_history_for_object(
        &self,
        subject: &DomainObjectRef,
    ) -> Result<Vec<ClaimRecord>, StorageError> {
        self.store.claim_history_for_object(subject)
    }

    /// Build compact provenance for one claim from attached evidence.
    pub fn provenance_for_claim(&self, claim_id: &str) -> Result<ProvenanceRecord, StorageError> {
        let evidence = self.store.evidence_for_claim(claim_id)?;
        let mut evidence_ids = Vec::new();
        let mut source_fact_ids = Vec::new();
        let mut objects = Vec::new();
        let mut relations = Vec::new();

        for record in evidence {
            evidence_ids.push(record.evidence_id.clone());
            source_fact_ids.push(record.source_fact_id.clone());
            objects.extend(record.objects);
            relations.extend(record.relations);
        }

        Ok(ProvenanceRecord {
            claim_id: claim_id.to_string(),
            evidence_ids,
            source_fact_ids,
            objects,
            relations,
        })
    }

    /// Read the supersession chain starting at one claim.
    pub fn supersession_chain_for_claim(
        &self,
        claim_id: &str,
    ) -> Result<Vec<ClaimRecord>, StorageError> {
        self.store.supersession_chain_for_claim(claim_id)
    }
}
