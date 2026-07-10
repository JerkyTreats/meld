//! Session subcommand: renders one session's timeline.

use meld_events::events::observability::{
    CoverageTruncation, EventObservabilityPort, SessionTimelineReport,
};
use meld_events::LedgerObservability;

use crate::error::ApiError;
use crate::events::tooling::{render, surface_error};
use crate::telemetry::ProgressRuntime;

pub(super) fn run(
    port: &LedgerObservability,
    progress: &ProgressRuntime,
    format: &str,
    session_id: &str,
) -> Result<String, ApiError> {
    let report = port
        .session(session_id)
        .map_err(|err| surface_error("session", err))?;
    if report.events_returned == 0 && progress.get_session(session_id)?.is_none() {
        return Err(ApiError::ConfigError(format!(
            "no session named '{session_id}' in the session store and no ledger events for it; run 'meld event tail' to see recent activity"
        )));
    }
    render(format, &report, render_text)
}

fn render_text(report: &SessionTimelineReport) -> String {
    let mut lines = vec![format!(
        "session {}: {} events, {} .. {}",
        report.session_id,
        report.events_returned,
        report.observed_started_at.as_deref().unwrap_or("-"),
        report.observed_ended_at.as_deref().unwrap_or("-"),
    )];
    let scanned = match (
        report.coverage.scanned_from_seq,
        report.coverage.scanned_through_seq,
    ) {
        (Some(from), Some(through)) => format!("{from}..={through}"),
        _ => "empty".to_string(),
    };
    lines.push(format!(
        "coverage retained_from={} tip={} scanned={} truncation={}",
        report.coverage.retained_from,
        report.coverage.tip_seq,
        scanned,
        truncation_label(report.coverage.truncation)
    ));
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

fn truncation_label(truncation: CoverageTruncation) -> &'static str {
    match truncation {
        CoverageTruncation::None => "none",
        CoverageTruncation::Before => "before",
        CoverageTruncation::After => "after",
        CoverageTruncation::Both => "both",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use meld_events::EventCursorRegistry;

    use super::*;

    fn bind(progress: &ProgressRuntime) -> LedgerObservability {
        let store = progress.store();
        let registry = EventCursorRegistry::open(store.db()).unwrap();
        LedgerObservability::new(
            Arc::new(store.clone()),
            progress.watermark(),
            registry,
            progress.dropped_handle(),
        )
    }

    fn open_runtime() -> ProgressRuntime {
        let db = sled::Config::new().temporary(true).open().unwrap();
        ProgressRuntime::new(db).unwrap()
    }

    #[test]
    fn unknown_session_id_is_an_error_not_an_empty_timeline() {
        let progress = open_runtime();
        let port = bind(&progress);
        let err = run(&port, &progress, "text", "no-such-session").unwrap_err();
        assert!(err.to_string().contains("no session named"));
    }

    #[test]
    fn known_session_renders_its_timeline() {
        let progress = open_runtime();
        let session_id = progress.start_command_session("scan".to_string()).unwrap();
        progress
            .finish_command_session(&session_id, true, None)
            .unwrap();
        let port = bind(&progress);
        let output = run(&port, &progress, "text", &session_id).unwrap();
        assert!(output.contains("session_started"));
        assert!(output.contains("session_ended"));
    }
}
