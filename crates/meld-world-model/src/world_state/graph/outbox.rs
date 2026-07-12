//! Durable retry state for graph-derived event publication.

use sled::Tree;

use crate::error::StorageError;
use crate::events::EventEnvelope;

const TREE_RUNTIME_META: &str = "traversal_runtime_meta";
const KEY_PENDING_DERIVED_EVENTS: &[u8] = b"pending_derived_events";

pub(super) struct GraphDerivedOutbox {
    tree: Tree,
}

impl GraphDerivedOutbox {
    pub(super) fn open(db: &sled::Db) -> Result<Self, StorageError> {
        Ok(Self {
            tree: db.open_tree(TREE_RUNTIME_META).map_err(to_storage_io)?,
        })
    }

    pub(super) fn pending(&self) -> Result<Vec<EventEnvelope>, StorageError> {
        let Some(raw) = self
            .tree
            .get(KEY_PENDING_DERIVED_EVENTS)
            .map_err(to_storage_io)?
        else {
            return Ok(Vec::new());
        };
        serde_json::from_slice(&raw).map_err(to_storage_data)
    }

    pub(super) fn replace(&self, envelopes: &[EventEnvelope]) -> Result<(), StorageError> {
        if envelopes.is_empty() {
            self.tree
                .remove(KEY_PENDING_DERIVED_EVENTS)
                .map_err(to_storage_io)?;
        } else {
            self.tree
                .insert(
                    KEY_PENDING_DERIVED_EVENTS,
                    serde_json::to_vec(envelopes).map_err(to_storage_data)?,
                )
                .map_err(to_storage_io)?;
        }
        self.tree.flush().map_err(to_storage_io)?;
        Ok(())
    }
}

fn to_storage_io(error: sled::Error) -> StorageError {
    StorageError::IoError(std::io::Error::other(error))
}

fn to_storage_data(error: serde_json::Error) -> StorageError {
    StorageError::InvalidPath(format!("invalid graph derived-event outbox: {error}"))
}
