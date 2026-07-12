//! Crash-recovery harness for the event ledger.
//!
//! Protocol: each parent test re-invokes this test binary as a writer child
//! (`child_writer_entrypoint`, selected via `--exact` and gated on
//! `MELD_EVENT_RECOVERY_ROLE`), lets it append events into a shared sled
//! directory, SIGKILLs it mid-write, reaps it, then reopens the database
//! in-process and asserts recovery invariants. The reopen must happen only
//! after the child is reaped so the sled directory lock is released.
//!
//! Set `MELD_EVENT_RECOVERY_SKIP` to skip the kill-cycle tests.

#![cfg(feature = "test-support")]

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use std::sync::Arc;

use meld_events::events::test_support::{
    EventStore, EventStoreTestSupport as _, EventWriter, EventWriterTestSupport as _,
};
use meld_events::{EventEnvelope, EventRecord};
use serde_json::json;

const SESSION_A: &str = "session-a";
const SESSION_B: &str = "session-b";
const RECORDED_AT: &str = "2026-04-26T16:00:00Z";

const SKIP_ENV: &str = "MELD_EVENT_RECOVERY_SKIP";
const ROLE_ENV: &str = "MELD_EVENT_RECOVERY_ROLE";
const DB_ENV: &str = "MELD_EVENT_RECOVERY_DB";
const CYCLE_ENV: &str = "MELD_EVENT_RECOVERY_CYCLE";
const STARTED_ENV: &str = "MELD_EVENT_RECOVERY_STARTED";
const WATERMARK_ENV: &str = "MELD_EVENT_RECOVERY_WATERMARK";

const ROLE_WRITER_FLUSH: &str = "writer-flush";
const ROLE_WRITER_NO_FLUSH: &str = "writer-no-flush";
const ROLE_WRITER_DURABLE: &str = "writer-durable";

const KILL_CYCLES: u64 = 3;
const FLUSH_INTERVAL: u64 = 8;
const FLUSH_BARRIERS_BEFORE_KILL: u64 = 4;
const CHILD_KILL_POINT_TIMEOUT: Duration = Duration::from_secs(20);
const NO_FLUSH_WRITE_WINDOW: Duration = Duration::from_millis(350);
const APPEND_THROTTLE: Duration = Duration::from_micros(200);

fn skip_requested() -> bool {
    std::env::var_os(SKIP_ENV).is_some()
}

fn recovery_envelope(session: &str, cycle: u64, counter: u64) -> EventEnvelope {
    EventEnvelope::new_domain(
        RECORDED_AT.to_string(),
        session,
        "execution",
        "workflow-recovery",
        "execution.recovery.tick",
        None,
        json!({ "cycle": cycle, "counter": counter }),
    )
}

/// Writer child entrypoint; a no-op unless spawned by a parent test with the
/// role env var set. Appends a mix of plain and idempotent envelopes until the
/// parent SIGKILLs the process.
#[test]
fn child_writer_entrypoint() {
    let Ok(role) = std::env::var(ROLE_ENV) else {
        return;
    };
    let db_path = std::env::var(DB_ENV).unwrap();
    let cycle: u64 = std::env::var(CYCLE_ENV).unwrap().parse().unwrap();
    let started_path = PathBuf::from(std::env::var(STARTED_ENV).unwrap());
    let watermark_path = std::env::var(WATERMARK_ENV).ok().map(PathBuf::from);

    // The flush role must reach durable storage only through explicit flush
    // barriers; the no-flush role relies on sled's background flusher alone;
    // the durable role goes through the ledger writer and trusts its acks.
    let flush_every_ms = match role.as_str() {
        ROLE_WRITER_FLUSH | ROLE_WRITER_DURABLE => None,
        ROLE_WRITER_NO_FLUSH => Some(100),
        other => panic!("unknown writer role {other}"),
    };
    let store = EventStore::new(open_db_with_retry(&db_path, flush_every_ms)).unwrap();
    let event_writer =
        (role == ROLE_WRITER_DURABLE).then(|| EventWriter::spawn(Arc::new(store.clone())));

    let mut run_start: Option<u64> = None;
    let mut max_appended = 0u64;
    let mut counter = 0u64;
    loop {
        counter += 1;
        let session = if counter.is_multiple_of(2) {
            SESSION_B
        } else {
            SESSION_A
        };
        let envelope = recovery_envelope(session, cycle, counter);
        // Record ids repeat within a cycle so the idempotent-hit path is
        // exercised while the child is being killed.
        let idempotent = counter.is_multiple_of(3);
        let envelope = if idempotent {
            envelope.with_record_id(format!("cycle-{cycle}-record-{}", counter % 5))
        } else {
            envelope
        };
        let seq = match &event_writer {
            Some(writer) => writer.append_durable(envelope, idempotent).unwrap(),
            None if idempotent => store.append_envelope_idempotent(envelope).unwrap(),
            None => store.append_envelope(envelope).unwrap(),
        };
        if run_start.is_none() {
            run_start = Some(seq);
            File::create(&started_path).unwrap();
        }
        max_appended = max_appended.max(seq);
        if role == ROLE_WRITER_FLUSH && counter.is_multiple_of(FLUSH_INTERVAL) {
            // Sequencing: the watermark may only become visible after the
            // flush barrier that made [run_start, max_appended] durable.
            store.flush().unwrap();
            write_watermark(
                watermark_path.as_deref().unwrap(),
                run_start.unwrap(),
                max_appended,
            );
        }
        if role == ROLE_WRITER_DURABLE {
            // The durable ack is the barrier: every acked sequence must
            // survive the kill, so the watermark records acks directly.
            write_watermark(
                watermark_path.as_deref().unwrap(),
                run_start.unwrap(),
                max_appended,
            );
        }
        std::thread::sleep(APPEND_THROTTLE);
    }
}

fn open_db_with_retry(path: &str, flush_every_ms: Option<u64>) -> sled::Db {
    // Sequencing: the parent may still be dropping its validation handle from
    // the previous cycle when the child starts, so retry the directory lock.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match sled::Config::new()
            .path(path)
            .flush_every_ms(flush_every_ms)
            .open()
        {
            Ok(db) => return db,
            Err(err) => {
                assert!(
                    Instant::now() < deadline,
                    "child failed to open ledger db: {err}"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

fn write_watermark(path: &Path, run_start: u64, flushed_max: u64) {
    // The watermark file is fsynced then renamed so the parent never observes
    // a torn watermark, even if the kill lands mid-write.
    let staging = path.with_extension("tmp");
    let mut file = File::create(&staging).unwrap();
    write!(file, "{run_start} {flushed_max}").unwrap();
    file.sync_all().unwrap();
    std::fs::rename(&staging, path).unwrap();
}

fn read_watermark(path: &Path) -> Option<(u64, u64)> {
    let raw = std::fs::read_to_string(path).ok()?;
    let mut parts = raw.split_whitespace();
    let run_start = parts.next()?.parse().ok()?;
    let flushed_max = parts.next()?.parse().ok()?;
    Some((run_start, flushed_max))
}

fn spawn_writer(
    role: &str,
    db_path: &Path,
    cycle: u64,
    started_path: &Path,
    watermark_path: Option<&Path>,
) -> Child {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg("child_writer_entrypoint")
        .arg("--exact")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(ROLE_ENV, role)
        .env(DB_ENV, db_path)
        .env(CYCLE_ENV, cycle.to_string())
        .env(STARTED_ENV, started_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if let Some(watermark_path) = watermark_path {
        command.env(WATERMARK_ENV, watermark_path);
    }
    command.spawn().unwrap()
}

fn wait_until(child: &mut Child, mut ready: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + CHILD_KILL_POINT_TIMEOUT;
    while Instant::now() < deadline {
        if ready() {
            return true;
        }
        if child.try_wait().unwrap().is_some() {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

fn fail_with_child_stderr(mut child: Child) -> ! {
    let _ = child.kill();
    let output = child.wait_with_output().unwrap();
    panic!(
        "writer child never reached the kill point: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn kill_at_crash_point(
    mut child: Child,
    started_path: &Path,
    watermark_path: Option<&Path>,
) -> Option<(u64, u64)> {
    let ready = match watermark_path {
        Some(watermark_path) => wait_until(&mut child, || {
            read_watermark(watermark_path).is_some_and(|(run_start, flushed_max)| {
                flushed_max >= run_start + FLUSH_INTERVAL * FLUSH_BARRIERS_BEFORE_KILL - 1
            })
        }),
        None => {
            let started = wait_until(&mut child, || started_path.exists());
            if started {
                std::thread::sleep(NO_FLUSH_WRITE_WINDOW);
            }
            started
        }
    };
    if !ready {
        fail_with_child_stderr(child);
    }
    child.kill().unwrap();
    // Sequencing: reap before reopening so the sled directory lock held by the
    // killed child is fully released.
    child.wait().unwrap();
    watermark_path.and_then(read_watermark)
}

fn run_kill_reopen_cycles(role: &str, mut check: impl FnMut(u64, &EventStore, Option<(u64, u64)>)) {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("events");
    for cycle in 1..=KILL_CYCLES {
        let started_path = temp_dir.path().join(format!("started-{cycle}"));
        let watermark_path = (role == ROLE_WRITER_FLUSH || role == ROLE_WRITER_DURABLE)
            .then(|| temp_dir.path().join(format!("watermark-{cycle}")));
        let child = spawn_writer(
            role,
            &db_path,
            cycle,
            &started_path,
            watermark_path.as_deref(),
        );
        let watermark = kill_at_crash_point(child, &started_path, watermark_path.as_deref());
        let store = EventStore::new(sled::open(&db_path).unwrap()).unwrap();
        check(cycle, &store, watermark);
        // The validation store drops here so the next cycle's child can take
        // the sled directory lock.
    }
}

fn persisted_events(store: &EventStore) -> Vec<EventRecord> {
    store.read_all_events_after(0).unwrap()
}

fn assert_unique_sorted_sequences(cycle: u64, events: &[EventRecord]) {
    // Invariant: persisted ledger sequences are unique after crash recovery.
    for pair in events.windows(2) {
        assert!(
            pair[0].seq < pair[1].seq,
            "cycle {cycle}: duplicate or unordered persisted seq {} then {}",
            pair[0].seq,
            pair[1].seq
        );
    }
}

fn assert_sequence_meta_ahead(cycle: u64, store: &EventStore, events: &[EventRecord]) {
    let max_seq = events.last().map(|event| event.seq).unwrap_or(0);
    // Invariant: sequence metadata never lags the greatest persisted sequence,
    // so a reopened store can never assign an already-used sequence. Probed
    // through an appended event because appends are the only allocation path.
    let probe = EventEnvelope::new_domain(
        String::new(),
        "recovery-probe",
        "event_recovery",
        "recovery-probe",
        "ledger.recovery.probe",
        None,
        serde_json::json!({ "cycle": cycle }),
    );
    let probe_seq = store.append_envelope(probe).unwrap();
    assert!(
        probe_seq > max_seq,
        "cycle {cycle}: probe append received {probe_seq} but max persisted seq is {max_seq}"
    );
}

#[test]
fn killed_flush_writer_records_decode_with_unique_sequences() {
    if skip_requested() {
        return;
    }
    run_kill_reopen_cycles(ROLE_WRITER_FLUSH, |cycle, store, _| {
        // Invariant: every persisted ledger value decodes as an EventRecord.
        let events = persisted_events(store);
        assert!(
            !events.is_empty(),
            "cycle {cycle}: no events survived despite flush barriers"
        );
        assert_unique_sorted_sequences(cycle, &events);
    });
}

// The no-flush role relies on sled's background flusher, so a cycle may
// legitimately persist zero events and pass vacuously; coverage of a
// guaranteed-durable prefix lives in the flush-role tests.
#[test]
fn killed_no_flush_writer_records_decode_with_unique_sequences() {
    if skip_requested() {
        return;
    }
    run_kill_reopen_cycles(ROLE_WRITER_NO_FLUSH, |cycle, store, _| {
        let events = persisted_events(store);
        assert_unique_sorted_sequences(cycle, &events);
    });
}

#[test]
fn sequence_meta_stays_ahead_after_flush_writer_kill() {
    if skip_requested() {
        return;
    }
    run_kill_reopen_cycles(ROLE_WRITER_FLUSH, |cycle, store, _| {
        let events = persisted_events(store);
        assert_sequence_meta_ahead(cycle, store, &events);
    });
}

#[test]
fn sequence_meta_stays_ahead_after_no_flush_writer_kill() {
    if skip_requested() {
        return;
    }
    run_kill_reopen_cycles(ROLE_WRITER_NO_FLUSH, |cycle, store, _| {
        let events = persisted_events(store);
        assert_sequence_meta_ahead(cycle, store, &events);
    });
}

#[test]
fn idempotency_index_matches_persisted_records_after_kill() {
    if skip_requested() {
        return;
    }
    run_kill_reopen_cycles(ROLE_WRITER_FLUSH, |cycle, store, _| {
        let events = persisted_events(store);
        let mut seq_by_record_id: BTreeMap<String, u64> = BTreeMap::new();
        for event in &events {
            let Some(record_id) = event.record_id.clone() else {
                continue;
            };
            // Invariant: at most one persisted record per idempotency key.
            let previous = seq_by_record_id.insert(record_id.clone(), event.seq);
            assert!(
                previous.is_none(),
                "cycle {cycle}: record_id {record_id} persisted at seqs {:?} and {}",
                previous,
                event.seq
            );
        }
        assert!(
            !seq_by_record_id.is_empty(),
            "cycle {cycle}: no idempotent records survived despite flush barriers"
        );

        // Invariant: replaying a persisted record_id returns the existing
        // sequence and never grows the ledger.
        for (record_id, seq) in &seq_by_record_id {
            let replayed = store
                .append_envelope_idempotent(
                    recovery_envelope(SESSION_A, cycle, 0).with_record_id(record_id.clone()),
                )
                .unwrap();
            assert_eq!(
                replayed, *seq,
                "cycle {cycle}: replay of record_id {record_id} returned {replayed}, expected {seq}"
            );
        }
        assert_eq!(
            persisted_events(store).len(),
            events.len(),
            "cycle {cycle}: idempotent replays grew the persisted ledger"
        );
    });
}

#[test]
fn session_reads_match_global_ledger_after_kill() {
    if skip_requested() {
        return;
    }
    run_kill_reopen_cycles(ROLE_WRITER_FLUSH, |cycle, store, _| {
        let events = persisted_events(store);
        let by_seq: BTreeMap<u64, &EventRecord> =
            events.iter().map(|event| (event.seq, event)).collect();
        let mut session_total = 0;
        for session in [SESSION_A, SESSION_B] {
            let session_events = store.read_events(session).unwrap();
            session_total += session_events.len();
            for event in &session_events {
                assert_eq!(
                    event.session, session,
                    "cycle {cycle}: session read returned foreign record seq {}",
                    event.seq
                );
                // Invariant: session reads never surface phantom records that
                // are missing from (or differ from) the global ledger.
                let global = by_seq.get(&event.seq).unwrap_or_else(|| {
                    panic!(
                        "cycle {cycle}: session {session} seq {} not in global ledger",
                        event.seq
                    )
                });
                assert_eq!(
                    *global, event,
                    "cycle {cycle}: session {session} seq {} diverges from global ledger",
                    event.seq
                );
            }
        }
        // Invariant: every persisted record is reachable through exactly one
        // of the writer's sessions.
        assert_eq!(
            session_total,
            events.len(),
            "cycle {cycle}: session reads do not partition the global ledger"
        );
    });
}

#[test]
fn flushed_prefix_survives_kill() {
    if skip_requested() {
        return;
    }
    run_kill_reopen_cycles(ROLE_WRITER_FLUSH, |cycle, store, watermark| {
        let (run_start, flushed_max) = watermark.expect("flush writer must publish a watermark");
        let persisted: BTreeSet<u64> = persisted_events(store)
            .iter()
            .map(|event| event.seq)
            .collect();
        // Invariant: every event appended before the child's last flush
        // barrier is durable across the kill.
        for seq in run_start..=flushed_max {
            assert!(
                persisted.contains(&seq),
                "cycle {cycle}: seq {seq} inside flushed prefix [{run_start}, {flushed_max}] lost"
            );
        }
    });
}

#[test]
fn acked_durable_appends_survive_kill() {
    if skip_requested() {
        return;
    }
    run_kill_reopen_cycles(ROLE_WRITER_DURABLE, |cycle, store, watermark| {
        let (run_start, acked_max) = watermark.expect("durable writer must publish acked seqs");
        let persisted: BTreeSet<u64> = persisted_events(store)
            .iter()
            .map(|event| event.seq)
            .collect();
        // Invariant: a durable ack means durable bytes, so every sequence
        // the ledger writer acked before the kill must survive reopen.
        for seq in run_start..=acked_max {
            assert!(
                persisted.contains(&seq),
                "cycle {cycle}: acked seq {seq} in [{run_start}, {acked_max}] lost after kill"
            );
        }
    });
}
