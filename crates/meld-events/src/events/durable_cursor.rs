//! Authoritative durable consumer cursor storage.
//!
//! Owner: event authority. This tree is the durable authority behind the
//! [`crate::events::consumer::DurableConsumerCursor`] contract. It is
//! deliberately parallel to the observational
//! [`crate::events::registry::EventCursorRegistry`] mirror: consumers that
//! only report into the mirror (such as the graph reducer) gain no
//! authoritative state here, so mirror semantics can never silently become
//! authoritative. Only consumers advanced through the durable cursor
//! contract own rows in this tree.
//!
//! Invariants: every row is bound to one ledger identity, advancement is
//! monotonic (a request behind the durable position writes nothing and
//! returns the current state), and every accepted advancement is flushed
//! before it is acknowledged so a reopen between consumer commit and cursor
//! commit replays idempotently.

use sled::{Db, Tree};

use crate::error::StorageError;
use crate::events::consumer::ConsumerCursorState;
use crate::events::identity::LedgerIdentity;

const TREE_DURABLE_CONSUMER_CURSORS: &str = "event_durable_consumer_cursors";

/// Identity-bound durable cursor rows for explicitly advanced consumers.
#[derive(Clone)]
pub(crate) struct DurableCursorTree {
    tree: Tree,
    ledger_id: LedgerIdentity,
}

impl DurableCursorTree {
    /// Opens the authoritative cursor tree bound to one ledger identity.
    pub(crate) fn open(db: &Db, ledger_id: LedgerIdentity) -> Result<Self, StorageError> {
        Ok(Self {
            tree: db
                .open_tree(TREE_DURABLE_CONSUMER_CURSORS)
                .map_err(to_storage_io)?,
            ledger_id,
        })
    }

    /// Returns one consumer's durable position when it has ever advanced.
    pub(crate) fn get(
        &self,
        consumer_id: &str,
    ) -> Result<Option<ConsumerCursorState>, StorageError> {
        validate_consumer_id(consumer_id)?;
        let Some(raw) = self
            .tree
            .get(consumer_id.as_bytes())
            .map_err(to_storage_io)?
        else {
            return Ok(None);
        };
        decode_state(&raw, self.ledger_id).map(Some)
    }

    /// Advances one consumer's durable position monotonically.
    ///
    /// A request behind or at the durable position is a no-op that returns
    /// the current state without writing. An accepted advancement is flushed
    /// before it is returned.
    pub(crate) fn advance(
        &self,
        consumer_id: &str,
        after_seq: u64,
    ) -> Result<ConsumerCursorState, StorageError> {
        validate_consumer_id(consumer_id)?;
        let key = consumer_id.as_bytes();
        loop {
            let observed = self.tree.get(key).map_err(to_storage_io)?;
            if let Some(raw) = observed.as_deref() {
                let current = decode_state(raw, self.ledger_id)?;
                if after_seq <= current.after_seq {
                    return Ok(current);
                }
            }
            let next = ConsumerCursorState {
                ledger_id: self.ledger_id,
                after_seq,
            };
            let encoded = serde_json::to_vec(&next).map_err(to_storage_data)?;
            match self
                .tree
                .compare_and_swap(key, observed.as_deref(), Some(encoded.as_slice()))
                .map_err(to_storage_io)?
            {
                Ok(()) => {
                    // Durable-before-acknowledged: the caller treats the
                    // returned state as safe to build replay resumption on.
                    self.tree.flush().map_err(to_storage_io)?;
                    return Ok(next);
                }
                // Another advancement raced this one; re-read and re-apply
                // the monotonic rule against the new durable position.
                Err(_) => continue,
            }
        }
    }
}

fn validate_consumer_id(consumer_id: &str) -> Result<(), StorageError> {
    if consumer_id.trim().is_empty() {
        return Err(StorageError::InvalidPath(
            "consumer id must be non-empty".to_string(),
        ));
    }
    Ok(())
}

fn decode_state(raw: &[u8], expected: LedgerIdentity) -> Result<ConsumerCursorState, StorageError> {
    let state: ConsumerCursorState = serde_json::from_slice(raw).map_err(|error| {
        invalid_data(format!("invalid durable consumer cursor payload: {error}"))
    })?;
    if state.ledger_id != expected {
        return Err(StorageError::IdentityMismatch {
            expected,
            actual: state.ledger_id,
        });
    }
    Ok(state)
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
    fn advance_is_monotonic_and_behind_requests_return_current_state() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let tree = DurableCursorTree::open(&db, LedgerIdentity::new()).unwrap();

        assert_eq!(tree.get("consumer").unwrap(), None);
        assert_eq!(tree.advance("consumer", 5).unwrap().after_seq, 5);
        assert_eq!(tree.advance("consumer", 3).unwrap().after_seq, 5);
        assert_eq!(tree.advance("consumer", 5).unwrap().after_seq, 5);
        assert_eq!(tree.advance("consumer", 8).unwrap().after_seq, 8);
        assert_eq!(tree.get("consumer").unwrap().unwrap().after_seq, 8);
    }

    #[test]
    fn rows_from_another_ledger_identity_are_rejected() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let first = LedgerIdentity::new();
        DurableCursorTree::open(&db, first)
            .unwrap()
            .advance("consumer", 4)
            .unwrap();

        let other = DurableCursorTree::open(&db, LedgerIdentity::new()).unwrap();
        assert!(matches!(
            other.get("consumer"),
            Err(StorageError::IdentityMismatch { .. })
        ));
    }

    #[test]
    fn empty_consumer_id_is_rejected() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let tree = DurableCursorTree::open(&db, LedgerIdentity::new()).unwrap();
        assert!(tree.advance(" ", 1).is_err());
        assert!(tree.get("").is_err());
    }
}
