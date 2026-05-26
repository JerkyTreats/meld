//! In-memory projections for legacy claim reduction.
//!
//! Reducers use these structures while replaying events before durable indexes
//! are queried. They are intentionally small mirrors of store indexes.

use std::collections::{BTreeMap, BTreeSet};

/// Current active claims grouped by subject object key.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CurrentClaimProjection {
    /// Active claim ids keyed by object index key.
    pub active_claims_by_object: BTreeMap<String, BTreeSet<String>>,
    /// Highest event sequence applied to this projection.
    pub last_applied_seq: u64,
}

impl CurrentClaimProjection {
    /// Mark a claim active for one object.
    pub fn activate(&mut self, object_key: String, claim_id: String, seq: u64) {
        self.active_claims_by_object
            .entry(object_key)
            .or_default()
            .insert(claim_id);
        self.last_applied_seq = self.last_applied_seq.max(seq);
    }

    /// Remove an active claim when another claim supersedes it.
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

/// Provenance edges gathered while reducing claim state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClaimProvenanceProjection {
    /// Evidence ids keyed by claim id.
    pub evidence_ids_by_claim: BTreeMap<String, BTreeSet<String>>,
    /// Source fact ids keyed by claim id.
    pub source_fact_ids_by_claim: BTreeMap<String, BTreeSet<String>>,
    /// Superseding claim ids keyed by original claim id.
    pub supersession_chain_by_claim: BTreeMap<String, Vec<String>>,
    /// Highest event sequence applied to this projection.
    pub last_applied_seq: u64,
}

impl ClaimProvenanceProjection {
    /// Attach evidence and source fact ids to a claim.
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

    /// Attach a supersession edge to a claim.
    pub fn add_supersession(&mut self, claim_id: &str, superseded_by: String, seq: u64) {
        self.supersession_chain_by_claim
            .entry(claim_id.to_string())
            .or_default()
            .push(superseded_by);
        self.last_applied_seq = self.last_applied_seq.max(seq);
    }
}
