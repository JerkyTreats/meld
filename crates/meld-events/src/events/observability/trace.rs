//! Causal trace computation over object references, relations, and
//! stored fact provenance.

use crate::error::StorageError;
use crate::events::observability::{
    surface_not_implemented, EventTraceReport, LedgerObservability, TraceSubject,
};

pub(super) fn compute(
    _backing: &LedgerObservability,
    _subject: TraceSubject,
) -> Result<EventTraceReport, StorageError> {
    Err(surface_not_implemented("trace"))
}
