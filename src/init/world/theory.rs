//! XDG theory configuration source for world initialization.
//!
//! Owner: root init. The stewardship selection names theory by identity;
//! this module resolves those identities to configuration bodies on disk.
//! Loading is configuration, installing is the owning domain's command —
//! nothing here writes anywhere.
//!
//! Convention: theory bodies live under the XDG config home only, never
//! under the target workspace:
//!
//! - belief families:   `$XDG_CONFIG_HOME/meld/theory/belief_families/<id>.json`
//! - curation rules:    `$XDG_CONFIG_HOME/meld/theory/curation_rules/<id>.json`
//! - outcome mappings:  `$XDG_CONFIG_HOME/meld/theory/outcome_mappings/<id>.json`
//! - planning theory:   `$XDG_CONFIG_HOME/meld/theory/planning/<expression>/`
//!   holding `methods/*.json`, `available_actions.json`, and
//!   `method_realizations.json`
//!
//! A file's content identity must match the selection identity: a belief
//! family file whose `family_id` differs from its selected id is rejected
//! rather than silently installed under the wrong name. Planning theory is
//! keyed by the stewardship expression because the selection names no
//! separate planning identity.

use std::path::PathBuf;

use meld_execution::planning::{AvailableActionSet, MethodRealizationBinding};
use meld_lang::Method;
use meld_world_model::agent::AgentCurationRuleConfig;
use meld_world_model::belief::{
    BeliefConfigLoader, BeliefFamilyConfig, ConfiguredOutcomeMappingSet, OutcomeMappingSetConfig,
};

use crate::config::xdg;
use crate::error::ApiError;

/// Root directory for theory configuration bodies.
pub fn theory_config_root() -> Result<PathBuf, ApiError> {
    Ok(xdg::config_home()?.join("meld").join("theory"))
}

/// Path of the belief family configuration file for one selected id.
pub fn belief_family_config_path(family_id: &str) -> Result<PathBuf, ApiError> {
    validate_theory_id("belief family id", family_id)?;
    Ok(theory_config_root()?
        .join("belief_families")
        .join(format!("{family_id}.json")))
}

/// Path of the curation rule configuration file for one selected id.
pub fn curation_rule_config_path(rule_id: &str) -> Result<PathBuf, ApiError> {
    validate_theory_id("curation rule id", rule_id)?;
    Ok(theory_config_root()?
        .join("curation_rules")
        .join(format!("{rule_id}.json")))
}

/// Load and validate the belief family selected by id.
///
/// Validation reuses the world-model config loader so the file that loads
/// here is exactly the file the registry will accept, and the embedded
/// `family_id` must equal the selected id.
pub fn load_belief_family_config(family_id: &str) -> Result<BeliefFamilyConfig, ApiError> {
    let path = belief_family_config_path(family_id)?;
    let raw = std::fs::read_to_string(&path).map_err(|error| {
        ApiError::ConfigError(format!(
            "cannot read belief family config '{}': {error}",
            path.display()
        ))
    })?;
    let snapshot = BeliefConfigLoader::load_json(&raw).map_err(|error| {
        ApiError::ConfigError(format!(
            "invalid belief family config '{}': {error}",
            path.display()
        ))
    })?;
    if snapshot.config.family_id != family_id {
        return Err(ApiError::ConfigError(format!(
            "belief family config '{}' declares family_id '{}' but was selected as '{}'",
            path.display(),
            snapshot.config.family_id,
            family_id
        )));
    }
    Ok(snapshot.config)
}

/// Load and validate the curation rule selected by id.
pub fn load_curation_rule_config(rule_id: &str) -> Result<AgentCurationRuleConfig, ApiError> {
    let path = curation_rule_config_path(rule_id)?;
    let raw = std::fs::read_to_string(&path).map_err(|error| {
        ApiError::ConfigError(format!(
            "cannot read curation rule config '{}': {error}",
            path.display()
        ))
    })?;
    let rule: AgentCurationRuleConfig = serde_json::from_str(&raw).map_err(|error| {
        ApiError::ConfigError(format!(
            "invalid curation rule config '{}': {error}",
            path.display()
        ))
    })?;
    rule.validate().map_err(|error| {
        ApiError::ConfigError(format!(
            "invalid curation rule config '{}': {error}",
            path.display()
        ))
    })?;
    Ok(rule)
}

/// Path of the outcome mapping set configuration file for one selected id.
pub fn outcome_mapping_config_path(mapping_id: &str) -> Result<PathBuf, ApiError> {
    validate_theory_id("outcome mapping id", mapping_id)?;
    Ok(theory_config_root()?
        .join("outcome_mappings")
        .join(format!("{mapping_id}.json")))
}

/// Root directory of planning theory for one stewardship expression.
pub fn planning_theory_root(expression: &str) -> Result<PathBuf, ApiError> {
    validate_theory_id("stewardship expression", expression)?;
    Ok(theory_config_root()?.join("planning").join(expression))
}

/// Load and validate the outcome mapping set selected by id.
///
/// Validation reuses the world-model set implementor so the file that
/// loads here is exactly the file the ingestion actor will accept, and the
/// embedded set `mapping_id` must equal the selected id.
pub fn load_outcome_mapping_config(mapping_id: &str) -> Result<OutcomeMappingSetConfig, ApiError> {
    let path = outcome_mapping_config_path(mapping_id)?;
    let raw = std::fs::read_to_string(&path).map_err(|error| {
        ApiError::ConfigError(format!(
            "cannot read outcome mapping config '{}': {error}",
            path.display()
        ))
    })?;
    let config: OutcomeMappingSetConfig = serde_json::from_str(&raw).map_err(|error| {
        ApiError::ConfigError(format!(
            "invalid outcome mapping config '{}': {error}",
            path.display()
        ))
    })?;
    ConfiguredOutcomeMappingSet::new(config.clone()).map_err(|error| {
        ApiError::ConfigError(format!(
            "invalid outcome mapping config '{}': {error}",
            path.display()
        ))
    })?;
    if config.mapping_id != mapping_id {
        return Err(ApiError::ConfigError(format!(
            "outcome mapping config '{}' declares mapping_id '{}' but was selected as '{}'",
            path.display(),
            config.mapping_id,
            mapping_id
        )));
    }
    Ok(config)
}

/// Load the planning methods installed for one stewardship expression.
///
/// Files load in deterministic path order so the method candidate order is
/// stable across boots. Verification against the capability catalog is the
/// method library's concern at assembly time, not a load concern.
pub fn load_planning_methods(expression: &str) -> Result<Vec<Method>, ApiError> {
    let dir = planning_theory_root(expression)?.join("methods");
    let entries = std::fs::read_dir(&dir).map_err(|error| {
        ApiError::ConfigError(format!(
            "cannot read planning methods directory '{}': {error}",
            dir.display()
        ))
    })?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json")
        })
        .collect();
    files.sort();
    let mut methods = Vec::new();
    for file in files {
        let raw = std::fs::read_to_string(&file).map_err(|error| {
            ApiError::ConfigError(format!(
                "cannot read planning method '{}': {error}",
                file.display()
            ))
        })?;
        let method: Method = serde_json::from_str(&raw).map_err(|error| {
            ApiError::ConfigError(format!(
                "invalid planning method '{}': {error}",
                file.display()
            ))
        })?;
        methods.push(method);
    }
    if methods.is_empty() {
        return Err(ApiError::ConfigError(format!(
            "planning methods directory '{}' contains no method files",
            dir.display()
        )));
    }
    Ok(methods)
}

/// Load the available-action set installed for one stewardship expression.
pub fn load_available_actions(expression: &str) -> Result<AvailableActionSet, ApiError> {
    let path = planning_theory_root(expression)?.join("available_actions.json");
    load_planning_json(&path, "available actions")
}

/// Load the method realizations installed for one stewardship expression.
pub fn load_method_realizations(
    expression: &str,
) -> Result<Vec<MethodRealizationBinding>, ApiError> {
    let path = planning_theory_root(expression)?.join("method_realizations.json");
    load_planning_json(&path, "method realizations")
}

fn load_planning_json<T: serde::de::DeserializeOwned>(
    path: &std::path::Path,
    label: &str,
) -> Result<T, ApiError> {
    let raw = std::fs::read_to_string(path).map_err(|error| {
        ApiError::ConfigError(format!(
            "cannot read {label} config '{}': {error}",
            path.display()
        ))
    })?;
    serde_json::from_str(&raw).map_err(|error| {
        ApiError::ConfigError(format!(
            "invalid {label} config '{}': {error}",
            path.display()
        ))
    })
}

/// Reject identities that would escape the theory directory.
pub(crate) fn validate_theory_id(label: &str, id: &str) -> Result<(), ApiError> {
    if id.trim().is_empty() {
        return Err(ApiError::ConfigError(format!("{label} must not be empty")));
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(ApiError::ConfigError(format!(
            "{label} '{id}' must not contain path separators"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theory_ids_with_path_separators_are_rejected() {
        assert!(belief_family_config_path("../escape").is_err());
        assert!(curation_rule_config_path("a/b").is_err());
        assert!(belief_family_config_path("").is_err());
    }
}
