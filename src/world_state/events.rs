use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::telemetry::events::ProgressEnvelope;
use crate::telemetry::{DomainObjectRef, EventRelation};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimAddedEventData {
    pub fact_id: String,
    pub claim_id: String,
    pub claim_kind: String,
    pub subject: DomainObjectRef,
    pub source_fact_id: String,
    pub seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimSupersededEventData {
    pub fact_id: String,
    pub claim_id: String,
    pub superseded_by: String,
    pub subject: DomainObjectRef,
    pub source_fact_id: String,
    pub seq: u64,
}

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
) -> ProgressEnvelope {
    ProgressEnvelope::with_now_domain(
        session_id.to_string(),
        "world_state".to_string(),
        stream_id.to_string(),
        event_type.to_string(),
        None,
        data,
    )
    .with_graph(objects, relations)
}

pub fn claim_added_envelope(session_id: &str, data: ClaimAddedEventData) -> ProgressEnvelope {
    world_state_envelope(
        session_id,
        &data.claim_id,
        "world_state.claim_added",
        json!(data.clone()),
        vec![data.subject],
        Vec::new(),
    )
}

pub fn claim_superseded_envelope(
    session_id: &str,
    data: ClaimSupersededEventData,
) -> ProgressEnvelope {
    world_state_envelope(
        session_id,
        &data.claim_id,
        "world_state.claim_superseded",
        json!(data.clone()),
        vec![data.subject],
        Vec::new(),
    )
}

pub fn evidence_attached_envelope(
    session_id: &str,
    claim_id: &str,
    data: EvidenceAttachedEventData,
    objects: Vec<DomainObjectRef>,
    relations: Vec<EventRelation>,
) -> ProgressEnvelope {
    world_state_envelope(
        session_id,
        claim_id,
        "world_state.evidence_attached",
        json!(data),
        objects,
        relations,
    )
}
