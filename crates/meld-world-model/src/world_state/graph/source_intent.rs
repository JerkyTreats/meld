use crate::error::StorageError;
use crate::events::{DomainObjectRef, EventRecord};
use crate::world_state::graph::contracts::{
    AnchorEndInput, AnchorSelectionInput, PerspectiveKey, TraversalIntent,
};

pub fn traversal_intents_for_event(
    event: &EventRecord,
    source_fact_id: &str,
) -> Result<Vec<TraversalIntent>, StorageError> {
    match event.event_type.as_str() {
        "workspace_fs.snapshot_selected" => workspace_snapshot_selected(event, source_fact_id),
        "context.head_selected" => context_head_selected(event, source_fact_id),
        "context.head_tombstoned" => context_head_tombstoned(event),
        "execution.task.artifact_emitted" => task_artifact_emitted(event, source_fact_id),
        _ => Ok(Vec::new()),
    }
}

fn workspace_snapshot_selected(
    event: &EventRecord,
    source_fact_id: &str,
) -> Result<Vec<TraversalIntent>, StorageError> {
    let Some(source) = find_object_ref(&event.objects, "workspace_fs", "source") else {
        return Ok(Vec::new());
    };
    let Some(snapshot) = find_object_ref(&event.objects, "workspace_fs", "snapshot") else {
        return Ok(Vec::new());
    };
    let anchor_ref = DomainObjectRef::new("workspace_fs", "snapshot_head", &source.object_id)?;
    Ok(vec![TraversalIntent::SelectAnchor(AnchorSelectionInput {
        anchor_ref,
        subject: source,
        perspective: PerspectiveKey::new("snapshot", "current")?,
        target: snapshot,
        source_fact_id: source_fact_id.to_string(),
    })])
}

fn context_head_selected(
    event: &EventRecord,
    source_fact_id: &str,
) -> Result<Vec<TraversalIntent>, StorageError> {
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

fn context_head_tombstoned(event: &EventRecord) -> Result<Vec<TraversalIntent>, StorageError> {
    let Some(head_ref) = find_object_ref(&event.objects, "context", "head") else {
        return Ok(Vec::new());
    };
    Ok(vec![TraversalIntent::EndAnchor(AnchorEndInput {
        anchor_ref: head_ref,
        ended_at_seq: event.seq,
    })])
}

fn task_artifact_emitted(
    event: &EventRecord,
    source_fact_id: &str,
) -> Result<Vec<TraversalIntent>, StorageError> {
    let Some(task_run) = find_object_ref(&event.objects, "execution", "task_run") else {
        return Ok(Vec::new());
    };
    let Some(artifact) = find_object_ref(&event.objects, "execution", "artifact") else {
        return Ok(Vec::new());
    };
    let Some(slot_ref) = find_object_ref(&event.objects, "execution", "artifact_slot") else {
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
