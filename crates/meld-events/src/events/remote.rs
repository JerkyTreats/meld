//! Transport-neutral contract for local and future remote event authorities.
//!
//! Owner: event authority remote seam.
//! Inputs: identity-bearing append, replay, subscription, watermark, and
//! observability requests.
//! Outputs: serializable authority products and typed authority errors.
//! Does not own: framing, endpoints, authentication, reconnects, process
//! lifecycle, or daemon hosting.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::EventAuthorityError;
use crate::events::authority::{
    AppendMode, AppendReceipt, BestEffortAppendReceipt, EventAppendCapability, EventAuthority,
    EventObservabilityCapability, EventPage, EventReplayCapability, EventSubscriptionCapability,
    EventWatermark, EventWatermarkCapability, ReplayRequest, SubscriptionPollRequest,
};
use crate::events::observability::{
    EventFlowReport, EventHealthReport, EventTraceReport, FlowWindow, SessionTimelineReport,
    TraceSubject,
};
use crate::events::{EventEnvelope, LedgerIdentity};

/// Reusable adapter-conformance assertions for future transports.
#[cfg(feature = "test-support")]
pub mod conformance;

/// Identity-bearing durable append request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DurableAppendRequest {
    /// Authority expected to accept the envelope.
    pub ledger_id: LedgerIdentity,
    /// Canonical envelope to append.
    pub envelope: EventEnvelope,
    /// Plain or idempotent append behavior.
    pub mode: AppendMode,
}

/// Identity-bearing best-effort append request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BestEffortAppendRequest {
    /// Authority expected to accept the envelope.
    pub ledger_id: LedgerIdentity,
    /// Canonical envelope to enqueue.
    pub envelope: EventEnvelope,
    /// Plain or idempotent append behavior.
    pub mode: AppendMode,
}

/// Request for an identity-validated watermark snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatermarkRequest {
    /// Authority whose watermark is requested.
    pub ledger_id: LedgerIdentity,
}

/// Request for an identity-validated health report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthRequest {
    /// Authority whose health is requested.
    pub ledger_id: LedgerIdentity,
}

/// Request for an identity-validated flow report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowRequest {
    /// Authority whose flow is requested.
    pub ledger_id: LedgerIdentity,
    /// Bounded trailing window.
    pub window: FlowWindow,
}

/// Request for an identity-validated structural trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceRequest {
    /// Authority whose records are traced.
    pub ledger_id: LedgerIdentity,
    /// Structural trace subject.
    pub subject: TraceSubject,
}

/// Request for an identity-validated session timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionRequest {
    /// Authority whose session records are requested.
    pub ledger_id: LedgerIdentity,
    /// Exact session identifier.
    pub session_id: String,
}

/// Synchronous contract shared by local and future remote authority clients.
pub trait EventAuthorityContract: Send + Sync {
    /// Appends durably and returns an identity-bearing receipt.
    fn durable_append(
        &self,
        request: DurableAppendRequest,
    ) -> Result<AppendReceipt, EventAuthorityError>;

    /// Accepts a best-effort append without claiming durability or sequence.
    fn best_effort_append(
        &self,
        request: BestEffortAppendRequest,
    ) -> Result<BestEffortAppendReceipt, EventAuthorityError>;

    /// Replays one bounded identity-bearing page.
    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError>;

    /// Polls one bounded identity-bearing subscription page.
    fn subscription_poll(
        &self,
        request: SubscriptionPollRequest,
    ) -> Result<EventPage, EventAuthorityError>;

    /// Returns the recovered committed watermark and durable tip.
    fn watermark(&self, request: WatermarkRequest) -> Result<EventWatermark, EventAuthorityError>;

    /// Returns one identity-bearing health report.
    fn health(&self, request: HealthRequest) -> Result<EventHealthReport, EventAuthorityError>;

    /// Returns one identity-bearing flow report.
    fn flow(&self, request: FlowRequest) -> Result<EventFlowReport, EventAuthorityError>;

    /// Returns one identity-bearing structural trace.
    fn trace(&self, request: TraceRequest) -> Result<EventTraceReport, EventAuthorityError>;

    /// Returns one identity-bearing bounded session timeline.
    fn session(
        &self,
        request: SessionRequest,
    ) -> Result<SessionTimelineReport, EventAuthorityError>;
}

/// In-process implementation of the transport-neutral authority contract.
#[derive(Clone)]
pub struct LocalEventAuthorityClient {
    ledger_id: LedgerIdentity,
    append: EventAppendCapability,
    replay: EventReplayCapability,
    subscription: EventSubscriptionCapability,
    watermark: EventWatermarkCapability,
    observability: EventObservabilityCapability,
}

impl LocalEventAuthorityClient {
    /// Derives a local client from one authority aggregate.
    pub fn new(authority: &EventAuthority) -> Self {
        Self {
            ledger_id: authority.ledger_identity(),
            append: authority.append_capability(),
            replay: authority.replay_capability(),
            subscription: authority.subscription_capability(),
            watermark: authority.watermark_capability(),
            observability: authority.observability_capability(),
        }
    }

    /// Returns the ledger accepted by this client.
    pub fn ledger_identity(&self) -> LedgerIdentity {
        self.ledger_id
    }

    fn validate_identity(&self, actual: LedgerIdentity) -> Result<(), EventAuthorityError> {
        if self.ledger_id == actual {
            Ok(())
        } else {
            Err(EventAuthorityError::IdentityMismatch {
                expected: self.ledger_id,
                actual,
            })
        }
    }
}

impl EventAuthorityContract for LocalEventAuthorityClient {
    fn durable_append(
        &self,
        request: DurableAppendRequest,
    ) -> Result<AppendReceipt, EventAuthorityError> {
        self.validate_identity(request.ledger_id)?;
        self.append.append_durable(request.envelope, request.mode)
    }

    fn best_effort_append(
        &self,
        request: BestEffortAppendRequest,
    ) -> Result<BestEffortAppendReceipt, EventAuthorityError> {
        self.validate_identity(request.ledger_id)?;
        self.append
            .append_best_effort(request.envelope, request.mode)
    }

    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        self.validate_identity(request.cursor.ledger_id)?;
        self.replay.replay(request)
    }

    fn subscription_poll(
        &self,
        request: SubscriptionPollRequest,
    ) -> Result<EventPage, EventAuthorityError> {
        self.validate_identity(request.replay.cursor.ledger_id)?;
        self.subscription.poll(request)
    }

    fn watermark(&self, request: WatermarkRequest) -> Result<EventWatermark, EventAuthorityError> {
        self.validate_identity(request.ledger_id)?;
        self.watermark.snapshot()
    }

    fn health(&self, request: HealthRequest) -> Result<EventHealthReport, EventAuthorityError> {
        self.validate_identity(request.ledger_id)?;
        self.observability.health(request.ledger_id)
    }

    fn flow(&self, request: FlowRequest) -> Result<EventFlowReport, EventAuthorityError> {
        self.validate_identity(request.ledger_id)?;
        self.observability.flow(request.ledger_id, request.window)
    }

    fn trace(&self, request: TraceRequest) -> Result<EventTraceReport, EventAuthorityError> {
        self.validate_identity(request.ledger_id)?;
        self.observability.trace(request.ledger_id, request.subject)
    }

    fn session(
        &self,
        request: SessionRequest,
    ) -> Result<SessionTimelineReport, EventAuthorityError> {
        self.validate_identity(request.ledger_id)?;
        self.observability
            .session(request.ledger_id, &request.session_id)
    }
}

/// In-process proof adapter that serde-round-trips every wire product.
///
/// This deliberately supplies no framing, endpoint, authentication,
/// reconnect, or lifecycle behavior. It proves only that the transport-
/// neutral contract survives serialization before a daemon exists.
#[derive(Clone)]
pub struct SerdeLoopbackEventAuthorityClient<C> {
    inner: C,
}

impl<C> SerdeLoopbackEventAuthorityClient<C> {
    /// Wraps another contract implementation with JSON wire round trips.
    pub fn new(inner: C) -> Self {
        Self { inner }
    }

    /// Returns the wrapped contract implementation.
    pub fn into_inner(self) -> C {
        self.inner
    }
}

impl<C> SerdeLoopbackEventAuthorityClient<C>
where
    C: EventAuthorityContract,
{
    fn call<Request, Response>(
        &self,
        request: Request,
        delegate: impl FnOnce(&C, Request) -> Result<Response, EventAuthorityError>,
    ) -> Result<Response, EventAuthorityError>
    where
        Request: Serialize + DeserializeOwned,
        Response: Serialize + DeserializeOwned,
    {
        let request = serde_round_trip(&request, "request")?;
        match delegate(&self.inner, request) {
            Ok(response) => serde_round_trip(&response, "response"),
            Err(error) => Err(serde_round_trip(&error, "error")?),
        }
    }
}

impl<C> EventAuthorityContract for SerdeLoopbackEventAuthorityClient<C>
where
    C: EventAuthorityContract,
{
    fn durable_append(
        &self,
        request: DurableAppendRequest,
    ) -> Result<AppendReceipt, EventAuthorityError> {
        self.call(request, EventAuthorityContract::durable_append)
    }

    fn best_effort_append(
        &self,
        request: BestEffortAppendRequest,
    ) -> Result<BestEffortAppendReceipt, EventAuthorityError> {
        self.call(request, EventAuthorityContract::best_effort_append)
    }

    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        self.call(request, EventAuthorityContract::replay)
    }

    fn subscription_poll(
        &self,
        request: SubscriptionPollRequest,
    ) -> Result<EventPage, EventAuthorityError> {
        self.call(request, EventAuthorityContract::subscription_poll)
    }

    fn watermark(&self, request: WatermarkRequest) -> Result<EventWatermark, EventAuthorityError> {
        self.call(request, EventAuthorityContract::watermark)
    }

    fn health(&self, request: HealthRequest) -> Result<EventHealthReport, EventAuthorityError> {
        self.call(request, EventAuthorityContract::health)
    }

    fn flow(&self, request: FlowRequest) -> Result<EventFlowReport, EventAuthorityError> {
        self.call(request, EventAuthorityContract::flow)
    }

    fn trace(&self, request: TraceRequest) -> Result<EventTraceReport, EventAuthorityError> {
        self.call(request, EventAuthorityContract::trace)
    }

    fn session(
        &self,
        request: SessionRequest,
    ) -> Result<SessionTimelineReport, EventAuthorityError> {
        self.call(request, EventAuthorityContract::session)
    }
}

fn serde_round_trip<T>(value: &T, product: &str) -> Result<T, EventAuthorityError>
where
    T: Serialize + DeserializeOwned,
{
    let bytes = serde_json::to_vec(value).map_err(|error| EventAuthorityError::Internal {
        message: format!("failed to serialize loopback {product}: {error}"),
    })?;
    serde_json::from_slice(&bytes).map_err(|error| EventAuthorityError::Internal {
        message: format!("failed to deserialize loopback {product}: {error}"),
    })
}
