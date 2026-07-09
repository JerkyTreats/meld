//! Session timeline and flow window computation.

use crate::error::StorageError;
use crate::events::observability::{
    surface_not_implemented, EventFlowReport, FlowWindow, LedgerObservability,
    SessionTimelineReport,
};

pub(super) fn compute_flow(
    _backing: &LedgerObservability,
    _window: FlowWindow,
) -> Result<EventFlowReport, StorageError> {
    Err(surface_not_implemented("flow"))
}

pub(super) fn compute_timeline(
    _backing: &LedgerObservability,
    _session_id: &str,
) -> Result<SessionTimelineReport, StorageError> {
    Err(surface_not_implemented("session"))
}
