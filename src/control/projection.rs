use std::collections::{BTreeMap, BTreeSet};

use serde_json::from_value;

use crate::error::StorageError;
use crate::events::{EventRecord, EventStore};
use crate::task::ExecutionTaskEventData;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExecutionProjection {
    pub active_tasks: BTreeSet<String>,
    pub blocked_tasks: BTreeSet<String>,
    pub completed_tasks: BTreeSet<String>,
    pub failed_tasks: BTreeSet<String>,
    pub artifacts_by_task_run: BTreeMap<String, BTreeSet<String>>,
    pub last_applied_seq: u64,
}

impl ExecutionProjection {
    pub fn replay_from_store(store: &EventStore, after_seq: u64) -> Result<Self, StorageError> {
        let mut projection = Self::default();
        for event in store.read_all_events_after(after_seq)? {
            projection.apply(&event)?;
        }
        Ok(projection)
    }

    pub fn apply(&mut self, event: &EventRecord) -> Result<(), StorageError> {
        if event.domain_id != "execution" {
            return Ok(());
        }

        self.last_applied_seq = self.last_applied_seq.max(event.seq);
        match event.event_type.as_str() {
            "execution.task.requested" | "execution.task.started" | "execution.task.progressed" => {
                let data = parse_task_event_data(event)?;
                self.active_tasks.insert(data.task_run_id.clone());
                self.blocked_tasks.remove(&data.task_run_id);
                self.completed_tasks.remove(&data.task_run_id);
                self.failed_tasks.remove(&data.task_run_id);
            }
            "execution.task.blocked" => {
                let data = parse_task_event_data(event)?;
                self.active_tasks.insert(data.task_run_id.clone());
                self.blocked_tasks.insert(data.task_run_id);
            }
            "execution.task.succeeded" => {
                let data = parse_task_event_data(event)?;
                self.active_tasks.remove(&data.task_run_id);
                self.blocked_tasks.remove(&data.task_run_id);
                self.failed_tasks.remove(&data.task_run_id);
                self.completed_tasks.insert(data.task_run_id);
            }
            "execution.task.failed" | "execution.task.cancelled" => {
                let data = parse_task_event_data(event)?;
                self.active_tasks.remove(&data.task_run_id);
                self.blocked_tasks.remove(&data.task_run_id);
                self.completed_tasks.remove(&data.task_run_id);
                self.failed_tasks.insert(data.task_run_id);
            }
            "execution.task.artifact_emitted" => {
                let data = parse_task_event_data(event)?;
                if let Some(artifact_id) = data.artifact_id {
                    self.artifacts_by_task_run
                        .entry(data.task_run_id)
                        .or_default()
                        .insert(artifact_id);
                }
            }
            "execution.control.generation_started"
            | "execution.control.level_started"
            | "execution.control.level_completed"
            | "execution.control.node_started"
            | "execution.control.node_completed"
            | "execution.control.node_failed"
            | "execution.control.generation_failed"
            | "execution.control.generation_completed"
            | "execution.workflow.turn_started"
            | "execution.workflow.turn_completed"
            | "execution.workflow.turn_failed" => {}
            _ => {}
        }
        Ok(())
    }
}

fn parse_task_event_data(event: &EventRecord) -> Result<ExecutionTaskEventData, StorageError> {
    from_value(event.data.clone()).map_err(|err| {
        StorageError::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            err.to_string(),
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn task_events_drive_projection_state() {
        let mut projection = ExecutionProjection::default();

        let requested = EventRecord {
            ts: "2026-01-01T00:00:00.000Z".to_string(),
            recorded_at: "2026-01-01T00:00:00.000Z".to_string(),
            session: "session".to_string(),
            seq: 1,
            domain_id: "execution".to_string(),
            stream_id: "run_a".to_string(),
            event_type: "execution.task.requested".to_string(),
            occurred_at: None,
            content_hash: None,
            objects: Vec::new(),
            relations: Vec::new(),
            data: json!({
                "task_id": "task_a",
                "task_run_id": "run_a",
                "capability_instance_id": null,
                "invocation_id": null,
                "artifact_id": null,
                "artifact_type_id": null,
                "attempt_index": null,
                "ready_count": null,
                "running_count": null,
                "blocked_reason": null,
                "error": null
            }),
        };

        let succeeded = EventRecord {
            seq: 2,
            event_type: "execution.task.succeeded".to_string(),
            ..requested.clone()
        };

        projection.apply(&requested).unwrap();
        projection.apply(&succeeded).unwrap();

        assert!(projection.active_tasks.is_empty());
        assert!(projection.completed_tasks.contains("run_a"));
        assert_eq!(projection.last_applied_seq, 2);
    }
}
