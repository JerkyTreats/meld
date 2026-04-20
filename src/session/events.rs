use serde_json::json;

use crate::events::EventEnvelope;

pub fn session_started_envelope(session_id: &str, command: &str) -> EventEnvelope {
    EventEnvelope::with_now(
        session_id.to_string(),
        "session_started",
        json!({ "command": command }),
    )
}

pub fn session_ended_envelope(
    session_id: &str,
    status: &str,
    error: Option<String>,
) -> EventEnvelope {
    EventEnvelope::with_now(
        session_id.to_string(),
        "session_ended",
        json!({ "status": status, "error": error }),
    )
}
