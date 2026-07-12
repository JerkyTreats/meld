//! Reusable behavioral assertions for event-authority contract adapters.
//!
//! Adapter tests provide lifecycle and retention controls through the
//! harness while every product operation travels through only
//! [`EventAuthorityContract`]. Future daemon adapters can reuse the same
//! assertions without sharing their transport or process-host machinery.

use std::fmt::Debug;

use serde_json::json;

use super::{
    BestEffortAppendRequest, DurableAppendRequest, EventAuthorityContract, FlowRequest,
    HealthRequest, SessionRequest, TraceRequest, WatermarkRequest,
};
use crate::error::EventAuthorityError;
use crate::events::authority::{
    AppendDisposition, AppendMode, LedgerCursor, ReplayRequest, SubscriptionPollRequest,
};
use crate::events::observability::{CoverageTruncation, FlowWindow, TraceSubject};
use crate::events::{DomainObjectRef, EventEnvelope, EventRecordRef, LedgerIdentity};

/// Lifecycle controls required by the reusable contract assertions.
///
/// Retention mutation and reopen are test-fixture controls, not additions to
/// the production remote contract.
pub trait EventAuthorityConformanceHarness {
    /// Returns the authority identity expected by every request.
    fn ledger_identity(&self) -> LedgerIdentity;

    /// Returns the currently open contract client.
    fn contract(&self) -> &dyn EventAuthorityContract;

    /// Drops and reopens the adapter over the same durable ledger.
    fn reopen(&mut self) -> Result<(), EventAuthorityError>;

    /// Raises the durable retention boundary for a gap assertion.
    fn set_retained_from(&mut self, retained_from: u64) -> Result<(), EventAuthorityError>;
}

/// Runs the common local-or-remote authority behavioral contract.
///
/// The supplied harness must own a new isolated ledger. A failure panics with
/// the violated contract, matching ordinary Rust test assertions.
pub fn assert_event_authority_conformance(harness: &mut impl EventAuthorityConformanceHarness) {
    let ledger_id = harness.ledger_identity();
    let first_envelope = envelope("remote.plain", json!({ "kind": "plain" })).with_graph(
        vec![DomainObjectRef::new("execution", "task", "task-a").unwrap()],
        Vec::new(),
    );
    let first = harness
        .contract()
        .durable_append(DurableAppendRequest {
            ledger_id,
            envelope: first_envelope,
            mode: AppendMode::Plain,
        })
        .expect("durable append must succeed");
    assert_eq!(first.ledger_id, ledger_id);
    assert_eq!(first.seq, 1);
    assert_eq!(first.disposition, AppendDisposition::Inserted);

    let idempotent_envelope = envelope("remote.idempotent", json!({ "kind": "idempotent" }))
        .with_record_id("remote-conformance-idempotent");
    let inserted = harness
        .contract()
        .durable_append(DurableAppendRequest {
            ledger_id,
            envelope: idempotent_envelope.clone(),
            mode: AppendMode::Idempotent,
        })
        .expect("first idempotent append must succeed");
    let duplicate = harness
        .contract()
        .durable_append(DurableAppendRequest {
            ledger_id,
            envelope: idempotent_envelope.clone(),
            mode: AppendMode::Idempotent,
        })
        .expect("idempotent retry must succeed");
    assert_eq!(inserted.seq, 2);
    assert_eq!(inserted.disposition, AppendDisposition::Inserted);
    assert_eq!(duplicate.seq, inserted.seq);
    assert_eq!(duplicate.disposition, AppendDisposition::Duplicate);

    let watermark = harness
        .contract()
        .watermark(WatermarkRequest { ledger_id })
        .expect("watermark must succeed");
    assert_eq!(watermark.ledger_id, ledger_id);
    assert_eq!(watermark.committed_seq, 2);
    assert_eq!(watermark.tip_seq, 2);

    let first_page = harness
        .contract()
        .replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id,
                after_seq: 0,
            },
            limit: 1,
        })
        .expect("bounded replay must succeed");
    assert_eq!(first_page.ledger_id, ledger_id);
    assert_eq!(first_page.records.len(), 1);
    assert_eq!(first_page.records[0].seq, 1);
    assert_eq!(first_page.next_cursor.after_seq, 1);
    assert_eq!(first_page.coverage.tip_seq, 2);
    assert_eq!(first_page.coverage.truncation, CoverageTruncation::After);

    let accepted = harness
        .contract()
        .best_effort_append(BestEffortAppendRequest {
            ledger_id,
            envelope: envelope("remote.best_effort", json!({ "kind": "best_effort" })),
            mode: AppendMode::Plain,
        })
        .expect("best-effort append must be accepted");
    assert_eq!(accepted.ledger_id, ledger_id);
    let subscribed = harness
        .contract()
        .subscription_poll(SubscriptionPollRequest {
            replay: ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id,
                    after_seq: 2,
                },
                limit: 8,
            },
            timeout_ms: 30_000,
        })
        .expect("subscription must observe an accepted append");
    assert_eq!(subscribed.records.len(), 1);
    assert_eq!(subscribed.records[0].seq, 3);
    assert_eq!(subscribed.next_cursor.after_seq, 3);

    let empty = harness
        .contract()
        .subscription_poll(SubscriptionPollRequest {
            replay: ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id,
                    after_seq: 3,
                },
                limit: 8,
            },
            timeout_ms: 0,
        })
        .expect("zero-timeout subscription at the tip must succeed");
    assert!(empty.records.is_empty());
    assert_eq!(empty.ledger_id, ledger_id);
    assert_eq!(empty.next_cursor.after_seq, 3);
    assert_eq!(empty.coverage.tip_seq, 3);

    assert_observability(harness.contract(), ledger_id);
    assert_identity_validation(harness.contract(), ledger_id);

    harness.reopen().expect("authority reopen must succeed");
    assert_eq!(harness.ledger_identity(), ledger_id);
    let recovered = harness
        .contract()
        .watermark(WatermarkRequest { ledger_id })
        .expect("recovered watermark must succeed");
    assert_eq!(recovered.committed_seq, 3);
    assert_eq!(recovered.tip_seq, 3);

    let duplicate_after_reopen = harness
        .contract()
        .durable_append(DurableAppendRequest {
            ledger_id,
            envelope: idempotent_envelope,
            mode: AppendMode::Idempotent,
        })
        .expect("idempotent retry after reopen must succeed");
    assert_eq!(duplicate_after_reopen.seq, 2);
    assert_eq!(
        duplicate_after_reopen.disposition,
        AppendDisposition::Duplicate
    );

    harness
        .set_retained_from(2)
        .expect("retention fixture control must succeed");
    let subscription_gap = harness
        .contract()
        .subscription_poll(SubscriptionPollRequest {
            replay: ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id,
                    after_seq: 0,
                },
                limit: 8,
            },
            timeout_ms: 0,
        });
    assert_eq!(
        subscription_gap.expect_err("subscription before retention must fail"),
        EventAuthorityError::RetentionGap {
            ledger_id,
            after_seq: 0,
            retained_from: 2,
        }
    );
    let gap = harness.contract().replay(ReplayRequest {
        cursor: LedgerCursor {
            ledger_id,
            after_seq: 0,
        },
        limit: 8,
    });
    assert_eq!(
        gap.expect_err("replay before retention must fail"),
        EventAuthorityError::RetentionGap {
            ledger_id,
            after_seq: 0,
            retained_from: 2,
        }
    );
}

fn assert_observability(contract: &dyn EventAuthorityContract, ledger_id: LedgerIdentity) {
    let health = contract
        .health(HealthRequest { ledger_id })
        .expect("health must succeed");
    assert_eq!(health.ledger_id, ledger_id);
    assert_eq!(health.tip_seq, 3);
    assert_eq!(health.committed_watermark, 3);

    let flow = contract
        .flow(FlowRequest {
            ledger_id,
            window: FlowWindow { max_events: 8 },
        })
        .expect("flow must succeed");
    assert_eq!(flow.ledger_id, ledger_id);
    assert_eq!(flow.window_events, 3);

    let trace = contract
        .trace(TraceRequest {
            ledger_id,
            subject: TraceSubject::Stream {
                domain_id: "execution".to_string(),
                stream_id: "remote-conformance".to_string(),
            },
        })
        .expect("trace must succeed");
    assert_eq!(trace.ledger_id, ledger_id);
    assert_eq!(trace.hops.len(), 3);

    let session = contract
        .session(SessionRequest {
            ledger_id,
            session_id: "remote-conformance".to_string(),
        })
        .expect("session must succeed");
    assert_eq!(session.ledger_id, ledger_id);
    assert_eq!(session.events_returned, 3);
}

fn assert_identity_validation(contract: &dyn EventAuthorityContract, ledger_id: LedgerIdentity) {
    let foreign = LedgerIdentity::new();
    let provenance_foreign = LedgerIdentity::new();
    let expected = EventAuthorityError::IdentityMismatch {
        expected: ledger_id,
        actual: foreign,
    };
    let foreign_envelope =
        envelope("remote.foreign", json!({})).with_source_records(vec![EventRecordRef {
            ledger_id: provenance_foreign,
            seq: 1,
        }]);

    assert_error(
        contract.durable_append(DurableAppendRequest {
            ledger_id: foreign,
            envelope: foreign_envelope.clone(),
            mode: AppendMode::Plain,
        }),
        &expected,
    );
    assert_error(
        contract.best_effort_append(BestEffortAppendRequest {
            ledger_id: foreign,
            envelope: foreign_envelope,
            mode: AppendMode::Plain,
        }),
        &expected,
    );
    assert_error(
        contract.replay(ReplayRequest {
            cursor: LedgerCursor {
                ledger_id: foreign,
                after_seq: 0,
            },
            limit: 0,
        }),
        &expected,
    );
    assert_error(
        contract.subscription_poll(SubscriptionPollRequest {
            replay: ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id: foreign,
                    after_seq: 0,
                },
                limit: 0,
            },
            timeout_ms: u64::MAX,
        }),
        &expected,
    );
    assert_error(
        contract.watermark(WatermarkRequest { ledger_id: foreign }),
        &expected,
    );
    assert_error(
        contract.health(HealthRequest { ledger_id: foreign }),
        &expected,
    );
    assert_error(
        contract.flow(FlowRequest {
            ledger_id: foreign,
            window: FlowWindow { max_events: 0 },
        }),
        &expected,
    );
    assert_error(
        contract.trace(TraceRequest {
            ledger_id: foreign,
            subject: TraceSubject::Record { seq: 1 },
        }),
        &expected,
    );
    assert_error(
        contract.session(SessionRequest {
            ledger_id: foreign,
            session_id: "remote-conformance".to_string(),
        }),
        &expected,
    );
}

fn assert_error<T: Debug>(result: Result<T, EventAuthorityError>, expected: &EventAuthorityError) {
    assert_eq!(result.expect_err("foreign identity must fail"), *expected);
}

fn envelope(event_type: &str, data: serde_json::Value) -> EventEnvelope {
    EventEnvelope::new_domain(
        "2026-07-10T00:00:00Z".to_string(),
        "remote-conformance",
        "execution",
        "remote-conformance",
        event_type,
        None,
        data,
    )
}
