#[path = "support/task_network.rs"]
mod task_network_support;

use meld_execution::task_network::command::Command;
use meld_execution::task_network::dispatch::Claim;
use meld_execution::task_network::journal::JournalRecord;
use meld_execution::task_network::mutation::{CommitRecord, Set};
use meld_execution::task_network::outcome::Publication;
use meld_execution::task_network::state::NetworkState;
use meld_execution::task_network::store::InMemoryTaskNetworkStore;

fn fixture(path: &str) -> serde_json::Value {
    serde_json::from_str(path).unwrap()
}

fn assert_fixture<T>(raw: &str, expected: &T)
where
    T: serde::de::DeserializeOwned + serde::Serialize + PartialEq + std::fmt::Debug,
{
    let fixture_value: serde_json::Value = fixture(raw);
    let decoded: T = serde_json::from_value(fixture_value.clone()).unwrap();
    assert_eq!(&decoded, expected);
    let encoded = serde_json::to_value(&decoded).unwrap();
    assert_eq!(fixture_value, encoded);
}

fn expected_commit_state_and_journal() -> (CommitRecord, NetworkState, JournalRecord) {
    let mutation_set = task_network_support::single_task_mutation_set("task-alpha");
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    let request = task_network_support::apply_memory_command(
        &store,
        "command-commit-task-alpha",
        Command::ApplyMutationSet(mutation_set),
    );
    store.submit(request);
    let journal_record = store.journal()[0].clone();
    let JournalRecord::Commit(commit_record) = journal_record.clone() else {
        unreachable!();
    };
    (commit_record, store.state().clone(), journal_record)
}

fn expected_claim_and_publication() -> (Claim, Publication) {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    task_network_support::commit_single_task(&mut store, "task-alpha");
    task_network_support::claim_ready_memory(&mut store, "command-claim", "claim-alpha");
    let claim = store.state().claims.get("claim-alpha").unwrap().clone();
    let outcome = task_network_support::outcome_for_claim("outcome-alpha", "task-alpha", &claim);
    let request = task_network_support::apply_memory_command(
        &store,
        "command-outcome",
        Command::RecordTaskOutcome(outcome),
    );
    store.submit(request);
    let publication = store.state().publications.values().next().unwrap().clone();
    (claim, publication)
}

#[test]
fn mutation_set_fixture_round_trips() {
    let expected = task_network_support::single_task_mutation_set("task-alpha");
    assert_fixture::<Set>(
        include_str!("fixtures/task_network/mutation_set_v1.json"),
        &expected,
    );
}

#[test]
fn commit_record_fixture_round_trips() {
    let (expected, _, _) = expected_commit_state_and_journal();
    assert_fixture::<CommitRecord>(
        include_str!("fixtures/task_network/commit_record_v1.json"),
        &expected,
    );
}

#[test]
fn network_state_fixture_round_trips() {
    let (_, expected, _) = expected_commit_state_and_journal();
    assert_fixture::<NetworkState>(
        include_str!("fixtures/task_network/network_state_v1.json"),
        &expected,
    );
}

#[test]
fn dispatch_claim_fixture_round_trips() {
    let (expected, _) = expected_claim_and_publication();
    assert_fixture::<Claim>(
        include_str!("fixtures/task_network/dispatch_claim_v1.json"),
        &expected,
    );
}

#[test]
fn publication_fixture_round_trips() {
    let (_, expected) = expected_claim_and_publication();
    assert_fixture::<Publication>(
        include_str!("fixtures/task_network/publication_v1.json"),
        &expected,
    );
}

#[test]
fn journal_record_fixture_round_trips() {
    let (_, _, expected) = expected_commit_state_and_journal();
    assert_fixture::<JournalRecord>(
        include_str!("fixtures/task_network/journal_record_v1.json"),
        &expected,
    );
}
