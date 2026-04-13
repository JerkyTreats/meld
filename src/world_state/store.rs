use std::io;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use crate::error::StorageError;
use crate::telemetry::DomainObjectRef;
use crate::world_state::contracts::{ClaimRecord, EvidenceRecord};

const TREE_FACTS: &str = "world_state_facts";
const TREE_CLAIMS: &str = "world_state_claims";
const TREE_EVIDENCE: &str = "world_state_evidence";
const TREE_CLAIM_EVIDENCE: &str = "world_state_claim_evidence";
const TREE_ACTIVE_BY_SUBJECT: &str = "world_state_active_by_subject";
const TREE_HISTORY_BY_SUBJECT: &str = "world_state_history_by_subject";
const TREE_SUPERSESSION: &str = "world_state_supersession";
const TREE_SOURCE_FACT_INDEX: &str = "world_state_source_fact_index";
const TREE_SEQ_INDEX: &str = "world_state_seq_index";
const KEY_PAD: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredWorldStateFact {
    pub fact_id: String,
    pub event_type: String,
    pub claim_id: Option<String>,
    pub evidence_id: Option<String>,
    pub source_spine_fact_id: Option<String>,
    pub seq: u64,
}

#[derive(Clone)]
pub struct WorldStateStore {
    db: Db,
    facts: Tree,
    claims: Tree,
    evidence: Tree,
    claim_evidence: Tree,
    active_by_subject: Tree,
    history_by_subject: Tree,
    supersession: Tree,
    source_fact_index: Tree,
    seq_index: Tree,
}

impl WorldStateStore {
    pub fn new(db: Db) -> Result<Self, StorageError> {
        Ok(Self {
            facts: db.open_tree(TREE_FACTS).map_err(to_storage_io)?,
            claims: db.open_tree(TREE_CLAIMS).map_err(to_storage_io)?,
            evidence: db.open_tree(TREE_EVIDENCE).map_err(to_storage_io)?,
            claim_evidence: db.open_tree(TREE_CLAIM_EVIDENCE).map_err(to_storage_io)?,
            active_by_subject: db.open_tree(TREE_ACTIVE_BY_SUBJECT).map_err(to_storage_io)?,
            history_by_subject: db.open_tree(TREE_HISTORY_BY_SUBJECT).map_err(to_storage_io)?,
            supersession: db.open_tree(TREE_SUPERSESSION).map_err(to_storage_io)?,
            source_fact_index: db.open_tree(TREE_SOURCE_FACT_INDEX).map_err(to_storage_io)?,
            seq_index: db.open_tree(TREE_SEQ_INDEX).map_err(to_storage_io)?,
            db,
        })
    }

    pub fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        Ok(Arc::new(Self::new(db)?))
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    pub fn put_fact(&self, fact: &StoredWorldStateFact) -> Result<(), StorageError> {
        self.facts
            .insert(
                fact.fact_id.as_bytes(),
                serde_json::to_vec(fact).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.seq_index
            .insert(
                encode_seq_index_key(fact.seq, &fact.fact_id).as_bytes(),
                fact.fact_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        if let Some(source_fact_id) = &fact.source_spine_fact_id {
            self.source_fact_index
                .insert(
                    encode_fact_membership_key(source_fact_id, &fact.fact_id).as_bytes(),
                    fact.fact_id.as_bytes(),
                )
                .map_err(to_storage_io)?;
        }
        Ok(())
    }

    pub fn get_fact(&self, fact_id: &str) -> Result<Option<StoredWorldStateFact>, StorageError> {
        let Some(raw) = self.facts.get(fact_id.as_bytes()).map_err(to_storage_io)? else {
            return Ok(None);
        };
        let parsed = serde_json::from_slice(&raw).map_err(to_storage_data)?;
        Ok(Some(parsed))
    }

    pub fn put_claim(&self, claim: &ClaimRecord) -> Result<(), StorageError> {
        let subject_key = claim.subject.index_key();
        self.claims
            .insert(
                claim.claim_id.as_bytes(),
                serde_json::to_vec(claim).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.history_by_subject
            .insert(
                encode_fact_membership_key(&subject_key, &claim.claim_id).as_bytes(),
                claim.claim_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    pub fn get_claim(&self, claim_id: &str) -> Result<Option<ClaimRecord>, StorageError> {
        let Some(raw) = self.claims.get(claim_id.as_bytes()).map_err(to_storage_io)? else {
            return Ok(None);
        };
        let parsed = serde_json::from_slice(&raw).map_err(to_storage_data)?;
        Ok(Some(parsed))
    }

    pub fn set_claim_active(
        &self,
        subject: &DomainObjectRef,
        claim_id: &str,
    ) -> Result<(), StorageError> {
        self.active_by_subject
            .insert(
                encode_fact_membership_key(&subject.index_key(), claim_id).as_bytes(),
                claim_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    pub fn clear_claim_active(
        &self,
        subject: &DomainObjectRef,
        claim_id: &str,
    ) -> Result<(), StorageError> {
        self.active_by_subject
            .remove(encode_fact_membership_key(&subject.index_key(), claim_id).as_bytes())
            .map_err(to_storage_io)?;
        Ok(())
    }

    pub fn put_evidence(&self, evidence: &EvidenceRecord) -> Result<(), StorageError> {
        self.evidence
            .insert(
                evidence.evidence_id.as_bytes(),
                serde_json::to_vec(evidence).map_err(to_storage_data)?,
            )
            .map_err(to_storage_io)?;
        self.claim_evidence
            .insert(
                encode_fact_membership_key(&evidence.claim_id, &evidence.evidence_id).as_bytes(),
                evidence.evidence_id.as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    pub fn get_evidence(&self, evidence_id: &str) -> Result<Option<EvidenceRecord>, StorageError> {
        let Some(raw) = self.evidence.get(evidence_id.as_bytes()).map_err(to_storage_io)? else {
            return Ok(None);
        };
        let parsed = serde_json::from_slice(&raw).map_err(to_storage_data)?;
        Ok(Some(parsed))
    }

    pub fn put_supersession(
        &self,
        claim_id: &str,
        superseded_by: &str,
    ) -> Result<(), StorageError> {
        self.supersession
            .insert(
                encode_fact_membership_key(claim_id, superseded_by).as_bytes(),
                superseded_by.as_bytes(),
            )
            .map_err(to_storage_io)?;
        Ok(())
    }

    pub fn current_claims_for_object(
        &self,
        subject: &DomainObjectRef,
    ) -> Result<Vec<ClaimRecord>, StorageError> {
        self.claims_from_index(&self.active_by_subject, &subject.index_key())
    }

    pub fn claim_history_for_object(
        &self,
        subject: &DomainObjectRef,
    ) -> Result<Vec<ClaimRecord>, StorageError> {
        self.claims_from_index(&self.history_by_subject, &subject.index_key())
    }

    pub fn evidence_for_claim(&self, claim_id: &str) -> Result<Vec<EvidenceRecord>, StorageError> {
        let mut evidence = Vec::new();
        let prefix = format!("{claim_id}::");
        for result in self.claim_evidence.scan_prefix(prefix.as_bytes()) {
            let (_, value) = result.map_err(to_storage_io)?;
            let evidence_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(record) = self.get_evidence(&evidence_id)? {
                evidence.push(record);
            }
        }
        Ok(evidence)
    }

    pub fn supersession_chain_for_claim(
        &self,
        claim_id: &str,
    ) -> Result<Vec<ClaimRecord>, StorageError> {
        let mut chain = Vec::new();
        let prefix = format!("{claim_id}::");
        for result in self.supersession.scan_prefix(prefix.as_bytes()) {
            let (_, value) = result.map_err(to_storage_io)?;
            let next_claim_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(record) = self.get_claim(&next_claim_id)? {
                chain.push(record);
            }
        }
        Ok(chain)
    }

    fn claims_from_index(
        &self,
        tree: &Tree,
        subject_key: &str,
    ) -> Result<Vec<ClaimRecord>, StorageError> {
        let mut claims = Vec::new();
        let prefix = format!("{subject_key}::");
        for result in tree.scan_prefix(prefix.as_bytes()) {
            let (_, value) = result.map_err(to_storage_io)?;
            let claim_id = String::from_utf8(value.to_vec()).map_err(to_storage_utf8)?;
            if let Some(record) = self.get_claim(&claim_id)? {
                claims.push(record);
            }
        }
        claims.sort_by_key(|record| record.created_at_seq);
        Ok(claims)
    }
}

fn encode_fact_membership_key(prefix: &str, item_id: &str) -> String {
    format!("{prefix}::{item_id}")
}

fn encode_seq_index_key(seq: u64, fact_id: &str) -> String {
    format!("{seq:0KEY_PAD$}::{fact_id}")
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
