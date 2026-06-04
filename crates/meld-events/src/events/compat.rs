//! Compatibility aliases for legacy progress event callers.
//!
//! Owner: event compatibility.
//! Inputs: callers still named around progress envelopes and progress events.
//! Outputs: aliases to the canonical event envelope and record contracts.
//! Does not own: this module does not preserve a separate progress event
//! schema.

use crate::events::{EventEnvelope, EventRecord};

/// Compatibility alias for callers that still publish progress envelopes.
pub type ProgressEnvelope = EventEnvelope;
/// Compatibility alias for callers that still read progress events.
pub type ProgressEvent = EventRecord;
