//! Minimal docs freshness stewardship selection schema and source-aware validation.
//!
//! Owner: root config. The selection names what the runtime stewards and
//! which theory it does so under — always by identity. Theory bodies
//! (evidence probability, satisfaction thresholds, task package meaning,
//! stale-signal interpretation) live in their owning domains and are
//! selected here by id per Runtime Initialization stage 2.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Origin label used when a field's config source cannot be determined.
const UNKNOWN_SOURCE: &str = "unknown configuration source";

/// Stewardship expression selections carried by the root config.
///
/// Absent selections are valid: a runtime with no stewardship selection
/// simply derives no stewardship bindings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StewardshipConfig {
    /// The minimal docs freshness selection, when the user selected one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs_freshness: Option<DocsFreshnessSelection>,
}

impl StewardshipConfig {
    /// True when no stewardship expression is selected.
    pub fn is_empty(&self) -> bool {
        self.docs_freshness.is_none()
    }
}

/// One minimal docs freshness stewardship selection.
///
/// Invariants: every field is an identity or a physical target, never a
/// theory body. `target_root` must be absolute so that resolution is
/// identical regardless of process working directory; the CLI passes its
/// path argument as the explicit default target per the recorded
/// requirements-gate decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Key into the root provider map binding the model provider.
    pub provider_id: String,

    /// Selected theory identities, resolved by their owning domains.
    pub theory: TheorySelection,
}

/// Theory selected by identity for one stewardship expression.
///
/// Identities only: the owning domains resolve each id to a content-hash
/// revision at installation time (Runtime Initialization stage 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TheorySelection {
    /// Belief family identity in the world model registry.
    pub belief_family_id: String,

    /// Outcome-to-evidence mapping identity in the world model.
    pub evidence_mapping_id: String,

    /// Curation rule reference bound at agent registration.
    pub curation_rule_id: String,
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
    /// Origin of the selection table itself; attributed to missing fields.
    table_origin: Option<String>,
    /// Origin per field path relative to the selection table.
    fields: HashMap<String, String>,
}

impl SelectionOrigins {
    /// Origins where every field is attributed to one named source.
    pub fn uniform(source: &str) -> Self {
        Self {
            table_origin: Some(source.to_string()),
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
        let Some(docs) = stewardship.get("docs_freshness") else {
            return origins;
        };
        origins.table_origin = docs.origin().map(str::to_string);
        if let config::ValueKind::Table(docs) = &docs.kind {
            for (key, value) in docs {
                if let config::ValueKind::Table(nested) = &value.kind {
                    for (nested_key, nested_value) in nested {
                        if let Some(origin) = nested_value.origin() {
                            origins
                                .fields
                                .insert(format!("{key}.{nested_key}"), origin.to_string());
                        }
                    }
                } else if let Some(origin) = value.origin() {
                    origins.fields.insert(key.clone(), origin.to_string());
                }
            }
        }
        origins
    }

    fn source_for(&self, field: &str) -> String {
        self.fields
            .get(field)
            .or(self.table_origin.as_ref())
            .cloned()
            .unwrap_or_else(|| UNKNOWN_SOURCE.to_string())
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
        let mut errors = Vec::new();
        let mut reject = |field: &str, message: String| {
            errors.push(SelectionFieldError {
                source: origins.source_for(field),
                field: format!("stewardship.docs_freshness.{field}"),
                message,
            });
        };

        if self.expression != "docs_freshness" {
            reject(
                "expression",
                format!(
                    "must be 'docs_freshness', got '{}'; other stewardship expressions are not selectable here",
                    self.expression
                ),
            );
        }
        if self.target_root.as_os_str().is_empty() {
            reject("target_root", "must not be empty".to_string());
        } else if !self.target_root.is_absolute() {
            // Relative targets would resolve against the process working
            // directory, breaking binding determinism across invocations.
            reject(
                "target_root",
                format!(
                    "must be an absolute path, got '{}'",
                    self.target_root.display()
                ),
            );
        }
        for (field, value) in [
            ("subject", &self.subject),
            ("agent_id", &self.agent_id),
            ("provider_id", &self.provider_id),
            ("theory.belief_family_id", &self.theory.belief_family_id),
            (
                "theory.evidence_mapping_id",
                &self.theory.evidence_mapping_id,
            ),
            ("theory.curation_rule_id", &self.theory.curation_rule_id),
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
            provider_id: "main-provider".to_string(),
            theory: TheorySelection {
                belief_family_id: "docs-freshness-family".to_string(),
                evidence_mapping_id: "publication-to-freshness".to_string(),
                curation_rule_id: "docs-curation".to_string(),
            },
        }
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
            "theory.belief_family_id".to_string(),
            "field source".to_string(),
        );

        let errors = selection.validate_sourced(&origins).unwrap_err();

        assert_eq!(errors[0].source, "field source");
    }
}
