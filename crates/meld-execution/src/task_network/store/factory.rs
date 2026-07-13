//! Factory for durable task network stores.

use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{codec::to_storage, SledTaskNetworkStore, TaskNetworkStoreError};

const OPEN_RETRY_ATTEMPTS: usize = 100;
const OPEN_RETRY_DELAY: Duration = Duration::from_millis(10);
const MAX_TASK_NETWORK_ID_BYTES: usize = 128;

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
        let db = open_sled_after_close(&self.root.join(format!("{storage_key}.sled")))?;
        SledTaskNetworkStore::open(db, network_id)
    }

    /// Return the root directory used for per-network databases.
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }
}

fn open_sled_after_close(path: &Path) -> Result<sled::Db, TaskNetworkStoreError> {
    open_sled_with_retry(|| sled::open(path))
}

fn open_sled_with_retry(
    mut open: impl FnMut() -> sled::Result<sled::Db>,
) -> Result<sled::Db, TaskNetworkStoreError> {
    for attempt in 0..OPEN_RETRY_ATTEMPTS {
        match open() {
            Ok(db) => return Ok(db),
            Err(error) if is_sled_lock_error(&error) && attempt + 1 < OPEN_RETRY_ATTEMPTS => {
                std::thread::sleep(OPEN_RETRY_DELAY);
            }
            Err(error) => return Err(to_storage(error)),
        }
    }
    unreachable!("the final open attempt returns its error")
}

fn is_sled_lock_error(error: &sled::Error) -> bool {
    matches!(error, sled::Error::Io(error) if error.to_string().contains("could not acquire lock"))
}

/// Return a filesystem-safe storage key for a task network id.
pub fn network_storage_key(network_id: &str) -> Result<String, TaskNetworkStoreError> {
    if network_id.is_empty() || network_id.len() > MAX_TASK_NETWORK_ID_BYTES {
        return Err(TaskNetworkStoreError::InvalidConfiguration(format!(
            "task network id must be non-empty and at most {MAX_TASK_NETWORK_ID_BYTES} bytes"
        )));
    }
    if !network_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.')
    {
        return Err(TaskNetworkStoreError::InvalidConfiguration(format!(
            "task network id '{network_id}' must contain only ASCII alphanumeric characters, dash, underscore, or dot"
        )));
    }
    Ok(network_id.to_string())
}

fn to_storage_error(error: impl ToString) -> TaskNetworkStoreError {
    TaskNetworkStoreError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sled_open_retries_only_lock_acquisition_errors() {
        let temp = tempfile::tempdir().unwrap();
        let mut attempts = 0;

        let db = open_sled_with_retry(|| {
            attempts += 1;
            if attempts < 4 {
                Err(sled::Error::Io(std::io::Error::other(
                    "could not acquire lock on test database",
                )))
            } else {
                sled::open(temp.path())
            }
        })
        .unwrap();

        assert_eq!(attempts, 4);
        db.insert(b"key", b"value").unwrap();
        db.flush().unwrap();
    }

    #[test]
    fn sled_open_does_not_retry_non_io_errors_with_lock_text() {
        let mut attempts = 0;

        let error = open_sled_with_retry(|| {
            attempts += 1;
            Err(sled::Error::Unsupported(
                "could not acquire lock on unsupported fixture".to_string(),
            ))
        })
        .expect_err("non-I/O errors must return immediately");

        assert_eq!(attempts, 1);
        assert!(matches!(error, TaskNetworkStoreError::CorruptStorage(_)));
    }

    #[test]
    fn sled_open_preserves_transient_io_classification() {
        let error = open_sled_with_retry(|| {
            Err(sled::Error::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "temporary open failure",
            )))
        })
        .expect_err("I/O errors without lock diagnostics must return immediately");

        assert!(matches!(error, TaskNetworkStoreError::Storage(_)));
    }
}
