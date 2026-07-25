//! Durable consumer cursor authority and genesis identity hygiene.
//!
//! Proves the identity-bound monotonic durable cursor on the consumer
//! registry capability, its separation from the observational mirror, and
//! the append-surface rejection of genesis identities whose segments carry
//! the `::` delimiter.

use meld_events::error::EventAuthorityError;
use meld_events::{
    AppendDisposition, AppendMode, DurableConsumerCursor, EventAuthority,
    EventAuthorityOpenOptions, EventEnvelope, LedgerCursor,
};
use serde_json::json;

fn open_temporary_authority() -> EventAuthority {
    let db = sled::Config::new().temporary(true).open().unwrap();
    EventAuthority::open(db, EventAuthorityOpenOptions::default()).unwrap()
}

#[test]
fn durable_cursor_advances_monotonically_and_no_ops_behind_position() {
    let authority = open_temporary_authority();
    let cursor = authority.consumer_registry_capability();

    assert_eq!(cursor.consumer_cursor("evidence").unwrap(), None);

    let advanced = cursor.advance_consumer_cursor("evidence", 5).unwrap();
    assert_eq!(advanced.after_seq, 5);
    assert_eq!(advanced.ledger_id, authority.ledger_identity());

    // Behind and equal requests are no-ops returning the durable state.
    assert_eq!(
        cursor
            .advance_consumer_cursor("evidence", 3)
            .unwrap()
            .after_seq,
        5
    );
    assert_eq!(
        cursor
            .advance_consumer_cursor("evidence", 5)
            .unwrap()
            .after_seq,
        5
    );

    assert_eq!(
        cursor
            .advance_consumer_cursor("evidence", 9)
            .unwrap()
            .after_seq,
        9
    );
    assert_eq!(
        cursor
            .consumer_cursor("evidence")
            .unwrap()
            .unwrap()
            .after_seq,
        9
    );
}

#[test]
fn durable_cursor_survives_reopen_with_the_same_ledger_identity() {
    let temp = tempfile::tempdir().unwrap();
    let authority = EventAuthority::open(
        sled::open(temp.path()).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let ledger_id = authority.ledger_identity();
    authority
        .consumer_registry_capability()
        .advance_consumer_cursor("evidence", 7)
        .unwrap();
    drop(authority);

    let reopened =
        EventAuthority::open_existing(sled::open(temp.path()).unwrap(), ledger_id).unwrap();
    let state = reopened
        .consumer_registry_capability()
        .consumer_cursor("evidence")
        .unwrap()
        .unwrap();
    assert_eq!(state.after_seq, 7);
    assert_eq!(state.ledger_id, ledger_id);
}

#[test]
fn durable_advancement_mirrors_into_the_observational_position_surface() {
    let authority = open_temporary_authority();
    let registry = authority.consumer_registry_capability();

    registry.advance_consumer_cursor("evidence", 4).unwrap();

    let position = registry.get("evidence").unwrap().unwrap();
    assert_eq!(position.reported_seq, 4);
    assert_eq!(position.ledger_id, authority.ledger_identity());
}

#[test]
fn mirror_reports_never_create_authoritative_durable_state() {
    let authority = open_temporary_authority();
    let registry = authority.consumer_registry_capability();

    registry
        .report(
            "world_state.graph.reducer",
            LedgerCursor {
                ledger_id: authority.ledger_identity(),
                after_seq: 9,
            },
        )
        .unwrap();

    // The mirror row exists, but the durable authority has no cursor for a
    // consumer that never advanced through the durable contract.
    assert!(registry.get("world_state.graph.reducer").unwrap().is_some());
    assert_eq!(
        registry
            .consumer_cursor("world_state.graph.reducer")
            .unwrap(),
        None
    );
}

#[test]
fn empty_consumer_id_is_rejected_without_retry_semantics() {
    let authority = open_temporary_authority();
    let cursor = authority.consumer_registry_capability();

    let error = cursor.advance_consumer_cursor(" ", 1).unwrap_err();
    assert!(!error.retryable);
}

#[test]
fn genesis_appends_reject_delimited_domain_and_stream_segments() {
    let authority = open_temporary_authority();
    let append = authority.append_capability();

    let bad_stream = EventEnvelope::epistemic_genesis(
        "session-a",
        "world_model",
        "obser::vation",
        "scope-a",
        "world_model.unobserved_scope",
        json!({}),
    );
    assert!(matches!(
        append.append_durable(bad_stream.clone(), AppendMode::Idempotent),
        Err(EventAuthorityError::InvalidRequest { .. })
    ));
    assert!(matches!(
        append.append_durable_batch(vec![bad_stream.clone()], AppendMode::Idempotent),
        Err(EventAuthorityError::InvalidRequest { .. })
    ));
    assert!(matches!(
        append.append_best_effort(bad_stream, AppendMode::Idempotent),
        Err(EventAuthorityError::InvalidRequest { .. })
    ));

    let bad_domain = EventEnvelope::epistemic_genesis(
        "session-a",
        "world::model",
        "observation",
        "scope-a",
        "world_model.unobserved_scope",
        json!({}),
    );
    assert!(matches!(
        append.append_durable(bad_domain, AppendMode::Idempotent),
        Err(EventAuthorityError::InvalidRequest { .. })
    ));
}

#[test]
fn clean_genesis_appends_stay_idempotent_and_scoped_records_stay_unaffected() {
    let authority = open_temporary_authority();
    let append = authority.append_capability();

    // Scope keys sit in the final identity segment, so a delimiter inside
    // them cannot shift segment boundaries and stays accepted.
    let genesis = EventEnvelope::epistemic_genesis(
        "session-a",
        "world_model",
        "observation",
        "node-a::analysis",
        "world_model.unobserved_scope",
        json!({ "declared_by": "init" }),
    );
    let first = append
        .append_durable(genesis.clone(), AppendMode::Idempotent)
        .unwrap();
    let second = append
        .append_durable(genesis, AppendMode::Idempotent)
        .unwrap();
    assert_eq!(first.disposition, AppendDisposition::Inserted);
    assert_eq!(second.disposition, AppendDisposition::Duplicate);
    assert_eq!(first.seq, second.seq);

    // Non-genesis records keep their existing freedom over stream naming.
    let plain = EventEnvelope::new_domain(
        "2026-07-24T00:00:00Z".to_string(),
        "session-a",
        "execution",
        "workflow::a",
        "execution.started",
        None,
        json!({}),
    );
    append.append_durable(plain, AppendMode::Plain).unwrap();
}
