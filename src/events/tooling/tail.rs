//! Tail subcommand: pages ledger records after a cursor and follows new
//! commits.
//!
//! One-shot mode returns a single rendered page. Follow mode prints pages as
//! they commit and runs until the process is interrupted; per the PLAN it is
//! the one intentionally long-running command in the `meld event` family.

use std::io::Write;

use meld_events::{EventObservabilityPort, EventPage, EventPageRequest, EventRecord};

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
        // Tail semantics: without a cursor, select the most recent records
        // by count. Sequence subtraction is incorrect for sparse migrated
        // ledgers whose adjacent records need not have adjacent numbers.
        let page = match after {
            Some(seq) => next_page(port, seq, limit, 0)?,
            None => port
                .newest_page(limit)
                .map_err(|err| surface_error("tail", err))?,
        };
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

fn next_page(
    port: &impl EventObservabilityPort,
    after_seq: u64,
    limit: usize,
    timeout_ms: u64,
) -> Result<EventPage, ApiError> {
    port.next_page(EventPageRequest {
        after_seq,
        limit,
        timeout_ms,
    })
    .map_err(|err| surface_error("tail", err))
}

/// Next cursor after a page: the page's cursor when it carried records, the
/// current cursor unchanged when it was empty (a timeout, not a gap).
fn advance_cursor(cursor: u64, page: &EventPage) -> u64 {
    if page.records.is_empty() {
        cursor
    } else {
        page.next_cursor.after_seq
    }
}

/// Renders one follow-mode page: text lines, or the whole page as a single
/// JSON object line so `--format json --follow` streams one object per page.
fn render_follow_page(format: &str, page: &EventPage) -> Result<String, ApiError> {
    match format {
        "json" => serde_json::to_string(page)
            .map_err(|err| ApiError::ConfigError(format!("failed to render event page: {err}"))),
        _ => Ok(render_page_text(page)),
    }
}

fn render_page_text(page: &EventPage) -> String {
    let mut lines = vec![
        format!("ledger_id: {}", page.ledger_id),
        format!(
            "coverage: {}",
            crate::events::tooling::status::coverage_text(&page.coverage)
        ),
    ];
    lines.extend(
        page.records
            .iter()
            .map(render_record_line)
            .collect::<Vec<_>>(),
    );
    lines.join("\n")
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
        CoverageTruncation, EventEnvelope, EventFlowReport, EventHealthReport, EventReadCoverage,
        EventTraceReport, FlowWindow, LedgerCursor, LedgerIdentity, SessionTimelineReport,
        TraceSubject,
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

    fn ledger_id() -> LedgerIdentity {
        "00000000-0000-0000-0000-000000000001".parse().unwrap()
    }

    fn coverage(records: &[EventRecord]) -> EventReadCoverage {
        EventReadCoverage {
            retained_from: 1,
            tip_seq: records.last().map(|record| record.seq).unwrap_or(0),
            scanned_from_seq: records.first().map(|record| record.seq),
            scanned_through_seq: records.last().map(|record| record.seq),
            truncation: CoverageTruncation::None,
        }
    }

    fn page(records: Vec<EventRecord>, next_after_seq: u64) -> EventPage {
        let coverage = coverage(&records);
        EventPage {
            ledger_id: ledger_id(),
            records,
            next_cursor: LedgerCursor {
                ledger_id: ledger_id(),
                after_seq: next_after_seq,
            },
            coverage,
        }
    }

    /// Port fake serving scripted pages while recording every request, so
    /// cursor and timeout discipline are observable without a live ledger.
    struct ScriptedPort {
        pages: RefCell<VecDeque<EventPage>>,
        requests: RefCell<Vec<EventPageRequest>>,
        newest_limits: RefCell<Vec<usize>>,
        tip_seq: u64,
        retained_from: u64,
    }

    impl ScriptedPort {
        fn new(pages: Vec<EventPage>) -> Self {
            Self::with_health(pages, 0, 1)
        }

        fn with_health(pages: Vec<EventPage>, tip_seq: u64, retained_from: u64) -> Self {
            Self {
                pages: RefCell::new(pages.into()),
                requests: RefCell::new(Vec::new()),
                newest_limits: RefCell::new(Vec::new()),
                tip_seq,
                retained_from,
            }
        }
    }

    impl EventObservabilityPort for ScriptedPort {
        fn health(&self) -> Result<EventHealthReport, StorageError> {
            Ok(EventHealthReport {
                ledger_id: ledger_id(),
                tip_seq: self.tip_seq,
                committed_watermark: self.tip_seq,
                retained_from: self.retained_from,
                dropped_events: 0,
                consumers: Vec::new(),
                append_rates: Vec::new(),
                append_rate_coverage: EventReadCoverage {
                    retained_from: self.retained_from,
                    tip_seq: self.tip_seq,
                    scanned_from_seq: None,
                    scanned_through_seq: None,
                    truncation: CoverageTruncation::None,
                },
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

        fn next_page(&self, request: EventPageRequest) -> Result<EventPage, StorageError> {
            self.requests.borrow_mut().push(request);
            Ok(self
                .pages
                .borrow_mut()
                .pop_front()
                .expect("test script ran out of pages"))
        }

        fn newest_page(&self, limit: usize) -> Result<EventPage, StorageError> {
            self.newest_limits.borrow_mut().push(limit);
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
        assert_eq!(text.lines().count(), 4);
        assert!(text.lines().next().unwrap().starts_with("ledger_id: "));
        assert!(text.lines().nth(2).unwrap().starts_with("1  "));
        assert!(text.lines().nth(3).unwrap().ends_with("stream=workflow-b"));
    }

    #[test]
    fn empty_page_text_carries_identity_and_coverage() {
        let rendered = render_page_text(&page(Vec::new(), 4));
        let lines: Vec<_> = rendered.lines().collect();
        assert_eq!(lines[0], "ledger_id: 00000000-0000-0000-0000-000000000001");
        assert_eq!(
            lines[1],
            "coverage: retained_from=1 tip=0 scanned=empty truncation=none"
        );
    }

    #[test]
    fn follow_page_json_is_one_object_line() {
        let rendered = render_follow_page("json", &page(vec![record(3, "s", "s")], 3)).unwrap();
        assert!(!rendered.contains('\n'));
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(value["next_cursor"]["after_seq"], 3);
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
        assert!(output.contains("90  "));
        assert!(port.requests.borrow().is_empty());
        assert_eq!(&*port.newest_limits.borrow(), &[16]);
    }

    #[test]
    fn one_shot_json_is_pretty_page() {
        let port = ScriptedPort::new(vec![page(vec![record(1, "s", "s")], 1)]);
        let output = run(&port, "json", Some(0), 16, false).unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["next_cursor"]["after_seq"], 1);
        // Pretty rendering distinguishes the one-shot page from the
        // follow-mode single-line object.
        assert!(output.contains('\n'));
    }

    #[test]
    fn one_shot_default_uses_record_count_on_a_real_sparse_ledger() {
        use std::sync::Arc;

        use meld_events::{EventCursorRegistry, EventWriter, LedgerObservability};

        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = meld_events::events::store::EventStore::shared(db.clone()).unwrap(); // boundary-allow: event-test
        store.append_event(&record(10, "s", "s")).unwrap();
        store.append_event(&record(90, "s", "s")).unwrap();
        let registry = EventCursorRegistry::open(&db).unwrap();
        let writer = EventWriter::spawn(Arc::clone(&store)); // boundary-allow: event-test
        let port = LedgerObservability::new(
            Arc::clone(&store),
            writer.watermark(),
            registry,
            writer.dropped_handle(),
        );

        let output = run(&port, "text", None, 1, false).unwrap();
        assert!(output.lines().any(|line| line.starts_with("90  ")));
        assert!(!output.lines().any(|line| line.starts_with("10  ")));
        assert!(output.contains("scanned=90..=90 truncation=before"));
    }
}
