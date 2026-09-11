//! Error types for the Merkle filesystem state management system.

use crate::metadata::frame_key_descriptor::FrameMetadataMutabilityClass;
use crate::types::{FrameID, Hash, NodeID};
use thiserror::Error;

/// Storage-related errors
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Node not found: {0:?}")]
    NodeNotFound(NodeID),

    #[error("Frame not found: {0:?}")]
    FrameNotFound(FrameID),

    #[error("Hash mismatch: expected {expected:?}, got {actual:?}")]
    HashMismatch { expected: Hash, actual: Hash },

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Backpressure: {0}")]
    Backpressure(String),

    #[error("Event authority unavailable: {0}")]
    EventAuthorityUnavailable(String),

    #[error("Durability indeterminate: {0}")]
    DurabilityIndeterminate(String),

    #[error("Event migration conflict: {0}")]
    MigrationConflict(String),

    #[error("Ledger identity mismatch: expected {expected}, got {actual}")]
    LedgerIdentityMismatch { expected: String, actual: String },

    #[error(
        "Retention gap: cursor {after_seq} predates retained history starting at {retained_from}"
    )]
    RetentionGap { after_seq: u64, retained_from: u64 },

    #[error("Storage I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

impl Clone for StorageError {
    fn clone(&self) -> Self {
        match self {
            StorageError::NodeNotFound(node_id) => StorageError::NodeNotFound(*node_id),
            StorageError::FrameNotFound(frame_id) => StorageError::FrameNotFound(*frame_id),
            StorageError::HashMismatch { expected, actual } => StorageError::HashMismatch {
                expected: *expected,
                actual: *actual,
            },
            StorageError::InvalidPath(path) => StorageError::InvalidPath(path.clone()),
            StorageError::Backpressure(message) => StorageError::Backpressure(message.clone()),
            StorageError::EventAuthorityUnavailable(message) => {
                StorageError::EventAuthorityUnavailable(message.clone())
            }
            StorageError::DurabilityIndeterminate(message) => {
                StorageError::DurabilityIndeterminate(message.clone())
            }
            StorageError::MigrationConflict(message) => {
                StorageError::MigrationConflict(message.clone())
            }
            StorageError::LedgerIdentityMismatch { expected, actual } => {
                StorageError::LedgerIdentityMismatch {
                    expected: expected.clone(),
                    actual: actual.clone(),
                }
            }
            StorageError::RetentionGap {
                after_seq,
                retained_from,
            } => StorageError::RetentionGap {
                after_seq: *after_seq,
                retained_from: *retained_from,
            },
            StorageError::IoError(err) => {
                StorageError::IoError(std::io::Error::new(err.kind(), err.to_string()))
            }
        }
    }
}

/// API-related errors for Phase 2
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Node not found: {0:?}")]
    NodeNotFound(NodeID),

    #[error("Frame not found: {0:?}")]
    FrameNotFound(FrameID),

    #[error("Agent unauthorized: {0}")]
    Unauthorized(String),

    #[error("Invalid frame: {0}")]
    InvalidFrame(String),

    #[error("Frame metadata policy violation: {0}")]
    FrameMetadataPolicyViolation(String),

    #[error("Frame metadata contains unknown key: {key}")]
    FrameMetadataUnknownKey { key: String },

    #[error("Frame metadata contains forbidden key: {key}")]
    FrameMetadataForbiddenKey { key: String },

    #[error("Frame metadata missing required key: {key}")]
    FrameMetadataMissingRequiredKey { key: String },

    #[error(
        "Frame metadata value for key '{key}' exceeds per-key budget: {actual_bytes} > {max_bytes} bytes"
    )]
    FrameMetadataPerKeyBudgetExceeded {
        key: String,
        actual_bytes: usize,
        max_bytes: usize,
    },

    #[error("Frame metadata total payload exceeds budget: {actual_bytes} > {max_bytes} bytes")]
    FrameMetadataTotalBudgetExceeded {
        actual_bytes: usize,
        max_bytes: usize,
    },

    #[error("Frame metadata key '{key}' violates immutable mutability class: {class:?}")]
    FrameMetadataMutabilityViolation {
        key: String,
        class: FrameMetadataMutabilityClass,
    },

    #[error("Prompt context artifact for kind '{kind}' exceeds budget: {actual_bytes} > {max_bytes} bytes")]
    PromptContextArtifactBudgetExceeded {
        kind: String,
        actual_bytes: usize,
        max_bytes: usize,
    },

    #[error("Prompt context artifact not found: {artifact_id}")]
    PromptContextArtifactNotFound { artifact_id: String },

    #[error(
        "Prompt context artifact digest mismatch for '{artifact_id}': expected {expected_digest}, got {actual_digest}"
    )]
    PromptContextArtifactDigestMismatch {
        artifact_id: String,
        expected_digest: String,
        actual_digest: String,
    },

    #[error(
        "Prompt context artifact size mismatch for '{artifact_id}': expected {expected_bytes}, got {actual_bytes}"
    )]
    PromptContextArtifactSizeMismatch {
        artifact_id: String,
        expected_bytes: usize,
        actual_bytes: usize,
    },

    #[error("Prompt link contract is invalid: {reason}")]
    PromptLinkContractInvalid { reason: String },

    #[error("Workflow record contract is invalid for '{record_type}': {reason}")]
    WorkflowRecordContractInvalid { record_type: String, reason: String },

    #[error("Workflow record reference is invalid for '{record_type}': {reason}")]
    WorkflowRecordReferenceInvalid { record_type: String, reason: String },

    #[error("Agent '{agent_id}' missing required prompt contract field '{field}'")]
    MissingPromptContractField {
        agent_id: String,
        field: &'static str,
    },

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),

    #[error("Provider request failed: {0}")]
    ProviderRequestFailed(String),

    /// Provider-owned execution evidence retained when no completion is accepted.
    #[error("Provider execution failed: {message}")]
    ProviderExecutionFailed {
        message: String,
        metadata: std::collections::BTreeMap<String, serde_json::Value>,
    },

    /// The provider rejected the request before producing a completion.
    #[error("Provider rejected request with HTTP {status}: {message}")]
    ProviderRequestRejected { status: u16, message: String },

    #[error("Provider authentication failed: {0}")]
    ProviderAuthFailed(String),

    #[error("Provider rate limit exceeded: {0}")]
    ProviderRateLimit(String),

    #[error("Provider model not found: {0}")]
    ProviderModelNotFound(String),

    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error(
        "Product runtime storage cannot use the target workspace: old path {old_path}; resolved or replacement path {new_path}"
    )]
    ProductRootInsideWorkspace {
        old_path: std::path::PathBuf,
        new_path: std::path::PathBuf,
    },

    /// An owner has determined that this invocation cannot complete by retrying.
    #[error("Terminal capability failure: {0}")]
    TerminalCapabilityFailure(#[source] Box<ApiError>),

    #[error("Generation failed: {0}")]
    GenerationFailed(String),

    #[error("Path not found in tree: {0}. Run `meld scan` to update tree or start `meld watch`.")]
    PathNotInTree(std::path::PathBuf),
}

impl Clone for ApiError {
    fn clone(&self) -> Self {
        match self {
            ApiError::NodeNotFound(node_id) => ApiError::NodeNotFound(*node_id),
            ApiError::FrameNotFound(frame_id) => ApiError::FrameNotFound(*frame_id),
            ApiError::Unauthorized(message) => ApiError::Unauthorized(message.clone()),
            ApiError::InvalidFrame(message) => ApiError::InvalidFrame(message.clone()),
            ApiError::FrameMetadataPolicyViolation(message) => {
                ApiError::FrameMetadataPolicyViolation(message.clone())
            }
            ApiError::FrameMetadataUnknownKey { key } => {
                ApiError::FrameMetadataUnknownKey { key: key.clone() }
            }
            ApiError::FrameMetadataForbiddenKey { key } => {
                ApiError::FrameMetadataForbiddenKey { key: key.clone() }
            }
            ApiError::FrameMetadataMissingRequiredKey { key } => {
                ApiError::FrameMetadataMissingRequiredKey { key: key.clone() }
            }
            ApiError::FrameMetadataPerKeyBudgetExceeded {
                key,
                actual_bytes,
                max_bytes,
            } => ApiError::FrameMetadataPerKeyBudgetExceeded {
                key: key.clone(),
                actual_bytes: *actual_bytes,
                max_bytes: *max_bytes,
            },
            ApiError::FrameMetadataTotalBudgetExceeded {
                actual_bytes,
                max_bytes,
            } => ApiError::FrameMetadataTotalBudgetExceeded {
                actual_bytes: *actual_bytes,
                max_bytes: *max_bytes,
            },
            ApiError::FrameMetadataMutabilityViolation { key, class } => {
                ApiError::FrameMetadataMutabilityViolation {
                    key: key.clone(),
                    class: *class,
                }
            }
            ApiError::PromptContextArtifactBudgetExceeded {
                kind,
                actual_bytes,
                max_bytes,
            } => ApiError::PromptContextArtifactBudgetExceeded {
                kind: kind.clone(),
                actual_bytes: *actual_bytes,
                max_bytes: *max_bytes,
            },
            ApiError::PromptContextArtifactNotFound { artifact_id } => {
                ApiError::PromptContextArtifactNotFound {
                    artifact_id: artifact_id.clone(),
                }
            }
            ApiError::PromptContextArtifactDigestMismatch {
                artifact_id,
                expected_digest,
                actual_digest,
            } => ApiError::PromptContextArtifactDigestMismatch {
                artifact_id: artifact_id.clone(),
                expected_digest: expected_digest.clone(),
                actual_digest: actual_digest.clone(),
            },
            ApiError::PromptContextArtifactSizeMismatch {
                artifact_id,
                expected_bytes,
                actual_bytes,
            } => ApiError::PromptContextArtifactSizeMismatch {
                artifact_id: artifact_id.clone(),
                expected_bytes: *expected_bytes,
                actual_bytes: *actual_bytes,
            },
            ApiError::PromptLinkContractInvalid { reason } => ApiError::PromptLinkContractInvalid {
                reason: reason.clone(),
            },
            ApiError::WorkflowRecordContractInvalid {
                record_type,
                reason,
            } => ApiError::WorkflowRecordContractInvalid {
                record_type: record_type.clone(),
                reason: reason.clone(),
            },
            ApiError::WorkflowRecordReferenceInvalid {
                record_type,
                reason,
            } => ApiError::WorkflowRecordReferenceInvalid {
                record_type: record_type.clone(),
                reason: reason.clone(),
            },
            ApiError::MissingPromptContractField { agent_id, field } => {
                ApiError::MissingPromptContractField {
                    agent_id: agent_id.clone(),
                    field,
                }
            }
            ApiError::ProviderError(message) => ApiError::ProviderError(message.clone()),
            ApiError::ProviderNotConfigured(message) => {
                ApiError::ProviderNotConfigured(message.clone())
            }
            ApiError::ProviderRequestFailed(message) => {
                ApiError::ProviderRequestFailed(message.clone())
            }
            ApiError::ProviderExecutionFailed { message, metadata } => {
                ApiError::ProviderExecutionFailed {
                    message: message.clone(),
                    metadata: metadata.clone(),
                }
            }
            ApiError::ProviderRequestRejected { status, message } => {
                ApiError::ProviderRequestRejected {
                    status: *status,
                    message: message.clone(),
                }
            }
            ApiError::ProviderAuthFailed(message) => ApiError::ProviderAuthFailed(message.clone()),
            ApiError::ProviderRateLimit(message) => ApiError::ProviderRateLimit(message.clone()),
            ApiError::ProviderModelNotFound(message) => {
                ApiError::ProviderModelNotFound(message.clone())
            }
            ApiError::StorageError(err) => ApiError::StorageError(err.clone()),
            ApiError::ConfigError(message) => ApiError::ConfigError(message.clone()),
            ApiError::ProductRootInsideWorkspace { old_path, new_path } => {
                ApiError::ProductRootInsideWorkspace {
                    old_path: old_path.clone(),
                    new_path: new_path.clone(),
                }
            }
            ApiError::TerminalCapabilityFailure(error) => {
                ApiError::TerminalCapabilityFailure(error.clone())
            }
            ApiError::GenerationFailed(message) => ApiError::GenerationFailed(message.clone()),
            ApiError::PathNotInTree(path) => ApiError::PathNotInTree(path.clone()),
        }
    }
}

impl From<config::ConfigError> for ApiError {
    fn from(err: config::ConfigError) -> Self {
        ApiError::ConfigError(err.to_string())
    }
}

impl From<meld_events::error::StorageError> for StorageError {
    fn from(err: meld_events::error::StorageError) -> Self {
        match err {
            meld_events::error::StorageError::InvalidPath(path) => StorageError::InvalidPath(path),
            meld_events::error::StorageError::Backpressure(message) => {
                StorageError::Backpressure(message)
            }
            meld_events::error::StorageError::Unavailable(message) => {
                StorageError::EventAuthorityUnavailable(message)
            }
            meld_events::error::StorageError::DurabilityIndeterminate(message) => {
                StorageError::DurabilityIndeterminate(message)
            }
            meld_events::error::StorageError::MigrationConflict(message) => {
                StorageError::MigrationConflict(message)
            }
            meld_events::error::StorageError::IdentityMismatch { expected, actual } => {
                StorageError::LedgerIdentityMismatch {
                    expected: expected.to_string(),
                    actual: actual.to_string(),
                }
            }
            meld_events::error::StorageError::RetentionGap {
                after_seq,
                retained_from,
            } => StorageError::RetentionGap {
                after_seq,
                retained_from,
            },
            meld_events::error::StorageError::IoError(err) => StorageError::IoError(err),
        }
    }
}

impl From<meld_events::error::ApiError> for ApiError {
    fn from(err: meld_events::error::ApiError) -> Self {
        match err {
            meld_events::error::ApiError::StorageError(err) => {
                ApiError::StorageError(StorageError::from(err))
            }
            meld_events::error::ApiError::ConfigError(message) => ApiError::ConfigError(message),
        }
    }
}

impl From<meld_execution::error::ExecutionInvariantError> for ApiError {
    fn from(err: meld_execution::error::ExecutionInvariantError) -> Self {
        match err {
            meld_execution::error::ExecutionInvariantError::ConfigError(message) => {
                ApiError::ConfigError(message)
            }
            meld_execution::error::ExecutionInvariantError::GenerationFailed(message) => {
                ApiError::GenerationFailed(message)
            }
        }
    }
}

impl From<meld_events::error::StorageError> for ApiError {
    fn from(err: meld_events::error::StorageError) -> Self {
        ApiError::StorageError(StorageError::from(err))
    }
}
