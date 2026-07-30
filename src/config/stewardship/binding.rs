//! Pure physical binding resolution for a validated stewardship selection.
//!
//! Owner: root config. Resolution is stage 0 of Runtime Initialization:
//! it turns the validated selection plus environment into one typed value
//! and produces no state anywhere — no filesystem writes, no store opens,
//! no side effects. Every input is absolute or environment-derived, so the
//! resolved binding is identical regardless of process working directory.

use crate::config::stewardship::selection::{DocsFreshnessSelection, SelectionOrigins};
use crate::config::MerkleConfig;
use crate::error::ApiError;
use std::path::PathBuf;

/// The stewardship package selected by identity for one binding.
///
/// Identities only; the owning domains resolve them to installed theory
/// revisions during initialization stage 2. This value never carries
/// theory bodies or internal runtime actor roles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedStewardshipPackage {
    /// Stewardship expression name, for example `docs_freshness`.
    pub expression: String,
    /// Belief family identity.
    pub belief_family_id: String,
    /// Outcome-to-evidence mapping identity.
    pub evidence_mapping_id: String,
    /// Curation rule reference.
    pub curation_rule_id: String,
}

/// One resolved physical runtime binding.
///
/// Owner: root config. Later composition derives required runtime
/// registrations from this value; this module does not produce
/// registrations and never enumerates internal actor topology.
///
/// Invariants: `workspace_root` is canonical and absolute;
/// `storage_root` resolves outside `workspace_root` (workspace purity is
/// enforced by the storage path resolver and preserved here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalBinding {
    /// Canonical target workspace subtree root the expression stewards.
    pub workspace_root: PathBuf,
    /// Subject identity the expression is about.
    pub subject: String,
    /// Durable agent identity that stewards the subject.
    pub agent_id: String,
    /// Provider key, guaranteed present in the root provider map.
    pub provider_id: String,
    /// Stewardship package selected by identity.
    pub package: SelectedStewardshipPackage,
    /// External product storage root, outside the target workspace.
    pub storage_root: PathBuf,
}

impl PhysicalBinding {
    /// Resolve the physical binding for the configured docs freshness
    /// selection.
    ///
    /// Pure resolution: reads the filesystem only to canonicalize
    /// existing paths and writes nothing. Fails when no selection is
    /// configured, the selection is invalid, the provider is unknown,
    /// the target root does not exist, or the storage root would land
    /// inside the target workspace.
    pub fn resolve(config: &MerkleConfig) -> Result<Self, ApiError> {
        let selection = config.stewardship.docs_freshness.as_ref().ok_or_else(|| {
            ApiError::ConfigError(
                "no docs freshness stewardship selection is configured (stewardship.docs_freshness)"
                    .to_string(),
            )
        })?;
        Self::resolve_selection(selection, config)
    }

    fn resolve_selection(
        selection: &DocsFreshnessSelection,
        config: &MerkleConfig,
    ) -> Result<Self, ApiError> {
        // Re-validate defensively so the resolver stays total even when a
        // caller skips the load-path validation.
        selection
            .validate_sourced(&SelectionOrigins::uniform("resolved configuration"))
            .map_err(|errors| {
                let joined = errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ");
                ApiError::ConfigError(format!(
                    "invalid docs freshness stewardship selection: {joined}"
                ))
            })?;

        if !config.providers.contains_key(&selection.provider_id) {
            return Err(ApiError::ConfigError(format!(
                "stewardship.docs_freshness.provider_id '{}' does not name a configured provider",
                selection.provider_id
            )));
        }

        // The selection's target root is absolute by validation, so
        // canonicalization never consults the working directory.
        let workspace_root = selection.target_root.canonicalize().map_err(|error| {
            ApiError::ConfigError(format!(
                "stewardship.docs_freshness.target_root '{}' cannot be resolved: {error}",
                selection.target_root.display()
            ))
        })?;

        // resolve_product_root enforces the external product root: any
        // storage root inside the target workspace is rejected there.
        let storage_root = config
            .system
            .storage
            .resolve_product_root(&workspace_root)?;

        Ok(Self {
            workspace_root,
            subject: selection.subject.clone(),
            agent_id: selection.agent_id.clone(),
            provider_id: selection.provider_id.clone(),
            package: SelectedStewardshipPackage {
                expression: selection.expression.clone(),
                belief_family_id: selection.theory.belief_family_id.clone(),
                evidence_mapping_id: selection.theory.evidence_mapping_id.clone(),
                curation_rule_id: selection.theory.curation_rule_id.clone(),
            },
            storage_root,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::stewardship::selection::{StewardshipConfig, TheorySelection};
    use crate::provider::CompletionOptions;
    use crate::provider::ProviderType;
    use std::path::Path;
    use std::sync::Mutex;

    // Serializes working-directory changes; the resolver itself must not
    // depend on the working directory, which is what these tests prove.
    static CWD_MUTEX: Mutex<()> = Mutex::new(());

    fn config_with_selection(target_root: &Path, storage_root: &Path) -> MerkleConfig {
        let mut config = MerkleConfig::default();
        config.providers.insert(
            "main-provider".to_string(),
            crate::config::ProviderConfig {
                provider_name: Some("main-provider".to_string()),
                provider_type: ProviderType::Ollama,
                model: "test-model".to_string(),
                api_key: None,
                endpoint: None,
                default_options: CompletionOptions::default(),
            },
        );
        config.system.storage.product_root = Some(storage_root.to_path_buf());
        config.stewardship = StewardshipConfig {
            docs_freshness: Some(DocsFreshnessSelection {
                expression: "docs_freshness".to_string(),
                target_root: target_root.to_path_buf(),
                subject: "docs".to_string(),
                agent_id: "docs-steward".to_string(),
                provider_id: "main-provider".to_string(),
                theory: TheorySelection {
                    belief_family_id: "docs_freshness".to_string(),
                    evidence_mapping_id: "docs_freshness_outcome_interpretation_v1".to_string(),
                    curation_rule_id: "docs_freshness".to_string(),
                },
            }),
        };
        config
    }

    fn entry_count(path: &Path) -> usize {
        std::fs::read_dir(path).unwrap().count()
    }

    #[test]
    fn resolves_one_typed_binding() {
        let workspace = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let storage = external.path().join("runtime");
        let config = config_with_selection(workspace.path(), &storage);

        let binding = PhysicalBinding::resolve(&config).unwrap();

        assert_eq!(
            binding.workspace_root,
            workspace.path().canonicalize().unwrap()
        );
        assert_eq!(binding.subject, "docs");
        assert_eq!(binding.agent_id, "docs-steward");
        assert_eq!(binding.provider_id, "main-provider");
        assert_eq!(binding.package.expression, "docs_freshness");
        assert_eq!(binding.package.belief_family_id, "docs_freshness");
        assert_eq!(
            binding.package.evidence_mapping_id,
            "docs_freshness_outcome_interpretation_v1"
        );
        assert_eq!(binding.package.curation_rule_id, "docs_freshness");
        assert!(!binding.storage_root.starts_with(&binding.workspace_root));
    }

    #[test]
    fn resolution_is_identical_across_working_directories() {
        let _guard = CWD_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let workspace = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let cwd_a = tempfile::tempdir().unwrap();
        let cwd_b = tempfile::tempdir().unwrap();
        let storage = external.path().join("runtime");
        let config = config_with_selection(workspace.path(), &storage);
        let original_cwd = std::env::current_dir().unwrap();

        std::env::set_current_dir(cwd_a.path()).unwrap();
        let from_a = PhysicalBinding::resolve(&config).unwrap();
        std::env::set_current_dir(cwd_b.path()).unwrap();
        let from_b = PhysicalBinding::resolve(&config).unwrap();
        std::env::set_current_dir(original_cwd).unwrap();

        assert_eq!(from_a, from_b);
    }

    #[test]
    fn resolution_creates_no_state_under_the_target_workspace() {
        let workspace = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let storage = external.path().join("runtime");
        let config = config_with_selection(workspace.path(), &storage);
        assert_eq!(entry_count(workspace.path()), 0);
        assert_eq!(entry_count(external.path()), 0);

        PhysicalBinding::resolve(&config).unwrap();

        // Resolution must not create the storage root either: stage 1
        // owns opening, stage 0 only names locations.
        assert_eq!(entry_count(workspace.path()), 0);
        assert_eq!(entry_count(external.path()), 0);
    }

    #[test]
    fn storage_root_inside_the_workspace_is_rejected() {
        let workspace = tempfile::tempdir().unwrap();
        let inside = workspace.path().join("runtime");
        let config = config_with_selection(workspace.path(), &inside);

        let error = PhysicalBinding::resolve(&config).unwrap_err();

        assert!(matches!(error, ApiError::ProductRootInsideWorkspace { .. }));
    }

    #[test]
    fn unknown_provider_is_an_unresolved_binding_error() {
        let workspace = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let mut config = config_with_selection(workspace.path(), &external.path().join("runtime"));
        config.providers.clear();

        let error = PhysicalBinding::resolve(&config).unwrap_err();

        assert!(matches!(
            error,
            ApiError::ConfigError(message) if message.contains("provider_id 'main-provider'")
        ));
    }

    #[test]
    fn missing_selection_is_an_error() {
        let config = MerkleConfig::default();

        let error = PhysicalBinding::resolve(&config).unwrap_err();

        assert!(matches!(
            error,
            ApiError::ConfigError(message) if message.contains("stewardship.docs_freshness")
        ));
    }
}
