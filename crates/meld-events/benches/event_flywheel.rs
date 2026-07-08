//! Macro benchmark simulating the cognitive flywheel over the event ledger.
//!
//! The dependency direction forbids importing meld-world-model here, so this
//! bench carries a reducer-shaped consumer of its own: it owns an in-memory
//! cursor, reads batches of 256 events past the cursor per tick, applies them
//! with a cheap event-type match, and appends one derived projection event per
//! four source events through the idempotent path with deterministic
//! record_ids. Derived events flow back through the same ledger; only source
//! domains count toward derivation so the loop cannot feed on itself.
//!
//! Two measurements:
//! - sustained producer+consumer throughput for a fixed event count, and
//! - append-to-projection-visible latency with a caught-up consumer on top of
//!   a 5,000-event history (each tick pays the current full-tree scan, which
//!   is exactly the latency being indicted).
//!
//! Set `MELD_EVENT_BENCH_LARGE=1` to run the throughput leg at the full
//! 10,000 events; the default is 2,000 because the idempotent append path
//! full-scans the event tree for every novel record_id, which makes the run
//! cost quadratic (~45s per iteration at 10k on the current baseline).

use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use meld_events::events::store::EventStore;
use meld_events::{DomainObjectRef, EventEnvelope, EventRecord, EventRelation};
use serde_json::json;
use tempfile::TempDir;

const TS: &str = "2026-07-08T00:00:00.000Z";
const BATCH_LIMIT: usize = 256;
const SOURCE_EVENTS_PER_DERIVED: u64 = 4;

fn temp_store() -> (TempDir, EventStore) {
    let dir = TempDir::new().expect("create bench tempdir");
    let db = sled::open(dir.path()).expect("open bench sled db");
    let store = EventStore::new(db).expect("open bench event store");
    (dir, store)
}

// Realistic producer mix: 60% execution task/control, 25% workspace_fs,
// 15% context, spread across 4 sessions, with graph refs on every fifth
// event to keep envelope sizes representative.
fn mixed_envelope(i: usize) -> EventEnvelope {
    let session = format!("session-{}", i % 4);
    let bucket = i % 100;
    let (domain, event_type) = if bucket < 60 {
        if bucket.is_multiple_of(2) {
            ("execution", "execution.task.completed")
        } else {
            ("execution", "execution.control.tick")
        }
    } else if bucket < 85 {
        ("workspace_fs", "workspace_fs.file.changed")
    } else {
        ("context", "context.belief.updated")
    };
    let mut envelope = EventEnvelope::new_domain(
        TS.to_string(),
        session.clone(),
        domain,
        session,
        event_type,
        None,
        json!({ "i": i }),
    );
    if bucket.is_multiple_of(5) {
        let src = DomainObjectRef::new(domain, "task_run", format!("run-{i}")).unwrap();
        let dst = DomainObjectRef::new(domain, "artifact", format!("artifact-{i}")).unwrap();
        let relation = EventRelation::new("produced", src.clone(), dst.clone()).unwrap();
        envelope = envelope.with_graph(vec![src, dst], vec![relation]);
    }
    envelope
}

struct ReducerConsumer {
    cursor: u64,
    source_applied: u64,
    derived_emitted: u64,
}

impl ReducerConsumer {
    fn new() -> Self {
        Self {
            cursor: 0,
            source_applied: 0,
            derived_emitted: 0,
        }
    }

    // One supervisor-style tick: read up to 256 events past the cursor,
    // "apply" them, emit derived projections, advance the cursor to the max
    // source seq seen.
    fn tick(&mut self, store: &EventStore) -> Vec<EventRecord> {
        let batch = store
            .read_all_events_after_limit(self.cursor, BATCH_LIMIT)
            .expect("consumer read");
        for record in &batch {
            self.cursor = self.cursor.max(record.seq);
            // Derived world_model events come back around through the ledger;
            // exclude them from derivation so the flywheel does not feed on
            // its own output.
            if !record.event_type.starts_with("world_model.") {
                self.source_applied += 1;
            }
        }
        while self.source_applied / SOURCE_EVENTS_PER_DERIVED > self.derived_emitted {
            self.derived_emitted += 1;
            let envelope = EventEnvelope::new_domain(
                TS.to_string(),
                "flywheel",
                "world_model",
                "projection",
                "world_model.projection.updated",
                None,
                json!({ "derived": self.derived_emitted }),
            )
            .with_record_id(format!("projection-{}", self.derived_emitted));
            store
                .append_envelope_idempotent(envelope)
                .expect("derived append");
        }
        batch
    }

    fn drain(&mut self, store: &EventStore) {
        loop {
            if self.tick(store).is_empty() {
                break;
            }
        }
    }
}

fn flywheel_throughput(c: &mut Criterion) {
    const PRODUCE_CHUNK: usize = 512;
    // Every derived append pays an O(history) scan on the current baseline
    // (record-index miss falls back to a full event-tree scan), so run cost
    // grows quadratically with total events; the 10k leg is opt-in.
    let total_events: usize = if std::env::var("MELD_EVENT_BENCH_LARGE").is_ok() {
        10_000
    } else {
        2_000
    };

    let mut group = c.benchmark_group("flywheel_throughput");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));
    group.throughput(Throughput::Elements(total_events as u64));

    group.bench_function(BenchmarkId::new("produce_consume", total_events), |b| {
        b.iter_batched(
            temp_store,
            |(dir, store)| {
                let mut consumer = ReducerConsumer::new();
                // Producer flushes once per chunk rather than per event; the
                // per-event fsync cost is measured separately in the ledger
                // micro benches.
                for chunk_start in (0..total_events).step_by(PRODUCE_CHUNK) {
                    let chunk_end = (chunk_start + PRODUCE_CHUNK).min(total_events);
                    for i in chunk_start..chunk_end {
                        store.append_envelope(mixed_envelope(i)).unwrap();
                    }
                    store.flush().unwrap();
                    loop {
                        let batch = consumer.tick(&store);
                        if batch.len() < BATCH_LIMIT {
                            break;
                        }
                    }
                }
                consumer.drain(&store);
                black_box(consumer.cursor);
                // Returned so tempdir and store teardown stay outside the
                // timed routine.
                (dir, store)
            },
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

/// Copies a sled directory so each iteration opens an identical history.
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let target = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir_recursive(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn flywheel_latency(c: &mut Criterion) {
    const HISTORY: usize = 5_000;

    // The timed routine appends into the store, so a shared fixture would
    // grow across iterations and later samples would measure a larger
    // history. A template history is built once, then every iteration
    // copies it and catches a consumer up in untimed setup, keeping each
    // measurement at exactly HISTORY events.
    let template_dir = tempfile::TempDir::new().unwrap();
    {
        let db = sled::open(template_dir.path()).unwrap();
        let store = EventStore::new(db).unwrap();
        for i in 0..HISTORY {
            store.append_envelope(mixed_envelope(i)).unwrap();
        }
        store.flush().unwrap();
    }

    let mut group = c.benchmark_group("flywheel_latency");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));

    group.bench_function("append_to_projection_visible", |b| {
        b.iter_batched(
            || {
                let dir = tempfile::TempDir::new().unwrap();
                copy_dir_recursive(template_dir.path(), dir.path());
                let db = sled::open(dir.path()).unwrap();
                let store = EventStore::new(db).unwrap();
                let mut consumer = ReducerConsumer::new();
                consumer.drain(&store);
                (dir, store, consumer)
            },
            |(dir, store, mut consumer)| {
                let envelope = EventEnvelope::new_domain(
                    TS.to_string(),
                    "session-0",
                    "execution",
                    "session-0",
                    "execution.task.completed",
                    None,
                    json!({ "marker": HISTORY }),
                );
                let seq = store.append_envelope(envelope).unwrap();
                store.flush().unwrap();
                loop {
                    let batch = consumer.tick(&store);
                    if batch.iter().any(|record| record.seq >= seq) {
                        break;
                    }
                }
                (dir, store, consumer)
            },
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

fn flywheel_benches(c: &mut Criterion) {
    flywheel_throughput(c);
    flywheel_latency(c);
}

criterion_group!(benches, flywheel_benches);
criterion_main!(benches);
