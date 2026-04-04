//! Task-local execution event records.

use serde::{Deserialize, Serialize};

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
