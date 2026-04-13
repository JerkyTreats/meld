use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::telemetry::contracts::{DomainObjectRef, EventRelation};
use crate::telemetry::events::ProgressEnvelope;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationStartedEventData {
    pub plan_id: String,
    pub total_levels: usize,
    pub total_nodes: usize,
    pub target_path: String,
    pub program_kind: String,
    pub workflow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelStartedEventData {
    pub plan_id: String,
    pub level_index: usize,
    pub total_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelCompletedEventData {
    pub plan_id: String,
    pub level_index: usize,
    pub generated_count: usize,
    pub failed_count: usize,
    pub total_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeStartedEventData {
    pub plan_id: String,
    pub level_index: usize,
    pub node_id: String,
    pub path: String,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub program_kind: String,
    pub workflow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeCompletedEventData {
    pub plan_id: String,
    pub level_index: usize,
    pub node_id: String,
    pub path: String,
    pub frame_id: String,
    pub program_kind: String,
    pub workflow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeFailedEventData {
    pub plan_id: String,
    pub level_index: usize,
    pub node_id: String,
    pub path: String,
    pub error: String,
    pub program_kind: String,
    pub workflow_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationFailedEventData {
    pub plan_id: String,
    pub reason: String,
    pub failed_level_index: Option<usize>,
    pub total_generated: usize,
    pub total_failed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationCompletedEventData {
    pub plan_id: String,
    pub total_generated: usize,
    pub total_failed: usize,
}

fn control_envelope(
    session_id: &str,
    plan_id: &str,
    event_type: &str,
    data: serde_json::Value,
) -> ProgressEnvelope {
    ProgressEnvelope::with_now_domain(
        session_id.to_string(),
        "execution".to_string(),
        plan_id.to_string(),
        event_type.to_string(),
        None,
        data,
    )
}

pub fn generation_started_envelope(
    session_id: &str,
    data: GenerationStartedEventData,
) -> ProgressEnvelope {
    control_envelope(
        session_id,
        &data.plan_id,
        "execution.control.generation_started",
        json!(data),
    )
}

pub fn level_started_envelope(session_id: &str, data: LevelStartedEventData) -> ProgressEnvelope {
    control_envelope(
        session_id,
        &data.plan_id,
        "execution.control.level_started",
        json!(data),
    )
}

pub fn level_completed_envelope(
    session_id: &str,
    data: LevelCompletedEventData,
) -> ProgressEnvelope {
    control_envelope(
        session_id,
        &data.plan_id,
        "execution.control.level_completed",
        json!(data),
    )
}

pub fn node_started_envelope(session_id: &str, data: NodeStartedEventData) -> ProgressEnvelope {
    control_envelope(
        session_id,
        &data.plan_id,
        "execution.control.node_started",
        json!(data),
    )
}

pub fn node_completed_envelope(
    session_id: &str,
    data: NodeCompletedEventData,
) -> ProgressEnvelope {
    control_envelope(
        session_id,
        &data.plan_id,
        "execution.control.node_completed",
        json!(data),
    )
    .with_graph(
        vec![
            plan_ref(&data.plan_id),
            workspace_node_ref(&data.node_id),
            frame_ref(&data.frame_id),
        ],
        vec![
            EventRelation::new(
                "targets",
                plan_ref(&data.plan_id),
                workspace_node_ref(&data.node_id),
            )
            .expect("control target relation should be valid"),
            EventRelation::new(
                "produced",
                workspace_node_ref(&data.node_id),
                frame_ref(&data.frame_id),
            )
            .expect("control produced relation should be valid"),
        ],
    )
}

pub fn node_failed_envelope(session_id: &str, data: NodeFailedEventData) -> ProgressEnvelope {
    control_envelope(
        session_id,
        &data.plan_id,
        "execution.control.node_failed",
        json!(data),
    )
    .with_graph(
        vec![plan_ref(&data.plan_id), workspace_node_ref(&data.node_id)],
        vec![
            EventRelation::new(
                "targets",
                plan_ref(&data.plan_id),
                workspace_node_ref(&data.node_id),
            )
            .expect("control target relation should be valid"),
        ],
    )
}

pub fn generation_failed_envelope(
    session_id: &str,
    data: GenerationFailedEventData,
) -> ProgressEnvelope {
    control_envelope(
        session_id,
        &data.plan_id,
        "execution.control.generation_failed",
        json!(data),
    )
}

pub fn generation_completed_envelope(
    session_id: &str,
    data: GenerationCompletedEventData,
) -> ProgressEnvelope {
    control_envelope(
        session_id,
        &data.plan_id,
        "execution.control.generation_completed",
        json!(data),
    )
}

fn plan_ref(plan_id: &str) -> DomainObjectRef {
    DomainObjectRef::new("execution", "plan", plan_id).expect("plan ref should be valid")
}

fn workspace_node_ref(node_id: &str) -> DomainObjectRef {
    DomainObjectRef::new("workspace_fs", "node", node_id)
        .expect("workspace node ref should be valid")
}

fn frame_ref(frame_id: &str) -> DomainObjectRef {
    DomainObjectRef::new("context", "frame", frame_id).expect("frame ref should be valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_event_uses_execution_domain_and_plan_stream() {
        let envelope = generation_started_envelope(
            "session_a",
            GenerationStartedEventData {
                plan_id: "plan_a".to_string(),
                total_levels: 2,
                total_nodes: 3,
                target_path: "/tmp".to_string(),
                program_kind: "workflow".to_string(),
                workflow_id: Some("wf_a".to_string()),
            },
        );

        assert_eq!(envelope.domain_id, "execution");
        assert_eq!(envelope.stream_id, "plan_a");
        assert_eq!(envelope.event_type, "execution.control.generation_started");
    }

    #[test]
    fn control_node_completed_emits_workspace_and_frame_refs() {
        let envelope = node_completed_envelope(
            "session_a",
            NodeCompletedEventData {
                plan_id: "plan_a".to_string(),
                level_index: 0,
                node_id: "node_a".to_string(),
                path: "/tmp/a".to_string(),
                frame_id: "frame_a".to_string(),
                program_kind: "workflow".to_string(),
                workflow_id: None,
            },
        );

        assert_eq!(envelope.objects.len(), 3);
        assert_eq!(envelope.objects[0].object_kind, "plan");
        assert_eq!(envelope.objects[1].object_kind, "node");
        assert_eq!(envelope.objects[2].object_kind, "frame");
        assert_eq!(envelope.relations.len(), 2);
        assert_eq!(envelope.relations[0].relation_type, "targets");
        assert_eq!(envelope.relations[1].relation_type, "produced");
    }
}
