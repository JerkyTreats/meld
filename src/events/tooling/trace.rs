//! Trace subcommand: renders causal chains and parses subject selectors.

use meld_events::LedgerObservability;

use crate::error::ApiError;
use crate::events::tooling::not_implemented;

pub(super) fn run(
    _port: &LedgerObservability,
    _format: &str,
    _object: Option<&str>,
    _stream: Option<&str>,
    _seq: Option<u64>,
) -> Result<String, ApiError> {
    Err(not_implemented("trace"))
}
