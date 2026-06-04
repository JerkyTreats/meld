//! Task network storage errors.

use thiserror::Error;

/// Error returned by task network store operations.
#[derive(Debug, Error)]
pub enum TaskNetworkStoreError {
    /// Sled returned an error.
    #[error("task network storage error: {0}")]
    Storage(String),
    /// Persisted data could not be decoded.
    #[error("task network decode error: {0}")]
    Decode(String),
}
