use serde_json::json;

use crate::telemetry::summary::TypedSummaryEvent;

pub fn command(
    action: &str,
    mutation: bool,
    ok: bool,
    duration_ms: u128,
    error: Option<&str>,
) -> TypedSummaryEvent {
    TypedSummaryEvent::new(
        "config_mutation_summary",
        json!({
            "scope": "agent",
            "action": action,
            "mutation": mutation,
            "ok": ok,
            "duration_ms": duration_ms,
            "error": error,
        }),
    )
}
