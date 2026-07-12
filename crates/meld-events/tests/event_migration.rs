use std::path::Path;

use meld_events::error::EventAuthorityError;
#[cfg(feature = "test-support")]
use meld_events::error::StorageError;
#[cfg(feature = "test-support")]
use meld_events::events::test_support::{EventStore, EventStoreTestSupport as _};
use meld_events::{
    AppendMode, EventAuthority, EventAuthorityOpenOptions, EventEnvelope, EventRecord,
    EventRecordRef, LedgerCursor, LedgerIdentity, LegacyEventMigrationOptions,
    LegacyEventMigrationSource, ReplayRequest,
};
use serde_json::{json, Value};

fn envelope(session: &str, event_type: &str, value: Value) -> EventEnvelope {
    EventEnvelope::new_domain(
        "2026-07-12T00:00:00Z".to_string(),
        session,
        "migration-test",
        format!("stream-{session}"),
        event_type,
        Some(format!("hash-{event_type}")),
        value,
    )
}

fn insert_spine(path: &Path, records: &[EventRecord]) {
    let db = sled::open(path).unwrap();
    let events = db.open_tree("obs_spine_events").unwrap();
    for record in records {
        events
            .insert(
                format!("{:020}", record.seq).as_bytes(),
                serde_json::to_vec(record).unwrap(),
            )
            .unwrap();
    }
    db.flush().unwrap();
}

fn insert_legacy(path: &Path, record: &EventRecord) {
    let db = sled::open(path).unwrap();
    db.open_tree("obs_events")
        .unwrap()
        .insert(
            format!("{}:{:020}", record.session, record.seq).as_bytes(),
            serde_json::to_vec(record).unwrap(),
        )
        .unwrap();
    db.flush().unwrap();
}

fn authority(path: &Path) -> EventAuthority {
    EventAuthority::open(
        sled::open(path).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap()
}

fn persist_identity(path: &Path, identity: LedgerIdentity) {
    let db = sled::open(path).unwrap();
    db.open_tree("obs_spine_meta")
        .unwrap()
        .insert(b"ledger_identity", identity.as_uuid().as_bytes())
        .unwrap();
    db.flush().unwrap();
}

fn replay(authority: &EventAuthority) -> Vec<EventRecord> {
    authority
        .replay_capability()
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: authority.ledger_identity(),
                after_seq: 0,
            },
            limit: 1_024,
        })
        .unwrap()
        .records
}

#[test]
fn empty_source_completes_with_identity_and_cutover_marker() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    sled::open(&source_path).unwrap().flush().unwrap();
    let target = authority(&target_path);

    let source = LegacyEventMigrationSource::open(&source_path).unwrap();
    let report = source
        .migrate_all_into(&target_path, &target, LegacyEventMigrationOptions::new(7))
        .unwrap();

    assert!(report.complete);
    assert_eq!(report.source_record_count, 0);
    assert_eq!(report.target_tip_seq, 0);
    let marker = source.cutover_marker().unwrap().unwrap();
    assert_eq!(marker.source_ledger_id, source.ledger_identity());
    assert_eq!(marker.target_ledger_id, target.ledger_identity());
    assert_eq!(marker.generation, 7);
}

#[test]
fn migration_preserves_target_prefix_and_semantic_envelopes() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    let first = envelope(
        "session:a;1",
        "legacy.first",
        json!({"nested": {"b": 2, "a": 1}}),
    )
    .with_record_id("source-first");
    let second = envelope("session::child", "legacy.second", json!([1, 2, 3]));
    insert_spine(
        &source_path,
        &[
            EventRecord::from_envelope(first.clone(), 4),
            EventRecord::from_envelope(second.clone(), 9),
        ],
    );

    let target = authority(&target_path);
    let prefix = envelope("product", "target.prefix", json!({"prefix": true}));
    target
        .append_capability()
        .append_durable(prefix.clone(), AppendMode::Plain)
        .unwrap();
    let source = LegacyEventMigrationSource::open(&source_path).unwrap();
    let report = source
        .migrate_all_into(
            &target_path,
            &target,
            LegacyEventMigrationOptions {
                generation: 1,
                batch_size: 1,
            },
        )
        .unwrap();

    assert_eq!(report.source_record_count, 2);
    assert_eq!(report.inserted_target_count, 2);
    let records = replay(&target);
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].envelope, prefix);
    assert_eq!(records[1].envelope, first);
    assert_eq!(records[2].envelope, second);
    assert_eq!(
        records.iter().map(|record| record.seq).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

#[test]
fn mixed_trees_and_per_session_sequence_collisions_are_all_migrated() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    insert_spine(
        &source_path,
        &[EventRecord::from_envelope(
            envelope("canonical", "canonical.one", json!({})),
            1,
        )],
    );
    insert_legacy(
        &source_path,
        &EventRecord::from_envelope(envelope("a:child", "legacy.a", json!({"n": 1})), 1),
    );
    insert_legacy(
        &source_path,
        &EventRecord::from_envelope(envelope("b;peer", "legacy.b", json!({"n": 2})), 1),
    );

    let target = authority(&target_path);
    let source = LegacyEventMigrationSource::open(&source_path).unwrap();
    let report = source
        .migrate_all_into(&target_path, &target, LegacyEventMigrationOptions::new(2))
        .unwrap();

    assert_eq!(report.source_record_count, 3);
    assert_eq!(
        replay(&target)
            .iter()
            .map(|record| record.event_type.as_str())
            .collect::<Vec<_>>(),
        vec!["canonical.one", "legacy.a", "legacy.b"]
    );
}

#[test]
fn one_record_batches_resume_and_completed_migration_is_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    insert_spine(
        &source_path,
        &(1..=3)
            .map(|seq| {
                EventRecord::from_envelope(
                    envelope("s", &format!("legacy.{seq}"), json!({"seq": seq})),
                    seq,
                )
            })
            .collect::<Vec<_>>(),
    );
    let target = authority(&target_path);
    let source = LegacyEventMigrationSource::open(&source_path).unwrap();

    let first = source
        .migrate_batch_into(&target_path, &target, 3, 1)
        .unwrap();
    assert!(!first.complete);
    assert_eq!(first.mapped_record_count, 1);
    let second = source
        .migrate_batch_into(&target_path, &target, 3, 1)
        .unwrap();
    assert!(!second.complete);
    assert_eq!(second.mapped_record_count, 2);
    let third = source
        .migrate_batch_into(&target_path, &target, 3, 1)
        .unwrap();
    assert!(third.complete);
    assert_eq!(third.mapped_record_count, 3);

    let rerun = source
        .migrate_all_into(&target_path, &target, LegacyEventMigrationOptions::new(3))
        .unwrap();
    assert_eq!(rerun, third);
    assert_eq!(replay(&target).len(), 3);
}

#[test]
fn identical_record_id_collision_reuses_prefix_but_divergent_collision_fails() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    let identical = envelope("s", "same", json!({"same": true})).with_record_id("shared-id");
    insert_spine(
        &source_path,
        &[EventRecord::from_envelope(identical.clone(), 1)],
    );
    let target = authority(&target_path);
    target
        .append_capability()
        .append_durable(identical, AppendMode::Idempotent)
        .unwrap();
    let source = LegacyEventMigrationSource::open(&source_path).unwrap();
    let report = source
        .migrate_all_into(&target_path, &target, LegacyEventMigrationOptions::new(1))
        .unwrap();
    assert_eq!(report.inserted_target_count, 0);
    assert_eq!(replay(&target).len(), 1);

    let other_source_path = temp.path().join("source-divergent");
    insert_spine(
        &other_source_path,
        &[EventRecord::from_envelope(
            envelope("s", "different", json!({"same": false})).with_record_id("shared-id"),
            1,
        )],
    );
    let other_source = LegacyEventMigrationSource::open(&other_source_path).unwrap();
    let error = other_source
        .migrate_all_into(&target_path, &target, LegacyEventMigrationOptions::new(2))
        .unwrap_err();
    assert!(matches!(
        error,
        EventAuthorityError::MigrationConflict { .. }
    ));
    assert_eq!(replay(&target).len(), 1);
}

#[test]
fn malformed_source_fails_before_any_target_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    let source_db = sled::open(&source_path).unwrap();
    source_db
        .open_tree("obs_spine_events")
        .unwrap()
        .insert(b"00000000000000000001", b"not-json")
        .unwrap();
    source_db.flush().unwrap();
    drop(source_db);
    let target = authority(&target_path);

    let error = match LegacyEventMigrationSource::open(&source_path) {
        Ok(_) => panic!("malformed source unexpectedly opened"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        EventAuthorityError::MigrationConflict { .. }
    ));
    assert!(replay(&target).is_empty());
}

#[test]
fn retention_boundary_excludes_pruned_canonical_prefix() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    insert_spine(
        &source_path,
        &(1..=3)
            .map(|seq| {
                EventRecord::from_envelope(envelope("s", &format!("source.{seq}"), json!({})), seq)
            })
            .collect::<Vec<_>>(),
    );
    let db = sled::open(&source_path).unwrap();
    db.open_tree("obs_spine_meta")
        .unwrap()
        .insert(b"retained_from", &3_u64.to_be_bytes())
        .unwrap();
    db.flush().unwrap();
    drop(db);

    let target = authority(&target_path);
    let source = LegacyEventMigrationSource::open(&source_path).unwrap();
    assert_eq!(source.retained_from(), 3);
    let report = source
        .migrate_all_into(&target_path, &target, LegacyEventMigrationOptions::new(1))
        .unwrap();
    assert_eq!(report.source_record_count, 1);
    assert_eq!(replay(&target)[0].event_type, "source.3");
}

#[test]
fn equal_resolved_paths_and_mismatched_rerun_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    sled::open(&source_path).unwrap().flush().unwrap();
    let target = authority(&target_path);
    let source = LegacyEventMigrationSource::open(&source_path).unwrap();

    let same_path = source
        .migrate_batch_into(&source_path, &target, 1, 1)
        .unwrap_err();
    assert!(matches!(
        same_path,
        EventAuthorityError::MigrationConflict { .. }
    ));

    source
        .migrate_all_into(&target_path, &target, LegacyEventMigrationOptions::new(1))
        .unwrap();
    let generation_error = source
        .migrate_batch_into(&target_path, &target, 2, 1)
        .unwrap_err();
    assert!(matches!(
        generation_error,
        EventAuthorityError::MigrationConflict { .. }
    ));
}

#[cfg(feature = "test-support")]
#[test]
fn source_semantic_appends_are_rejected_after_cutover_while_other_trees_work() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    insert_spine(
        &source_path,
        &[EventRecord::from_envelope(
            envelope("s", "source.one", json!({})),
            1,
        )],
    );
    let target = authority(&target_path);
    let source = LegacyEventMigrationSource::open(&source_path).unwrap();
    source
        .migrate_all_into(&target_path, &target, LegacyEventMigrationOptions::new(1))
        .unwrap();
    drop(source);

    let db = sled::open(&source_path).unwrap();
    let store = EventStore::new(db.clone()).unwrap();
    let error = store
        .append_envelope(envelope("s", "forbidden", json!({})))
        .unwrap_err();
    assert!(matches!(error, StorageError::MigrationConflict(_)));

    db.open_tree("telemetry_sessions")
        .unwrap()
        .insert(b"session", b"still-writable")
        .unwrap();
    db.flush().unwrap();
    assert_eq!(
        db.open_tree("telemetry_sessions")
            .unwrap()
            .get(b"session")
            .unwrap()
            .unwrap()
            .as_ref(),
        b"still-writable"
    );
}

#[test]
fn forward_and_backward_provenance_translate_through_the_complete_plan() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    let source_id = LedgerIdentity::new();
    persist_identity(&source_path, source_id);
    let forward =
        envelope("s", "source.forward", json!({})).with_source_records(vec![EventRecordRef {
            ledger_id: source_id,
            seq: 2,
        }]);
    let backward =
        envelope("s", "source.backward", json!({})).with_source_records(vec![EventRecordRef {
            ledger_id: source_id,
            seq: 1,
        }]);
    insert_spine(
        &source_path,
        &[
            EventRecord::from_envelope(forward, 1),
            EventRecord::from_envelope(backward, 2),
        ],
    );
    let target = authority(&target_path);
    let source = LegacyEventMigrationSource::open(&source_path).unwrap();
    source
        .migrate_all_into(&target_path, &target, LegacyEventMigrationOptions::new(1))
        .unwrap();

    let records = replay(&target);
    assert_eq!(
        records[0].provenance.source_records[0],
        EventRecordRef {
            ledger_id: target.ledger_identity(),
            seq: 2,
        }
    );
    assert_eq!(
        records[1].provenance.source_records[0],
        EventRecordRef {
            ledger_id: target.ledger_identity(),
            seq: 1,
        }
    );
}

#[test]
fn later_batch_record_id_conflict_fails_preflight_without_copying_prefix() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    insert_spine(
        &source_path,
        &[
            EventRecord::from_envelope(envelope("s", "source.safe", json!({})), 1),
            EventRecord::from_envelope(
                envelope("s", "source.divergent", json!({"source": true}))
                    .with_record_id("target-prefix"),
                2,
            ),
        ],
    );
    let target = authority(&target_path);
    let prefix = envelope("target", "target.existing", json!({"source": false}))
        .with_record_id("target-prefix");
    target
        .append_capability()
        .append_durable(prefix.clone(), AppendMode::Idempotent)
        .unwrap();
    let source = LegacyEventMigrationSource::open(&source_path).unwrap();

    let error = source
        .migrate_batch_into(&target_path, &target, 1, 1)
        .unwrap_err();
    assert!(matches!(
        error,
        EventAuthorityError::MigrationConflict { .. }
    ));
    assert_eq!(
        replay(&target)
            .into_iter()
            .map(|record| record.envelope)
            .collect::<Vec<_>>(),
        vec![prefix]
    );
}

#[test]
fn duplicate_source_record_ids_fail_before_target_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    let duplicate = envelope("s", "source.duplicate", json!({})).with_record_id("duplicate");
    insert_spine(
        &source_path,
        &[
            EventRecord::from_envelope(duplicate.clone(), 1),
            EventRecord::from_envelope(duplicate, 2),
        ],
    );
    let target = authority(&target_path);
    let error = match LegacyEventMigrationSource::open(&source_path) {
        Ok(_) => panic!("duplicate source record IDs unexpectedly passed preflight"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        EventAuthorityError::MigrationConflict { .. }
    ));
    assert!(replay(&target).is_empty());
}

#[test]
fn non_monotonic_target_prefix_collision_fails_before_copy() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    insert_spine(
        &source_path,
        &[
            EventRecord::from_envelope(envelope("s", "source.new", json!({})), 1),
            EventRecord::from_envelope(
                envelope("target", "target.prefix", json!({})).with_record_id("prefix-id"),
                2,
            ),
        ],
    );
    let target = authority(&target_path);
    let prefix = envelope("target", "target.prefix", json!({})).with_record_id("prefix-id");
    target
        .append_capability()
        .append_durable(prefix.clone(), AppendMode::Idempotent)
        .unwrap();
    let source = LegacyEventMigrationSource::open(&source_path).unwrap();
    let error = source
        .migrate_batch_into(&target_path, &target, 1, 1)
        .unwrap_err();
    assert!(matches!(
        error,
        EventAuthorityError::MigrationConflict { .. }
    ));
    assert_eq!(replay(&target).len(), 1);
    assert_eq!(replay(&target)[0].envelope, prefix);
}

#[test]
fn missing_legacy_timestamps_are_stable_across_reopen_and_resume() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    let mut first = envelope("s", "source.first", json!({}));
    first.ts.clear();
    first.recorded_at.clear();
    let mut second = envelope("s", "source.second", json!({}));
    second.ts.clear();
    second.recorded_at.clear();
    insert_spine(
        &source_path,
        &[
            EventRecord::from_envelope(first, 1),
            EventRecord::from_envelope(second, 2),
        ],
    );

    let target_id;
    {
        let target = authority(&target_path);
        target_id = target.ledger_identity();
        let source = LegacyEventMigrationSource::open(&source_path).unwrap();
        let partial = source
            .migrate_batch_into(&target_path, &target, 1, 1)
            .unwrap();
        assert_eq!(partial.mapped_record_count, 1);
        assert_eq!(replay(&target)[0].recorded_at, "1970-01-01T00:00:00.000Z");
    }

    let target = EventAuthority::open(
        sled::open(&target_path).unwrap(),
        EventAuthorityOpenOptions {
            expected_ledger_id: Some(target_id),
        },
    )
    .unwrap();
    let source = LegacyEventMigrationSource::open(&source_path).unwrap();
    let complete = source
        .migrate_all_into(&target_path, &target, LegacyEventMigrationOptions::new(1))
        .unwrap();
    assert!(complete.complete);
    assert!(replay(&target).iter().all(|record| {
        record.ts.is_empty() && record.recorded_at == "1970-01-01T00:00:00.000Z"
    }));
}

#[test]
fn equal_identities_and_identity_bypass_provenance_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    let shared = LedgerIdentity::new();
    persist_identity(&source_path, shared);
    persist_identity(&target_path, shared);
    insert_spine(
        &source_path,
        &[EventRecord::from_envelope(
            envelope("s", "source.one", json!({})),
            1,
        )],
    );
    let target = EventAuthority::open(
        sled::open(&target_path).unwrap(),
        EventAuthorityOpenOptions {
            expected_ledger_id: Some(shared),
        },
    )
    .unwrap();
    let source = LegacyEventMigrationSource::open(&source_path).unwrap();
    let error = source
        .migrate_batch_into(&target_path, &target, 1, 1)
        .unwrap_err();
    assert!(matches!(
        error,
        EventAuthorityError::MigrationConflict { .. }
    ));
    assert!(replay(&target).is_empty());

    let foreign_source_path = temp.path().join("foreign-source");
    let source_id = LedgerIdentity::new();
    persist_identity(&foreign_source_path, source_id);
    insert_spine(
        &foreign_source_path,
        &[EventRecord::from_envelope(
            envelope("s", "source.bypass", json!({})).with_source_records(vec![EventRecordRef {
                ledger_id: shared,
                seq: 1,
            }]),
            1,
        )],
    );
    let bypass = match LegacyEventMigrationSource::open(&foreign_source_path) {
        Ok(_) => panic!("foreign target identity provenance bypassed source validation"),
        Err(error) => error,
    };
    assert!(matches!(
        bypass,
        EventAuthorityError::MigrationConflict { .. }
    ));
}

#[test]
fn inserted_mapping_with_missing_target_row_blocks_resume_before_copy() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    insert_spine(
        &source_path,
        &[
            EventRecord::from_envelope(envelope("s", "source.first", json!({})), 1),
            EventRecord::from_envelope(envelope("s", "source.second", json!({})), 2),
        ],
    );

    let target_id;
    {
        let target = authority(&target_path);
        target_id = target.ledger_identity();
        let source = LegacyEventMigrationSource::open(&source_path).unwrap();
        let partial = source
            .migrate_batch_into(&target_path, &target, 1, 1)
            .unwrap();
        assert_eq!(partial.mapped_record_count, 1);
    }

    let db = sled::open(&target_path).unwrap();
    db.open_tree("obs_spine_events")
        .unwrap()
        .remove(b"00000000000000000001")
        .unwrap();
    db.flush().unwrap();
    drop(db);

    let target = EventAuthority::open(
        sled::open(&target_path).unwrap(),
        EventAuthorityOpenOptions {
            expected_ledger_id: Some(target_id),
        },
    )
    .unwrap();
    let source = LegacyEventMigrationSource::open(&source_path).unwrap();
    let error = source
        .migrate_batch_into(&target_path, &target, 1, 1)
        .unwrap_err();
    assert!(matches!(
        error,
        EventAuthorityError::MigrationConflict { .. }
    ));
    assert!(replay(&target).is_empty());
}

#[test]
fn dangling_target_record_index_blocks_migration_before_copy() {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source");
    let target_path = temp.path().join("target");
    insert_spine(
        &source_path,
        &[EventRecord::from_envelope(
            envelope("s", "source.safe", json!({})),
            1,
        )],
    );

    let target_id;
    {
        let target = authority(&target_path);
        target_id = target.ledger_identity();
        target
            .append_capability()
            .append_durable(
                envelope("target", "target.dangling", json!({})).with_record_id("dangling-id"),
                AppendMode::Idempotent,
            )
            .unwrap();
    }
    let db = sled::open(&target_path).unwrap();
    db.open_tree("obs_spine_events")
        .unwrap()
        .remove(b"00000000000000000001")
        .unwrap();
    db.flush().unwrap();
    drop(db);

    let target = EventAuthority::open(
        sled::open(&target_path).unwrap(),
        EventAuthorityOpenOptions {
            expected_ledger_id: Some(target_id),
        },
    )
    .unwrap();
    let source = LegacyEventMigrationSource::open(&source_path).unwrap();
    let error = source
        .migrate_batch_into(&target_path, &target, 1, 1)
        .unwrap_err();
    assert!(matches!(
        error,
        EventAuthorityError::MigrationConflict { .. }
    ));
    assert!(replay(&target).is_empty());
}
