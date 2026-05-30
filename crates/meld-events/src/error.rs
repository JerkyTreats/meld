use thiserror::Error;

/// Storage-layer failures raised by event persistence and validation.
#[derive(Debug, Error)]
pub enum StorageError {
    /// A persisted path, identifier, or domain object coordinate is invalid.
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// The non-blocking event bus cannot accept more events right now.
    #[error("Backpressure: {0}")]
    Backpressure(String),

    /// Sled or serialization I/O failed while reading or writing event data.
    #[error("Storage I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

impl Clone for StorageError {
    fn clone(&self) -> Self {
        match self {
            StorageError::InvalidPath(path) => StorageError::InvalidPath(path.clone()),
            StorageError::Backpressure(message) => StorageError::Backpressure(message.clone()),
            StorageError::IoError(err) => {
                StorageError::IoError(std::io::Error::new(err.kind(), err.to_string()))
            }
        }
    }
}

/// API-facing event runtime failures.
#[derive(Debug, Error, Clone)]
pub enum ApiError {
    /// The runtime could not persist or validate an event.
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),

    /// The runtime was configured with unsupported or incomplete input.
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
