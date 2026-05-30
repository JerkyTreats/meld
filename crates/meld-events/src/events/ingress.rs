use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::error::StorageError;
use crate::events::store::EventStore;
use crate::events::EventEnvelope;

const DEFAULT_EVENT_BUS_CAPACITY: usize = 1024;

/// Non-blocking producer side of the in-process event bus.
#[derive(Clone)]
pub struct EventBus {
    sender: SyncSender<EventEnvelope>,
}

impl EventBus {
    /// Creates a bus with the default bounded capacity.
    pub fn new_pair() -> (Self, Receiver<EventEnvelope>) {
        Self::new_pair_with_capacity(DEFAULT_EVENT_BUS_CAPACITY)
    }

    /// Creates a bus with a caller-specified bounded capacity.
    pub fn new_pair_with_capacity(capacity: usize) -> (Self, Receiver<EventEnvelope>) {
        let (sender, receiver) = sync_channel(capacity);
        (Self { sender }, receiver)
    }

    /// Attempts to enqueue an already-built envelope without blocking.
    #[allow(clippy::result_large_err)]
    pub fn emit_envelope(
        &self,
        envelope: EventEnvelope,
    ) -> Result<(), TrySendError<EventEnvelope>> {
        self.sender.try_send(envelope)
    }

    /// Builds and attempts to enqueue a legacy telemetry envelope.
    #[allow(clippy::result_large_err)]
    pub fn emit(
        &self,
        session: impl Into<String>,
        event_type: impl Into<String>,
        data: Value,
    ) -> Result<(), TrySendError<EventEnvelope>> {
        let envelope = EventEnvelope::with_now(session, event_type, data);
        self.emit_envelope(envelope)
    }
}

/// Consumer that drains queued envelopes into the event store.
pub struct EventIngestor {
    store: Arc<EventStore>,
    receiver: Receiver<EventEnvelope>,
}

impl EventIngestor {
    /// Creates an ingestor over a shared store and bus receiver.
    pub fn new(store: Arc<EventStore>, receiver: Receiver<EventEnvelope>) -> Self {
        Self { store, receiver }
    }

    /// Drains all currently queued envelopes and returns the number ingested.
    pub fn ingest_pending(&mut self) -> Result<usize, StorageError> {
        let mut count = 0usize;
        while let Ok(envelope) = self.receiver.try_recv() {
            self.ingest_one(envelope)?;
            count += 1;
        }
        Ok(count)
    }

    fn ingest_one(&self, envelope: EventEnvelope) -> Result<(), StorageError> {
        self.store.append_envelope(envelope)?;
        Ok(())
    }
}

/// Mutex-protected ingestor used by synchronous runtime APIs.
#[derive(Clone)]
pub struct SharedIngestor(Arc<Mutex<EventIngestor>>);

impl SharedIngestor {
    /// Wraps an ingestor for shared synchronous access.
    pub fn new(inner: EventIngestor) -> Self {
        Self(Arc::new(Mutex::new(inner)))
    }

    /// Drains all currently queued envelopes.
    pub fn drain(&self) -> Result<usize, StorageError> {
        let mut guard = self.0.lock().expect("ingestor lock poisoned");
        guard.ingest_pending()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_wide_sequence_is_monotonic() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = EventStore::shared(db).unwrap();
        let (bus, rx) = EventBus::new_pair();
        let mut ingestor = EventIngestor::new(store.clone(), rx);
        bus.emit("s1", "session_started", serde_json::json!({}))
            .unwrap();
        bus.emit("s2", "session_started", serde_json::json!({}))
            .unwrap();
        ingestor.ingest_pending().unwrap();
        let events = store.read_all_events_after(0).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[1].seq, 2);
        assert_eq!(events[0].session, "s1");
        assert_eq!(events[1].session, "s2");
    }
}
