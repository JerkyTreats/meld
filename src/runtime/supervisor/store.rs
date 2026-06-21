//! Durable storage for root supervisor lifecycle records.

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Sled-backed root supervisor store.
///
/// This store is reserved for lifecycle records such as runtime instances,
/// leases, heartbeats, health snapshots, restart decisions, and shutdown
/// records. It must not contain domain progress cursors.
#[derive(Clone)]
pub struct SupervisorStore {
    path: PathBuf,
    db: sled::Db,
}

/// Error returned while opening or flushing supervisor storage.
#[derive(Debug, Error)]
pub enum SupervisorStoreError {
    /// Filesystem setup failed before sled opened.
    #[error("supervisor store io error: {0}")]
    Io(String),
    /// Sled returned an open or flush error.
    #[error("supervisor store sled error: {0}")]
    Sled(String),
}

impl SupervisorStore {
    /// Open supervisor lifecycle storage at the supplied path.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, SupervisorStoreError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(to_io)?;
        }
        let db = sled::open(&path).map_err(to_sled)?;
        Ok(Self { path, db })
    }

    /// Return the filesystem path used by this supervisor store.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Flush supervisor lifecycle records separately from product stores.
    pub fn flush(&self) -> Result<(), SupervisorStoreError> {
        self.db.flush().map_err(to_sled)?;
        Ok(())
    }
}

fn to_io(error: impl ToString) -> SupervisorStoreError {
    SupervisorStoreError::Io(error.to_string())
}

fn to_sled(error: impl ToString) -> SupervisorStoreError {
    SupervisorStoreError::Sled(error.to_string())
}
