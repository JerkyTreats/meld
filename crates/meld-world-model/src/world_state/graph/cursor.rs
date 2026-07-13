//! Identity-bearing durable cursor for the graph projection.

#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use sled::Tree;

use crate::error::StorageError;
use crate::events::{LedgerCursor, LedgerIdentity};
use crate::world_state::graph::store::TraversalStore;

const TREE_RUNTIME_META: &str = "traversal_runtime_meta";
const KEY_AUTHORITY_CURSOR: &[u8] = b"event_authority_cursor";
const KEY_LEGACY_CURSOR: &[u8] = b"last_reduced_seq";
const KEY_LEGACY_CURSOR_EVIDENCE: &[u8] = b"legacy_last_reduced_seq_evidence";
const KEY_LEGACY_RESET_PENDING: &[u8] = b"legacy_projection_reset_pending";
#[cfg(test)]
const KEY_PENDING_DERIVED_EVENTS: &[u8] = b"pending_derived_events";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct PersistedGraphCursor {
    ledger_id: LedgerIdentity,
    after_seq: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct PendingLegacyProjectionReset {
    target_ledger_id: LedgerIdentity,
    legacy_cursor: u64,
    legacy_evidence: u64,
}

pub(super) struct GraphProjectionCursor {
    tree: Tree,
    ledger_id: LedgerIdentity,
}

impl GraphProjectionCursor {
    pub(super) fn open(
        traversal: &TraversalStore,
        ledger_id: LedgerIdentity,
    ) -> Result<Self, StorageError> {
        let tree = traversal
            .db()
            .open_tree(TREE_RUNTIME_META)
            .map_err(to_storage_io)?;
        let cursor = Self { tree, ledger_id };
        if let Some(raw) = cursor
            .tree
            .get(KEY_LEGACY_RESET_PENDING)
            .map_err(to_storage_io)?
        {
            let pending = cursor.decode_pending_reset(raw.as_ref())?;
            cursor.validate_pending_reset(&pending)?;
            cursor.finish_legacy_reset(traversal)?;
            return Ok(cursor);
        }
        if let Some(raw) = cursor
            .tree
            .get(KEY_AUTHORITY_CURSOR)
            .map_err(to_storage_io)?
        {
            cursor.decode(raw.as_ref())?;
            return Ok(cursor);
        }

        // TODO compat-shim: remove after the minimum supported traversal schema
        // guarantees an identity-bearing authority cursor. Until then,
        // graph_runtime_resets_legacy_cursor_and_preserves_migration_evidence and
        // graph_runtime_legacy_cursor_reset_rebuilds_conflicting_projection_sequence_space
        // prove the old cursor is preserved as evidence and the projection is rebuilt.
        if let Some(legacy) = cursor.tree.get(KEY_LEGACY_CURSOR).map_err(to_storage_io)? {
            let legacy_cursor = decode_legacy_cursor(legacy.as_ref())?;
            match cursor
                .tree
                .get(KEY_LEGACY_CURSOR_EVIDENCE)
                .map_err(to_storage_io)?
            {
                Some(existing) if existing != legacy => {
                    return Err(StorageError::InvalidPath(
                        "graph legacy cursor evidence conflicts with the persisted cursor"
                            .to_string(),
                    ));
                }
                Some(_) => {}
                None => {
                    cursor
                        .tree
                        .insert(KEY_LEGACY_CURSOR_EVIDENCE, &legacy)
                        .map_err(to_storage_io)?;
                }
            }
            let legacy_evidence = cursor
                .tree
                .get(KEY_LEGACY_CURSOR_EVIDENCE)
                .map_err(to_storage_io)?
                .ok_or_else(|| {
                    StorageError::InvalidPath(
                        "graph legacy cursor evidence disappeared during migration".to_string(),
                    )
                })?;
            let legacy_evidence = decode_legacy_cursor(legacy_evidence.as_ref())?;
            let pending = PendingLegacyProjectionReset {
                target_ledger_id: ledger_id,
                legacy_cursor,
                legacy_evidence,
            };
            cursor
                .tree
                .insert(
                    KEY_LEGACY_RESET_PENDING,
                    serde_json::to_vec(&pending).map_err(to_storage_data)?,
                )
                .map_err(to_storage_io)?;
            cursor.tree.flush().map_err(to_storage_io)?;
            fail_after_pending_reset_persisted()?;
            cursor.finish_legacy_reset(traversal)?;
        }
        Ok(cursor)
    }

    pub(super) fn get(&self) -> Result<LedgerCursor, StorageError> {
        let after_seq = match self.tree.get(KEY_AUTHORITY_CURSOR).map_err(to_storage_io)? {
            Some(raw) => self.decode(raw.as_ref())?,
            None => 0,
        };
        Ok(LedgerCursor {
            ledger_id: self.ledger_id,
            after_seq,
        })
    }

    pub(super) fn advance(
        &self,
        expected_after_seq: u64,
        after_seq: u64,
    ) -> Result<LedgerCursor, StorageError> {
        if after_seq <= expected_after_seq {
            return Err(StorageError::InvalidPath(
                "graph cursor must advance to a newer sequence".to_string(),
            ));
        }
        let current = self.tree.get(KEY_AUTHORITY_CURSOR).map_err(to_storage_io)?;
        let current_after_seq = current
            .as_deref()
            .map(|raw| self.decode(raw))
            .transpose()?
            .unwrap_or(0);
        if current_after_seq != expected_after_seq {
            return Err(StorageError::Backpressure(format!(
                "graph cursor changed from expected sequence {expected_after_seq} to {current_after_seq}"
            )));
        }
        let next = serde_json::to_vec(&PersistedGraphCursor {
            ledger_id: self.ledger_id,
            after_seq,
        })
        .map_err(to_storage_data)?;
        match self
            .tree
            .compare_and_swap(
                KEY_AUTHORITY_CURSOR,
                current.as_deref(),
                Some(next.as_slice()),
            )
            .map_err(to_storage_io)?
        {
            Ok(()) => {
                self.tree.flush().map_err(to_storage_io)?;
            }
            Err(_) => {
                return Err(StorageError::Backpressure(
                    "graph cursor changed concurrently".to_string(),
                ));
            }
        }
        Ok(LedgerCursor {
            ledger_id: self.ledger_id,
            after_seq,
        })
    }

    fn persist(&self, after_seq: u64) -> Result<(), StorageError> {
        let bytes = serde_json::to_vec(&PersistedGraphCursor {
            ledger_id: self.ledger_id,
            after_seq,
        })
        .map_err(to_storage_data)?;
        self.tree
            .insert(KEY_AUTHORITY_CURSOR, bytes)
            .map_err(to_storage_io)?;
        self.tree.flush().map_err(to_storage_io)?;
        Ok(())
    }

    fn decode(&self, raw: &[u8]) -> Result<u64, StorageError> {
        let persisted: PersistedGraphCursor = serde_json::from_slice(raw).map_err(|error| {
            StorageError::InvalidPath(format!("invalid graph authority cursor: {error}"))
        })?;
        if persisted.ledger_id != self.ledger_id {
            return Err(StorageError::IdentityMismatch {
                expected: self.ledger_id,
                actual: persisted.ledger_id,
            });
        }
        Ok(persisted.after_seq)
    }

    fn finish_legacy_reset(&self, traversal: &TraversalStore) -> Result<(), StorageError> {
        traversal.reset_for_event_authority_migration()?;
        self.persist(0)?;
        self.tree
            .remove(KEY_LEGACY_RESET_PENDING)
            .map_err(to_storage_io)?;
        self.tree.flush().map_err(to_storage_io)?;
        Ok(())
    }

    fn decode_pending_reset(
        &self,
        raw: &[u8],
    ) -> Result<PendingLegacyProjectionReset, StorageError> {
        serde_json::from_slice(raw).map_err(|error| {
            StorageError::InvalidPath(format!(
                "invalid pending graph legacy projection reset: {error}"
            ))
        })
    }

    fn validate_pending_reset(
        &self,
        pending: &PendingLegacyProjectionReset,
    ) -> Result<(), StorageError> {
        if pending.target_ledger_id != self.ledger_id {
            return Err(StorageError::IdentityMismatch {
                expected: pending.target_ledger_id,
                actual: self.ledger_id,
            });
        }
        if pending.legacy_cursor != pending.legacy_evidence {
            return Err(StorageError::InvalidPath(
                "pending graph legacy reset cursor conflicts with its evidence".to_string(),
            ));
        }
        let evidence = self
            .tree
            .get(KEY_LEGACY_CURSOR_EVIDENCE)
            .map_err(to_storage_io)?
            .ok_or_else(|| {
                StorageError::InvalidPath(
                    "pending graph legacy reset is missing cursor evidence".to_string(),
                )
            })?;
        if decode_legacy_cursor(evidence.as_ref())? != pending.legacy_evidence {
            return Err(StorageError::InvalidPath(
                "pending graph legacy reset conflicts with persisted cursor evidence".to_string(),
            ));
        }
        if let Some(legacy) = self.tree.get(KEY_LEGACY_CURSOR).map_err(to_storage_io)? {
            if decode_legacy_cursor(legacy.as_ref())? != pending.legacy_cursor {
                return Err(StorageError::InvalidPath(
                    "pending graph legacy reset conflicts with the legacy cursor".to_string(),
                ));
            }
        }
        Ok(())
    }
}

fn decode_legacy_cursor(raw: &[u8]) -> Result<u64, StorageError> {
    let text = std::str::from_utf8(raw).map_err(|error| {
        StorageError::InvalidPath(format!("invalid graph legacy cursor encoding: {error}"))
    })?;
    text.parse::<u64>().map_err(|error| {
        StorageError::InvalidPath(format!("invalid graph legacy cursor value: {error}"))
    })
}

#[cfg(test)]
static FAIL_AFTER_PENDING_RESET_PERSISTED: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
fn fail_after_pending_reset_persisted() -> Result<(), StorageError> {
    if FAIL_AFTER_PENDING_RESET_PERSISTED.swap(false, Ordering::SeqCst) {
        return Err(StorageError::Unavailable(
            "injected crash after pending legacy projection reset persisted".to_string(),
        ));
    }
    Ok(())
}

#[cfg(not(test))]
fn fail_after_pending_reset_persisted() -> Result<(), StorageError> {
    Ok(())
}

fn to_storage_io(error: sled::Error) -> StorageError {
    StorageError::IoError(std::io::Error::other(error))
}

fn to_storage_data(error: serde_json::Error) -> StorageError {
    StorageError::InvalidPath(format!("invalid graph cursor payload: {error}"))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static FAILPOINT_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn stale_writer_cannot_advance_over_a_newer_graph_cursor() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let traversal = TraversalStore::new(db).unwrap();
        let ledger_id = "00000000-0000-0000-0000-000000000123".parse().unwrap();
        let first = GraphProjectionCursor::open(&traversal, ledger_id).unwrap();
        let stale = GraphProjectionCursor::open(&traversal, ledger_id).unwrap();

        assert_eq!(first.advance(0, 1).unwrap().after_seq, 1);
        assert!(matches!(
            stale.advance(0, 2),
            Err(StorageError::Backpressure(message))
                if message.contains("expected sequence 0")
        ));
        assert_eq!(stale.get().unwrap().after_seq, 1);
    }

    fn leave_pending_reset(
        traversal: &TraversalStore,
        target_ledger_id: LedgerIdentity,
    ) -> sled::Tree {
        traversal.set_last_reduced_seq(42).unwrap();
        let facts = traversal.db().open_tree("traversal_facts").unwrap();
        facts.insert("legacy-fact", "legacy-value").unwrap();
        let runtime_meta = traversal.db().open_tree(TREE_RUNTIME_META).unwrap();
        runtime_meta
            .insert(KEY_PENDING_DERIVED_EVENTS, "legacy-outbox")
            .unwrap();
        traversal.flush().unwrap();
        FAIL_AFTER_PENDING_RESET_PERSISTED.store(true, Ordering::SeqCst);

        assert!(matches!(
            GraphProjectionCursor::open(traversal, target_ledger_id),
            Err(StorageError::Unavailable(message))
                if message.contains("pending legacy projection reset")
        ));
        let pending: PendingLegacyProjectionReset = serde_json::from_slice(
            &runtime_meta
                .get(KEY_LEGACY_RESET_PENDING)
                .unwrap()
                .expect("failpoint leaves the typed reset marker"),
        )
        .unwrap();
        assert_eq!(pending.target_ledger_id, target_ledger_id);
        assert_eq!(pending.legacy_cursor, 42);
        assert_eq!(pending.legacy_evidence, 42);
        facts
    }

    #[test]
    fn same_identity_resumes_pending_legacy_projection_reset() {
        let _guard = FAILPOINT_TEST_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let traversal = TraversalStore::new(sled::open(temp.path()).unwrap()).unwrap();
        let ledger_id = LedgerIdentity::new();
        let facts = leave_pending_reset(&traversal, ledger_id);
        let runtime_meta = traversal.db().open_tree(TREE_RUNTIME_META).unwrap();

        let cursor = GraphProjectionCursor::open(&traversal, ledger_id).unwrap();

        assert_eq!(cursor.get().unwrap().after_seq, 0);
        assert!(facts.is_empty());
        assert!(runtime_meta
            .get(KEY_PENDING_DERIVED_EVENTS)
            .unwrap()
            .is_none());
        assert!(runtime_meta
            .get(KEY_LEGACY_RESET_PENDING)
            .unwrap()
            .is_none());
        assert_eq!(
            runtime_meta
                .get(KEY_LEGACY_CURSOR_EVIDENCE)
                .unwrap()
                .unwrap(),
            b"42".as_slice()
        );
    }

    #[test]
    fn foreign_identity_rejects_pending_reset_without_clearing_projection() {
        let _guard = FAILPOINT_TEST_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let traversal = TraversalStore::new(sled::open(temp.path()).unwrap()).unwrap();
        let target_ledger_id = LedgerIdentity::new();
        let foreign_ledger_id = LedgerIdentity::new();
        let facts = leave_pending_reset(&traversal, target_ledger_id);
        let runtime_meta = traversal.db().open_tree(TREE_RUNTIME_META).unwrap();

        assert!(matches!(
            GraphProjectionCursor::open(&traversal, foreign_ledger_id),
            Err(StorageError::IdentityMismatch { expected, actual })
                if expected == target_ledger_id && actual == foreign_ledger_id
        ));

        assert_eq!(
            facts.get("legacy-fact").unwrap().unwrap(),
            b"legacy-value".as_slice()
        );
        assert_eq!(
            runtime_meta
                .get(KEY_PENDING_DERIVED_EVENTS)
                .unwrap()
                .unwrap(),
            b"legacy-outbox".as_slice()
        );
        assert!(runtime_meta
            .get(KEY_LEGACY_RESET_PENDING)
            .unwrap()
            .is_some());
        assert!(runtime_meta.get(KEY_AUTHORITY_CURSOR).unwrap().is_none());
        assert_eq!(traversal.last_reduced_seq().unwrap(), 42);
    }
}
