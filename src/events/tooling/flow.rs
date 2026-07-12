//! Flow subcommand: renders event flow over a trailing window.

use meld_events::events::observability::{EventFlowReport, EventObservabilityPort, FlowWindow};
use meld_events::LedgerObservability;

use crate::error::ApiError;
use crate::events::tooling::status::coverage_text;
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
    let mut lines = vec![
        format!("ledger_id: {}", report.ledger_id),
        format!("flow: {} events over {span}", report.window_events),
        format!("coverage: {}", coverage_text(&report.coverage)),
        format!(
            "silent_domain_coverage: {}",
            coverage_text(&report.silent_domain_coverage)
        ),
    ];
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

#[cfg(test)]
mod tests {
    use meld_events::{CoverageTruncation, EventReadCoverage, LedgerIdentity};

    use super::*;

    fn coverage(truncation: CoverageTruncation) -> EventReadCoverage {
        EventReadCoverage {
            retained_from: 2,
            tip_seq: 11,
            scanned_from_seq: Some(7),
            scanned_through_seq: Some(11),
            truncation,
        }
    }

    #[test]
    fn flow_coverage_rendering_uses_stable_common_shape() {
        let report = EventFlowReport {
            ledger_id: "00000000-0000-0000-0000-000000000001"
                .parse::<LedgerIdentity>()
                .unwrap(),
            coverage: coverage(CoverageTruncation::Before),
            window_events: 5,
            span_seconds: None,
            by_domain: Vec::new(),
            by_type: Vec::new(),
            silent_domains: Vec::new(),
            silent_domain_coverage: coverage(CoverageTruncation::Both),
        };

        let rendered = render_text(&report);
        let lines: Vec<_> = rendered.lines().collect();
        assert_eq!(
            lines[2],
            "coverage: retained_from=2 tip=11 scanned=7..=11 truncation=before"
        );
        assert_eq!(
            lines[3],
            "silent_domain_coverage: retained_from=2 tip=11 scanned=7..=11 truncation=both"
        );
    }
}
