//! Event-domain error contracts.
//!
//! Owner: event runtime and store.
//! Inputs: persistence, validation, and bus emission failures.
//! Outputs: storage errors for low-level callers and API errors for runtime
//! facades.
//! Does not own: this module does not classify task, workflow, provider, or
//! world model failures.

use thiserror::Error;

use crate::events::identity::LedgerIdentity;

/// Storage-layer failures raised by event persistence and validation.
#[derive(Debug, Error)]
pub enum StorageError {
    /// A persisted path, identifier, or domain object coordinate is invalid.
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// The non-blocking event bus cannot accept more events right now.
    #[error("Backpressure: {0}")]
    Backpressure(String),

    /// The in-process authority ingress is stopped or disconnected.
    #[error("Event authority unavailable: {0}")]
    Unavailable(String),

    /// A durability-class append reached the ledger but its flush failed, so
    /// the caller cannot know whether the bytes will survive a crash.
    #[error("Durability indeterminate: {0}")]
    DurabilityIndeterminate(String),

    /// An identity-bearing persisted payload belongs to another ledger.
    #[error("Ledger identity mismatch: expected {expected}, got {actual}")]
    IdentityMismatch {
        /// Identity expected by the opened capability.
        expected: LedgerIdentity,
        /// Identity found in the request or durable payload.
        actual: LedgerIdentity,
    },

    /// The requested cursor predates the retained lower boundary, so replay
    /// would silently skip compacted history; callers rebuild from a genesis
    /// fact instead of replaying through the gap.
    #[error(
        "Retention gap: cursor {after_seq} predates retained history starting at {retained_from}"
    )]
    RetentionGap {
        /// Cursor the caller supplied.
        after_seq: u64,
        /// First sequence still retained by the ledger.
        retained_from: u64,
    },

    /// Sled or serialization I/O failed while reading or writing event data.
    #[error("Storage I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

impl Clone for StorageError {
    fn clone(&self) -> Self {
        match self {
            StorageError::InvalidPath(path) => StorageError::InvalidPath(path.clone()),
            StorageError::Backpressure(message) => StorageError::Backpressure(message.clone()),
            StorageError::Unavailable(message) => StorageError::Unavailable(message.clone()),
            StorageError::DurabilityIndeterminate(message) => {
                StorageError::DurabilityIndeterminate(message.clone())
            }
            StorageError::IdentityMismatch { expected, actual } => StorageError::IdentityMismatch {
                expected: *expected,
                actual: *actual,
            },
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

/// Serializable failures exposed by the identity-bearing event authority.
#[derive(Debug, Error, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventAuthorityError {
    /// The request violates a documented bound or invariant.
    #[error("Invalid request: {message}")]
    InvalidRequest {
        /// Human-readable validation detail.
        message: String,
    },

    /// A request or persisted binding names a different ledger.
    #[error("Ledger identity mismatch: expected {expected}, got {actual}")]
    IdentityMismatch {
        /// Identity owned by the authority.
        expected: LedgerIdentity,
        /// Identity supplied by the caller or persisted payload.
        actual: LedgerIdentity,
    },

    /// This process already owns a writable authority lease for the ledger.
    #[error("Duplicate writable authority binding for ledger {ledger_id}")]
    DuplicateAuthorityBinding {
        /// Identity whose lease is already active.
        ledger_id: LedgerIdentity,
    },

    /// The requested cursor predates retained history.
    #[error(
        "Retention gap in ledger {ledger_id}: cursor {after_seq} predates retained history starting at {retained_from}"
    )]
    RetentionGap {
        /// Ledger whose sequence space contains the gap.
        ledger_id: LedgerIdentity,
        /// Cursor supplied by the caller.
        after_seq: u64,
        /// First sequence still retained.
        retained_from: u64,
    },

    /// A non-blocking writer queue rejected an append.
    #[error("Backpressure: {message}")]
    Backpressure {
        /// Queue rejection detail.
        message: String,
    },

    /// A durable append was written but its flush outcome is unknown.
    #[error("Durability indeterminate: {message}")]
    DurabilityIndeterminate {
        /// Persistence failure detail.
        message: String,
    },

    /// The authority or a future remote host is unavailable.
    #[error("Event authority unavailable: {message}")]
    Unavailable {
        /// Availability failure detail.
        message: String,
    },

    /// Durable storage failed before an operation could complete.
    #[error("Event persistence failure: {message}")]
    Persistence {
        /// Persistence failure detail.
        message: String,
    },

    /// An authority invariant failed without a more precise public category.
    #[error("Event authority internal failure: {message}")]
    Internal {
        /// Internal failure detail.
        message: String,
    },

    /// The persisted ledger identity is missing bytes or is not a UUID.
    #[error("Corrupt persisted ledger identity: {message}")]
    CorruptPersistedIdentity {
        /// Corruption detail.
        message: String,
    },

    /// Durable migration evidence conflicts with the requested migration.
    #[error("Event migration conflict: {message}")]
    MigrationConflict {
        /// Migration conflict detail.
        message: String,
    },
}

impl EventAuthorityError {
    /// Constructs a typed invalid-request error.
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::InvalidRequest {
            message: message.into(),
        }
    }

    /// Adds authority identity to storage failures that carry bare sequences.
    pub(crate) fn from_storage_for_ledger(ledger_id: LedgerIdentity, error: StorageError) -> Self {
        match error {
            StorageError::RetentionGap {
                after_seq,
                retained_from,
            } => Self::RetentionGap {
                ledger_id,
                after_seq,
                retained_from,
            },
            error => error.into(),
        }
    }
}

impl From<StorageError> for EventAuthorityError {
    fn from(error: StorageError) -> Self {
        match error {
            StorageError::InvalidPath(message) => Self::InvalidRequest { message },
            StorageError::Backpressure(message) => Self::Backpressure { message },
            StorageError::Unavailable(message) => Self::Unavailable { message },
            StorageError::DurabilityIndeterminate(message) => {
                Self::DurabilityIndeterminate { message }
            }
            StorageError::IdentityMismatch { expected, actual } => {
                Self::IdentityMismatch { expected, actual }
            }
            StorageError::RetentionGap {
                after_seq,
                retained_from,
            } => Self::Internal {
                message: format!(
                    "identity-free storage retention gap at cursor {after_seq}, retained from {retained_from}"
                ),
            },
            StorageError::IoError(error) => Self::Persistence {
                message: error.to_string(),
            },
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
