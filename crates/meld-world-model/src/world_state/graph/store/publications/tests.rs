use super::*;
use crate::world_state::graph::events::owner_publication_envelope;
use crate::world_state::graph::test_support::GraphRuntimeTestFixture;

fn operation(revision: &str) -> OwnerPublicationOperation {
    let scope = OwnerPublicationScope {
        scope_id: "quarantine-test".into(),
        branch_id: None,
        perspective_id: None,
        valid_at: None,
    };
    OwnerPublicationOperation::reconstruct(
        "quarantine-rule",
        OwnerPublicationBatch {
            work_input_basis_id: None,
            owner_id: "test-owner".into(),
            revision_id: revision.into(),
            scope: scope.clone(),
            objects: vec![],
            relations: vec![],
            completeness: OwnerCompletenessReceipt {
                receipt_id: format!("receipt-{revision}"),
                scope,
                included_ids: vec![],
                exclusions: vec![],
                failures: vec![],
                status: OwnerCompletenessStatus::Complete,
            },
        },
    )
    .unwrap()
}

#[test]
fn quarantined_publication_handles_block_replay_until_all_holders_release() {
    let directory = tempfile::tempdir().unwrap();
    let db = sled::open(directory.path().join("events")).unwrap();
    let path = directory.path().join("graph.agdb");
    let fixture = GraphRuntimeTestFixture::open(db.clone(), &path).unwrap();
    let runtime = fixture.runtime();
    let store = runtime.traversal_store();
    let independently_opened = super::super::TraversalStore::new(db.clone(), &path).unwrap();
    let first = operation("one");
    let first_seq = fixture
        .append(owner_publication_envelope("test-session", &first).unwrap())
        .unwrap()
        .seq;
    runtime.catch_up().unwrap();
    let before = runtime.durable_event_cursor().unwrap();
    assert_eq!(before.after_seq, first_seq);
    let next = operation("two");
    let next_seq = fixture
        .append(owner_publication_envelope("test-session", &next).unwrap())
        .unwrap()
        .seq;

    // Real engine transaction panic retains an unfinished write and WAL. Generic
    // Database tests separately inject backend errors during rollback and sync.
    let interrupted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        store.publications.db.mutate::<()>(|db| {
            transact(db, |tx| {
                tx.exec_mut(Q::insert().nodes().aliases("unfinished").query())
                    .map_err(data)?;
                panic!("interrupt publication storage mutation");
            })
        })
    }));
    assert!(interrupted.is_err());
    for shared in [&*store, &independently_opened] {
        assert!(shared.owner_publication_for_event(first_seq).is_err());
        assert!(shared.owner_publications_through_seq(next_seq).is_err());
        assert!(shared.flush().is_err());
        assert!(shared.publications.complete_migration().is_err());
        assert!(shared.publications.clear().is_err());
        assert!(shared.publications.migration_complete().is_err());
    }
    assert!(PublicationStore::open(&path, store.resource_id()).is_err());
    assert!(runtime
        .catch_up()
        .unwrap_err()
        .to_string()
        .contains("quarantined"));
    assert_eq!(runtime.durable_event_cursor().unwrap(), before);

    drop(store);
    drop(runtime);
    drop(fixture);
    assert!(PublicationStore::open(&path, independently_opened.resource_id()).is_err());
    drop(independently_opened);

    let recovered = GraphRuntimeTestFixture::open(db.clone(), &path).unwrap();
    let runtime = recovered.runtime();
    let store = runtime.traversal_store();
    assert_eq!(runtime.durable_event_cursor().unwrap(), before);
    assert!(alias(&store.publications.db.read().unwrap(), "unfinished")
        .unwrap()
        .is_none());
    assert_eq!(
        store
            .owner_publication_for_event(first_seq)
            .unwrap()
            .unwrap()
            .operation,
        first
    );
    assert!(store
        .owner_publication_for_event(next_seq)
        .unwrap()
        .is_none());
    runtime.catch_up().unwrap();
    assert_eq!(runtime.durable_event_cursor().unwrap().after_seq, next_seq);
    assert_eq!(
        store
            .owner_publication_for_event(next_seq)
            .unwrap()
            .unwrap()
            .operation,
        next
    );
    drop(store);
    drop(runtime);
    drop(recovered);

    let reopened = GraphRuntimeTestFixture::open(db, &path).unwrap();
    let runtime = reopened.runtime();
    assert_eq!(runtime.durable_event_cursor().unwrap().after_seq, next_seq);
    assert_eq!(
        runtime
            .traversal_store()
            .owner_publication_for_event(next_seq)
            .unwrap()
            .unwrap()
            .operation,
        next
    );
    assert_eq!(runtime.catch_up().unwrap(), 0);
}

#[test]
fn ordinary_missing_alias_and_domain_conflict_do_not_quarantine() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("graph.agdb");
    let store = PublicationStore::open(&path, "resource").unwrap();
    assert!(store.publication(999).unwrap().is_none());
    let first = ProjectedOwnerPublication {
        source_event: crate::events::EventRecordRef {
            ledger_id: crate::events::LedgerIdentity::new(),
            seq: 1,
        },
        source_route: None,
        operation: operation("one"),
    };
    store.put(&first).unwrap();
    let mut divergent = first.clone();
    divergent.operation = operation("two");
    assert!(store
        .put(&divergent)
        .unwrap_err()
        .to_string()
        .contains("divergent"));
    assert_eq!(store.publication(1).unwrap(), Some(first));
    store.flush().unwrap();
}
