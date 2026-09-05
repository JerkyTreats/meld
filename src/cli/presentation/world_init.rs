//! World init command presentation: report formatters.

use crate::error::ApiError;
use crate::init::world::{StageDisposition, WorldInitReport, WorldInitStage};

/// Validate the requested output format before any stage runs.
pub fn validate_world_init_format(format: &str) -> Result<(), ApiError> {
    match format {
        "text" | "json" => Ok(()),
        other => Err(ApiError::ConfigError(format!(
            "invalid format '{other}', expected 'text' or 'json'"
        ))),
    }
}

/// Render a world initialization report as text or JSON.
pub fn format_world_init_report(
    report: &WorldInitReport,
    format: &str,
) -> Result<String, ApiError> {
    validate_world_init_format(format)?;
    if format == "json" {
        return serde_json::to_string_pretty(report).map_err(|error| {
            ApiError::ConfigError(format!("report serialization failed: {error}"))
        });
    }

    let mut output = String::from("World initialization report:\n");
    if report.stage_reports.is_empty() {
        output.push_str("  no stages selected\n");
        return Ok(output);
    }
    for stage_report in &report.stage_reports {
        output.push_str(&format!(
            "  {}: {}\n",
            stage_name(stage_report.stage),
            disposition_name(stage_report.disposition)
        ));
        for record_id in &stage_report.record_ids {
            output.push_str(&format!("    {record_id}\n"));
        }
    }
    Ok(output)
}

fn stage_name(stage: WorldInitStage) -> &'static str {
    match stage {
        WorldInitStage::InstallTheory => "install-theory",
        WorldInitStage::GenesisIdentities => "genesis-identities",
        WorldInitStage::PrepareActivation => "prepare-activation",
        WorldInitStage::SeedEpistemicFacts => "seed-epistemic-facts",
    }
}

fn disposition_name(disposition: StageDisposition) -> &'static str {
    match disposition {
        StageDisposition::Applied => "applied",
        StageDisposition::Unchanged => "unchanged",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init::world::WorldInitStageReport;

    fn sample_report() -> WorldInitReport {
        WorldInitReport {
            stage_reports: vec![WorldInitStageReport {
                stage: WorldInitStage::SeedEpistemicFacts,
                disposition: StageDisposition::Applied,
                record_ids: vec!["genesis::world_model::observation::a".to_string()],
            }],
        }
    }

    #[test]
    fn text_report_names_stage_disposition_and_record_ids() {
        let output = format_world_init_report(&sample_report(), "text").unwrap();
        assert!(output.contains("seed-epistemic-facts: applied"));
        assert!(output.contains("genesis::world_model::observation::a"));
    }

    #[test]
    fn json_report_round_trips_the_frozen_shape() {
        let output = format_world_init_report(&sample_report(), "json").unwrap();
        let parsed: WorldInitReport = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed, sample_report());
    }

    #[test]
    fn unknown_format_is_rejected() {
        assert!(format_world_init_report(&sample_report(), "yaml").is_err());
    }
}
