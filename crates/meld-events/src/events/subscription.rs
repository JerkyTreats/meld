//! Consumer subscription surface: watermark notification plus bounded replay.
//!
//! Owner: event subscription.
//! Inputs: a shared store, the writer's commit watermark, and caller-supplied
//! cursors.
//! Outputs: bounded, sequence-ordered batches that block until new events
//! commit, and a durable cursor helper consumers embed in their own trees.
//! Does not own: consumer cursor positions. The ledger never persists a
//! consumer's progress; `EventCursor` writes into a consumer-owned tree.

use std::sync::Arc;
use std::time::Duration;

use sled::Tree;

use serde::{Deserialize, Serialize};

use crate::error::StorageError;
use crate::events::identity::LedgerIdentity;
use crate::events::store::EventStore;
use crate::events::writer::CommitWatermark;
use crate::events::EventRecord;

/// Blocking, bounded reader over the ledger for one consumer.
///
/// Reads happen before any wait, so events appended outside the writer,
/// such as reducer-derived records, are always observable even though they
/// do not advance the watermark.
pub struct EventSubscription {
    store: Arc<EventStore>,
    watermark: Arc<CommitWatermark>,
}

impl EventSubscription {
    /// Binds a subscription to a store and its writer's watermark.
    pub fn new(store: Arc<EventStore>, watermark: Arc<CommitWatermark>) -> Self {
        // TODO compat-shim: E5 removes arbitrary store/watermark pairing after
        // authority subscription and direct CLI paging parity tests pass.
        Self { store, watermark }
    }

    /// Returns a bounded batch after the cursor, blocking until the
    /// watermark passes the cursor or the timeout elapses when nothing is
    /// immediately readable. An empty batch means the timeout expired.
    pub fn next_batch(
        &self,
        after_seq: u64,
        limit: usize,
        timeout: Duration,
    ) -> Result<Vec<EventRecord>, StorageError> {
        let batch = self.store.read_all_events_after_limit(after_seq, limit)?;
        if !batch.is_empty() {
            return Ok(batch);
        }
        self.watermark.wait_past(after_seq, timeout);
        self.store.read_all_events_after_limit(after_seq, limit)
    }

    /// Returns the shared commit watermark for callers that wake themselves.
    pub fn watermark(&self) -> Arc<CommitWatermark> {
        // TODO compat-shim: E5 removes raw watermark access after runtime and
        // observability callers consume EventWatermarkCapability.
        Arc::clone(&self.watermark)
    }
}

/// Durable consumer cursor stored in the consumer's own tree.
///
/// The consumer advances the cursor only after its derived state is durable,
/// per the event runtime requirements; the helper enforces monotonicity but
/// owns no policy about when to advance.
pub struct EventCursor {
    tree: Tree,
    key: Vec<u8>,
    binding: CursorBinding,
}

#[derive(Clone, Copy)]
enum CursorBinding {
    Legacy,
    Authority(LedgerIdentity),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct PersistedCursor {
    ledger_id: LedgerIdentity,
    after_seq: u64,
}

impl EventCursor {
    /// Binds a named cursor inside a consumer-owned tree.
    pub fn new(tree: Tree, name: impl AsRef<str>) -> Self {
        // TODO compat-shim: E5 removes this unbound constructor after legacy
        // graph cursor rebuild and authority cursor parity tests pass.
        Self {
            key: format!("event_cursor::{}", name.as_ref()).into_bytes(),
            tree,
            binding: CursorBinding::Legacy,
        }
    }

    /// Binds an identity-bearing cursor for compatibility and migration tests.
    ///
    /// TODO compat-shim: E5 replaces raw tree binding with the world-model
    /// consumer cursor port after cursor rebuild and reopen parity tests pass.
    pub fn bind_compatibility(
        tree: Tree,
        name: impl AsRef<str>,
        ledger_id: LedgerIdentity,
    ) -> Self {
        Self {
            key: format!("event_cursor::{}", name.as_ref()).into_bytes(),
            tree,
            binding: CursorBinding::Authority(ledger_id),
        }
    }

    /// Explicitly relabels one legacy eight-byte cursor after its sequence
    /// space has been proven to belong to `ledger_id`.
    ///
    /// TODO compat-shim: E5 removes this migration entry after legacy graph
    /// cursor rebuild and cursor migration parity tests pass.
    pub fn migrate_legacy(
        tree: Tree,
        name: impl AsRef<str>,
        ledger_id: LedgerIdentity,
    ) -> Result<Self, StorageError> {
        let cursor = Self::bind_compatibility(tree, name, ledger_id);
        let Some(raw) = cursor.tree.get(&cursor.key).map_err(to_storage_io)? else {
            return Ok(cursor);
        };
        if raw.len() != 8 {
            cursor.decode(raw.as_ref())?;
            return Ok(cursor);
        }
        let after_seq = decode_legacy_cursor(raw.as_ref())?;
        cursor
            .tree
            .insert(&cursor.key, cursor.encode(after_seq)?)
            .map_err(to_storage_io)?;
        cursor.tree.flush().map_err(to_storage_io)?;
        Ok(cursor)
    }

    /// Returns the cursor position, zero when never advanced.
    pub fn get(&self) -> Result<u64, StorageError> {
        let Some(raw) = self.tree.get(&self.key).map_err(to_storage_io)? else {
            return Ok(0);
        };
        self.decode(raw.as_ref())
    }

    /// Advances the cursor monotonically; regressions are ignored so replays
    /// and restarts can never move a consumer backwards.
    pub fn advance(&self, seq: u64) -> Result<u64, StorageError> {
        loop {
            let observed = self.tree.get(&self.key).map_err(to_storage_io)?;
            let current = match observed.as_deref() {
                Some(raw) => self.decode(raw)?,
                None => 0,
            };
            if seq <= current {
                self.tree.flush().map_err(to_storage_io)?;
                return Ok(current);
            }

            let encoded = self.encode(seq)?;
            let prior = observed.as_deref();
            match self
                .tree
                .compare_and_swap(&self.key, prior, Some(encoded.as_slice()))
                .map_err(to_storage_io)?
            {
                Ok(()) => {
                    self.tree.flush().map_err(to_storage_io)?;
                    return Ok(seq);
                }
                Err(_) => continue,
            }
        }
    }

    fn decode(&self, raw: &[u8]) -> Result<u64, StorageError> {
        match self.binding {
            CursorBinding::Legacy => decode_legacy_cursor(raw),
            CursorBinding::Authority(expected) => {
                let cursor: PersistedCursor = serde_json::from_slice(raw).map_err(|error| {
                    invalid_cursor(format!("invalid identity-bearing ledger cursor: {error}"))
                })?;
                if cursor.ledger_id != expected {
                    return Err(StorageError::IdentityMismatch {
                        expected,
                        actual: cursor.ledger_id,
                    });
                }
                Ok(cursor.after_seq)
            }
        }
    }

    fn encode(&self, after_seq: u64) -> Result<Vec<u8>, StorageError> {
        match self.binding {
            CursorBinding::Legacy => Ok(after_seq.to_be_bytes().to_vec()),
            CursorBinding::Authority(ledger_id) => serde_json::to_vec(&PersistedCursor {
                ledger_id,
                after_seq,
            })
            .map_err(|error| invalid_cursor(error.to_string())),
        }
    }
}

fn decode_legacy_cursor(raw: &[u8]) -> Result<u64, StorageError> {
    let bytes: [u8; 8] = raw
        .try_into()
        .map_err(|_| invalid_cursor("invalid ledger cursor payload"))?;
    Ok(u64::from_be_bytes(bytes))
}

fn invalid_cursor(message: impl Into<String>) -> StorageError {
    StorageError::IoError(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message.into(),
    ))
}

fn to_storage_io(err: sled::Error) -> StorageError {
    StorageError::IoError(std::io::Error::other(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::writer::EventWriter;
    use crate::events::EventEnvelope;
    use serde_json::json;

    fn subscription() -> (
        tempfile::TempDir,
        Arc<EventStore>,
        EventWriter,
        EventSubscription,
    ) {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = EventStore::shared(db).unwrap();
        let writer = EventWriter::spawn(Arc::clone(&store));
        let subscription = EventSubscription::new(Arc::clone(&store), writer.watermark());
        (dir, store, writer, subscription)
    }

    fn envelope(i: usize) -> EventEnvelope {
        EventEnvelope::new_domain(
            "2026-07-08T00:00:00Z".to_string(),
            "session-s",
            "execution",
            "session-s",
            "execution.task.progress",
            None,
            json!({ "i": i }),
        )
    }

    #[test]
    fn next_batch_returns_immediately_when_events_exist() {
        let (_dir, _store, writer, subscription) = subscription();
        writer.append_durable(envelope(0), false).unwrap();
        let batch = subscription
            .next_batch(0, 16, Duration::from_millis(1))
            .unwrap();
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn next_batch_blocks_until_commit() {
        let (_dir, _store, writer, subscription) = subscription();
        let waiter = std::thread::spawn(move || {
            subscription
                .next_batch(0, 16, Duration::from_secs(5))
                .unwrap()
        });
        writer.append_durable(envelope(0), false).unwrap();
        assert_eq!(waiter.join().unwrap().len(), 1);
    }

    #[test]
    fn next_batch_times_out_empty() {
        let (_dir, _store, _writer, subscription) = subscription();
        let batch = subscription
            .next_batch(0, 16, Duration::from_millis(10))
            .unwrap();
        assert!(batch.is_empty());
    }

    #[test]
    fn next_batch_observes_direct_store_appends_without_watermark() {
        let (_dir, store, _writer, subscription) = subscription();
        store.append_envelope(envelope(0)).unwrap();
        let batch = subscription
            .next_batch(0, 16, Duration::from_millis(1))
            .unwrap();
        assert_eq!(batch.len(), 1);
    }

    #[test]
    fn cursor_is_monotonic_and_durable() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let tree = db.open_tree("consumer_meta").unwrap();
        let cursor = EventCursor::new(tree.clone(), "graph");

        assert_eq!(cursor.get().unwrap(), 0);
        assert_eq!(cursor.advance(7).unwrap(), 7);
        assert_eq!(cursor.advance(3).unwrap(), 7);
        assert_eq!(cursor.get().unwrap(), 7);

        let reopened = EventCursor::new(tree, "graph");
        assert_eq!(reopened.get().unwrap(), 7);
    }
}
