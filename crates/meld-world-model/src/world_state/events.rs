//! Event constructors for legacy claim state.
//!
//! These helpers wrap claim and evidence payloads in the shared event envelope
//! format. Reducers later project those envelopes into claim records,
//! provenance, and current-claim indexes.
//!
//! # Example
//!
//! ```rust
//! use meld_world_model::events::DomainObjectRef;
//! use meld_world_model::world_state::events::{claim_added_envelope, ClaimAddedEventData};
//! use meld_world_model::{ClaimKind, ClaimRecord, SettlementStatus};
//!
//! let subject = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
//! let envelope = claim_added_envelope(
//!     "session-a",
//!     ClaimAddedEventData {
//!         claim: ClaimRecord {
//!             claim_id: "claim-a".to_string(),
//!             claim_kind: ClaimKind::GenerationSucceeded,
//!             subject,
//!             status: SettlementStatus::Active,
//!             supporting_fact_ids: vec!["spine-a".to_string()],
//!             superseded_by: None,
//!             created_by_fact_id: "fact-a".to_string(),
//!             created_at_seq: 1,
//!             last_updated_seq: 1,
//!         },
//!     },
//! );
//! assert_eq!(envelope.event_type, "world_state.claim_added");
//! ```

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::events::{DomainObjectRef, EventEnvelope, EventRelation};
use crate::world_state::contracts::{ClaimRecord, EvidenceRecord};

/// Payload for adding a claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimAddedEventData {
    pub claim: ClaimRecord,
}

/// Payload for superseding a claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimSupersededEventData {
    pub claim: ClaimRecord,
}

/// Payload for attaching provenance evidence to a claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceAttachedEventData {
    pub evidence: EvidenceRecord,
}

fn world_state_envelope(
    session_id: &str,
    stream_id: &str,
    event_type: &str,
    data: serde_json::Value,
    objects: Vec<DomainObjectRef>,
    relations: Vec<EventRelation>,
) -> EventEnvelope {
    EventEnvelope::with_now_domain(
        session_id.to_string(),
        "world_state".to_string(),
        stream_id.to_string(),
        event_type.to_string(),
        None,
        data,
    )
    .with_graph(objects, relations)
}

/// Build a world-state event envelope for a new claim.
pub fn claim_added_envelope(session_id: &str, data: ClaimAddedEventData) -> EventEnvelope {
    let claim = &data.claim;
    world_state_envelope(
        session_id,
        &claim.claim_id,
        "world_state.claim_added",
        json!(data.clone()),
        vec![claim.subject.clone()],
        Vec::new(),
    )
    .with_record_id(claim.created_by_fact_id.clone())
}

/// Build a world-state event envelope for claim supersession.
pub fn claim_superseded_envelope(
    session_id: &str,
    data: ClaimSupersededEventData,
) -> EventEnvelope {
    let claim = &data.claim;
    world_state_envelope(
        session_id,
        &claim.claim_id,
        "world_state.claim_superseded",
        json!(data.clone()),
        vec![claim.subject.clone()],
        Vec::new(),
    )
    .with_record_id(format!(
        "world_state::claim_superseded::{}::{}",
        claim.claim_id, claim.last_updated_seq
    ))
}

/// Build a world-state event envelope for claim evidence.
pub fn evidence_attached_envelope(
    session_id: &str,
    data: EvidenceAttachedEventData,
) -> EventEnvelope {
    let evidence = &data.evidence;
    world_state_envelope(
        session_id,
        &evidence.claim_id,
        "world_state.evidence_attached",
        json!(data.clone()),
        evidence.objects.clone(),
        evidence.relations.clone(),
    )
    .with_record_id(format!(
        "world_state::evidence_attached::{}",
        evidence.evidence_id
    ))
}
