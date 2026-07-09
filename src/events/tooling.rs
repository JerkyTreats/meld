//! Event ledger observability CLI adapter.
//!
//! Parses the `meld event` command family, binds the in-process observability
//! port over the ledger database, and delegates each subcommand to its unit
//! module. The adapter routes and renders only; every computation lives
//! behind [`meld_events::EventObservabilityPort`].

/// Flow subcommand rendering.
mod flow;
/// Session subcommand rendering.
mod session;
/// Status subcommand rendering.
mod status;
/// Tail subcommand rendering and follow loop.
mod tail;
/// Trace subcommand rendering and subject parsing.
mod trace;

use std::sync::Arc;

use meld_events::{EventCursorRegistry, LedgerObservability};

use crate::cli::EventCommands;
use crate::error::ApiError;
use crate::telemetry::ProgressRuntime;

/// Dispatch one `meld event` command over the CLI assembly's ledger.
pub fn handle_cli_command(
    progress: &Arc<ProgressRuntime>,
    command: &EventCommands,
) -> Result<String, ApiError> {
    let port = bind_port(progress)?;
    match command {
        EventCommands::Status { format } => {
            validate_format(format)?;
            status::run(&port, format)
        }
        EventCommands::Tail {
            format,
            after,
            limit,
            follow,
        } => {
            validate_format(format)?;
            tail::run(&port, format, *after, *limit, *follow)
        }
        EventCommands::Trace {
            format,
            object,
            stream,
            seq,
        } => {
            validate_format(format)?;
            trace::run(&port, format, object.as_deref(), stream.as_deref(), *seq)
        }
        EventCommands::Session { format, session_id } => {
            validate_format(format)?;
            session::run(&port, format, session_id)
        }
        EventCommands::Flow { format, window } => {
            validate_format(format)?;
            flow::run(&port, format, *window)
        }
    }
}

fn bind_port(progress: &Arc<ProgressRuntime>) -> Result<LedgerObservability, ApiError> {
    let store = progress.store();
    let registry =
        EventCursorRegistry::open(store.db()).map_err(crate::error::StorageError::from)?;
    Ok(LedgerObservability::new(
        Arc::new(store.clone()),
        progress.watermark(),
        registry,
        progress.dropped_handle(),
    ))
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

pub(crate) fn surface_error(_surface: &str, err: meld_events::error::StorageError) -> ApiError {
    ApiError::StorageError(crate::error::StorageError::from(err))
}
