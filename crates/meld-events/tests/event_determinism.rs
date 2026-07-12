#![cfg(feature = "test-support")]

use meld_events::events::test_support::{EventStore, EventStoreTestSupport as _};
use meld_events::{EventEnvelope, EventRecord};
use proptest::prelude::*;
use serde_json::json;

const RECORDED_AT: &str = "2026-04-26T16:00:00Z";
const SESSIONS: [&str; 4] = ["session-a", "session-b", "session-c", "session-d"];

fn event_store() -> (tempfile::TempDir, EventStore) {
    let temp_dir = tempfile::tempdir().unwrap();
    let db = sled::open(temp_dir.path().join("events")).unwrap();
    (temp_dir, EventStore::new(db).unwrap())
}

fn ledger_envelope(session: &str, marker: usize) -> EventEnvelope {
    EventEnvelope::new_domain(
        RECORDED_AT.to_string(),
        session,
        "execution",
        format!("workflow-{session}"),
        "execution.replay.event",
        None,
        json!({ "marker": marker }),
    )
}

fn populated_store(total: usize) -> (tempfile::TempDir, EventStore) {
    let (temp_dir, store) = event_store();
    for marker in 0..total {
        store
            .append_envelope(ledger_envelope(SESSIONS[marker % SESSIONS.len()], marker))
            .unwrap();
    }
    (temp_dir, store)
}

#[test]
fn read_all_events_after_replays_identically_for_every_cursor() {
    let total = 60usize;
    let (_temp_dir, store) = populated_store(total);

    let baseline = store.read_all_events_after(0).unwrap();
    assert_eq!(baseline.len(), total);

    for cursor in 0..=(total as u64 + 2) {
        let first = store.read_all_events_after(cursor).unwrap();
        let second = store.read_all_events_after(cursor).unwrap();
        assert_eq!(first, second);

        let expected: Vec<EventRecord> = baseline
            .iter()
            .filter(|record| record.seq > cursor)
            .cloned()
            .collect();
        assert_eq!(first, expected);
    }
}

#[test]
fn limited_windows_stitch_into_the_unbounded_read() {
    let total = 60usize;
    let (_temp_dir, store) = populated_store(total);

    for start in [0u64, 1, 7, total as u64 - 1, total as u64, total as u64 + 5] {
        let unbounded = store.read_all_events_after(start).unwrap();
        for limit in [1usize, 2, 7, 16, total] {
            let mut stitched = Vec::new();
            let mut cursor = start;
            loop {
                let window = store.read_all_events_after_limit(cursor, limit).unwrap();
                assert_eq!(
                    window,
                    store.read_all_events_after_limit(cursor, limit).unwrap()
                );
                assert!(window.len() <= limit);
                let Some(last) = window.last() else {
                    break;
                };
                cursor = last.seq;
                stitched.extend(window);
            }
            assert_eq!(stitched, unbounded);
        }
    }
}

#[test]
fn session_reads_match_session_filtered_global_reads() {
    let total = 48usize;
    let (_temp_dir, store) = populated_store(total);

    let global = store.read_all_events_after(0).unwrap();
    for session in SESSIONS {
        let expected_all: Vec<EventRecord> = global
            .iter()
            .filter(|record| record.session == session)
            .cloned()
            .collect();
        assert!(!expected_all.is_empty());
        assert_eq!(store.read_events(session).unwrap(), expected_all);

        for cursor in [0u64, 1, 5, 24, 47, 48, 60] {
            let expected: Vec<EventRecord> = global
                .iter()
                .filter(|record| record.session == session && record.seq > cursor)
                .cloned()
                .collect();
            assert_eq!(store.read_events_after(session, cursor).unwrap(), expected);
        }
    }
}

#[test]
fn cursor_consumer_never_skips_or_repeats_across_interleaved_appends() {
    let (_temp_dir, store) = event_store();
    let mut appended = 0u64;
    let mut cursor = 0u64;
    let mut consumed: Vec<u64> = Vec::new();

    for round in 0..30usize {
        for _ in 0..(round % 4) {
            appended += 1;
            store
                .append_envelope(ledger_envelope(
                    SESSIONS[round % SESSIONS.len()],
                    appended as usize,
                ))
                .unwrap();
        }

        let limit = round % 3 + 1;
        for record in store.read_all_events_after_limit(cursor, limit).unwrap() {
            assert!(
                record.seq > cursor,
                "consumer saw seq {} at or before cursor {cursor}",
                record.seq
            );
            cursor = record.seq;
            consumed.push(record.seq);
        }
    }

    for record in store.read_all_events_after(cursor).unwrap() {
        assert!(record.seq > cursor);
        cursor = record.seq;
        consumed.push(record.seq);
    }

    // Cursor-advancing consumption saw every appended seq exactly once.
    assert_eq!(consumed, (1..=appended).collect::<Vec<_>>());
}

#[derive(Clone, Debug)]
struct GeneratedEventSpec {
    session: String,
    domain_id: String,
    stream_id: String,
    event_type: String,
    marker: u16,
}

impl GeneratedEventSpec {
    fn envelope(&self) -> EventEnvelope {
        EventEnvelope::new_domain(
            RECORDED_AT.to_string(),
            self.session.clone(),
            self.domain_id.clone(),
            self.stream_id.clone(),
            self.event_type.clone(),
            None,
            json!({ "marker": self.marker }),
        )
    }
}

prop_compose! {
    fn generated_component()(value in "[a-z][a-z0-9_-]{0,16}") -> String {
        value
    }
}

prop_compose! {
    fn generated_event_type()(
        domain in "[a-z][a-z0-9_]{0,12}",
        action in "[a-z][a-z0-9_]{0,12}",
    ) -> String {
        format!("{domain}.{action}")
    }
}

prop_compose! {
    fn generated_event_spec()(
        session_index in 0..SESSIONS.len(),
        domain_id in generated_component(),
        stream_id in generated_component(),
        event_type in generated_event_type(),
        marker in any::<u16>(),
    ) -> GeneratedEventSpec {
        GeneratedEventSpec {
            session: SESSIONS[session_index].to_string(),
            domain_id,
            stream_id,
            event_type,
            marker,
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(8))]

    #[test]
    fn generated_appends_replay_against_in_memory_oracle(
        specs in prop::collection::vec(generated_event_spec(), 100..=200),
        raw_cursors in prop::collection::vec(any::<u16>(), 4),
        limits in prop::collection::vec(0usize..48, 3),
    ) {
        let (_temp_dir, store) = event_store();
        let mut oracle: Vec<EventRecord> = Vec::new();
        for (index, spec) in specs.iter().enumerate() {
            let seq = store.append_envelope(spec.envelope()).unwrap();
            prop_assert_eq!(seq, index as u64 + 1);
            oracle.push(EventRecord::from_envelope(spec.envelope(), seq));
        }

        for &raw in &raw_cursors {
            let cursor = u64::from(raw) % (oracle.len() as u64 + 2);
            let expected: Vec<EventRecord> = oracle
                .iter()
                .filter(|record| record.seq > cursor)
                .cloned()
                .collect();
            prop_assert_eq!(store.read_all_events_after(cursor).unwrap(), expected.clone());

            for &limit in &limits {
                let expected_window: Vec<EventRecord> =
                    expected.iter().take(limit).cloned().collect();
                prop_assert_eq!(
                    store.read_all_events_after_limit(cursor, limit).unwrap(),
                    expected_window
                );
            }

            for session in SESSIONS {
                let expected_session: Vec<EventRecord> = expected
                    .iter()
                    .filter(|record| record.session == session)
                    .cloned()
                    .collect();
                prop_assert_eq!(
                    store.read_events_after(session, cursor).unwrap(),
                    expected_session
                );
            }
        }
    }
}
