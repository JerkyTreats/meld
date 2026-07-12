//! Session subcommand: renders one session's timeline.

use meld_events::{EventObservabilityCapability, SessionTimelineReport};

use crate::error::ApiError;
use crate::events::tooling::status::coverage_text;
use crate::events::tooling::{render, surface_authority_error};
use crate::telemetry::ProgressRuntime;

pub(super) fn run(
    observability: &EventObservabilityCapability,
    progress: &ProgressRuntime,
    format: &str,
    session_id: &str,
) -> Result<String, ApiError> {
    let report = observability
        .session(observability.ledger_identity(), session_id)
        .map_err(|err| surface_authority_error("session", err))?;
    if report.events_returned == 0 && progress.get_session(session_id)?.is_none() {
        return Err(ApiError::ConfigError(format!(
            "no session named '{session_id}' in the session store and no ledger events for it; run 'meld event tail' to see recent activity"
        )));
    }
    render(format, &report, render_text)
}

fn render_text(report: &SessionTimelineReport) -> String {
    let mut lines = vec![
        format!("ledger_id: {}", report.ledger_id),
        format!(
            "session {}: {} events, {} .. {}",
            report.session_id,
            report.events_returned,
            report.observed_started_at.as_deref().unwrap_or("-"),
            report.observed_ended_at.as_deref().unwrap_or("-"),
        ),
    ];
    lines.push(format!("coverage: {}", coverage_text(&report.coverage)));
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use meld_events::{EventAuthority, EventAuthorityOpenOptions};

    use super::*;

    fn open_runtime() -> (EventAuthority, ProgressRuntime) {
        let authority = EventAuthority::open(
            sled::Config::new().temporary(true).open().unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let session_store = crate::session::SessionStore::shared(
            sled::Config::new().temporary(true).open().unwrap(),
        )
        .unwrap();
        let sessions = Arc::new(crate::session::SessionRuntime::new(session_store));
        let progress = ProgressRuntime::from_capabilities(authority.append_capability(), sessions);
        (authority, progress)
    }

    #[test]
    fn unknown_session_id_is_an_error_not_an_empty_timeline() {
        let (authority, progress) = open_runtime();
        let observability = authority.observability_capability();
        let err = run(&observability, &progress, "text", "no-such-session").unwrap_err();
        assert!(err.to_string().contains("no session named"));
    }

    #[test]
    fn known_session_renders_its_timeline() {
        let (authority, progress) = open_runtime();
        let session_id = progress.start_command_session("scan".to_string()).unwrap();
        progress
            .finish_command_session(&session_id, true, None)
            .unwrap();
        let observability = authority.observability_capability();
        let output = run(&observability, &progress, "text", &session_id).unwrap();
        assert!(output.contains("session_started"));
        assert!(output.contains("session_ended"));
        assert_eq!(
            output.lines().find(|line| line.starts_with("coverage:")),
            Some("coverage: retained_from=1 tip=2 scanned=1..=2 truncation=none")
        );
    }
}
