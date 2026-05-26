//! Claim records used by the legacy world-state projection.
//!
//! This surface preserves the claim and provenance model that predates the
//! graph traversal substrate. New planner-facing state should prefer graph and
//! belief views, while these contracts remain useful for compatibility tests and
//! query adapters.
//!
//! # Example
//!
//! ```rust
//! use meld_world_model::events::DomainObjectRef;
//! use meld_world_model::{ClaimKind, ClaimRecord, SettlementStatus};
//!
//! let claim = ClaimRecord {
//!     claim_id: "claim-a".to_string(),
//!     claim_kind: ClaimKind::GenerationSucceeded,
//!     subject: DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap(),
//!     status: SettlementStatus::Active,
//!     supporting_fact_ids: vec!["fact-a".to_string()],
//!     superseded_by: None,
//!     created_by_fact_id: "fact-a".to_string(),
//!     created_at_seq: 1,
//!     last_updated_seq: 1,
//! };
//!
//! assert_eq!(claim.claim_kind.as_str(), "generation_succeeded");
//! ```

use serde::{Deserialize, Serialize};

use crate::events::{DomainObjectRef, EventRelation};

/// Durable claim identifier.
pub type ClaimId = String;
/// Durable evidence identifier.
pub type EvidenceId = String;
/// Durable world-state fact identifier.
pub type WorldStateFactId = String;

/// Coarse claim categories produced by reducers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimKind {
    /// A generation task completed successfully.
    GenerationSucceeded,
    /// A generation task reached a terminal failure.
    GenerationFailed,
    /// A produced artifact is available for the subject.
    ArtifactAvailable,
}

impl ClaimKind {
    /// Stable storage and event label for the claim kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GenerationSucceeded => "generation_succeeded",
            Self::GenerationFailed => "generation_failed",
            Self::ArtifactAvailable => "artifact_available",
        }
    }
}

/// Current settlement state for a claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementStatus {
    /// The claim is the current assertion for its subject and kind.
    Active,
    /// A later claim has explicitly replaced this claim.
    Superseded,
}

/// Append-only claim plus current settlement metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimRecord {
    /// Stable claim id.
    pub claim_id: ClaimId,
    /// Claim category.
    pub claim_kind: ClaimKind,
    /// Object that the claim describes.
    pub subject: DomainObjectRef,
    /// Current settlement state.
    pub status: SettlementStatus,
    /// Source fact ids that support the claim.
    pub supporting_fact_ids: Vec<String>,
    /// Replacement claim id when superseded.
    pub superseded_by: Option<ClaimId>,
    /// Fact that originally created this claim.
    pub created_by_fact_id: String,
    /// Runtime sequence where the claim first appeared.
    pub created_at_seq: u64,
    /// Last runtime sequence that changed settlement metadata.
    pub last_updated_seq: u64,
}

/// Evidence attached to one claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    /// Stable evidence id.
    pub evidence_id: EvidenceId,
    /// Claim supported by this evidence.
    pub claim_id: ClaimId,
    /// Source fact that produced this evidence record.
    pub source_fact_id: String,
    /// Source event type for audit and replay.
    pub source_event_type: String,
    /// Objects copied from the source fact.
    pub objects: Vec<DomainObjectRef>,
    /// Relations copied from the source fact.
    pub relations: Vec<EventRelation>,
}

/// Compact provenance summary for planner-safe claim reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    /// Claim whose provenance is summarized.
    pub claim_id: ClaimId,
    /// Evidence ids attached to the claim.
    pub evidence_ids: Vec<EvidenceId>,
    /// Source fact ids behind the evidence.
    pub source_fact_ids: Vec<String>,
    /// Objects observed in evidence.
    pub objects: Vec<DomainObjectRef>,
    /// Relations observed in evidence.
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
