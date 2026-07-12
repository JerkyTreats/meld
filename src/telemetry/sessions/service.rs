//! Compatibility facade for session lifecycle and canonical event emission.

use std::sync::Arc;

use serde_json::Value;

use crate::error::ApiError;
use crate::events::store::EventStore;
use crate::events::EventRuntime;
use crate::session as lifecycle;
use crate::session::events::{session_ended_envelope, session_started_envelope};

#[derive(Clone)]
pub struct ProgressRuntime {
    events: Arc<EventRuntime>,
    sessions: Arc<lifecycle::SessionRuntime>,
}

impl ProgressRuntime {
    pub fn new(db: sled::Db) -> Result<Self, crate::error::StorageError> {
        // TODO compat-shim: E5 injects EventAppendCapability after
        // product_event_authority_cutover and removes this writable runtime.
        let events = Arc::new(EventRuntime::new(db.clone())?); // boundary-allow: event-compat
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
        self.events
            .emit_event(session_id, event_type, data)
            .map_err(ApiError::from)
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
        self.events
            .emit_domain_event(
                session_id,
                domain_id,
                stream_id,
                event_type,
                content_hash,
                data,
            )
            .map_err(ApiError::from)
    }

    pub fn emit_envelope(&self, envelope: crate::events::EventEnvelope) -> Result<(), ApiError> {
        self.events.emit_envelope(envelope).map_err(ApiError::from)
    }

    pub fn emit_envelope_idempotent(
        &self,
        envelope: crate::events::EventEnvelope,
    ) -> Result<(), ApiError> {
        self.events
            .emit_envelope_idempotent(envelope)
            .map_err(ApiError::from)
    }

    /// Enqueues without waiting for durability; readers synchronize through
    /// [`ProgressRuntime::barrier`].
    pub fn emit_event_best_effort(&self, session_id: &str, event_type: &str, data: Value) {
        self.events
            .emit_event_best_effort(session_id, event_type, data);
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
        self.events.emit_domain_event_best_effort(
            session_id,
            domain_id,
            stream_id,
            event_type,
            content_hash,
            data,
        );
    }

    /// Enqueues without waiting for durability; readers synchronize through
    /// [`ProgressRuntime::barrier`].
    pub fn emit_envelope_best_effort(&self, envelope: crate::events::EventEnvelope) {
        self.events.emit_envelope_best_effort(envelope);
    }

    /// Enqueues without waiting for durability; readers synchronize through
    /// [`ProgressRuntime::barrier`].
    pub fn emit_envelope_idempotent_best_effort(&self, envelope: crate::events::EventEnvelope) {
        self.events.emit_envelope_idempotent_best_effort(envelope);
    }

    pub fn mark_interrupted_sessions(&self) -> Result<usize, ApiError> {
        let changed = self.sessions.mark_interrupted_sessions()?;
        self.events.store().flush().map_err(ApiError::from)?;
        Ok(changed)
    }

    pub fn prune(
        &self,
        policy: crate::telemetry::sessions::policy::Policy,
    ) -> Result<usize, ApiError> {
        let pruned = self.sessions.prune(policy)?;
        self.events.store().flush().map_err(ApiError::from)?;
        Ok(pruned)
    }

    /// Blocks until every previously enqueued emit has reached the store,
    /// the synchronization point for observing best-effort emissions.
    pub fn barrier(&self) -> Result<(), ApiError> {
        self.events.barrier().map_err(ApiError::from)
    }

    /// Returns the writer's commit watermark for observability consumers.
    pub fn watermark(&self) -> std::sync::Arc<crate::events::CommitWatermark> {
        self.events.watermark()
    }

    /// Returns the writer's shared drop counter for observability consumers.
    pub fn dropped_handle(&self) -> std::sync::Arc<std::sync::atomic::AtomicU64> {
        self.events.dropped_handle()
    }

    pub fn store(&self) -> &EventStore {
        self.events.store()
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
}
