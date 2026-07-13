#[path = "support/task_network.rs"]
mod task_network_support;

use meld_events::{AppendDisposition, AppendReceipt, DomainObjectRef, LedgerIdentity};
use meld_execution::task_network::command::{Command, Response};
use meld_execution::task_network::mutation::{Mutation, ReadPrecondition, Rejection, Set};
use meld_execution::task_network::outcome::PublicationState;
use meld_execution::task_network::state::TaskStatus;
use meld_execution::task_network::store::{
    network_storage_key, InMemoryTaskNetworkStore, SledTaskNetworkStore, TaskNetworkStoreError,
    TaskNetworkStoreFactory,
};
use meld_execution::task_network::AttributedOutcome;
use proptest::prelude::*;
use serde_json::json;

fn open_store(db: &sled::Db) -> SledTaskNetworkStore {
    SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap()
}

fn open_db() -> sled::Db {
    sled::Config::new().temporary(true).open().unwrap()
}

fn assert_decode_error(error: TaskNetworkStoreError) {
    assert!(matches!(error, TaskNetworkStoreError::Decode(_)));
}

fn commit_single_task_for_store(store: &mut SledTaskNetworkStore, task_instance_id: &str) {
    let set = Set::new(
        store.state().network_id.clone(),
        "composition-fixture",
        format!("inject-{task_instance_id}"),
        vec![Mutation::Inject(task_network_support::inject_for_node(
            task_network_support::single_task_node(task_instance_id),
            vec![],
        ))],
        vec![],
    );
    let request = task_network_support::apply_sled_command(
        store,
        &format!("command-commit-{task_instance_id}"),
        Command::ApplyMutationSet(set),
    );
    store.submit(request).unwrap();
}

#[test]
fn task_network_factory_opens_flushes_and_reopens_network() {
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    {
        let mut store = factory.open_network("network-a").unwrap();
        commit_single_task_for_store(&mut store, "task-alpha");
        store.flush().unwrap();
    }

    let reopened = factory.open_network("network-a").unwrap();

    assert_eq!(reopened.state().revision, 1);
    assert!(reopened.state().tasks.contains_key("task-alpha"));
}

#[test]
fn task_network_factory_isolates_network_databases() {
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    {
        let mut left = factory.open_network("network-a").unwrap();
        commit_single_task_for_store(&mut left, "task-alpha");
        left.flush().unwrap();
        let mut right = factory.open_network("network-b").unwrap();
        commit_single_task_for_store(&mut right, "task-beta");
        right.flush().unwrap();
    }

    let left = factory.open_network("network-a").unwrap();
    let right = factory.open_network("network-b").unwrap();

    assert!(temp.path().join("network-a.sled").exists());
    assert!(temp.path().join("network-b.sled").exists());
    assert!(left.state().tasks.contains_key("task-alpha"));
    assert!(!left.state().tasks.contains_key("task-beta"));
    assert!(right.state().tasks.contains_key("task-beta"));
    assert!(!right.state().tasks.contains_key("task-alpha"));
}

#[test]
fn network_storage_key_rejects_empty_id() {
    let error = network_storage_key("").unwrap_err();

    assert!(matches!(error, TaskNetworkStoreError::Storage(_)));
}

#[test]
fn network_storage_key_rejects_invalid_id() {
    let error = network_storage_key("network/a").unwrap_err();

    assert!(matches!(error, TaskNetworkStoreError::Storage(_)));
}

#[test]
fn network_storage_key_keeps_valid_id_unchanged() {
    assert_eq!(
        network_storage_key("network-a_1.docs").unwrap(),
        "network-a_1.docs"
    );
}

fn stored_request_json(command_id: &str, request_hash: &str) -> serde_json::Value {
    let state = meld_execution::task_network::state::NetworkState::empty("network-docs");
    let request = task_network_support::command_for_state(
        "network-docs",
        state.revision,
        &state.state_hash,
        command_id,
        Command::ApplyMutationSet(Set::empty("network-docs", "composition-empty", "empty")),
    );
    json!({
        "command_id": command_id,
        "request_hash": request_hash,
        "request": request,
    })
}

fn stored_response_json(command_id: &str, request_hash: &str) -> serde_json::Value {
    json!({
        "command_id": command_id,
        "request_hash": request_hash,
        "response": Response::Rejected(Rejection::InvalidGraph("rejected".to_string())),
    })
}

fn committed_journal_json(revision: u64, state_hash: String) -> serde_json::Value {
    let mut memory = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut memory, "task-alpha");
    json!({
        "network_id": "network-docs",
        "revision": revision,
        "state_hash": state_hash,
        "record": memory.journal()[0],
    })
}

#[test]
fn accepted_inject_mutation_survives_reopen() {
    let db = open_db();
    {
        let mut store = open_store(&db);
        task_network_support::commit_single_task_sled(&mut store, "task-alpha");
    }

    let store = open_store(&db);

    assert_eq!(store.state().revision, 1);
    assert!(store.state().tasks.contains_key("task-alpha"));
    assert_eq!(store.journal().len(), 1);
}

#[test]
fn claim_survives_reopen() {
    let db = open_db();
    {
        let mut store = open_store(&db);
        task_network_support::commit_single_task_sled(&mut store, "task-alpha");
        task_network_support::claim_ready_sled(&mut store, "command-claim", "claim-alpha");
    }

    let store = open_store(&db);

    assert!(matches!(
        store.state().statuses.get("task-alpha"),
        Some(TaskStatus::Running { claim_id }) if claim_id == "claim-alpha"
    ));
    assert!(store.state().claims.contains_key("claim-alpha"));
}

#[test]
fn outcome_survives_reopen() {
    let db = open_db();
    {
        let mut store = open_store(&db);
        task_network_support::commit_single_task_sled(&mut store, "task-alpha");
        let task_instance_id =
            task_network_support::claim_ready_sled(&mut store, "command-claim", "claim-alpha");
        let claim = store.state().claims.get("claim-alpha").unwrap().clone();
        let outcome =
            task_network_support::outcome_for_claim("outcome-alpha", &task_instance_id, &claim);
        let request = task_network_support::apply_sled_command(
            &store,
            "command-outcome",
            Command::RecordTaskOutcome(outcome),
        );
        store.submit(request).unwrap();
    }

    let store = open_store(&db);

    assert!(matches!(
        store.state().statuses.get("task-alpha"),
        Some(TaskStatus::Succeeded { outcome_id }) if outcome_id == "outcome-alpha"
    ));
    assert!(store.state().outcomes.contains_key("outcome-alpha"));
}

#[test]
fn attributed_outcome_preserves_semantic_lineage_across_reopen() {
    let db = open_db();
    let publication_id;
    {
        let mut store = open_store(&db);
        let mut node = task_network_support::single_task_node("task-alpha");
        node.lineage.subject = Some(
            DomainObjectRef::new("workspace_fs", "node", "readme")
                .expect("subject identity is valid"),
        );
        let set = Set::new(
            "network-docs",
            "composition-fixture",
            "inject-attributed",
            vec![Mutation::Inject(task_network_support::inject_for_node(
                node.clone(),
                vec![],
            ))],
            vec![],
        );
        let request = task_network_support::apply_sled_command(
            &store,
            "command-inject-attributed",
            Command::ApplyMutationSet(set),
        );
        store.submit(request).unwrap();
        let task_instance_id =
            task_network_support::claim_ready_sled(&mut store, "command-claim", "claim-alpha");
        let claim = store.state().claims.get("claim-alpha").unwrap().clone();
        let outcome =
            task_network_support::outcome_for_claim("outcome-alpha", &task_instance_id, &claim);
        let legacy_request = task_network_support::apply_sled_command(
            &store,
            "command-outcome-legacy",
            Command::RecordTaskOutcome(outcome.clone()),
        );
        assert!(matches!(
            store.submit(legacy_request).unwrap(),
            Response::Rejected(_)
        ));
        let attributed = AttributedOutcome::for_task(outcome, &node).unwrap();
        let request = task_network_support::apply_sled_command(
            &store,
            "command-outcome",
            Command::RecordAttributedTaskOutcome(attributed),
        );
        assert!(matches!(
            store.submit(request).unwrap(),
            Response::Accepted { .. }
        ));
        publication_id = store.state().publications.keys().next().unwrap().clone();
    }

    let store = open_store(&db);
    let publication = store.state().publications.get(&publication_id).unwrap();
    let lineage = publication.semantic_lineage.as_ref().unwrap();
    assert_eq!(lineage.goal.object_id, "goal-fixture");
    assert_eq!(lineage.method.object_id, "method-fixture");
    assert_eq!(lineage.projection_frame.object_id, "frame-fixture");
    assert_eq!(lineage.subject.domain_id, "workspace_fs");
    assert_eq!(lineage.subject.object_kind, "node");
    assert_eq!(lineage.subject.object_id, "readme");
}

#[test]
fn pending_publication_survives_reopen() {
    let db = open_db();
    let publication_id;
    {
        let mut store = open_store(&db);
        task_network_support::commit_single_task_sled(&mut store, "task-alpha");
        let task_instance_id =
            task_network_support::claim_ready_sled(&mut store, "command-claim", "claim-alpha");
        let claim = store.state().claims.get("claim-alpha").unwrap().clone();
        let outcome =
            task_network_support::outcome_for_claim("outcome-alpha", &task_instance_id, &claim);
        let request = task_network_support::apply_sled_command(
            &store,
            "command-outcome",
            Command::RecordTaskOutcome(outcome),
        );
        store.submit(request).unwrap();
        publication_id = store.state().publications.keys().next().unwrap().clone();
    }

    let store = open_store(&db);

    assert!(matches!(
        store
            .state()
            .publications
            .get(&publication_id)
            .unwrap()
            .state,
        PublicationState::Pending
    ));
}

#[test]
fn marked_publication_survives_reopen() {
    let db = open_db();
    let publication_id;
    {
        let mut store = open_store(&db);
        task_network_support::commit_single_task_sled(&mut store, "task-alpha");
        let task_instance_id =
            task_network_support::claim_ready_sled(&mut store, "command-claim", "claim-alpha");
        let claim = store.state().claims.get("claim-alpha").unwrap().clone();
        let outcome =
            task_network_support::outcome_for_claim("outcome-alpha", &task_instance_id, &claim);
        let request = task_network_support::apply_sled_command(
            &store,
            "command-outcome",
            Command::RecordTaskOutcome(outcome),
        );
        store.submit(request).unwrap();
        let mut publication = store.state().publications.values().next().unwrap().clone();
        publication.state = PublicationState::Published {
            marked_revision: 0,
            receipt: Some(AppendReceipt {
                ledger_id: LedgerIdentity::new(),
                seq: 1,
                disposition: AppendDisposition::Inserted,
            }),
            legacy_event_seq: None,
        };
        publication_id = publication.publication_id.clone();
        let request = task_network_support::apply_sled_command(
            &store,
            "command-mark-publication",
            Command::MarkPublication(publication),
        );
        store.submit(request).unwrap();
    }

    let store = open_store(&db);

    assert!(matches!(
        store.state().publications.get(&publication_id).unwrap().state,
        PublicationState::Published { marked_revision, .. } if marked_revision == 4
    ));
}

#[test]
fn duplicate_accepted_command_replays_after_reopen() {
    let db = open_db();
    let request;
    {
        let mut store = open_store(&db);
        request = task_network_support::apply_sled_command(
            &store,
            "command-commit",
            Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-alpha")),
        );
        store.submit(request.clone()).unwrap();
    }

    let mut store = open_store(&db);
    let duplicate = store.submit(request).unwrap();

    assert!(matches!(duplicate, Response::Duplicate { revision: 1, .. }));
}

#[test]
fn duplicate_rejected_command_replays_after_reopen() {
    let db = open_db();
    let request;
    let failed_precondition = ReadPrecondition::NodeExists("task-alpha".to_string());
    {
        let mut store = open_store(&db);
        let mut rejected_request = task_network_support::apply_sled_command(
            &store,
            "command-rejected",
            Command::ApplyMutationSet(Set::empty("network-docs", "composition-empty", "empty")),
        );
        rejected_request
            .read_preconditions
            .push(failed_precondition.clone());
        store.submit(rejected_request.clone()).unwrap();
        request = rejected_request;
    }

    let mut store = open_store(&db);
    task_network_support::commit_single_task_sled(&mut store, "task-alpha");
    let duplicate = store.submit(request).unwrap();

    assert_eq!(
        duplicate,
        Response::Rejected(Rejection::FailedPrecondition(failed_precondition))
    );
    assert_eq!(store.state().revision, 1);
}

#[test]
fn same_command_id_with_different_payload_rejects_after_reopen() {
    let db = open_db();
    {
        let mut store = open_store(&db);
        task_network_support::commit_single_task_sled(&mut store, "task-alpha");
    }

    let mut store = open_store(&db);
    let duplicate = task_network_support::apply_sled_command(
        &store,
        "command-commit-task-alpha",
        Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-beta")),
    );

    assert!(matches!(
        store.submit(duplicate).unwrap(),
        Response::Rejected(Rejection::DuplicateCommand(command_id))
            if command_id == "command-commit-task-alpha"
    ));
}

#[test]
fn stale_proposal_persists_rejection_and_duplicate_replays_same_rejection() {
    let db = open_db();
    let request;
    {
        let mut store = open_store(&db);
        request = task_network_support::command_for_state(
            "network-docs",
            7,
            &store.state().state_hash,
            "command-stale",
            Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-alpha")),
        );
        assert!(matches!(
            store.submit(request.clone()).unwrap(),
            Response::Rejected(Rejection::StaleBase { .. })
        ));
    }

    let mut store = open_store(&db);
    let duplicate = store.submit(request).unwrap();

    assert!(matches!(
        duplicate,
        Response::Rejected(Rejection::StaleBase {
            expected: 7,
            actual: 0
        })
    ));
    assert_eq!(store.state().revision, 0);
}

#[test]
fn legacy_durable_record_shapes_replay_from_contained_products() {
    let db = open_db();
    let mut memory = InMemoryTaskNetworkStore::new("network-docs");
    let request = task_network_support::apply_memory_command(
        &memory,
        "command-commit-task-alpha",
        Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-alpha")),
    );
    let response = memory.submit(request.clone());
    let state = memory.state().clone();
    let request_hash = "legacy-request-hash";

    db.open_tree("task_network_command_requests")
        .unwrap()
        .insert(
            request.command_id.as_bytes(),
            serde_json::to_vec(&json!({
                "command_id": request.command_id.clone(),
                "request_hash": request_hash,
                "request": request,
            }))
            .unwrap(),
        )
        .unwrap();
    db.open_tree("task_network_command_responses")
        .unwrap()
        .insert(
            "command-commit-task-alpha",
            serde_json::to_vec(&json!({
                "command_id": "command-commit-task-alpha",
                "request_hash": request_hash,
                "response": response,
            }))
            .unwrap(),
        )
        .unwrap();
    db.open_tree("task_network_journal_by_revision")
        .unwrap()
        .insert(
            1_u64.to_be_bytes(),
            serde_json::to_vec(&json!({
                "network_id": "network-docs",
                "revision": 1,
                "state_hash": state.state_hash.clone(),
                "record": memory.journal()[0],
            }))
            .unwrap(),
        )
        .unwrap();
    db.open_tree("task_network_latest_state")
        .unwrap()
        .insert(
            "latest",
            serde_json::to_vec(&json!({
                "network_id": "network-docs",
                "revision": state.revision,
                "state_hash": state.state_hash.clone(),
                "state": state,
            }))
            .unwrap(),
        )
        .unwrap();
    db.flush().unwrap();

    let store = SledTaskNetworkStore::open(db, "network-docs").unwrap();

    assert_eq!(store.state().revision, 1);
    assert!(store.state().tasks.contains_key("task-alpha"));
    assert_eq!(store.journal().len(), 1);
}

#[test]
fn new_durable_record_shapes_omit_duplicate_identity_and_state_fields() {
    let db = open_db();
    {
        let mut store = open_store(&db);
        task_network_support::commit_single_task_sled(&mut store, "task-alpha");
    }
    let command_request_tree = db.open_tree("task_network_command_requests").unwrap();
    let request_value: serde_json::Value = serde_json::from_slice(
        &command_request_tree
            .get("command-commit-task-alpha")
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert!(request_value.get("command_id").is_none());
    assert!(request_value.get("request_hash").is_some());
    assert_eq!(
        request_value["request"]["command_id"],
        json!("command-commit-task-alpha")
    );

    let journal_tree = db.open_tree("task_network_journal_by_revision").unwrap();
    let (_, journal_bytes) = journal_tree.iter().next().unwrap().unwrap();
    let journal_value: serde_json::Value = serde_json::from_slice(&journal_bytes).unwrap();
    assert!(journal_value.get("network_id").is_none());
    assert!(journal_value.get("revision").is_none());
    assert!(journal_value.get("state_hash").is_none());
    assert!(journal_value.get("record").is_some());

    let snapshot_tree = db.open_tree("task_network_latest_state").unwrap();
    let snapshot_value: serde_json::Value =
        serde_json::from_slice(&snapshot_tree.get("latest").unwrap().unwrap()).unwrap();
    assert!(snapshot_value.get("network_id").is_none());
    assert!(snapshot_value.get("revision").is_none());
    assert!(snapshot_value.get("state_hash").is_none());
    assert_eq!(snapshot_value["state"]["network_id"], json!("network-docs"));
}

#[test]
fn corrupt_journal_record_returns_decode_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    db.open_tree("task_network_journal_by_revision")
        .unwrap()
        .insert(1_u64.to_be_bytes(), b"not json".as_slice())
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn corrupt_command_request_record_returns_decode_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    db.open_tree("task_network_command_requests")
        .unwrap()
        .insert("command-a", b"not json".as_slice())
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn corrupt_command_response_record_returns_decode_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    db.open_tree("task_network_command_responses")
        .unwrap()
        .insert("command-a", b"not json".as_slice())
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn command_request_without_response_returns_decode_error_on_open() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    db.open_tree("task_network_command_requests")
        .unwrap()
        .insert(
            "command-a",
            serde_json::to_vec(&stored_request_json("command-a", "hash-a")).unwrap(),
        )
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn command_response_without_request_returns_decode_error_on_open() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    db.open_tree("task_network_command_responses")
        .unwrap()
        .insert(
            "command-a",
            serde_json::to_vec(&stored_response_json("command-a", "hash-a")).unwrap(),
        )
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn request_and_response_hash_mismatch_returns_decode_error_on_open() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    db.open_tree("task_network_command_requests")
        .unwrap()
        .insert(
            "command-a",
            serde_json::to_vec(&stored_request_json("command-a", "hash-a")).unwrap(),
        )
        .unwrap();
    db.open_tree("task_network_command_responses")
        .unwrap()
        .insert(
            "command-a",
            serde_json::to_vec(&stored_response_json("command-a", "hash-b")).unwrap(),
        )
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn corrupt_latest_snapshot_returns_decode_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    db.open_tree("task_network_latest_state")
        .unwrap()
        .insert("latest", b"not json".as_slice())
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn latest_snapshot_hash_mismatch_returns_decode_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    let state = meld_execution::task_network::state::NetworkState::empty("network-docs");
    let snapshot = json!({
        "network_id": "network-docs",
        "revision": 0,
        "state_hash": "wrong",
        "state": state,
    });
    db.open_tree("task_network_latest_state")
        .unwrap()
        .insert("latest", serde_json::to_vec(&snapshot).unwrap())
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn latest_snapshot_top_level_network_mismatch_returns_decode_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    let state = meld_execution::task_network::state::NetworkState::empty("network-docs");
    let snapshot = json!({
        "network_id": "other-network",
        "revision": 0,
        "state_hash": state.state_hash,
        "state": state,
    });
    db.open_tree("task_network_latest_state")
        .unwrap()
        .insert("latest", serde_json::to_vec(&snapshot).unwrap())
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn latest_snapshot_top_level_revision_mismatch_returns_decode_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    let state = meld_execution::task_network::state::NetworkState::empty("network-docs");
    let snapshot = json!({
        "network_id": "network-docs",
        "revision": 1,
        "state_hash": state.state_hash,
        "state": state,
    });
    db.open_tree("task_network_latest_state")
        .unwrap()
        .insert("latest", serde_json::to_vec(&snapshot).unwrap())
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn latest_snapshot_nested_state_hash_mismatch_returns_decode_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    let state = meld_execution::task_network::state::NetworkState::empty("network-docs");
    let mut nested_state = serde_json::to_value(&state).unwrap();
    nested_state["state_hash"] = json!("wrong");
    let snapshot = json!({
        "network_id": "network-docs",
        "revision": 0,
        "state_hash": state.state_hash,
        "state": nested_state,
    });
    db.open_tree("task_network_latest_state")
        .unwrap()
        .insert("latest", serde_json::to_vec(&snapshot).unwrap())
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn journal_revision_gap_returns_decode_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    let state_hash = "unused-because-gap-is-checked-first".to_string();
    db.open_tree("task_network_journal_by_revision")
        .unwrap()
        .insert(
            2_u64.to_be_bytes(),
            serde_json::to_vec(&committed_journal_json(2, state_hash)).unwrap(),
        )
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn journal_replay_state_hash_mismatch_returns_decode_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    db.open_tree("task_network_journal_by_revision")
        .unwrap()
        .insert(
            1_u64.to_be_bytes(),
            serde_json::to_vec(&committed_journal_json(1, "wrong".to_string())).unwrap(),
        )
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn journal_replay_commit_record_network_mismatch_returns_decode_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    let mut memory = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut memory, "task-alpha");
    let mut record = serde_json::to_value(&memory.journal()[0]).unwrap();
    record["Commit"]["network_id"] = json!("other-network");
    let stored = json!({
        "network_id": "network-docs",
        "revision": 1,
        "state_hash": memory.state().state_hash,
        "record": record,
    });
    db.open_tree("task_network_journal_by_revision")
        .unwrap()
        .insert(1_u64.to_be_bytes(), serde_json::to_vec(&stored).unwrap())
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn journal_replay_commit_previous_state_hash_mismatch_returns_decode_error() {
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    let mut memory = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut memory, "task-alpha");
    let mut record = serde_json::to_value(&memory.journal()[0]).unwrap();
    record["Commit"]["previous_state_hash"] = json!("wrong");
    let stored = json!({
        "network_id": "network-docs",
        "revision": 1,
        "state_hash": memory.state().state_hash,
        "record": record,
    });
    db.open_tree("task_network_journal_by_revision")
        .unwrap()
        .insert(1_u64.to_be_bytes(), serde_json::to_vec(&stored).unwrap())
        .unwrap();
    db.flush().unwrap();

    assert_decode_error(SledTaskNetworkStore::open(db, "network-docs").unwrap_err());
}

#[test]
fn missing_latest_snapshot_still_opens_from_journal_replay() {
    let db = open_db();
    {
        let mut store = open_store(&db);
        task_network_support::commit_single_task_sled(&mut store, "task-alpha");
    }
    db.open_tree("task_network_latest_state")
        .unwrap()
        .remove("latest")
        .unwrap();
    db.flush().unwrap();

    let store = open_store(&db);

    assert_eq!(store.state().revision, 1);
    assert!(store.state().tasks.contains_key("task-alpha"));
}

#[test]
fn store_can_open_on_caller_supplied_path_outside_workspace() {
    let workspace = tempfile::tempdir().unwrap();
    let storage = tempfile::tempdir().unwrap();
    assert!(!storage.path().starts_with(workspace.path()));

    let db = sled::open(storage.path().join("task-network")).unwrap();
    let mut store = SledTaskNetworkStore::open(db, "network-docs").unwrap();
    task_network_support::commit_single_task_sled(&mut store, "task-alpha");
    store.flush().unwrap();
}

proptest! {
    #[test]
    fn sled_reducer_matches_memory_over_accepted_command_order(count in 1usize..=8) {
        let mut memory = InMemoryTaskNetworkStore::new("network-docs");
        let db = sled::Config::new().temporary(true).open().unwrap();
        let mut sled_store = SledTaskNetworkStore::open(db, "network-docs").unwrap();

        for index in 0..count {
            let task_id = format!("task-{index}");
            let request = task_network_support::apply_memory_command(
                &memory,
                &format!("command-{task_id}"),
                Command::ApplyMutationSet(task_network_support::single_task_mutation_set(&task_id)),
            );
            memory.submit(request.clone());
            sled_store.submit(request).unwrap();
        }
        sled_store.flush().unwrap();
        prop_assert_eq!(sled_store.state().revision, memory.state().revision);
        prop_assert_eq!(&sled_store.state().state_hash, &memory.state().state_hash);
        prop_assert_eq!(&sled_store.state().tasks, &memory.state().tasks);
        prop_assert_eq!(&sled_store.state().statuses, &memory.state().statuses);
        prop_assert_eq!(sled_store.journal().len(), memory.journal().len());
    }

    #[test]
    fn interleaved_stale_proposal_can_be_rebased(a in 0usize..100, b in 0usize..100) {
        prop_assume!(a != b);
        let mut store = InMemoryTaskNetworkStore::new("network-docs");
        let first_id = format!("task-{a}");
        let second_id = format!("task-{b}");
        let first = task_network_support::apply_memory_command(
            &store,
            "command-first",
            Command::ApplyMutationSet(task_network_support::single_task_mutation_set(&first_id)),
        );
        let stale_second = task_network_support::apply_memory_command(
            &store,
            "command-second",
            Command::ApplyMutationSet(task_network_support::single_task_mutation_set(&second_id)),
        );
        let first_accepted = matches!(store.submit(first), Response::Accepted { .. });
        prop_assert_eq!(first_accepted, true);
        let state_hash = store.state().state_hash.clone();
        let stale_rejected = matches!(
            store.submit(stale_second),
            Response::Rejected(Rejection::StaleBase { .. })
        );
        prop_assert_eq!(stale_rejected, true);
        prop_assert_eq!(&store.state().state_hash, &state_hash);

        let rebased = task_network_support::apply_memory_command(
            &store,
            "command-second-rebased",
            Command::ApplyMutationSet(task_network_support::single_task_mutation_set(&second_id)),
        );
        let rebase_accepted = matches!(store.submit(rebased), Response::Accepted { .. });
        prop_assert_eq!(rebase_accepted, true);
        prop_assert!(store.state().tasks.contains_key(&first_id));
        prop_assert!(store.state().tasks.contains_key(&second_id));
    }

    #[test]
    fn rejected_command_replays_after_reopen(base_revision in 1u64..100) {
        let db = open_db();
        let request;
        {
            let mut store = open_store(&db);
            request = task_network_support::command_for_state(
                "network-docs",
                base_revision,
                &store.state().state_hash,
                "command-stale",
                Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-alpha")),
            );
            let rejected = matches!(
                store.submit(request.clone()).unwrap(),
                Response::Rejected(Rejection::StaleBase { .. })
            );
            prop_assert_eq!(rejected, true);
            store.flush().unwrap();
            drop(store);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let mut store = open_store(&db);
        let duplicate = store.submit(request).unwrap();
        let duplicate_rejected = matches!(
            duplicate,
            Response::Rejected(Rejection::StaleBase { .. })
        );
        prop_assert_eq!(duplicate_rejected, true);
        prop_assert_eq!(store.state().revision, 0);
        store.flush().unwrap();
        drop(store);
    }
}
