//! Event ledger observability CLI adapter.
//!
//! Parses the `meld event` command family, binds the in-process observability
//! port over the ledger database, and formats reports as text or JSON. The
//! adapter routes and renders only; every computation lives behind
//! [`meld_events::EventObservabilityPort`].

use std::sync::Arc;

use meld_events::events::observability::EventObservabilityPort;
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
            let report = port.health().map_err(|err| surface_error("status", err))?;
            render(format, &report, render_status_text)
        }
        EventCommands::Tail { format, .. } => {
            validate_format(format)?;
            Err(not_implemented("tail"))
        }
        EventCommands::Trace { format, .. } => {
            validate_format(format)?;
            Err(not_implemented("trace"))
        }
        EventCommands::Session { format, .. } => {
            validate_format(format)?;
            Err(not_implemented("session"))
        }
        EventCommands::Flow { format, .. } => {
            validate_format(format)?;
            Err(not_implemented("flow"))
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
    ))
}

fn render<T: serde::Serialize>(
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

fn render_status_text(report: &meld_events::EventHealthReport) -> String {
    // Placeholder rendering; the health unit owns the real formatter.
    format!("{report:#?}")
}

fn validate_format(format: &str) -> Result<(), ApiError> {
    match format {
        "text" | "json" => Ok(()),
        other => Err(ApiError::ConfigError(format!(
            "invalid format '{other}', expected 'text' or 'json'"
        ))),
    }
}

fn not_implemented(surface: &str) -> ApiError {
    ApiError::ConfigError(format!(
        "meld event {surface} is not implemented yet; it lands with the observability surface units"
    ))
}

// Unsupported surfaces read as friendly stubs, not disk failures; the
// mapping retires with the last stub.
fn surface_error(surface: &str, err: meld_events::error::StorageError) -> ApiError {
    if let meld_events::error::StorageError::IoError(io) = &err {
        if io.kind() == std::io::ErrorKind::Unsupported {
            return not_implemented(surface);
        }
    }
    ApiError::StorageError(crate::error::StorageError::from(err))
}
