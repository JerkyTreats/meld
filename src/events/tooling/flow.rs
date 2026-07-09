//! Flow subcommand: renders event flow over a trailing window.

use meld_events::LedgerObservability;

use crate::error::ApiError;
use crate::events::tooling::not_implemented;

pub(super) fn run(
    _port: &LedgerObservability,
    _format: &str,
    _window: usize,
) -> Result<String, ApiError> {
    Err(not_implemented("flow"))
}
