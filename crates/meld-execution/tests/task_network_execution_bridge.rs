#[path = "support/task_network.rs"]
mod task_network_support;

use meld_execution::task_network::command::Command;
use meld_execution::task_network::outcome::PublicationState;
use meld_execution::task_network::readiness::compute_ready_set;
use meld_execution::task_network::state::TaskStatus;

#[test]
fn phase7_task_network_slice_survives_reopen() {
    let plan = task_network_support::lower();
    let tempdir = tempfile::tempdir().unwrap();
    let db = sled::open(tempdir.path()).unwrap();
    let mut store =
        meld_execution::task_network::store::SledTaskNetworkStore::open(db, "network-docs")
            .unwrap();
    let commit = task_network_support::apply_sled_command(
        &store,
        "command-commit",
        Command::ApplyMutationSet(plan.mutations),
    );
    store.submit(commit).unwrap();

    let task_instance_id =
        task_network_support::claim_ready_sled(&mut store, "command-claim", "claim-docs");
    let claim = store.state().claims.get("claim-docs").unwrap().clone();
    let outcome =
        task_network_support::outcome_for_claim("outcome-docs", &task_instance_id, &claim);
    let outcome_command = task_network_support::apply_sled_command(
        &store,
        "command-outcome",
        Command::RecordTaskOutcome(outcome),
    );
    store.submit(outcome_command).unwrap();

    let mut publication = store.state().publications.values().next().unwrap().clone();
    publication.state = PublicationState::Published { marked_revision: 0 };
    let publication_id = publication.publication_id.clone();
    let mark_command = task_network_support::apply_sled_command(
        &store,
        "command-mark-publication",
        Command::MarkPublication(publication),
    );
    store.submit(mark_command).unwrap();
    drop(store);

    let db = sled::open(tempdir.path()).unwrap();
    let store = meld_execution::task_network::store::SledTaskNetworkStore::open(db, "network-docs")
        .unwrap();
    assert_eq!(store.state().revision, 4);
    assert!(matches!(
        store.state().statuses.get(&task_instance_id),
        Some(TaskStatus::Succeeded { outcome_id }) if outcome_id == "outcome-docs"
    ));
    assert!(matches!(
        store.state().publications.get(&publication_id).unwrap().state,
        PublicationState::Published { marked_revision } if marked_revision == 4
    ));
    assert_eq!(store.journal().len(), 4);
    assert!(compute_ready_set(store.state())
        .task_instance_ids
        .is_empty());
}
