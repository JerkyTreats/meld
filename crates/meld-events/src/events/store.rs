//! Append-only event spine storage.
//!
//! Owner: event store.
//! Inputs: sequenced records, unsequenced envelopes, and legacy session event
//! rows.
//! Outputs: runtime-wide sequence allocation, idempotent append results,
//! session-scoped reads, and cursor reads across sessions.
//! Does not own: this module does not publish to telemetry sinks or interpret
//! producer payloads.
//!
//! # Example
//!
//! ```rust
//! use meld_events::events::store::EventStore;
//! use meld_events::EventEnvelope;
//! use serde_json::json;
//!
//! let db = sled::Config::new().temporary(true).open().unwrap();
//! let store = EventStore::new(db).unwrap();
//! let seq = store.append_envelope(EventEnvelope::new_domain(
//!     "2026-04-26T16:00:00Z".to_string(),
//!     "session-a",
//!     "execution",
//!     "workflow-a",
//!     "execution.started",
//!     None,
//!     json!({ "started": true }),
//! )).unwrap();
//!
//! assert_eq!(seq, 1);
//! assert_eq!(store.read_events("session-a").unwrap().len(), 1);
//! ```

use std::io;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use crate::error::StorageError;
use crate::events::EventEnvelope;
use crate::events::EventRecord;

const TREE_EVENTS: &str = "obs_events";
const TREE_SPINE_EVENTS: &str = "obs_spine_events";
const TREE_SESSION_EVENT_INDEX: &str = "obs_session_event_index";
const TREE_SPINE_META: &str = "obs_spine_meta";
const TREE_SPINE_RECORD_INDEX: &str = "obs_spine_record_index";
const EVENT_KEY_PAD: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpineMeta {
    next_seq: u64,
}

/// Append-only event spine backed by sled.
///
/// The store owns sequence allocation, idempotency lookup, session indexes,
/// and compatibility reads from the older session event tree.
#[derive(Clone)]
pub struct EventStore {
    db: Db,
    legacy_events: Tree,
    spine_events: Tree,
    session_event_index: Tree,
    spine_meta: Tree,
    spine_record_index: Tree,
}

impl EventStore {
    /// Opens all event trees on the supplied database handle.
    pub fn new(db: Db) -> Result<Self, StorageError> {
        let legacy_events = db.open_tree(TREE_EVENTS).map_err(to_storage_io)?;
        let spine_events = db.open_tree(TREE_SPINE_EVENTS).map_err(to_storage_io)?;
        let session_event_index = db
            .open_tree(TREE_SESSION_EVENT_INDEX)
            .map_err(to_storage_io)?;
        let spine_meta = db.open_tree(TREE_SPINE_META).map_err(to_storage_io)?;
        let spine_record_index = db
            .open_tree(TREE_SPINE_RECORD_INDEX)
            .map_err(to_storage_io)?;
        Ok(Self {
            db,
            legacy_events,
            spine_events,
            session_event_index,
            spine_meta,
            spine_record_index,
        })
    }

    /// Opens the store behind an `Arc` for runtimes and ingestors.
    pub fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        Ok(Arc::new(Self::new(db)?))
    }

    /// Returns the underlying sled database.
    pub fn db(&self) -> &Db {
        &self.db
    }

    /// Appends a pre-sequenced record and advances future sequence allocation.
    pub fn append_event(&self, event: &EventRecord) -> Result<(), StorageError> {
        self.write_event(event)?;
        Ok(())
    }

    /// Appends a pre-sequenced record unless its idempotency key already exists.
    pub fn append_event_idempotent(&self, event: &EventRecord) -> Result<u64, StorageError> {
        let Some(record_id) = event.record_id.as_deref() else {
            self.append_event(event)?;
            return Ok(event.seq);
        };

        if let Some(existing_seq) = self.lookup_record_seq(record_id)? {
            return Ok(existing_seq);
        }

        self.write_event(event)?;
        Ok(event.seq)
    }

    /// Allocates the next spine sequence and appends an envelope.
    pub fn append_envelope(&self, envelope: EventEnvelope) -> Result<u64, StorageError> {
        let seq = self.allocate_next_seq()?;
        let event = EventRecord::from_envelope(envelope, seq);
        self.append_event(&event)?;
        Ok(seq)
    }

    /// Allocates and appends an envelope unless its idempotency key already exists.
    pub fn append_envelope_idempotent(&self, envelope: EventEnvelope) -> Result<u64, StorageError> {
        if let Some(record_id) = envelope.record_id.as_deref() {
            if let Some(existing_seq) = self.lookup_record_seq(record_id)? {
                return Ok(existing_seq);
            }
        }

        let seq = self.allocate_next_seq()?;
        let event = EventRecord::from_envelope(envelope, seq);
        self.append_event_idempotent(&event)
    }

    fn write_event(&self, event: &EventRecord) -> Result<(), StorageError> {
        self.advance_next_seq_past(event.seq)?;
        let key = encode_spine_key(event.seq);
        let index_key = encode_session_event_index_key(&event.session, event.seq);
        let value = serde_json::to_vec(event).map_err(to_storage_data)?;
        self.spine_events
            .insert(key.as_bytes(), value.clone())
            .map_err(to_storage_io)?;
        self.session_event_index
            .insert(index_key.as_bytes(), value)
            .map_err(to_storage_io)?;
        if let Some(record_id) = event.record_id.as_deref() {
            self.spine_record_index
                .insert(record_id.as_bytes(), &encode_seq(event.seq))
                .map_err(to_storage_io)?;
        }
        Ok(())
    }

    /// Reads all events for a session from both current and legacy indexes.
    pub fn read_events(&self, session_id: &str) -> Result<Vec<EventRecord>, StorageError> {
        self.read_events_after(session_id, 0)
    }

    /// Reads events for a session after a spine sequence.
    pub fn read_events_after(
        &self,
        session_id: &str,
        after_seq: u64,
    ) -> Result<Vec<EventRecord>, StorageError> {
        let mut out = self.read_spine_session_events_after(session_id, after_seq)?;
        let mut legacy = self.read_legacy_events_after(session_id, after_seq)?;
        out.append(&mut legacy);
        out.sort_by_key(|event| event.seq);
        Ok(out)
    }

    /// Reads all events after a spine sequence across sessions.
    pub fn read_all_events_after(&self, after_seq: u64) -> Result<Vec<EventRecord>, StorageError> {
        let mut out = Vec::new();
        for result in self.spine_events.iter() {
            let (_, value) = result.map_err(to_storage_io)?;
            let parsed = decode_event(&value)?;
            if parsed.seq > after_seq {
                out.push(parsed);
            }
        }
        out.sort_by_key(|event| event.seq);
        Ok(out)
    }

    /// Reserves the next runtime-wide spine sequence.
    pub fn allocate_next_seq(&self) -> Result<u64, StorageError> {
        let mut meta = self.get_spine_meta()?.unwrap_or(SpineMeta { next_seq: 1 });
        let seq = meta.next_seq;
        meta.next_seq += 1;
        self.put_spine_meta(&meta)?;
        Ok(seq)
    }

    /// Flushes pending sled writes to durable storage.
    pub fn flush(&self) -> Result<(), StorageError> {
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    /// Encodes a legacy session event key.
    pub fn encode_event_key(session_id: &str, seq: u64) -> String {
        encode_legacy_event_key(session_id, seq)
    }

    fn read_spine_session_events_after(
        &self,
        session_id: &str,
        after_seq: u64,
    ) -> Result<Vec<EventRecord>, StorageError> {
        let prefix = format!("{session_id}:");
        let mut out = Vec::new();
        for result in self.session_event_index.scan_prefix(prefix.as_bytes()) {
            let (_, value) = result.map_err(to_storage_io)?;
            let parsed = decode_event(&value)?;
            if parsed.seq > after_seq {
                out.push(parsed);
            }
        }
        Ok(out)
    }

    fn read_legacy_events_after(
        &self,
        session_id: &str,
        after_seq: u64,
    ) -> Result<Vec<EventRecord>, StorageError> {
        let prefix = format!("{session_id}:");
        let mut out = Vec::new();
        for result in self.legacy_events.scan_prefix(prefix.as_bytes()) {
            let (_, value) = result.map_err(to_storage_io)?;
            let parsed = decode_event(&value)?;
            if parsed.seq > after_seq {
                out.push(parsed);
            }
        }
        Ok(out)
    }

    fn get_spine_meta(&self) -> Result<Option<SpineMeta>, StorageError> {
        let Some(raw) = self.spine_meta.get(b"global").map_err(to_storage_io)? else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_slice(&raw).map_err(to_storage_data)?))
    }

    fn put_spine_meta(&self, meta: &SpineMeta) -> Result<(), StorageError> {
        let value = serde_json::to_vec(meta).map_err(to_storage_data)?;
        self.spine_meta
            .insert(b"global", value)
            .map_err(to_storage_io)?;
        Ok(())
    }

    fn lookup_record_seq(&self, record_id: &str) -> Result<Option<u64>, StorageError> {
        let Some(raw) = self
            .spine_record_index
            .get(record_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        Ok(Some(decode_seq(&raw)?))
    }

    fn advance_next_seq_past(&self, seq: u64) -> Result<(), StorageError> {
        let mut meta = self.get_spine_meta()?.unwrap_or(SpineMeta { next_seq: 1 });
        if meta.next_seq <= seq {
            meta.next_seq = seq + 1;
            self.put_spine_meta(&meta)?;
        }
        Ok(())
    }
}

fn encode_legacy_event_key(session_id: &str, seq: u64) -> String {
    format!("{session_id}:{seq:0EVENT_KEY_PAD$}")
}

fn encode_spine_key(seq: u64) -> String {
    format!("{seq:0EVENT_KEY_PAD$}")
}

fn encode_session_event_index_key(session_id: &str, seq: u64) -> String {
    format!("{session_id}:{seq:0EVENT_KEY_PAD$}")
}

fn encode_seq(seq: u64) -> [u8; 8] {
    seq.to_be_bytes()
}

fn decode_seq(raw: &[u8]) -> Result<u64, StorageError> {
    let bytes: [u8; 8] = raw.try_into().map_err(|_| {
        StorageError::IoError(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid spine record index payload",
        ))
    })?;
    Ok(u64::from_be_bytes(bytes))
}

fn decode_event(raw: &[u8]) -> Result<EventRecord, StorageError> {
    Ok(serde_json::from_slice::<EventRecord>(raw)
        .map_err(to_storage_data)?
        .normalize_legacy_defaults())
}

fn to_storage_io(err: sled::Error) -> StorageError {
    StorageError::IoError(io::Error::other(err.to_string()))
}

fn to_storage_data(err: serde_json::Error) -> StorageError {
    StorageError::IoError(io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_record(seq: u64, ts: &str, session: &str, event_type: &str) -> EventRecord {
        EventRecord::from_envelope(
            EventEnvelope::new(
                ts.to_string(),
                session.to_string(),
                event_type,
                serde_json::json!({}),
            ),
            seq,
        )
    }

    #[test]
    fn key_encoding_is_lexicographic() {
        let k1 = EventStore::encode_event_key("s1", 2);
        let k2 = EventStore::encode_event_key("s1", 10);
        assert!(k1 < k2);
    }

    #[test]
    fn write_and_read_events_sorted() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = EventStore::new(db).unwrap();
        let session = "abc";

        let e2 = test_record(2, "2", session, "session_ended");
        let e1 = test_record(1, "1", session, "session_started");
        store.append_event(&e2).unwrap();
        store.append_event(&e1).unwrap();
        let events = store.read_events(session).unwrap();
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[1].seq, 2);
    }

    #[test]
    fn read_all_events_after_returns_runtime_order() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = EventStore::new(db).unwrap();

        let e1 = test_record(1, "1", "s1", "session_started");
        let e2 = test_record(2, "2", "s2", "session_started");

        store.append_event(&e2).unwrap();
        store.append_event(&e1).unwrap();

        let events = store.read_all_events_after(0).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[1].seq, 2);
    }

    #[test]
    fn legacy_events_remain_readable() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = EventStore::new(db.clone()).unwrap();
        let session = "legacy_session";

        let legacy_tree = db.open_tree("obs_events").unwrap();
        let key = EventStore::encode_event_key(session, 1);
        let raw = serde_json::to_vec(&serde_json::json!({
            "ts": "1",
            "recorded_at": "",
            "record_id": null,
            "session": session,
            "seq": 1,
            "domain_id": "",
            "stream_id": "",
            "type": "session_started",
            "occurred_at": null,
            "content_hash": null,
            "objects": [],
            "relations": [],
            "data": {}
        }))
        .unwrap();
        legacy_tree.insert(key.as_bytes(), raw).unwrap();

        let events = store.read_events(session).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].domain_id, "telemetry");
        assert_eq!(events[0].stream_id, session);
    }
}
