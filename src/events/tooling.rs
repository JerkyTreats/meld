//! Event ledger command adapter.
//!
//! Parses the `meld event` command family and delegates each subcommand to
//! its unit module. The adapter routes and renders only; every ledger read is
//! supplied by the product-resolved event authority.

/// Flow subcommand rendering.
mod flow;
/// Live transport delegates to the same native event contracts.
mod live;
pub use live::try_live;
/// Session subcommand rendering.
mod session;
/// Status subcommand rendering.
mod status;
/// Tail subcommand rendering and follow loop.
mod tail;
/// Trace subcommand rendering and subject parsing.
mod trace;

use std::sync::Arc;

use meld_events::error::EventAuthorityError;
use meld_events::{
    EventObservabilityCapability, EventReplayCapability, EventSubscriptionCapability,
};

use crate::cli::EventCommands;
use crate::error::ApiError;
use crate::telemetry::ProgressRuntime;

/// Dispatch one `meld event` command over the CLI assembly's ledger.
pub fn handle_cli_command(
    observability: &EventObservabilityCapability,
    replay: &EventReplayCapability,
    subscription: &EventSubscriptionCapability,
    progress: &Arc<ProgressRuntime>,
    command: &EventCommands,
) -> Result<String, ApiError> {
    let ledger_id = observability.ledger_identity();
    for actual in [replay.ledger_identity(), subscription.ledger_identity()] {
        if actual != ledger_id {
            return Err(surface_authority_error(
                "event",
                EventAuthorityError::IdentityMismatch {
                    expected: ledger_id,
                    actual,
                },
            ));
        }
    }
    match command {
        EventCommands::Append { .. } => Err(ApiError::ConfigError(
            "event append requires a live runtime; use runtime start".into(),
        )),
        EventCommands::Status { format } => {
            validate_format(format)?;
            status::run(observability, format)
        }
        EventCommands::Tail {
            format,
            after,
            limit,
            follow,
        } => {
            validate_format(format)?;
            tail::run(
                observability,
                replay,
                subscription,
                format,
                *after,
                *limit,
                *follow,
            )
        }
        EventCommands::Trace {
            format,
            object,
            stream,
            seq,
        } => {
            validate_format(format)?;
            trace::run(
                observability,
                format,
                object.as_deref(),
                stream.as_deref(),
                *seq,
            )
        }
        EventCommands::Session { format, session_id } => {
            validate_format(format)?;
            session::run(observability, progress, format, session_id)
        }
        EventCommands::Flow { format, window } => {
            validate_format(format)?;
            flow::run(observability, format, *window)
        }
    }
}

pub(crate) fn render<T: serde::Serialize>(
    format: &str,
    report: &T,
    text: impl Fn(&T) -> String,
) -> Result<String, ApiError> {
    match format {
        "json" => serde_json::to_string_pretty(report)
            .map_err(|err| ApiError::ConfigError(format!("failed to render report: {err}"))),
        _ => Ok(text(report)),
    }
}

fn validate_format(format: &str) -> Result<(), ApiError> {
    match format {
        "text" | "json" => Ok(()),
        other => Err(ApiError::ConfigError(format!(
            "invalid format '{other}', expected 'text' or 'json'"
        ))),
    }
}

pub(crate) fn surface_authority_error(_surface: &str, error: EventAuthorityError) -> ApiError {
    use crate::error::StorageError;

    let error = match error {
        EventAuthorityError::InvalidRequest { message } => StorageError::InvalidPath(message),
        EventAuthorityError::IdentityMismatch { expected, actual } => {
            StorageError::LedgerIdentityMismatch {
                expected: expected.to_string(),
                actual: actual.to_string(),
            }
        }
        EventAuthorityError::DuplicateAuthorityBinding { ledger_id } => {
            StorageError::EventAuthorityUnavailable(format!(
                "duplicate writable binding for ledger {ledger_id}"
            ))
        }
        EventAuthorityError::RetentionGap {
            after_seq,
            retained_from,
            ..
        } => StorageError::RetentionGap {
            after_seq,
            retained_from,
        },
        EventAuthorityError::Backpressure { message } => StorageError::Backpressure(message),
        EventAuthorityError::DurabilityIndeterminate { message } => {
            StorageError::DurabilityIndeterminate(message)
        }
        EventAuthorityError::Unavailable { message } => {
            StorageError::EventAuthorityUnavailable(message)
        }
        EventAuthorityError::MigrationConflict { message } => {
            StorageError::MigrationConflict(message)
        }
        EventAuthorityError::Persistence { message }
        | EventAuthorityError::Internal { message }
        | EventAuthorityError::CorruptPersistedIdentity { message } => {
            StorageError::IoError(std::io::Error::other(message))
        }
    };
    ApiError::StorageError(error)
}
