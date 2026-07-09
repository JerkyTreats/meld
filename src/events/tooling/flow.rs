//! Flow subcommand: renders event flow over a trailing window.

use meld_events::events::observability::{EventFlowReport, EventObservabilityPort, FlowWindow};
use meld_events::LedgerObservability;

use crate::error::ApiError;
use crate::events::tooling::{render, surface_error};

/// Event types rendered before the text view truncates with a count; the
/// JSON view always carries the full report.
const TYPE_LINES: usize = 10;

pub(super) fn run(
    port: &LedgerObservability,
    format: &str,
    window: usize,
) -> Result<String, ApiError> {
    let report = port
        .flow(FlowWindow { max_events: window })
        .map_err(|err| surface_error("flow", err))?;
    render(format, &report, render_text)
}

fn render_text(report: &EventFlowReport) -> String {
    let span = report
        .span_seconds
        .map(|seconds| format!("{seconds}s"))
        .unwrap_or_else(|| "-".to_string());
    let mut lines = vec![format!("flow: {} events over {span}", report.window_events)];
    for domain in &report.by_domain {
        lines.push(format!(
            "domain {}: {} events, last seq {}",
            domain.domain_id, domain.count, domain.last_seq
        ));
    }
    for type_flow in report.by_type.iter().take(TYPE_LINES) {
        lines.push(format!(
            "type {}: {} events",
            type_flow.event_type, type_flow.count
        ));
    }
    if report.by_type.len() > TYPE_LINES {
        lines.push(format!(
            "... and {} more",
            report.by_type.len() - TYPE_LINES
        ));
    }
    for silent in &report.silent_domains {
        lines.push(format!(
            "silent {}: last seq {} at {}",
            silent.domain_id, silent.last_seq, silent.last_recorded_at
        ));
    }
    lines.join("\n")
}
