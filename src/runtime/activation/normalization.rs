use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use meld_events::{validate_event_structural_identifier, EventStructuralIdentifierKind};
use meld_world_model::activation::validate_world_model_activation;
use meld_world_model::belief::BeliefConfigLoader;
use serde::Serialize;
use unicode_normalization::UnicodeNormalization;

use super::contracts::{
    ActivationHash, ActivationLoadError, DocsFreshnessActivationConfig,
    DocsFreshnessActivationDocument, DOCS_FRESHNESS_ACTIVATION_SCHEMA_VERSION,
};
use super::packages::{build_runtime_inputs, ProductActivationRuntimeInputs};

const HASH_DOMAIN: &[u8] = b"meld.product-activation.docs-freshness.v1";
const DOCS_FRESHNESS_KEY: &str = "docs_freshness";
const BOOTSTRAP_RUNTIME_ID: &str = "world_model.agent.bootstrap.docs_freshness";
const GRAPH_REPLAY_RUNTIME_ID: &str = "world_model.graph_replay";

pub(crate) struct NormalizedActivation {
    pub document: DocsFreshnessActivationDocument,
    pub resolved_target: PathBuf,
    pub activation_hash: ActivationHash,
    pub runtime_inputs: ProductActivationRuntimeInputs,
}

pub(crate) fn normalize_and_validate(
    document: DocsFreshnessActivationDocument,
    canonical_workspace_root: &Path,
) -> Result<NormalizedActivation, ActivationLoadError> {
    let mut document = normalize_document(document)?;
    let resolved_target = {
        let config = single_config_mut(&mut document)?;
        validate_config(config)?;
        resolve_target(canonical_workspace_root, &config.execution.target.value)?
    };
    let activation_hash = activation_hash(&document, canonical_workspace_root, &resolved_target)?;
    let canonical_target = path_text("execution.target", &resolved_target)?;
    let config = document
        .flywheel
        .get(DOCS_FRESHNESS_KEY)
        .ok_or_else(|| invalid("flywheel.docs_freshness", "is required"))?;
    let runtime_inputs = build_runtime_inputs(config, activation_hash.as_str(), canonical_target);
    validate_world_model_activation(&runtime_inputs.world_model)
        .map_err(|error| invalid(format!("world_model.{}", error.field), error.message))?;
    Ok(NormalizedActivation {
        document,
        resolved_target,
        activation_hash,
        runtime_inputs,
    })
}

fn normalize_document(
    document: DocsFreshnessActivationDocument,
) -> Result<DocsFreshnessActivationDocument, ActivationLoadError> {
    let target_values = document
        .flywheel
        .iter()
        .map(|(key, config)| (key.clone(), config.execution.target.value.clone()))
        .collect::<Vec<_>>();
    let mut value = serde_json::to_value(document)
        .map_err(|error| ActivationLoadError::Hash(error.to_string()))?;
    normalize_json_strings(&mut value);
    let mut normalized: DocsFreshnessActivationDocument = serde_json::from_value(value)
        .map_err(|error| ActivationLoadError::Hash(error.to_string()))?;
    // Filesystem names are byte-sensitive on supported Unix hosts. Preserve
    // the authored target until OS path resolution establishes its canonical
    // deployment coordinate.
    for (key, target_value) in target_values {
        if let Some(config) = normalized.flywheel.get_mut(&key) {
            config.execution.target.value = target_value;
        }
    }
    Ok(normalized)
}

fn normalize_json_strings(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) => *text = text.nfc().collect(),
        serde_json::Value::Array(values) => {
            for value in values {
                normalize_json_strings(value);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values_mut() {
                normalize_json_strings(value);
            }
        }
        _ => {}
    }
}

fn single_config_mut(
    document: &mut DocsFreshnessActivationDocument,
) -> Result<&mut DocsFreshnessActivationConfig, ActivationLoadError> {
    if document.schema_version != DOCS_FRESHNESS_ACTIVATION_SCHEMA_VERSION {
        return Err(invalid(
            "schema_version",
            format!("must equal {}", DOCS_FRESHNESS_ACTIVATION_SCHEMA_VERSION),
        ));
    }
    if document.flywheel.len() != 1 || !document.flywheel.contains_key(DOCS_FRESHNESS_KEY) {
        return Err(invalid(
            "flywheel",
            "must contain only the docs_freshness key",
        ));
    }
    document
        .flywheel
        .get_mut(DOCS_FRESHNESS_KEY)
        .ok_or_else(|| invalid("flywheel.docs_freshness", "is required"))
}

fn validate_config(config: &mut DocsFreshnessActivationConfig) -> Result<(), ActivationLoadError> {
    for (field, value) in identity_fields(config) {
        validate_identity(field, value)?;
    }
    for (field, value) in content_fields(config) {
        validate_content(field, value)?;
    }
    validate_event_type(
        "publication.success_event_type",
        &config.publication.success_event_type,
    )?;
    validate_event_type(
        "publication.failure_event_type",
        &config.publication.failure_event_type,
    )?;

    if config.activation_id != DOCS_FRESHNESS_KEY {
        return Err(invalid(
            "activation_id",
            "must equal docs_freshness for the first schema",
        ));
    }
    if config.belief_family.config_ref != config.belief_family.family_id {
        return Err(invalid(
            "belief_family.config_ref",
            "must equal belief_family.family_id",
        ));
    }
    if config.belief_family.config_version != "1" {
        return Err(invalid("belief_family.config_version", "must equal 1"));
    }
    if config.directive.directive_id != config.seed_agent.directive_id {
        return Err(invalid(
            "seed_agent.directive_id",
            "must equal directive.directive_id",
        ));
    }
    if config.seed_agent.observation_scope != config.belief_family.dimension_id {
        return Err(invalid(
            "seed_agent.observation_scope",
            "must equal belief_family.dimension_id",
        ));
    }
    if config.curation_rule.dimension_id != config.belief_family.dimension_id {
        return Err(invalid(
            "curation_rule.dimension_id",
            "must equal belief_family.dimension_id",
        ));
    }
    if config.curation_rule.threshold != config.belief_family.planner_projection.threshold {
        return Err(invalid(
            "curation_rule.threshold",
            "must equal belief_family.planner_projection.threshold",
        ));
    }
    if config.curation_rule.priority_urgency == 0 {
        return Err(invalid(
            "curation_rule.priority_urgency",
            "must be greater than zero",
        ));
    }
    if config.execution.workspace_scan_capability_version == 0 {
        return Err(invalid(
            "execution.workspace_scan_capability_version",
            "must be greater than zero",
        ));
    }
    if config.execution.target.kind != meld_execution::activation::ExecutionTargetKind::Path {
        return Err(invalid(
            "execution.target.kind",
            "must equal path for activation schema one",
        ));
    }
    if config.runtime.bootstrap_runtime_id != BOOTSTRAP_RUNTIME_ID {
        return Err(invalid(
            "runtime.bootstrap_runtime_id",
            format!("must equal {BOOTSTRAP_RUNTIME_ID}"),
        ));
    }

    validate_unique_ids(
        "belief_family.evidence_schemas.schema_id",
        config
            .belief_family
            .evidence_schemas
            .iter()
            .map(|value| value.schema_id.as_str()),
    )?;
    validate_unique_ids(
        "belief_family.source_mappings.mapping_id",
        config
            .belief_family
            .source_mappings
            .iter()
            .map(|value| value.mapping_id.as_str()),
    )?;
    validate_unique_ids(
        "belief_family.comparator.factors.factor_id",
        config
            .belief_family
            .comparator
            .factors
            .iter()
            .map(|value| value.factor_id.as_str()),
    )?;

    let mut runtime_ids = BTreeSet::new();
    for runtime_id in &config.runtime.enabled_runtime_ids {
        crate::runtime::supervisor::contracts::validate_runtime_id(runtime_id)
            .map_err(|error| invalid("runtime.enabled_runtime_ids", error.to_string()))?;
        if !runtime_ids.insert(runtime_id.clone()) {
            return Err(invalid(
                "runtime.enabled_runtime_ids",
                format!("contains duplicate id {runtime_id}"),
            ));
        }
        if runtime_id != BOOTSTRAP_RUNTIME_ID && runtime_id != GRAPH_REPLAY_RUNTIME_ID {
            return Err(invalid(
                "runtime.enabled_runtime_ids",
                format!("runtime {runtime_id} is not enabled during Wave 2"),
            ));
        }
    }
    if !runtime_ids.contains(BOOTSTRAP_RUNTIME_ID) || !runtime_ids.contains(GRAPH_REPLAY_RUNTIME_ID)
    {
        return Err(invalid(
            "runtime.enabled_runtime_ids",
            "must contain graph replay and the docs freshness bootstrap runtime",
        ));
    }
    config.runtime.enabled_runtime_ids = runtime_ids.into_iter().collect();

    let owner_config = config.belief_family.to_owner_config();
    BeliefConfigLoader::validate(&owner_config)
        .map_err(|error| invalid("belief_family", error.to_string()))?;
    validate_mapping_factors(config)?;
    Ok(())
}

fn identity_fields(config: &DocsFreshnessActivationConfig) -> Vec<(&'static str, &str)> {
    let mut fields = vec![
        ("activation_id", config.activation_id.as_str()),
        ("subject.domain_id", config.subject.domain_id.as_str()),
        ("subject.object_kind", config.subject.object_kind.as_str()),
        ("subject.object_id", config.subject.object_id.as_str()),
        (
            "branch_scope.branch_id",
            config.branch_scope.branch_id.as_str(),
        ),
        (
            "perspective.perspective_kind",
            config.perspective.perspective_kind.as_str(),
        ),
        (
            "perspective.perspective_id",
            config.perspective.perspective_id.as_str(),
        ),
        (
            "belief_family.config_ref",
            config.belief_family.config_ref.as_str(),
        ),
        (
            "belief_family.family_id",
            config.belief_family.family_id.as_str(),
        ),
        (
            "belief_family.dimension_id",
            config.belief_family.dimension_id.as_str(),
        ),
        (
            "belief_family.predicate_id",
            config.belief_family.predicate_id.as_str(),
        ),
        (
            "belief_family.evidence_policy_id",
            config.belief_family.evidence_policy_id.as_str(),
        ),
        (
            "belief_family.comparator.engine_id",
            config.belief_family.comparator.engine_id.as_str(),
        ),
        (
            "belief_family.comparator.engine_version",
            config.belief_family.comparator.engine_version.as_str(),
        ),
        (
            "belief_family.planner_projection.confidence_field",
            config
                .belief_family
                .planner_projection
                .confidence_field
                .as_str(),
        ),
        (
            "belief_family.planner_projection.posterior_meaning",
            config
                .belief_family
                .planner_projection
                .posterior_meaning
                .as_str(),
        ),
        (
            "belief_family.config_version",
            config.belief_family.config_version.as_str(),
        ),
        (
            "directive.directive_id",
            config.directive.directive_id.as_str(),
        ),
        ("seed_agent.agent_id", config.seed_agent.agent_id.as_str()),
        (
            "seed_agent.directive_id",
            config.seed_agent.directive_id.as_str(),
        ),
        (
            "seed_agent.observation_scope",
            config.seed_agent.observation_scope.as_str(),
        ),
        (
            "curation_rule.rule_id",
            config.curation_rule.rule_id.as_str(),
        ),
        (
            "curation_rule.dimension_id",
            config.curation_rule.dimension_id.as_str(),
        ),
        (
            "curation_rule.source_kind",
            config.curation_rule.source_kind.as_str(),
        ),
        ("execution.method_id", config.execution.method_id.as_str()),
        (
            "execution.workspace_scan_step_id",
            config.execution.workspace_scan_step_id.as_str(),
        ),
        (
            "execution.task_package_id",
            config.execution.task_package_id.as_str(),
        ),
        (
            "execution.workflow_id",
            config.execution.workflow_id.as_str(),
        ),
        (
            "execution.task_network_id",
            config.execution.task_network_id.as_str(),
        ),
        (
            "execution.required_artifact_type_id",
            config.execution.required_artifact_type_id.as_str(),
        ),
        (
            "execution.provider_binding_ref",
            config.execution.provider_binding_ref.as_str(),
        ),
        ("execution.frame_type", config.execution.frame_type.as_str()),
        (
            "execution.workspace_scan_capability_type_id",
            config.execution.workspace_scan_capability_type_id.as_str(),
        ),
        (
            "publication.content_source_kind",
            config.publication.content_source_kind.as_str(),
        ),
        (
            "runtime.bootstrap_runtime_id",
            config.runtime.bootstrap_runtime_id.as_str(),
        ),
    ];
    for schema in &config.belief_family.evidence_schemas {
        fields.push((
            "belief_family.evidence_schemas.schema_id",
            &schema.schema_id,
        ));
    }
    for mapping in &config.belief_family.source_mappings {
        fields.extend([
            (
                "belief_family.source_mappings.mapping_id",
                mapping.mapping_id.as_str(),
            ),
            (
                "belief_family.source_mappings.source_kind",
                mapping.source_kind.as_str(),
            ),
            (
                "belief_family.source_mappings.evidence_schema_id",
                mapping.evidence_schema_id.as_str(),
            ),
            (
                "belief_family.source_mappings.subject_from",
                mapping.subject_from.as_str(),
            ),
            (
                "belief_family.source_mappings.value_field",
                mapping.value_field.as_str(),
            ),
            (
                "belief_family.source_mappings.factor_id",
                mapping.factor_id.as_str(),
            ),
        ]);
    }
    for factor in &config.belief_family.comparator.factors {
        fields.extend([
            (
                "belief_family.comparator.factors.factor_id",
                factor.factor_id.as_str(),
            ),
            (
                "belief_family.comparator.factors.evidence_schema_id",
                factor.evidence_schema_id.as_str(),
            ),
        ]);
    }
    fields
}

fn content_fields(config: &DocsFreshnessActivationConfig) -> [(&'static str, &str); 4] {
    [
        ("directive.text", &config.directive.text),
        (
            "seed_agent.seed_provenance",
            &config.seed_agent.seed_provenance,
        ),
        (
            "curation_rule.desired_summary",
            &config.curation_rule.desired_summary,
        ),
        ("execution.target.value", &config.execution.target.value),
    ]
}

fn validate_identity(field: &str, value: &str) -> Result<(), ActivationLoadError> {
    let bytes = value.as_bytes();
    let valid = !bytes.is_empty()
        && bytes.len() <= 128
        && bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        });
    if valid {
        Ok(())
    } else {
        Err(invalid(
            field,
            "must be a lowercase structural identifier of at most 128 bytes",
        ))
    }
}

fn validate_content(field: &str, value: &str) -> Result<(), ActivationLoadError> {
    if value.is_empty()
        || value.trim() != value
        || value.chars().any(char::is_control)
        || value.len() > 4096
    {
        Err(invalid(
            field,
            "must be non-empty normalized text without surrounding whitespace or control characters",
        ))
    } else {
        Ok(())
    }
}

fn validate_event_type(field: &str, value: &str) -> Result<(), ActivationLoadError> {
    validate_event_structural_identifier(EventStructuralIdentifierKind::EventType, value)
        .map_err(|error| invalid(field, error.message))
}

fn validate_unique_ids<'a>(
    field: &str,
    values: impl Iterator<Item = &'a str>,
) -> Result<(), ActivationLoadError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(invalid(field, format!("contains duplicate id {value}")));
        }
    }
    Ok(())
}

fn validate_mapping_factors(
    config: &DocsFreshnessActivationConfig,
) -> Result<(), ActivationLoadError> {
    let factors = config
        .belief_family
        .comparator
        .factors
        .iter()
        .map(|factor| factor.factor_id.as_str())
        .collect::<BTreeSet<_>>();
    for mapping in &config.belief_family.source_mappings {
        if !factors.contains(mapping.factor_id.as_str()) {
            return Err(invalid(
                "belief_family.source_mappings.factor_id",
                format!("unknown comparator factor {}", mapping.factor_id),
            ));
        }
    }
    Ok(())
}

fn resolve_target(
    canonical_workspace_root: &Path,
    target: &str,
) -> Result<PathBuf, ActivationLoadError> {
    let target = Path::new(target);
    if target.is_absolute() {
        return Err(invalid(
            "execution.target.value",
            "must be a workspace-relative path",
        ));
    }
    let resolved = dunce::canonicalize(canonical_workspace_root.join(target)).map_err(|error| {
        invalid(
            "execution.target.value",
            format!("could not canonicalize target: {error}"),
        )
    })?;
    if !resolved.starts_with(canonical_workspace_root) {
        return Err(invalid(
            "execution.target.value",
            "must resolve within the canonical workspace root",
        ));
    }
    Ok(resolved)
}

#[derive(Serialize)]
struct HashProjection<'a> {
    schema: &'static str,
    canonical_workspace_root: &'a str,
    resolved_target: &'a str,
    document: &'a DocsFreshnessActivationDocument,
}

fn activation_hash(
    document: &DocsFreshnessActivationDocument,
    canonical_workspace_root: &Path,
    resolved_target: &Path,
) -> Result<ActivationHash, ActivationLoadError> {
    let workspace = path_text("workspace_root", canonical_workspace_root)?;
    let target = path_text("execution.target", resolved_target)?;
    let mut document = document.clone();
    document
        .flywheel
        .get_mut(DOCS_FRESHNESS_KEY)
        .ok_or_else(|| invalid("flywheel.docs_freshness", "is required"))?
        .execution
        .target
        .value = target.clone();
    let encoded = serde_json::to_vec(&HashProjection {
        schema: "docs_freshness_activation_v1",
        canonical_workspace_root: &workspace,
        resolved_target: &target,
        document: &document,
    })
    .map_err(|error| ActivationLoadError::Hash(error.to_string()))?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(HASH_DOMAIN);
    hasher.update(&[0]);
    hasher.update(&(encoded.len() as u64).to_be_bytes());
    hasher.update(&encoded);
    Ok(ActivationHash(hasher.finalize().to_hex().to_string()))
}

fn path_text(field: &str, path: &Path) -> Result<String, ActivationLoadError> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| invalid(field, "must be valid Unicode"))
}

fn invalid(field: impl Into<String>, message: impl Into<String>) -> ActivationLoadError {
    ActivationLoadError::Validation {
        field: field.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/runtime/docs_freshness_activation.toml"
    ));

    #[test]
    fn canonical_hash_projection_has_a_golden_digest() {
        let document: DocsFreshnessActivationDocument = toml::from_str(FIXTURE).unwrap();
        let document = normalize_document(document).unwrap();
        document
            .flywheel
            .get("docs_freshness")
            .expect("fixture must contain docs freshness");
        let mut document = document;
        validate_config(single_config_mut(&mut document).unwrap()).unwrap();

        let digest = activation_hash(
            &document,
            Path::new("/canonical/workspace"),
            Path::new("/canonical/workspace"),
        )
        .unwrap();

        assert_eq!(
            digest.as_str(),
            "98812f8050499efeb88b9c5856b348ad19f42546b9a5da7c0d9d975be67fb8ec"
        );
    }
}
