//! `/v1` route table: contract types in, contract types out.
//!
//! Every response body is a serialized existing contract type — the event
//! authority products, the durable action records, the harness walks. The
//! routes parse method and path, decode the request shape, delegate, and
//! encode the result; nothing here computes semantic answers, so a
//! consumer of this table and a consumer of the in-process contracts can
//! never diverge.

use meld_events::remote::{
    BestEffortAppendRequest, DurableAppendRequest, EventAuthorityContract, FlowRequest,
    HealthRequest, SessionRequest, TraceRequest, WatermarkRequest,
};
use meld_events::{ReplayRequest, SubscriptionPollRequest};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::harness::eligibility::EligibilityQuestion;
use crate::harness::projections::{
    parent, subagent, user, ParentProjectionRequest, SubagentProjectionRequest,
};
use crate::harness::walk::ThreadSubject;
use crate::runtime::contracts::RuntimeStatusReader as _;
use crate::serve::sources::ServeSources;

/// One dispatched response: an HTTP status and a JSON body.
pub struct RouteResponse {
    /// HTTP status code.
    pub status: u16,
    /// JSON body bytes.
    pub body: Vec<u8>,
}

/// Request shape for the recent-actions report read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentActionsRequest {
    /// Maximum records returned, newest window in ascending order.
    pub limit: usize,
}

/// Request shape for the per-runtime latest report read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatestActionRequest {
    /// Supervised runtime id.
    pub runtime_id: String,
}

/// Request shape for the causal thread walk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreadWalkRequest {
    /// Identity to resolve.
    pub subject: ThreadSubject,
    /// Optional node bound override.
    #[serde(default)]
    pub max_nodes: Option<usize>,
}

/// Body served for every error, so failures are machine-readable too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteError {
    /// Human-readable failure detail.
    pub error: String,
}

/// Dispatch one request against the served sources.
pub fn dispatch(sources: &ServeSources, method: &str, path: &str, body: &[u8]) -> RouteResponse {
    match (method, path) {
        ("GET", "/v1/ledger") => {
            respond(Ok::<_, std::convert::Infallible>(sources.ledger_identity()))
        }
        ("POST", "/v1/events/durable_append") => handle(body, |request: DurableAppendRequest| {
            sources.events.durable_append(request)
        }),
        ("POST", "/v1/events/best_effort_append") => {
            handle(body, |request: BestEffortAppendRequest| {
                sources.events.best_effort_append(request)
            })
        }
        ("POST", "/v1/events/replay") => handle(body, |request: ReplayRequest| {
            sources.events.replay(request)
        }),
        ("POST", "/v1/events/subscription_poll") => {
            // The one blocking watch: a long-poll over the commit
            // watermark wait, bounded by the request's own timeout.
            handle(body, |request: SubscriptionPollRequest| {
                sources.events.subscription_poll(request)
            })
        }
        ("POST", "/v1/events/watermark") => handle(body, |request: WatermarkRequest| {
            sources.events.watermark(request)
        }),
        ("POST", "/v1/events/health") => handle(body, |request: HealthRequest| {
            sources.events.health(request)
        }),
        ("POST", "/v1/events/flow") => {
            handle(body, |request: FlowRequest| sources.events.flow(request))
        }
        ("POST", "/v1/events/trace") => {
            handle(body, |request: TraceRequest| sources.events.trace(request))
        }
        ("POST", "/v1/events/session") => handle(body, |request: SessionRequest| {
            sources.events.session(request)
        }),
        ("POST", "/v1/reports/recent_actions") => handle(body, |request: RecentActionsRequest| {
            sources
                .reports
                .read_recent_actions_since(sources.action_floor, request.limit)
        }),
        ("POST", "/v1/reports/latest_for_runtime") => {
            handle(body, |request: LatestActionRequest| {
                sources
                    .reports
                    .latest_action_for_runtime_since(sources.action_floor, &request.runtime_id)
            })
        }
        ("GET", "/v1/reports/latest_snapshot") => respond(sources.reports.read_latest_snapshot()),
        ("POST", "/v1/walks/thread") => handle(body, |request: ThreadWalkRequest| {
            let mut walker = sources.thread_walker();
            if let Some(max_nodes) = request.max_nodes {
                walker = walker.with_max_nodes(max_nodes);
            }
            walker.walk(request.subject)
        }),
        ("POST", "/v1/walks/eligibility") => handle(body, |request: EligibilityQuestion| {
            sources.eligibility_walker().why_absent(request)
        }),
        ("POST", "/v1/projections/subagent") => {
            handle(body, |request: SubagentProjectionRequest| {
                subagent(&sources.projection_sources(), &request)
            })
        }
        ("POST", "/v1/projections/parent") => handle(body, |request: ParentProjectionRequest| {
            parent(&sources.projection_sources(), &request)
        }),
        ("GET", "/v1/projections/user") => respond(user(&sources.projection_sources())),
        _ => error_response(404, format!("no route for {method} {path}")),
    }
}

/// Decode a request body, run the handler, encode the result.
fn handle<Request, Response, Error>(
    body: &[u8],
    run: impl FnOnce(Request) -> Result<Response, Error>,
) -> RouteResponse
where
    Request: DeserializeOwned,
    Response: Serialize,
    Error: std::fmt::Display,
{
    let request: Request = match serde_json::from_slice(body) {
        Ok(request) => request,
        Err(error) => return error_response(400, format!("invalid request body: {error}")),
    };
    respond(run(request))
}

/// Encode one handler result.
fn respond<Response, Error>(result: Result<Response, Error>) -> RouteResponse
where
    Response: Serialize,
    Error: std::fmt::Display,
{
    match result {
        Ok(response) => match serde_json::to_vec(&response) {
            Ok(body) => RouteResponse { status: 200, body },
            Err(error) => error_response(500, format!("response encoding failed: {error}")),
        },
        // Handler failures are consumer-visible reads of authority or
        // store rejections, not server faults.
        Err(error) => error_response(409, error.to_string()),
    }
}

fn error_response(status: u16, error: String) -> RouteResponse {
    let body = serde_json::to_vec(&RouteError { error })
        .unwrap_or_else(|_| b"{\"error\":\"unencodable error\"}".to_vec());
    RouteResponse { status, body }
}
