//! Summary contracts and `command_summary` payload shape.

use serde_json::Value;

pub use crate::telemetry::events::SummaryEventData;

#[derive(Debug, Clone, PartialEq)]
pub struct TypedSummaryEvent {
    pub event_type: &'static str,
    pub data: Value,
}

impl TypedSummaryEvent {
    pub fn new(event_type: &'static str, data: Value) -> Self {
        Self { event_type, data }
    }
}
