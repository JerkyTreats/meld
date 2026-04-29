use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum ExecutionInvariantError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Generation failed: {0}")]
    GenerationFailed(String),
}

pub type ApiError = ExecutionInvariantError;
