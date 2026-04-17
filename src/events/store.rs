use std::io;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use crate::error::StorageError;
use crate::events::EventRecord;
use crate::session::contracts::{SessionMeta as Meta, SessionRecord as Record};
use crate::session::storage::SessionStore;

const TREE_EVENTS: &str = "obs_events";
const TREE_SPINE_EVENTS: &str = "obs_spine_events";
const TREE_SESSION_EVENT_INDEX: &str = "obs_session_event_index";
const TREE_SPINE_META: &str = "obs_spine_meta";
const EVENT_KEY_PAD: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpineMeta {
    next_seq: u64,
}

#[derive(Clone)]
pub struct EventStore {
    db: Db,
    session_store: SessionStore,
    legacy_events: Tree,
    spine_events: Tree,
    session_event_index: Tree,
    spine_meta: Tree,
}

impl EventStore {
    pub fn new(db: Db) -> Result<Self, StorageError> {
        let legacy_events = db.open_tree(TREE_EVENTS).map_err(to_storage_io)?;
        let spine_events = db.open_tree(TREE_SPINE_EVENTS).map_err(to_storage_io)?;
        let session_event_index = db
            .open_tree(TREE_SESSION_EVENT_INDEX)
            .map_err(to_storage_io)?;
        let spine_meta = db.open_tree(TREE_SPINE_META).map_err(to_storage_io)?;
        Ok(Self {
            session_store: SessionStore::new(db.clone())?,
            db,
            legacy_events,
            spine_events,
            session_event_index,
            spine_meta,
        })
    }

    pub fn shared(db: Db) -> Result<Arc<Self>, StorageError> {
        Ok(Arc::new(Self::new(db)?))
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    pub fn put_session(&self, record: &Record) -> Result<(), StorageError> {
        self.session_store.put_session(record)
    }

    pub fn get_session(&self, session_id: &str) -> Result<Option<Record>, StorageError> {
        self.session_store.get_session(session_id)
    }

    pub fn list_sessions(&self) -> Result<Vec<Record>, StorageError> {
        self.session_store.list_sessions()
    }

    pub fn put_meta(&self, session_id: &str, meta: &Meta) -> Result<(), StorageError> {
        self.session_store.put_meta(session_id, meta)
    }

    pub fn get_meta(&self, session_id: &str) -> Result<Option<Meta>, StorageError> {
        self.session_store.get_meta(session_id)
    }

    pub fn append_event(&self, event: &EventRecord) -> Result<(), StorageError> {
        let key = encode_spine_key(event.seq);
        let index_key = encode_session_event_index_key(&event.session, event.seq);
        let value = serde_json::to_vec(event).map_err(to_storage_data)?;
        self.spine_events
            .insert(key.as_bytes(), value.clone())
            .map_err(to_storage_io)?;
        self.session_event_index
            .insert(index_key.as_bytes(), value)
            .map_err(to_storage_io)?;
        Ok(())
    }

    pub fn read_events(&self, session_id: &str) -> Result<Vec<EventRecord>, StorageError> {
        self.read_events_after(session_id, 0)
    }

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

    pub fn allocate_next_seq(&self) -> Result<u64, StorageError> {
        let mut meta = self.get_spine_meta()?.unwrap_or(SpineMeta { next_seq: 1 });
        let seq = meta.next_seq;
        meta.next_seq += 1;
        self.put_spine_meta(&meta)?;
        Ok(seq)
    }

    pub fn mark_interrupted_sessions(&self) -> Result<usize, StorageError> {
        self.session_store.mark_interrupted_sessions()
    }

    pub fn prune_completed(
        &self,
        max_completed: usize,
        max_age_ms: u64,
        now_ms: u64,
    ) -> Result<usize, StorageError> {
        self.session_store
            .prune_completed(max_completed, max_age_ms, now_ms)
    }

    pub fn flush(&self) -> Result<(), StorageError> {
        self.db.flush().map_err(to_storage_io)?;
        Ok(())
    }

    pub fn encode_event_key(session_id: &str, seq: u64) -> String {
        encode_legacy_event_key(session_id, seq)
    }

    pub fn delete_session(&self, session_id: &str) -> Result<(), StorageError> {
        self.session_store.delete_session(session_id)?;

        let prefix = format!("{session_id}:");
        let legacy_keys: Vec<Vec<u8>> = self
            .legacy_events
            .scan_prefix(prefix.as_bytes())
            .filter_map(|result| result.ok().map(|(key, _)| key.to_vec()))
            .collect();
        for key in legacy_keys {
            self.legacy_events.remove(key).map_err(to_storage_io)?;
        }

        let session_keys: Vec<(Vec<u8>, u64)> = self
            .session_event_index
            .scan_prefix(prefix.as_bytes())
            .filter_map(|result| {
                result.ok().and_then(|(key, value)| {
                    decode_event(&value)
                        .ok()
                        .map(|event| (key.to_vec(), event.seq))
                })
            })
            .collect();
        for (key, seq) in session_keys {
            self.session_event_index
                .remove(key)
                .map_err(to_storage_io)?;
            self.spine_events
                .remove(encode_spine_key(seq).as_bytes())
                .map_err(to_storage_io)?;
        }

        Ok(())
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

        let e2 = EventRecord {
            ts: "2".to_string(),
            recorded_at: "2".to_string(),
            session: session.to_string(),
            seq: 2,
            domain_id: "telemetry".to_string(),
            stream_id: session.to_string(),
            event_type: "session_ended".to_string(),
            occurred_at: None,
            content_hash: None,
            objects: Vec::new(),
            relations: Vec::new(),
            data: serde_json::json!({}),
        };
        let e1 = EventRecord {
            ts: "1".to_string(),
            recorded_at: "1".to_string(),
            session: session.to_string(),
            seq: 1,
            domain_id: "telemetry".to_string(),
            stream_id: session.to_string(),
            event_type: "session_started".to_string(),
            occurred_at: None,
            content_hash: None,
            objects: Vec::new(),
            relations: Vec::new(),
            data: serde_json::json!({}),
        };
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

        let e1 = EventRecord {
            ts: "1".to_string(),
            recorded_at: "1".to_string(),
            session: "s1".to_string(),
            seq: 1,
            domain_id: "telemetry".to_string(),
            stream_id: "s1".to_string(),
            event_type: "session_started".to_string(),
            occurred_at: None,
            content_hash: None,
            objects: Vec::new(),
            relations: Vec::new(),
            data: serde_json::json!({}),
        };
        let e2 = EventRecord {
            ts: "2".to_string(),
            recorded_at: "2".to_string(),
            session: "s2".to_string(),
            seq: 2,
            domain_id: "telemetry".to_string(),
            stream_id: "s2".to_string(),
            event_type: "session_started".to_string(),
            occurred_at: None,
            content_hash: None,
            objects: Vec::new(),
            relations: Vec::new(),
            data: serde_json::json!({}),
        };

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
        let raw = serde_json::to_vec(&EventRecord {
            ts: "1".to_string(),
            recorded_at: String::new(),
            session: session.to_string(),
            seq: 1,
            domain_id: String::new(),
            stream_id: String::new(),
            event_type: "session_started".to_string(),
            occurred_at: None,
            content_hash: None,
            objects: Vec::new(),
            relations: Vec::new(),
            data: serde_json::json!({}),
        })
        .unwrap();
        legacy_tree.insert(key.as_bytes(), raw).unwrap();

        let events = store.read_events(session).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].domain_id, "telemetry");
        assert_eq!(events[0].stream_id, session);
    }
}
