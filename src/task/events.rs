//! Task-local execution event records.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::telemetry::events::ProgressEnvelope;

/// Structured task event emitted by the task executor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEvent {
    pub event_type: String,
    pub task_id: String,
    pub task_run_id: String,
    pub capability_instance_id: Option<String>,
    pub invocation_id: Option<String>,
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

pub fn build_execution_task_envelope(
    session_id: &str,
    event: &TaskEvent,
) -> Option<ProgressEnvelope> {
    let event_type = canonical_task_event_type(&event.event_type)?;
    let data = ExecutionTaskEventData::from(event);
    Some(ProgressEnvelope::with_now_domain(
        session_id.to_string(),
        "execution".to_string(),
        event.task_run_id.clone(),
        event_type.to_string(),
        None,
        json!(data),
    ))
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
}
