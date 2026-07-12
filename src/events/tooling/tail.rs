//! Tail subcommand: pages ledger records after a cursor and follows new
//! commits.
//!
//! One-shot mode returns a single rendered page. Follow mode prints pages as
//! they commit and runs until the process is interrupted; per the PLAN it is
//! the one intentionally long-running command in the `meld event` family.

use std::io::Write;

use meld_events::{
    EventObservabilityCapability, EventPage, EventRecord, EventReplayCapability,
    EventSubscriptionCapability, LedgerCursor, ReplayRequest, SubscriptionPollRequest,
};

use crate::error::ApiError;
use crate::events::tooling::{render, surface_authority_error};

/// Per-page wait in follow mode: short enough that interruption feels
/// immediate, long enough to avoid busy polling on a quiet ledger.
const FOLLOW_PAGE_TIMEOUT_MS: u64 = 500;

pub(super) fn run(
    observability: &EventObservabilityCapability,
    replay: &EventReplayCapability,
    subscription: &EventSubscriptionCapability,
    format: &str,
    after: Option<u64>,
    limit: usize,
    follow: bool,
) -> Result<String, ApiError> {
    if follow {
        follow_forever(observability, subscription, format, after, limit)
    } else {
        // Tail semantics: without a cursor, select the most recent records
        // by count. Sequence subtraction is incorrect for sparse migrated
        // ledgers whose adjacent records need not have adjacent numbers.
        let page = match after {
            Some(seq) => replay_page(replay, seq, limit)?,
            None => replay
                .newest_page(limit)
                .map_err(|err| surface_authority_error("tail", err))?,
        };
        render(format, &page, render_page_text)
    }
}

/// Prints committed pages to stdout as they arrive until interrupted or the
/// output pipe closes; a closed pipe ends the command cleanly so the session
/// lifecycle still completes.
fn follow_forever(
    observability: &EventObservabilityCapability,
    subscription: &EventSubscriptionCapability,
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
            observability
                .health(observability.ledger_identity())
                .map_err(|err| surface_authority_error("tail", err))?
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
        let page = poll_page(subscription, cursor, limit, FOLLOW_PAGE_TIMEOUT_MS)?;
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

fn replay_page(
    replay: &EventReplayCapability,
    after_seq: u64,
    limit: usize,
) -> Result<EventPage, ApiError> {
    replay
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: replay.ledger_identity(),
                after_seq,
            },
            limit,
        })
        .map_err(|err| surface_authority_error("tail", err))
}

fn poll_page(
    subscription: &EventSubscriptionCapability,
    after_seq: u64,
    limit: usize,
    timeout_ms: u64,
) -> Result<EventPage, ApiError> {
    subscription
        .poll(SubscriptionPollRequest {
            replay: ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id: subscription.ledger_identity(),
                    after_seq,
                },
                limit,
            },
            timeout_ms,
        })
        .map_err(|err| surface_authority_error("tail", err))
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
    use meld_events::{
        AppendMode, CoverageTruncation, EventAuthority, EventAuthorityOpenOptions, EventEnvelope,
        EventReadCoverage, LedgerIdentity,
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

    fn authority() -> EventAuthority {
        EventAuthority::open(
            sled::Config::new().temporary(true).open().unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap()
    }

    fn append(authority: &EventAuthority, session: &str, stream: &str) {
        authority
            .append_capability()
            .append_durable(
                EventEnvelope::new_domain(
                    "2026-07-08T12:00:00Z".to_string(),
                    session,
                    "execution",
                    stream,
                    "execution.task.progress",
                    None,
                    json!({}),
                ),
                AppendMode::Plain,
            )
            .unwrap();
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
        let authority = authority();
        append(&authority, "old", "old");
        append(&authority, "new", "new");
        let output = run(
            &authority.observability_capability(),
            &authority.replay_capability(),
            &authority.subscription_capability(),
            "text",
            None,
            1,
            false,
        )
        .unwrap();
        assert!(output.lines().any(|line| line.starts_with("2  ")));
        assert!(!output.lines().any(|line| line.starts_with("1  ")));
        assert!(output.contains("scanned=2..=2 truncation=before"));
    }

    #[test]
    fn one_shot_json_is_pretty_page() {
        let authority = authority();
        append(&authority, "s", "s");
        let output = run(
            &authority.observability_capability(),
            &authority.replay_capability(),
            &authority.subscription_capability(),
            "json",
            Some(0),
            16,
            false,
        )
        .unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["next_cursor"]["after_seq"], 1);
        // Pretty rendering distinguishes the one-shot page from the
        // follow-mode single-line object.
        assert!(output.contains('\n'));
    }

    #[test]
    fn authority_capability_rejects_zero_limit_honestly() {
        let authority = authority();
        let error = run(
            &authority.observability_capability(),
            &authority.replay_capability(),
            &authority.subscription_capability(),
            "text",
            Some(0),
            0,
            false,
        )
        .unwrap_err();
        assert!(error.to_string().contains("replay limit must be in"));
    }
}
