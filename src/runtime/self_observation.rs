//! Promoted runtime-health facts: the runtime observing itself.
//!
//! A threshold watcher over the ledger health report and supervisor restart
//! counts emits `runtime` domain facts through the durable class, so the
//! world model can reduce the runtime's own health like any other domain.
//!
//! The inclusion rule is sovereign here: gauges and per-tick samples never
//! enter the ledger. Each condition is a state machine that fires once per
//! crossing with an idempotent record id and re-arms only when the condition
//! clears; a quiet runtime emits nothing. Watcher state is process-local, so
//! a restart re-observes a still-standing condition as a new crossing under
//! a new id — a new observation epoch, recorded in the PLAN.

use std::collections::BTreeMap;

use meld_events::{EventEnvelope, EventHealthReport};
use meld_execution::task_network::EventAppendSink;
use serde_json::json;

/// Default consumer lag that counts as fallen behind.
const DEFAULT_LAG_THRESHOLD: u64 = 1024;

/// Session partition for runtime self-observation facts.
const RUNTIME_SESSION: &str = "runtime_self_observation";

/// Threshold watcher emitting once-per-crossing runtime facts.
pub struct SelfObservationWatcher {
    lag_threshold: u64,
    storm_threshold: u64,
    lag_fired: BTreeMap<String, bool>,
    gap_fired: BTreeMap<String, bool>,
    storm_fired: BTreeMap<String, bool>,
    drops_fired: bool,
    last_dropped: Option<u64>,
}

impl SelfObservationWatcher {
    /// Creates a watcher with every condition armed.
    ///
    /// The storm threshold is the supervisor's configured restart attempt
    /// limit rather than an independent constant, so a storm is exactly
    /// "restarts exhausted the limit". A limit of zero disables restarts
    /// entirely; the floor of one keeps the watcher well formed there, but
    /// no storm fact can fire because the count never leaves zero.
    pub fn new(restart_attempt_limit: u64) -> Self {
        Self::with_thresholds(DEFAULT_LAG_THRESHOLD, restart_attempt_limit.max(1))
    }

    /// Creates a watcher with explicit thresholds.
    pub fn with_thresholds(lag_threshold: u64, storm_threshold: u64) -> Self {
        Self {
            lag_threshold: lag_threshold.max(1),
            storm_threshold: storm_threshold.max(1),
            lag_fired: BTreeMap::new(),
            gap_fired: BTreeMap::new(),
            storm_fired: BTreeMap::new(),
            drops_fired: false,
            last_dropped: None,
        }
    }

    /// Observes one health report plus restart counts and emits threshold
    /// crossings durably; returns how many facts were emitted.
    ///
    /// Emission failures leave the condition fired rather than retrying every
    /// tick: the idempotent record id makes a later manual replay safe, and a
    /// runtime that cannot append has louder problems the heartbeat path
    /// already reports.
    pub fn observe(
        &mut self,
        health: &EventHealthReport,
        restart_counts: &[(String, u64)],
        epoch: &str,
        sink: &impl EventAppendSink,
    ) -> usize {
        let mut emitted = 0;

        // Fired state for consumers absent from this report is dropped so
        // the maps stay bounded by live registry names; a consumer that
        // returns still lagging counts as a new crossing.
        let live: std::collections::BTreeSet<&str> = health
            .consumers
            .iter()
            .map(|consumer| consumer.name.as_str())
            .collect();
        self.lag_fired
            .retain(|name, _| live.contains(name.as_str()));
        self.gap_fired
            .retain(|name, _| live.contains(name.as_str()));

        for consumer in &health.consumers {
            let fired = self.lag_fired.entry(consumer.name.clone()).or_default();
            if consumer.lag >= self.lag_threshold && !*fired {
                *fired = true;
                emitted += emit(
                    sink,
                    "runtime.consumer_lag_exceeded",
                    format!(
                        "runtime::consumer_lag_exceeded::{}::{}",
                        consumer.name, health.committed_watermark
                    ),
                    json!({
                        "consumer": consumer.name,
                        "lag": consumer.lag,
                        "reported_seq": consumer.reported_seq,
                        "committed_watermark": health.committed_watermark,
                        "threshold": self.lag_threshold,
                    }),
                );
            } else if consumer.lag < self.lag_threshold {
                *fired = false;
            }

            // A consumer whose next read predates retained history is
            // stranded: replay would gap, so the fact fires until the
            // cursor moves past the boundary.
            let gap_fired = self.gap_fired.entry(consumer.name.clone()).or_default();
            let gapped = consumer.reported_seq.saturating_add(1) < health.retained_from;
            if gapped && !*gap_fired {
                *gap_fired = true;
                emitted += emit(
                    sink,
                    "runtime.retention_gap_encountered",
                    format!(
                        "runtime::retention_gap_encountered::{}::{}",
                        consumer.name, health.retained_from
                    ),
                    json!({
                        "consumer": consumer.name,
                        "reported_seq": consumer.reported_seq,
                        "retained_from": health.retained_from,
                    }),
                );
            } else if !gapped {
                *gap_fired = false;
            }
        }

        // Drops are cumulative for the process lifetime: the burst fact
        // fires when they first increase and re-arms only after an
        // observation with no new drops.
        if let Some(last) = self.last_dropped {
            if health.dropped_events > last && !self.drops_fired {
                self.drops_fired = true;
                emitted += emit(
                    sink,
                    "runtime.ingest_drops_burst",
                    format!("runtime::ingest_drops_burst::{}", health.dropped_events),
                    json!({
                        "dropped_events": health.dropped_events,
                        "new_drops": health.dropped_events - last,
                    }),
                );
            } else if health.dropped_events == last {
                self.drops_fired = false;
            }
        }
        self.last_dropped = Some(health.dropped_events);

        for (runtime_id, restart_count) in restart_counts {
            let fired = self.storm_fired.entry(runtime_id.clone()).or_default();
            if *restart_count >= self.storm_threshold && !*fired {
                *fired = true;
                // Restart counts reset every supervisor run, so the epoch
                // keeps genuinely distinct storms from deduping into the
                // first one ever recorded.
                emitted += emit(
                    sink,
                    "runtime.restart_storm",
                    format!("runtime::restart_storm::{runtime_id}::{epoch}::{restart_count}"),
                    json!({
                        "runtime_id": runtime_id,
                        "restart_count": restart_count,
                        "threshold": self.storm_threshold,
                    }),
                );
            } else if *restart_count < self.storm_threshold {
                *fired = false;
            }
        }

        emitted
    }
}

fn emit(
    sink: &impl EventAppendSink,
    event_type: &str,
    record_id: String,
    data: serde_json::Value,
) -> usize {
    let envelope = EventEnvelope::with_now_domain(
        RUNTIME_SESSION,
        "runtime",
        "self_observation",
        event_type,
        None,
        data,
    )
    .with_record_id(record_id);
    match sink.append_envelope_idempotent(envelope) {
        Ok(_) => 1,
        Err(error) => {
            tracing::warn!(error = %error, event_type, "failed to emit runtime fact");
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use meld_events::ConsumerLagReport;

    use super::*;

    #[derive(Default)]
    struct RecordingSink {
        envelopes: RefCell<Vec<EventEnvelope>>,
    }

    impl EventAppendSink for RecordingSink {
        fn append_envelope_idempotent(&self, envelope: EventEnvelope) -> Result<u64, String> {
            self.envelopes.borrow_mut().push(envelope);
            Ok(self.envelopes.borrow().len() as u64)
        }
    }

    fn health(lag: u64, dropped: u64, retained_from: u64) -> EventHealthReport {
        EventHealthReport {
            tip_seq: 10_000,
            committed_watermark: 10_000,
            retained_from,
            dropped_events: dropped,
            consumers: vec![ConsumerLagReport {
                name: "world_state.graph.reducer".to_string(),
                reported_seq: 10_000 - lag,
                lag,
            }],
            append_rates: Vec::new(),
        }
    }

    #[test]
    fn quiet_runtime_emits_nothing_under_storm_of_observations() {
        let sink = RecordingSink::default();
        let mut watcher = SelfObservationWatcher::new(3);
        for _ in 0..1_000 {
            assert_eq!(
                watcher.observe(&health(0, 0, 1), &[], "instance-a", &sink),
                0
            );
        }
        assert!(sink.envelopes.borrow().is_empty());
    }

    #[test]
    fn lag_crossing_fires_once_and_rearms_after_recovery() {
        let sink = RecordingSink::default();
        let mut watcher = SelfObservationWatcher::new(3);

        for _ in 0..100 {
            watcher.observe(
                &health(DEFAULT_LAG_THRESHOLD + 5, 0, 1),
                &[],
                "instance-a",
                &sink,
            );
        }
        assert_eq!(sink.envelopes.borrow().len(), 1);
        assert_eq!(
            sink.envelopes.borrow()[0].event_type,
            "runtime.consumer_lag_exceeded"
        );
        assert_eq!(sink.envelopes.borrow()[0].domain_id, "runtime");
        assert!(sink.envelopes.borrow()[0].record_id.is_some());

        // Recovery re-arms; the next crossing fires exactly once more.
        watcher.observe(&health(0, 0, 1), &[], "instance-a", &sink);
        for _ in 0..100 {
            watcher.observe(
                &health(DEFAULT_LAG_THRESHOLD, 0, 1),
                &[],
                "instance-a",
                &sink,
            );
        }
        assert_eq!(sink.envelopes.borrow().len(), 2);
    }

    #[test]
    fn explicit_thresholds_override_the_defaults() {
        let sink = RecordingSink::default();
        let mut watcher = SelfObservationWatcher::with_thresholds(4, 3);

        // Lag below the explicit threshold but far below the default stays
        // quiet; crossing the explicit threshold fires.
        assert_eq!(
            watcher.observe(&health(3, 0, 1), &[], "instance-a", &sink),
            0
        );
        assert_eq!(
            watcher.observe(&health(4, 0, 1), &[], "instance-a", &sink),
            1
        );
        assert_eq!(
            sink.envelopes.borrow()[0].event_type,
            "runtime.consumer_lag_exceeded"
        );
    }

    #[test]
    fn drops_burst_fires_once_per_burst() {
        let sink = RecordingSink::default();
        let mut watcher = SelfObservationWatcher::new(3);

        watcher.observe(&health(0, 0, 1), &[], "instance-a", &sink);
        for dropped in [5, 9, 12] {
            watcher.observe(&health(0, dropped, 1), &[], "instance-a", &sink);
        }
        assert_eq!(sink.envelopes.borrow().len(), 1);
        assert_eq!(
            sink.envelopes.borrow()[0].event_type,
            "runtime.ingest_drops_burst"
        );

        // A quiet observation re-arms; a new burst fires once more.
        watcher.observe(&health(0, 12, 1), &[], "instance-a", &sink);
        watcher.observe(&health(0, 20, 1), &[], "instance-a", &sink);
        watcher.observe(&health(0, 25, 1), &[], "instance-a", &sink);
        assert_eq!(sink.envelopes.borrow().len(), 2);
    }

    #[test]
    fn retention_gap_fires_once_per_stranding() {
        let sink = RecordingSink::default();
        let mut watcher = SelfObservationWatcher::new(3);

        let mut report = health(0, 0, 1);
        report.consumers[0].reported_seq = 10;
        report.retained_from = 500;
        for _ in 0..50 {
            watcher.observe(&report, &[], "instance-a", &sink);
        }
        assert_eq!(sink.envelopes.borrow().len(), 1);
        assert_eq!(
            sink.envelopes.borrow()[0].event_type,
            "runtime.retention_gap_encountered"
        );
    }

    #[test]
    fn restart_storm_fires_once_per_runtime() {
        let sink = RecordingSink::default();
        let mut watcher = SelfObservationWatcher::new(3);

        let restarts = vec![("world_model.graph_replay".to_string(), 3)];
        for _ in 0..50 {
            watcher.observe(&health(0, 0, 1), &restarts, "instance-a", &sink);
        }
        assert_eq!(sink.envelopes.borrow().len(), 1);
        assert_eq!(
            sink.envelopes.borrow()[0].event_type,
            "runtime.restart_storm"
        );
        assert_eq!(
            sink.envelopes.borrow()[0].record_id.as_deref(),
            Some("runtime::restart_storm::world_model.graph_replay::instance-a::3")
        );
        // A second runtime storms independently.
        let both = vec![
            ("world_model.graph_replay".to_string(), 4),
            ("execution.task_dispatch".to_string(), 3),
        ];
        watcher.observe(&health(0, 0, 1), &both, "instance-a", &sink);
        assert_eq!(sink.envelopes.borrow().len(), 2);
    }

    #[test]
    fn record_ids_are_idempotent_per_crossing() {
        let sink = RecordingSink::default();
        let mut watcher = SelfObservationWatcher::new(3);
        watcher.observe(
            &health(DEFAULT_LAG_THRESHOLD, 0, 1),
            &[],
            "instance-a",
            &sink,
        );
        let record_id = sink.envelopes.borrow()[0].record_id.clone().unwrap();
        assert_eq!(
            record_id,
            "runtime::consumer_lag_exceeded::world_state.graph.reducer::10000"
        );
    }
}
