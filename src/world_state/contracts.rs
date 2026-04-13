use serde::{Deserialize, Serialize};

use crate::telemetry::{DomainObjectRef, EventRelation};

pub type ClaimId = String;
pub type EvidenceId = String;
pub type WorldStateFactId = String;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimKind {
    GenerationSucceeded,
    GenerationFailed,
    ArtifactAvailable,
}

impl ClaimKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GenerationSucceeded => "generation_succeeded",
            Self::GenerationFailed => "generation_failed",
            Self::ArtifactAvailable => "artifact_available",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementStatus {
    Active,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimRecord {
    pub claim_id: ClaimId,
    pub claim_kind: ClaimKind,
    pub subject: DomainObjectRef,
    pub status: SettlementStatus,
    pub supporting_fact_ids: Vec<String>,
    pub superseded_by: Option<ClaimId>,
    pub created_by_fact_id: String,
    pub created_at_seq: u64,
    pub last_updated_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub evidence_id: EvidenceId,
    pub claim_id: ClaimId,
    pub source_fact_id: String,
    pub source_event_type: String,
    pub objects: Vec<DomainObjectRef>,
    pub relations: Vec<EventRelation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub claim_id: ClaimId,
    pub evidence_ids: Vec<EvidenceId>,
    pub source_fact_ids: Vec<String>,
    pub objects: Vec<DomainObjectRef>,
    pub relations: Vec<EventRelation>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_state_records_round_trip() {
        let claim = ClaimRecord {
            claim_id: "claim_a".to_string(),
            claim_kind: ClaimKind::GenerationSucceeded,
            subject: DomainObjectRef::new("workspace_fs", "node", "node_a").unwrap(),
            status: SettlementStatus::Active,
            supporting_fact_ids: vec!["fact_a".to_string()],
            superseded_by: None,
            created_by_fact_id: "fact_a".to_string(),
            created_at_seq: 1,
            last_updated_seq: 1,
        };
        let serialized = serde_json::to_string(&claim).unwrap();
        let parsed: ClaimRecord = serde_json::from_str(&serialized).unwrap();
        assert_eq!(parsed.claim_id, "claim_a");
        assert_eq!(parsed.claim_kind, ClaimKind::GenerationSucceeded);
    }
}
