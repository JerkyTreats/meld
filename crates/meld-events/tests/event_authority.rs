use std::sync::{Arc, Barrier};
use std::time::Duration;

use meld_events::error::{EventAuthorityError, StorageError};
use meld_events::events::authority::{
    AppendDisposition, AppendMode, EventAuthority, EventAuthorityOpenOptions,
    EventIngressFenceState, LedgerCursor, ReplayRequest, SubscriptionPollRequest, MAX_REPLAY_LIMIT,
    MAX_SUBSCRIPTION_TIMEOUT_MS,
};
use meld_events::events::identity::LedgerIdentity;
use meld_events::events::observability::CoverageTruncation;
#[cfg(feature = "test-support")]
use meld_events::events::test_support::{
    EventCursor, EventCursorTestSupport as _, EventStore, EventStoreTestSupport as _,
};
use meld_events::{DomainObjectRef, EventAppendValidationCode, EventEnvelope, EventRelation};
use serde_json::json;

const META_TREE: &str = "obs_spine_meta";
const IDENTITY_KEY: &[u8] = b"ledger_identity";

fn envelope(index: usize) -> EventEnvelope {
    EventEnvelope::new_domain(
        "2026-07-10T00:00:00Z".to_string(),
        "authority-session",
        "execution",
        "authority-stream",
        "execution.authority.test",
        None,
        json!({ "index": index }),
    )
}

fn temporary_authority() -> (tempfile::TempDir, EventAuthority) {
    let dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(dir.path()).unwrap();
    let authority = EventAuthority::open(db, EventAuthorityOpenOptions::default()).unwrap();
    (dir, authority)
}

#[test]
fn first_open_persists_identity_and_reopen_is_stable() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let identity = {
        let authority = EventAuthority::open(
            sled::open(&path).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        authority.ledger_identity()
    };

    let reopened = EventAuthority::open(
        sled::open(&path).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    assert_eq!(reopened.ledger_identity(), identity);
}

#[test]
fn expected_identity_seeds_an_empty_ledger() {
    let expected = LedgerIdentity::new();
    let db = sled::Config::new().temporary(true).open().unwrap();
    let authority = EventAuthority::open(
        db,
        EventAuthorityOpenOptions {
            expected_ledger_id: Some(expected),
        },
    )
    .unwrap();
    assert_eq!(authority.ledger_identity(), expected);
}

#[test]
fn concurrent_first_open_establishes_one_identity_atomically() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let db = db.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                EventAuthority::open(db, EventAuthorityOpenOptions::default())
            })
        })
        .collect();
    let results: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(
                result,
                Err(EventAuthorityError::DuplicateAuthorityBinding { .. })
            ))
            .count(),
        1
    );
    let persisted = db.open_tree(META_TREE).unwrap().get(IDENTITY_KEY).unwrap();
    assert_eq!(persisted.unwrap().len(), 16);
}

#[test]
fn corrupt_and_mismatched_identities_fail_closed() {
    let corrupt_db = sled::Config::new().temporary(true).open().unwrap();
    corrupt_db
        .open_tree(META_TREE)
        .unwrap()
        .insert(IDENTITY_KEY, b"bad".as_slice())
        .unwrap();
    assert!(matches!(
        EventAuthority::open(corrupt_db, EventAuthorityOpenOptions::default()),
        Err(EventAuthorityError::CorruptPersistedIdentity { .. })
    ));

    let db = sled::Config::new().temporary(true).open().unwrap();
    let actual = LedgerIdentity::new();
    db.open_tree(META_TREE)
        .unwrap()
        .insert(IDENTITY_KEY, actual.as_uuid().as_bytes())
        .unwrap();
    let expected = LedgerIdentity::new();
    assert_eq!(
        EventAuthority::open(
            db,
            EventAuthorityOpenOptions {
                expected_ledger_id: Some(expected),
            },
        )
        .err(),
        Some(EventAuthorityError::IdentityMismatch { expected, actual })
    );
}

#[cfg(feature = "test-support")]
#[test]
fn duplicate_same_database_and_copied_identity_are_rejected() {
    let first_db = sled::Config::new().temporary(true).open().unwrap();
    let first =
        EventAuthority::open(first_db.clone(), EventAuthorityOpenOptions::default()).unwrap();
    let identity = first.ledger_identity();
    assert!(matches!(
        EventAuthority::open(first_db, EventAuthorityOpenOptions::default()),
        Err(EventAuthorityError::DuplicateAuthorityBinding { ledger_id }) if ledger_id == identity
    ));

    let copied_db = sled::Config::new().temporary(true).open().unwrap();
    copied_db
        .open_tree(META_TREE)
        .unwrap()
        .insert(IDENTITY_KEY, identity.as_uuid().as_bytes())
        .unwrap();
    let copied_store = EventStore::new(copied_db.clone()).unwrap();
    copied_store.append_envelope(envelope(0)).unwrap();
    copied_store.flush().unwrap();
    drop(copied_store);
    assert!(matches!(
        EventAuthority::open(copied_db, EventAuthorityOpenOptions::default()),
        Err(EventAuthorityError::DuplicateAuthorityBinding { ledger_id }) if ledger_id == identity
    ));
}

#[test]
fn final_capability_drop_releases_the_authority_lease() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let authority = EventAuthority::open(db.clone(), EventAuthorityOpenOptions::default()).unwrap();
    let append = authority.append_capability();
    drop(authority);
    assert!(matches!(
        EventAuthority::open(db.clone(), EventAuthorityOpenOptions::default()),
        Err(EventAuthorityError::DuplicateAuthorityBinding { .. })
    ));
    drop(append);
    EventAuthority::open(db, EventAuthorityOpenOptions::default()).unwrap();
}

#[cfg(feature = "test-support")]
#[test]
fn watermark_recovers_the_durable_tip() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    {
        let store = EventStore::new(sled::open(&path).unwrap()).unwrap();
        store.append_envelope(envelope(0)).unwrap();
        store.append_envelope(envelope(1)).unwrap();
        store.flush().unwrap();
    }
    let authority = EventAuthority::open(
        sled::open(&path).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let snapshot = authority.watermark_capability().snapshot().unwrap();
    assert_eq!(snapshot.committed_seq, 2);
    assert_eq!(snapshot.tip_seq, 2);
}

#[test]
fn all_authority_append_modes_advance_one_watermark() {
    let (_dir, authority) = temporary_authority();
    let ledger_id = authority.ledger_identity();
    let append = authority.append_capability();
    let wait_for_next = |after_seq| {
        let subscription = authority.subscription_capability();
        let (started_tx, started_rx) = std::sync::mpsc::sync_channel(1);
        let waiter = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            let started = std::time::Instant::now();
            let page = subscription
                .poll(SubscriptionPollRequest {
                    replay: ReplayRequest {
                        cursor: LedgerCursor {
                            ledger_id,
                            after_seq,
                        },
                        limit: 16,
                    },
                    timeout_ms: 2_000,
                })
                .unwrap();
            (page, started.elapsed())
        });
        started_rx.recv().unwrap();
        std::thread::sleep(Duration::from_millis(50));
        waiter
    };

    let plain_waiter = wait_for_next(0);
    let plain = append
        .append_durable(envelope(0), AppendMode::Plain)
        .unwrap();
    let (plain_page, plain_elapsed) = plain_waiter.join().unwrap();
    assert_eq!(plain_page.records[0].seq, plain.seq);
    assert!(plain_elapsed < Duration::from_secs(1));

    let idempotent_waiter = wait_for_next(plain.seq);
    let idempotent = append
        .append_durable(
            envelope(1).with_record_id("authority-idempotent"),
            AppendMode::Idempotent,
        )
        .unwrap();
    let (idempotent_page, idempotent_elapsed) = idempotent_waiter.join().unwrap();
    assert_eq!(idempotent_page.records[0].seq, idempotent.seq);
    assert!(idempotent_elapsed < Duration::from_secs(1));

    let best_effort_waiter = wait_for_next(idempotent.seq);
    append
        .append_best_effort(envelope(2), AppendMode::Plain)
        .unwrap();
    let (best_effort_page, best_effort_elapsed) = best_effort_waiter.join().unwrap();
    assert_eq!(best_effort_page.records[0].seq, 3);
    assert!(best_effort_elapsed < Duration::from_secs(1));

    let snapshot = authority.watermark_capability().snapshot().unwrap();
    assert_eq!(snapshot.committed_seq, 3);
    assert_eq!(snapshot.tip_seq, 3);
}

#[test]
fn idempotent_duplicate_receipt_reuses_the_original_sequence() {
    let (_dir, authority) = temporary_authority();
    let append = authority.append_capability();
    let tagged = envelope(0).with_record_id("same-record");
    let first = append
        .append_durable(tagged.clone(), AppendMode::Idempotent)
        .unwrap();
    let duplicate = append
        .append_durable(tagged, AppendMode::Idempotent)
        .unwrap();
    assert_eq!(first.seq, duplicate.seq);
    assert_eq!(first.disposition, AppendDisposition::Inserted);
    assert_eq!(duplicate.disposition, AppendDisposition::Duplicate);
}

#[test]
fn idempotent_append_requires_a_stable_record_id_before_admission() {
    let (_dir, authority) = temporary_authority();
    let append = authority.append_capability();

    assert!(matches!(
        append.append_durable(envelope(0), AppendMode::Idempotent),
        Err(EventAuthorityError::AppendValidation {
            code: EventAppendValidationCode::MissingIdempotencyRecordId,
            ..
        })
    ));
    let watermark = authority.watermark_capability().snapshot().unwrap();
    assert_eq!(watermark.tip_seq, 0);
    assert_eq!(watermark.committed_seq, 0);
}

#[test]
fn append_ingress_rejects_duplicate_objects_and_undeclared_relation_endpoints() {
    let (_dir, authority) = temporary_authority();
    let append = authority.append_capability();
    let subject = DomainObjectRef::new("workspace_fs", "node", "readme").unwrap();
    let duplicate = envelope(0).with_graph(vec![subject.clone(), subject.clone()], vec![]);
    assert!(matches!(
        append.append_durable(duplicate, AppendMode::Plain),
        Err(EventAuthorityError::AppendValidation {
            code: EventAppendValidationCode::DuplicateObjectReference,
            ..
        })
    ));

    let missing = DomainObjectRef::new("execution", "artifact", "artifact-a").unwrap();
    let relation = EventRelation::new("selected", subject.clone(), missing).unwrap();
    let undeclared = envelope(1).with_graph(vec![subject], vec![relation]);
    assert!(matches!(
        append.append_durable(undeclared, AppendMode::Plain),
        Err(EventAuthorityError::AppendValidation {
            code: EventAppendValidationCode::RelationEndpointNotDeclared,
            ..
        })
    ));
}

#[test]
fn final_barrier_closes_shared_ingress_after_draining_accepted_work() {
    let (_dir, authority) = temporary_authority();
    let first = authority.append_capability();
    let contender = first.clone();
    first
        .append_best_effort(envelope(0), AppendMode::Plain)
        .unwrap();

    let barrier = first.close_and_drain().unwrap();

    assert_eq!(barrier.fence().state, EventIngressFenceState::Closed);
    assert_eq!(barrier.fence().ledger_id, authority.ledger_identity());
    assert_eq!(barrier.watermark().committed_seq, 1);
    assert_eq!(barrier.watermark().tip_seq, 1);
    assert!(matches!(
        contender.append_durable(envelope(1), AppendMode::Plain),
        Err(EventAuthorityError::Unavailable { .. })
    ));
    assert_eq!(contender.ingress_fence(), barrier.fence());
}

#[cfg(feature = "test-support")]
#[test]
fn failed_final_barrier_keeps_ingress_in_draining_state() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let authority = EventAuthority::open(db.clone(), EventAuthorityOpenOptions::default()).unwrap();
    let raw_store = EventStore::new(db).unwrap();
    raw_store.append_envelope(envelope(0)).unwrap();
    raw_store.flush().unwrap();
    let append = authority.append_capability();

    assert!(matches!(
        append.close_and_drain(),
        Err(EventAuthorityError::InvalidRequest { .. })
    ));
    assert_eq!(
        append.ingress_fence().state,
        EventIngressFenceState::Draining
    );
}

#[test]
fn foreign_replay_subscription_watermark_and_registry_requests_are_rejected() {
    let (_dir, authority) = temporary_authority();
    let foreign = LedgerIdentity::new();
    let replay = ReplayRequest {
        cursor: LedgerCursor {
            ledger_id: foreign,
            after_seq: 0,
        },
        limit: 1,
    };
    assert!(matches!(
        authority.replay_capability().replay(replay),
        Err(EventAuthorityError::IdentityMismatch { .. })
    ));
    assert!(matches!(
        authority
            .subscription_capability()
            .poll(SubscriptionPollRequest {
                replay,
                timeout_ms: 0,
            }),
        Err(EventAuthorityError::IdentityMismatch { .. })
    ));
    assert!(matches!(
        authority
            .watermark_capability()
            .wait_past(replay.cursor, Duration::from_millis(0)),
        Err(EventAuthorityError::IdentityMismatch { .. })
    ));
    assert!(matches!(
        authority
            .consumer_registry_capability()
            .report("graph", replay.cursor),
        Err(EventAuthorityError::IdentityMismatch { .. })
    ));
}

#[cfg(feature = "test-support")]
#[test]
fn bound_cursor_rejects_legacy_and_foreign_payloads_until_explicit_migration() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let tree = db.open_tree("consumer").unwrap();
    EventCursor::new(tree.clone(), "graph").advance(7).unwrap();
    let identity = LedgerIdentity::new();
    assert!(
        EventCursor::bind_compatibility(tree.clone(), "graph", identity)
            .get()
            .is_err()
    );

    let migrated = EventCursor::migrate_legacy(tree.clone(), "graph", identity).unwrap();
    assert_eq!(migrated.get().unwrap(), 7);
    let foreign = EventCursor::bind_compatibility(tree, "graph", LedgerIdentity::new());
    assert!(matches!(
        foreign.get(),
        Err(StorageError::IdentityMismatch { .. })
    ));
}

#[test]
fn registry_report_is_identity_bearing_and_durable_on_return() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    let identity = {
        let authority = EventAuthority::open(
            sled::open(&path).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let identity = authority.ledger_identity();
        let position = authority
            .consumer_registry_capability()
            .report(
                "graph",
                LedgerCursor {
                    ledger_id: identity,
                    after_seq: 9,
                },
            )
            .unwrap();
        assert_eq!(position.ledger_id, identity);
        identity
    };

    let reopened = EventAuthority::open(
        sled::open(&path).unwrap(),
        EventAuthorityOpenOptions {
            expected_ledger_id: Some(identity),
        },
    )
    .unwrap();
    assert_eq!(
        reopened
            .consumer_registry_capability()
            .get("graph")
            .unwrap()
            .unwrap()
            .reported_seq,
        9
    );
}

#[test]
fn bound_registry_rejects_a_foreign_persisted_identity() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let expected = LedgerIdentity::new();
    let foreign = LedgerIdentity::new();
    db.open_tree(META_TREE)
        .unwrap()
        .insert(IDENTITY_KEY, expected.as_uuid().as_bytes())
        .unwrap();
    db.open_tree("event_consumer_cursors")
        .unwrap()
        .insert(
            b"graph",
            serde_json::to_vec(&json!({
                "ledger_id": foreign,
                "reported_seq": 3,
            }))
            .unwrap(),
        )
        .unwrap();
    let authority = EventAuthority::open(
        db,
        EventAuthorityOpenOptions {
            expected_ledger_id: Some(expected),
        },
    )
    .unwrap();
    assert_eq!(
        authority.consumer_registry_capability().get("graph").err(),
        Some(EventAuthorityError::IdentityMismatch {
            expected,
            actual: foreign,
        })
    );
}

#[test]
fn replay_enforces_bounds_and_reports_before_after_coverage() {
    let (_dir, authority) = temporary_authority();
    let identity = authority.ledger_identity();
    let append = authority.append_capability();
    for index in 0..4 {
        append
            .append_durable(envelope(index), AppendMode::Plain)
            .unwrap();
    }
    let replay = authority.replay_capability();
    for invalid in [0, MAX_REPLAY_LIMIT + 1, usize::MAX] {
        assert!(matches!(
            replay.replay(ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id: identity,
                    after_seq: 0,
                },
                limit: invalid,
            }),
            Err(EventAuthorityError::InvalidRequest { .. })
        ));
    }

    let first = replay
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: identity,
                after_seq: 0,
            },
            limit: 2,
        })
        .unwrap();
    assert_eq!(first.coverage.truncation, CoverageTruncation::After);
    assert_eq!(first.coverage.scanned_from_seq, Some(1));
    assert_eq!(first.coverage.scanned_through_seq, Some(2));

    let middle = replay
        .replay(ReplayRequest {
            cursor: first.next_cursor,
            limit: 1,
        })
        .unwrap();
    assert_eq!(middle.coverage.truncation, CoverageTruncation::Both);
    assert_eq!(middle.records[0].seq, 3);

    let final_page = replay
        .replay(ReplayRequest {
            cursor: middle.next_cursor,
            limit: MAX_REPLAY_LIMIT,
        })
        .unwrap();
    assert_eq!(final_page.coverage.truncation, CoverageTruncation::Before);
    assert_eq!(final_page.records[0].seq, 4);
}

#[test]
fn subscription_rejects_excessive_timeout() {
    let (_dir, authority) = temporary_authority();
    let identity = authority.ledger_identity();
    assert!(matches!(
        authority
            .subscription_capability()
            .poll(SubscriptionPollRequest {
                replay: ReplayRequest {
                    cursor: LedgerCursor {
                        ledger_id: identity,
                        after_seq: 0,
                    },
                    limit: 1,
                },
                timeout_ms: MAX_SUBSCRIPTION_TIMEOUT_MS + 1,
            }),
        Err(EventAuthorityError::InvalidRequest { .. })
    ));
}

#[test]
fn unavailable_writer_errors_keep_their_serializable_authority_category() {
    let error = EventAuthorityError::from(StorageError::Unavailable(
        "ledger writer disconnected".to_string(),
    ));
    assert_eq!(
        error,
        EventAuthorityError::Unavailable {
            message: "ledger writer disconnected".to_string(),
        }
    );
    let encoded = serde_json::to_vec(&error).unwrap();
    assert_eq!(
        serde_json::from_slice::<EventAuthorityError>(&encoded).unwrap(),
        error
    );
}
