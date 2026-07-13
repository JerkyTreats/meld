//! Task network storage errors.

use thiserror::Error;

/// Error returned by task network store operations.
#[derive(Debug, Error)]
pub enum TaskNetworkStoreError {
    /// Caller supplied invalid task network storage configuration.
    #[error("invalid task network storage configuration: {0}")]
    InvalidConfiguration(String),
    /// Sled returned an error.
    #[error("task network storage error: {0}")]
    Storage(String),
    /// Durable storage reported corruption or a nonrecoverable engine failure.
    #[error("task network corrupt storage error: {0}")]
    CorruptStorage(String),
    /// Persisted data could not be decoded.
    #[error("task network decode error: {0}")]
    Decode(String),
}

#[derive(Debug)]
pub(crate) enum AuthorityStoreError {
    StaleEpoch { expected: u64, actual: u64 },
    Store(TaskNetworkStoreError),
}

impl AuthorityStoreError {
    pub(crate) fn into_store_error(self) -> TaskNetworkStoreError {
        match self {
            Self::StaleEpoch { expected, actual } => TaskNetworkStoreError::Storage(format!(
                "task network authority epoch is stale: expected {expected}, actual {actual}"
            )),
            Self::Store(error) => error,
        }
    }
}

impl From<TaskNetworkStoreError> for AuthorityStoreError {
    fn from(error: TaskNetworkStoreError) -> Self {
        Self::Store(error)
    }
}
