use serde::{Deserialize, Serialize};
use serde_json::json;

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
    }
}
