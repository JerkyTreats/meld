//! Pure validation and identity derivation for world-model activation.

use std::error::Error;
use std::fmt;

use serde::Serialize;

use super::{WorldModelActivationIdentity, WorldModelActivationInput};
use crate::belief::BeliefConfigLoader;

/// Structured failure returned before world-model bootstrap opens or writes stores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldModelActivationValidationError {
    /// Stable field path associated with the failure.
    pub field: String,
    /// Human-readable validation detail.
    pub message: String,
}

impl fmt::Display for WorldModelActivationValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.message)
    }
}

impl Error for WorldModelActivationValidationError {}

/// Validate one typed owner package and derive its deterministic identity.
pub fn validate_world_model_activation(
    input: &WorldModelActivationInput,
) -> Result<WorldModelActivationIdentity, WorldModelActivationValidationError> {
    require_blake3_hash("activation_hash", &input.activation_hash)?;
    require_text("activation_id", &input.activation_id)?;
    require_text("bootstrap_id", &input.bootstrap_id)?;
    require_text("directive.directive_id", &input.directive.directive_id)?;
    require_text("directive.text", &input.directive.text)?;
    require_text("seed_agent.agent_id", &input.seed_agent.agent_id)?;
    require_text(
        "seed_agent.observation_scope",
        &input.seed_agent.observation_scope,
    )?;
    require_text(
        "seed_agent.seed_provenance",
        &input.seed_agent.seed_provenance,
    )?;
    input
        .seed_agent
        .perspective_key
        .validate()
        .map_err(|error| invalid("seed_agent.perspective_key", error.to_string()))?;
    input
        .seed_agent
        .subject
        .validate()
        .map_err(|error| invalid("seed_agent.subject", error.to_string()))?;
    require_text(
        "seed_agent.branch_scope.branch_id",
        &input.seed_agent.branch_scope.branch_id,
    )?;
    input
        .belief_key
        .validate()
        .map_err(|error| invalid("belief_key", error.to_string()))?;
    input
        .curation_rule
        .config
        .validate()
        .map_err(|error| invalid("curation_rule.config", error.to_string()))?;
    require_text("curation_rule.rule_id", &input.curation_rule.rule_id)?;
    require_text("curation_rule.agent_id", &input.curation_rule.agent_id)?;

    if input.seed_agent.directive_id != input.directive.directive_id {
        return Err(invalid(
            "seed_agent.directive_id",
            "must reference directive.directive_id",
        ));
    }
    if input.curation_rule.agent_id != input.seed_agent.agent_id {
        return Err(invalid(
            "curation_rule.agent_id",
            "must reference seed_agent.agent_id",
        ));
    }
    if input.belief_key.subject != input.seed_agent.subject {
        return Err(invalid(
            "belief_key.subject",
            "must equal seed_agent.subject",
        ));
    }
    if input.belief_key.perspective != input.seed_agent.perspective_key {
        return Err(invalid(
            "belief_key.perspective",
            "must equal seed_agent.perspective_key",
        ));
    }
    if input.belief_key.branch_scope != input.seed_agent.branch_scope {
        return Err(invalid(
            "belief_key.branch_scope",
            "must equal seed_agent.branch_scope",
        ));
    }
    if input.belief_key.dimension_id != input.belief_family.dimension_id {
        return Err(invalid(
            "belief_key.dimension_id",
            "must equal belief_family.dimension_id",
        ));
    }
    if input.belief_key.predicate_id != input.belief_family.predicate_id {
        return Err(invalid(
            "belief_key.predicate_id",
            "must equal belief_family.predicate_id",
        ));
    }
    if input.belief_key.evidence_policy_id != input.belief_family.evidence_policy_id {
        return Err(invalid(
            "belief_key.evidence_policy_id",
            "must equal belief_family.evidence_policy_id",
        ));
    }
    if input.curation_rule.config.dimension_id != input.belief_family.dimension_id {
        return Err(invalid(
            "curation_rule.config.dimension_id",
            "must equal belief_family.dimension_id",
        ));
    }
    if input.curation_rule.config.threshold != input.belief_family.planner_projection.threshold {
        return Err(invalid(
            "curation_rule.config.threshold",
            "must equal belief_family.planner_projection.threshold",
        ));
    }

    let snapshot = BeliefConfigLoader::snapshot(input.belief_family.clone())
        .map_err(|error| invalid("belief_family", error.to_string()))?;
    Ok(WorldModelActivationIdentity {
        activation_hash: input.activation_hash.clone(),
        activation_id: input.activation_id.clone(),
        bootstrap_id: input.bootstrap_id.clone(),
        input_hash: semantic_digest(input)?,
        belief_config_hash: snapshot.hash,
    })
}

fn semantic_digest(value: &impl Serialize) -> Result<String, WorldModelActivationValidationError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| invalid("activation_input", error.to_string()))?;
    Ok(blake3::hash(&encoded).to_hex().to_string())
}

fn require_blake3_hash(
    field: &str,
    value: &str,
) -> Result<(), WorldModelActivationValidationError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(invalid(
            field,
            "must be a lowercase BLAKE3 hexadecimal digest",
        ))
    }
}

fn require_text(field: &str, value: &str) -> Result<(), WorldModelActivationValidationError> {
    if value.trim().is_empty() {
        Err(invalid(field, "must be non-empty"))
    } else {
        Ok(())
    }
}

fn invalid(
    field: impl Into<String>,
    message: impl Into<String>,
) -> WorldModelActivationValidationError {
    WorldModelActivationValidationError {
        field: field.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::activation::{AgentCurationRuleRecord, DirectiveRecord, SeedAgentActivation};
    use crate::agent::AgentCurationRuleConfig;
    use crate::belief::{BeliefFamilyConfig, BeliefKey, BranchScope};
    use crate::events::DomainObjectRef;
    use crate::world_state::graph::PerspectiveKey;

    fn input() -> WorldModelActivationInput {
        let belief_family: BeliefFamilyConfig = serde_json::from_str(
            r#"{
                "family_id":"docs_freshness",
                "dimension_id":"docs_freshness",
                "predicate_id":"confidence",
                "evidence_policy_id":"default_policy",
                "evidence_schemas":[{"schema_id":"content","required":true,"role":"Support","reliability":1.0,"precision":1.0}],
                "source_mappings":[{"mapping_id":"content","source_kind":"content_written","evidence_schema_id":"content","subject_from":"record.subject","value_field":"stale_probability","factor_id":"freshness"}],
                "comparator":{"engine_id":"weighted_bayesian","engine_version":"1","factors":[{"factor_id":"freshness","evidence_schema_id":"content","weight":1.0,"polarity":"Supports"}],"missing_evidence_uncertainty":0.5},
                "default_prior":0.8,
                "planner_projection":{"confidence_field":"confidence","threshold":0.7,"posterior_meaning":"stale_probability"},
                "config_version":"1"
            }"#,
        )
        .unwrap();
        let subject = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
        let perspective = PerspectiveKey::new("default", "default").unwrap();
        let branch_scope = BranchScope::main();
        WorldModelActivationInput {
            activation_hash: "a".repeat(64),
            activation_id: "activation.docs_freshness".to_string(),
            bootstrap_id: "bootstrap.docs_freshness".to_string(),
            directive: DirectiveRecord {
                directive_id: "directive.docs_freshness".to_string(),
                text: "curate docs freshness goals".to_string(),
            },
            seed_agent: SeedAgentActivation {
                agent_id: "seed.docs_freshness".to_string(),
                perspective_key: perspective.clone(),
                subject: subject.clone(),
                branch_scope: branch_scope.clone(),
                observation_scope: "docs_freshness".to_string(),
                directive_id: "directive.docs_freshness".to_string(),
                seed_provenance: "trusted init".to_string(),
            },
            curation_rule: AgentCurationRuleRecord {
                rule_id: "rule.docs_freshness".to_string(),
                agent_id: "seed.docs_freshness".to_string(),
                config: AgentCurationRuleConfig {
                    dimension_id: "docs_freshness".to_string(),
                    threshold: 0.7,
                    priority_urgency: 50,
                    desired_summary: "confidence above threshold".to_string(),
                    source_kind: "belief_divergence".to_string(),
                },
            },
            belief_key: BeliefKey {
                subject,
                dimension_id: "docs_freshness".to_string(),
                predicate_id: "confidence".to_string(),
                perspective,
                branch_scope,
                evidence_policy_id: "default_policy".to_string(),
            },
            belief_family,
        }
    }

    #[test]
    fn validation_is_deterministic_and_source_format_neutral() {
        let first = input();
        let encoded = serde_json::to_string_pretty(&first).unwrap();
        let second: WorldModelActivationInput = serde_json::from_str(&encoded).unwrap();

        assert_eq!(
            validate_world_model_activation(&first).unwrap(),
            validate_world_model_activation(&second).unwrap()
        );
        assert_eq!(
            validate_world_model_activation(&first)
                .unwrap()
                .activation_id,
            "activation.docs_freshness"
        );
    }

    #[test]
    fn seed_activation_leaves_persistence_ordering_to_bootstrap() {
        let encoded = serde_json::to_value(input().seed_agent).unwrap();

        assert!(!encoded.as_object().unwrap().contains_key("created_at_seq"));
    }

    #[test]
    fn validation_rejects_cross_owner_identity_drift() {
        let mut value = input();
        value.seed_agent.directive_id = "directive.other".to_string();

        let error = validate_world_model_activation(&value).unwrap_err();
        assert_eq!(error.field, "seed_agent.directive_id");
    }

    #[test]
    fn validation_rejects_threshold_drift() {
        let mut value = input();
        value.curation_rule.config.threshold = 0.8;

        let error = validate_world_model_activation(&value).unwrap_err();
        assert_eq!(error.field, "curation_rule.config.threshold");
    }

    #[test]
    fn contracts_reject_unknown_fields() {
        let mut value = serde_json::to_value(input()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("source_format".to_string(), serde_json::json!("toml"));

        assert!(serde_json::from_value::<WorldModelActivationInput>(value).is_err());
    }
}
