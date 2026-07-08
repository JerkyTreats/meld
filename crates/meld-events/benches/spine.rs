//! Micro benchmarks for the event spine store and runtime.
//!
//! These benches track the spine cost profile across the overhaul: producer
//! path flush cost, seek-based cursor reads, and atomically sequenced
//! appends. Compare runs against the recorded baselines in the overhaul PLAN
//! to catch regressions.
//!
//! Set `MELD_SPINE_BENCH_LARGE=1` to add a 1,000,000-event history size to the
//! replay and idle-tick curves; it is off by default to keep suite runtime
//! sane.
//!
//! Durability benches fsync through the tempdir filesystem. On machines where
//! `/tmp` is tmpfs the fsync is nearly free and group-commit gains vanish;
//! set `TMPDIR` to a disk-backed path for meaningful durable-append numbers.

use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use meld_events::events::store::EventStore;
use meld_events::{EventEnvelope, EventRuntime, SpineWriter};
use serde_json::json;
use tempfile::TempDir;

const TS: &str = "2026-07-08T00:00:00.000Z";

fn temp_store() -> (TempDir, EventStore) {
    let dir = TempDir::new().expect("create bench tempdir");
    let db = sled::open(dir.path()).expect("open bench sled db");
    let store = EventStore::new(db).expect("open bench event store");
    (dir, store)
}

fn bench_envelope(session: &str, i: usize) -> EventEnvelope {
    EventEnvelope::new_domain(
        TS.to_string(),
        session,
        "execution",
        session,
        "execution.task.progress",
        None,
        json!({ "i": i }),
    )
}

fn populate(store: &EventStore, sessions: usize, total: usize) {
    for i in 0..total {
        let session = format!("session-{}", i % sessions);
        store
            .append_envelope(bench_envelope(&session, i))
            .expect("populate append");
    }
    store.flush().expect("populate flush");
}

fn history_sizes() -> Vec<usize> {
    let mut sizes = vec![1_000, 10_000, 100_000];
    if std::env::var("MELD_SPINE_BENCH_LARGE").is_ok() {
        sizes.push(1_000_000);
    }
    sizes
}

struct HistoryFixture {
    _dir: TempDir,
    store: EventStore,
    size: usize,
}

// Fixtures are built once per size, outside all timing loops, and shared by
// the replay and idle-tick groups.
fn build_history_fixtures() -> Vec<HistoryFixture> {
    history_sizes()
        .into_iter()
        .map(|size| {
            let (dir, store) = temp_store();
            populate(&store, 4, size);
            HistoryFixture {
                _dir: dir,
                store,
                size,
            }
        })
        .collect()
}

fn append_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("append_throughput");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));

    // The real durable-append cost today: one full fsync per event, mirroring
    // what emit_domain_event forces on every producer call.
    group.throughput(Throughput::Elements(1));
    group.bench_function("append_flush_per_event", |b| {
        let (_dir, store) = temp_store();
        let mut i = 0usize;
        b.iter(|| {
            store
                .append_envelope(bench_envelope("session-0", i))
                .unwrap();
            store.flush().unwrap();
            i += 1;
        });
    });

    // Group-commit headroom: amortize one fsync across 100 appends.
    group.throughput(Throughput::Elements(100));
    group.bench_function("append_flush_per_100", |b| {
        let (_dir, store) = temp_store();
        let mut i = 0usize;
        b.iter(|| {
            for _ in 0..100 {
                store
                    .append_envelope(bench_envelope("session-0", i))
                    .unwrap();
                i += 1;
            }
            store.flush().unwrap();
        });
    });

    // The real producer path: enqueue -> drain -> flush per event.
    group.throughput(Throughput::Elements(1));
    group.bench_function("emit_domain_event", |b| {
        let _dir = TempDir::new().unwrap();
        let db = sled::open(_dir.path()).unwrap();
        let runtime = EventRuntime::new(db).unwrap();
        let mut i = 0usize;
        b.iter(|| {
            runtime
                .emit_domain_event(
                    "session-0",
                    "execution",
                    "session-0",
                    "execution.task.progress",
                    None,
                    json!({ "i": i }),
                )
                .unwrap();
            i += 1;
        });
    });

    // Multi-producer contention on a shared store: sequence allocation is
    // transactional, so the numbers capture serialization plus fsync cost
    // under contention.
    const MULTI_TOTAL: usize = 1_000;
    group.throughput(Throughput::Elements(MULTI_TOTAL as u64));
    for threads in [2usize, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("multi_producer", threads),
            &threads,
            |b, &threads| {
                b.iter_batched(
                    temp_store,
                    |(dir, store)| {
                        let store = Arc::new(store);
                        let per_thread = MULTI_TOTAL / threads;
                        let handles: Vec<_> = (0..threads)
                            .map(|t| {
                                let store = Arc::clone(&store);
                                std::thread::spawn(move || {
                                    let session = format!("session-{t}");
                                    for i in 0..per_thread {
                                        store.append_envelope(bench_envelope(&session, i)).unwrap();
                                    }
                                })
                            })
                            .collect();
                        for handle in handles {
                            handle.join().unwrap();
                        }
                        store.flush().unwrap();
                        // Returned so tempdir and store teardown stay outside
                        // the timed routine.
                        (dir, store)
                    },
                    BatchSize::PerIteration,
                );
            },
        );
    }

    // Durable producers through the single-writer ingress: every append
    // waits for its fsynced ack, but concurrent producers share group
    // commits, so throughput should rise with threads instead of degrading.
    for threads in [2usize, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("writer_durable_multi", threads),
            &threads,
            |b, &threads| {
                b.iter_batched(
                    || {
                        let (dir, store) = temp_store();
                        let writer = SpineWriter::spawn(Arc::new(store));
                        (dir, Arc::new(writer))
                    },
                    |(dir, writer)| {
                        let per_thread = MULTI_TOTAL / threads;
                        let handles: Vec<_> = (0..threads)
                            .map(|t| {
                                let writer = Arc::clone(&writer);
                                std::thread::spawn(move || {
                                    let session = format!("session-{t}");
                                    for i in 0..per_thread {
                                        writer
                                            .append_durable(bench_envelope(&session, i), false)
                                            .unwrap();
                                    }
                                })
                            })
                            .collect();
                        for handle in handles {
                            handle.join().unwrap();
                        }
                        (dir, writer)
                    },
                    BatchSize::PerIteration,
                );
            },
        );
    }
    group.finish();
}

fn replay_vs_history(c: &mut Criterion, fixtures: &[HistoryFixture]) {
    let mut group = c.benchmark_group("replay_vs_history");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));

    for fixture in fixtures {
        let size = fixture.size;
        // Cursor parked at tip returns zero events; reads seek to the
        // cursor key, so this must stay flat as total history grows.
        let tip = size as u64;
        group.bench_with_input(
            BenchmarkId::new("tip_zero_events", size),
            &tip,
            |b, &tip| {
                b.iter(|| black_box(fixture.store.read_all_events_after(tip).unwrap()));
            },
        );

        // Cursor at 90% returns the last 10% of history.
        let cursor = (size as u64 / 10) * 9;
        group.bench_with_input(
            BenchmarkId::new("tail_10_percent", size),
            &cursor,
            |b, &cursor| {
                b.iter(|| black_box(fixture.store.read_all_events_after(cursor).unwrap()));
            },
        );
    }
    group.finish();
}

fn session_reads(c: &mut Criterion) {
    let (_dir, store) = temp_store();
    populate(&store, 10, 10_000);
    let tip = 10_000u64;

    let mut group = c.benchmark_group("session_reads");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));

    group.bench_function("session_after_tip", |b| {
        b.iter(|| black_box(store.read_events_after("session-3", tip).unwrap()));
    });
    group.bench_function("session_from_zero", |b| {
        b.iter(|| black_box(store.read_events_after("session-3", 0).unwrap()));
    });
    group.finish();
}

fn idle_catch_up(c: &mut Criterion, fixtures: &[HistoryFixture]) {
    let mut group = c.benchmark_group("idle_catch_up");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(2));

    for fixture in fixtures {
        // Supervisor idle tick: limit 256 with the cursor at tip returns
        // nothing and must stay flat as history grows.
        let tip = fixture.size as u64;
        group.bench_with_input(
            BenchmarkId::new("limit_256_at_tip", fixture.size),
            &tip,
            |b, &tip| {
                b.iter(|| black_box(fixture.store.read_all_events_after_limit(tip, 256).unwrap()));
            },
        );
    }
    group.finish();
}

// Not a timing bench: reports on-disk bytes per event for a 10k-event store.
// sled's size_on_disk reports allocated segments, so treat this as an upper
// bound on storage amplification rather than exact payload size.
fn bytes_per_event(c: &mut Criterion) {
    const EVENTS: usize = 10_000;
    let (_dir, store) = temp_store();
    populate(&store, 4, EVENTS);
    let bytes = store.db().size_on_disk().expect("size_on_disk");
    eprintln!(
        "bytes_per_event: {} bytes on disk / {} events = {:.1} bytes/event",
        bytes,
        EVENTS,
        bytes as f64 / EVENTS as f64
    );

    let mut group = c.benchmark_group("bytes_per_event");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(1));
    group.bench_function("size_on_disk", |b| {
        b.iter(|| black_box(store.db().size_on_disk().unwrap()));
    });
    group.finish();
}

fn spine_benches(c: &mut Criterion) {
    append_throughput(c);
    let fixtures = build_history_fixtures();
    replay_vs_history(c, &fixtures);
    session_reads(c);
    idle_catch_up(c, &fixtures);
    bytes_per_event(c);
}

criterion_group!(benches, spine_benches);
criterion_main!(benches);
