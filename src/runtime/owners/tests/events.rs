use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::sync::Arc;

use meld_events::events::remote::LocalEventAuthorityClient;
use meld_events::{
    AppendMode, EventAppendCapability, EventAuthority, EventAuthorityOpenOptions, EventEnvelope,
    LedgerIdentity,
};

use super::super::events::{CallbackEventAuthorityClient, OwnerEventCallbacks};
use super::super::*;
use crate::runtime::lifecycle::ParticipantLifecycleContextV1;

struct ObservingOwner {
    ledger: LedgerIdentity,
    events: Option<EventAppendCapability>,
    envelope: EventEnvelope,
}

impl server::PackageOwner for ObservingOwner {
    fn handle(
        &mut self,
        command: OwnerCommandV1,
        callbacks: Arc<dyn OwnerCallbackPort>,
    ) -> OwnerResult {
        let events = self.events.get_or_insert_with(|| {
            EventAppendCapability::from_remote(
                self.ledger,
                Arc::new(CallbackEventAuthorityClient::new(callbacks)),
            )
        });
        let result = match command {
            OwnerCommandV1::Observe { .. } => {
                events.append_durable_proven(self.envelope.clone(), AppendMode::Idempotent)
            }
            OwnerCommandV1::Readiness { .. } => events
                .replay_capability()
                .prove_existing(&self.envelope)
                .and_then(|proof| {
                    proof.ok_or_else(|| {
                        meld_events::error::EventAuthorityError::invalid_request(
                            "observation absent",
                        )
                    })
                }),
            _ => {
                return Err(OwnerDiagnosticV1::new(
                    "unsupported",
                    "specimen accepts observation and readiness only",
                ))
            }
        };
        encode_owner_result(
            result
                .map_err(|error| OwnerDiagnosticV1::new("owner_event_failed", error))?
                .seq(),
        )
    }
}

fn context() -> ParticipantLifecycleContextV1 {
    ParticipantLifecycleContextV1 {
        generation_id: "generation".into(),
        incarnation_id: "incarnation".into(),
        realization_id: "realization".into(),
        participant_id: "specimen-observation".into(),
        owner_domain: "specimen".into(),
        kind: crate::theory::ParticipantKind::BoundedActor,
        readiness_contract_ref: "specimen-ready".into(),
        wake_contract_ref: "specimen-wake".into(),
        safe_point_contract_ref: "specimen-safe".into(),
        stop_contract_ref: "specimen-stop".into(),
        lease_ref: "lease".into(),
    }
}

#[test]
fn retained_owner_capabilities_follow_each_native_command_grant() {
    let authority = EventAuthority::open(
        sled::Config::new().temporary(true).open().unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let ledger = authority.ledger_identity();
    let native = Arc::new(LocalEventAuthorityClient::new(&authority));
    let write = OwnerEventCallbacks::publishing(
        ledger,
        native.clone(),
        "specimen",
        "session",
        std::collections::BTreeSet::from([("specimen.observed".into(), "subject".into())]),
    )
    .unwrap();
    let read = OwnerEventCallbacks::read_only(ledger, native);
    let envelope = EventEnvelope::with_now_domain(
        "session",
        "specimen",
        "subject",
        "specimen.observed",
        None,
        serde_json::json!({"observed":true}),
    )
    .with_record_id("specimen-observation");
    let expected = envelope.clone();
    let mut outside = envelope.clone();
    outside.stream_id = "another-assignment".into();
    outside.record_id = Some("foreign-stream".into());
    let denied = write
        .call(OwnerCallbackV1::AppendBatch {
            request: meld_events::events::remote::DurableAppendBatchRequest {
                ledger_id: ledger,
                envelopes: vec![envelope.clone(), outside],
                mode: AppendMode::Idempotent,
            },
        })
        .unwrap();
    assert!(serde_json::from_value::<
        Result<Vec<meld_events::AppendReceipt>, meld_events::error::EventAuthorityError>,
    >(denied)
    .unwrap()
    .is_err());
    assert!(authority
        .replay_capability()
        .committed_record("specimen-observation")
        .unwrap()
        .is_none());
    let (mut parent, child) = UnixStream::pair().unwrap();
    let mut reader = BufReader::new(parent.try_clone().unwrap());
    let thread = std::thread::spawn(move || {
        server::serve_owner(
            &mut ObservingOwner {
                ledger,
                events: None,
                envelope,
            },
            BufReader::new(child.try_clone().unwrap()),
            child,
            65_536,
        )
    });
    let mut invoke = |request_id, command, grant: &dyn OwnerCallbackPort| {
        server::write_message(
            &mut parent,
            &HostMessageV1::Command {
                protocol_version: OWNER_PROTOCOL_VERSION,
                request_id,
                command: Box::new(command),
            },
            65_536,
        )
        .unwrap();
        loop {
            match server::read_message::<OwnerMessageV1>(&mut reader, 65_536)
                .unwrap()
                .unwrap()
            {
                OwnerMessageV1::Callback {
                    request_id: returned,
                    callback_id,
                    callback,
                } => {
                    assert_eq!(returned, request_id);
                    server::write_message(
                        &mut parent,
                        &HostMessageV1::CallbackReturn {
                            request_id,
                            callback_id,
                            result: grant.call(*callback),
                        },
                        65_536,
                    )
                    .unwrap();
                }
                OwnerMessageV1::Return {
                    request_id: returned,
                    result,
                } => {
                    assert_eq!(returned, request_id);
                    break result;
                }
            }
        }
    };
    let observe = || OwnerCommandV1::Observe {
        context: context(),
        budget: crate::runtime::contracts::WorkBudget { max_items: 1 },
    };
    let first = invoke(1, observe(), &write).unwrap();
    assert_eq!(
        first,
        authority
            .replay_capability()
            .prove_existing(&expected)
            .unwrap()
            .unwrap()
            .seq()
    );
    assert!(invoke(2, observe(), &read).is_err());
    assert_eq!(
        invoke(3, OwnerCommandV1::Readiness { context: context() }, &read).unwrap(),
        first
    );
    drop(reader);
    drop(parent);
    thread.join().unwrap().unwrap();
}
