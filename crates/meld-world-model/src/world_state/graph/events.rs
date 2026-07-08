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
//!         anchor: meld_world_model::AnchorSelectionRecord {
//!             anchor_id: "anchor-a".to_string(),
//!             anchor_ref,
//!             subject: node,
//!             perspective: meld_world_model::PerspectiveKey::new("frame_type", "analysis").unwrap(),
//!             target: frame,
//!             source_fact_ids: vec!["ledger-a".to_string()],
//!             created_by_fact_id: "fact-a".to_string(),
//!             selected_at_seq: 1,
//!             ended_at_seq: None,
//!             ended_by_anchor_id: None,
//!             ended_by_fact_id: None,
//!         },
//!     },
//! );
//! assert_eq!(envelope.event_type, "world_state.anchor_selected");
//! ```

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::events::{DomainObjectRef, EventEnvelope, EventRelation};
use crate::world_state::graph::contracts::AnchorSelectionRecord;

/// Payload for selecting a current anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorSelectedEventData {
    pub anchor: AnchorSelectionRecord,
}

/// Payload for superseding a selected anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorSupersededEventData {
    pub anchor: AnchorSelectionRecord,
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
    let anchor = &data.anchor;
    traversal_envelope(
        session_id,
        &anchor.anchor_id,
        "world_state.anchor_selected",
        json!(data.clone()),
        vec![
            anchor.anchor_ref.clone(),
            anchor.subject.clone(),
            anchor.target.clone(),
        ],
        Vec::new(),
    )
    .with_record_id(anchor.created_by_fact_id.clone())
}

/// Build a traversal event envelope for superseding an anchor.
pub fn anchor_superseded_envelope(
    session_id: &str,
    data: AnchorSupersededEventData,
) -> EventEnvelope {
    let anchor = &data.anchor;
    let fact_id = anchor
        .ended_by_fact_id
        .clone()
        .unwrap_or_else(|| format!("world_state::anchor_superseded::{}", anchor.anchor_id));
    traversal_envelope(
        session_id,
        &anchor.anchor_id,
        "world_state.anchor_superseded",
        json!(data.clone()),
        vec![
            anchor.anchor_ref.clone(),
            anchor.subject.clone(),
            anchor.target.clone(),
        ],
        Vec::new(),
    )
    .with_record_id(fact_id)
}
