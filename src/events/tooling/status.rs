//! Status subcommand: renders the ledger health report.

use meld_events::events::observability::{EventHealthReport, EventObservabilityPort};
use meld_events::LedgerObservability;

use crate::error::ApiError;
use crate::events::tooling::{render, surface_error};

pub(super) fn run(port: &LedgerObservability, format: &str) -> Result<String, ApiError> {
    let report = port.health().map_err(|err| surface_error("status", err))?;
    render(format, &report, render_text)
}

fn render_text(report: &EventHealthReport) -> String {
    let mut out = String::new();
    scalar(&mut out, "tip_seq", report.tip_seq);
    scalar(&mut out, "committed_watermark", report.committed_watermark);
    scalar(&mut out, "retained_from", report.retained_from);
    scalar(&mut out, "dropped_events", report.dropped_events);

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

fn scalar(out: &mut String, key: &str, value: u64) {
    // Width fits the longest scalar key, "committed_watermark:".
    out.push_str(&format!("{:<20} {value}\n", format!("{key}:")));
}
