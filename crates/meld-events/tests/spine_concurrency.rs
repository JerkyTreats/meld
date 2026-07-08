use meld_events::events::store::EventStore;
use meld_events::{EventEnvelope, EventRuntime};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Barrier};
use std::thread;

const RECORDED_AT: &str = "2026-04-26T16:00:00Z";
const STORM_THREADS: usize = 8;
const STORM_APPENDS: usize = 250;

fn shared_store() -> (tempfile::TempDir, Arc<EventStore>) {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("events")).unwrap();
    (temp_dir, EventStore::shared(db).unwrap())
}

fn event_runtime() -> (tempfile::TempDir, EventRuntime) {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("events")).unwrap();
    (temp_dir, EventRuntime::new(db).unwrap())
}

fn storm_envelope(thread_index: usize, append_index: usize) -> EventEnvelope {
    EventEnvelope::new_domain(
        RECORDED_AT.to_string(),
        format!("session-{thread_index}"),
        "execution",
        format!("workflow-{thread_index}"),
        "execution.storm.append",
        None,
        json!({ "thread": thread_index, "append": append_index }),
    )
}

fn run_storm(
    threads: usize,
    appends: usize,
    append: impl Fn(usize, usize) -> u64 + Sync,
) -> Vec<u64> {
    let barrier = Barrier::new(threads);
    thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|thread_index| {
                let barrier = &barrier;
                let append = &append;
                scope.spawn(move || {
                    barrier.wait();
                    (0..appends)
                        .map(|append_index| append(thread_index, append_index))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("storm thread panicked"))
            .collect()
    })
}

fn assert_storm_kept_every_event(store: &EventStore, returned_seqs: &[u64], expected_total: usize) {
    let records = store.read_all_events_after(0).unwrap();
    assert_eq!(
        records.len(),
        expected_total,
        "store lost {} of {expected_total} concurrent appends",
        expected_total - records.len()
    );

    let distinct: BTreeSet<u64> = returned_seqs.iter().copied().collect();
    assert_eq!(
        distinct.len(),
        expected_total,
        "concurrent appenders were handed duplicate sequences"
    );

    let expected: Vec<u64> = (1..=expected_total as u64).collect();
    let mut returned = returned_seqs.to_vec();
    returned.sort_unstable();
    assert_eq!(returned, expected);
    assert_eq!(
        records.iter().map(|record| record.seq).collect::<Vec<_>>(),
        expected
    );
}

#[test]
#[ignore = "red: non-atomic sequence allocation loses concurrent appends; fixed by correctness core phase"]
fn concurrent_append_envelope_storm_keeps_all_events_with_gapless_sequences() {
    let (_temp_dir, store) = shared_store();

    let seqs = run_storm(
        STORM_THREADS,
        STORM_APPENDS,
        |thread_index, append_index| {
            store
                .append_envelope(storm_envelope(thread_index, append_index))
                .unwrap()
        },
    );

    assert_storm_kept_every_event(&store, &seqs, STORM_THREADS * STORM_APPENDS);
}

#[test]
#[ignore = "red: non-atomic sequence allocation loses concurrent appends; fixed by correctness core phase"]
fn concurrent_idempotent_storm_with_unique_record_ids_keeps_all_events() {
    let (_temp_dir, store) = shared_store();

    let seqs = run_storm(
        STORM_THREADS,
        STORM_APPENDS,
        |thread_index, append_index| {
            let envelope = storm_envelope(thread_index, append_index)
                .with_record_id(format!("record-{thread_index}-{append_index}"));
            store.append_envelope_idempotent(envelope).unwrap()
        },
    );

    assert_storm_kept_every_event(&store, &seqs, STORM_THREADS * STORM_APPENDS);

    // Index consistency: replaying any record_id must return the seq of a
    // stored record that still carries that record_id.
    let record_id_by_seq: BTreeMap<u64, String> = store
        .read_all_events_after(0)
        .unwrap()
        .iter()
        .map(|record| (record.seq, record.record_id.clone().unwrap()))
        .collect();
    for thread_index in 0..STORM_THREADS {
        for append_index in 0..STORM_APPENDS {
            let record_id = format!("record-{thread_index}-{append_index}");
            let replay_seq = store
                .append_envelope_idempotent(
                    storm_envelope(thread_index, append_index).with_record_id(record_id.clone()),
                )
                .unwrap();
            assert_eq!(record_id_by_seq.get(&replay_seq), Some(&record_id));
        }
    }
}

#[test]
fn concurrent_duplicate_record_id_storm_settles_to_single_agreed_record() {
    let (_temp_dir, store) = shared_store();

    let seqs = run_storm(STORM_THREADS, 100, |thread_index, append_index| {
        let envelope = storm_envelope(thread_index, append_index).with_record_id("record-shared");
        store.append_envelope_idempotent(envelope).unwrap()
    });

    let records = store.read_all_events_after(0).unwrap();
    assert_eq!(
        records.len(),
        1,
        "duplicate record_id produced {} records",
        records.len()
    );
    assert_eq!(records[0].record_id.as_deref(), Some("record-shared"));

    let winning_seq = records[0].seq;
    let distinct: BTreeSet<u64> = seqs.iter().copied().collect();
    assert_eq!(
        distinct,
        BTreeSet::from([winning_seq]),
        "callers disagreed on the settled seq"
    );
}

#[test]
#[ignore = "red: non-atomic sequence allocation loses concurrent appends; fixed by correctness core phase"]
fn mixed_runtime_and_idempotent_path_storm_keeps_all_events() {
    let (_temp_dir, runtime) = event_runtime();
    let domain_threads = 4;
    let idempotent_threads = 4;
    let appends = 200;
    let barrier = Barrier::new(domain_threads + idempotent_threads);

    thread::scope(|scope| {
        for thread_index in 0..domain_threads {
            let runtime = &runtime;
            let barrier = &barrier;
            scope.spawn(move || {
                barrier.wait();
                for append_index in 0..appends {
                    runtime
                        .emit_domain_event(
                            &format!("session-domain-{thread_index}"),
                            "execution",
                            &format!("workflow-domain-{thread_index}"),
                            "execution.storm.mixed",
                            None,
                            json!({ "thread": thread_index, "append": append_index }),
                        )
                        .unwrap();
                }
            });
        }
        for thread_index in 0..idempotent_threads {
            let runtime = &runtime;
            let barrier = &barrier;
            scope.spawn(move || {
                barrier.wait();
                for append_index in 0..appends {
                    runtime
                        .emit_envelope_idempotent(
                            storm_envelope(thread_index, append_index)
                                .with_record_id(format!("mixed-{thread_index}-{append_index}")),
                        )
                        .unwrap();
                }
            });
        }
    });

    let expected_total = (domain_threads + idempotent_threads) * appends;
    let records = runtime.store().read_all_events_after(0).unwrap();
    assert_eq!(
        records.len(),
        expected_total,
        "mixed-path storm lost {} of {expected_total} events",
        expected_total - records.len()
    );
    assert_eq!(
        records.iter().map(|record| record.seq).collect::<Vec<_>>(),
        (1..=expected_total as u64).collect::<Vec<_>>()
    );
}

#[test]
fn sequential_appends_produce_gapless_sequences() {
    let (_temp_dir, store) = shared_store();
    let total = 200usize;

    let seqs: Vec<u64> = (0..total)
        .map(|append_index| {
            store
                .append_envelope(storm_envelope(0, append_index))
                .unwrap()
        })
        .collect();

    assert_eq!(seqs, (1..=total as u64).collect::<Vec<_>>());
    assert_eq!(
        store
            .read_all_events_after(0)
            .unwrap()
            .iter()
            .map(|record| record.seq)
            .collect::<Vec<_>>(),
        seqs
    );
}
