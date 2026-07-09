//! Status subcommand: renders the ledger health report.

use meld_events::events::observability::EventObservabilityPort;
use meld_events::LedgerObservability;

use crate::error::ApiError;
use crate::events::tooling::{render, surface_error};

pub(super) fn run(port: &LedgerObservability, format: &str) -> Result<String, ApiError> {
    let report = port.health().map_err(|err| surface_error("status", err))?;
    render(format, &report, |report| format!("{report:#?}"))
}
