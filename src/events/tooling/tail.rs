//! Tail subcommand: pages ledger records after a cursor and follows new
//! commits.
//!
//! One-shot mode returns a single rendered page. Follow mode prints pages as
//! they commit and runs until the process is interrupted; per the PLAN it is
//! the one intentionally long-running command in the `meld event` family.

use std::io::Write;

use meld_events::{EventObservabilityPort, EventPageRequest, EventRecord, LegacyEventPage};

use crate::error::ApiError;
use crate::events::tooling::{render, surface_error};

/// Per-page wait in follow mode: short enough that interruption feels
/// immediate, long enough to avoid busy polling on a quiet ledger.
const FOLLOW_PAGE_TIMEOUT_MS: u64 = 500;

pub(super) fn run(
    port: &impl EventObservabilityPort,
    format: &str,
    after: Option<u64>,
    limit: usize,
    follow: bool,
) -> Result<String, ApiError> {
    if follow {
        follow_forever(port, format, after, limit)
    } else {
        // Tail semantics: without a cursor, show the most recent records.
        let cursor = match after {
            Some(seq) => seq,
            None => default_cursor(port, limit as u64)?,
        };
        let page = next_page(port, cursor, limit, 0)?;
        render(format, &page, render_page_text)
    }
}

/// Prints committed pages to stdout as they arrive until interrupted or the
/// output pipe closes; a closed pipe ends the command cleanly so the session
/// lifecycle still completes.
fn follow_forever(
    port: &impl EventObservabilityPort,
    format: &str,
    after: Option<u64>,
    limit: usize,
) -> Result<String, ApiError> {
    // Without an explicit cursor, follow means "new events only", starting
    // at the tip. An explicit --after replays from that cursor instead and
    // may honestly trip a retention gap below the boundary.
    let mut cursor = match after {
        Some(seq) => seq,
        None => {
            port.health()
                .map_err(|err| surface_error("tail", err))?
                .tip_seq
        }
    };
    // The follower holds the single-process database lock, so concurrent
    // meld commands cannot produce events while it watches; cross-process
    // live following arrives with the daemon edge. Said out loud so a
    // silent follow is not mistaken for a dead ledger.
    eprintln!(
        "watching the ledger from this process; other meld commands cannot \
         write while tail runs"
    );
    let stdout = std::io::stdout();
    loop {
        let page = next_page(port, cursor, limit, FOLLOW_PAGE_TIMEOUT_MS)?;
        if !page.records.is_empty() {
            let mut out = stdout.lock();
            let rendered = render_follow_page(format, &page)?;
            if writeln!(out, "{rendered}").is_err() || out.flush().is_err() {
                // A closed pipe is the reader hanging up, not a failure.
                return Ok(String::new());
            }
        }
        cursor = advance_cursor(cursor, &page);
    }
}

/// Default one-shot cursor: the last `limit` records, never below the first
/// readable cursor so the default can never trip a retention gap; explicit
/// cursors keep their honest gap semantics.
fn default_cursor(port: &impl EventObservabilityPort, limit: u64) -> Result<u64, ApiError> {
    let health = port.health().map_err(|err| surface_error("tail", err))?;
    Ok(health
        .tip_seq
        .saturating_sub(limit)
        .max(health.retained_from.saturating_sub(1)))
}

fn next_page(
    port: &impl EventObservabilityPort,
    after_seq: u64,
    limit: usize,
    timeout_ms: u64,
) -> Result<LegacyEventPage, ApiError> {
    port.next_page(EventPageRequest {
        after_seq,
        limit,
        timeout_ms,
    })
    .map_err(|err| surface_error("tail", err))
}

/// Next cursor after a page: the page's cursor when it carried records, the
/// current cursor unchanged when it was empty (a timeout, not a gap).
fn advance_cursor(cursor: u64, page: &LegacyEventPage) -> u64 {
    if page.records.is_empty() {
        cursor
    } else {
        page.next_after_seq
    }
}

/// Renders one follow-mode page: text lines, or the whole page as a single
/// JSON object line so `--format json --follow` streams one object per page.
fn render_follow_page(format: &str, page: &LegacyEventPage) -> Result<String, ApiError> {
    match format {
        "json" => serde_json::to_string(page)
            .map_err(|err| ApiError::ConfigError(format!("failed to render event page: {err}"))),
        _ => Ok(render_page_text(page)),
    }
}

fn render_page_text(page: &LegacyEventPage) -> String {
    page.records
        .iter()
        .map(render_record_line)
        .collect::<Vec<_>>()
        .join("\n")
}

/// One text line per record; the stream is shown only when it differs from
/// the session, since legacy telemetry streams equal their session.
fn render_record_line(record: &EventRecord) -> String {
    let mut line = format!(
        "{}  {}  {}  {}  session={}",
        record.seq, record.recorded_at, record.domain_id, record.event_type, record.session
    );
    if record.stream_id != record.session {
        line.push_str(&format!(" stream={}", record.stream_id));
    }
    line
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use meld_events::error::StorageError;
    use meld_events::{
        EventEnvelope, EventFlowReport, EventHealthReport, EventTraceReport, FlowWindow,
        SessionTimelineReport, TraceSubject,
    };
    use serde_json::json;

    use super::*;

    fn record(seq: u64, session: &str, stream: &str) -> EventRecord {
        let envelope = EventEnvelope::new_domain(
            "2026-07-08T12:00:00Z".to_string(),
            session,
            "execution",
            stream,
            "execution.task.progress",
            None,
            json!({ "seq": seq }),
        );
        EventRecord::from_envelope(envelope, seq)
    }

    fn page(records: Vec<EventRecord>, next_after_seq: u64) -> LegacyEventPage {
        LegacyEventPage {
            records,
            next_after_seq,
        }
    }

    /// Port fake serving scripted pages while recording every request, so
    /// cursor and timeout discipline are observable without a live ledger.
    struct ScriptedPort {
        pages: RefCell<VecDeque<LegacyEventPage>>,
        requests: RefCell<Vec<EventPageRequest>>,
        tip_seq: u64,
        retained_from: u64,
    }

    impl ScriptedPort {
        fn new(pages: Vec<LegacyEventPage>) -> Self {
            Self::with_health(pages, 0, 1)
        }

        fn with_health(pages: Vec<LegacyEventPage>, tip_seq: u64, retained_from: u64) -> Self {
            Self {
                pages: RefCell::new(pages.into()),
                requests: RefCell::new(Vec::new()),
                tip_seq,
                retained_from,
            }
        }
    }

    impl EventObservabilityPort for ScriptedPort {
        fn health(&self) -> Result<EventHealthReport, StorageError> {
            Ok(EventHealthReport {
                tip_seq: self.tip_seq,
                committed_watermark: self.tip_seq,
                retained_from: self.retained_from,
                dropped_events: 0,
                consumers: Vec::new(),
                append_rates: Vec::new(),
            })
        }

        fn flow(&self, _window: FlowWindow) -> Result<EventFlowReport, StorageError> {
            unimplemented!("tail tests exercise next_page only")
        }

        fn trace(&self, _subject: TraceSubject) -> Result<EventTraceReport, StorageError> {
            unimplemented!("tail tests exercise next_page only")
        }

        fn session(&self, _session_id: &str) -> Result<SessionTimelineReport, StorageError> {
            unimplemented!("tail tests exercise next_page only")
        }

        fn next_page(&self, request: EventPageRequest) -> Result<LegacyEventPage, StorageError> {
            self.requests.borrow_mut().push(request);
            Ok(self
                .pages
                .borrow_mut()
                .pop_front()
                .expect("test script ran out of pages"))
        }
    }

    #[test]
    fn record_line_omits_stream_matching_session() {
        assert_eq!(
            render_record_line(&record(7, "session-a", "session-a")),
            "7  2026-07-08T12:00:00Z  execution  execution.task.progress  session=session-a"
        );
    }

    #[test]
    fn record_line_shows_stream_differing_from_session() {
        assert_eq!(
            render_record_line(&record(7, "session-a", "workflow-b")),
            "7  2026-07-08T12:00:00Z  execution  execution.task.progress  \
             session=session-a stream=workflow-b"
        );
    }

    #[test]
    fn page_text_renders_one_line_per_record() {
        let text = render_page_text(&page(
            vec![record(1, "s", "s"), record(2, "s", "workflow-b")],
            2,
        ));
        assert_eq!(text.lines().count(), 2);
        assert!(text.lines().next().unwrap().starts_with("1  "));
        assert!(text.lines().nth(1).unwrap().ends_with("stream=workflow-b"));
    }

    #[test]
    fn empty_page_text_is_empty() {
        assert_eq!(render_page_text(&page(Vec::new(), 4)), "");
    }

    #[test]
    fn follow_page_json_is_one_object_line() {
        let rendered = render_follow_page("json", &page(vec![record(3, "s", "s")], 3)).unwrap();
        assert!(!rendered.contains('\n'));
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(value["next_after_seq"], 3);
        assert_eq!(value["records"][0]["seq"], 3);
    }

    #[test]
    fn follow_page_text_matches_page_text() {
        let page = page(vec![record(3, "s", "s")], 3);
        assert_eq!(
            render_follow_page("text", &page).unwrap(),
            render_page_text(&page)
        );
    }

    #[test]
    fn cursor_advances_to_page_cursor_when_records_arrive() {
        assert_eq!(advance_cursor(3, &page(vec![record(9, "s", "s")], 9)), 9);
    }

    #[test]
    fn cursor_holds_on_empty_page() {
        assert_eq!(advance_cursor(3, &page(Vec::new(), 3)), 3);
    }

    #[test]
    fn one_shot_defaults_to_the_most_recent_records() {
        let port = ScriptedPort::with_health(vec![page(vec![record(90, "s", "s")], 90)], 100, 1);
        let output = run(&port, "text", None, 16, false).unwrap();
        assert!(output.starts_with("90  "));
        let requests = port.requests.borrow();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].after_seq, 84);
        assert_eq!(requests[0].limit, 16);
        assert_eq!(requests[0].timeout_ms, 0);
    }

    #[test]
    fn one_shot_default_cursor_never_falls_below_the_retention_boundary() {
        let port = ScriptedPort::with_health(vec![page(Vec::new(), 0)], 100, 95);
        run(&port, "text", None, 16, false).unwrap();
        let requests = port.requests.borrow();
        assert_eq!(requests[0].after_seq, 94);
    }

    #[test]
    fn one_shot_json_is_pretty_page() {
        let port = ScriptedPort::new(vec![page(vec![record(1, "s", "s")], 1)]);
        let output = run(&port, "json", Some(0), 16, false).unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["next_after_seq"], 1);
        // Pretty rendering distinguishes the one-shot page from the
        // follow-mode single-line object.
        assert!(output.contains('\n'));
    }

    #[test]
    fn default_cursor_is_tip_minus_limit_clamped_to_retention() {
        let plenty = ScriptedPort::with_health(Vec::new(), 100, 1);
        assert_eq!(default_cursor(&plenty, 16).unwrap(), 84);
        let short = ScriptedPort::with_health(Vec::new(), 10, 1);
        assert_eq!(default_cursor(&short, 16).unwrap(), 0);
        let compacted = ScriptedPort::with_health(Vec::new(), 100, 95);
        assert_eq!(default_cursor(&compacted, 16).unwrap(), 94);
    }
}
