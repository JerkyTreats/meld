//! Health surface computation over ledger, watermark, and registry.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use chrono::DateTime;

use crate::error::StorageError;
use crate::events::observability::{
    coverage_truncation, ConsumerLagReport, DomainAppendRate, EventHealthReport, EventReadCoverage,
    LedgerObservability,
};
use crate::events::store::EventStore;

/// Trailing event-count window for append rates. Sequence order is the
/// ledger's native clock, so the window is bounded by events, not time; the
/// observed wall-clock span travels alongside each rate.
const APPEND_RATE_WINDOW_EVENTS: usize = 512;

pub(super) fn compute(backing: &LedgerObservability) -> Result<EventHealthReport, StorageError> {
    let store = backing.store();
    let ledger_id = backing.ledger_identity();
    let tip_seq = store.tip_seq()?;
    // The writer can advance between freezing the durable tip and sampling
    // its watermark. Clamp that later observation to the report's frozen
    // ledger snapshot so the report never claims a commit it did not scan.
    let committed_watermark = backing.watermark().committed_seq().min(tip_seq);
    let retained_from = store.retained_lower_boundary()?;
    let dropped_events = backing.dropped_events();

    let consumers = backing
        .consumer_snapshot()?
        .into_iter()
        .map(|cursor| ConsumerLagReport {
            lag: committed_watermark.saturating_sub(cursor.reported_seq),
            name: cursor.name,
            reported_seq: cursor.reported_seq,
        })
        .collect();

    let (append_rates, append_rate_coverage) = compute_append_rates(store, tip_seq, retained_from)?;

    Ok(EventHealthReport {
        ledger_id,
        tip_seq,
        committed_watermark,
        retained_from,
        dropped_events,
        consumers,
        append_rates,
        append_rate_coverage,
    })
}

fn compute_append_rates(
    store: &EventStore,
    tip_seq: u64,
    retained_from: u64,
) -> Result<(Vec<DomainAppendRate>, EventReadCoverage), StorageError> {
    let mut window =
        store.read_newest_events_through(tip_seq, APPEND_RATE_WINDOW_EVENTS.saturating_add(1))?;
    let truncated_by_limit = window.len() > APPEND_RATE_WINDOW_EVENTS;
    if truncated_by_limit {
        window.remove(0);
    }
    let coverage = EventReadCoverage {
        retained_from,
        tip_seq,
        scanned_from_seq: window.first().map(|record| record.seq),
        scanned_through_seq: window.last().map(|record| record.seq),
        truncation: coverage_truncation(retained_from > 1 || truncated_by_limit, false),
    };

    let window_seconds = observed_window_seconds(
        window
            .first()
            .map(|record| record.envelope.recorded_at.as_str()),
        window
            .last()
            .map(|record| record.envelope.recorded_at.as_str()),
    );

    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for record in &window {
        *counts.entry(record.envelope.domain_id.clone()).or_default() += 1;
    }

    let mut rates: Vec<DomainAppendRate> = counts
        .into_iter()
        .map(|(domain_id, events_in_window)| DomainAppendRate {
            domain_id,
            events_in_window,
            window_seconds,
        })
        .collect();
    rates.sort_by(|a, b| {
        (Reverse(a.events_in_window), &a.domain_id)
            .cmp(&(Reverse(b.events_in_window), &b.domain_id))
    });
    Ok((rates, coverage))
}

fn observed_window_seconds(first: Option<&str>, last: Option<&str>) -> Option<u64> {
    let first = DateTime::parse_from_rfc3339(first?).ok()?;
    let last = DateTime::parse_from_rfc3339(last?).ok()?;
    // Clock skew can order timestamps against sequence order; a negative
    // span reads as zero rather than wrapping.
    Some((last - first).num_seconds().max(0) as u64)
}
