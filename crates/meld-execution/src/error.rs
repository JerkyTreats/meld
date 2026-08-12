//! Execution-domain error aliases and invariant failures.

use thiserror::Error;

/// Stable marker for a capability failure that cannot improve by replaying
/// the same fenced invocation.
pub const TERMINAL_CAPABILITY_FAILURE_MARKER: &str = "Terminal capability failure";

/// Execution-domain error used for invalid contracts and failed generation paths.
#[derive(Debug, Error, Clone)]
pub enum ExecutionInvariantError {
    /// Invalid configuration, contract, or adapter input.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Provider generation or execution orchestration failed.
    #[error("Generation failed: {0}")]
    GenerationFailed(String),
}

/// Shared execution API error type used by this crate.
pub type ApiError = ExecutionInvariantError;
