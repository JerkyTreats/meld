use std::collections::BTreeMap;
use std::path::PathBuf;

use meld_execution::activation::{ExecutionForcePolicy, ExecutionTargetKind};
use meld_world_model::belief::{
    ComparatorConfig, EvidenceSchemaConfig, EvidenceSourceMapping, PlannerProjectionConfig,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::packages::ProductActivationRuntimeInputs;

/// Supported product activation schema version.
pub const DOCS_FRESHNESS_ACTIVATION_SCHEMA_VERSION: u32 = 1;
/// Maximum number of source bytes accepted by the activation loader.
pub const MAX_ACTIVATION_SOURCE_BYTES: usize = 1024 * 1024;

/// Strict source document for the first product flywheel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsFreshnessActivationDocument {
    /// Version of the source schema.
    pub schema_version: u32,
    /// Named flywheel activation entries.
    pub flywheel: BTreeMap<String, DocsFreshnessActivationConfig>,
}

/// Strict source configuration for one docs freshness activation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsFreshnessActivationConfig {
    /// Stable activation identity.
    pub activation_id: String,
    /// Subject assessed by the flywheel.
    pub subject: DocsFreshnessSubjectConfig,
    /// Branch-local belief scope.
    pub branch_scope: DocsFreshnessBranchScopeConfig,
    /// Perspective used by belief and agent reads.
    pub perspective: DocsFreshnessPerspectiveConfig,
    /// Inline belief family configuration.
    pub belief_family: DocsFreshnessBeliefFamilyConfig,
    /// Trusted directive configuration.
    pub directive: DocsFreshnessDirectiveConfig,
    /// Seed agent configuration.
    pub seed_agent: DocsFreshnessSeedAgentConfig,
    /// Threshold curation rule configuration.
    pub curation_rule: DocsFreshnessCurationRuleConfig,
    /// Execution binding configuration.
    pub execution: DocsFreshnessExecutionConfig,
    /// Publication mapping configuration.
    pub publication: DocsFreshnessPublicationConfig,
    /// Runtime selection configuration.
    pub runtime: DocsFreshnessRuntimeConfig,
}

/// Strict subject coordinates from the source document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsFreshnessSubjectConfig {
    /// Owning domain id.
    pub domain_id: String,
    /// Object kind within the domain.
    pub object_kind: String,
    /// Domain-local object id.
    pub object_id: String,
}

/// Strict branch scope from the source document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsFreshnessBranchScopeConfig {
    /// Stable branch id.
    pub branch_id: String,
}

/// Strict perspective key from the source document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsFreshnessPerspectiveConfig {
    /// Perspective family id.
    pub perspective_kind: String,
    /// Perspective member id.
    pub perspective_id: String,
}

/// Strict inline belief family configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsFreshnessBeliefFamilyConfig {
    /// Stable reference used by activation consumers.
    pub config_ref: String,
    /// Belief family id.
    pub family_id: String,
    /// Assessed dimension id.
    pub dimension_id: String,
    /// Predicate id.
    pub predicate_id: String,
    /// Evidence policy id.
    pub evidence_policy_id: String,
    /// Configured evidence schemas.
    pub evidence_schemas: Vec<StrictEvidenceSchemaConfig>,
    /// Mappings from promoted sources into evidence.
    pub source_mappings: Vec<StrictEvidenceSourceMapping>,
    /// Comparator configuration.
    pub comparator: StrictComparatorConfig,
    /// Prior probability used before evidence.
    pub default_prior: f64,
    /// Planner-facing projection configuration.
    pub planner_projection: StrictPlannerProjectionConfig,
    /// Owner config schema version.
    pub config_version: String,
}

/// Strict evidence schema entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrictEvidenceSchemaConfig {
    /// Stable schema id.
    pub schema_id: String,
    /// Whether settlement requires this schema.
    pub required: bool,
    /// Evidence role.
    pub role: meld_world_model::belief::EvidenceRole,
    /// Source reliability.
    pub reliability: f64,
    /// Source precision.
    pub precision: f64,
}

/// Strict promoted source mapping entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrictEvidenceSourceMapping {
    /// Stable mapping id.
    pub mapping_id: String,
    /// Promoted source kind.
    pub source_kind: String,
    /// Target evidence schema id.
    pub evidence_schema_id: String,
    /// Subject selector expression.
    pub subject_from: String,
    /// Value field selector.
    pub value_field: String,
    /// Comparator factor id.
    pub factor_id: String,
}

/// Strict comparator configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrictComparatorConfig {
    /// Comparator engine id.
    pub engine_id: String,
    /// Comparator engine version.
    pub engine_version: String,
    /// Ordered comparator factors.
    pub factors: Vec<StrictComparatorFactorConfig>,
    /// Uncertainty used when evidence is missing.
    pub missing_evidence_uncertainty: f64,
}

/// Strict comparator factor entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrictComparatorFactorConfig {
    /// Stable factor id.
    pub factor_id: String,
    /// Evidence schema consumed by the factor.
    pub evidence_schema_id: String,
    /// Relative factor weight.
    pub weight: f64,
    /// Contribution direction.
    pub polarity: meld_world_model::belief::EvidencePolarity,
}

/// Strict planner projection configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrictPlannerProjectionConfig {
    /// Confidence field name.
    pub confidence_field: String,
    /// Goal curation threshold.
    pub threshold: f64,
    /// Meaning assigned to the posterior.
    pub posterior_meaning: String,
}

/// Trusted directive source configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsFreshnessDirectiveConfig {
    /// Stable directive id.
    pub directive_id: String,
    /// Directive text owned by the world model.
    pub text: String,
}

/// Seed agent source configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsFreshnessSeedAgentConfig {
    /// Stable agent id.
    pub agent_id: String,
    /// Referenced directive id.
    pub directive_id: String,
    /// Named observation scope.
    pub observation_scope: String,
    /// Trusted seed provenance.
    pub seed_provenance: String,
}

/// Threshold curation rule source configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsFreshnessCurationRuleConfig {
    /// Stable rule id.
    pub rule_id: String,
    /// Belief dimension assessed by the rule.
    pub dimension_id: String,
    /// Confidence threshold below which a goal is proposed.
    pub threshold: f64,
    /// Goal priority urgency.
    pub priority_urgency: u32,
    /// Desired state summary.
    pub desired_summary: String,
    /// Goal source family.
    pub source_kind: String,
}

/// Execution source configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsFreshnessExecutionConfig {
    /// Bound method id.
    pub method_id: String,
    /// Exact method step that must resolve workspace scan.
    pub workspace_scan_step_id: String,
    /// Bound task package id.
    pub task_package_id: String,
    /// Bound workflow id.
    pub workflow_id: String,
    /// Configured task network id.
    pub task_network_id: String,
    /// Required output artifact type.
    pub required_artifact_type_id: String,
    /// Repository provider binding reference.
    pub provider_binding_ref: String,
    /// Frame type supplied to the task package.
    pub frame_type: String,
    /// Existing output handling policy.
    pub force_policy: ExecutionForcePolicy,
    /// Workspace target selector.
    pub target: ActivationTargetSelector,
    /// Workspace scan capability id.
    pub workspace_scan_capability_type_id: String,
    /// Workspace scan capability version.
    pub workspace_scan_capability_version: u32,
}

/// Strict execution target selector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationTargetSelector {
    /// Target selector kind.
    pub kind: ExecutionTargetKind,
    /// Workspace-relative target value.
    pub value: String,
}

/// Publication mapping source configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsFreshnessPublicationConfig {
    /// Canonical success event type.
    pub success_event_type: String,
    /// Canonical failure event type.
    pub failure_event_type: String,
    /// Promoted content source kind.
    pub content_source_kind: String,
}

/// Runtime selection source configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocsFreshnessRuntimeConfig {
    /// Enabled runtime ids for this activation.
    pub enabled_runtime_ids: Vec<String>,
    /// One-shot bootstrap runtime id.
    pub bootstrap_runtime_id: String,
}

/// Canonical digest of normalized content and resolved deployment coordinates.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActivationHash(pub String);

impl ActivationHash {
    /// Borrow the lowercase BLAKE3 digest.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Loader-owned source metadata used only for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationSource {
    /// Canonical source file path.
    pub canonical_path: PathBuf,
    /// Number of bytes read from the source.
    pub byte_count: usize,
}

/// Machine-readable passive activation diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationDiagnostic {
    /// Stable diagnostic code.
    pub code: String,
    /// Field or boundary associated with the diagnostic.
    pub field: String,
    /// Human-readable detail.
    pub message: String,
}

/// Ordered diagnostics from passive activation validation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationDiagnostics {
    /// Collected diagnostics.
    pub entries: Vec<ActivationDiagnostic>,
}

/// Validated activation and typed owner packages.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ValidatedDocsFreshnessActivation {
    /// Loader metadata excluded from activation identity and owner packages.
    pub source: ActivationSource,
    /// Canonical product workspace root.
    pub canonical_workspace_root: PathBuf,
    /// Canonical target selected within the workspace.
    pub resolved_target: PathBuf,
    /// Normalized typed source content.
    pub document: DocsFreshnessActivationDocument,
    /// Canonical content and deployment identity.
    pub activation_hash: ActivationHash,
    /// Independent owner-scoped runtime packages.
    pub runtime_inputs: ProductActivationRuntimeInputs,
}

impl ValidatedDocsFreshnessActivation {
    /// Create a source-neutral passive description for CLI presentation.
    pub fn passive_description(&self) -> PassiveActivationDescription {
        PassiveActivationDescription {
            activation_id: self.runtime_inputs.runtime.activation_id.clone(),
            activation_hash: self.activation_hash.clone(),
            canonical_workspace_root: self.canonical_workspace_root.clone(),
            resolved_target: self.resolved_target.clone(),
            source_path: self.source.canonical_path.clone(),
            source_bytes: self.source.byte_count,
            enabled_runtime_ids: self.runtime_inputs.runtime.enabled_runtime_ids.clone(),
            validation_scope: "source_and_owner_packages".to_string(),
            application_ready: false,
            diagnostics: ActivationDiagnostics::default(),
        }
    }
}

/// Serializable result for early passive CLI routing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassiveActivationDescription {
    /// Stable activation id.
    pub activation_id: String,
    /// Canonical activation hash.
    pub activation_hash: ActivationHash,
    /// Canonical workspace root.
    pub canonical_workspace_root: PathBuf,
    /// Canonical execution target.
    pub resolved_target: PathBuf,
    /// Canonical activation source path.
    pub source_path: PathBuf,
    /// Number of activation source bytes read.
    pub source_bytes: usize,
    /// Canonically sorted enabled runtime ids.
    pub enabled_runtime_ids: Vec<String>,
    /// Stable description of the passive validation boundary completed.
    pub validation_scope: String,
    /// Whether all domain asset binding and apply gates have completed.
    pub application_ready: bool,
    /// Passive validation diagnostics.
    pub diagnostics: ActivationDiagnostics,
}

/// Fail-closed activation loading error.
#[derive(Debug, Error)]
pub enum ActivationLoadError {
    /// Workspace root could not be canonicalized.
    #[error("activation workspace root is invalid: {0}")]
    Workspace(String),
    /// Source path or opened file violated the source policy.
    #[error("activation source is invalid: {0}")]
    Source(String),
    /// Source exceeded the byte cap.
    #[error("activation source exceeds the one MiB byte limit")]
    SourceTooLarge,
    /// TOML could not be parsed into the strict source DTO.
    #[error("activation TOML is invalid: {0}")]
    Parse(String),
    /// Typed activation content violated the frozen contract.
    #[error("activation validation failed at {field}: {message}")]
    Validation {
        /// Field path associated with the failure.
        field: String,
        /// Human-readable validation detail.
        message: String,
    },
    /// Canonical activation hashing failed.
    #[error("activation hash projection failed: {0}")]
    Hash(String),
}

impl DocsFreshnessBeliefFamilyConfig {
    pub(crate) fn to_owner_config(&self) -> meld_world_model::belief::BeliefFamilyConfig {
        meld_world_model::belief::BeliefFamilyConfig {
            family_id: self.family_id.clone(),
            dimension_id: self.dimension_id.clone(),
            predicate_id: self.predicate_id.clone(),
            evidence_policy_id: self.evidence_policy_id.clone(),
            evidence_schemas: self
                .evidence_schemas
                .iter()
                .cloned()
                .map(EvidenceSchemaConfig::from)
                .collect(),
            source_mappings: self
                .source_mappings
                .iter()
                .cloned()
                .map(EvidenceSourceMapping::from)
                .collect(),
            comparator: ComparatorConfig {
                engine_id: self.comparator.engine_id.clone(),
                engine_version: self.comparator.engine_version.clone(),
                factors: self
                    .comparator
                    .factors
                    .iter()
                    .cloned()
                    .map(meld_world_model::belief::ComparatorFactorConfig::from)
                    .collect(),
                missing_evidence_uncertainty: self.comparator.missing_evidence_uncertainty,
            },
            default_prior: self.default_prior,
            planner_projection: PlannerProjectionConfig {
                confidence_field: self.planner_projection.confidence_field.clone(),
                threshold: self.planner_projection.threshold,
                posterior_meaning: self.planner_projection.posterior_meaning.clone(),
            },
            config_version: self.config_version.clone(),
        }
    }
}

impl From<StrictEvidenceSchemaConfig> for EvidenceSchemaConfig {
    fn from(value: StrictEvidenceSchemaConfig) -> Self {
        Self {
            schema_id: value.schema_id,
            required: value.required,
            role: value.role,
            reliability: value.reliability,
            precision: value.precision,
        }
    }
}

impl From<StrictEvidenceSourceMapping> for EvidenceSourceMapping {
    fn from(value: StrictEvidenceSourceMapping) -> Self {
        Self {
            mapping_id: value.mapping_id,
            source_kind: value.source_kind,
            evidence_schema_id: value.evidence_schema_id,
            subject_from: value.subject_from,
            value_field: value.value_field,
            factor_id: value.factor_id,
        }
    }
}

impl From<StrictComparatorFactorConfig> for meld_world_model::belief::ComparatorFactorConfig {
    fn from(value: StrictComparatorFactorConfig) -> Self {
        Self {
            factor_id: value.factor_id,
            evidence_schema_id: value.evidence_schema_id,
            weight: value.weight,
            polarity: value.polarity,
        }
    }
}
