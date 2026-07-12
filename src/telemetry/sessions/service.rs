//! Compatibility facade for session lifecycle and canonical event emission.

use std::sync::Arc;

use meld_events::error::EventAuthorityError;
use serde_json::Value;
use tracing::warn;

use crate::error::{ApiError, StorageError};
use crate::events::{AppendMode, EventAppendCapability, EventEnvelope};
use crate::session as lifecycle;
use crate::session::events::{session_ended_envelope, session_started_envelope};

#[derive(Clone)]
pub struct ProgressRuntime {
    append: EventAppendCapability,
    sessions: Arc<lifecycle::SessionRuntime>,
}

impl ProgressRuntime {
    /// Constructs the production facade from product-resolved capabilities.
    pub fn from_capabilities(
        append: EventAppendCapability,
        sessions: Arc<lifecycle::SessionRuntime>,
    ) -> Self {
        Self { append, sessions }
    }

    pub fn start_command_session(&self, command_name: String) -> Result<String, ApiError> {
        let session_id = self.sessions.start_command_session(command_name.clone())?;
        self.append
            .append_durable(
                session_started_envelope(&session_id, &command_name),
                AppendMode::Plain,
            )
            .map_err(authority_api_error)?;
        Ok(session_id)
    }

    pub fn finish_command_session(
        &self,
        session_id: &str,
        success: bool,
        error: Option<String>,
    ) -> Result<(), ApiError> {
        let status = if success { "completed" } else { "failed" };
        self.append
            .append_durable(
                session_ended_envelope(session_id, status, error.clone()),
                AppendMode::Plain,
            )
            .map_err(authority_api_error)?;
        self.sessions
            .finish_command_session(session_id, success, error)?;
        Ok(())
    }

    pub fn emit_event(
        &self,
        session_id: &str,
        event_type: &str,
        data: Value,
    ) -> Result<(), ApiError> {
        self.emit_envelope(EventEnvelope::with_now(
            session_id.to_string(),
            event_type,
            data,
        ))
    }

    pub fn emit_domain_event(
        &self,
        session_id: &str,
        domain_id: &str,
        stream_id: &str,
        event_type: &str,
        content_hash: Option<String>,
        data: Value,
    ) -> Result<(), ApiError> {
        self.emit_envelope(EventEnvelope::with_now_domain(
            session_id.to_string(),
            domain_id.to_string(),
            stream_id.to_string(),
            event_type,
            content_hash,
            data,
        ))
    }

    pub fn emit_envelope(&self, envelope: crate::events::EventEnvelope) -> Result<(), ApiError> {
        self.append
            .append_durable(envelope, AppendMode::Plain)
            .map(|_| ())
            .map_err(authority_api_error)
    }

    pub fn emit_envelope_idempotent(
        &self,
        envelope: crate::events::EventEnvelope,
    ) -> Result<(), ApiError> {
        self.append
            .append_durable(envelope, AppendMode::Idempotent)
            .map(|_| ())
            .map_err(authority_api_error)
    }

    /// Enqueues without waiting for durability; readers synchronize through
    /// [`ProgressRuntime::barrier`].
    pub fn emit_event_best_effort(&self, session_id: &str, event_type: &str, data: Value) {
        self.emit_envelope_best_effort(EventEnvelope::with_now(
            session_id.to_string(),
            event_type,
            data,
        ));
    }

    /// Enqueues without waiting for durability; readers synchronize through
    /// [`ProgressRuntime::barrier`].
    pub fn emit_domain_event_best_effort(
        &self,
        session_id: &str,
        domain_id: &str,
        stream_id: &str,
        event_type: &str,
        content_hash: Option<String>,
        data: Value,
    ) {
        self.emit_envelope_best_effort(EventEnvelope::with_now_domain(
            session_id.to_string(),
            domain_id.to_string(),
            stream_id.to_string(),
            event_type,
            content_hash,
            data,
        ));
    }

    /// Enqueues without waiting for durability; readers synchronize through
    /// [`ProgressRuntime::barrier`].
    pub fn emit_envelope_best_effort(&self, envelope: crate::events::EventEnvelope) {
        self.append_best_effort(envelope, AppendMode::Plain);
    }

    /// Enqueues without waiting for durability; readers synchronize through
    /// [`ProgressRuntime::barrier`].
    pub fn emit_envelope_idempotent_best_effort(&self, envelope: crate::events::EventEnvelope) {
        self.append_best_effort(envelope, AppendMode::Idempotent);
    }

    pub fn mark_interrupted_sessions(&self) -> Result<usize, ApiError> {
        let changed = self.sessions.mark_interrupted_sessions()?;
        self.sessions.store().flush()?;
        Ok(changed)
    }

    pub fn prune(
        &self,
        policy: crate::telemetry::sessions::policy::Policy,
    ) -> Result<usize, ApiError> {
        let pruned = self.sessions.prune(policy)?;
        self.sessions.store().flush()?;
        Ok(pruned)
    }

    /// Blocks until every previously enqueued emit has reached the store,
    /// the synchronization point for observing best-effort emissions.
    pub fn barrier(&self) -> Result<(), ApiError> {
        self.append.barrier().map_err(authority_api_error)
    }

    pub fn list_sessions(&self) -> Result<Vec<crate::session::SessionRecord>, ApiError> {
        self.sessions
            .store()
            .list_sessions()
            .map_err(ApiError::from)
    }

    pub fn get_session(
        &self,
        session_id: &str,
    ) -> Result<Option<crate::session::SessionRecord>, ApiError> {
        self.sessions
            .store()
            .get_session(session_id)
            .map_err(ApiError::from)
    }

    fn append_best_effort(&self, envelope: EventEnvelope, mode: AppendMode) {
        let session_id = envelope.session.clone();
        let event_type = envelope.event_type.clone();
        if let Err(error) = self.append.append_best_effort(envelope, mode) {
            warn!(
                session_id = %session_id,
                event_type = %event_type,
                error = %error,
                "failed to enqueue event"
            );
        }
    }
}

fn authority_api_error(error: EventAuthorityError) -> ApiError {
    ApiError::StorageError(authority_storage_error(error))
}

fn authority_storage_error(error: EventAuthorityError) -> StorageError {
    match error {
        EventAuthorityError::InvalidRequest { message } => StorageError::InvalidPath(message),
        EventAuthorityError::IdentityMismatch { expected, actual } => {
            StorageError::LedgerIdentityMismatch {
                expected: expected.to_string(),
                actual: actual.to_string(),
            }
        }
        EventAuthorityError::DuplicateAuthorityBinding { ledger_id } => {
            StorageError::EventAuthorityUnavailable(format!(
                "duplicate writable binding for ledger {ledger_id}"
            ))
        }
        EventAuthorityError::RetentionGap {
            after_seq,
            retained_from,
            ..
        } => StorageError::RetentionGap {
            after_seq,
            retained_from,
        },
        EventAuthorityError::Backpressure { message } => StorageError::Backpressure(message),
        EventAuthorityError::DurabilityIndeterminate { message } => {
            StorageError::DurabilityIndeterminate(message)
        }
        EventAuthorityError::Unavailable { message } => {
            StorageError::EventAuthorityUnavailable(message)
        }
        EventAuthorityError::MigrationConflict { message } => {
            StorageError::MigrationConflict(message)
        }
        EventAuthorityError::Persistence { message }
        | EventAuthorityError::Internal { message }
        | EventAuthorityError::CorruptPersistedIdentity { message } => {
            StorageError::IoError(std::io::Error::other(message))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventAuthority, EventAuthorityOpenOptions, LedgerCursor, ReplayRequest};
    use crate::session::SessionStatus;
    use serde_json::json;

    #[test]
    fn injected_capability_owns_canonical_event_emission() {
        let event_db = sled::Config::new().temporary(true).open().unwrap();
        let authority =
            EventAuthority::open(event_db, EventAuthorityOpenOptions::default()).unwrap();
        let session_db = sled::Config::new().temporary(true).open().unwrap();
        let sessions = Arc::new(lifecycle::SessionRuntime::new(
            lifecycle::SessionStore::shared(session_db).unwrap(),
        ));
        let runtime = ProgressRuntime::from_capabilities(
            authority.append_capability(),
            Arc::clone(&sessions),
        );

        let session_id = runtime.start_command_session("scan".to_string()).unwrap();
        runtime.emit_event_best_effort(&session_id, "scan.progress", json!({ "files": 3 }));
        runtime.barrier().unwrap();
        runtime
            .finish_command_session(&session_id, true, None)
            .unwrap();

        let page = authority
            .replay_capability()
            .replay(ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id: authority.ledger_identity(),
                    after_seq: 0,
                },
                limit: 10,
            })
            .unwrap();
        let event_types: Vec<&str> = page
            .records
            .iter()
            .map(|record| record.event_type.as_str())
            .collect();
        assert_eq!(
            event_types,
            vec!["session_started", "scan.progress", "session_ended"]
        );
        assert_eq!(
            sessions
                .store()
                .get_session(&session_id)
                .unwrap()
                .unwrap()
                .status,
            SessionStatus::Completed
        );
    }

    #[test]
    fn interrupted_session_repair_flushes_session_storage_directly() {
        let root = tempfile::TempDir::new().unwrap();
        let session_path = root.path().join("sessions");
        let event_db = sled::Config::new().temporary(true).open().unwrap();
        let authority =
            EventAuthority::open(event_db, EventAuthorityOpenOptions::default()).unwrap();
        let session_db = sled::open(&session_path).unwrap();
        let session_store = lifecycle::SessionStore::shared(session_db.clone()).unwrap();
        let sessions = Arc::new(lifecycle::SessionRuntime::new(session_store.clone()));
        let runtime = ProgressRuntime::from_capabilities(authority.append_capability(), sessions);

        let session_id = runtime.start_command_session("watch".to_string()).unwrap();
        assert_eq!(runtime.mark_interrupted_sessions().unwrap(), 1);
        drop(runtime);
        drop(session_store);
        drop(session_db);

        let reopened = lifecycle::SessionStore::new(sled::open(&session_path).unwrap()).unwrap();
        assert_eq!(
            reopened.get_session(&session_id).unwrap().unwrap().status,
            SessionStatus::Interrupted
        );
    }
}
