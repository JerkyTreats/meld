//! Event constructors for graph traversal anchors.
//!
//! These helpers produce event envelopes that the traversal reducer understands.
//! They keep event type strings and graph object bindings in one place so
//! callers do not duplicate reducer-facing protocol details.
//!
//! # Example
//!
//! ```rust
//! use meld_world_model::events::DomainObjectRef;
//! use meld_world_model::graph::events::{anchor_selected_envelope, AnchorSelectedEventData};
//!
//! let node = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
//! let frame = DomainObjectRef::new("context", "frame", "frame-a").unwrap();
//! let anchor_ref = DomainObjectRef::new("context", "head", "node-a::analysis").unwrap();
//! let envelope = anchor_selected_envelope(
//!     "session-a",
//!     AnchorSelectedEventData {
//!         fact_id: "fact-a".to_string(),
//!         anchor_id: "anchor-a".to_string(),
//!         anchor_ref,
//!         subject: node,
//!         perspective_kind: "frame_type".to_string(),
//!         perspective_id: "analysis".to_string(),
//!         target: frame,
//!         source_fact_id: "spine-a".to_string(),
//!         seq: 1,
//!     },
//! );
//! assert_eq!(envelope.event_type, "world_state.anchor_selected");
//! ```

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::events::{DomainObjectRef, EventEnvelope, EventRelation};

/// Payload for selecting a current anchor.
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

/// Payload for superseding a selected anchor.
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

/// Build a traversal event envelope for selecting an anchor.
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

/// Build a traversal event envelope for superseding an anchor.
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
