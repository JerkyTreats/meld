use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum ApiError {
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
