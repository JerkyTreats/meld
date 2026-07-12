use std::collections::{BTreeMap, BTreeSet};

use serde_json::from_value;

use meld_events::error::EventAuthorityError;

use crate::error::StorageError;
use crate::events::store::EventStore;
use crate::events::{EventPage, EventRecord, LedgerCursor, LedgerIdentity, ReplayRequest};
use crate::task::ExecutionTaskEventData;

/// Identity-bearing bounded replay required by the execution projection.
pub trait ExecutionProjectionReplaySource {
    /// Ledger whose records the source returns.
    fn ledger_identity(&self) -> LedgerIdentity;

    /// Replays one bounded page after an identity-bearing cursor.
    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError>;
}

impl ExecutionProjectionReplaySource for meld_events::EventReplayCapability {
    fn ledger_identity(&self) -> LedgerIdentity {
        meld_events::EventReplayCapability::ledger_identity(self)
    }

    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        meld_events::EventReplayCapability::replay(self, request)
    }
}

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
    /// Replays one identity-checked bounded authority page.
    pub fn replay_from_source(
        source: &impl ExecutionProjectionReplaySource,
        cursor: LedgerCursor,
        limit: usize,
    ) -> Result<Self, EventAuthorityError> {
        if cursor.ledger_id != source.ledger_identity() {
            return Err(EventAuthorityError::IdentityMismatch {
                expected: source.ledger_identity(),
                actual: cursor.ledger_id,
            });
        }
        let page = source.replay(ReplayRequest { cursor, limit })?;
        if page.ledger_id != source.ledger_identity() {
            return Err(EventAuthorityError::IdentityMismatch {
                expected: source.ledger_identity(),
                actual: page.ledger_id,
            });
        }
        let mut projection = Self::default();
        for event in page.records {
            projection
                .apply(&event)
                .map_err(|error| EventAuthorityError::Internal {
                    message: error.to_string(),
                })?;
        }
        Ok(projection)
    }

    /// TODO compat-shim: E5 removes raw-store replay after
    /// `product_event_authority_cutover` proves projection parity through
    /// `replay_from_source`.
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
    use crate::events::{AppendMode, EventAuthority, EventAuthorityOpenOptions, EventEnvelope};
    use serde_json::json;

    #[test]
    fn task_events_drive_projection_state() {
        let mut projection = ExecutionProjection::default();

        let requested = EventRecord::from_envelope(
            EventEnvelope::new_domain(
                "2026-01-01T00:00:00.000Z".to_string(),
                "session",
                "execution",
                "run_a",
                "execution.task.requested",
                None,
                json!({
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
            ),
            1,
        );

        let mut succeeded = requested.clone();
        succeeded.seq = 2;
        succeeded.envelope_mut().event_type = "execution.task.succeeded".to_string();

        projection.apply(&requested).unwrap();
        projection.apply(&succeeded).unwrap();

        assert!(projection.active_tasks.is_empty());
        assert!(projection.completed_tasks.contains("run_a"));
        assert_eq!(projection.last_applied_seq, 2);
    }

    #[test]
    fn authority_replay_binds_projection_to_one_ledger() {
        let authority = EventAuthority::open(
            sled::Config::new().temporary(true).open().unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let ledger_id = authority.ledger_identity();
        authority
            .append_capability()
            .append_durable(
                EventEnvelope::new_domain(
                    "2026-01-01T00:00:00.000Z".to_string(),
                    "session",
                    "execution",
                    "run_a",
                    "execution.task.started",
                    None,
                    json!({
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
                ),
                AppendMode::Plain,
            )
            .unwrap();
        let source = authority.replay_capability();

        let projection = ExecutionProjection::replay_from_source(
            &source,
            LedgerCursor {
                ledger_id,
                after_seq: 0,
            },
            16,
        )
        .unwrap();
        assert!(projection.active_tasks.contains("run_a"));

        let error = ExecutionProjection::replay_from_source(
            &source,
            LedgerCursor {
                ledger_id: LedgerIdentity::new(),
                after_seq: 0,
            },
            16,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            EventAuthorityError::IdentityMismatch { .. }
        ));
    }
}
