//! Named consumer cursor registry for lag observability.
//!
//! Owner: event authority observability.
//! Inputs: consumer-reported durable positions.
//! Outputs: atomic, monotonic cursor mirrors bound to one ledger identity.
//! Does not own: authoritative consumer state or projection transactions.

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use crate::error::StorageError;
use crate::events::identity::LedgerIdentity;

const TREE_CONSUMER_CURSORS: &str = "event_consumer_cursors";

/// Observational registry of named consumer cursor positions.
#[derive(Clone)]
pub struct EventCursorRegistry {
    tree: Tree,
    binding: RegistryBinding,
}

#[derive(Clone, Copy)]
enum RegistryBinding {
    Legacy,
    Authority(LedgerIdentity),
}

/// One consumer's reported position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerCursor {
    /// Stable consumer name, such as `world_state.graph.reducer`.
    pub name: String,
    /// Highest ledger sequence the consumer has reported as durably reduced.
    pub reported_seq: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedConsumerCursor {
    ledger_id: LedgerIdentity,
    reported_seq: u64,
}

impl EventCursorRegistry {
    /// Opens the legacy unbound registry tree.
    ///
    /// TODO compat-shim: E5 removes this constructor after graph cursor
    /// rebuild and observability parity tests use the authority capability.
    pub fn open(db: &Db) -> Result<Self, StorageError> {
        Ok(Self {
            tree: db.open_tree(TREE_CONSUMER_CURSORS).map_err(to_storage_io)?,
            binding: RegistryBinding::Legacy,
        })
    }

    pub(crate) fn open_bound(db: &Db, ledger_id: LedgerIdentity) -> Result<Self, StorageError> {
        Ok(Self {
            tree: db.open_tree(TREE_CONSUMER_CURSORS).map_err(to_storage_io)?,
            binding: RegistryBinding::Authority(ledger_id),
        })
    }

    /// Explicitly relabels legacy eight-byte registry payloads when the caller
    /// has proved their sequence space belongs to `ledger_id`.
    ///
    /// TODO compat-shim: E5 removes this migration entry after legacy graph
    /// cursor rebuild and authority registry parity tests pass.
    pub fn migrate_legacy_payloads(
        db: &Db,
        ledger_id: LedgerIdentity,
    ) -> Result<Self, StorageError> {
        let registry = Self::open_bound(db, ledger_id)?;
        for result in registry.tree.iter() {
            let (key, raw) = result.map_err(to_storage_io)?;
            if raw.len() != 8 {
                let _ = decode_bound_cursor(&raw, ledger_id)?;
                continue;
            }
            let reported_seq = decode_legacy_seq(&raw)?;
            registry
                .tree
                .insert(
                    key,
                    encode_bound_cursor(PersistedConsumerCursor {
                        ledger_id,
                        reported_seq,
                    })?,
                )
                .map_err(to_storage_io)?;
        }
        registry.tree.flush().map_err(to_storage_io)?;
        Ok(registry)
    }

    /// Reports a consumer position, keeping the higher of old and new.
    pub fn report(&self, name: &str, seq: u64) -> Result<u64, StorageError> {
        let key = name.as_bytes();
        loop {
            let observed = self.tree.get(key).map_err(to_storage_io)?;
            let current = match observed.as_deref() {
                Some(raw) => self.decode_reported_seq(raw)?,
                None => 0,
            };
            if seq <= current {
                self.tree.flush().map_err(to_storage_io)?;
                return Ok(current);
            }
            let encoded = self.encode_reported_seq(seq)?;
            match self
                .tree
                .compare_and_swap(key, observed.as_deref(), Some(encoded.as_slice()))
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

    /// Returns one consumer's reported position when it has ever reported.
    pub fn get(&self, name: &str) -> Result<Option<u64>, StorageError> {
        let Some(raw) = self.tree.get(name.as_bytes()).map_err(to_storage_io)? else {
            return Ok(None);
        };
        self.decode_reported_seq(&raw).map(Some)
    }

    /// Returns every registered consumer in name order.
    pub fn snapshot(&self) -> Result<Vec<ConsumerCursor>, StorageError> {
        let mut out = Vec::new();
        for result in self.tree.iter() {
            let (key, raw) = result.map_err(to_storage_io)?;
            out.push(ConsumerCursor {
                name: String::from_utf8_lossy(&key).into_owned(),
                reported_seq: self.decode_reported_seq(&raw)?,
            });
        }
        Ok(out)
    }

    fn decode_reported_seq(&self, raw: &[u8]) -> Result<u64, StorageError> {
        match self.binding {
            RegistryBinding::Legacy => decode_legacy_seq(raw),
            RegistryBinding::Authority(ledger_id) => {
                decode_bound_cursor(raw, ledger_id).map(|cursor| cursor.reported_seq)
            }
        }
    }

    fn encode_reported_seq(&self, reported_seq: u64) -> Result<Vec<u8>, StorageError> {
        match self.binding {
            RegistryBinding::Legacy => Ok(reported_seq.to_be_bytes().to_vec()),
            RegistryBinding::Authority(ledger_id) => encode_bound_cursor(PersistedConsumerCursor {
                ledger_id,
                reported_seq,
            }),
        }
    }
}

fn encode_bound_cursor(cursor: PersistedConsumerCursor) -> Result<Vec<u8>, StorageError> {
    serde_json::to_vec(&cursor).map_err(to_storage_data)
}

fn decode_bound_cursor(
    raw: &[u8],
    expected: LedgerIdentity,
) -> Result<PersistedConsumerCursor, StorageError> {
    let cursor: PersistedConsumerCursor = serde_json::from_slice(raw).map_err(|error| {
        invalid_data(format!("invalid identity-bearing consumer cursor: {error}"))
    })?;
    if cursor.ledger_id != expected {
        return Err(StorageError::IdentityMismatch {
            expected,
            actual: cursor.ledger_id,
        });
    }
    Ok(cursor)
}

fn decode_legacy_seq(raw: &[u8]) -> Result<u64, StorageError> {
    let bytes: [u8; 8] = raw
        .try_into()
        .map_err(|_| invalid_data("invalid consumer cursor payload"))?;
    Ok(u64::from_be_bytes(bytes))
}

fn invalid_data(message: impl Into<String>) -> StorageError {
    StorageError::IoError(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        message.into(),
    ))
}

fn to_storage_data(error: serde_json::Error) -> StorageError {
    invalid_data(error.to_string())
}

fn to_storage_io(error: sled::Error) -> StorageError {
    StorageError::IoError(std::io::Error::other(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_reports_remain_monotonic_and_enumerable() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let registry = EventCursorRegistry::open(&db).unwrap();

        assert_eq!(registry.report("graph", 9).unwrap(), 9);
        assert_eq!(registry.report("graph", 4).unwrap(), 9);
        assert_eq!(registry.report("belief", 2).unwrap(), 2);
        assert_eq!(registry.snapshot().unwrap().len(), 2);
    }

    #[test]
    fn bound_registry_rejects_legacy_until_explicit_migration() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        EventCursorRegistry::open(&db)
            .unwrap()
            .report("graph", 7)
            .unwrap();
        let identity = LedgerIdentity::new();
        let bound = EventCursorRegistry::open_bound(&db, identity).unwrap();
        assert!(bound.get("graph").is_err());

        let migrated = EventCursorRegistry::migrate_legacy_payloads(&db, identity).unwrap();
        assert_eq!(migrated.get("graph").unwrap(), Some(7));
    }
}
