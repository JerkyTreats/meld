use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use meld_events::error::EventAuthorityError;
use meld_events::{
    AppendDisposition, AppendMode, EventAuthority, EventAuthorityOpenOptions, EventEnvelope,
    EventIngressFenceState, LedgerCursor, ReplayRequest,
};
use serde_json::json;

const CHILD_ROLE: &str = "MELD_W1B5_EVENT_CHILD_ROLE";
const CHILD_DB_PATH: &str = "MELD_W1B5_EVENT_DB_PATH";
const CHILD_MARKER_PATH: &str = "MELD_W1B5_EVENT_MARKER_PATH";

fn write_marker(path: &Path, value: &str) {
    let mut marker = File::create(path).unwrap();
    marker.write_all(value.as_bytes()).unwrap();
    marker.sync_all().unwrap();
}

fn run_event_boundary_child() -> bool {
    let Ok(role) = std::env::var(CHILD_ROLE) else {
        return false;
    };
    let db_path = std::env::var_os(CHILD_DB_PATH).unwrap();
    let marker_path = std::env::var_os(CHILD_MARKER_PATH).unwrap();
    let authority = EventAuthority::open(
        sled::open(db_path).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let append = authority.append_capability();
    match role.as_str() {
        "before_durable_barrier" => {
            append
                .append_best_effort(envelope(1), AppendMode::Idempotent)
                .unwrap();
            write_marker(Path::new(&marker_path), "before_durable_barrier");
        }
        "after_durable_ack" => {
            append
                .append_durable(envelope(1), AppendMode::Idempotent)
                .unwrap();
            write_marker(Path::new(&marker_path), "after_durable_ack");
        }
        "after_final_barrier" => {
            append
                .append_durable(envelope(1), AppendMode::Idempotent)
                .unwrap();
            append.close_and_drain().unwrap();
            write_marker(Path::new(&marker_path), "after_final_barrier");
        }
        other => panic!("unknown event child role {other}"),
    }
    std::process::abort();
}

fn run_aborting_child(test_name: &str, role: &str, db_path: &Path, marker_path: &Path) {
    let mut child = ProcessCommand::new(std::env::current_exe().unwrap())
        .arg(test_name)
        .arg("--exact")
        .current_dir(marker_path.parent().unwrap())
        .env(CHILD_ROLE, role)
        .env(CHILD_DB_PATH, db_path)
        .env(CHILD_MARKER_PATH, marker_path)
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("event boundary child exceeded its deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(!status.success());
    assert_eq!(std::fs::read_to_string(marker_path).unwrap(), role);
}

fn envelope(cycle: u64) -> EventEnvelope {
    EventEnvelope::new_domain(
        "2026-07-12T00:00:00Z".to_string(),
        "wave1-harness",
        "execution",
        "wave1-harness",
        "execution.wave1_harness",
        None,
        json!({ "cycle": cycle }),
    )
    .with_record_id(format!("wave1-cycle-{cycle}"))
}

#[test]
fn ingress_fence_rejects_late_writes_and_final_barrier_retries_across_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("events");
    let db = sled::open(&path).unwrap();
    let mut ledger_id = None;

    for cycle in 1..=6 {
        let authority = EventAuthority::open(
            db.clone(),
            EventAuthorityOpenOptions {
                expected_ledger_id: ledger_id,
            },
        )
        .unwrap();
        let identity = authority.ledger_identity();
        assert_eq!(ledger_id.get_or_insert(identity), &identity);
        let append = authority.append_capability();
        let contender = append.clone();
        let receipt = append
            .append_durable(envelope(cycle), AppendMode::Idempotent)
            .unwrap();
        assert_eq!(receipt.ledger_id, identity);
        assert_eq!(receipt.seq, cycle);
        assert_eq!(receipt.disposition, AppendDisposition::Inserted);

        let barrier = append.close_and_drain().unwrap();
        assert_eq!(barrier.fence().state, EventIngressFenceState::Closed);
        assert_eq!(barrier.watermark().committed_seq, cycle);
        assert_eq!(barrier.watermark().tip_seq, cycle);
        assert_eq!(append.close_and_drain().unwrap(), barrier);
        assert!(matches!(
            contender.append_durable(envelope(cycle + 100), AppendMode::Idempotent),
            Err(EventAuthorityError::Unavailable { .. })
        ));

        let page = authority
            .replay_capability()
            .replay(ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id: identity,
                    after_seq: 0,
                },
                limit: 16,
            })
            .unwrap();
        assert_eq!(
            page.records
                .iter()
                .map(|record| record.seq)
                .collect::<Vec<_>>(),
            (1..=cycle).collect::<Vec<_>>()
        );
        assert_eq!(page.next_cursor.after_seq, cycle);
    }

    assert!(ledger_id.is_some());
}

#[test]
fn abrupt_append_recovery_distinguishes_pre_and_post_durable_ack() {
    if run_event_boundary_child() {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let before_db = temp.path().join("before-events");
    let before_marker = temp.path().join("before.marker");
    run_aborting_child(
        "abrupt_append_recovery_distinguishes_pre_and_post_durable_ack",
        "before_durable_barrier",
        &before_db,
        &before_marker,
    );
    let before = EventAuthority::open(
        sled::open(&before_db).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let receipt = before
        .append_capability()
        .append_durable(envelope(1), AppendMode::Idempotent)
        .unwrap();
    assert_eq!(receipt.seq, 1);
    assert!(matches!(
        receipt.disposition,
        AppendDisposition::Inserted | AppendDisposition::Duplicate
    ));
    let barrier = before.append_capability().close_and_drain().unwrap();
    assert_eq!(barrier.watermark().tip_seq, 1);
    assert_eq!(
        before
            .replay_capability()
            .replay(ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id: before.ledger_identity(),
                    after_seq: 0,
                },
                limit: 2,
            })
            .unwrap()
            .records
            .len(),
        1
    );
    drop(before);

    let after_db = temp.path().join("after-events");
    let after_marker = temp.path().join("after.marker");
    run_aborting_child(
        "abrupt_append_recovery_distinguishes_pre_and_post_durable_ack",
        "after_durable_ack",
        &after_db,
        &after_marker,
    );
    let after = EventAuthority::open(
        sled::open(&after_db).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let receipt = after
        .append_capability()
        .append_durable(envelope(1), AppendMode::Idempotent)
        .unwrap();
    assert_eq!(receipt.seq, 1);
    assert_eq!(receipt.disposition, AppendDisposition::Duplicate);
    assert_eq!(
        after
            .append_capability()
            .close_and_drain()
            .unwrap()
            .watermark()
            .tip_seq,
        1
    );
}

#[test]
fn concurrent_append_and_drain_admission_is_linearizable() {
    const APPENDERS: usize = 12;
    for cycle in 0..12 {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let authority = EventAuthority::open(db, EventAuthorityOpenOptions::default()).unwrap();
        let start = Arc::new(Barrier::new(APPENDERS + 2));
        let writers = (0..APPENDERS)
            .map(|index| {
                let append = authority.append_capability();
                let start = Arc::clone(&start);
                std::thread::spawn(move || {
                    start.wait();
                    append.append_durable(
                        envelope(cycle * 100 + index as u64 + 1),
                        AppendMode::Idempotent,
                    )
                })
            })
            .collect::<Vec<_>>();
        let drain = authority.append_capability();
        let drain_start = Arc::clone(&start);
        let drain = std::thread::spawn(move || {
            drain_start.wait();
            drain.close_and_drain().unwrap()
        });
        start.wait();
        let results = writers
            .into_iter()
            .map(|writer| writer.join().unwrap())
            .collect::<Vec<_>>();
        let barrier = drain.join().unwrap();
        let mut accepted = Vec::new();
        for result in results {
            match result {
                Ok(receipt) => accepted.push(receipt.seq),
                Err(error) => assert!(matches!(error, EventAuthorityError::Unavailable { .. })),
            }
        }
        accepted.sort_unstable();
        assert_eq!(accepted, (1..=accepted.len() as u64).collect::<Vec<_>>());
        assert_eq!(barrier.watermark().committed_seq, accepted.len() as u64);
        assert_eq!(barrier.watermark().tip_seq, accepted.len() as u64);
        assert_eq!(
            authority.append_capability().close_and_drain().unwrap(),
            barrier
        );
        assert!(matches!(
            authority
                .append_capability()
                .append_durable(envelope(10_000 + cycle), AppendMode::Idempotent),
            Err(EventAuthorityError::Unavailable { .. })
        ));
    }
}

#[test]
fn abrupt_final_barrier_reopens_with_recovered_tip_and_fresh_ingress() {
    if run_event_boundary_child() {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("barrier-events");
    let marker_path = temp.path().join("barrier.marker");
    run_aborting_child(
        "abrupt_final_barrier_reopens_with_recovered_tip_and_fresh_ingress",
        "after_final_barrier",
        &db_path,
        &marker_path,
    );

    let authority = EventAuthority::open(
        sled::open(&db_path).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    assert_eq!(
        authority.append_capability().ingress_fence().state,
        EventIngressFenceState::Open
    );
    let recovered = authority.watermark_capability().snapshot().unwrap();
    assert_eq!(recovered.committed_seq, 1);
    assert_eq!(recovered.tip_seq, 1);
    let second = authority
        .append_capability()
        .append_durable(envelope(2), AppendMode::Idempotent)
        .unwrap();
    assert_eq!(second.seq, 2);
    let barrier = authority.append_capability().close_and_drain().unwrap();
    assert_eq!(barrier.watermark().committed_seq, 2);
    assert_eq!(barrier.watermark().tip_seq, 2);
    assert_eq!(
        authority.append_capability().close_and_drain().unwrap(),
        barrier
    );
    assert!(matches!(
        authority
            .append_capability()
            .append_durable(envelope(3), AppendMode::Idempotent),
        Err(EventAuthorityError::Unavailable { .. })
    ));
}
