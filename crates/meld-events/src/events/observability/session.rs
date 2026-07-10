//! Session timeline and flow window computation.
//!
//! Owner: event observability, session surface unit.
//! Both reports read the ledger store directly and summarize structure only:
//! sequences, identity, and recorded times. Timestamps are best-effort wall
//! clock; timing fields go absent rather than failing the report when a
//! recorded time does not parse as RFC 3339.

use std::collections::BTreeMap;

use chrono::{DateTime, FixedOffset};

use crate::error::StorageError;
use crate::events::observability::{
    CoverageTruncation, DomainFlow, EventFlowReport, EventReadCoverage, FlowWindow,
    LedgerObservability, SessionStep, SessionTimelineReport, SilentDomain, TypeFlow,
    MAX_SESSION_SCAN_EVENTS,
};
use crate::events::EventRecord;

/// How many events before the window start the silent-domain census reads.
///
/// A full-history domain census needs a per-domain index; until volumes
/// demand one, a bounded trailing slice keeps flow cost constant while still
/// catching domains that went quiet recently — the case the report exists
/// for.
const SILENT_CENSUS_EVENTS: usize = 512;

pub(super) fn compute_timeline(
    backing: &LedgerObservability,
    session_id: &str,
) -> Result<SessionTimelineReport, StorageError> {
    compute_timeline_with_scan_limit(backing, session_id, MAX_SESSION_SCAN_EVENTS)
}

fn compute_timeline_with_scan_limit(
    backing: &LedgerObservability,
    session_id: &str,
    scan_limit: usize,
) -> Result<SessionTimelineReport, StorageError> {
    let store = backing.store();
    let tip_seq = store.tip_seq()?;
    let retained_from = store.retained_lower_boundary()?;
    let mut scanned = store.read_newest_events_through(tip_seq, scan_limit.saturating_add(1))?;
    let truncated_by_limit = scanned.len() > scan_limit;
    if truncated_by_limit {
        scanned.remove(0);
    }
    let coverage = EventReadCoverage {
        retained_from,
        tip_seq,
        scanned_from_seq: scanned.first().map(|record| record.seq),
        scanned_through_seq: scanned.last().map(|record| record.seq),
        truncation: if retained_from > 1 || truncated_by_limit {
            CoverageTruncation::Before
        } else {
            CoverageTruncation::None
        },
    };
    let records: Vec<EventRecord> = scanned
        .into_iter()
        .filter(|record| record.session == session_id)
        .collect();

    let mut steps = Vec::with_capacity(records.len());
    let mut previous: Option<DateTime<FixedOffset>> = None;
    for record in &records {
        let current = parse_recorded_at(record);
        // The first step has no predecessor; later steps lose their gap when
        // either endpoint's timestamp is unparseable or the clock ran
        // backwards, since a fabricated number is worse than an absent one.
        let gap_ms = match (previous, current) {
            (Some(prev), Some(curr)) => u64::try_from((curr - prev).num_milliseconds()).ok(),
            _ => None,
        };
        previous = current;
        steps.push(SessionStep {
            seq: record.seq,
            recorded_at: record.envelope.recorded_at.clone(),
            domain_id: record.envelope.domain_id.clone(),
            event_type: record.envelope.event_type.clone(),
            gap_ms,
        });
    }

    Ok(SessionTimelineReport {
        session_id: session_id.to_string(),
        observed_started_at: records
            .first()
            .map(|record| record.envelope.recorded_at.clone()),
        observed_ended_at: records
            .last()
            .map(|record| record.envelope.recorded_at.clone()),
        events_returned: records.len() as u64,
        coverage,
        steps,
    })
}

pub(super) fn compute_flow(
    backing: &LedgerObservability,
    window: FlowWindow,
) -> Result<EventFlowReport, StorageError> {
    let store = backing.store();
    let tip = store.tip_seq()?;

    let records = store.read_newest_events_through(tip, window.max_events)?;

    let span_seconds = match (records.first(), records.last()) {
        (Some(first), Some(last)) => match (parse_recorded_at(first), parse_recorded_at(last)) {
            (Some(start), Some(end)) => u64::try_from((end - start).num_seconds()).ok(),
            _ => None,
        },
        _ => None,
    };

    // BTreeMap keys give the deterministic tie order for free: equal counts
    // fall back to ascending domain or type identifier.
    let mut domains: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    let mut types: BTreeMap<String, u64> = BTreeMap::new();
    for record in &records {
        let entry = domains
            .entry(record.envelope.domain_id.clone())
            .or_insert((0, 0));
        entry.0 += 1;
        entry.1 = entry.1.max(record.seq);
        *types.entry(record.envelope.event_type.clone()).or_insert(0) += 1;
    }

    let mut by_domain: Vec<DomainFlow> = domains
        .into_iter()
        .map(|(domain_id, (count, last_seq))| DomainFlow {
            domain_id,
            count,
            last_seq,
        })
        .collect();
    by_domain.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.domain_id.cmp(&b.domain_id))
    });

    let mut by_type: Vec<TypeFlow> = types
        .into_iter()
        .map(|(event_type, count)| TypeFlow { event_type, count })
        .collect();
    by_type.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.event_type.cmp(&b.event_type))
    });

    let silent_domains = compute_silent_domains(backing, &records, &by_domain)?;

    Ok(EventFlowReport {
        window_events: records.len() as u64,
        span_seconds,
        by_domain,
        by_type,
        silent_domains,
    })
}

/// Finds domains active shortly before the window that went quiet inside it.
///
/// The census reads one bounded slice of [`SILENT_CENSUS_EVENTS`] records
/// preceding the window start; domains that last emitted earlier than that
/// slice are invisible to it. A full-history domain census is future work
/// when volumes demand a per-domain index.
fn compute_silent_domains(
    backing: &LedgerObservability,
    window_records: &[EventRecord],
    by_domain: &[DomainFlow],
) -> Result<Vec<SilentDomain>, StorageError> {
    let Some(window_start) = window_records.first().map(|record| record.seq) else {
        // An empty window means an empty (or fully compacted) ledger; there
        // is no "before the window" to census.
        return Ok(Vec::new());
    };

    let preceding = backing
        .store()
        .read_newest_events_through(window_start.saturating_sub(1), SILENT_CENSUS_EVENTS)?;

    let mut last_seen: BTreeMap<String, (u64, String)> = BTreeMap::new();
    for record in &preceding {
        // The limit can spill past sparse preceding history into the window
        // itself; only records strictly before the window start count.
        if record.seq >= window_start {
            break;
        }
        last_seen.insert(
            record.envelope.domain_id.clone(),
            (record.seq, record.envelope.recorded_at.clone()),
        );
    }

    Ok(last_seen
        .into_iter()
        .filter(|(domain_id, _)| !by_domain.iter().any(|flow| &flow.domain_id == domain_id))
        .map(|(domain_id, (last_seq, last_recorded_at))| SilentDomain {
            domain_id,
            last_seq,
            last_recorded_at,
        })
        .collect())
}

fn parse_recorded_at(record: &EventRecord) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(&record.envelope.recorded_at).ok()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::*;
    use crate::events::registry::EventCursorRegistry;
    use crate::events::store::EventStore;
    use crate::events::writer::EventWriter;
    use crate::events::EventEnvelope;

    #[test]
    fn bounded_session_selects_newest_records_from_sparse_sequence_space() {
        assert_eq!(MAX_SESSION_SCAN_EVENTS, 100_000);
        let dir = tempfile::TempDir::new().unwrap();
        let db = sled::open(dir.path()).unwrap();
        let store = EventStore::shared(db.clone()).unwrap();
        let registry = EventCursorRegistry::open(&db).unwrap();
        let writer = EventWriter::spawn(Arc::clone(&store));
        let port = LedgerObservability::new(
            Arc::clone(&store),
            writer.watermark(),
            registry,
            writer.dropped_handle(),
        );

        for seq in [1, 100, 10_000] {
            let envelope = EventEnvelope::new_domain(
                "2026-07-08T00:00:00Z".to_string(),
                "session-a",
                "execution",
                "run-a",
                "execution.tick",
                None,
                json!({ "seq": seq }),
            );
            store.append_event(&EventRecord { seq, envelope }).unwrap();
        }

        let report = compute_timeline_with_scan_limit(&port, "session-a", 2).unwrap();
        assert_eq!(
            report.steps.iter().map(|step| step.seq).collect::<Vec<_>>(),
            vec![100, 10_000]
        );
        assert_eq!(report.events_returned, 2);
        assert_eq!(report.coverage.scanned_from_seq, Some(100));
        assert_eq!(report.coverage.scanned_through_seq, Some(10_000));
        assert_eq!(report.coverage.truncation, CoverageTruncation::Before);
    }
}
