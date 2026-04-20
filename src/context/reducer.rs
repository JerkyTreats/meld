use crate::error::StorageError;
use crate::events::{DomainObjectRef, EventRecord};
use crate::world_state::graph::contracts::{
    AnchorEndInput, AnchorSelectionInput, PerspectiveKey, TraversalIntent,
};

pub(crate) fn graph_reducer_intents_for_event(
    event: &EventRecord,
    source_fact_id: &str,
) -> Result<Vec<TraversalIntent>, StorageError> {
    match event.event_type.as_str() {
        "context.head_selected" => {
            let Some(head_ref) = find_object_ref(&event.objects, "context", "head") else {
                return Ok(Vec::new());
            };
            let Some(subject) = find_object_ref(&event.objects, "workspace_fs", "node") else {
                return Ok(Vec::new());
            };
            let Some(target) = find_object_ref(&event.objects, "context", "frame") else {
                return Ok(Vec::new());
            };
            let frame_type = head_ref
                .object_id
                .split_once("::")
                .map(|(_, frame_type)| frame_type.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            Ok(vec![TraversalIntent::SelectAnchor(AnchorSelectionInput {
                anchor_ref: head_ref,
                subject,
                perspective: PerspectiveKey::new("frame_type", frame_type)?,
                target,
                source_fact_id: source_fact_id.to_string(),
            })])
        }
        "context.head_tombstoned" => {
            let Some(head_ref) = find_object_ref(&event.objects, "context", "head") else {
                return Ok(Vec::new());
            };
            Ok(vec![TraversalIntent::EndAnchor(AnchorEndInput {
                anchor_ref: head_ref,
                ended_at_seq: event.seq,
            })])
        }
        _ => Ok(Vec::new()),
    }
}

fn find_object_ref(
    objects: &[DomainObjectRef],
    domain_id: &str,
    object_kind: &str,
) -> Option<DomainObjectRef> {
    objects
        .iter()
        .find(|object| object.domain_id == domain_id && object.object_kind == object_kind)
        .cloned()
}
