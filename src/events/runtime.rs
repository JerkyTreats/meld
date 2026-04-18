use std::sync::Arc;

use serde_json::Value;
use tracing::warn;

use crate::error::{ApiError, StorageError};
use crate::events::ingress::{EventBus, EventIngestor, SharedIngestor};
use crate::events::store::EventStore;
use crate::events::EventEnvelope;

#[derive(Clone)]
pub struct EventRuntime {
    store: Arc<EventStore>,
    bus: EventBus,
    ingestor: SharedIngestor,
}

impl EventRuntime {
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

    pub fn emit_envelope(&self, envelope: EventEnvelope) -> Result<(), ApiError> {
        self.bus.emit_envelope(envelope).map_err(to_api_error)?;
        self.ingestor.drain()?;
        self.store.flush()?;
        Ok(())
    }

    pub fn emit_envelope_idempotent(&self, envelope: EventEnvelope) -> Result<(), ApiError> {
        self.emit_envelope(envelope)
    }

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

    pub fn emit_envelopes_idempotent<I>(&self, envelopes: I) -> Result<(), ApiError>
    where
        I: IntoIterator<Item = EventEnvelope>,
    {
        self.emit_envelopes(envelopes)
    }

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
