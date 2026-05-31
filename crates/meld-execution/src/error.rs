//! Execution-domain error aliases and invariant failures.

use thiserror::Error;

/// Execution-domain error used for invalid contracts and failed generation paths.
#[derive(Debug, Error, Clone)]
pub enum ExecutionInvariantError {
    /// Config error variant for this execution contract.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Generation failed variant for this execution contract.
    #[error("Generation failed: {0}")]
    GenerationFailed(String),
}

/// Shared execution API error type used by this crate.
pub type ApiError = ExecutionInvariantError;
