//! Tail subcommand: follows ledger records as they commit.

use meld_events::LedgerObservability;

use crate::error::ApiError;
use crate::events::tooling::not_implemented;

pub(super) fn run(
    _port: &LedgerObservability,
    _format: &str,
    _after: Option<u64>,
    _limit: usize,
    _follow: bool,
) -> Result<String, ApiError> {
    Err(not_implemented("tail"))
}
