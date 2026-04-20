use crate::error::StorageError;
use crate::events::{DomainObjectRef, EventRecord};
use crate::world_state::graph::contracts::{AnchorSelectionInput, PerspectiveKey, TraversalIntent};

pub(crate) fn graph_reducer_intents_for_event(
    event: &EventRecord,
    source_fact_id: &str,
) -> Result<Vec<TraversalIntent>, StorageError> {
    match event.event_type.as_str() {
        "execution.task.artifact_emitted" => {
            let Some(task_run) = find_object_ref(&event.objects, "execution", "task_run") else {
                return Ok(Vec::new());
            };
            let Some(artifact) = find_object_ref(&event.objects, "execution", "artifact") else {
                return Ok(Vec::new());
            };
            let Some(slot_ref) = find_object_ref(&event.objects, "execution", "artifact_slot")
            else {
                return Ok(Vec::new());
            };
            let artifact_type_id = slot_ref
                .object_id
                .rsplit_once("::")
                .map(|(_, artifact_type_id)| artifact_type_id.to_string())
                .unwrap_or_else(|| "artifact".to_string());
            Ok(vec![TraversalIntent::SelectAnchor(AnchorSelectionInput {
                anchor_ref: slot_ref,
                subject: task_run,
                perspective: PerspectiveKey::new("artifact_type", artifact_type_id)?,
                target: artifact,
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
