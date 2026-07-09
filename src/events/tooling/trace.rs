//! Trace subcommand: renders causal chains and parses subject selectors.

use meld_events::events::observability::EventObservabilityPort;
use meld_events::{
    DomainObjectRef, EventTraceReport, LedgerObservability, TraceLink, TraceSubject,
};

use crate::error::ApiError;
use crate::events::tooling::{render, surface_error};

pub(super) fn run(
    port: &LedgerObservability,
    format: &str,
    object: Option<&str>,
    stream: Option<&str>,
    seq: Option<u64>,
) -> Result<String, ApiError> {
    let subject = parse_subject(object, stream, seq)?;
    let report = port
        .trace(subject)
        .map_err(|err| surface_error("trace", err))?;
    render(format, &report, format_text)
}

/// Parses the three mutually exclusive subject flags into a trace subject.
///
/// Flag conflicts are already rejected by the parser; this guards the "no
/// subject at all" case and validates the selector formats.
fn parse_subject(
    object: Option<&str>,
    stream: Option<&str>,
    seq: Option<u64>,
) -> Result<TraceSubject, ApiError> {
    match (object, stream, seq) {
        (Some(object), None, None) => {
            let parts: Vec<&str> = object.split("::").collect();
            let [domain_id, object_kind, object_id] = parts.as_slice() else {
                return Err(ApiError::ConfigError(format!(
                    "invalid trace object '{object}', expected domain::kind::id"
                )));
            };
            let object_ref = DomainObjectRef::new(*domain_id, *object_kind, *object_id)
                .map_err(|err| ApiError::ConfigError(format!("invalid trace object: {err}")))?;
            Ok(TraceSubject::Object(object_ref))
        }
        (None, Some(stream), None) => {
            let parts: Vec<&str> = stream.split("::").collect();
            let [domain_id, stream_id] = parts.as_slice() else {
                return Err(ApiError::ConfigError(format!(
                    "invalid trace stream '{stream}', expected domain::stream"
                )));
            };
            Ok(TraceSubject::Stream {
                domain_id: (*domain_id).to_string(),
                stream_id: (*stream_id).to_string(),
            })
        }
        (None, None, Some(seq)) => Ok(TraceSubject::Record { seq }),
        _ => Err(ApiError::ConfigError(
            "trace requires exactly one subject: --object domain::kind::id, \
             --stream domain::stream, or --seq N"
                .to_string(),
        )),
    }
}

fn format_text(report: &EventTraceReport) -> String {
    let mut out = format!("trace {}\n", subject_label(&report.subject));
    for hop in &report.hops {
        // Field order mirrors tail lines so hops correlate against a tail
        // capture by eye.
        out.push_str(&format!(
            "{}  {}  {}  {}  [{}]  session={}\n",
            hop.seq,
            hop.recorded_at,
            hop.domain_id,
            hop.event_type,
            link_label(&hop.link),
            hop.session_id
        ));
    }
    out
}

fn subject_label(subject: &TraceSubject) -> String {
    match subject {
        TraceSubject::Object(object_ref) => format!("object {}", object_ref.index_key()),
        TraceSubject::Stream {
            domain_id,
            stream_id,
        } => format!("stream {domain_id}::{stream_id}"),
        TraceSubject::Record { seq } => format!("record {seq}"),
    }
}

fn link_label(link: &TraceLink) -> String {
    match link {
        TraceLink::Subject => "subject".to_string(),
        TraceLink::ObjectRef => "object_ref".to_string(),
        TraceLink::Relation { relation_type } => format!("relation:{relation_type}"),
        TraceLink::SourceFact { .. } => "source_fact".to_string(),
        TraceLink::Stream => "stream".to_string(),
    }
}
