use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Backpressure: {0}")]
    Backpressure(String),

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

#[derive(Debug, Error, Clone)]
pub enum ApiError {
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}
