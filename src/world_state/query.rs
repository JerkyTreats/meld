use crate::error::StorageError;
use crate::events::DomainObjectRef;
use crate::world_state::contracts::{ClaimRecord, ProvenanceRecord};
use crate::world_state::store::WorldStateStore;

pub struct WorldStateQuery<'a> {
    store: &'a WorldStateStore,
}

impl<'a> WorldStateQuery<'a> {
    pub fn new(store: &'a WorldStateStore) -> Self {
        Self { store }
    }

    pub fn current_claims_for_object(
        &self,
        subject: &DomainObjectRef,
    ) -> Result<Vec<ClaimRecord>, StorageError> {
        self.store.current_claims_for_object(subject)
    }

    pub fn claim_history_for_object(
        &self,
        subject: &DomainObjectRef,
    ) -> Result<Vec<ClaimRecord>, StorageError> {
        self.store.claim_history_for_object(subject)
    }

    pub fn provenance_for_claim(&self, claim_id: &str) -> Result<ProvenanceRecord, StorageError> {
        let evidence = self.store.evidence_for_claim(claim_id)?;
        let mut evidence_ids = Vec::new();
        let mut source_fact_ids = Vec::new();
        let mut objects = Vec::new();
        let mut relations = Vec::new();

        for record in evidence {
            evidence_ids.push(record.evidence_id.clone());
            source_fact_ids.push(record.source_fact_id.clone());
            objects.extend(record.objects.into_iter());
            relations.extend(record.relations.into_iter());
        }

        Ok(ProvenanceRecord {
            claim_id: claim_id.to_string(),
            evidence_ids,
            source_fact_ids,
            objects,
            relations,
        })
    }

    pub fn supersession_chain_for_claim(
        &self,
        claim_id: &str,
    ) -> Result<Vec<ClaimRecord>, StorageError> {
        self.store.supersession_chain_for_claim(claim_id)
    }
}
