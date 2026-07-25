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
//! - belief families: `$XDG_CONFIG_HOME/meld/theory/belief_families/<id>.json`
//! - curation rules:  `$XDG_CONFIG_HOME/meld/theory/curation_rules/<id>.json`
//!
//! A file's content identity must match the selection identity: a belief
//! family file whose `family_id` differs from its selected id is rejected
//! rather than silently installed under the wrong name.

use std::path::PathBuf;

use meld_world_model::agent::AgentCurationRuleConfig;
use meld_world_model::belief::{BeliefConfigLoader, BeliefFamilyConfig};

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

/// Reject identities that would escape the theory directory.
fn validate_theory_id(label: &str, id: &str) -> Result<(), ApiError> {
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
