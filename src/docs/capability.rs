//! Atomic documentation capability publication and invocation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use walkdir::{DirEntry, WalkDir};

use crate::capability::{
    ArtifactSchemaVersionRange, CapabilityInvocationPayload, CapabilityInvocationResult,
    CapabilityInvoker, CapabilityRuntimeInit, CapabilityTypeContract, EffectKind, EffectSpec,
    ExecutionClass, ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec,
    ScopeContract, SuppliedValueRef,
};
use crate::context::generation::contracts::GenerationOrchestrationRequest;
use crate::docs::claim_validation::{
    validate_patch_set, verify_validated_patch_set, DocsClaimPolicy, ValidatedDocsPatchSet,
};
use crate::error::ApiError;
use crate::execution::{ExecutionEventContext, ExecutionRuntimeContext};
use crate::provider::executor::{execute_completion, prepare_provider_for_request};
use crate::provider::{ChatMessage, MessageRole, ProviderExecutionBinding};
use crate::task::{ArtifactProducerRef, ArtifactRecord};
use meld_execution::error::TERMINAL_CAPABILITY_FAILURE_MARKER;

pub const INSPECT_SCOPE: &str = "docs.inspect_scope";
pub const DRAFT_PATCH_SET: &str = "docs.draft_patch_set";
pub const VALIDATE_PATCH_SET: &str = "docs.validate_patch_set";
pub const PUBLISH_PATCH_SET: &str = "docs.publish_patch_set";
pub const ASSESS_PUBLISHED_SCOPE: &str = "docs.assess_published_scope";

pub const EVIDENCE_BUNDLE: &str = "docs_evidence_bundle";
pub const PATCH_SET: &str = "docs_patch_set";
pub const VALIDATED_PATCH_SET: &str = "docs_validated_patch_set";
pub const PUBLICATION_RECEIPT: &str = "docs_publication_receipt";
pub const FRESHNESS_ASSESSMENT: &str = "docs_freshness_assessment";

const VERSION: u32 = 1;
const MAX_DIRECTORIES: usize = 64;
const MAX_FILE_BYTES: usize = 8 * 1024;
const MAX_DIRECTORY_EVIDENCE_BYTES: usize = 24 * 1024;
const MAX_CHILD_README_BYTES: usize = 3 * 1024;

#[derive(Debug, Clone)]
pub struct DocsCapabilityConfig {
    pub target_root: PathBuf,
    pub subject_id: String,
    pub agent_id: String,
    pub provider: ProviderExecutionBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirectoryEvidence {
    pub path: String,
    pub direct_files: Vec<String>,
    pub child_directories: Vec<String>,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsEvidenceBundle {
    pub source_fingerprint: String,
    pub directories: Vec<DirectoryEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadmePatch {
    pub path: String,
    pub content: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocsPatchSet {
    pub source_fingerprint: String,
    pub patches: Vec<ReadmePatch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishedReadme {
    pub path: String,
    pub content_hash: String,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocsPublicationReceipt {
    pub source_fingerprint: String,
    pub policy_identity: String,
    pub validation_fingerprint: String,
    pub weighted_groundedness: f64,
    pub unsupported_claim_mass: f64,
    pub contradiction_claim_mass: f64,
    pub published: Vec<PublishedReadme>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocsFreshnessAssessment {
    pub subject_id: String,
    pub source_fingerprint: String,
    pub stale_probability: f64,
    pub expected_readmes: usize,
    pub verified_readmes: usize,
    pub policy_identity: String,
    pub validation_fingerprint: String,
    pub weighted_groundedness: f64,
    pub unsupported_claim_mass: f64,
    pub contradiction_claim_mass: f64,
}

#[derive(Debug, Clone)]
pub struct InspectScopeCapability {
    config: DocsCapabilityConfig,
}

#[derive(Debug, Clone)]
pub struct DraftPatchSetCapability {
    config: DocsCapabilityConfig,
}

#[derive(Debug, Clone)]
pub struct ValidatePatchSetCapability {
    config: DocsCapabilityConfig,
    policy: DocsClaimPolicy,
}

#[derive(Debug, Clone)]
pub struct PublishPatchSetCapability {
    config: DocsCapabilityConfig,
    policy: DocsClaimPolicy,
}

#[derive(Debug, Clone)]
pub struct AssessPublishedScopeCapability {
    config: DocsCapabilityConfig,
}

impl InspectScopeCapability {
    pub fn new(config: DocsCapabilityConfig) -> Self {
        Self { config }
    }
}

impl DraftPatchSetCapability {
    pub fn new(config: DocsCapabilityConfig) -> Self {
        Self { config }
    }
}

impl ValidatePatchSetCapability {
    pub fn new(config: DocsCapabilityConfig, policy: DocsClaimPolicy) -> Self {
        Self { config, policy }
    }
}

impl PublishPatchSetCapability {
    pub fn new(config: DocsCapabilityConfig, policy: DocsClaimPolicy) -> Self {
        Self { config, policy }
    }
}

impl AssessPublishedScopeCapability {
    pub fn new(config: DocsCapabilityConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl CapabilityInvoker for InspectScopeCapability {
    type Error = ApiError;
    type ExecutionApi = dyn ExecutionRuntimeContext;

    fn contract(&self) -> CapabilityTypeContract {
        contract(INSPECT_SCOPE, &[], EVIDENCE_BUNDLE, EffectKind::Emit)
    }

    async fn invoke(
        &self,
        _api: &dyn ExecutionRuntimeContext,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        _event_context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;
        let bundle = inspect_scope(&self.config.target_root).map_err(terminalize_docs_error)?;
        Ok(single_artifact(
            payload,
            runtime_init,
            EVIDENCE_BUNDLE,
            to_value(bundle)?,
        ))
    }
}

#[async_trait]
impl CapabilityInvoker for DraftPatchSetCapability {
    type Error = ApiError;
    type ExecutionApi = dyn ExecutionRuntimeContext;

    fn contract(&self) -> CapabilityTypeContract {
        contract(
            DRAFT_PATCH_SET,
            &[EVIDENCE_BUNDLE],
            PATCH_SET,
            EffectKind::Emit,
        )
    }

    async fn invoke(
        &self,
        api: &dyn ExecutionRuntimeContext,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;
        let bundle: DocsEvidenceBundle = decode_input(payload, EVIDENCE_BUNDLE)?;
        let patches = draft_patch_set(api, &self.config, &bundle, event_context)
            .await
            .map_err(terminalize_docs_error)?;
        Ok(single_artifact(
            payload,
            runtime_init,
            PATCH_SET,
            to_value(patches)?,
        ))
    }
}

#[async_trait]
impl CapabilityInvoker for ValidatePatchSetCapability {
    type Error = ApiError;
    type ExecutionApi = dyn ExecutionRuntimeContext;

    fn contract(&self) -> CapabilityTypeContract {
        contract(
            VALIDATE_PATCH_SET,
            &[EVIDENCE_BUNDLE, PATCH_SET],
            VALIDATED_PATCH_SET,
            EffectKind::Emit,
        )
    }

    async fn invoke(
        &self,
        api: &dyn ExecutionRuntimeContext,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;
        let bundle: DocsEvidenceBundle = decode_input(payload, EVIDENCE_BUNDLE)?;
        let patches: DocsPatchSet = decode_input(payload, PATCH_SET)?;
        let validated = validate_patch_set(
            api,
            &self.config,
            &self.policy,
            &bundle,
            &patches,
            event_context,
        )
        .await
        .map_err(terminalize_docs_error)?;
        Ok(single_artifact(
            payload,
            runtime_init,
            VALIDATED_PATCH_SET,
            to_value(validated)?,
        ))
    }
}

#[async_trait]
impl CapabilityInvoker for PublishPatchSetCapability {
    type Error = ApiError;
    type ExecutionApi = dyn ExecutionRuntimeContext;

    fn contract(&self) -> CapabilityTypeContract {
        contract(
            PUBLISH_PATCH_SET,
            &[VALIDATED_PATCH_SET],
            PUBLICATION_RECEIPT,
            EffectKind::Write,
        )
    }

    async fn invoke(
        &self,
        _api: &dyn ExecutionRuntimeContext,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        _event_context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;
        let patches: ValidatedDocsPatchSet = decode_input(payload, VALIDATED_PATCH_SET)?;
        let receipt = publish_patch_set(&self.config.target_root, &self.policy, &patches)
            .map_err(terminalize_docs_error)?;
        Ok(single_artifact(
            payload,
            runtime_init,
            PUBLICATION_RECEIPT,
            to_value(receipt)?,
        ))
    }
}

#[async_trait]
impl CapabilityInvoker for AssessPublishedScopeCapability {
    type Error = ApiError;
    type ExecutionApi = dyn ExecutionRuntimeContext;

    fn contract(&self) -> CapabilityTypeContract {
        contract(
            ASSESS_PUBLISHED_SCOPE,
            &[PUBLICATION_RECEIPT],
            FRESHNESS_ASSESSMENT,
            EffectKind::Emit,
        )
    }

    async fn invoke(
        &self,
        _api: &dyn ExecutionRuntimeContext,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        _event_context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;
        let receipt: DocsPublicationReceipt = decode_input(payload, PUBLICATION_RECEIPT)?;
        let assessment =
            assess_published_scope(&self.config.target_root, &self.config.subject_id, &receipt)
                .map_err(terminalize_docs_error)?;
        Ok(single_artifact(
            payload,
            runtime_init,
            FRESHNESS_ASSESSMENT,
            to_value(assessment)?,
        ))
    }
}

fn contract(
    capability_type_id: &str,
    input_artifacts: &[&str],
    output_artifact: &str,
    effect_kind: EffectKind,
) -> CapabilityTypeContract {
    CapabilityTypeContract {
        capability_type_id: capability_type_id.to_string(),
        capability_version: VERSION,
        owning_domain: "docs".to_string(),
        scope_contract: ScopeContract {
            scope_kind: "repository".to_string(),
            scope_ref_kind: "subject_id".to_string(),
            allow_fan_out: false,
        },
        binding_contract: Vec::new(),
        input_contract: input_artifacts
            .iter()
            .map(|artifact| InputSlotSpec {
                slot_id: (*artifact).to_string(),
                accepted_artifact_type_ids: vec![(*artifact).to_string()],
                schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
                required: true,
                cardinality: InputCardinality::One,
            })
            .collect(),
        output_contract: vec![OutputSlotSpec {
            slot_id: output_artifact.to_string(),
            artifact_type_id: output_artifact.to_string(),
            schema_version: VERSION,
            guaranteed: true,
        }],
        effect_contract: vec![EffectSpec {
            effect_id: format!("{capability_type_id}.effect"),
            kind: effect_kind,
            target: "stewarded_repository".to_string(),
            exclusive: matches!(effect_kind, EffectKind::Write),
        }],
        execution_contract: ExecutionContract {
            execution_class: if is_provider_capability(capability_type_id) {
                ExecutionClass::Queued
            } else {
                ExecutionClass::Inline
            },
            completion_semantics: "artifact_or_failure".to_string(),
            retry_class: if is_provider_capability(capability_type_id) {
                "bounded_provider_io".to_string()
            } else {
                "deterministic_local".to_string()
            },
            cancellation_supported: is_provider_capability(capability_type_id),
        },
    }
}

/// Publish the complete docs capability contract set without constructing executors.
pub fn published_contracts() -> Vec<CapabilityTypeContract> {
    vec![
        contract(INSPECT_SCOPE, &[], EVIDENCE_BUNDLE, EffectKind::Emit),
        contract(
            DRAFT_PATCH_SET,
            &[EVIDENCE_BUNDLE],
            PATCH_SET,
            EffectKind::Emit,
        ),
        contract(
            VALIDATE_PATCH_SET,
            &[EVIDENCE_BUNDLE, PATCH_SET],
            VALIDATED_PATCH_SET,
            EffectKind::Emit,
        ),
        contract(
            PUBLISH_PATCH_SET,
            &[VALIDATED_PATCH_SET],
            PUBLICATION_RECEIPT,
            EffectKind::Write,
        ),
        contract(
            ASSESS_PUBLISHED_SCOPE,
            &[PUBLICATION_RECEIPT],
            FRESHNESS_ASSESSMENT,
            EffectKind::Emit,
        ),
    ]
}

/// Register docs invokers selected by exact installed contracts.
///
/// Selection is by capability type, version, and content identity. The
/// stewardship expression name does not participate in implementation
/// binding.
pub fn register_exact_contracts(
    config: DocsCapabilityConfig,
    claim_policy: DocsClaimPolicy,
    exact_contracts: &[CapabilityTypeContract],
    catalog: &mut crate::capability::CapabilityCatalog,
    registry: &mut crate::capability::CapabilityExecutorRegistry,
) -> Result<usize, ApiError> {
    claim_policy.validate()?;
    let mut registered = 0;
    registered += register_exact(
        InspectScopeCapability::new(config.clone()),
        exact_contracts,
        catalog,
        registry,
    )? as usize;
    registered += register_exact(
        DraftPatchSetCapability::new(config.clone()),
        exact_contracts,
        catalog,
        registry,
    )? as usize;
    registered += register_exact(
        ValidatePatchSetCapability::new(config.clone(), claim_policy.clone()),
        exact_contracts,
        catalog,
        registry,
    )? as usize;
    registered += register_exact(
        PublishPatchSetCapability::new(config.clone(), claim_policy),
        exact_contracts,
        catalog,
        registry,
    )? as usize;
    registered += register_exact(
        AssessPublishedScopeCapability::new(config),
        exact_contracts,
        catalog,
        registry,
    )? as usize;
    Ok(registered)
}

fn register_exact<I>(
    invoker: I,
    exact_contracts: &[CapabilityTypeContract],
    catalog: &mut crate::capability::CapabilityCatalog,
    registry: &mut crate::capability::CapabilityExecutorRegistry,
) -> Result<bool, ApiError>
where
    I: CapabilityInvoker<Error = ApiError, ExecutionApi = dyn ExecutionRuntimeContext> + 'static,
{
    let published = invoker.contract();
    let Some(exact) = exact_contracts.iter().find(|contract| {
        contract.capability_type_id == published.capability_type_id
            && contract.capability_version == published.capability_version
    }) else {
        return Ok(false);
    };
    if exact.content_identity() != published.content_identity() {
        return Err(ApiError::ConfigError(format!(
            "docs executor contract identity drift for '{}' version '{}'",
            exact.capability_type_id, exact.capability_version
        )));
    }
    registry.register(catalog, invoker)?;
    Ok(true)
}

fn is_provider_capability(capability_type_id: &str) -> bool {
    matches!(capability_type_id, DRAFT_PATCH_SET | VALIDATE_PATCH_SET)
}

fn terminalize_docs_error(error: ApiError) -> ApiError {
    match error {
        retryable @ (ApiError::ProviderError(_)
        | ApiError::ProviderRequestFailed(_)
        | ApiError::ProviderRateLimit(_)
        | ApiError::StorageError(_)
        | ApiError::GenerationFailed(_)) => retryable,
        terminal => {
            ApiError::ConfigError(format!("{TERMINAL_CAPABILITY_FAILURE_MARKER}: {terminal}"))
        }
    }
}

fn decode_input<T: serde::de::DeserializeOwned>(
    payload: &CapabilityInvocationPayload,
    slot_id: &str,
) -> Result<T, ApiError> {
    let input = payload
        .supplied_inputs
        .iter()
        .find(|input| input.slot_id == slot_id)
        .ok_or_else(|| ApiError::ConfigError(format!("missing docs input slot '{slot_id}'")))?;
    let value = match &input.value {
        SuppliedValueRef::Artifact(artifact) => artifact.content.clone(),
        SuppliedValueRef::StructuredValue(value) => value.clone(),
    };
    serde_json::from_value(value)
        .map_err(|error| ApiError::ConfigError(format!("invalid docs artifact: {error}")))
}

fn to_value(value: impl Serialize) -> Result<Value, ApiError> {
    serde_json::to_value(value)
        .map_err(|error| ApiError::ConfigError(format!("cannot encode docs artifact: {error}")))
}

fn io_error(error: std::io::Error) -> ApiError {
    ApiError::StorageError(crate::error::StorageError::IoError(error))
}

fn single_artifact(
    payload: &CapabilityInvocationPayload,
    runtime_init: &CapabilityRuntimeInit,
    artifact_type: &str,
    content: Value,
) -> CapabilityInvocationResult {
    CapabilityInvocationResult {
        emitted_artifacts: vec![ArtifactRecord {
            artifact_id: format!("{}::{artifact_type}", payload.invocation_id),
            artifact_type_id: artifact_type.to_string(),
            schema_version: VERSION,
            content,
            producer: ArtifactProducerRef {
                task_id: payload
                    .upstream_lineage
                    .as_ref()
                    .map(|lineage| lineage.task_id.clone())
                    .unwrap_or_default(),
                capability_instance_id: runtime_init.capability_instance_id.clone(),
                invocation_id: Some(payload.invocation_id.clone()),
                output_slot_id: Some(artifact_type.to_string()),
            },
        }],
    }
}

pub fn inspect_scope(root: &Path) -> Result<DocsEvidenceBundle, ApiError> {
    let root = root.canonicalize().map_err(|error| {
        ApiError::ConfigError(format!(
            "docs target '{}' is unavailable: {error}",
            root.display()
        ))
    })?;
    let mut direct_files = BTreeMap::<PathBuf, Vec<PathBuf>>::new();
    let mut children = BTreeMap::<PathBuf, BTreeSet<PathBuf>>::new();
    let mut source_hasher = blake3::Hasher::new();

    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(include_entry)
    {
        let entry = entry.map_err(|error| ApiError::ConfigError(error.to_string()))?;
        let path = entry.path();
        if entry.file_type().is_dir() {
            direct_files.entry(path.to_path_buf()).or_default();
            if let Some(parent) = path.parent().filter(|parent| *parent != path) {
                if path != root && parent.starts_with(&root) {
                    children
                        .entry(parent.to_path_buf())
                        .or_default()
                        .insert(path.to_path_buf());
                }
            }
            continue;
        }
        if !entry.file_type().is_file() || is_managed_readme(path) {
            continue;
        }
        let Some(parent) = path.parent() else {
            continue;
        };
        let bytes = std::fs::read(path).map_err(io_error)?;
        if bytes.contains(&0) {
            continue;
        }
        let relative = path.strip_prefix(&root).unwrap_or(path);
        source_hasher.update(relative.to_string_lossy().as_bytes());
        source_hasher.update(&(bytes.len() as u64).to_le_bytes());
        source_hasher.update(blake3::hash(&bytes).as_bytes());
        direct_files
            .entry(parent.to_path_buf())
            .or_default()
            .push(path.to_path_buf());
    }

    let mut meaningful = BTreeSet::new();
    for directory in direct_files
        .iter()
        .filter(|(_, files)| !files.is_empty())
        .map(|(directory, _)| directory)
    {
        let mut cursor = Some(directory.as_path());
        while let Some(path) = cursor {
            if !path.starts_with(&root) {
                break;
            }
            meaningful.insert(path.to_path_buf());
            if path == root {
                break;
            }
            cursor = path.parent();
        }
    }
    if meaningful.len() > MAX_DIRECTORIES {
        return Err(ApiError::ConfigError(format!(
            "docs scope contains {} meaningful directories, exceeding the bound of {MAX_DIRECTORIES}",
            meaningful.len()
        )));
    }

    let mut directories = meaningful.iter().cloned().collect::<Vec<_>>();
    directories.sort_by(|left, right| {
        right
            .components()
            .count()
            .cmp(&left.components().count())
            .then_with(|| left.cmp(right))
    });
    let mut evidence = Vec::new();
    for directory in directories {
        let mut files = direct_files.remove(&directory).unwrap_or_default();
        files.sort();
        let mut rendered = String::new();
        let mut names = Vec::new();
        for file in files {
            let relative = file.strip_prefix(&root).unwrap_or(&file);
            names.push(relative.to_string_lossy().to_string());
            let bytes = std::fs::read(&file).map_err(io_error)?;
            let text = String::from_utf8_lossy(&bytes[..bytes.len().min(MAX_FILE_BYTES)]);
            let section = format!("\n--- {} ---\n{}\n", relative.display(), text);
            if rendered.len() + section.len() > MAX_DIRECTORY_EVIDENCE_BYTES {
                break;
            }
            rendered.push_str(&section);
        }
        let child_directories = children
            .get(&directory)
            .into_iter()
            .flatten()
            .filter(|child| meaningful.contains(*child))
            .filter_map(|child| child.strip_prefix(&root).ok())
            .map(relative_display)
            .collect::<Vec<_>>();
        evidence.push(DirectoryEvidence {
            path: relative_display(directory.strip_prefix(&root).unwrap_or(&directory)),
            direct_files: names,
            child_directories,
            evidence: rendered,
        });
    }
    Ok(DocsEvidenceBundle {
        source_fingerprint: source_hasher.finalize().to_hex().to_string(),
        directories: evidence,
    })
}

async fn draft_patch_set(
    api: &dyn ExecutionRuntimeContext,
    config: &DocsCapabilityConfig,
    bundle: &DocsEvidenceBundle,
    event_context: Option<&ExecutionEventContext>,
) -> Result<DocsPatchSet, ApiError> {
    let mut child_readmes = BTreeMap::<String, String>::new();
    let mut patches = Vec::new();
    for directory in &bundle.directories {
        let child_context = directory
            .child_directories
            .iter()
            .filter_map(|child| child_readmes.get(child).map(|readme| (child, readme)))
            .map(|(child, readme)| {
                format!(
                    "\n--- child {child} README ---\n{}\n",
                    truncate_chars(readme, MAX_CHILD_README_BYTES)
                )
            })
            .collect::<String>();
        let content = generate_readme(
            api,
            config,
            directory,
            &bundle.source_fingerprint,
            &directory.evidence,
            &child_context,
            event_context,
        )
        .await?;
        let readme_path = if directory.path == "." {
            "README.md".to_string()
        } else {
            format!("{}/README.md", directory.path)
        };
        child_readmes.insert(directory.path.clone(), content.clone());
        patches.push(ReadmePatch {
            path: readme_path,
            content_hash: blake3::hash(content.as_bytes()).to_hex().to_string(),
            content,
        });
    }
    patches.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(DocsPatchSet {
        source_fingerprint: bundle.source_fingerprint.clone(),
        patches,
    })
}

async fn generate_readme(
    api: &dyn ExecutionRuntimeContext,
    config: &DocsCapabilityConfig,
    directory: &DirectoryEvidence,
    source_fingerprint: &str,
    direct_evidence: &str,
    child_evidence: &str,
    event_context: Option<&ExecutionEventContext>,
) -> Result<String, ApiError> {
    let preparation_request = generation_request(config, directory, source_fingerprint, 0);
    let preparation = prepare_provider_for_request(api, &preparation_request)?;
    let mut evidence_limit = MAX_DIRECTORY_EVIDENCE_BYTES + MAX_CHILD_README_BYTES;
    let mut last_error = None;
    for retry in 0..=2 {
        let request = generation_request(config, directory, source_fingerprint, retry);
        let messages = readme_messages(directory, direct_evidence, child_evidence, evidence_limit);
        match execute_completion(api, &request, &preparation, messages, event_context).await {
            Ok(response) => {
                let content =
                    stabilize_aggregate_heading(&normalize_markdown(&response.content), directory);
                if content.starts_with('#') && content.len() >= 32 {
                    return Ok(content);
                }
                last_error = Some("provider returned an invalid or empty README".to_string());
            }
            Err(error) => {
                let message = error.to_string();
                if !is_context_limit_error(&message) {
                    return Err(error);
                }
                last_error = Some(message);
            }
        }
        evidence_limit = (evidence_limit / 2).max(2 * 1024);
    }
    Err(ApiError::ConfigError(format!(
        "README generation failed for '{}': {}",
        directory.path,
        last_error.unwrap_or_else(|| "unknown provider failure".to_string())
    )))
}

fn readme_messages(
    directory: &DirectoryEvidence,
    direct_evidence: &str,
    child_evidence: &str,
    evidence_limit: usize,
) -> Vec<ChatMessage> {
    let direct_limit = evidence_limit.min(MAX_DIRECTORY_EVIDENCE_BYTES);
    let child_limit = evidence_limit.saturating_sub(direct_limit);
    let direct_files = if directory.direct_files.is_empty() {
        "- none".to_string()
    } else {
        directory
            .direct_files
            .iter()
            .map(|path| format!("- {path}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let child_directories = if directory.child_directories.is_empty() {
        "- none".to_string()
    } else {
        directory
            .child_directories
            .iter()
            .map(|path| format!("- {path}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let prompt = format!(
        "Current directory path:\n{}\n\nDirect source files in this directory:\n{}\n\nDirect child directories in the managed scope:\n{}\n\nDirect source evidence:\n{}\n\nDescendant README evidence:\n{}\n\nWrite the README for the current directory only and keep it under 450 words. Descendant README evidence describes child components, not files or APIs directly owned by the current directory. Preserve every identifier, path, filename, command, and environment variable exactly as supplied. Never rewrite hyphens as underscores or otherwise normalize identifiers. Do not claim a project name unless direct source evidence establishes it. Do not invent clone URLs, external links, installation steps, credential filenames, configuration filenames, commands, test frameworks, or test invocations. Include any of those only when their exact form is present in direct source evidence. Do not turn a filename or symbol name alone into an unsupported behavioral claim. When the current directory has no direct source files, describe it as an organizational scope over its listed children. Avoid repeating the same inventory in multiple sections. Omit sections that the evidence cannot support. End with a complete sentence or complete list item.",
        directory.path,
        direct_files,
        child_directories,
        truncate_chars(direct_evidence, direct_limit),
        truncate_chars(child_evidence, child_limit),
    );
    vec![
        ChatMessage {
            role: MessageRole::System,
            content: "Write a concise and accurate Markdown README from bounded evidence. Ground every factual claim in the supplied direct or descendant evidence. Keep ownership boundaries explicit and preserve source identifiers exactly. Return Markdown only.".to_string(),
        },
        ChatMessage {
            role: MessageRole::User,
            content: prompt,
        },
    ]
}

fn stabilize_aggregate_heading(content: &str, directory: &DirectoryEvidence) -> String {
    if !directory.direct_files.is_empty() {
        return content.to_string();
    }
    let heading = if directory.path == "." {
        "Repository root"
    } else {
        &directory.path
    };
    let body = content.lines().skip(1).collect::<Vec<_>>().join("\n");
    format!("# {heading}\n{}\n", body.trim_end())
}

fn is_context_limit_error(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    [
        "context window",
        "context length",
        "too many tokens",
        "token limit",
        "maximum context",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn generation_request(
    config: &DocsCapabilityConfig,
    directory: &DirectoryEvidence,
    fingerprint: &str,
    retry_count: usize,
) -> GenerationOrchestrationRequest {
    let digest = blake3::hash(format!("{}::{fingerprint}", directory.path).as_bytes());
    let mut request_bytes = [0_u8; 8];
    request_bytes.copy_from_slice(&digest.as_bytes()[..8]);
    GenerationOrchestrationRequest {
        request_id: u64::from_le_bytes(request_bytes),
        node_id: *digest.as_bytes(),
        agent_id: config.agent_id.clone(),
        provider: config.provider.clone(),
        frame_type: "docs-readme".to_string(),
        retry_count,
        force: true,
    }
}

pub fn publish_patch_set(
    root: &Path,
    policy: &DocsClaimPolicy,
    patches: &ValidatedDocsPatchSet,
) -> Result<DocsPublicationReceipt, ApiError> {
    verify_validated_patch_set(policy, patches)?;
    let root = root.canonicalize().map_err(io_error)?;
    let mut published = Vec::new();
    for patch in &patches.patches {
        let relative = safe_readme_path(&patch.path)?;
        let destination = root.join(&relative);
        let parent = destination.parent().ok_or_else(|| {
            ApiError::ConfigError(format!("README path '{}' has no parent", patch.path))
        })?;
        let canonical_parent = parent.canonicalize().map_err(io_error)?;
        if !canonical_parent.starts_with(&root) {
            return Err(ApiError::ConfigError(format!(
                "README path '{}' escapes the docs target",
                patch.path
            )));
        }
        let actual_hash = blake3::hash(patch.content.as_bytes()).to_hex().to_string();
        if actual_hash != patch.content_hash {
            return Err(ApiError::ConfigError(format!(
                "README patch '{}' content hash is invalid",
                patch.path
            )));
        }
        let changed = std::fs::read(&destination)
            .map(|existing| existing != patch.content.as_bytes())
            .unwrap_or(true);
        if changed {
            std::fs::write(&destination, patch.content.as_bytes()).map_err(io_error)?;
        }
        published.push(PublishedReadme {
            path: patch.path.clone(),
            content_hash: patch.content_hash.clone(),
            changed,
        });
    }
    Ok(DocsPublicationReceipt {
        source_fingerprint: patches.source_fingerprint.clone(),
        policy_identity: patches.policy_identity.clone(),
        validation_fingerprint: patches.validation_fingerprint.clone(),
        weighted_groundedness: patches.weighted_groundedness,
        unsupported_claim_mass: patches.unsupported_claim_mass,
        contradiction_claim_mass: patches.contradiction_claim_mass,
        published,
    })
}

pub fn assess_published_scope(
    root: &Path,
    subject_id: &str,
    receipt: &DocsPublicationReceipt,
) -> Result<DocsFreshnessAssessment, ApiError> {
    let current = inspect_scope(root)?;
    let mut verified = 0;
    for expected in &receipt.published {
        let relative = safe_readme_path(&expected.path)?;
        let bytes = std::fs::read(root.join(relative)).map_err(io_error)?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        if hash == expected.content_hash && !bytes.is_empty() {
            verified += 1;
        }
    }
    let complete = current.source_fingerprint == receipt.source_fingerprint
        && verified == receipt.published.len()
        && !receipt.published.is_empty()
        && !receipt.policy_identity.is_empty()
        && !receipt.validation_fingerprint.is_empty()
        && receipt.weighted_groundedness.is_finite()
        && receipt.weighted_groundedness > 0.0
        && receipt.unsupported_claim_mass == 0.0
        && receipt.contradiction_claim_mass == 0.0;
    Ok(DocsFreshnessAssessment {
        subject_id: subject_id.to_string(),
        source_fingerprint: current.source_fingerprint,
        stale_probability: if complete { 0.0 } else { 1.0 },
        expected_readmes: receipt.published.len(),
        verified_readmes: verified,
        policy_identity: receipt.policy_identity.clone(),
        validation_fingerprint: receipt.validation_fingerprint.clone(),
        weighted_groundedness: receipt.weighted_groundedness,
        unsupported_claim_mass: receipt.unsupported_claim_mass,
        contradiction_claim_mass: receipt.contradiction_claim_mass,
    })
}

fn include_entry(entry: &DirEntry) -> bool {
    if entry.depth() == 0 {
        return true;
    }
    if !entry.file_type().is_dir() {
        return true;
    }
    let name = entry.file_name().to_string_lossy();
    if name.starts_with('.') {
        return false;
    }
    !matches!(
        name.as_ref(),
        ".git"
            | ".venv"
            | ".tox"
            | ".mypy_cache"
            | ".pytest_cache"
            | ".ruff_cache"
            | "node_modules"
            | "target"
            | "__pycache__"
    )
}

fn is_managed_readme(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("README.md"))
}

fn safe_readme_path(path: &str) -> Result<PathBuf, ApiError> {
    let relative = PathBuf::from(path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || relative.file_name().and_then(|name| name.to_str()) != Some("README.md")
    {
        return Err(ApiError::ConfigError(format!(
            "invalid managed README path '{path}'"
        )));
    }
    Ok(relative)
}

fn relative_display(path: &Path) -> String {
    if path.as_os_str().is_empty() {
        ".".to_string()
    } else {
        path.to_string_lossy().replace('\\', "/")
    }
}

fn truncate_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn normalize_markdown(value: &str) -> String {
    let trimmed = value.trim();
    let unwrapped = if trimmed.starts_with("```") && trimmed.ends_with("```") {
        let body = trimmed
            .strip_prefix("```markdown")
            .or_else(|| trimmed.strip_prefix("```md"))
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        body.strip_suffix("```").unwrap_or(body).trim()
    } else {
        trimmed
    };
    format!("{}\n", unwrapped.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docs::claim_validation::{
        extract_claims, CitationScope, ClaimAssessment, ClaimCitation, ClaimVerdict,
        ReadmeClaimReport,
    };

    fn capability_config(root: &Path) -> DocsCapabilityConfig {
        DocsCapabilityConfig {
            target_root: root.to_path_buf(),
            subject_id: "subject".to_string(),
            agent_id: "agent".to_string(),
            provider: ProviderExecutionBinding::new(
                "provider",
                crate::provider::ProviderRuntimeOverrides::default(),
            )
            .unwrap(),
        }
    }

    fn claim_policy() -> DocsClaimPolicy {
        DocsClaimPolicy {
            policy_id: "test-policy".to_string(),
            minimum_claim_confidence: 0.8,
            minimum_groundedness: 0.8,
            maximum_unsupported_claim_mass: 0.0,
            maximum_contradiction_claim_mass: 0.0,
            maximum_revision_attempts: 1,
            title_weight: 1.0,
            prose_weight: 1.0,
            list_item_weight: 1.0,
            code_line_weight: 1.0,
            table_row_weight: 1.0,
        }
    }

    fn validated_patch_set(source_fingerprint: String, content: String) -> ValidatedDocsPatchSet {
        let policy = claim_policy();
        let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
        let assessments = extract_claims("README.md", &content)
            .into_iter()
            .map(|claim| ClaimAssessment {
                claim,
                verdict: ClaimVerdict::Supported,
                confidence: 1.0,
                citations: vec![ClaimCitation {
                    scope: CitationScope::Direct,
                    quote: "pub fn run".to_string(),
                }],
                rationale: "supported".to_string(),
            })
            .collect::<Vec<_>>();
        let reports = vec![ReadmeClaimReport {
            path: "README.md".to_string(),
            content_hash: content_hash.clone(),
            revision_attempts: 0,
            assessments,
            weighted_groundedness: 1.0,
            unsupported_claim_mass: 0.0,
            contradiction_claim_mass: 0.0,
            accepted: true,
        }];
        let patches = vec![ReadmePatch {
            path: "README.md".to_string(),
            content_hash,
            content,
        }];
        let policy_identity = policy.content_identity();
        let seed = serde_json::to_vec(&(&source_fingerprint, &policy_identity, &patches, &reports))
            .unwrap();
        ValidatedDocsPatchSet {
            source_fingerprint,
            policy_id: policy.policy_id,
            policy_identity,
            validation_fingerprint: blake3::hash(&seed).to_hex().to_string(),
            patches,
            reports,
            weighted_groundedness: 1.0,
            unsupported_claim_mass: 0.0,
            contradiction_claim_mass: 0.0,
        }
    }

    #[test]
    fn managed_readmes_do_not_change_source_fingerprint() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("src")).unwrap();
        std::fs::write(root.path().join("src/lib.rs"), "pub fn run() {}\n").unwrap();
        let before = inspect_scope(root.path()).unwrap();
        std::fs::write(root.path().join("README.md"), "# Root\n").unwrap();
        std::fs::write(root.path().join("src/README.md"), "# Source\n").unwrap();
        let after = inspect_scope(root.path()).unwrap();
        assert_eq!(before.source_fingerprint, after.source_fingerprint);
    }

    #[test]
    fn exact_contract_activation_registers_every_selected_docs_invoker() {
        let root = tempfile::tempdir().unwrap();
        let contracts = published_contracts();
        let mut catalog = crate::capability::CapabilityCatalog::new();
        let mut registry = crate::capability::CapabilityExecutorRegistry::new();

        let count = register_exact_contracts(
            capability_config(root.path()),
            claim_policy(),
            &contracts,
            &mut catalog,
            &mut registry,
        )
        .unwrap();

        assert_eq!(count, contracts.len());
        assert!(contracts.iter().all(|contract| registry
            .get(&contract.capability_type_id, contract.capability_version)
            .is_some()));
    }

    #[test]
    fn exact_contract_activation_rejects_implementation_drift() {
        let root = tempfile::tempdir().unwrap();
        let mut contracts = published_contracts();
        contracts[0].execution_contract.retry_class = "drifted".to_string();
        let mut catalog = crate::capability::CapabilityCatalog::new();
        let mut registry = crate::capability::CapabilityExecutorRegistry::new();

        let error = register_exact_contracts(
            capability_config(root.path()),
            claim_policy(),
            &contracts,
            &mut catalog,
            &mut registry,
        )
        .unwrap_err();

        assert!(error.to_string().contains("identity drift"));
    }

    #[test]
    fn publication_and_assessment_verify_exact_bytes() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
        let inspection = inspect_scope(root.path()).unwrap();
        let content = "# Example\n\nGenerated documentation.\n".to_string();
        let patches = validated_patch_set(inspection.source_fingerprint, content);
        let receipt = publish_patch_set(root.path(), &claim_policy(), &patches).unwrap();
        let fresh = assess_published_scope(root.path(), "subject", &receipt).unwrap();
        assert_eq!(fresh.stale_probability, 0.0);
        std::fs::write(root.path().join("README.md"), "# Drifted\n").unwrap();
        let stale = assess_published_scope(root.path(), "subject", &receipt).unwrap();
        assert_eq!(stale.stale_probability, 1.0);
    }

    #[test]
    fn publication_rejects_a_tampered_validation_artifact_before_writing() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
        let inspection = inspect_scope(root.path()).unwrap();
        let mut patches = validated_patch_set(
            inspection.source_fingerprint,
            "# Example\n\nGenerated documentation.\n".to_string(),
        );
        patches.validation_fingerprint = "forged".to_string();

        assert!(publish_patch_set(root.path(), &claim_policy(), &patches).is_err());
        assert!(!root.path().join("README.md").exists());
    }

    #[test]
    fn final_assessment_refuses_invalid_claim_metrics() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
        let inspection = inspect_scope(root.path()).unwrap();
        let patches = validated_patch_set(
            inspection.source_fingerprint,
            "# Example\n\nGenerated documentation.\n".to_string(),
        );
        let mut receipt = publish_patch_set(root.path(), &claim_policy(), &patches).unwrap();
        receipt.unsupported_claim_mass = 0.1;

        let assessment = assess_published_scope(root.path(), "subject", &receipt).unwrap();
        assert_eq!(assessment.stale_probability, 1.0);
    }

    #[test]
    fn publication_rejects_paths_outside_the_target() {
        assert!(safe_readme_path("../README.md").is_err());
        assert!(safe_readme_path("docs/notes.md").is_err());
    }

    #[test]
    fn only_context_limit_failures_enter_the_shrinking_retry_path() {
        assert!(is_context_limit_error("maximum context length exceeded"));
        assert!(is_context_limit_error("too many tokens"));
        assert!(!is_context_limit_error("connection refused"));
    }

    #[test]
    fn deterministic_docs_failure_is_terminal_but_provider_transport_can_retry() {
        let terminal = terminalize_docs_error(ApiError::ConfigError("invalid claims".to_string()));
        assert!(terminal
            .to_string()
            .contains(TERMINAL_CAPABILITY_FAILURE_MARKER));

        let retryable =
            terminalize_docs_error(ApiError::ProviderRequestFailed("offline".to_string()));
        assert!(!retryable
            .to_string()
            .contains(TERMINAL_CAPABILITY_FAILURE_MARKER));
    }

    #[test]
    fn readme_only_directories_do_not_enter_managed_scope_evidence() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("production/scripts")).unwrap();
        std::fs::create_dir(root.path().join("staging")).unwrap();
        std::fs::write(
            root.path().join("production/scripts/deploy.sh"),
            "#!/bin/sh\n",
        )
        .unwrap();
        std::fs::write(root.path().join("staging/README.md"), "# Staging\n").unwrap();

        let bundle = inspect_scope(root.path()).unwrap();
        let paths = bundle
            .directories
            .iter()
            .map(|directory| directory.path.as_str())
            .collect::<Vec<_>>();
        assert_eq!(paths, vec!["production/scripts", "production", "."]);
        let root_evidence = bundle
            .directories
            .iter()
            .find(|directory| directory.path == ".")
            .unwrap();
        assert_eq!(root_evidence.child_directories, vec!["production"]);
    }

    #[test]
    fn hidden_directories_do_not_become_readme_targets() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join(".github/workflows")).unwrap();
        std::fs::write(
            root.path().join(".github/workflows/publish.yaml"),
            "name: publish\n",
        )
        .unwrap();
        std::fs::write(root.path().join("app.py"), "print('hello')\n").unwrap();

        let bundle = inspect_scope(root.path()).unwrap();
        assert_eq!(bundle.directories.len(), 1);
        assert_eq!(bundle.directories[0].path, ".");
        assert_eq!(bundle.directories[0].direct_files, vec!["app.py"]);
    }

    #[test]
    fn readme_prompt_keeps_direct_and_descendant_evidence_separate() {
        let directory = DirectoryEvidence {
            path: "production".to_string(),
            direct_files: Vec::new(),
            child_directories: vec!["production/scripts".to_string()],
            evidence: String::new(),
        };
        let messages = readme_messages(
            &directory,
            "",
            "--- child production/scripts README ---\n# Scripts\n",
            MAX_DIRECTORY_EVIDENCE_BYTES + MAX_CHILD_README_BYTES,
        );
        let prompt = &messages[1].content;
        assert!(prompt.contains("Direct source files in this directory:\n- none"));
        assert!(prompt.contains("Descendant README evidence:"));
        assert!(prompt.contains("not files or APIs directly owned"));
        assert!(prompt.contains("Never rewrite hyphens as underscores"));
        assert!(prompt.contains("Do not invent clone URLs"));
        assert!(prompt.contains("keep it under 450 words"));
    }

    #[test]
    fn aggregate_headings_are_bound_to_the_current_scope() {
        let directory = DirectoryEvidence {
            path: ".".to_string(),
            direct_files: Vec::new(),
            child_directories: vec!["production".to_string()],
            evidence: String::new(),
        };
        assert_eq!(
            stabilize_aggregate_heading("# Staging\n\nBody\n", &directory),
            "# Repository root\n\nBody\n"
        );
    }
}
