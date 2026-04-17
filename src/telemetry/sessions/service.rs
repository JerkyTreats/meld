//! Session lifecycle service. No CLI dependency; takes command_name as String.

use std::sync::Arc;

use serde_json::Value;
use tracing::warn;

use crate::error::{ApiError, StorageError};
use crate::session as lifecycle;
use crate::session::events::{session_ended_envelope, session_started_envelope};
use crate::telemetry::events::ProgressEnvelope;
use crate::telemetry::routing::bus::ProgressBus;
use crate::telemetry::routing::ingestor::{EventIngestor, SharedIngestor};
use crate::telemetry::sinks::store::ProgressStore;

/// Runtime for session lifecycle and event emission. Holds store, bus, ingestor.
#[derive(Clone)]
pub struct ProgressRuntime {
    store: Arc<ProgressStore>,
    sessions: Arc<lifecycle::SessionRuntime>,
    bus: ProgressBus,
    ingestor: SharedIngestor,
}

impl ProgressRuntime {
    pub fn new(db: sled::Db) -> Result<Self, StorageError> {
        let store = ProgressStore::shared(db)?;
        let session_store = Arc::new(lifecycle::SessionStore::new(store.db().clone())?);
        let sessions = Arc::new(lifecycle::SessionRuntime::new(session_store));
        let (bus, rx) = ProgressBus::new_pair();
        let ingestor = SharedIngestor::new(EventIngestor::new(store.clone(), rx));
        Ok(Self {
            store,
            sessions,
            bus,
            ingestor,
        })
    }

    pub fn start_command_session(&self, command_name: String) -> Result<String, ApiError> {
        let session_id = self.sessions.start_command_session(command_name.clone())?;
        self.bus
            .emit_envelope(session_started_envelope(&session_id, &command_name))
            .map_err(to_api_error)?;
        self.ingestor.drain()?;
        self.store.flush()?;
        Ok(session_id)
    }

    pub fn finish_command_session(
        &self,
        session_id: &str,
        success: bool,
        error: Option<String>,
    ) -> Result<(), ApiError> {
        let status = if success { "completed" } else { "failed" };
        self.bus
            .emit_envelope(session_ended_envelope(session_id, status, error.clone()))
            .map_err(to_api_error)?;
        self.ingestor.drain()?;
        self.sessions
            .finish_command_session(session_id, success, error)?;
        self.store.flush()?;
        Ok(())
    }

    pub fn emit_event(
        &self,
        session_id: &str,
        event_type: &str,
        data: Value,
    ) -> Result<(), ApiError> {
        self.bus
            .emit(session_id.to_string(), event_type, data)
            .map_err(to_api_error)?;
        self.ingestor.drain()?;
        self.store.flush()?;
        Ok(())
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
        self.bus
            .emit_envelope(ProgressEnvelope::with_now_domain(
                session_id.to_string(),
                domain_id.to_string(),
                stream_id.to_string(),
                event_type.to_string(),
                content_hash,
                data,
            ))
            .map_err(to_api_error)?;
        self.ingestor.drain()?;
        self.store.flush()?;
        Ok(())
    }

    pub fn emit_envelope(&self, envelope: ProgressEnvelope) -> Result<(), ApiError> {
        self.bus.emit_envelope(envelope).map_err(to_api_error)?;
        self.ingestor.drain()?;
        self.store.flush()?;
        Ok(())
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
        self.store.flush()?;
        Ok(changed)
    }

    pub fn prune(
        &self,
        policy: crate::telemetry::sessions::policy::Policy,
    ) -> Result<usize, ApiError> {
        let pruned = self.sessions.prune(policy)?;
        self.store.flush()?;
        Ok(pruned)
    }

    pub fn store(&self) -> &ProgressStore {
        &self.store
    }
}

fn to_api_error(err: std::sync::mpsc::TrySendError<ProgressEnvelope>) -> ApiError {
    match err {
        std::sync::mpsc::TrySendError::Full(_) => ApiError::StorageError(
            StorageError::Backpressure("progress bus is full".to_string()),
        ),
        std::sync::mpsc::TrySendError::Disconnected(_) => ApiError::StorageError(
            StorageError::IoError(std::io::Error::other("progress bus disconnected")),
        ),
    }
}
