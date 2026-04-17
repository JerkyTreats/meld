//! Compatibility facade for session lifecycle and canonical event emission.

use std::sync::Arc;

use serde_json::Value;
use tracing::warn;

use crate::error::ApiError;
use crate::events::{EventRuntime, EventStore};
use crate::session as lifecycle;
use crate::session::events::{session_ended_envelope, session_started_envelope};
use crate::telemetry::events::ProgressEnvelope;

#[derive(Clone)]
pub struct ProgressRuntime {
    events: Arc<EventRuntime>,
    sessions: Arc<lifecycle::SessionRuntime>,
}

impl ProgressRuntime {
    pub fn new(db: sled::Db) -> Result<Self, crate::error::StorageError> {
        let events = Arc::new(EventRuntime::new(db.clone())?);
        let session_store = Arc::new(lifecycle::SessionStore::new(db)?);
        let sessions = Arc::new(lifecycle::SessionRuntime::new(session_store));
        Ok(Self { events, sessions })
    }

    pub fn start_command_session(&self, command_name: String) -> Result<String, ApiError> {
        let session_id = self.sessions.start_command_session(command_name.clone())?;
        self.events
            .emit_envelope(session_started_envelope(&session_id, &command_name))?;
        Ok(session_id)
    }

    pub fn finish_command_session(
        &self,
        session_id: &str,
        success: bool,
        error: Option<String>,
    ) -> Result<(), ApiError> {
        let status = if success { "completed" } else { "failed" };
        self.events
            .emit_envelope(session_ended_envelope(session_id, status, error.clone()))?;
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
        self.events.emit_event(session_id, event_type, data)
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
        self.events.emit_domain_event(
            session_id,
            domain_id,
            stream_id,
            event_type,
            content_hash,
            data,
        )
    }

    pub fn emit_envelope(&self, envelope: ProgressEnvelope) -> Result<(), ApiError> {
        self.events.emit_envelope(envelope)
    }

    pub fn emit_event_best_effort(&self, session_id: &str, event_type: &str, data: Value) {
        if let Err(err) = self.emit_event(session_id, event_type, data) {
            warn!(
                session_id = %session_id,
                event_type = %event_type,
                error = %err,
                "failed to emit progress event"
            );
        }
    }

    pub fn emit_domain_event_best_effort(
        &self,
        session_id: &str,
        domain_id: &str,
        stream_id: &str,
        event_type: &str,
        content_hash: Option<String>,
        data: Value,
    ) {
        if let Err(err) = self.emit_domain_event(
            session_id,
            domain_id,
            stream_id,
            event_type,
            content_hash,
            data,
        ) {
            warn!(
                session_id = %session_id,
                domain_id = %domain_id,
                stream_id = %stream_id,
                event_type = %event_type,
                error = %err,
                "failed to emit domain event"
            );
        }
    }

    pub fn emit_envelope_best_effort(&self, envelope: ProgressEnvelope) {
        let session_id = envelope.session.clone();
        let event_type = envelope.event_type.clone();
        if let Err(err) = self.emit_envelope(envelope) {
            warn!(
                session_id = %session_id,
                event_type = %event_type,
                error = %err,
                "failed to emit envelope"
            );
        }
    }

    pub fn mark_interrupted_sessions(&self) -> Result<usize, ApiError> {
        let changed = self.sessions.mark_interrupted_sessions()?;
        self.events
            .store()
            .flush()
            .map_err(ApiError::StorageError)?;
        Ok(changed)
    }

    pub fn prune(
        &self,
        policy: crate::telemetry::sessions::policy::Policy,
    ) -> Result<usize, ApiError> {
        let pruned = self.sessions.prune(policy)?;
        self.events
            .store()
            .flush()
            .map_err(ApiError::StorageError)?;
        Ok(pruned)
    }

    pub fn store(&self) -> &EventStore {
        self.events.store()
    }
}
