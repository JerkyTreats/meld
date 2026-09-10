use serde_json::json;

use crate::telemetry::summary::TypedSummaryEvent;

pub fn command(ok: bool, duration_ms: u128, error: Option<&str>) -> TypedSummaryEvent {
    TypedSummaryEvent::new(
        "init_summary",
        json!({ "ok": ok, "duration_ms": duration_ms, "error": error }),
    )
}
