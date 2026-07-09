//! Health surface computation over ledger, watermark, and registry.

use crate::error::StorageError;
use crate::events::observability::{
    surface_not_implemented, EventHealthReport, LedgerObservability,
};

pub(super) fn compute(_backing: &LedgerObservability) -> Result<EventHealthReport, StorageError> {
    Err(surface_not_implemented("health"))
}
