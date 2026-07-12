#[path = "support/task_network.rs"]
mod task_network_support;

use meld_events::{AppendDisposition, AppendReceipt, LedgerIdentity};
use meld_execution::task_network::command::{Command, Response};
use meld_execution::task_network::mutation::{ReadPrecondition, Rejection};
use meld_execution::task_network::outcome::PublicationState;

fn canonical_receipt(seq: u64) -> AppendReceipt {
    AppendReceipt {
        ledger_id: LedgerIdentity::new(),
        seq,
        disposition: AppendDisposition::Inserted,
    }
}

#[test]
fn task_outcome_creates_pending_publication_before_external_publication() {
    let (store, task_instance_id, publication_id) =
        task_network_support::memory_store_with_pending_publication();
    let publication = store.state().publications.get(&publication_id).unwrap();

    assert_eq!(publication.outcome.task_instance_id, task_instance_id);
    assert!(matches!(publication.state, PublicationState::Pending));
    assert_eq!(publication.event_type(), "execution.task.succeeded");
}

#[test]
fn new_publication_mark_rejects_receipt_less_legacy_state() {
    let (mut store, _, publication_id) =
        task_network_support::memory_store_with_pending_publication();
    let mut publication = store.state().publications[&publication_id].clone();
    publication.state = PublicationState::Published {
        marked_revision: 0,
        receipt: None,
        legacy_event_seq: Some(7),
    };
    let request = task_network_support::apply_memory_command(
        &store,
        "command-attempt-new-legacy-publication",
        Command::MarkPublication(publication),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::InvalidLifecycleTransition(message))
            if message.contains("requires a canonical append receipt")
    ));
    assert!(matches!(
        store.state().publications[&publication_id].state,
        PublicationState::Pending
    ));
}

#[test]
fn publication_mark_preserves_immutable_payload_fields() {
    let (mut store, _, publication_id) =
        task_network_support::memory_store_with_pending_publication();
    let original = store
        .state()
        .publications
        .get(&publication_id)
        .unwrap()
        .clone();
    let mut publication = original.clone();
    publication.state = PublicationState::Published {
        marked_revision: 0,
        receipt: Some(canonical_receipt(1)),
        legacy_event_seq: None,
    };
    let request = task_network_support::apply_memory_command(
        &store,
        "command-mark-publication",
        Command::MarkPublication(publication),
    );

    assert!(matches!(store.submit(request), Response::Accepted { .. }));
    let marked = store.state().publications.get(&publication_id).unwrap();
    assert_eq!(marked.event_type(), original.event_type());
    assert_eq!(marked.event_payload(), original.event_payload());
}

#[test]
fn tampered_publication_outcome_rejects() {
    let (mut store, _, publication_id) =
        task_network_support::memory_store_with_pending_publication();
    let mut publication = store
        .state()
        .publications
        .get(&publication_id)
        .unwrap()
        .clone();
    publication.outcome.error = Some("tampered".to_string());
    publication.state = PublicationState::Published {
        marked_revision: 0,
        receipt: Some(canonical_receipt(1)),
        legacy_event_seq: None,
    };
    let request = task_network_support::apply_memory_command(
        &store,
        "command-mark-publication",
        Command::MarkPublication(publication),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::InvalidLifecycleTransition(_))
    ));
}

#[test]
fn tampered_publication_network_rejects() {
    let (mut store, _, publication_id) =
        task_network_support::memory_store_with_pending_publication();
    let mut publication = store
        .state()
        .publications
        .get(&publication_id)
        .unwrap()
        .clone();
    publication.network_id = "other-network".to_string();
    publication.state = PublicationState::Published {
        marked_revision: 0,
        receipt: Some(canonical_receipt(1)),
        legacy_event_seq: None,
    };
    let request = task_network_support::apply_memory_command(
        &store,
        "command-mark-wrong-network-publication",
        Command::MarkPublication(publication),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::InvalidLifecycleTransition(_))
    ));
}

#[test]
fn tampered_publication_task_identity_rejects() {
    let (mut store, _, publication_id) =
        task_network_support::memory_store_with_pending_publication();
    let mut publication = store
        .state()
        .publications
        .get(&publication_id)
        .unwrap()
        .clone();
    publication.outcome.task_instance_id = "other-task".to_string();
    publication.state = PublicationState::Published {
        marked_revision: 0,
        receipt: Some(canonical_receipt(1)),
        legacy_event_seq: None,
    };
    let request = task_network_support::apply_memory_command(
        &store,
        "command-mark-wrong-task-publication",
        Command::MarkPublication(publication),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::InvalidLifecycleTransition(_))
    ));
}

#[test]
fn publish_mark_sets_current_revision() {
    let (mut store, _, publication_id) =
        task_network_support::memory_store_with_pending_publication();
    let mut publication = store
        .state()
        .publications
        .get(&publication_id)
        .unwrap()
        .clone();
    publication.state = PublicationState::Published {
        marked_revision: 0,
        receipt: Some(canonical_receipt(1)),
        legacy_event_seq: None,
    };
    let request = task_network_support::apply_memory_command(
        &store,
        "command-mark-publication",
        Command::MarkPublication(publication),
    );

    assert!(matches!(store.submit(request), Response::Accepted { .. }));
    let marked = store.state().publications.get(&publication_id).unwrap();
    assert!(matches!(
        marked.state,
        PublicationState::Published { marked_revision, .. } if marked_revision == store.state().revision
    ));
}

#[test]
fn failure_mark_preserves_payload_and_stores_error() {
    let (mut store, _, publication_id) =
        task_network_support::memory_store_with_pending_publication();
    let original = store
        .state()
        .publications
        .get(&publication_id)
        .unwrap()
        .clone();
    let mut publication = original.clone();
    publication.state = PublicationState::Failed {
        error: "append failed".to_string(),
    };
    let request = task_network_support::apply_memory_command(
        &store,
        "command-fail-publication",
        Command::MarkPublication(publication),
    );

    assert!(matches!(store.submit(request), Response::Accepted { .. }));
    let marked = store.state().publications.get(&publication_id).unwrap();
    assert_eq!(marked.event_payload(), original.event_payload());
    assert!(matches!(
        &marked.state,
        PublicationState::Failed { error } if error == "append failed"
    ));
}

#[test]
fn second_publication_mark_rejects_with_publication_already_marked() {
    let (mut store, _, publication_id) =
        task_network_support::memory_store_with_pending_publication();
    let mut publication = store
        .state()
        .publications
        .get(&publication_id)
        .unwrap()
        .clone();
    publication.state = PublicationState::Published {
        marked_revision: 0,
        receipt: Some(canonical_receipt(1)),
        legacy_event_seq: None,
    };
    let request = task_network_support::apply_memory_command(
        &store,
        "command-mark-publication",
        Command::MarkPublication(publication.clone()),
    );
    store.submit(request);
    let request = task_network_support::apply_memory_command(
        &store,
        "command-mark-publication-again",
        Command::MarkPublication(publication),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::PublicationAlreadyMarked(id)) if id == publication_id
    ));
}

#[test]
fn marking_unknown_publication_rejects_with_failed_precondition() {
    let (mut store, _, _) = task_network_support::memory_store_with_pending_publication();
    let mut publication = store.state().publications.values().next().unwrap().clone();
    publication.publication_id = "missing-publication".to_string();
    publication.state = PublicationState::Published {
        marked_revision: 0,
        receipt: Some(canonical_receipt(1)),
        legacy_event_seq: None,
    };
    let request = task_network_support::apply_memory_command(
        &store,
        "command-missing-publication",
        Command::MarkPublication(publication),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::FailedPrecondition(
            ReadPrecondition::PublicationPending(id)
        )) if id == "missing-publication"
    ));
}

#[test]
fn pending_publication_mark_rejects_because_mark_must_be_terminal() {
    let (mut store, _, _) = task_network_support::memory_store_with_pending_publication();
    let publication = store.state().publications.values().next().unwrap().clone();
    let request = task_network_support::apply_memory_command(
        &store,
        "command-pending-publication",
        Command::MarkPublication(publication),
    );

    assert!(matches!(
        store.submit(request),
        Response::Rejected(Rejection::InvalidLifecycleTransition(_))
    ));
}
