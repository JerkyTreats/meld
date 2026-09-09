//! Pure physical binding resolution for a validated stewardship selection.
//!
//! Owner: root config. Resolution is stage 0 of Runtime Initialization:
//! it turns the validated selection plus environment into one typed value
//! and produces no state anywhere — no filesystem writes, no store opens,
//! no side effects. Every input is absolute or environment-derived, so the
//! resolved binding is identical regardless of process working directory.

use crate::config::stewardship::selection::{NamedStewardshipDeclaration, SelectionOrigins};
use crate::config::MerkleConfig;
use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The stewardship package selected by identity for one binding.
///
/// Identities only; the owning domains resolve them to installed theory
/// revisions during initialization stage 2. This value never carries
/// theory bodies or internal runtime actor roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedStewardshipPackage {
    /// Stewardship expression name, for example `docs_freshness`.
    pub expression: String,
    /// Principal whose grant bounds this package.
    pub principal_id: String,
    /// Belief family identity.
    pub belief_family_id: String,
    /// Outcome-to-evidence mapping identity.
    pub evidence_mapping_id: String,
    /// Curation rule reference.
    pub curation_rule_id: String,
    /// Standing maintained-condition identity.
    pub maintained_condition_id: String,
    /// Complete Strategy theory package identity.
    pub strategy_theory_id: String,
    /// Effective-authority policy identity.
    pub authority_policy_id: String,
    /// Docs claim policy identity.
    #[serde(default)]
    pub claim_policy_id: String,
}

/// One resolved physical runtime binding.
///
/// Owner: root config. Later composition derives required runtime
/// registrations from this value; this module does not produce
/// registrations and never enumerates internal actor topology.
///
/// A declared workspace is canonical and absolute, and product storage
/// remains outside it. Runtime subjects need neither a workspace nor a provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalBinding {
    /// Exact resource references carried intact into owner preparation.
    pub bindings: std::collections::BTreeMap<String, super::activation::PhysicalBindingRef>,
    /// Optional canonical workspace supplied to the selected owner implementations.
    pub workspace_root: Option<PathBuf>,
    /// Subject identity the expression is about.
    pub subject: meld_events::DomainObjectRef,
    /// Durable agent identity that stewards the subject.
    pub agent_id: String,
    /// Optional provider key, validated against the root map when declared.
    pub provider_id: Option<String>,
    /// Stewardship package selected by identity.
    pub package: SelectedStewardshipPackage,
    /// External product storage root, outside the target workspace.
    pub storage_root: PathBuf,
}

impl PhysicalBinding {
    /// Resolve the only configured stewardship declaration.
    ///
    /// Compatibility entry point for callers that predate named declaration
    /// collections.
    // TODO compat-shim: remove after all callers select through resolve_all
    // or resolve_for_target and the single-selection characterization tests
    // remain green.
    pub fn resolve(config: &MerkleConfig) -> Result<Self, ApiError> {
        let mut bindings = Self::resolve_all(config)?;
        match bindings.len() {
            1 => Ok(bindings.remove(0)),
            0 => Err(ApiError::ConfigError(
                "no stewardship declaration is configured".to_string(),
            )),
            count => Err(ApiError::ConfigError(format!(
                "expected one stewardship declaration, found {count}"
            ))),
        }
    }

    /// Resolve every configured declaration in deterministic id order.
    pub fn resolve_all(config: &MerkleConfig) -> Result<Vec<Self>, ApiError> {
        config
            .stewardship
            .validate_sourced(&SelectionOrigins::uniform("resolved configuration"))
            .map_err(selection_errors)?;
        config
            .stewardship
            .lowered_declarations()
            .map_err(selection_errors)?
            .into_iter()
            .map(|named| Self::resolve_declaration(&named, config))
            .collect()
    }

    /// Resolve at most one declaration for an addressed workspace target.
    ///
    /// Ambiguous target ownership fails instead of depending on declaration
    /// order.
    pub fn resolve_for_target(
        config: &MerkleConfig,
        target: &std::path::Path,
    ) -> Result<Option<Self>, ApiError> {
        config
            .stewardship
            .validate_sourced(&SelectionOrigins::uniform("resolved configuration"))
            .map_err(selection_errors)?;
        let mut matches = Vec::new();
        for named in config
            .stewardship
            .lowered_declarations()
            .map_err(selection_errors)?
        {
            let addresses_target = match &named.declaration.target_root {
                None => true,
                Some(root) => {
                    root == target
                        || target.canonicalize().is_ok_and(|target| {
                            root.canonicalize().is_ok_and(|root| root == target)
                        })
                }
            };
            if addresses_target {
                matches.push(Self::resolve_declaration(&named, config)?);
            }
        }
        match matches.len() {
            0 => Ok(None),
            1 => Ok(matches.pop()),
            count => Err(ApiError::ConfigError(format!(
                "{count} stewardship declarations target '{}'; select one with --assignment",
                target.display()
            ))),
        }
    }

    fn resolve_declaration(
        named: &NamedStewardshipDeclaration,
        config: &MerkleConfig,
    ) -> Result<Self, ApiError> {
        let selection = &named.declaration;
        let path = &named.source_path;

        if let Some(provider_id) = &selection.provider_id {
            if !config.providers.contains_key(provider_id) {
                return Err(ApiError::ConfigError(format!(
                    "{path}.provider_id '{}' does not name a configured provider",
                    provider_id,
                )));
            }
        }

        // The selection's target root is absolute by validation, so
        // canonicalization never consults the working directory.
        let workspace_root = selection
            .target_root
            .as_ref()
            .map(|target_root| {
                target_root.canonicalize().map_err(|error| {
                    ApiError::ConfigError(format!(
                        "{path}.target_root '{}' cannot be resolved: {error}",
                        target_root.display()
                    ))
                })
            })
            .transpose()?;

        // resolve_product_root enforces the external product root: any
        // storage root inside the target workspace is rejected there.
        let storage_root = match &workspace_root {
            Some(root) => config.system.storage.resolve_product_root(root)?,
            None => config.system.storage.resolve_unscoped_product_root()?,
        };

        Ok(Self {
            bindings: selection.bindings.clone(),
            workspace_root,
            subject: selection.subject.clone(),
            agent_id: selection.agent_id.clone(),
            provider_id: selection.provider_id.clone(),
            package: SelectedStewardshipPackage {
                expression: selection.expression.clone(),
                principal_id: selection.principal_id.clone(),
                belief_family_id: selection.theory.belief_family_id.clone(),
                evidence_mapping_id: selection.theory.evidence_mapping_id.clone(),
                curation_rule_id: selection.theory.curation_rule_id.clone(),
                maintained_condition_id: selection.theory.maintained_condition_id.clone(),
                strategy_theory_id: selection.theory.strategy_theory_id.clone(),
                authority_policy_id: selection.theory.authority_policy_id.clone(),
                claim_policy_id: selection.theory.claim_policy_id.clone(),
            },
            storage_root,
        })
    }

    /// Independent assignment address for heads, sessions and mutable owner state.
    pub fn assignment_scope_id(&self) -> String {
        super::assignment::assignment_scope_id(
            &self.package.expression,
            &self.package.principal_id,
            &self.subject,
            [self.agent_id.as_str()],
        )
    }

    /// Exact structural bindings supplied to selected owner implementations.
    pub fn activation_bindings(
        &self,
    ) -> std::collections::BTreeMap<String, super::activation::PhysicalBindingRef> {
        use super::activation::PhysicalBindingRef;
        let mut bindings = self.bindings.clone();
        bindings.extend([
            (
                "subject".into(),
                PhysicalBindingRef::ConfigRef(self.subject.object_id.clone()),
            ),
            (
                "agent".into(),
                PhysicalBindingRef::ConfigRef(self.agent_id.clone()),
            ),
        ]);
        if let Some(root) = &self.workspace_root {
            bindings.insert(
                "workspace".into(),
                PhysicalBindingRef::WorkspaceRef(root.display().to_string()),
            );
        }
        if let Some(provider) = &self.provider_id {
            bindings.insert(
                "provider".into(),
                PhysicalBindingRef::ProviderRef(provider.clone()),
            );
        }
        bindings
    }

    pub fn owner_binding_values(&self) -> std::collections::BTreeMap<String, String> {
        use super::activation::PhysicalBindingRef;
        self.activation_bindings()
            .into_iter()
            .map(|(key, binding)| {
                let value = match binding {
                    PhysicalBindingRef::WorkspaceRef(value)
                    | PhysicalBindingRef::ProviderRef(value)
                    | PhysicalBindingRef::CredentialRef(value)
                    | PhysicalBindingRef::EndpointRef(value)
                    | PhysicalBindingRef::ExecutableRef(value)
                    | PhysicalBindingRef::ConfigRef(value) => value,
                };
                (key, value)
            })
            .collect()
    }
}

fn selection_errors(errors: Vec<crate::config::SelectionFieldError>) -> ApiError {
    let joined = errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ");
    ApiError::ConfigError(format!("invalid stewardship declaration: {joined}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::stewardship::selection::{
        DocsFreshnessSelection, StewardshipConfig, StewardshipDeclaration, TheorySelection,
    };
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
            declarations: Default::default(),
            docs_freshness: Some(DocsFreshnessSelection {
                expression: "docs_freshness".to_string(),
                target_root: target_root.to_path_buf(),
                subject: "docs".to_string(),
                agent_id: "docs-steward".to_string(),
                principal_id: "workspace-owner".to_string(),
                provider_id: "main-provider".to_string(),
                theory: TheorySelection {
                    belief_family_id: "docs_freshness".to_string(),
                    evidence_mapping_id: "docs_freshness_outcome_interpretation_v1".to_string(),
                    curation_rule_id: "docs_freshness".to_string(),
                    maintained_condition_id: "docs_freshness".to_string(),
                    strategy_theory_id: "docs_freshness".to_string(),
                    authority_policy_id: "docs_workspace_local".to_string(),
                    claim_policy_id: "docs-claims-strict-v1".to_string(),
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
            Some(workspace.path().canonicalize().unwrap())
        );
        assert_eq!(
            binding.subject,
            meld_events::DomainObjectRef::new("workspace_fs", "node", "docs").unwrap()
        );
        assert_eq!(binding.agent_id, "docs-steward");
        assert_eq!(binding.provider_id.as_deref(), Some("main-provider"));
        assert_eq!(binding.package.expression, "docs_freshness");
        assert_eq!(binding.package.belief_family_id, "docs_freshness");
        assert_eq!(
            binding.package.evidence_mapping_id,
            "docs_freshness_outcome_interpretation_v1"
        );
        assert_eq!(binding.package.curation_rule_id, "docs_freshness");
        assert_eq!(binding.package.maintained_condition_id, "docs_freshness");
        assert!(!binding
            .storage_root
            .starts_with(binding.workspace_root.as_ref().unwrap()));
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
    fn runtime_declaration_resolves_without_a_workspace_or_provider() {
        let external = tempfile::tempdir().unwrap();
        let unavailable = external.path().join("no-workspace");
        let mut config = config_with_selection(&unavailable, &external.path().join("runtime"));
        let legacy = config.stewardship.docs_freshness.take().unwrap();
        config.providers.clear();
        let subject = meld_events::DomainObjectRef::new("runtime", "instance", "meld").unwrap();
        let declaration: StewardshipDeclaration = serde_json::from_value(serde_json::json!({
            "expression": "startup", "subject": subject,
            "agent_id": "startup-agent", "principal_id": "runtime-owner",
            "theory": legacy.theory,
        }))
        .unwrap();
        config
            .stewardship
            .declarations
            .insert("startup".into(), declaration);
        let binding = PhysicalBinding::resolve_for_target(&config, &unavailable)
            .unwrap()
            .unwrap();
        assert_eq!(binding.subject, subject);
        assert!(binding.workspace_root.is_none());
        assert!(binding.provider_id.is_none());
        assert!(!binding.activation_bindings().contains_key("workspace"));
        assert!(!binding.activation_bindings().contains_key("provider"));
        assert!(!unavailable.exists());
        assert!(!binding.storage_root.exists());
        config.system.storage.product_root = None;
        assert!(PhysicalBinding::resolve_for_target(&config, &unavailable).is_err());
        config.system.storage.product_root = Some("relative-product".into());
        assert!(PhysicalBinding::resolve_for_target(&config, &unavailable).is_err());
        assert!(!unavailable.exists());
        assert!(!binding.storage_root.exists());
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
            ApiError::ConfigError(message) if message.contains("no stewardship declaration")
        ));
    }

    #[test]
    fn canonical_expression_resolves_without_name_dispatch() {
        let workspace = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let storage = external.path().join("runtime");
        let mut config = config_with_selection(workspace.path(), &storage);
        let legacy = config.stewardship.docs_freshness.take().unwrap();
        config.stewardship.declarations.insert(
            "repository".to_string(),
            StewardshipDeclaration {
                bindings: Default::default(),
                expression: "repository_health".to_string(),
                ..legacy.into()
            },
        );

        let binding = PhysicalBinding::resolve_for_target(&config, workspace.path())
            .unwrap()
            .unwrap();

        assert_eq!(binding.package.expression, "repository_health");
        assert_eq!(binding.package.strategy_theory_id, "docs_freshness");
    }

    #[test]
    fn duplicate_target_declarations_are_rejected_as_ambiguous() {
        let workspace = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let storage = external.path().join("runtime");
        let mut config = config_with_selection(workspace.path(), &storage);
        let legacy = config.stewardship.docs_freshness.take().unwrap();
        let first: StewardshipDeclaration = legacy.into();
        let mut second = first.clone();
        second.expression = "second_expression".to_string();
        config.stewardship.declarations = std::collections::BTreeMap::from([
            ("first".to_string(), first),
            ("second".to_string(), second),
        ]);

        let error = PhysicalBinding::resolve_for_target(&config, workspace.path()).unwrap_err();

        assert!(matches!(
            error,
            ApiError::ConfigError(message) if message.contains("2 stewardship declarations target")
        ));
    }

    #[test]
    fn unrelated_unavailable_declaration_does_not_block_target_resolution() {
        let workspace = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        let storage = external.path().join("runtime");
        let mut config = config_with_selection(workspace.path(), &storage);
        let legacy = config.stewardship.docs_freshness.take().unwrap();
        let active: StewardshipDeclaration = legacy.into();
        let mut unrelated = active.clone();
        unrelated.expression = "offline_expression".to_string();
        unrelated.target_root = Some(external.path().join("absent-workspace"));
        unrelated.provider_id = Some("offline-provider".to_string());
        config.stewardship.declarations = std::collections::BTreeMap::from([
            ("active".to_string(), active),
            ("offline".to_string(), unrelated),
        ]);

        let binding = PhysicalBinding::resolve_for_target(&config, workspace.path())
            .unwrap()
            .unwrap();

        assert_eq!(
            binding.workspace_root,
            Some(workspace.path().canonicalize().unwrap())
        );
    }
}
