use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CurrentClaimProjection {
    pub active_claims_by_object: BTreeMap<String, BTreeSet<String>>,
    pub last_applied_seq: u64,
}

impl CurrentClaimProjection {
    pub fn activate(&mut self, object_key: String, claim_id: String, seq: u64) {
        self.active_claims_by_object
            .entry(object_key)
            .or_default()
            .insert(claim_id);
        self.last_applied_seq = self.last_applied_seq.max(seq);
    }

    pub fn supersede(&mut self, object_key: &str, claim_id: &str, seq: u64) {
        if let Some(claims) = self.active_claims_by_object.get_mut(object_key) {
            claims.remove(claim_id);
            if claims.is_empty() {
                self.active_claims_by_object.remove(object_key);
            }
        }
        self.last_applied_seq = self.last_applied_seq.max(seq);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClaimProvenanceProjection {
    pub evidence_ids_by_claim: BTreeMap<String, BTreeSet<String>>,
    pub source_fact_ids_by_claim: BTreeMap<String, BTreeSet<String>>,
    pub supersession_chain_by_claim: BTreeMap<String, Vec<String>>,
    pub last_applied_seq: u64,
}

impl ClaimProvenanceProjection {
    pub fn add_evidence(
        &mut self,
        claim_id: &str,
        evidence_id: String,
        source_fact_id: String,
        seq: u64,
    ) {
        self.evidence_ids_by_claim
            .entry(claim_id.to_string())
            .or_default()
            .insert(evidence_id);
        self.source_fact_ids_by_claim
            .entry(claim_id.to_string())
            .or_default()
            .insert(source_fact_id);
        self.last_applied_seq = self.last_applied_seq.max(seq);
    }

    pub fn add_supersession(&mut self, claim_id: &str, superseded_by: String, seq: u64) {
        self.supersession_chain_by_claim
            .entry(claim_id.to_string())
            .or_default()
            .push(superseded_by);
        self.last_applied_seq = self.last_applied_seq.max(seq);
    }
}
