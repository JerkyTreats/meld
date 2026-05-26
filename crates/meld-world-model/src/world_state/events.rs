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
//!
//! let subject = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
//! let envelope = claim_added_envelope(
//!     "session-a",
//!     ClaimAddedEventData {
//!         fact_id: "fact-a".to_string(),
//!         claim_id: "claim-a".to_string(),
//!         claim_kind: "generation_succeeded".to_string(),
//!         subject,
//!         source_fact_id: "spine-a".to_string(),
//!         seq: 1,
//!     },
//! );
//! assert_eq!(envelope.event_type, "world_state.claim_added");
//! ```

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::events::{DomainObjectRef, EventEnvelope, EventRelation};

/// Payload for adding a claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimAddedEventData {
    pub fact_id: String,
    pub claim_id: String,
    pub claim_kind: String,
    pub subject: DomainObjectRef,
    pub source_fact_id: String,
    pub seq: u64,
}

/// Payload for superseding a claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimSupersededEventData {
    pub fact_id: String,
    pub claim_id: String,
    pub superseded_by: String,
    pub subject: DomainObjectRef,
    pub source_fact_id: String,
    pub seq: u64,
}

/// Payload for attaching provenance evidence to a claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceAttachedEventData {
    pub fact_id: String,
    pub evidence_id: String,
    pub claim_id: String,
    pub source_fact_id: String,
    pub source_event_type: String,
    pub seq: u64,
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
    world_state_envelope(
        session_id,
        &data.claim_id,
        "world_state.claim_added",
        json!(data.clone()),
        vec![data.subject],
        Vec::new(),
    )
}

/// Build a world-state event envelope for claim supersession.
pub fn claim_superseded_envelope(
    session_id: &str,
    data: ClaimSupersededEventData,
) -> EventEnvelope {
    world_state_envelope(
        session_id,
        &data.claim_id,
        "world_state.claim_superseded",
        json!(data.clone()),
        vec![data.subject],
        Vec::new(),
    )
}

/// Build a world-state event envelope for claim evidence.
pub fn evidence_attached_envelope(
    session_id: &str,
    claim_id: &str,
    data: EvidenceAttachedEventData,
    objects: Vec<DomainObjectRef>,
    relations: Vec<EventRelation>,
) -> EventEnvelope {
    world_state_envelope(
        session_id,
        claim_id,
        "world_state.evidence_attached",
        json!(data),
        objects,
        relations,
    )
}
