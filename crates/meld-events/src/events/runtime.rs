//! Synchronous event runtime facade.
//!
//! Owner: event runtime.
//! Inputs: legacy telemetry events, domain events, and prepared envelopes.
//! Outputs: flushed records in the backing [`crate::events::store::EventStore`].
//! Does not own: this module does not interpret event payloads or materialize
//! graph facts.
//!
//! # Example
//!
//! ```rust
//! use meld_events::EventRuntime;
//! use serde_json::json;
//!
//! let db = sled::Config::new().temporary(true).open().unwrap();
//! let runtime = EventRuntime::new(db).unwrap();
//! runtime.emit_domain_event(
//!     "session-a",
//!     "execution",
//!     "workflow-a",
//!     "execution.started",
//!     None,
//!     json!({ "started": true }),
//! ).unwrap();
//!
//! let events = runtime.store().read_events("session-a").unwrap();
//! assert_eq!(events.len(), 1);
//! ```

use std::sync::Arc;

use serde_json::Value;
use tracing::warn;

use crate::error::{ApiError, StorageError};
use crate::events::ingress::{EventBus, EventIngestor, SharedIngestor};
use crate::events::store::EventStore;
use crate::events::EventEnvelope;

/// Synchronous event runtime that drains emitted events into the store.
#[derive(Clone)]
pub struct EventRuntime {
    store: Arc<EventStore>,
    bus: EventBus,
    ingestor: SharedIngestor,
}

impl EventRuntime {
    /// Creates a runtime with an in-process bus and shared store.
    pub fn new(db: sled::Db) -> Result<Self, StorageError> {
        let store = EventStore::shared(db)?;
        let (bus, rx) = EventBus::new_pair();
        let ingestor = SharedIngestor::new(EventIngestor::new(store.clone(), rx));
        Ok(Self {
            store,
            bus,
            ingestor,
        })
    }

    /// Emits a legacy telemetry event and drains it immediately.
    pub fn emit_event(
        &self,
        session_id: &str,
        event_type: &str,
        data: Value,
    ) -> Result<(), ApiError> {
        self.bus
            .emit(session_id.to_string(), event_type.to_string(), data)
            .map_err(to_api_error)?;
        self.ingestor.drain()?;
        self.store.flush()?;
        Ok(())
    }

    /// Emits a domain event and drains it immediately.
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
            .emit_envelope(EventEnvelope::with_now_domain(
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

    /// Emits a prepared envelope and drains it immediately.
    pub fn emit_envelope(&self, envelope: EventEnvelope) -> Result<(), ApiError> {
        self.bus.emit_envelope(envelope).map_err(to_api_error)?;
        self.ingestor.drain()?;
        self.store.flush()?;
        Ok(())
    }

    /// Appends a prepared envelope through the idempotent store path.
    pub fn emit_envelope_idempotent(&self, envelope: EventEnvelope) -> Result<(), ApiError> {
        self.store.append_envelope_idempotent(envelope)?;
        self.store.flush()?;
        Ok(())
    }

    /// Emits a batch of envelopes and drains once after enqueueing.
    pub fn emit_envelopes<I>(&self, envelopes: I) -> Result<(), ApiError>
    where
        I: IntoIterator<Item = EventEnvelope>,
    {
        for envelope in envelopes {
            self.bus.emit_envelope(envelope).map_err(to_api_error)?;
        }
        self.ingestor.drain()?;
        self.store.flush()?;
        Ok(())
    }

    /// Appends a batch of envelopes through the idempotent store path.
    pub fn emit_envelopes_idempotent<I>(&self, envelopes: I) -> Result<(), ApiError>
    where
        I: IntoIterator<Item = EventEnvelope>,
    {
        for envelope in envelopes {
            self.store.append_envelope_idempotent(envelope)?;
        }
        self.store.flush()?;
        Ok(())
    }

    /// Emits a legacy telemetry event and logs any failure.
    pub fn emit_event_best_effort(&self, session_id: &str, event_type: &str, data: Value) {
        if let Err(err) = self.emit_event(session_id, event_type, data) {
            warn!(
                session_id = %session_id,
                event_type = %event_type,
                error = %err,
                "failed to emit event"
            );
        }
    }

    /// Emits a domain event and logs any failure.
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

    /// Emits a prepared envelope and logs any failure.
    pub fn emit_envelope_best_effort(&self, envelope: EventEnvelope) {
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

    /// Appends a prepared envelope idempotently and logs any failure.
    pub fn emit_envelope_idempotent_best_effort(&self, envelope: EventEnvelope) {
        let session_id = envelope.session.clone();
        let event_type = envelope.event_type.clone();
        if let Err(err) = self.emit_envelope_idempotent(envelope) {
            warn!(
                session_id = %session_id,
                event_type = %event_type,
                error = %err,
                "failed to emit idempotent envelope"
            );
        }
    }

    /// Returns the backing event store for queries and tests.
    pub fn store(&self) -> &EventStore {
        &self.store
    }
}

fn to_api_error(err: std::sync::mpsc::TrySendError<EventEnvelope>) -> ApiError {
    match err {
        std::sync::mpsc::TrySendError::Full(_) => {
            ApiError::StorageError(StorageError::Backpressure("event bus is full".to_string()))
        }
        std::sync::mpsc::TrySendError::Disconnected(_) => ApiError::StorageError(
            StorageError::IoError(std::io::Error::other("event bus disconnected")),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_or_missing_consumer_does_not_break_append() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let runtime = EventRuntime::new(db).unwrap();

        runtime
            .emit_event("session_a", "session_started", serde_json::json!({}))
            .unwrap();
        runtime
            .emit_event("session_a", "session_ended", serde_json::json!({}))
            .unwrap();

        let events = runtime.store().read_events("session_a").unwrap();
        assert_eq!(events.len(), 2);
    }
}
