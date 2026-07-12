//! Event constructors for graph traversal anchors.
//!
//! These helpers produce event envelopes that the traversal reducer understands.
//! They keep event type strings and graph object bindings in one place so
//! callers do not duplicate reducer-facing protocol details.
//!
//! # Example
//!
//! ```rust
//! use meld_world_model::events::{DomainObjectRef, EventRecordRef, LedgerIdentity};
//! use meld_world_model::graph::events::{
//!     anchor_selected_envelope_from_record, AnchorSelectedEventData,
//! };
//! use std::str::FromStr;
//!
//! let node = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
//! let frame = DomainObjectRef::new("context", "frame", "frame-a").unwrap();
//! let anchor_ref = DomainObjectRef::new("context", "head", "node-a::analysis").unwrap();
//! let envelope = anchor_selected_envelope_from_record(
//!     "session-a",
//!     EventRecordRef {
//!         ledger_id: LedgerIdentity::from_str("018d2fd1-6030-7c6a-b03f-41d44f348d63").unwrap(),
//!         seq: 1,
//!     },
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

use crate::events::{DomainObjectRef, EventEnvelope, EventRecordRef, EventRelation};
use crate::world_state::graph::contracts::AnchorSelectionRecord;
use serde::{Deserialize, Serialize};
use serde_json::json;

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
// TODO compat-shim: E4 removes this identity-less wrapper after
// graph_derived_envelopes_carry_structural_source_record_provenance and the
// graph-runtime port parity tests supply EventRecordRef for every append.
pub fn anchor_selected_envelope(session_id: &str, data: AnchorSelectedEventData) -> EventEnvelope {
    anchor_selected_envelope_inner(session_id, data)
}

/// Build a traversal event for selecting an anchor from one canonical source.
pub fn anchor_selected_envelope_from_record(
    session_id: &str,
    source_record: EventRecordRef,
    data: AnchorSelectedEventData,
) -> EventEnvelope {
    anchor_selected_envelope_inner(session_id, data).with_source_records(vec![source_record])
}

fn anchor_selected_envelope_inner(
    session_id: &str,
    data: AnchorSelectedEventData,
) -> EventEnvelope {
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
// TODO compat-shim: E4 removes this identity-less wrapper after
// graph_derived_envelopes_carry_structural_source_record_provenance and the
// graph-runtime port parity tests supply EventRecordRef for every append.
pub fn anchor_superseded_envelope(
    session_id: &str,
    data: AnchorSupersededEventData,
) -> EventEnvelope {
    anchor_superseded_envelope_inner(session_id, data)
}

/// Build a traversal event for superseding an anchor from one canonical source.
pub fn anchor_superseded_envelope_from_record(
    session_id: &str,
    source_record: EventRecordRef,
    data: AnchorSupersededEventData,
) -> EventEnvelope {
    anchor_superseded_envelope_inner(session_id, data).with_source_records(vec![source_record])
}

fn anchor_superseded_envelope_inner(
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
