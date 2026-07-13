#[path = "support/task_network.rs"]
mod task_network_support;

use std::sync::{Arc, Barrier};

use meld_execution::task_network::authority::{
    TaskNetworkAuthorities, TaskNetworkAuthority, TaskNetworkAuthorityError,
    TaskNetworkAuthorityLifecycle,
};
use meld_execution::task_network::command::{Command, Response};
use meld_execution::task_network::store::TaskNetworkStoreFactory;

fn command_for(
    state: &meld_execution::task_network::NetworkState,
    command_id: &str,
) -> meld_execution::task_network::CommandRequest {
    task_network_support::command_for_state(
        &state.network_id,
        state.revision,
        &state.state_hash,
        command_id,
        Command::ApplyMutationSet(task_network_support::single_task_mutation_set("task-alpha")),
    )
}

#[test]
fn authority_acknowledges_only_durable_state_and_closes_both_ports() {
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    let mut authority = TaskNetworkAuthority::open(&factory, "network-docs", 8).unwrap();
    let command_port = authority.command_port();
    let query_port = authority.query_port();
    let initial = query_port.state().unwrap();
    assert!(query_port.ready_set().unwrap().task_instance_ids.is_empty());
    assert!(query_port
        .publication("publication-missing")
        .unwrap()
        .is_none());
    assert!(query_port.retryable_publications(8).unwrap().is_empty());

    let response = command_port
        .try_submit(command_for(&initial, "command-alpha"))
        .unwrap();

    assert!(matches!(response, Response::Accepted { revision: 1, .. }));
    let durable = query_port.state().unwrap();
    assert_eq!(durable.revision, 1);
    assert!(durable.tasks.contains_key("task-alpha"));
    assert_eq!(query_port.journal().unwrap().len(), 1);

    let receipt = authority.shutdown().unwrap();
    assert_eq!(receipt.final_revision, 1);
    assert_eq!(receipt.final_state_hash, durable.state_hash);
    assert_eq!(receipt.journal_records, 1);
    assert!(!receipt.was_poisoned);
    assert_eq!(
        command_port.lifecycle().lifecycle,
        TaskNetworkAuthorityLifecycle::Closed
    );
    assert!(matches!(
        query_port.state(),
        Err(TaskNetworkAuthorityError::Closed { .. })
    ));
    assert!(matches!(
        command_port.try_submit(command_for(&durable, "command-after-close")),
        Err(TaskNetworkAuthorityError::Closed { .. })
    ));
}

#[test]
fn authority_reopen_recovers_state_and_advances_durable_epoch() {
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    let first_epoch;
    {
        let mut authority = TaskNetworkAuthority::open(&factory, "network-docs", 4).unwrap();
        first_epoch = authority.lifecycle().epoch;
        let query = authority.query_port();
        let state = query.state().unwrap();
        authority
            .command_port()
            .try_submit(command_for(&state, "command-alpha"))
            .unwrap();
        authority.shutdown().unwrap();
    }

    let mut reopened = TaskNetworkAuthority::open(&factory, "network-docs", 4).unwrap();
    let query = reopened.query_port();
    let recovered = query.state().unwrap();

    assert_eq!(reopened.lifecycle().epoch, first_epoch + 1);
    assert_eq!(recovered.revision, 1);
    assert!(recovered.tasks.contains_key("task-alpha"));
    assert_eq!(query.journal().unwrap().len(), 1);
    assert!(matches!(
        reopened
            .command_port()
            .try_submit(command_for(
                &meld_execution::task_network::NetworkState::empty("network-docs"),
                "command-alpha",
            ))
            .unwrap(),
        Response::Duplicate { revision: 1, .. }
    ));
    reopened.shutdown().unwrap();
}

#[test]
fn configured_authority_set_rejects_duplicates_and_isolates_networks() {
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    let duplicate = TaskNetworkAuthorities::open(
        &factory,
        vec!["network-a".to_string(), "network-a".to_string()],
        4,
    );
    assert!(matches!(
        duplicate,
        Err(TaskNetworkAuthorityError::DuplicateNetwork(network_id))
            if network_id == "network-a"
    ));

    let mut authorities = TaskNetworkAuthorities::open(
        &factory,
        vec!["network-a".to_string(), "network-b".to_string()],
        4,
    )
    .unwrap();

    assert_eq!(authorities.len(), 2);
    assert!(!authorities.is_empty());
    assert_eq!(
        authorities
            .query_port("network-a")
            .unwrap()
            .state()
            .unwrap()
            .network_id,
        "network-a"
    );
    assert_eq!(
        authorities
            .query_port("network-b")
            .unwrap()
            .state()
            .unwrap()
            .network_id,
        "network-b"
    );
    assert!(authorities.command_port("network-missing").is_none());
    let receipts = authorities.shutdown_all().unwrap();
    assert_eq!(receipts.len(), 2);
}

#[test]
fn bounded_mailbox_reports_saturation_under_concurrent_admission() {
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    let mut authority = TaskNetworkAuthority::open(&factory, "network-docs", 1).unwrap();
    let initial = authority.query_port().state().unwrap();
    let workers = 64;
    let barrier = Arc::new(Barrier::new(workers + 1));
    let mut joins = Vec::new();

    for index in 0..workers {
        let port = authority.command_port();
        let barrier = Arc::clone(&barrier);
        let mut request = command_for(&initial, &format!("command-{index}"));
        request.command =
            Command::ApplyMutationSet(meld_execution::task_network::mutation::Set::empty(
                "network-docs",
                format!("composition-{index}"),
                format!("once-{index}"),
            ));
        joins.push(std::thread::spawn(move || {
            barrier.wait();
            port.try_submit(request)
        }));
    }
    barrier.wait();

    let results = joins
        .into_iter()
        .map(|join| join.join().unwrap())
        .collect::<Vec<_>>();
    assert!(results.iter().any(|result| {
        matches!(
            result,
            Err(TaskNetworkAuthorityError::Full { capacity: 1, .. })
        )
    }));
    assert!(results.iter().any(Result::is_ok));
    authority.shutdown().unwrap();
}

#[test]
fn shutdown_during_saturation_drains_admitted_commands_and_reopens_without_loss() {
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    let mut authority = TaskNetworkAuthority::open(&factory, "network-docs", 1).unwrap();
    let initial = authority.query_port().state().unwrap();
    let baseline_request = command_for(&initial, "command-baseline");
    assert!(matches!(
        authority
            .command_port()
            .try_submit(baseline_request.clone())
            .unwrap(),
        Response::Accepted { revision: 1, .. }
    ));
    let concurrent_base = authority.query_port().state().unwrap();
    let workers = 64;
    let barrier = Arc::new(Barrier::new(workers + 2));
    let mut joins = Vec::new();

    for index in 0..workers {
        let port = authority.command_port();
        let barrier = Arc::clone(&barrier);
        let mut request = command_for(&concurrent_base, &format!("command-racing-{index}"));
        request.command =
            Command::ApplyMutationSet(meld_execution::task_network::mutation::Set::empty(
                "network-docs",
                format!("composition-racing-{index}"),
                format!("once-racing-{index}"),
            ));
        joins.push(std::thread::spawn(move || {
            barrier.wait();
            let result = port.try_submit(request.clone());
            (request, result)
        }));
    }

    let shutdown_barrier = Arc::clone(&barrier);
    let shutdown = std::thread::spawn(move || {
        shutdown_barrier.wait();
        authority.shutdown().unwrap()
    });
    barrier.wait();

    let outcomes = joins
        .into_iter()
        .map(|join| join.join().unwrap())
        .collect::<Vec<_>>();
    let receipt = shutdown.join().unwrap();
    assert!(outcomes.iter().all(|(_, result)| {
        result.is_ok()
            || matches!(
                result,
                Err(TaskNetworkAuthorityError::Full { .. })
                    | Err(TaskNetworkAuthorityError::Closing { .. })
                    | Err(TaskNetworkAuthorityError::Closed { .. })
            )
    }));

    let mut accepted_revisions = outcomes
        .iter()
        .filter_map(|(_, result)| match result {
            Ok(Response::Accepted { revision, .. }) => Some(*revision),
            _ => None,
        })
        .collect::<Vec<_>>();
    accepted_revisions.push(1);
    accepted_revisions.sort_unstable();
    assert_eq!(
        accepted_revisions,
        (1..=receipt.final_revision).collect::<Vec<_>>()
    );
    assert_eq!(receipt.journal_records as u64, receipt.final_revision);

    let mut reopened = TaskNetworkAuthority::open(&factory, "network-docs", 1).unwrap();
    assert_eq!(
        reopened.query_port().journal().unwrap().len(),
        receipt.journal_records
    );
    assert_eq!(
        reopened.query_port().state().unwrap().state_hash,
        receipt.final_state_hash
    );
    assert!(matches!(
        reopened
            .command_port()
            .try_submit(baseline_request)
            .unwrap(),
        Response::Duplicate { revision: 1, .. }
    ));
    for (request, result) in outcomes {
        let Ok(original) = result else {
            continue;
        };
        let replay = reopened.command_port().try_submit(request).unwrap();
        match original {
            Response::Accepted {
                revision,
                state_hash,
            } => assert_eq!(
                replay,
                Response::Duplicate {
                    revision,
                    state_hash
                }
            ),
            Response::Rejected(rejection) => {
                assert_eq!(replay, Response::Rejected(rejection));
            }
            Response::Duplicate { .. } => panic!("new command returned duplicate"),
        }
    }
    reopened.shutdown().unwrap();
}

#[test]
fn zero_capacity_is_rejected_before_store_open() {
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());

    let error = TaskNetworkAuthority::open(&factory, "network-docs", 0)
        .err()
        .expect("zero capacity must fail");

    assert!(matches!(
        error,
        TaskNetworkAuthorityError::InvalidConfiguration(_)
    ));
    assert!(!temp.path().join("network-docs.sled").exists());
}
