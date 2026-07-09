//! Session subcommand: renders one session's timeline.

use meld_events::events::observability::{EventObservabilityPort, SessionTimelineReport};
use meld_events::LedgerObservability;

use crate::error::ApiError;
use crate::events::tooling::{render, surface_error};

pub(super) fn run(
    port: &LedgerObservability,
    format: &str,
    session_id: &str,
) -> Result<String, ApiError> {
    let report = port
        .session(session_id)
        .map_err(|err| surface_error("session", err))?;
    render(format, &report, render_text)
}

fn render_text(report: &SessionTimelineReport) -> String {
    let mut lines = vec![format!(
        "session {}: {} events, {} .. {}",
        report.session_id,
        report.total_events,
        report.started_at.as_deref().unwrap_or("-"),
        report.ended_at.as_deref().unwrap_or("-"),
    )];
    for step in &report.steps {
        let gap = step
            .gap_ms
            .map(|gap_ms| format!("  (+{gap_ms}ms)"))
            .unwrap_or_default();
        lines.push(format!(
            "{}  {}  {}  {}{gap}",
            step.seq, step.recorded_at, step.domain_id, step.event_type
        ));
    }
    lines.join("\n")
}
