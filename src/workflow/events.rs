use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::telemetry::contracts::{DomainObjectRef, EventRelation};
use crate::telemetry::events::ProgressEnvelope;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionWorkflowTurnEventData {
    pub workflow_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub turn_seq: u32,
    pub node_id: String,
    pub path: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub attempt: usize,
    pub plan_id: Option<String>,
    pub level_index: Option<usize>,
    pub final_frame_id: Option<String>,
    pub error: Option<String>,
}

impl From<crate::telemetry::WorkflowTurnEventData> for ExecutionWorkflowTurnEventData {
    fn from(value: crate::telemetry::WorkflowTurnEventData) -> Self {
        Self {
            workflow_id: value.workflow_id,
            thread_id: value.thread_id,
            turn_id: value.turn_id,
            turn_seq: value.turn_seq,
            node_id: value.node_id,
            path: value.path,
            agent_id: value.agent_id,
            provider_name: value.provider_name,
            frame_type: value.frame_type,
            attempt: value.attempt,
            plan_id: value.plan_id,
            level_index: value.level_index,
            final_frame_id: value.final_frame_id,
            error: value.error,
        }
    }
}

fn workflow_envelope(
    session_id: &str,
    event_type: &str,
    data: ExecutionWorkflowTurnEventData,
) -> ProgressEnvelope {
    ProgressEnvelope::with_now_domain(
        session_id.to_string(),
        "execution".to_string(),
        data.workflow_id.clone(),
        event_type.to_string(),
        None,
        json!(data),
    )
    .with_graph(workflow_objects(&data), workflow_relations(&data))
}

pub fn workflow_turn_started_envelope(
    session_id: &str,
    data: ExecutionWorkflowTurnEventData,
) -> ProgressEnvelope {
    workflow_envelope(session_id, "execution.workflow.turn_started", data)
}

pub fn workflow_turn_completed_envelope(
    session_id: &str,
    data: ExecutionWorkflowTurnEventData,
) -> ProgressEnvelope {
    workflow_envelope(session_id, "execution.workflow.turn_completed", data)
}

pub fn workflow_turn_failed_envelope(
    session_id: &str,
    data: ExecutionWorkflowTurnEventData,
) -> ProgressEnvelope {
    workflow_envelope(session_id, "execution.workflow.turn_failed", data)
}

fn workflow_objects(data: &ExecutionWorkflowTurnEventData) -> Vec<DomainObjectRef> {
    let mut objects = vec![
        workflow_ref(&data.workflow_id),
        workflow_thread_ref(&data.thread_id),
        workflow_turn_ref(&data.thread_id, &data.turn_id, data.turn_seq),
    ];
    if let Some(node) = workspace_node_ref(&data.node_id) {
        objects.push(node);
    }
    if let Some(final_frame_id) = &data.final_frame_id {
        objects.push(frame_ref(final_frame_id));
    }
    if let Some(plan_id) = &data.plan_id {
        objects.push(plan_ref(plan_id));
    }
    objects
}

fn workflow_relations(data: &ExecutionWorkflowTurnEventData) -> Vec<EventRelation> {
    let thread = workflow_thread_ref(&data.thread_id);
    let workflow = workflow_ref(&data.workflow_id);
    let turn = workflow_turn_ref(&data.thread_id, &data.turn_id, data.turn_seq);
    let mut relations = vec![
        EventRelation::new("belongs_to", thread.clone(), workflow)
            .expect("workflow thread relation should be valid"),
        EventRelation::new("belongs_to", turn.clone(), thread)
            .expect("workflow turn thread relation should be valid"),
    ];
    if let Some(node) = workspace_node_ref(&data.node_id) {
        relations.push(
            EventRelation::new("targets", turn.clone(), node)
                .expect("workflow target relation should be valid"),
        );
    }
    if let Some(final_frame_id) = &data.final_frame_id {
        relations.push(
            EventRelation::new("produced", turn.clone(), frame_ref(final_frame_id))
                .expect("workflow produced relation should be valid"),
        );
    }
    if let Some(plan_id) = &data.plan_id {
        relations.push(
            EventRelation::new("belongs_to", turn, plan_ref(plan_id))
                .expect("workflow plan relation should be valid"),
        );
    }
    relations
}

fn workflow_ref(workflow_id: &str) -> DomainObjectRef {
    DomainObjectRef::new("execution", "workflow", workflow_id)
        .expect("workflow ref should be valid")
}

fn workflow_thread_ref(thread_id: &str) -> DomainObjectRef {
    DomainObjectRef::new("execution", "workflow_thread", thread_id)
        .expect("workflow thread ref should be valid")
}

fn workflow_turn_ref(thread_id: &str, turn_id: &str, turn_seq: u32) -> DomainObjectRef {
    DomainObjectRef::new(
        "execution",
        "workflow_turn",
        format!("{thread_id}::{turn_id}::{turn_seq}"),
    )
    .expect("workflow turn ref should be valid")
}

fn workspace_node_ref(node_id: &str) -> Option<DomainObjectRef> {
    if node_id.trim().is_empty() {
        return None;
    }
    Some(
        DomainObjectRef::new("workspace_fs", "node", node_id)
            .expect("workspace node ref should be valid"),
    )
}

fn frame_ref(frame_id: &str) -> DomainObjectRef {
    DomainObjectRef::new("context", "frame", frame_id)
        .expect("frame ref should be valid")
}

fn plan_ref(plan_id: &str) -> DomainObjectRef {
    DomainObjectRef::new("execution", "plan", plan_id)
        .expect("plan ref should be valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_event_uses_workflow_stream() {
        let envelope = workflow_turn_started_envelope(
            "session_a",
            ExecutionWorkflowTurnEventData {
                workflow_id: "wf_a".to_string(),
                thread_id: "thread_a".to_string(),
                turn_id: "turn_a".to_string(),
                turn_seq: 1,
                node_id: "node_a".to_string(),
                path: "/tmp/a.md".to_string(),
                agent_id: "writer".to_string(),
                provider_name: "mock".to_string(),
                frame_type: "analysis".to_string(),
                attempt: 1,
                plan_id: Some("plan_a".to_string()),
                level_index: Some(0),
                final_frame_id: None,
                error: None,
            },
        );

        assert_eq!(envelope.domain_id, "execution");
        assert_eq!(envelope.stream_id, "wf_a");
        assert_eq!(envelope.event_type, "execution.workflow.turn_started");
        assert!(envelope
            .objects
            .iter()
            .any(|object| object.object_kind == "workflow"));
        assert!(envelope
            .objects
            .iter()
            .any(|object| object.object_kind == "workflow_thread"));
        assert!(envelope
            .objects
            .iter()
            .any(|object| object.object_kind == "workflow_turn"));
        assert!(envelope
            .objects
            .iter()
            .any(|object| object.object_kind == "node"));
    }

    #[test]
    fn workflow_turn_completed_links_turn_node_and_frame() {
        let envelope = workflow_turn_completed_envelope(
            "session_a",
            ExecutionWorkflowTurnEventData {
                workflow_id: "wf_a".to_string(),
                thread_id: "thread_a".to_string(),
                turn_id: "turn_a".to_string(),
                turn_seq: 1,
                node_id: "node_a".to_string(),
                path: "/tmp/a.md".to_string(),
                agent_id: "writer".to_string(),
                provider_name: "mock".to_string(),
                frame_type: "analysis".to_string(),
                attempt: 1,
                plan_id: Some("plan_a".to_string()),
                level_index: Some(0),
                final_frame_id: Some("frame_a".to_string()),
                error: None,
            },
        );

        assert!(envelope
            .relations
            .iter()
            .any(|relation| relation.relation_type == "targets"));
        assert!(envelope
            .relations
            .iter()
            .any(|relation| relation.relation_type == "produced"));
        assert!(envelope
            .relations
            .iter()
            .any(|relation| relation.relation_type == "belongs_to"));
    }
}
