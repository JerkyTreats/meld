//! Served Event reads preserve native identity, cursors and retention coverage.

use super::{flow, render, session, status, tail, trace, validate_format};
use crate::cli::EventCommands;
use crate::error::ApiError;
use crate::runtime::assembly::ProductRuntimeAssembly;
use meld_events::remote::{
    FlowRequest, HealthRequest, NewestPageRequest, SessionRequest, TraceRequest,
};
use meld_events::{
    EventFlowReport, EventHealthReport, EventPage, EventTraceReport, FlowWindow, LedgerCursor,
    LedgerIdentity, ReplayRequest, SessionTimelineReport, SubscriptionPollRequest,
};
use serde::{de::DeserializeOwned, Serialize};
use std::io::Write;
use std::path::Path;
use std::time::Duration;

fn error(message: impl ToString) -> ApiError {
    ApiError::ConfigError(message.to_string())
}

fn post<T: DeserializeOwned>(
    url: &str,
    path: &str,
    request: &impl Serialize,
) -> Result<T, ApiError> {
    let response = ureq::post(&format!("{url}{path}"))
        .timeout(Duration::from_secs(3))
        .send_json(request)
        .map_err(|failure| match failure {
            ureq::Error::Status(code, response) => error(
                response
                    .into_json::<crate::serve::routes::RouteError>()
                    .map(|body| body.error)
                    .unwrap_or_else(|_| format!("event read returned HTTP {code}")),
            ),
            failure => error(failure),
        })?;
    response.into_json().map_err(error)
}

pub fn try_live(
    workspace: &Path,
    config: &crate::config::MerkleConfig,
    command: &EventCommands,
) -> Option<Result<String, ApiError>> {
    let target = ProductRuntimeAssembly::describe_for_workspace(workspace, config).ok()?;
    let (url, instance) = match crate::runtime::managed::discover_live(&target) {
        Ok(Some(live)) => live,
        Ok(None) => return None,
        Err(error) => return Some(Err(error)),
    };
    Some((|| {
        let ledger_id: LedgerIdentity = ureq::get(&format!("{url}/v1/ledger"))
            .timeout(Duration::from_secs(2))
            .call()
            .map_err(error)?
            .into_json()
            .map_err(error)?;
        match command {
            EventCommands::Status { format } => {
                validate_format(format)?;
                let report: EventHealthReport =
                    post(&url, "/v1/events/health", &HealthRequest { ledger_id })?;
                render(format, &report, status::render_text)
            }
            EventCommands::Flow { format, window } => {
                validate_format(format)?;
                let report: EventFlowReport = post(
                    &url,
                    "/v1/events/flow",
                    &FlowRequest {
                        ledger_id,
                        window: FlowWindow {
                            max_events: *window,
                        },
                    },
                )?;
                render(format, &report, flow::render_text)
            }
            EventCommands::Trace {
                format,
                object,
                stream,
                seq,
            } => {
                validate_format(format)?;
                let subject = trace::parse_subject(object.as_deref(), stream.as_deref(), *seq)?;
                let report: EventTraceReport = post(
                    &url,
                    "/v1/events/trace",
                    &TraceRequest { ledger_id, subject },
                )?;
                render(format, &report, trace::format_text)
            }
            EventCommands::Session { format, session_id } => {
                validate_format(format)?;
                let report: SessionTimelineReport = post(
                    &url,
                    "/v1/events/session",
                    &SessionRequest {
                        ledger_id,
                        session_id: session_id.clone(),
                    },
                )?;
                if report.events_returned == 0 {
                    return Err(error("no retained ledger events for this session; use event tail to discover session identities"));
                }
                render(format, &report, session::render_text)
            }
            EventCommands::Tail {
                format,
                after,
                limit,
                follow,
            } => {
                validate_format(format)?;
                if *limit == 0 || *limit > 1000 {
                    return Err(error("limit must be between 1 and 1000"));
                }
                if !follow {
                    let page: EventPage = match after {
                        Some(after_seq) => post(
                            &url,
                            "/v1/events/replay",
                            &ReplayRequest {
                                cursor: LedgerCursor {
                                    ledger_id,
                                    after_seq: *after_seq,
                                },
                                limit: *limit,
                            },
                        )?,
                        None => post(
                            &url,
                            "/v1/events/newest_page",
                            &NewestPageRequest {
                                ledger_id,
                                limit: *limit,
                            },
                        )?,
                    };
                    return render(format, &page, tail::render_page_text);
                }
                let mut cursor = match after {
                    Some(after) => *after,
                    None => {
                        post::<EventHealthReport>(
                            &url,
                            "/v1/events/health",
                            &HealthRequest { ledger_id },
                        )?
                        .tip_seq
                    }
                };
                loop {
                    let Some((_, current)) = crate::runtime::managed::discover_live(&target)?
                    else {
                        return Ok(String::new());
                    };
                    if current.instance.instance_id != instance.instance.instance_id {
                        return Ok(String::new());
                    }
                    let page: EventPage = post(
                        &url,
                        "/v1/events/subscription_poll",
                        &SubscriptionPollRequest {
                            replay: ReplayRequest {
                                cursor: LedgerCursor {
                                    ledger_id,
                                    after_seq: cursor,
                                },
                                limit: *limit,
                            },
                            timeout_ms: 500,
                        },
                    )?;
                    if !page.records.is_empty() {
                        let mut stdout = std::io::stdout().lock();
                        if writeln!(stdout, "{}", tail::render_follow_page(format, &page)?).is_err()
                            || stdout.flush().is_err()
                        {
                            return Ok(String::new());
                        }
                    }
                    cursor = tail::advance_cursor(cursor, &page);
                }
            }
        }
    })())
}
