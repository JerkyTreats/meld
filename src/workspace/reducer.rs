use crate::error::StorageError;
use crate::events::{DomainObjectRef, EventRecord};
use crate::world_state::graph::contracts::{AnchorSelectionInput, PerspectiveKey, TraversalIntent};

pub(crate) fn graph_reducer_intents_for_event(
    event: &EventRecord,
    source_fact_id: &str,
) -> Result<Vec<TraversalIntent>, StorageError> {
    match event.event_type.as_str() {
        "workspace_fs.snapshot_selected" => {
            let Some(source) = find_object_ref(&event.objects, "workspace_fs", "source") else {
                return Ok(Vec::new());
            };
            let Some(snapshot) = find_object_ref(&event.objects, "workspace_fs", "snapshot") else {
                return Ok(Vec::new());
            };
            let anchor_ref =
                DomainObjectRef::new("workspace_fs", "snapshot_head", &source.object_id)?;
            Ok(vec![TraversalIntent::SelectAnchor(AnchorSelectionInput {
                anchor_ref,
                subject: source,
                perspective: PerspectiveKey::new("snapshot", "current")?,
                target: snapshot,
                source_fact_id: source_fact_id.to_string(),
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
