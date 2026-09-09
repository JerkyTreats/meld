//! Native Event transport for package owners and scoped parent callbacks.

use std::collections::BTreeSet;
use std::sync::Arc;

use meld_events::error::EventAuthorityError;
use meld_events::events::remote::*;
use meld_events::{
    AppendReceipt, BestEffortAppendReceipt, EventEnvelope, EventFlowReport, EventHealthReport,
    EventPage, EventTraceReport, EventWatermark, LedgerIdentity, ReplayRequest,
    SessionTimelineReport, SubscriptionPollRequest,
};
use serde::de::DeserializeOwned;

use super::{encode_owner_result, OwnerCallbackPort, OwnerCallbackV1, OwnerResult};

/// Callback endpoint retained by native owner capabilities. It has no ledger or
/// writer; the parent checks each request against its outstanding native command.
pub struct CallbackEventAuthorityClient {
    callbacks: Arc<dyn OwnerCallbackPort>,
}

impl CallbackEventAuthorityClient {
    pub fn new(callbacks: Arc<dyn OwnerCallbackPort>) -> Self {
        Self { callbacks }
    }

    fn call<T: DeserializeOwned>(
        &self,
        request: OwnerCallbackV1,
    ) -> Result<T, EventAuthorityError> {
        let result =
            self.callbacks
                .call(request)
                .map_err(|error| EventAuthorityError::Internal {
                    message: error.to_string(),
                })?;
        serde_json::from_value::<Result<T, EventAuthorityError>>(result).map_err(|error| {
            EventAuthorityError::Internal {
                message: error.to_string(),
            }
        })?
    }
}

impl EventAuthorityContract for CallbackEventAuthorityClient {
    fn durable_append(
        &self,
        request: DurableAppendRequest,
    ) -> Result<AppendReceipt, EventAuthorityError> {
        self.call(OwnerCallbackV1::Append { request })
    }
    fn durable_append_batch(
        &self,
        request: DurableAppendBatchRequest,
    ) -> Result<Vec<AppendReceipt>, EventAuthorityError> {
        self.call(OwnerCallbackV1::AppendBatch { request })
    }
    fn best_effort_append(
        &self,
        request: BestEffortAppendRequest,
    ) -> Result<BestEffortAppendReceipt, EventAuthorityError> {
        self.call(OwnerCallbackV1::BestEffortAppend { request })
    }
    fn committed_record(
        &self,
        request: CommittedRecordRequest,
    ) -> Result<CommittedRecordResponse, EventAuthorityError> {
        self.call(OwnerCallbackV1::CommittedRecord { request })
    }
    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        self.call(OwnerCallbackV1::Replay { request })
    }
    fn newest_page(&self, request: NewestPageRequest) -> Result<EventPage, EventAuthorityError> {
        self.call(OwnerCallbackV1::NewestPage { request })
    }
    fn barrier(&self, request: BarrierRequest) -> Result<(), EventAuthorityError> {
        self.call(OwnerCallbackV1::Barrier { request })
    }
    fn subscription_poll(
        &self,
        _: SubscriptionPollRequest,
    ) -> Result<EventPage, EventAuthorityError> {
        Err(ungranted())
    }
    fn watermark(&self, _: WatermarkRequest) -> Result<EventWatermark, EventAuthorityError> {
        Err(ungranted())
    }
    fn health(&self, _: HealthRequest) -> Result<EventHealthReport, EventAuthorityError> {
        Err(ungranted())
    }
    fn flow(&self, _: FlowRequest) -> Result<EventFlowReport, EventAuthorityError> {
        Err(ungranted())
    }
    fn trace(&self, _: TraceRequest) -> Result<EventTraceReport, EventAuthorityError> {
        Err(ungranted())
    }
    fn session(&self, _: SessionRequest) -> Result<SessionTimelineReport, EventAuthorityError> {
        Err(ungranted())
    }
}

fn ungranted() -> EventAuthorityError {
    EventAuthorityError::invalid_request(
        "this owner connection has no grant for the requested Event operation",
    )
}

/// Parent-side grant supplied by the native operation, never supplied by the child.
/// Read-only recovery cannot append; write grants bind the native session and owner.
pub struct OwnerEventCallbacks {
    ledger_id: LedgerIdentity,
    authority: Arc<dyn EventAuthorityContract>,
    publication: Option<PublicationGrant>,
}

struct PublicationGrant {
    owner: String,
    routes: BTreeSet<(String, String, String)>,
}

impl OwnerEventCallbacks {
    pub fn read_only(
        ledger_id: LedgerIdentity,
        authority: Arc<dyn EventAuthorityContract>,
    ) -> Self {
        Self {
            ledger_id,
            authority,
            publication: None,
        }
    }

    /// Routes contain exact event-type and stream pairs resolved by the native
    /// operation from its selected package and assignment or effect scope.
    pub fn publishing(
        ledger_id: LedgerIdentity,
        authority: Arc<dyn EventAuthorityContract>,
        owner: &str,
        session: &str,
        routes: BTreeSet<(String, String)>,
    ) -> Result<Self, EventAuthorityError> {
        Self::publishing_routes(
            ledger_id,
            authority,
            owner,
            routes
                .into_iter()
                .map(|(kind, stream)| (session.to_string(), kind, stream))
                .collect(),
        )
    }

    /// Exact session, event-type and stream grants for retained publications.
    pub fn publishing_routes(
        ledger_id: LedgerIdentity,
        authority: Arc<dyn EventAuthorityContract>,
        owner: &str,
        routes: BTreeSet<(String, String, String)>,
    ) -> Result<Self, EventAuthorityError> {
        if owner.trim().is_empty()
            || routes.is_empty()
            || routes.iter().any(|(session, kind, stream)| {
                session.trim().is_empty() || kind.trim().is_empty() || stream.trim().is_empty()
            })
        {
            return Err(ungranted());
        }
        Ok(Self {
            ledger_id,
            authority,
            publication: Some(PublicationGrant {
                owner: owner.into(),
                routes,
            }),
        })
    }

    fn ledger(&self, actual: LedgerIdentity) -> Result<(), EventAuthorityError> {
        if actual != self.ledger_id {
            return Err(EventAuthorityError::IdentityMismatch {
                expected: self.ledger_id,
                actual,
            });
        }
        Ok(())
    }

    fn publication(
        &self,
        ledger: LedgerIdentity,
        envelope: &EventEnvelope,
    ) -> Result<(), EventAuthorityError> {
        self.ledger(ledger)?;
        match &self.publication {
            Some(grant)
                if envelope.domain_id == grant.owner
                    && grant.routes.contains(&(
                        envelope.session.clone(),
                        envelope.event_type.clone(),
                        envelope.stream_id.clone(),
                    )) =>
            {
                Ok(())
            }
            _ => Err(ungranted()),
        }
    }
}

impl OwnerCallbackPort for OwnerEventCallbacks {
    fn call(&self, callback: OwnerCallbackV1) -> OwnerResult {
        match callback {
            OwnerCallbackV1::Append { request } => encode_owner_result(
                self.publication(request.ledger_id, &request.envelope)
                    .and_then(|_| self.authority.durable_append(request)),
            ),
            OwnerCallbackV1::AppendBatch { request } => {
                let allowed = self.ledger(request.ledger_id).and_then(|_| {
                    if self.publication.is_none() {
                        return Err(ungranted());
                    }
                    request
                        .envelopes
                        .iter()
                        .try_for_each(|envelope| self.publication(request.ledger_id, envelope))
                });
                encode_owner_result(
                    allowed.and_then(|_| self.authority.durable_append_batch(request)),
                )
            }
            OwnerCallbackV1::BestEffortAppend { request } => encode_owner_result(
                self.publication(request.ledger_id, &request.envelope)
                    .and_then(|_| self.authority.best_effort_append(request)),
            ),
            OwnerCallbackV1::CommittedRecord { request } => encode_owner_result(
                self.ledger(request.ledger_id)
                    .and_then(|_| self.authority.committed_record(request)),
            ),
            OwnerCallbackV1::Replay { request } => encode_owner_result(
                self.ledger(request.cursor.ledger_id)
                    .and_then(|_| self.authority.replay(request)),
            ),
            OwnerCallbackV1::NewestPage { request } => encode_owner_result(
                self.ledger(request.ledger_id)
                    .and_then(|_| self.authority.newest_page(request)),
            ),
            OwnerCallbackV1::Barrier { request } => {
                encode_owner_result(self.ledger(request.ledger_id).and_then(|_| {
                    if self.publication.is_none() {
                        return Err(ungranted());
                    }
                    self.authority.barrier(request)
                }))
            }
            OwnerCallbackV1::Provider { .. } => Err(super::OwnerDiagnosticV1::new(
                "owner_callback_not_granted",
                "Event access grants no provider execution",
            )),
        }
    }
}
