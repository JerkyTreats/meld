use serde_json::json;

use crate::telemetry::summary::TypedSummaryEvent;

pub fn command(
    force: bool,
    list_only: bool,
    ok: bool,
    duration_ms: u128,
    error: Option<&str>,
) -> TypedSummaryEvent {
    TypedSummaryEvent::new(
        "init_summary",
        json!({
            "force": force,
            "list_only": list_only,
            "ok": ok,
            "duration_ms": duration_ms,
            "error": error,
        }),
    )
}
