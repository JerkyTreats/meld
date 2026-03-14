//! Generic typed summary and `command_summary` payload helpers.

use serde_json::Value;

use crate::telemetry::events::SummaryEventData;
use crate::telemetry::summary::TypedSummaryEvent;

pub const COMMAND_SUMMARY_MESSAGE_MAX_CHARS: usize = 256;

pub fn typed_summary_event(
    typed_summary: Option<TypedSummaryEvent>,
) -> Option<(&'static str, Value)> {
    typed_summary.map(|typed_summary| (typed_summary.event_type, typed_summary.data))
}

pub fn truncate_summary_message(value: &str, max_chars: usize) -> (String, bool) {
    if value.chars().count() <= max_chars {
        return (value.to_string(), false);
    }
    (value.chars().take(max_chars).collect(), true)
}

/// Build `command_summary` payload. Caller provides command name and result-derived fields.
pub fn command_summary_data(
    command_name: &str,
    ok: bool,
    duration_ms: u128,
    message: Option<String>,
    output_chars: Option<usize>,
    error_chars: Option<usize>,
    truncated: Option<bool>,
) -> SummaryEventData {
    SummaryEventData {
        command: command_name.to_string(),
        ok,
        duration_ms,
        message,
        output_chars,
        error_chars,
        truncated,
    }
}
