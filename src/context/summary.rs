use serde_json::json;

use crate::telemetry::summary::TypedSummaryEvent;

#[allow(clippy::too_many_arguments)]
pub fn generation(
    action: &str,
    target_path: bool,
    target_node: bool,
    recursive: bool,
    force: bool,
    ok: bool,
    duration_ms: u128,
    error: Option<&str>,
) -> TypedSummaryEvent {
    TypedSummaryEvent::new(
        "context_generation_summary",
        json!({
            "scope": "context",
            "action": action,
            "target": summary_target(target_path, target_node),
            "recursive": recursive,
            "force": force,
            "ok": ok,
            "duration_ms": duration_ms,
            "error": error,
        }),
    )
}

fn summary_target(target_path: bool, target_node: bool) -> &'static str {
    if target_path {
        "path"
    } else if target_node {
        "node"
    } else {
        "unknown"
    }
}
