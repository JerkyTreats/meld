//! Factory for durable task network stores.

use std::path::PathBuf;

use super::{SledTaskNetworkStore, TaskNetworkStoreError};

/// Opens durable task network stores below a product-owned root directory.
///
/// Each network gets a separate sled database because current task network tree
/// keys are scoped to a single network store.
#[derive(Clone)]
pub struct TaskNetworkStoreFactory {
    root: PathBuf,
}

impl TaskNetworkStoreFactory {
    /// Create a factory rooted at a product-owned task network directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Open one durable task network by stable network id.
    pub fn open_network(
        &self,
        network_id: impl Into<String>,
    ) -> Result<SledTaskNetworkStore, TaskNetworkStoreError> {
        let network_id = network_id.into();
        let storage_key = network_storage_key(&network_id)?;
        std::fs::create_dir_all(&self.root).map_err(to_storage_error)?;
        let db =
            sled::open(self.root.join(format!("{storage_key}.sled"))).map_err(to_storage_error)?;
        SledTaskNetworkStore::open(db, network_id)
    }

    /// Return the root directory used for per-network databases.
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }
}

/// Return a filesystem-safe storage key for a task network id.
pub fn network_storage_key(network_id: &str) -> Result<String, TaskNetworkStoreError> {
    if network_id.is_empty() {
        return Err(TaskNetworkStoreError::Storage(
            "task network id must not be empty".to_string(),
        ));
    }
    if !network_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
    {
        return Err(TaskNetworkStoreError::Storage(format!(
            "task network id '{network_id}' must contain only ASCII alphanumeric characters, dash, underscore, or dot"
        )));
    }
    Ok(network_id.to_string())
}

fn to_storage_error(error: impl ToString) -> TaskNetworkStoreError {
    TaskNetworkStoreError::Storage(error.to_string())
}
