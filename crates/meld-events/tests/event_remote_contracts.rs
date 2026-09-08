//! Wire and delegation contracts for the transport-neutral event authority.

use std::str::FromStr;

use meld_events::error::EventAuthorityError;
use meld_events::events::authority::{
    AppendDisposition, AppendMode, AppendReceipt, BestEffortAppendReceipt, EventPage,
    EventWatermark, LedgerCursor, ReplayRequest, SubscriptionPollRequest,
};
use meld_events::events::observability::{
    CoverageTruncation, EventFlowReport, EventHealthReport, EventReadCoverage, EventTraceReport,
    FlowWindow, SessionTimelineReport, TraceSubject,
};
use meld_events::events::remote::{
    BestEffortAppendRequest, DurableAppendRequest, EventAuthorityContract, FlowRequest,
    HealthRequest, SerdeLoopbackEventAuthorityClient, SessionRequest, TraceRequest,
    WatermarkRequest,
};
use meld_events::{EventEnvelope, LedgerIdentity};
use serde_json::json;

#[test]
fn callback_backed_native_capabilities_preserve_proofs_batches_and_reopen() {
    use meld_events::events::remote::LocalEventAuthorityClient;
    use meld_events::{EventAppendCapability, EventAuthority, EventAuthorityOpenOptions};
    use std::sync::Arc;
    let root = tempfile::tempdir().unwrap();
    let expected = envelope().with_record_id("external-owner-record");
    let ledger;
    let position;
    {
        let authority = EventAuthority::open(
            sled::open(root.path()).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        ledger = authority.ledger_identity();
        let client = Arc::new(SerdeLoopbackEventAuthorityClient::new(
            LocalEventAuthorityClient::new(&authority),
        ));
        let append = EventAppendCapability::from_remote(ledger, client.clone());
        let proof = append
            .append_durable_proven(expected.clone(), AppendMode::Idempotent)
            .unwrap();
        position = proof.seq();
        assert_eq!(
            proof,
            authority
                .replay_capability()
                .prove_existing(&expected)
                .unwrap()
                .unwrap()
        );
        assert_eq!(
            proof,
            append
                .append_durable_proven(expected.clone(), AppendMode::Idempotent)
                .unwrap()
        );
        let mut changed = expected.clone();
        changed.data = json!({"task":"another-task"});
        assert!(append.replay_capability().prove_existing(&changed).is_err());
        assert!(append
            .replay_capability()
            .prove_existing(&envelope().with_record_id("absent"))
            .unwrap()
            .is_none());

        let batch = vec![
            envelope().with_record_id("batch-one"),
            envelope().with_record_id("batch-two"),
        ];
        let receipts = append
            .append_durable_batch(batch.clone(), AppendMode::Idempotent)
            .unwrap();
        assert_eq!(receipts.len(), 2);
        for (receipt, event) in receipts.iter().zip(&batch) {
            assert_eq!(
                authority
                    .replay_capability()
                    .prove_existing(event)
                    .unwrap()
                    .unwrap()
                    .seq(),
                receipt.seq
            );
        }
        append
            .append_best_effort(envelope().with_record_id("queued"), AppendMode::Idempotent)
            .unwrap();
        append.barrier().unwrap();
        assert_eq!(
            append.replay_capability().newest_page(4).unwrap(),
            authority.replay_capability().newest_page(4).unwrap()
        );

        let foreign = EventAppendCapability::from_remote(LedgerIdentity::new(), client);
        assert!(matches!(
            foreign.append_durable(expected.clone(), AppendMode::Idempotent),
            Err(EventAuthorityError::IdentityMismatch { .. })
        ));
        assert!(matches!(
            foreign
                .replay_capability()
                .committed_record("external-owner-record"),
            Err(EventAuthorityError::IdentityMismatch { .. })
        ));
    }
    let reopened = EventAuthority::open_existing(sled::open(root.path()).unwrap(), ledger).unwrap();
    let append = EventAppendCapability::from_remote(
        ledger,
        Arc::new(SerdeLoopbackEventAuthorityClient::new(
            LocalEventAuthorityClient::new(&reopened),
        )),
    );
    let proof = append
        .replay_capability()
        .prove_existing(&expected)
        .unwrap()
        .unwrap();
    assert_eq!(proof.seq(), position);
    assert_eq!(proof.ledger_id(), ledger);
    assert_eq!(
        append
            .append_durable_proven(expected, AppendMode::Idempotent)
            .unwrap(),
        proof
    );
}

fn ledger_id() -> LedgerIdentity {
    LedgerIdentity::from_str("00000000-0000-4000-8000-000000000001").unwrap()
}

fn envelope() -> EventEnvelope {
    EventEnvelope::new_domain(
        "2026-07-10T00:00:00Z".to_string(),
        "session-a",
        "execution",
        "run-a",
        "execution.task.started",
        None,
        json!({ "task": "task-a" }),
    )
}

#[test]
fn remote_request_wire_shapes_are_pinned() {
    let ledger_id = ledger_id();
    let envelope_shape = json!({
        "ts": "2026-07-10T00:00:00Z",
        "recorded_at": "2026-07-10T00:00:00Z",
        "record_id": null,
        "session": "session-a",
        "domain_id": "execution",
        "stream_id": "run-a",
        "type": "execution.task.started",
        "occurred_at": null,
        "content_hash": null,
        "objects": [],
        "relations": [],
        "data": { "task": "task-a" }
    });
    assert_wire_shape(
        DurableAppendRequest {
            ledger_id,
            envelope: envelope(),
            mode: AppendMode::Idempotent,
        },
        json!({
            "ledger_id": ledger_id,
            "envelope": envelope_shape,
            "mode": "idempotent"
        }),
    );
    assert_wire_shape(
        BestEffortAppendRequest {
            ledger_id,
            envelope: envelope(),
            mode: AppendMode::Plain,
        },
        json!({
            "ledger_id": ledger_id,
            "envelope": envelope_shape,
            "mode": "plain"
        }),
    );

    let replay = ReplayRequest {
        cursor: LedgerCursor {
            ledger_id,
            after_seq: 7,
        },
        limit: 64,
    };
    assert_wire_shape(
        replay,
        json!({
            "cursor": { "ledger_id": ledger_id, "after_seq": 7 },
            "limit": 64
        }),
    );
    assert_wire_shape(
        SubscriptionPollRequest {
            replay,
            timeout_ms: 30_000,
        },
        json!({
            "replay": {
                "cursor": { "ledger_id": ledger_id, "after_seq": 7 },
                "limit": 64
            },
            "timeout_ms": 30_000
        }),
    );

    assert_eq!(
        serde_json::to_value(WatermarkRequest { ledger_id }).unwrap(),
        json!({ "ledger_id": ledger_id })
    );
    assert_eq!(
        serde_json::to_value(HealthRequest { ledger_id }).unwrap(),
        json!({ "ledger_id": ledger_id })
    );
    assert_eq!(
        serde_json::to_value(FlowRequest {
            ledger_id,
            window: FlowWindow { max_events: 64 },
        })
        .unwrap(),
        json!({ "ledger_id": ledger_id, "window": { "max_events": 64 } })
    );
    assert_eq!(
        serde_json::to_value(TraceRequest {
            ledger_id,
            subject: TraceSubject::Record { seq: 7 },
        })
        .unwrap(),
        json!({ "ledger_id": ledger_id, "subject": { "record": { "seq": 7 } } })
    );
    assert_eq!(
        serde_json::to_value(SessionRequest {
            ledger_id,
            session_id: "session-a".to_string(),
        })
        .unwrap(),
        json!({ "ledger_id": ledger_id, "session_id": "session-a" })
    );
}

#[test]
fn remote_response_wire_shapes_are_pinned() {
    let ledger_id = ledger_id();
    assert_wire_shape(
        AppendReceipt {
            ledger_id,
            seq: 9,
            disposition: AppendDisposition::Duplicate,
        },
        json!({
            "ledger_id": ledger_id,
            "seq": 9,
            "disposition": "duplicate"
        }),
    );
    assert_wire_shape(
        BestEffortAppendReceipt { ledger_id },
        json!({ "ledger_id": ledger_id }),
    );

    let coverage = EventReadCoverage {
        retained_from: 4,
        tip_seq: 12,
        scanned_from_seq: None,
        scanned_through_seq: None,
        truncation: CoverageTruncation::Before,
    };
    assert_wire_shape(
        EventPage {
            ledger_id,
            records: Vec::new(),
            next_cursor: LedgerCursor {
                ledger_id,
                after_seq: 12,
            },
            coverage,
        },
        json!({
            "ledger_id": ledger_id,
            "records": [],
            "next_cursor": { "ledger_id": ledger_id, "after_seq": 12 },
            "coverage": {
                "retained_from": 4,
                "tip_seq": 12,
                "scanned_from_seq": null,
                "scanned_through_seq": null,
                "truncation": "before"
            }
        }),
    );
    assert_wire_shape(
        EventWatermark {
            ledger_id,
            committed_seq: 11,
            tip_seq: 12,
        },
        json!({
            "ledger_id": ledger_id,
            "committed_seq": 11,
            "tip_seq": 12
        }),
    );
}

fn assert_wire_shape<T>(product: T, expected: serde_json::Value)
where
    T: std::fmt::Debug + PartialEq + serde::Serialize + serde::de::DeserializeOwned,
{
    assert_eq!(serde_json::to_value(&product).unwrap(), expected);
    assert_eq!(serde_json::from_value::<T>(expected).unwrap(), product);
}

#[test]
fn authority_error_kinds_and_payloads_round_trip() {
    let ledger_id = ledger_id();
    let foreign = LedgerIdentity::from_str("00000000-0000-4000-8000-000000000002").unwrap();
    assert_eq!(
        serde_json::to_value(EventAuthorityError::RetentionGap {
            ledger_id,
            after_seq: 1,
            retained_from: 2,
        })
        .unwrap(),
        json!({
            "kind": "retention_gap",
            "ledger_id": ledger_id,
            "after_seq": 1,
            "retained_from": 2
        })
    );
    let cases = vec![
        (
            EventAuthorityError::InvalidRequest {
                message: "invalid".to_string(),
            },
            "invalid_request",
        ),
        (
            EventAuthorityError::IdentityMismatch {
                expected: ledger_id,
                actual: foreign,
            },
            "identity_mismatch",
        ),
        (
            EventAuthorityError::DuplicateAuthorityBinding { ledger_id },
            "duplicate_authority_binding",
        ),
        (
            EventAuthorityError::RetentionGap {
                ledger_id,
                after_seq: 1,
                retained_from: 2,
            },
            "retention_gap",
        ),
        (
            EventAuthorityError::Backpressure {
                message: "full".to_string(),
            },
            "backpressure",
        ),
        (
            EventAuthorityError::DurabilityIndeterminate {
                message: "flush".to_string(),
            },
            "durability_indeterminate",
        ),
        (
            EventAuthorityError::Unavailable {
                message: "offline".to_string(),
            },
            "unavailable",
        ),
        (
            EventAuthorityError::Persistence {
                message: "disk".to_string(),
            },
            "persistence",
        ),
        (
            EventAuthorityError::Internal {
                message: "invariant".to_string(),
            },
            "internal",
        ),
        (
            EventAuthorityError::CorruptPersistedIdentity {
                message: "uuid".to_string(),
            },
            "corrupt_persisted_identity",
        ),
        (
            EventAuthorityError::MigrationConflict {
                message: "mapping".to_string(),
            },
            "migration_conflict",
        ),
    ];

    for (error, expected_kind) in cases {
        let value = serde_json::to_value(&error).unwrap();
        assert_eq!(value["kind"], json!(expected_kind));
        assert_eq!(
            serde_json::from_value::<EventAuthorityError>(value).unwrap(),
            error
        );
    }
}

#[derive(Clone)]
struct AlwaysFails {
    error: EventAuthorityError,
}

impl EventAuthorityContract for AlwaysFails {
    fn durable_append_batch(
        &self,
        _: meld_events::events::remote::DurableAppendBatchRequest,
    ) -> Result<Vec<AppendReceipt>, EventAuthorityError> {
        Err(self.error.clone())
    }
    fn committed_record(
        &self,
        _: meld_events::events::remote::CommittedRecordRequest,
    ) -> Result<meld_events::events::remote::CommittedRecordResponse, EventAuthorityError> {
        Err(self.error.clone())
    }
    fn newest_page(
        &self,
        _: meld_events::events::remote::NewestPageRequest,
    ) -> Result<EventPage, EventAuthorityError> {
        Err(self.error.clone())
    }
    fn barrier(
        &self,
        _: meld_events::events::remote::BarrierRequest,
    ) -> Result<(), EventAuthorityError> {
        Err(self.error.clone())
    }
    fn durable_append(
        &self,
        _request: DurableAppendRequest,
    ) -> Result<AppendReceipt, EventAuthorityError> {
        Err(self.error.clone())
    }

    fn best_effort_append(
        &self,
        _request: BestEffortAppendRequest,
    ) -> Result<BestEffortAppendReceipt, EventAuthorityError> {
        Err(self.error.clone())
    }

    fn replay(&self, _request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        Err(self.error.clone())
    }

    fn subscription_poll(
        &self,
        _request: SubscriptionPollRequest,
    ) -> Result<EventPage, EventAuthorityError> {
        Err(self.error.clone())
    }

    fn watermark(&self, _request: WatermarkRequest) -> Result<EventWatermark, EventAuthorityError> {
        Err(self.error.clone())
    }

    fn health(&self, _request: HealthRequest) -> Result<EventHealthReport, EventAuthorityError> {
        Err(self.error.clone())
    }

    fn flow(&self, _request: FlowRequest) -> Result<EventFlowReport, EventAuthorityError> {
        Err(self.error.clone())
    }

    fn trace(&self, _request: TraceRequest) -> Result<EventTraceReport, EventAuthorityError> {
        Err(self.error.clone())
    }

    fn session(
        &self,
        _request: SessionRequest,
    ) -> Result<SessionTimelineReport, EventAuthorityError> {
        Err(self.error.clone())
    }
}

#[test]
fn loopback_round_trips_errors_from_every_contract_method() {
    let ledger_id = ledger_id();
    let error = EventAuthorityError::MigrationConflict {
        message: "wire sentinel".to_string(),
    };
    let client = SerdeLoopbackEventAuthorityClient::new(AlwaysFails {
        error: error.clone(),
    });
    let cursor = LedgerCursor {
        ledger_id,
        after_seq: 0,
    };
    let replay = ReplayRequest { cursor, limit: 1 };

    assert_eq!(
        client
            .durable_append(DurableAppendRequest {
                ledger_id,
                envelope: envelope(),
                mode: AppendMode::Plain,
            })
            .unwrap_err(),
        error
    );
    assert_eq!(
        client
            .best_effort_append(BestEffortAppendRequest {
                ledger_id,
                envelope: envelope(),
                mode: AppendMode::Plain,
            })
            .unwrap_err(),
        error
    );
    assert_eq!(client.replay(replay).unwrap_err(), error);
    assert_eq!(
        client
            .subscription_poll(SubscriptionPollRequest {
                replay,
                timeout_ms: 0,
            })
            .unwrap_err(),
        error
    );
    assert_eq!(
        client
            .watermark(WatermarkRequest { ledger_id })
            .unwrap_err(),
        error
    );
    assert_eq!(
        client.health(HealthRequest { ledger_id }).unwrap_err(),
        error
    );
    assert_eq!(
        client
            .flow(FlowRequest {
                ledger_id,
                window: FlowWindow { max_events: 1 },
            })
            .unwrap_err(),
        error
    );
    assert_eq!(
        client
            .trace(TraceRequest {
                ledger_id,
                subject: TraceSubject::Record { seq: 1 },
            })
            .unwrap_err(),
        error
    );
    assert_eq!(
        client
            .session(SessionRequest {
                ledger_id,
                session_id: "session-a".to_string(),
            })
            .unwrap_err(),
        error
    );
}
