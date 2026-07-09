//! Named consumer cursor registry for lag observability.
//!
//! Owner: event observability.
//! Inputs: consumer-reported cursor positions keyed by stable consumer names.
//! Outputs: an enumerable snapshot of consumer positions for lag computation
//! and, later, compaction boundary selection.
//! Does not own: authoritative consumer progress. The registry is an
//! observational mirror; the durable cursor of record stays in the consumer's
//! own store, and consumers report here after their state is durable.

use sled::{Db, Tree};

use crate::error::StorageError;

const TREE_CONSUMER_CURSORS: &str = "event_consumer_cursors";

/// Observational registry of named consumer cursor positions.
///
/// Reports are monotonic per name so restarts and replays can never make a
/// consumer appear to move backwards. Registration happens on first report;
/// a consumer that has never reported is absent from snapshots, and `get`
/// distinguishes that absence from a cursor at zero.
#[derive(Clone)]
pub struct EventCursorRegistry {
    tree: Tree,
}

/// One consumer's reported position.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConsumerCursor {
    /// Stable consumer name, such as `world_state.graph.reducer`.
    pub name: String,
    /// Highest ledger sequence the consumer has reported as durably reduced.
    pub reported_seq: u64,
}

impl EventCursorRegistry {
    /// Opens the registry tree on the ledger database.
    pub fn open(db: &Db) -> Result<Self, StorageError> {
        Ok(Self {
            tree: db.open_tree(TREE_CONSUMER_CURSORS).map_err(to_storage_io)?,
        })
    }

    /// Reports a consumer position, keeping the higher of old and new.
    ///
    /// The merge is atomic, so concurrent reporters for one name can never
    /// persist a regression.
    pub fn report(&self, name: &str, seq: u64) -> Result<u64, StorageError> {
        let merged = self
            .tree
            .update_and_fetch(name.as_bytes(), |current| {
                let current = current.and_then(|raw| decode_seq(raw).ok()).unwrap_or(0);
                Some(seq.max(current).to_be_bytes().to_vec())
            })
            .map_err(to_storage_io)?;
        match merged {
            Some(raw) => decode_seq(&raw),
            None => Ok(seq),
        }
    }

    /// Returns one consumer's reported position when it has ever reported.
    pub fn get(&self, name: &str) -> Result<Option<u64>, StorageError> {
        let Some(raw) = self.tree.get(name.as_bytes()).map_err(to_storage_io)? else {
            return Ok(None);
        };
        decode_seq(&raw).map(Some)
    }

    /// Returns every registered consumer in name order.
    pub fn snapshot(&self) -> Result<Vec<ConsumerCursor>, StorageError> {
        let mut out = Vec::new();
        for result in self.tree.iter() {
            let (key, raw) = result.map_err(to_storage_io)?;
            let name = String::from_utf8_lossy(&key).into_owned();
            out.push(ConsumerCursor {
                name,
                reported_seq: decode_seq(&raw)?,
            });
        }
        Ok(out)
    }
}

fn decode_seq(raw: &[u8]) -> Result<u64, StorageError> {
    let bytes: [u8; 8] = raw.try_into().map_err(|_| {
        StorageError::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid consumer cursor payload",
        ))
    })?;
    Ok(u64::from_be_bytes(bytes))
}

fn to_storage_io(err: sled::Error) -> StorageError {
    StorageError::IoError(std::io::Error::other(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_are_monotonic_and_enumerable() {
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let registry = EventCursorRegistry::open(&db).unwrap();

        assert_eq!(registry.get("graph").unwrap(), None);
        assert_eq!(registry.report("graph", 9).unwrap(), 9);
        assert_eq!(registry.report("graph", 4).unwrap(), 9);
        assert_eq!(registry.report("belief", 2).unwrap(), 2);

        let snapshot = registry.snapshot().unwrap();
        assert_eq!(
            snapshot,
            vec![
                ConsumerCursor {
                    name: "belief".to_string(),
                    reported_seq: 2,
                },
                ConsumerCursor {
                    name: "graph".to_string(),
                    reported_seq: 9,
                },
            ]
        );
    }

    #[test]
    fn positions_survive_reopen() {
        let dir = tempfile::TempDir::new().unwrap();
        {
            let db = sled::open(dir.path()).unwrap();
            EventCursorRegistry::open(&db)
                .unwrap()
                .report("graph", 7)
                .unwrap();
            db.flush().unwrap();
        }
        let db = sled::open(dir.path()).unwrap();
        let registry = EventCursorRegistry::open(&db).unwrap();
        assert_eq!(registry.get("graph").unwrap(), Some(7));
    }
}
