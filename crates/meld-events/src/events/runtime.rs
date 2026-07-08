//! Event runtime facade over the spine writer.
//!
//! Owner: event runtime.
//! Inputs: legacy telemetry events, domain events, and prepared envelopes.
//! Outputs: writer-committed records in the backing
//! [`crate::events::store::EventStore`], with durable acks or counted
//! best-effort drops per the durability class.
//! Does not own: this module does not interpret event payloads or materialize
//! graph facts.
//!
//! Two durability classes exist. Plain `emit_*` calls are durable: they
//! return after the writer's group commit fsyncs the record, so success means
//! durable bytes. `*_best_effort` calls enqueue and return; a full queue
//! drops the event, counts it, and logs, never blocking the producer.
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
use crate::events::store::EventStore;
use crate::events::writer::{CommitWatermark, SpineWriter};
use crate::events::EventEnvelope;

/// Producer-facing event runtime routing all appends through one writer.
#[derive(Clone)]
pub struct EventRuntime {
    store: Arc<EventStore>,
    writer: Arc<SpineWriter>,
}

impl EventRuntime {
    /// Creates a runtime and its writer over a dedicated database handle.
    pub fn new(db: sled::Db) -> Result<Self, StorageError> {
        Ok(Self::from_store(EventStore::shared(db)?))
    }

    /// Creates a runtime and its writer over an already opened store.
    pub fn from_store(store: Arc<EventStore>) -> Self {
        let writer = Arc::new(SpineWriter::spawn(Arc::clone(&store)));
        Self { store, writer }
    }

    /// Emits a legacy telemetry event durably.
    pub fn emit_event(
        &self,
        session_id: &str,
        event_type: &str,
        data: Value,
    ) -> Result<(), ApiError> {
        let envelope = EventEnvelope::with_now(session_id.to_string(), event_type, data);
        self.writer.append_durable(envelope, false)?;
        Ok(())
    }

    /// Emits a domain event durably.
    pub fn emit_domain_event(
        &self,
        session_id: &str,
        domain_id: &str,
        stream_id: &str,
        event_type: &str,
        content_hash: Option<String>,
        data: Value,
    ) -> Result<(), ApiError> {
        let envelope = EventEnvelope::with_now_domain(
            session_id.to_string(),
            domain_id.to_string(),
            stream_id.to_string(),
            event_type,
            content_hash,
            data,
        );
        self.writer.append_durable(envelope, false)?;
        Ok(())
    }

    /// Emits a prepared envelope durably.
    pub fn emit_envelope(&self, envelope: EventEnvelope) -> Result<(), ApiError> {
        self.writer.append_durable(envelope, false)?;
        Ok(())
    }

    /// Emits a prepared envelope durably through the idempotent path.
    pub fn emit_envelope_idempotent(&self, envelope: EventEnvelope) -> Result<(), ApiError> {
        self.writer.append_durable(envelope, true)?;
        Ok(())
    }

    /// Emits a batch durably, sharing the writer's group commit.
    pub fn emit_envelopes<I>(&self, envelopes: I) -> Result<(), ApiError>
    where
        I: IntoIterator<Item = EventEnvelope>,
    {
        self.writer
            .append_durable_batch(envelopes.into_iter().collect(), false)?;
        Ok(())
    }

    /// Emits a batch durably through the idempotent path.
    pub fn emit_envelopes_idempotent<I>(&self, envelopes: I) -> Result<(), ApiError>
    where
        I: IntoIterator<Item = EventEnvelope>,
    {
        self.writer
            .append_durable_batch(envelopes.into_iter().collect(), true)?;
        Ok(())
    }

    /// Enqueues a legacy telemetry event without waiting for durability.
    pub fn emit_event_best_effort(&self, session_id: &str, event_type: &str, data: Value) {
        let envelope = EventEnvelope::with_now(session_id.to_string(), event_type, data);
        if let Err(err) = self.writer.append_best_effort(envelope, false) {
            warn!(
                session_id = %session_id,
                event_type = %event_type,
                error = %err,
                "failed to enqueue event"
            );
        }
    }

    /// Enqueues a domain event without waiting for durability.
    pub fn emit_domain_event_best_effort(
        &self,
        session_id: &str,
        domain_id: &str,
        stream_id: &str,
        event_type: &str,
        content_hash: Option<String>,
        data: Value,
    ) {
        let envelope = EventEnvelope::with_now_domain(
            session_id.to_string(),
            domain_id.to_string(),
            stream_id.to_string(),
            event_type,
            content_hash,
            data,
        );
        if let Err(err) = self.writer.append_best_effort(envelope, false) {
            warn!(
                session_id = %session_id,
                domain_id = %domain_id,
                stream_id = %stream_id,
                event_type = %event_type,
                error = %err,
                "failed to enqueue domain event"
            );
        }
    }

    /// Enqueues a prepared envelope without waiting for durability.
    pub fn emit_envelope_best_effort(&self, envelope: EventEnvelope) {
        let session_id = envelope.session.clone();
        let event_type = envelope.event_type.clone();
        if let Err(err) = self.writer.append_best_effort(envelope, false) {
            warn!(
                session_id = %session_id,
                event_type = %event_type,
                error = %err,
                "failed to enqueue envelope"
            );
        }
    }

    /// Enqueues a prepared envelope idempotently without waiting for durability.
    pub fn emit_envelope_idempotent_best_effort(&self, envelope: EventEnvelope) {
        let session_id = envelope.session.clone();
        let event_type = envelope.event_type.clone();
        if let Err(err) = self.writer.append_best_effort(envelope, true) {
            warn!(
                session_id = %session_id,
                event_type = %event_type,
                error = %err,
                "failed to enqueue idempotent envelope"
            );
        }
    }

    /// Blocks until every previously enqueued emit has reached the store.
    ///
    /// Best-effort emits return before persistence; this is the
    /// synchronization point for observing them.
    pub fn barrier(&self) -> Result<(), ApiError> {
        self.writer.barrier()?;
        Ok(())
    }

    /// Returns the writer's committed-sequence watermark for consumers.
    pub fn watermark(&self) -> Arc<CommitWatermark> {
        self.writer.watermark()
    }

    /// Returns how many best-effort events backpressure has dropped.
    pub fn dropped_events(&self) -> u64 {
        self.writer.dropped_events()
    }

    /// Returns the backing event store for queries and tests.
    pub fn store(&self) -> &EventStore {
        &self.store
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

    #[test]
    fn durable_emit_advances_watermark() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let runtime = EventRuntime::new(db).unwrap();

        runtime
            .emit_event("session_a", "session_started", serde_json::json!({}))
            .unwrap();

        assert_eq!(runtime.watermark().committed_seq(), 1);
        assert_eq!(runtime.dropped_events(), 0);
    }
}
