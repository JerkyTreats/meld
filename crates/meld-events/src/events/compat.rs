use crate::events::{EventEnvelope, EventRecord};

/// Compatibility alias for callers that still publish progress envelopes.
pub type ProgressEnvelope = EventEnvelope;
/// Compatibility alias for callers that still read progress events.
pub type ProgressEvent = EventRecord;
