use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::events::{DomainObjectRef, EventEnvelope, EventRelation};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorSelectedEventData {
    pub fact_id: String,
    pub anchor_id: String,
    pub anchor_ref: DomainObjectRef,
    pub subject: DomainObjectRef,
    pub perspective_kind: String,
    pub perspective_id: String,
    pub target: DomainObjectRef,
    pub source_fact_id: String,
    pub seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorSupersededEventData {
    pub fact_id: String,
    pub anchor_id: String,
    pub anchor_ref: DomainObjectRef,
    pub superseded_by_anchor_id: String,
    pub source_fact_id: String,
    pub seq: u64,
}

fn traversal_envelope(
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

pub fn anchor_selected_envelope(session_id: &str, data: AnchorSelectedEventData) -> EventEnvelope {
    traversal_envelope(
        session_id,
        &data.anchor_id,
        "world_state.anchor_selected",
        json!(data.clone()),
        vec![data.anchor_ref, data.subject, data.target],
        Vec::new(),
    )
    .with_record_id(data.fact_id)
}

pub fn anchor_superseded_envelope(
    session_id: &str,
    data: AnchorSupersededEventData,
) -> EventEnvelope {
    traversal_envelope(
        session_id,
        &data.anchor_id,
        "world_state.anchor_superseded",
        json!(data.clone()),
        vec![data.anchor_ref],
        Vec::new(),
    )
    .with_record_id(data.fact_id)
}
