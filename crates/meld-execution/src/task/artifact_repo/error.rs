//! Task artifact repository storage errors.

use thiserror::Error;

/// Error returned by durable task artifact repository operations.
#[derive(Debug, Error)]
pub enum TaskArtifactRepoError {
    /// Sled returned an error.
    #[error("task artifact repo storage error: {0}")]
    Storage(String),
    /// Persisted data could not be decoded or validated.
    #[error("task artifact repo decode error: {0}")]
    Decode(String),
    /// Repository invariants were violated during a write.
    #[error("task artifact repo invariant error: {0}")]
    Invariant(String),
}
