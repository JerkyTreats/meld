use super::*;
use crate::task_network::mutation::Set;

const LIFECYCLE_TREE: &str = "task_network_authority_lifecycle";
const EPOCH_KEY: &[u8] = b"epoch";

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
        Err(TaskNetworkAuthorityError::Poisoned { .. })
    ));
    let receipt = authority.shutdown().unwrap();
    assert!(receipt.was_poisoned);
    assert_eq!(receipt.final_revision, 0);
    drop(authority);
    drop(db);

    let factory = TaskNetworkStoreFactory::new(temp.path());
    let mut reopened = TaskNetworkAuthority::open(&factory, "network-docs", 4).unwrap();
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

    assert!(matches!(
        authority
            .command_port()
            .try_submit(empty_command("command-corrupt")),
        Err(TaskNetworkAuthorityError::Poisoned { .. })
    ));
    assert!(matches!(
        authority
            .command_port()
            .try_submit(empty_command("command-after-poison")),
        Err(TaskNetworkAuthorityError::Poisoned { .. })
    ));
    let receipt = authority.shutdown().unwrap();
    assert!(receipt.was_poisoned);
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
