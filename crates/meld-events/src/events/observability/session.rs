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
    DomainFlow, EventFlowReport, FlowWindow, LedgerObservability, SessionStep,
    SessionTimelineReport, SilentDomain, TypeFlow,
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
    let records = backing.store().read_events_after(
        session_id,
        // Diagnostic reads degrade to everything-retained instead of
        // tripping the retention gap; sessions above the boundary read
        // identically either way.
        backing.store().retained_lower_boundary()?.saturating_sub(1),
    )?;

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
        started_at: records
            .first()
            .map(|record| record.envelope.recorded_at.clone()),
        ended_at: records
            .last()
            .map(|record| record.envelope.recorded_at.clone()),
        total_events: records.len() as u64,
        steps,
    })
}

pub(super) fn compute_flow(
    backing: &LedgerObservability,
    window: FlowWindow,
) -> Result<EventFlowReport, StorageError> {
    let store = backing.store();
    let tip = store.tip_seq()?;
    let retained_from = store.retained_lower_boundary()?;

    // Seek to the trailing window by cursor, then clamp the cursor up to the
    // retained boundary before reading: a window larger than retained history
    // must degrade to "everything retained", never surface a retention-gap
    // error for a purely diagnostic read.
    let after_seq = clamp_to_retention(tip.saturating_sub(window.max_events as u64), retained_from);
    let records = store.read_all_events_after_limit(after_seq, window.max_events)?;

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

    let silent_domains = compute_silent_domains(backing, &records, retained_from, &by_domain)?;

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
    retained_from: u64,
    by_domain: &[DomainFlow],
) -> Result<Vec<SilentDomain>, StorageError> {
    let Some(window_start) = window_records.first().map(|record| record.seq) else {
        // An empty window means an empty (or fully compacted) ledger; there
        // is no "before the window" to census.
        return Ok(Vec::new());
    };

    let census_after = clamp_to_retention(
        window_start.saturating_sub(SILENT_CENSUS_EVENTS as u64 + 1),
        retained_from,
    );
    let preceding = backing
        .store()
        .read_all_events_after_limit(census_after, SILENT_CENSUS_EVENTS)?;

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

/// Raises a read cursor to the retained boundary so bounded diagnostic reads
/// never trip the store's typed retention gap.
fn clamp_to_retention(after_seq: u64, retained_from: u64) -> u64 {
    after_seq.max(retained_from.saturating_sub(1))
}

fn parse_recorded_at(record: &EventRecord) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(&record.envelope.recorded_at).ok()
}
