//! Task-local execution event records.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::events::{DomainObjectRef, EventEnvelope, EventRelation};
use crate::task::TaskInitializationPayload;

/// Structured task event emitted by the task executor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEvent {
    pub event_type: String,
    pub task_id: String,
    pub task_run_id: String,
    pub capability_instance_id: Option<String>,
    pub invocation_id: Option<String>,
    pub target_node_id: Option<String>,
    pub artifact_id: Option<String>,
    pub artifact_type_id: Option<String>,
    pub attempt_index: Option<u32>,
    pub ready_count: Option<usize>,
    pub running_count: Option<usize>,
    pub blocked_reason: Option<String>,
    pub error: Option<String>,
}

impl TaskEvent {
    /// Creates one structured task event.
    pub fn new(
        event_type: impl Into<String>,
        task_id: impl Into<String>,
        task_run_id: impl Into<String>,
    ) -> Self {
        Self {
            event_type: event_type.into(),
            task_id: task_id.into(),
            task_run_id: task_run_id.into(),
            capability_instance_id: None,
            invocation_id: None,
            target_node_id: None,
            artifact_id: None,
            artifact_type_id: None,
            attempt_index: None,
            ready_count: None,
            running_count: None,
            blocked_reason: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionTaskEventData {
    pub task_id: String,
    pub task_run_id: String,
    pub capability_instance_id: Option<String>,
    pub invocation_id: Option<String>,
    pub target_node_id: Option<String>,
    pub artifact_id: Option<String>,
    pub artifact_type_id: Option<String>,
    pub attempt_index: Option<u32>,
    pub ready_count: Option<usize>,
    pub running_count: Option<usize>,
    pub blocked_reason: Option<String>,
    pub error: Option<String>,
}

impl From<&TaskEvent> for ExecutionTaskEventData {
    fn from(event: &TaskEvent) -> Self {
        Self {
            task_id: event.task_id.clone(),
            task_run_id: event.task_run_id.clone(),
            capability_instance_id: event.capability_instance_id.clone(),
            invocation_id: event.invocation_id.clone(),
            target_node_id: event.target_node_id.clone(),
            artifact_id: event.artifact_id.clone(),
            artifact_type_id: event.artifact_type_id.clone(),
            attempt_index: event.attempt_index,
            ready_count: event.ready_count,
            running_count: event.running_count,
            blocked_reason: event.blocked_reason.clone(),
            error: event.error.clone(),
        }
    }
}

pub fn canonical_task_event_type(event_type: &str) -> Option<&'static str> {
    match event_type {
        "task_requested" => Some("execution.task.requested"),
        "task_started" => Some("execution.task.started"),
        "task_progressed" => Some("execution.task.progressed"),
        "task_blocked" => Some("execution.task.blocked"),
        "task_succeeded" => Some("execution.task.succeeded"),
        "task_failed" => Some("execution.task.failed"),
        "task_cancelled" => Some("execution.task.cancelled"),
        "task_artifact_emitted" => Some("execution.task.artifact_emitted"),
        _ => None,
    }
}

pub fn target_node_id_from_init_payload(payload: &TaskInitializationPayload) -> Option<String> {
    payload
        .init_artifacts
        .iter()
        .find(|artifact| artifact.init_slot_id == "target_selector")
        .and_then(|artifact| artifact.content.get("node_id"))
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
}

pub fn build_execution_task_envelope(session_id: &str, event: &TaskEvent) -> Option<EventEnvelope> {
    let event_type = canonical_task_event_type(&event.event_type)?;
    let data = ExecutionTaskEventData::from(event);
    Some(
        EventEnvelope::with_now_domain(
            session_id.to_string(),
            "execution".to_string(),
            event.task_run_id.clone(),
            event_type.to_string(),
            None,
            json!(data),
        )
        .with_graph(
            task_event_objects(event_type, event),
            task_event_relations(event_type, event),
        ),
    )
}

fn task_event_objects(event_type: &str, event: &TaskEvent) -> Vec<DomainObjectRef> {
    let mut objects = vec![task_run_ref(&event.task_run_id)];
    if let Some(target_node_id) = &event.target_node_id {
        objects.push(workspace_node_ref(target_node_id));
    }

    if event_type == "execution.task.artifact_emitted" {
        objects.push(artifact_slot_ref(
            &event.task_run_id,
            artifact_type_id_or_default(event),
        ));
        if let Some(artifact_id) = &event.artifact_id {
            objects.push(artifact_ref(artifact_id));
        }
    }

    objects
}

fn task_event_relations(event_type: &str, event: &TaskEvent) -> Vec<EventRelation> {
    let mut relations = Vec::new();

    if let Some(target_node_id) = &event.target_node_id {
        relations.push(
            EventRelation::new(
                "targets",
                task_run_ref(&event.task_run_id),
                workspace_node_ref(target_node_id),
            )
            .expect("task target relation should be valid"),
        );
    }

    if event_type == "execution.task.artifact_emitted" {
        let Some(artifact_id) = event.artifact_id.as_ref() else {
            return relations;
        };
        let artifact_slot =
            artifact_slot_ref(&event.task_run_id, artifact_type_id_or_default(event));
        relations.push(
            EventRelation::new(
                "attached_to",
                artifact_slot.clone(),
                task_run_ref(&event.task_run_id),
            )
            .expect("task artifact slot relation should be valid"),
        );
        relations.push(
            EventRelation::new("selected", artifact_slot, artifact_ref(artifact_id))
                .expect("task artifact selected relation should be valid"),
        );
    }

    relations
}

fn task_run_ref(task_run_id: &str) -> DomainObjectRef {
    DomainObjectRef::new("execution", "task_run", task_run_id)
        .expect("task run ref should be valid")
}

fn artifact_ref(artifact_id: &str) -> DomainObjectRef {
    DomainObjectRef::new("execution", "artifact", artifact_id)
        .expect("artifact ref should be valid")
}

fn artifact_slot_ref(task_run_id: &str, artifact_type_id: &str) -> DomainObjectRef {
    DomainObjectRef::new(
        "execution",
        "artifact_slot",
        format!("{task_run_id}::{artifact_type_id}"),
    )
    .expect("artifact slot ref should be valid")
}

fn workspace_node_ref(node_id: &str) -> DomainObjectRef {
    DomainObjectRef::new("workspace_fs", "node", node_id)
        .expect("workspace node ref should be valid")
}

fn artifact_type_id_or_default(event: &TaskEvent) -> &str {
    event.artifact_type_id.as_deref().unwrap_or("artifact")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_event_maps_to_canonical_execution_event() {
        let mut event = TaskEvent::new("task_progressed", "task_a", "run_a");
        event.invocation_id = Some("invoke_a".to_string());

        let envelope = build_execution_task_envelope("session_a", &event).unwrap();

        assert_eq!(envelope.domain_id, "execution");
        assert_eq!(envelope.stream_id, "run_a");
        assert_eq!(envelope.event_type, "execution.task.progressed");
    }

    #[test]
    fn task_artifact_event_emits_task_and_artifact_refs() {
        let mut event = TaskEvent::new("task_artifact_emitted", "task_a", "run_a");
        event.artifact_id = Some("artifact_a".to_string());
        event.artifact_type_id = Some("frame_ref".to_string());
        event.target_node_id = Some("node_a".to_string());

        let envelope = build_execution_task_envelope("session_a", &event).unwrap();

        assert_eq!(envelope.objects.len(), 4);
        assert_eq!(envelope.objects[0].domain_id, "execution");
        assert_eq!(envelope.objects[0].object_kind, "task_run");
        assert!(envelope
            .objects
            .iter()
            .any(|object| object.object_kind == "node"));
        assert!(envelope
            .objects
            .iter()
            .any(|object| object.object_kind == "artifact_slot"));
        assert!(envelope
            .objects
            .iter()
            .any(|object| object.object_kind == "artifact"));
        assert_eq!(envelope.relations.len(), 3);
        assert!(envelope
            .relations
            .iter()
            .any(|relation| relation.relation_type == "targets"));
        assert!(envelope
            .relations
            .iter()
            .any(|relation| relation.relation_type == "attached_to"));
        assert!(envelope
            .relations
            .iter()
            .any(|relation| relation.relation_type == "selected"));
    }

    #[test]
    fn task_target_node_id_reads_from_init_payload() {
        let payload = TaskInitializationPayload {
            task_id: "task_a".to_string(),
            compiled_task_ref: "compiled".to_string(),
            init_artifacts: vec![crate::task::InitArtifactValue {
                init_slot_id: "target_selector".to_string(),
                artifact_type_id: "target_selector".to_string(),
                schema_version: 1,
                content: json!({ "node_id": "node_a", "path": "docs/a.md" }),
            }],
            task_run_context: crate::task::TaskRunContext {
                task_run_id: "run_a".to_string(),
                session_id: None,
                trigger: "test".to_string(),
            },
        };

        assert_eq!(
            target_node_id_from_init_payload(&payload).as_deref(),
            Some("node_a")
        );
    }
}
