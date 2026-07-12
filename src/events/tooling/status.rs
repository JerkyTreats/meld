//! Status subcommand: renders the ledger health report.

use meld_events::{EventHealthReport, EventObservabilityCapability};

use crate::error::ApiError;
use crate::events::tooling::{render, surface_authority_error};

pub(super) fn run(
    observability: &EventObservabilityCapability,
    format: &str,
) -> Result<String, ApiError> {
    let report = observability
        .health(observability.ledger_identity())
        .map_err(|err| surface_authority_error("status", err))?;
    render(format, &report, render_text)
}

fn render_text(report: &EventHealthReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("ledger_id:           {}\n", report.ledger_id));
    scalar(&mut out, "tip_seq", report.tip_seq);
    scalar(&mut out, "committed_watermark", report.committed_watermark);
    scalar(&mut out, "retained_from", report.retained_from);
    scalar(&mut out, "dropped_events", report.dropped_events);
    out.push_str(&format!(
        "append_rate_coverage: {}\n",
        coverage_text(&report.append_rate_coverage)
    ));

    out.push_str("consumers:");
    if report.consumers.is_empty() {
        out.push_str(" (none)\n");
    } else {
        out.push('\n');
        let name_width = report
            .consumers
            .iter()
            .map(|consumer| consumer.name.len())
            .max()
            .unwrap_or(0);
        for consumer in &report.consumers {
            out.push_str(&format!(
                "  {:<name_width$}  cursor={:<8}  lag={}\n",
                consumer.name, consumer.reported_seq, consumer.lag
            ));
        }
    }

    out.push_str("append_rates:");
    if report.append_rates.is_empty() {
        out.push_str(" (none)\n");
    } else {
        out.push('\n');
        let domain_width = report
            .append_rates
            .iter()
            .map(|rate| rate.domain_id.len())
            .max()
            .unwrap_or(0);
        for rate in &report.append_rates {
            let span = rate
                .window_seconds
                .map(|seconds| format!("{seconds}s"))
                .unwrap_or_else(|| "-".to_string());
            out.push_str(&format!(
                "  {:<domain_width$}  events={:<8}  window={}\n",
                rate.domain_id, rate.events_in_window, span
            ));
        }
    }

    out
}

pub(super) fn coverage_text(coverage: &meld_events::EventReadCoverage) -> String {
    let scanned = match (coverage.scanned_from_seq, coverage.scanned_through_seq) {
        (Some(from), Some(through)) => format!("{from}..={through}"),
        _ => "empty".to_string(),
    };
    format!(
        "retained_from={} tip={} scanned={} truncation={}",
        coverage.retained_from,
        coverage.tip_seq,
        scanned,
        truncation_label(coverage.truncation)
    )
}

pub(super) fn truncation_label(truncation: meld_events::CoverageTruncation) -> &'static str {
    match truncation {
        meld_events::CoverageTruncation::None => "none",
        meld_events::CoverageTruncation::Before => "before",
        meld_events::CoverageTruncation::After => "after",
        meld_events::CoverageTruncation::Both => "both",
    }
}

fn scalar(out: &mut String, key: &str, value: u64) {
    // Width fits the longest scalar key, "committed_watermark:".
    out.push_str(&format!("{:<20} {value}\n", format!("{key}:")));
}

#[cfg(test)]
mod tests {
    use meld_events::{CoverageTruncation, EventReadCoverage};

    use super::*;

    #[test]
    fn coverage_text_is_stable_and_lowercase() {
        assert_eq!(
            coverage_text(&EventReadCoverage {
                retained_from: 3,
                tip_seq: 90,
                scanned_from_seq: Some(10),
                scanned_through_seq: Some(90),
                truncation: CoverageTruncation::Both,
            }),
            "retained_from=3 tip=90 scanned=10..=90 truncation=both"
        );
        assert_eq!(
            coverage_text(&EventReadCoverage {
                retained_from: 1,
                tip_seq: 0,
                scanned_from_seq: None,
                scanned_through_seq: None,
                truncation: CoverageTruncation::None,
            }),
            "retained_from=1 tip=0 scanned=empty truncation=none"
        );
    }
}
