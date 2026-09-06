//! Stewardship declaration selection and source-aware validation.
//!
//! Owner: root config. A declaration names what the runtime stewards and
//! which theory it does so under — always by identity. Theory bodies
//! (evidence probability, satisfaction thresholds, task package meaning,
//! stale-signal interpretation) live in their owning domains and are
//! selected here by id per Runtime Initialization stage 2.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// Origin label used when a field's config source cannot be determined.
const UNKNOWN_SOURCE: &str = "unknown configuration source";

/// Stewardship expression selections carried by the root config.
///
/// Absent selections are valid: a runtime with no stewardship selection
/// simply derives no stewardship bindings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StewardshipConfig {
    /// Canonical named stewardship declarations.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub declarations: BTreeMap<String, StewardshipDeclaration>,

    /// Legacy docs freshness selection.
    // TODO compat-shim: remove after shipped configuration and every
    // characterization fixture use stewardship.declarations and the
    // canonical declaration loading tests remain green.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs_freshness: Option<DocsFreshnessSelection>,
}

impl StewardshipConfig {
    /// True when no stewardship expression is selected.
    pub fn is_empty(&self) -> bool {
        self.declarations.is_empty() && self.docs_freshness.is_none()
    }

    /// Lower canonical and compatibility inputs into one declaration set.
    pub fn lowered_declarations(
        &self,
    ) -> Result<Vec<NamedStewardshipDeclaration>, Vec<SelectionFieldError>> {
        let mut lowered = self
            .declarations
            .iter()
            .map(
                |(declaration_id, declaration)| NamedStewardshipDeclaration {
                    declaration_id: declaration_id.clone(),
                    source_path: format!("stewardship.declarations.{declaration_id}"),
                    declaration: declaration.clone(),
                },
            )
            .collect::<Vec<_>>();
        if let Some(legacy) = &self.docs_freshness {
            const LEGACY_ID: &str = "docs_freshness";
            if self.declarations.contains_key(LEGACY_ID) {
                return Err(vec![SelectionFieldError {
                    source: UNKNOWN_SOURCE.to_string(),
                    field: "stewardship.docs_freshness".to_string(),
                    message: "conflicts with canonical declaration id 'docs_freshness'".to_string(),
                }]);
            }
            lowered.push(NamedStewardshipDeclaration {
                declaration_id: LEGACY_ID.to_string(),
                source_path: "stewardship.docs_freshness".to_string(),
                declaration: legacy.clone().into(),
            });
        }
        Ok(lowered)
    }

    /// Validate every lowered declaration with source attribution.
    pub fn validate_sourced(
        &self,
        origins: &SelectionOrigins,
    ) -> Result<(), Vec<SelectionFieldError>> {
        let mut errors = Vec::new();
        for (declaration_id, declaration) in &self.declarations {
            let path = format!("stewardship.declarations.{declaration_id}");
            if declaration_id.trim().is_empty() {
                errors.push(SelectionFieldError {
                    source: origins.source_for(&path),
                    field: path.clone(),
                    message: "declaration id must not be empty".to_string(),
                });
            }
            if let Err(mut declaration_errors) = declaration.validate_sourced(origins, &path) {
                errors.append(&mut declaration_errors);
            }
        }
        if let Some(legacy) = &self.docs_freshness {
            if let Err(mut legacy_errors) = legacy.validate_sourced(origins) {
                errors.append(&mut legacy_errors);
            }
            if self.declarations.contains_key("docs_freshness") {
                errors.push(SelectionFieldError {
                    source: origins.source_for("stewardship.docs_freshness"),
                    field: "stewardship.docs_freshness".to_string(),
                    message: "conflicts with canonical declaration id 'docs_freshness'".to_string(),
                });
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// One canonical named declaration after configuration lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedStewardshipDeclaration {
    /// Stable configuration-local declaration identity.
    pub declaration_id: String,
    /// Source-shaped path retained for actionable binding diagnostics.
    pub source_path: String,
    /// Canonical expression selection.
    pub declaration: StewardshipDeclaration,
}

/// Minimal canonical stewardship declaration for the implemented runtime.
///
/// This is intentionally smaller than the eventual principal-facing PDS
/// declaration. It selects identities and physical bindings without
/// embedding owner-controlled theory bodies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StewardshipDeclaration {
    /// Name of the stewardship expression this declaration instantiates.
    pub expression: String,

    /// Target workspace subtree root the expression stewards. Absolute.
    pub target_root: PathBuf,

    /// Subject identity the stewardship expression is about.
    pub subject: String,

    /// Durable agent identity that stewards the subject.
    pub agent_id: String,

    /// Principal whose grant bounds this stewardship declaration.
    pub principal_id: String,

    /// Key into the root provider map binding the model provider.
    pub provider_id: String,

    /// Selected theory identities, resolved by their owning domains.
    pub theory: TheorySelection,
}

/// One minimal docs freshness stewardship selection.
///
/// Invariants: every field is an identity or a physical target, never a
/// theory body. `target_root` must be absolute so that resolution is
/// identical regardless of process working directory; the CLI passes its
/// path argument as the explicit default target per the recorded
/// requirements-gate decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsFreshnessSelection {
    /// Name of the stewardship expression this selection instantiates.
    /// Must be `docs_freshness` for this selection shape.
    pub expression: String,

    /// Target workspace subtree root the expression stewards. Absolute.
    pub target_root: PathBuf,

    /// Subject identity the stewardship expression is about.
    pub subject: String,

    /// Durable agent identity that stewards the subject.
    pub agent_id: String,

    /// Principal whose grant bounds this stewardship selection.
    pub principal_id: String,

    /// Key into the root provider map binding the model provider.
    pub provider_id: String,

    /// Selected theory identities, resolved by their owning domains.
    pub theory: TheorySelection,
}

/// Theory selected by identity for one stewardship expression.
///
/// Identities only: the owning domains resolve each id to a content-hash
/// revision at installation time (Runtime Initialization stage 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TheorySelection {
    /// Belief family identity in the world model registry.
    pub belief_family_id: String,

    /// Outcome-to-evidence mapping identity in the world model.
    pub evidence_mapping_id: String,

    /// Curation rule reference bound at agent registration.
    pub curation_rule_id: String,

    /// Standing maintained-condition identity owned by the Agent domain.
    pub maintained_condition_id: String,

    /// Complete Strategy theory package identity.
    pub strategy_theory_id: String,

    /// Effective-authority policy identity.
    pub authority_policy_id: String,

    /// Docs claim policy identity.
    #[serde(default)]
    pub claim_policy_id: String,
}

/// One rejected docs freshness selection field with its config source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionFieldError {
    /// The config source that supplied (or omitted) the field.
    pub source: String,
    /// Full field path under the root config, for example
    /// `stewardship.docs_freshness.subject`.
    pub field: String,
    /// Why the field is invalid.
    pub message: String,
}

impl std::fmt::Display for SelectionFieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (from {}): {}", self.field, self.source, self.message)
    }
}

impl std::error::Error for SelectionFieldError {}

/// Per-field config source origins for the docs freshness selection.
///
/// Built from the merged source tree before deserialization, so each
/// validation error can name the source that supplied the invalid field.
#[derive(Debug, Clone, Default)]
pub struct SelectionOrigins {
    /// Origin used when no more specific path was captured.
    fallback_origin: Option<String>,
    /// Origin per complete field path.
    fields: HashMap<String, String>,
}

impl SelectionOrigins {
    /// Origins where every field is attributed to one named source.
    pub fn uniform(source: &str) -> Self {
        Self {
            fallback_origin: Some(source.to_string()),
            fields: HashMap::new(),
        }
    }

    /// Extract docs freshness field origins from a built config source.
    ///
    /// Origins come from the merged value tree, so a field keeps the
    /// source that last set it under the merge precedence order.
    pub fn from_source(source: &dyn config::Source) -> Self {
        let mut origins = Self::default();
        let Ok(map) = source.collect() else {
            return origins;
        };
        let Some(stewardship) = map.get("stewardship") else {
            return origins;
        };
        let config::ValueKind::Table(stewardship) = &stewardship.kind else {
            return origins;
        };
        origins.capture_table("stewardship", stewardship);
        origins
    }

    fn capture_table(&mut self, prefix: &str, table: &config::Map<String, config::Value>) {
        for (key, value) in table {
            let path = format!("{prefix}.{key}");
            if let Some(origin) = value.origin() {
                self.fields.insert(path.clone(), origin.to_string());
            }
            if let config::ValueKind::Table(nested) = &value.kind {
                self.capture_table(&path, nested);
            }
        }
    }

    fn source_for(&self, field: &str) -> String {
        self.fields
            .get(field)
            .or_else(|| {
                field
                    .rmatch_indices('.')
                    .find_map(|(index, _)| self.fields.get(&field[..index]))
            })
            .or(self.fallback_origin.as_ref())
            .cloned()
            .unwrap_or_else(|| UNKNOWN_SOURCE.to_string())
    }
}

impl StewardshipDeclaration {
    /// Validate every declaration field without expression vocabulary.
    pub fn validate_sourced(
        &self,
        origins: &SelectionOrigins,
        path: &str,
    ) -> Result<(), Vec<SelectionFieldError>> {
        validate_declaration(self, origins, path, false)
    }
}

impl From<DocsFreshnessSelection> for StewardshipDeclaration {
    fn from(selection: DocsFreshnessSelection) -> Self {
        Self {
            expression: selection.expression,
            target_root: selection.target_root,
            subject: selection.subject,
            agent_id: selection.agent_id,
            principal_id: selection.principal_id,
            provider_id: selection.provider_id,
            theory: selection.theory,
        }
    }
}

impl DocsFreshnessSelection {
    /// Validate every field, attributing each error to its config source.
    ///
    /// Pure: reads no environment and touches no filesystem, so validity
    /// is identical regardless of process working directory.
    pub fn validate_sourced(
        &self,
        origins: &SelectionOrigins,
    ) -> Result<(), Vec<SelectionFieldError>> {
        validate_declaration(
            &StewardshipDeclaration::from(self.clone()),
            origins,
            "stewardship.docs_freshness",
            true,
        )
    }
}

fn validate_declaration(
    declaration: &StewardshipDeclaration,
    origins: &SelectionOrigins,
    path: &str,
    require_docs_expression: bool,
) -> Result<(), Vec<SelectionFieldError>> {
    let mut errors = Vec::new();
    let mut reject = |field: &str, message: String| {
        let field = format!("{path}.{field}");
        errors.push(SelectionFieldError {
            source: origins.source_for(&field),
            field,
            message,
        });
    };

    if require_docs_expression && declaration.expression != "docs_freshness" {
        reject(
                "expression",
                format!(
                    "must be 'docs_freshness', got '{}'; other stewardship expressions are not selectable here",
                    declaration.expression
                ),
            );
    } else if declaration.expression.trim().is_empty() {
        reject("expression", "must not be empty".to_string());
    }
    if declaration.target_root.as_os_str().is_empty() {
        reject("target_root", "must not be empty".to_string());
    } else if !declaration.target_root.is_absolute() {
        // Relative targets would resolve against the process working
        // directory, breaking binding determinism across invocations.
        reject(
            "target_root",
            format!(
                "must be an absolute path, got '{}'",
                declaration.target_root.display()
            ),
        );
    }
    for (field, value) in [
        ("subject", &declaration.subject),
        ("agent_id", &declaration.agent_id),
        ("principal_id", &declaration.principal_id),
        ("provider_id", &declaration.provider_id),
        (
            "theory.belief_family_id",
            &declaration.theory.belief_family_id,
        ),
        (
            "theory.evidence_mapping_id",
            &declaration.theory.evidence_mapping_id,
        ),
        (
            "theory.curation_rule_id",
            &declaration.theory.curation_rule_id,
        ),
        (
            "theory.maintained_condition_id",
            &declaration.theory.maintained_condition_id,
        ),
        (
            "theory.strategy_theory_id",
            &declaration.theory.strategy_theory_id,
        ),
        (
            "theory.authority_policy_id",
            &declaration.theory.authority_policy_id,
        ),
    ] {
        if value.trim().is_empty() {
            reject(field, "must not be empty".to_string());
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_selection() -> DocsFreshnessSelection {
        DocsFreshnessSelection {
            expression: "docs_freshness".to_string(),
            target_root: PathBuf::from("/workspace/docs"),
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
        }
    }

    fn valid_declaration() -> StewardshipDeclaration {
        valid_selection().into()
    }

    #[test]
    fn valid_selection_passes() {
        let selection = valid_selection();
        let origins = SelectionOrigins::uniform("test source");
        assert!(selection.validate_sourced(&origins).is_ok());
    }

    #[test]
    fn invalid_field_identifies_source_and_field() {
        let mut selection = valid_selection();
        selection.subject = String::new();
        let origins = SelectionOrigins::uniform("/etc/xdg/meld/config.toml");

        let errors = selection.validate_sourced(&origins).unwrap_err();

        assert_eq!(errors.len(), 1);
        let rendered = errors[0].to_string();
        assert!(rendered.contains("stewardship.docs_freshness.subject"));
        assert!(rendered.contains("/etc/xdg/meld/config.toml"));
    }

    #[test]
    fn relative_target_root_is_rejected() {
        let mut selection = valid_selection();
        selection.target_root = PathBuf::from("relative/docs");
        let origins = SelectionOrigins::uniform("test source");

        let errors = selection.validate_sourced(&origins).unwrap_err();

        assert!(errors
            .iter()
            .any(|e| e.field == "stewardship.docs_freshness.target_root"
                && e.message.contains("absolute")));
    }

    #[test]
    fn wrong_expression_name_is_rejected() {
        let mut selection = valid_selection();
        selection.expression = "code_health".to_string();
        let origins = SelectionOrigins::uniform("test source");

        let errors = selection.validate_sourced(&origins).unwrap_err();

        assert!(errors
            .iter()
            .any(|e| e.field == "stewardship.docs_freshness.expression"));
    }

    #[test]
    fn field_origin_wins_over_table_origin() {
        let mut selection = valid_selection();
        selection.theory.belief_family_id = String::new();
        let mut origins = SelectionOrigins::uniform("table source");
        origins.fields.insert(
            "stewardship.docs_freshness.theory.belief_family_id".to_string(),
            "field source".to_string(),
        );

        let errors = selection.validate_sourced(&origins).unwrap_err();

        assert_eq!(errors[0].source, "field source");
    }

    #[test]
    fn canonical_declaration_accepts_expression_names_as_data() {
        let mut declaration = valid_declaration();
        declaration.expression = "repository_health".to_string();

        let result = declaration.validate_sourced(
            &SelectionOrigins::uniform("test source"),
            "stewardship.declarations.repository",
        );

        assert!(result.is_ok());
    }

    #[test]
    fn legacy_selection_lowers_into_the_canonical_shape() {
        let config = StewardshipConfig {
            declarations: BTreeMap::new(),
            docs_freshness: Some(valid_selection()),
        };

        let lowered = config.lowered_declarations().unwrap();

        assert_eq!(lowered.len(), 1);
        assert_eq!(lowered[0].declaration_id, "docs_freshness");
        assert_eq!(lowered[0].declaration, valid_declaration());
    }

    #[test]
    fn canonical_and_legacy_ids_cannot_collide() {
        let config = StewardshipConfig {
            declarations: BTreeMap::from([("docs_freshness".to_string(), valid_declaration())]),
            docs_freshness: Some(valid_selection()),
        };

        let errors = config
            .validate_sourced(&SelectionOrigins::uniform("test source"))
            .unwrap_err();

        assert!(errors
            .iter()
            .any(|error| error.message.contains("conflicts")));
    }
}
