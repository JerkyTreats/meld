//! Health surface computation over ledger, watermark, and registry.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use chrono::DateTime;

use crate::error::StorageError;
use crate::events::observability::{
    ConsumerLagReport, DomainAppendRate, EventHealthReport, LedgerObservability,
};
use crate::events::store::EventStore;

/// Trailing event-count window for append rates. Sequence order is the
/// ledger's native clock, so the window is bounded by events, not time; the
/// observed wall-clock span travels alongside each rate.
const APPEND_RATE_WINDOW_EVENTS: u64 = 512;

pub(super) fn compute(backing: &LedgerObservability) -> Result<EventHealthReport, StorageError> {
    let store = backing.store();
    let tip_seq = store.tip_seq()?;
    let committed_watermark = backing.watermark().committed_seq();
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

    let append_rates = compute_append_rates(store, tip_seq, retained_from)?;

    Ok(EventHealthReport {
        tip_seq,
        committed_watermark,
        retained_from,
        dropped_events,
        consumers,
        append_rates,
    })
}

fn compute_append_rates(
    store: &EventStore,
    tip_seq: u64,
    retained_from: u64,
) -> Result<Vec<DomainAppendRate>, StorageError> {
    // The window cursor never dips below retained history, so a raised
    // compaction boundary shortens the window instead of failing health
    // with a retention gap.
    let after_seq = tip_seq
        .saturating_sub(APPEND_RATE_WINDOW_EVENTS)
        .max(retained_from.saturating_sub(1));
    let window =
        store.read_all_events_after_limit(after_seq, APPEND_RATE_WINDOW_EVENTS as usize)?;

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
    Ok(rates)
}

fn observed_window_seconds(first: Option<&str>, last: Option<&str>) -> Option<u64> {
    let first = DateTime::parse_from_rfc3339(first?).ok()?;
    let last = DateTime::parse_from_rfc3339(last?).ok()?;
    // Clock skew can order timestamps against sequence order; a negative
    // span reads as zero rather than wrapping.
    Some((last - first).num_seconds().max(0) as u64)
}
