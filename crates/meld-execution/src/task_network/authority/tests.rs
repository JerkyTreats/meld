use super::*;
use crate::task_network::mutation::Set;
use crate::task_network::store::InMemoryTaskNetworkStore;
use serde_json::json;

const LIFECYCLE_TREE: &str = "task_network_authority_lifecycle";
const EPOCH_KEY: &[u8] = b"epoch";
const RESPONSE_TREE: &str = "task_network_command_responses";

fn empty_command(command_id: &str) -> command::Request {
    let state = NetworkState::empty("network-docs");
    command::Request {
        command_id: command_id.to_string(),
        network_id: state.network_id.clone(),
        base_revision: state.revision,
        base_state_hash: state.state_hash,
        read_preconditions: Vec::new(),
        command: command::Command::ApplyMutationSet(Set::empty(
            "network-docs",
            "composition-docs",
            "once",
        )),
    }
}

fn spawn_with_retained_db() -> (sled::Db, tempfile::TempDir, TaskNetworkAuthority) {
    let temp = tempfile::tempdir().unwrap();
    let db = sled::open(temp.path()).unwrap();
    let store = SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap();
    let authority =
        TaskNetworkAuthority::spawn_store(store, "network-docs".to_string(), 4).unwrap();
    (db, temp, authority)
}

#[test]
fn stale_epoch_poisons_worker_without_advancing_semantic_state() {
    let (db, temp, mut authority) = spawn_with_retained_db();
    let epoch = authority.lifecycle().epoch;
    db.open_tree(LIFECYCLE_TREE)
        .unwrap()
        .insert(EPOCH_KEY, (epoch + 1).to_be_bytes().as_slice())
        .unwrap();
    db.flush().unwrap();

    let error = authority
        .command_port()
        .try_submit(empty_command("command-stale"))
        .unwrap_err();

    assert!(matches!(
        error,
        TaskNetworkAuthorityError::StaleEpoch {
            expected,
            actual,
            ..
        } if expected == epoch && actual == epoch + 1
    ));
    assert_eq!(
        authority.lifecycle().lifecycle,
        TaskNetworkAuthorityLifecycle::Poisoned
    );
    assert!(matches!(
        authority.query_port().state(),
        Err(TaskNetworkAuthorityError::Poisoned {
            kind: TaskNetworkAuthorityPoisonKind::StaleEpoch,
            ..
        })
    ));
    let receipt = authority.shutdown().unwrap();
    assert!(receipt.was_poisoned);
    assert_eq!(receipt.final_revision, 0);
    drop(authority);
    drop(db);

    let reopened_db = sled::open(temp.path()).unwrap();
    let reopened_store = SledTaskNetworkStore::open(reopened_db.clone(), "network-docs").unwrap();
    let mut reopened =
        TaskNetworkAuthority::spawn_store(reopened_store, "network-docs".to_string(), 4).unwrap();
    assert_eq!(reopened.query_port().state().unwrap().revision, 0);
    reopened.shutdown().unwrap();
}

#[test]
fn malformed_durable_epoch_poisoning_is_sticky_until_shutdown() {
    let (db, _temp, mut authority) = spawn_with_retained_db();
    db.open_tree(LIFECYCLE_TREE)
        .unwrap()
        .insert(EPOCH_KEY, b"bad".as_slice())
        .unwrap();
    db.flush().unwrap();

    for command_id in ["command-corrupt", "command-after-poison"] {
        let error = authority
            .command_port()
            .try_submit(empty_command(command_id))
            .unwrap_err();
        assert!(matches!(
            &error,
            TaskNetworkAuthorityError::Poisoned {
                kind: TaskNetworkAuthorityPoisonKind::CorruptState,
                ..
            }
        ));
        if let TaskNetworkAuthorityError::Poisoned { kind, .. } = error {
            assert!(!kind.retryable());
        }
    }
    let receipt = authority.shutdown().unwrap();
    assert!(receipt.was_poisoned);
}

#[test]
fn outcome_query_rejects_reauthenticated_external_response_substitution() {
    let (db, _temp, mut authority) = spawn_with_retained_db();
    assert!(matches!(
        authority
            .command_port()
            .try_submit(empty_command("command-substituted"))
            .unwrap(),
        command::Response::Accepted { .. }
    ));
    let responses = db.open_tree(RESPONSE_TREE).unwrap();
    let raw = responses.get("command-substituted").unwrap().unwrap();
    let mut stored: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    let substituted = command::Response::Accepted {
        revision: 1,
        state_hash: "externally-substituted-state".to_string(),
    };
    let request_hash = stored["request_hash"].as_str().unwrap().to_string();
    stored["response"] = serde_json::to_value(&substituted).unwrap();
    stored["authentication"] = serde_json::to_value(
        command::ResponseAuthentication::bind("command-substituted", &request_hash, &substituted)
            .unwrap(),
    )
    .unwrap();
    responses
        .insert("command-substituted", serde_json::to_vec(&stored).unwrap())
        .unwrap();
    db.flush().unwrap();

    assert!(matches!(
        authority
            .query_port()
            .command_outcome("command-substituted"),
        Err(TaskNetworkAuthorityError::Poisoned {
            kind: TaskNetworkAuthorityPoisonKind::CorruptState,
            ..
        })
    ));
    assert!(authority.shutdown().unwrap().was_poisoned);
}

#[test]
fn query_port_rejects_a_superseded_durable_epoch() {
    let (db, _temp, mut authority) = spawn_with_retained_db();
    let epoch = authority.lifecycle().epoch;
    db.open_tree(LIFECYCLE_TREE)
        .unwrap()
        .insert(EPOCH_KEY, (epoch + 1).to_be_bytes().as_slice())
        .unwrap();
    db.flush().unwrap();

    assert!(matches!(
        authority.query_port().state(),
        Err(TaskNetworkAuthorityError::StaleEpoch {
            expected,
            actual,
            ..
        }) if expected == epoch && actual == epoch + 1
    ));
    assert!(matches!(
        authority.query_port().journal(),
        Err(TaskNetworkAuthorityError::Poisoned {
            kind: TaskNetworkAuthorityPoisonKind::StaleEpoch,
            ..
        })
    ));
    assert!(authority.shutdown().unwrap().was_poisoned);
}

#[test]
fn command_outcome_query_returns_the_original_exact_receipt_after_reopen() {
    let (db, temp, mut authority) = spawn_with_retained_db();
    let request = empty_command("command-receipt");
    let query = authority.query_port();
    assert!(query
        .command_outcome(&request.command_id)
        .unwrap()
        .is_none());

    let response = authority
        .command_port()
        .try_submit(request.clone())
        .unwrap();
    assert!(matches!(response, command::Response::Accepted { .. }));
    let receipt = query.command_outcome(&request.command_id).unwrap().unwrap();
    assert_eq!(receipt.command_id(), request.command_id.as_str());
    assert_eq!(receipt.request_hash(), command::request_hash(&request));
    assert_eq!(receipt.request(), &request);
    assert_eq!(receipt.response(), &response);
    assert!(matches!(
        authority
            .command_port()
            .try_submit(request.clone())
            .unwrap(),
        command::Response::Duplicate { .. }
    ));
    assert_eq!(
        query.command_outcome(&request.command_id).unwrap(),
        Some(receipt.clone())
    );

    authority.shutdown().unwrap();
    drop(authority);
    drop(db);
    let reopened_db = sled::open(temp.path()).unwrap();
    let reopened_store = SledTaskNetworkStore::open(reopened_db.clone(), "network-docs").unwrap();
    let mut reopened =
        TaskNetworkAuthority::spawn_store(reopened_store, "network-docs".to_string(), 4).unwrap();
    assert_eq!(
        reopened
            .query_port()
            .command_outcome(&request.command_id)
            .unwrap(),
        Some(receipt)
    );
    reopened.shutdown().unwrap();
    drop(reopened_db);
}

#[test]
fn legacy_command_outcome_query_preserves_historical_request_hash() {
    let temp = tempfile::tempdir().unwrap();
    let db = sled::open(temp.path()).unwrap();
    let request = empty_command("command-legacy-receipt");
    let mut memory = InMemoryTaskNetworkStore::new("network-docs");
    let response = memory.submit(request.clone());
    let request_hash = "legacy-arbitrary-request-hash";
    db.open_tree("task_network_command_requests")
        .unwrap()
        .insert(
            request.command_id.as_bytes(),
            serde_json::to_vec(&json!({
                "command_id": request.command_id.clone(),
                "request_hash": request_hash,
                "request": request.clone(),
            }))
            .unwrap(),
        )
        .unwrap();
    db.open_tree("task_network_command_responses")
        .unwrap()
        .insert(
            "command-legacy-receipt",
            serde_json::to_vec(&json!({
                "command_id": "command-legacy-receipt",
                "request_hash": request_hash,
                "response": response.clone(),
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
                "state_hash": memory.state().state_hash.clone(),
                "record": memory.journal()[0].clone(),
            }))
            .unwrap(),
        )
        .unwrap();
    db.flush().unwrap();

    let store = SledTaskNetworkStore::open(db.clone(), "network-docs").unwrap();
    let mut authority =
        TaskNetworkAuthority::spawn_store(store, "network-docs".to_string(), 4).unwrap();
    let receipt = authority
        .query_port()
        .command_outcome("command-legacy-receipt")
        .unwrap()
        .unwrap();
    assert_eq!(receipt.request_hash(), request_hash);
    assert_eq!(receipt.request(), &request);
    assert_eq!(receipt.response(), &response);
    authority.shutdown().unwrap();
}

#[test]
fn command_outcome_query_refuses_a_tampered_response_before_issuance() {
    let (db, _temp, mut authority) = spawn_with_retained_db();
    let request = empty_command("command-tampered-receipt");
    authority
        .command_port()
        .try_submit(request.clone())
        .unwrap();
    let responses = db.open_tree("task_network_command_responses").unwrap();
    let mut stored: serde_json::Value = serde_json::from_slice(
        &responses
            .get(request.command_id.as_bytes())
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    stored["response"]["Accepted"]["state_hash"] = json!("tampered");
    responses
        .insert(
            request.command_id.as_bytes(),
            serde_json::to_vec(&stored).unwrap(),
        )
        .unwrap();
    db.flush().unwrap();

    assert!(matches!(
        authority.query_port().command_outcome(&request.command_id),
        Err(TaskNetworkAuthorityError::Poisoned { .. })
    ));
    assert!(authority.shutdown().unwrap().was_poisoned);
}

#[test]
fn admission_fence_reports_closing_before_ports_close() {
    let shared = Arc::new(SharedLifecycle::new("network-docs".to_string(), 1, 1));
    let (sender, _receiver) = sync_channel(1);
    assert!(!shared.begin_shutdown().unwrap());

    let port = TaskNetworkCommandPort {
        sender,
        shared: Arc::clone(&shared),
    };

    assert_eq!(
        port.lifecycle().lifecycle,
        TaskNetworkAuthorityLifecycle::Closing
    );
    assert!(matches!(
        port.try_submit(empty_command("command-closing")),
        Err(TaskNetworkAuthorityError::Closing { .. })
    ));
}
