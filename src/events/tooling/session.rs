//! Session subcommand: renders one session's timeline.

use meld_events::LedgerObservability;

use crate::error::ApiError;
use crate::events::tooling::not_implemented;

pub(super) fn run(
    _port: &LedgerObservability,
    _format: &str,
    _session_id: &str,
) -> Result<String, ApiError> {
    Err(not_implemented("session"))
}
