use serde_json::json;

use crate::telemetry::summary::TypedSummaryEvent;

pub fn command(
    action: &str,
    ok: bool,
    duration_ms: u128,
    error: Option<&str>,
) -> TypedSummaryEvent {
    TypedSummaryEvent::new(
        "workflow_summary",
        json!({
            "scope": "workflow",
            "action": action,
            "ok": ok,
            "duration_ms": duration_ms,
            "error": error,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::command;
    use serde_json::json;

    #[test]
    fn workflow_command_summary_preserves_event_contract() {
        let event = command("list", true, 42, None);

        assert_eq!(event.event_type, "workflow_summary");
        assert_eq!(
            event.data,
            json!({
                "scope": "workflow",
                "action": "list",
                "ok": true,
                "duration_ms": 42,
                "error": null,
            })
        );
    }
}
