use std::path::Path;
use std::time::{Duration, Instant};

use meld_events::error::EventAuthorityError;
use meld_events::{
    AppendDisposition, AppendMode, EventAuthority, EventAuthorityOpenOptions, EventEnvelope,
    EventIngressFenceState, LedgerCursor, ReplayRequest,
};
use serde_json::json;

fn reopen_sled(path: &Path) -> sled::Db {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match sled::open(path) {
            Ok(db) => return db,
            Err(error)
                if error.to_string().contains("could not acquire lock")
                    && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("cannot reopen event database: {error}"),
        }
    }
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
    let mut ledger_id = None;

    for cycle in 1..=6 {
        let authority = EventAuthority::open(
            reopen_sled(&path),
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
