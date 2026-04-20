use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::context::frame::Basis;
use crate::events::{DomainObjectRef, EventEnvelope, EventRelation};
use crate::types::{FrameID, NodeID};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameAddedEventData {
    pub node_id: String,
    pub frame_id: String,
    pub frame_type: String,
    pub agent_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadSelectedEventData {
    pub node_id: String,
    pub frame_type: String,
    pub frame_id: String,
    pub previous_frame_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadTombstonedEventData {
    pub node_id: String,
    pub frame_type: String,
    pub previous_frame_id: Option<String>,
}

fn context_envelope(
    session_id: &str,
    stream_id: &str,
    event_type: &str,
    data: serde_json::Value,
    objects: Vec<DomainObjectRef>,
    relations: Vec<EventRelation>,
) -> EventEnvelope {
    EventEnvelope::with_now_domain(
        session_id.to_string(),
        "context".to_string(),
        stream_id.to_string(),
        event_type.to_string(),
        None,
        data,
    )
    .with_graph(objects, relations)
}

pub fn frame_added_envelope(
    session_id: &str,
    node_id: NodeID,
    basis: &Basis,
    frame_id: FrameID,
    frame_type: &str,
    agent_id: &str,
) -> EventEnvelope {
    let mut objects = vec![frame_ref(frame_id)];
    let mut relations = Vec::new();
    match basis {
        Basis::Node(basis_node) => {
            objects.push(node_ref(*basis_node));
            relations.push(
                EventRelation::new("attached_to", frame_ref(frame_id), node_ref(*basis_node))
                    .expect("attached_to relation should be valid"),
            );
        }
        Basis::Frame(previous_frame_id) => {
            objects.push(frame_ref(*previous_frame_id));
            relations.push(
                EventRelation::new(
                    "derived_from",
                    frame_ref(frame_id),
                    frame_ref(*previous_frame_id),
                )
                .expect("derived_from relation should be valid"),
            );
        }
        Basis::Both { node, frame } => {
            objects.push(node_ref(*node));
            objects.push(frame_ref(*frame));
            relations.push(
                EventRelation::new("attached_to", frame_ref(frame_id), node_ref(*node))
                    .expect("attached_to relation should be valid"),
            );
            relations.push(
                EventRelation::new("derived_from", frame_ref(frame_id), frame_ref(*frame))
                    .expect("derived_from relation should be valid"),
            );
        }
    }
    context_envelope(
        session_id,
        &hex::encode(node_id),
        "context.frame_added",
        json!(FrameAddedEventData {
            node_id: hex::encode(node_id),
            frame_id: hex::encode(frame_id),
            frame_type: frame_type.to_string(),
            agent_id: agent_id.to_string(),
        }),
        objects,
        relations,
    )
}

pub fn head_selected_envelope(
    session_id: &str,
    node_id: NodeID,
    frame_type: &str,
    frame_id: FrameID,
    previous_frame_id: Option<FrameID>,
) -> EventEnvelope {
    let mut relations = vec![
        EventRelation::new(
            "attached_to",
            head_ref(node_id, frame_type),
            node_ref(node_id),
        )
        .expect("head attached_to relation should be valid"),
        EventRelation::new(
            "selected",
            head_ref(node_id, frame_type),
            frame_ref(frame_id),
        )
        .expect("head selected relation should be valid"),
    ];
    if let Some(previous_frame_id) = previous_frame_id {
        relations.push(
            EventRelation::new(
                "supersedes",
                frame_ref(frame_id),
                frame_ref(previous_frame_id),
            )
            .expect("head supersedes relation should be valid"),
        );
    }
    context_envelope(
        session_id,
        &head_object_id(node_id, frame_type),
        "context.head_selected",
        json!(HeadSelectedEventData {
            node_id: hex::encode(node_id),
            frame_type: frame_type.to_string(),
            frame_id: hex::encode(frame_id),
            previous_frame_id: previous_frame_id.map(hex::encode),
        }),
        vec![
            head_ref(node_id, frame_type),
            node_ref(node_id),
            frame_ref(frame_id),
        ],
        relations,
    )
}

pub fn head_tombstoned_envelope(
    session_id: &str,
    node_id: NodeID,
    frame_type: &str,
    previous_frame_id: Option<FrameID>,
) -> EventEnvelope {
    let mut objects = vec![head_ref(node_id, frame_type), node_ref(node_id)];
    if let Some(previous_frame_id) = previous_frame_id {
        objects.push(frame_ref(previous_frame_id));
    }
    context_envelope(
        session_id,
        &head_object_id(node_id, frame_type),
        "context.head_tombstoned",
        json!(HeadTombstonedEventData {
            node_id: hex::encode(node_id),
            frame_type: frame_type.to_string(),
            previous_frame_id: previous_frame_id.map(hex::encode),
        }),
        objects,
        vec![EventRelation::new(
            "attached_to",
            head_ref(node_id, frame_type),
            node_ref(node_id),
        )
        .expect("head attached_to relation should be valid")],
    )
}

pub fn head_ref(node_id: NodeID, frame_type: &str) -> DomainObjectRef {
    DomainObjectRef::new("context", "head", head_object_id(node_id, frame_type))
        .expect("head ref should be valid")
}

fn head_object_id(node_id: NodeID, frame_type: &str) -> String {
    format!("{}::{}", hex::encode(node_id), frame_type)
}

fn node_ref(node_id: NodeID) -> DomainObjectRef {
    DomainObjectRef::new("workspace_fs", "node", hex::encode(node_id))
        .expect("node ref should be valid")
}

fn frame_ref(frame_id: FrameID) -> DomainObjectRef {
    DomainObjectRef::new("context", "frame", hex::encode(frame_id))
        .expect("frame ref should be valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_head_ref_is_stable_for_node_and_frame_type() {
        let node_id = [7u8; 32];
        let head_ref = head_ref(node_id, "analysis");
        assert_eq!(head_ref.domain_id, "context");
        assert_eq!(head_ref.object_kind, "head");
        assert_eq!(
            head_ref.object_id,
            format!("{}::analysis", hex::encode(node_id))
        );
    }
}
