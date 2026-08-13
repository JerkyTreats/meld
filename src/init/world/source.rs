//! Theory-source provisioning into the XDG theory root.
//!
//! Owner: root init. A theory source directory carries canonical authored
//! bodies in the shipped `<kind>.<id>.json` layout — the same layout the
//! repository's `theory/` packages use. Provisioning copies each body into
//! the XDG theory root under the loader convention in [`super::theory`],
//! validating that every body's content identity matches the stewardship
//! selection before anything is written.
//!
//! Provisioning writes only under the XDG config home — never under the
//! target workspace and never into durable stores. Installing provisioned
//! theory into domain registries stays with the staged pipeline; this
//! module only puts files where the loaders resolve them.
//!
//! Idempotency is byte identity: an unchanged body is reported unchanged,
//! a changed body overwrites as a new authored revision.

use std::path::{Path, PathBuf};

use meld_world_model::agent::AgentCurationRuleConfig;
use meld_world_model::belief::{BeliefConfigLoader, ConfiguredOutcomeMappingSet};
use meld_world_model::strategy::{validate_strategy_theory_package, StrategyTheoryPackage};

use crate::config::SelectedStewardshipPackage;
use crate::docs::claim_validation::DocsClaimPolicy;
use crate::error::ApiError;
use crate::init::world::theory::{
    belief_family_config_path, claim_policy_config_path, curation_rule_config_path,
    outcome_mapping_config_path, strategy_theory_config_path,
};

/// Disposition of one provisioned theory body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionedBody {
    /// Theory kind label, for example `belief_family`.
    pub kind: String,
    /// Destination path under the XDG theory root.
    pub destination: PathBuf,
    /// Whether the write changed anything.
    pub changed: bool,
}

/// Report of one theory-source provisioning run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TheorySourceReport {
    /// Every provisioned body in deterministic destination order.
    pub bodies: Vec<ProvisionedBody>,
}

/// Provision a theory source directory for one stewardship selection.
///
/// Requires the source to carry the belief family, outcome mapping, and
/// curation rule the selection names. Strategy and capability vocabulary
/// are published by the selected PDS and are not legacy planning files.
pub fn provision_theory_source(
    source_dir: &Path,
    package: &SelectedStewardshipPackage,
) -> Result<TheorySourceReport, ApiError> {
    if !source_dir.is_dir() {
        return Err(ApiError::ConfigError(format!(
            "theory source '{}' is not a directory",
            source_dir.display()
        )));
    }
    let mut prepared = Vec::new();

    let family_raw = read_single(source_dir, "belief_family")?;
    let family = BeliefConfigLoader::load_json(&family_raw)
        .map_err(|error| source_error(source_dir, "belief_family", error))?;
    if family.config.family_id != package.belief_family_id {
        return Err(ApiError::ConfigError(format!(
            "theory source belief family declares family_id '{}' but the selection names '{}'",
            family.config.family_id, package.belief_family_id
        )));
    }
    let destination = belief_family_config_path(&package.belief_family_id)?;
    prepared.push(("belief_family", destination, family_raw));

    let mapping_raw = read_single(source_dir, "outcome_interpretation")?;
    let mapping: meld_world_model::belief::OutcomeMappingSetConfig =
        serde_json::from_str(&mapping_raw)
            .map_err(|error| source_error(source_dir, "outcome_interpretation", error))?;
    ConfiguredOutcomeMappingSet::new(mapping.clone())
        .map_err(|error| source_error(source_dir, "outcome_interpretation", error))?;
    if mapping.mapping_id != package.evidence_mapping_id {
        return Err(ApiError::ConfigError(format!(
            "theory source outcome mapping declares mapping_id '{}' but the selection names '{}'",
            mapping.mapping_id, package.evidence_mapping_id
        )));
    }
    let destination = outcome_mapping_config_path(&package.evidence_mapping_id)?;
    prepared.push(("outcome_interpretation", destination, mapping_raw));

    let rule_raw = read_single(source_dir, "curation_rule")?;
    let rule: AgentCurationRuleConfig = serde_json::from_str(&rule_raw)
        .map_err(|error| source_error(source_dir, "curation_rule", error))?;
    rule.validate()
        .map_err(|error| source_error(source_dir, "curation_rule", error))?;
    let destination = curation_rule_config_path(&package.curation_rule_id)?;
    prepared.push(("curation_rule", destination, rule_raw));

    let strategy_raw = read_single(source_dir, "strategy_theory")?;
    let strategy: StrategyTheoryPackage = serde_json::from_str(&strategy_raw)
        .map_err(|error| source_error(source_dir, "strategy_theory", error))?;
    validate_strategy_theory_package(&strategy)
        .map_err(|error| source_error(source_dir, "strategy_theory", error))?;
    if strategy.snapshot.theory_id != package.strategy_theory_id {
        return Err(ApiError::ConfigError(format!(
            "theory source Strategy package declares theory_id '{}' but the selection names '{}'",
            strategy.snapshot.theory_id, package.strategy_theory_id
        )));
    }
    prepared.push((
        "strategy_theory",
        strategy_theory_config_path(&package.strategy_theory_id)?,
        strategy_raw,
    ));

    let claim_raw = read_single(source_dir, "claim_policy")?;
    let claim: DocsClaimPolicy = serde_json::from_str(&claim_raw)
        .map_err(|error| source_error(source_dir, "claim_policy", error))?;
    claim
        .validate()
        .map_err(|error| source_error(source_dir, "claim_policy", error))?;
    if claim.policy_id != package.claim_policy_id {
        return Err(ApiError::ConfigError(format!(
            "theory source claim policy declares policy_id '{}' but the selection names '{}'",
            claim.policy_id, package.claim_policy_id
        )));
    }
    prepared.push((
        "claim_policy",
        claim_policy_config_path(&package.claim_policy_id)?,
        claim_raw,
    ));

    let bodies = prepared
        .into_iter()
        .map(|(kind, destination, raw)| write_body(kind, &destination, &raw))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(TheorySourceReport { bodies })
}

/// Locate exactly one `<kind>.<id>.json` body and return its content.
fn read_single(source_dir: &Path, kind: &str) -> Result<String, ApiError> {
    let path = find_single(source_dir, kind)?.ok_or_else(|| {
        ApiError::ConfigError(format!(
            "theory source '{}' carries no {kind} body",
            source_dir.display()
        ))
    })?;
    read_file(&path)
}

/// Locate at most one `<kind>.<id>.json` body in the source directory.
fn find_single(source_dir: &Path, kind: &str) -> Result<Option<PathBuf>, ApiError> {
    let prefix = format!("{kind}.");
    let mut matches: Vec<PathBuf> = std::fs::read_dir(source_dir)
        .map_err(|error| {
            ApiError::ConfigError(format!(
                "cannot read theory source '{}': {error}",
                source_dir.display()
            ))
        })?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".json"))
        })
        .collect();
    matches.sort();
    match matches.len() {
        0 => Ok(None),
        1 => Ok(Some(matches.remove(0))),
        _ => Err(ApiError::ConfigError(format!(
            "theory source '{}' carries more than one {kind} body",
            source_dir.display()
        ))),
    }
}

fn read_file(path: &Path) -> Result<String, ApiError> {
    std::fs::read_to_string(path).map_err(|error| {
        ApiError::ConfigError(format!(
            "cannot read theory source body '{}': {error}",
            path.display()
        ))
    })
}

/// Write one body under the loader convention, byte-idempotently.
fn write_body(kind: &str, destination: &Path, raw: &str) -> Result<ProvisionedBody, ApiError> {
    let existing = std::fs::read_to_string(destination).ok();
    let changed = existing.as_deref() != Some(raw);
    if changed {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                ApiError::ConfigError(format!(
                    "cannot create theory directory '{}': {error}",
                    parent.display()
                ))
            })?;
        }
        std::fs::write(destination, raw).map_err(|error| {
            ApiError::ConfigError(format!(
                "cannot write theory body '{}': {error}",
                destination.display()
            ))
        })?;
    }
    Ok(ProvisionedBody {
        kind: kind.to_string(),
        destination: destination.to_path_buf(),
        changed,
    })
}

fn source_error(source_dir: &Path, kind: &str, error: impl std::fmt::Display) -> ApiError {
    ApiError::ConfigError(format!(
        "invalid {kind} body in theory source '{}': {error}",
        source_dir.display()
    ))
}
