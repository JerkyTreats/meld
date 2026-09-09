//! Atomic documentation capability publication and invocation.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::observation::MAX_DIRECTORY_EVIDENCE_BYTES;
use super::semantics::DocsJudgmentOperation;
use crate::capability::{
    ArtifactSchemaVersionRange, CapabilityInvocationPayload, CapabilityInvocationResult,
    CapabilityInvoker, CapabilityRuntimeInit, CapabilityTypeContract, EffectKind, EffectSpec,
    ExecutionClass, ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec,
    ScopeContract, SuppliedValueRef,
};
use crate::docs::claim_validation::{
    validate_patch_set, verify_validated_patch_set, DocsClaimPolicy, ValidatedDocsPatchSet,
};
use crate::error::ApiError;
use crate::execution::{ExecutionEventContext, ExecutionRuntimeContext};
use crate::provider::ProviderExecutionBinding;
use crate::task::{ArtifactProducerRef, ArtifactRecord};

pub const INSPECT_SCOPE: &str = "docs.inspect_scope";
pub const DRAFT_PATCH_SET: &str = "docs.draft_patch_set";
pub const VALIDATE_PATCH_SET: &str = "docs.validate_patch_set";
pub const PUBLISH_PATCH_SET: &str = "docs.publish_patch_set";

pub const EVIDENCE_BUNDLE: &str = "docs_evidence_bundle";
pub const PATCH_SET: &str = "docs_patch_set";
pub const VALIDATED_PATCH_SET: &str = "docs_validated_patch_set";
pub const PUBLICATION_RECEIPT: &str = "docs_publication_receipt";

const VERSION: u32 = 1;
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
    /// Exact current observations. Historical artifacts without this field cannot prove observation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<super::observation::DocsScopeObservation>,
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

#[derive(Debug, Clone)]
pub struct InspectScopeCapability {
    config: DocsCapabilityConfig,
    policy: DocsClaimPolicy,
}

#[derive(Debug, Clone)]
pub struct DraftPatchSetCapability {
    config: DocsCapabilityConfig,
    policy: DocsClaimPolicy,
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

impl InspectScopeCapability {
    pub fn new(config: DocsCapabilityConfig, policy: DocsClaimPolicy) -> Self {
        Self { config, policy }
    }
}

impl DraftPatchSetCapability {
    pub fn new(config: DocsCapabilityConfig, policy: DocsClaimPolicy) -> Self {
        Self { config, policy }
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
        self.invoke_owned(_api, runtime_init, payload, _event_context)
            .await
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
        self.invoke_owned(api, runtime_init, payload, event_context)
            .await
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
        self.invoke_owned(api, runtime_init, payload, event_context)
            .await
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

    async fn recover(
        &self,
        events: Option<&meld_events::EventReplayCapability>,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<Option<CapabilityInvocationResult>, ApiError> {
        let patches: ValidatedDocsPatchSet = decode_input(payload, VALIDATED_PATCH_SET)?;
        let invocation = super::publication_return::PublicationInvocation::new(
            &self.config,
            &self.policy,
            &patches,
            runtime_init,
            payload,
            event_context,
        )?;
        let Some(events) = events else {
            return Ok(None);
        };
        invocation
            .recover(events)?
            .map(|receipt| {
                Ok(single_artifact(
                    payload,
                    runtime_init,
                    PUBLICATION_RECEIPT,
                    to_value(receipt)?,
                ))
            })
            .transpose()
    }

    async fn invoke(
        &self,
        api: &dyn ExecutionRuntimeContext,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        self.invoke_owned(api, runtime_init, payload, event_context)
            .await
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
            completion_semantics: if capability_type_id == DRAFT_PATCH_SET {
                crate::capability::EXACT_INPUT_SHARING_V1.to_string()
            } else {
                "artifact_or_failure".to_string()
            },
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
        InspectScopeCapability::new(config.clone(), claim_policy.clone()),
        exact_contracts,
        catalog,
        registry,
    )? as usize;
    registered += register_exact(
        DraftPatchSetCapability::new(config.clone(), claim_policy.clone()),
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
        PublishPatchSetCapability::new(config, claim_policy),
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
        terminal @ ApiError::TerminalCapabilityFailure(_) => terminal,
        terminal => ApiError::TerminalCapabilityFailure(Box::new(terminal)),
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

impl InspectScopeCapability {
    pub(crate) async fn invoke_owned<
        P: crate::runtime::owners::execution::OwnerExecutionPorts + ?Sized,
    >(
        &self,
        _api: &P,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        _event_context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;
        let bundle = super::observation::inspect_scope_selected(
            &self.config.target_root,
            self.policy.semantics()?.scope()?,
        )
        .map_err(terminalize_docs_error)?;
        Ok(single_artifact(
            payload,
            runtime_init,
            EVIDENCE_BUNDLE,
            to_value(bundle)?,
        ))
    }
}

impl DraftPatchSetCapability {
    pub(crate) async fn invoke_owned<
        P: crate::runtime::owners::execution::OwnerExecutionPorts + ?Sized,
    >(
        &self,
        api: &P,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;
        let bundle: DocsEvidenceBundle = decode_input(payload, EVIDENCE_BUNDLE)?;
        let patches = draft_patch_set(api, &self.config, &self.policy, &bundle, event_context)
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

impl ValidatePatchSetCapability {
    pub(crate) async fn invoke_owned<
        P: crate::runtime::owners::execution::OwnerExecutionPorts + ?Sized,
    >(
        &self,
        api: &P,
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

impl PublishPatchSetCapability {
    pub(crate) async fn invoke_owned<
        P: crate::runtime::owners::execution::OwnerExecutionPorts + ?Sized,
    >(
        &self,
        api: &P,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;
        let patches: ValidatedDocsPatchSet = decode_input(payload, VALIDATED_PATCH_SET)?;
        let invocation = super::publication_return::PublicationInvocation::new(
            &self.config,
            &self.policy,
            &patches,
            runtime_init,
            payload,
            event_context,
        )
        .map_err(terminalize_docs_error)?;
        let events = api.owner_events().ok_or_else(|| {
            terminalize_docs_error(ApiError::ConfigError(
                "Docs publication has no durable Event authority".into(),
            ))
        })?;
        let receipt = invocation
            .publish(&events)
            .map_err(terminalize_docs_error)?;
        Ok(single_artifact(
            payload,
            runtime_init,
            PUBLICATION_RECEIPT,
            to_value(receipt)?,
        ))
    }
}

#[cfg(test)]
pub use super::observation::inspect_scope;

pub(crate) async fn draft_patch_set<P: crate::provider::ProviderCompletionPort + ?Sized>(
    api: &P,
    config: &DocsCapabilityConfig,
    policy: &DocsClaimPolicy,
    bundle: &DocsEvidenceBundle,
    event_context: Option<&ExecutionEventContext>,
) -> Result<DocsPatchSet, ApiError> {
    super::observation::validate_selected_scope(policy, bundle)?;
    policy.validate()?;
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
        let readme_path = policy.semantics()?.scope()?.document_path(&directory.path);
        let current_readme = bundle.observation.as_ref().and_then(|observation| {
            observation.readmes.iter().find_map(|readme| {
                if readme.path != readme_path {
                    return None;
                }
                match &readme.state {
                    super::observation::ObservedReadmeState::Present { content, .. } => {
                        Some(content.as_str())
                    }
                    _ => None,
                }
            })
        });
        let content = generate_readme(
            api,
            config,
            policy,
            directory,
            &directory.evidence,
            &child_context,
            current_readme,
            event_context,
        )
        .await?;
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

#[allow(clippy::too_many_arguments)]
async fn generate_readme<P: crate::provider::ProviderCompletionPort + ?Sized>(
    api: &P,
    config: &DocsCapabilityConfig,
    policy: &DocsClaimPolicy,
    directory: &DirectoryEvidence,
    direct_evidence: &str,
    child_evidence: &str,
    current_readme: Option<&str>,
    event_context: Option<&ExecutionEventContext>,
) -> Result<String, ApiError> {
    let mut evidence_limit = MAX_DIRECTORY_EVIDENCE_BYTES + MAX_CHILD_README_BYTES;
    let mut last_error = None;
    for retry in 0..=2 {
        let generation = policy.semantics()?.generation(
            config,
            &policy.content_identity(),
            DocsJudgmentOperation::Drafting,
            readme_input(
                directory,
                direct_evidence,
                child_evidence,
                current_readme,
                evidence_limit,
            ),
            retry,
            0,
        )?;
        match api
            .complete_provider_request(&generation.request, generation.messages, event_context)
            .await
            .map(|completion| completion.response)
        {
            Ok(response) => {
                let content = normalize_markdown(&response.content);
                if !content.trim().is_empty() {
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

fn readme_input(
    directory: &DirectoryEvidence,
    direct_evidence: &str,
    child_evidence: &str,
    current_readme: Option<&str>,
    evidence_limit: usize,
) -> serde_json::Value {
    let direct_limit = evidence_limit.min(MAX_DIRECTORY_EVIDENCE_BYTES);
    let child_limit = evidence_limit.saturating_sub(direct_limit);
    serde_json::json!({
        "directory": directory.path,
        "direct_files": directory.direct_files,
        "child_directories": directory.child_directories,
        "direct_evidence": truncate_chars(direct_evidence, direct_limit),
        "descendant_drafts": truncate_chars(child_evidence, child_limit),
        "current_readme": current_readme,
    })
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

pub(super) fn publish_patch_set(
    root: &Path,
    policy: &DocsClaimPolicy,
    patches: &ValidatedDocsPatchSet,
) -> Result<DocsPublicationReceipt, ApiError> {
    verify_validated_patch_set(policy, patches)?;
    let root = root.canonicalize().map_err(io_error)?;
    let bundle = super::observation::inspect_scope_selected(&root, policy.semantics()?.scope()?)?;
    if bundle.source_fingerprint != patches.source_fingerprint {
        return Err(ApiError::ConfigError(
            "docs source changed after claim validation".into(),
        ));
    }
    super::claim_validation::verify_publication_evidence(&bundle, patches)?;
    let mut published = Vec::new();
    for patch in &patches.patches {
        let relative = safe_document_path(&patch.path, policy.semantics()?.scope()?)?;
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
        let destination =
            canonical_parent.join(relative.file_name().expect("validated README name"));
        let existing = match std::fs::symlink_metadata(&destination) {
            Ok(metadata) if metadata.file_type().is_file() => Some(metadata),
            Ok(_) => {
                return Err(ApiError::ConfigError(format!(
                    "README path '{}' is not a regular file",
                    patch.path
                )))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(io_error(error)),
        };
        let changed = match &existing {
            Some(_) => std::fs::read(&destination).map_err(io_error)? != patch.content.as_bytes(),
            None => true,
        };
        if changed {
            replace_readme(
                &canonical_parent,
                &destination,
                existing.as_ref(),
                patch.content.as_bytes(),
            )?;
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

// Replacing the directory entry avoids following a raced leaf symlink or changing
// other hard links to the old README. This is one file commit, not a scope transaction.
fn replace_readme(
    parent: &Path,
    destination: &Path,
    existing: Option<&std::fs::Metadata>,
    content: &[u8],
) -> Result<(), ApiError> {
    // Hidden staging stays outside the canonical reader's managed source scope.
    let stage = tempfile::Builder::new()
        .prefix(".meld-docs-")
        .tempdir_in(parent)
        .map_err(io_error)?;
    let mut builder = tempfile::Builder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o666));
    }
    let mut file = builder.tempfile_in(stage.path()).map_err(io_error)?;
    if let Some(metadata) = existing {
        file.as_file()
            .set_permissions(metadata.permissions())
            .map_err(io_error)?;
    }
    file.write_all(content).map_err(io_error)?;
    file.as_file().sync_all().map_err(io_error)?;
    file.persist(destination)
        .map_err(|error| io_error(error.error))?;
    #[cfg(unix)]
    std::fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(io_error)?;
    Ok(())
}

fn safe_document_path(
    path: &str,
    scope: &super::scope::DocsScopePolicy,
) -> Result<PathBuf, ApiError> {
    let relative = PathBuf::from(path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || relative.file_name().and_then(|name| name.to_str()) != Some(scope.document_name.as_str())
    {
        return Err(ApiError::ConfigError(format!(
            "invalid managed README path '{path}'"
        )));
    }
    Ok(relative)
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
            semantic_theory: crate::docs::claim_observation::test_support::policy().semantic_theory,
            acceptance_evaluator: Some(
                crate::docs::claim_validation::DocsAcceptanceEvaluator::WeightedClaimMassV1,
            ),
            minimum_claim_confidence: 0.8,
            minimum_groundedness: 0.8,
            maximum_unsupported_claim_mass: 0.0,
            maximum_contradiction_claim_mass: 0.0,
            maximum_revision_attempts: 2,
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
                execution: None,
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

    #[tokio::test]
    async fn publication_receipt_recovers_exact_history_without_touching_changed_workspace() {
        use super::super::publication_return::PublicationInvocation;
        use crate::capability::{CapabilityExecutionContext, InputValueSource, SuppliedInputValue};
        use meld_events::{DomainObjectRef, EventAuthority, EventAuthorityOpenOptions};

        let root = tempfile::tempdir().unwrap();
        let source_path = root.path().join("lib.rs");
        let readme_path = root.path().join("README.md");
        std::fs::write(&source_path, "pub fn run() {}\n").unwrap();
        let patches = validated_patch_set(
            inspect_scope(root.path()).unwrap().source_fingerprint,
            "# run\n\n`run` exists.\n".into(),
        );
        let config = capability_config(root.path());
        let policy = claim_policy();
        let publisher = PublishPatchSetCapability::new(config.clone(), policy.clone());
        let contract = publisher.contract();
        let runtime = CapabilityRuntimeInit {
            capability_instance_id: "publication-instance".into(),
            capability_type_id: contract.capability_type_id,
            capability_version: contract.capability_version,
            scope_ref: config.subject_id.clone(),
            scope_kind: contract.scope_contract.scope_kind,
            binding_values: Vec::new(),
            input_contract: contract.input_contract,
            output_contract: contract.output_contract,
            effect_contract: contract.effect_contract,
            execution_contract: contract.execution_contract,
        };
        let payload = CapabilityInvocationPayload {
            invocation_id: "publication-invocation".into(),
            capability_instance_id: runtime.capability_instance_id.clone(),
            supplied_inputs: vec![SuppliedInputValue {
                slot_id: VALIDATED_PATCH_SET.into(),
                source: InputValueSource::ArtifactHandoff,
                value: SuppliedValueRef::StructuredValue(serde_json::to_value(&patches).unwrap()),
            }],
            upstream_lineage: None,
            execution_context: CapabilityExecutionContext::default(),
        };
        let context = ExecutionEventContext {
            session_id: "publication-session".into(),
            effect_authority: Some(meld_execution::ExecutionEffectAuthority {
                request_ref: None,
                issuer_ref: config.agent_id.clone(),
                principal_id: "workspace-owner".into(),
                subject: DomainObjectRef::new("workspace", "repository", &config.subject_id)
                    .unwrap(),
                fence_ref: "task-fence".into(),
            }),
        };
        let events = EventAuthority::open(
            sled::Config::new().temporary(true).open().unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let replay = events.replay_capability();
        assert!(
            PublicationInvocation::new(&config, &policy, &patches, &runtime, &payload, None)
                .is_err()
        );
        assert!(publisher
            .recover(Some(&replay), &runtime, &payload, Some(&context))
            .await
            .unwrap()
            .is_none());
        assert!(!readme_path.exists());
        let invocation = PublicationInvocation::new(
            &config,
            &policy,
            &patches,
            &runtime,
            &payload,
            Some(&context),
        )
        .unwrap();
        let receipt = invocation.publish(&events.append_capability()).unwrap();
        assert!(receipt.published[0].changed);
        let original = publisher
            .recover(Some(&replay), &runtime, &payload, Some(&context))
            .await
            .unwrap()
            .unwrap();
        let watermark = events.watermark_capability().snapshot().unwrap();

        std::fs::write(&source_path, "pub fn changed() {}\n").unwrap();
        std::fs::write(&readme_path, "external revision\n").unwrap();
        let modified = std::fs::metadata(&readme_path).unwrap().modified().unwrap();
        assert_eq!(
            invocation.publish(&events.append_capability()).unwrap(),
            receipt
        );
        assert_eq!(
            publisher
                .recover(Some(&replay), &runtime, &payload, Some(&context))
                .await
                .unwrap()
                .unwrap()
                .emitted_artifacts,
            original.emitted_artifacts
        );
        assert_eq!(
            std::fs::read_to_string(&readme_path).unwrap(),
            "external revision\n"
        );
        assert_eq!(
            std::fs::metadata(&readme_path).unwrap().modified().unwrap(),
            modified
        );

        let mut foreign_payload = payload.clone();
        foreign_payload.invocation_id = "another-invocation".into();
        assert!(publisher
            .recover(Some(&replay), &runtime, &foreign_payload, Some(&context))
            .await
            .unwrap()
            .is_none());
        let mut foreign_context = context.clone();
        foreign_context.effect_authority.as_mut().unwrap().fence_ref = "another-fence".into();
        assert!(publisher
            .recover(Some(&replay), &runtime, &payload, Some(&foreign_context))
            .await
            .unwrap()
            .is_none());
        foreign_context
            .effect_authority
            .as_mut()
            .unwrap()
            .issuer_ref = "another-agent".into();
        assert!(publisher
            .recover(Some(&replay), &runtime, &payload, Some(&foreign_context))
            .await
            .is_err());
        let foreign_events = EventAuthority::open(
            sled::Config::new().temporary(true).open().unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        assert!(publisher
            .recover(
                Some(&foreign_events.replay_capability()),
                &runtime,
                &payload,
                Some(&context)
            )
            .await
            .unwrap()
            .is_none());
        let mut changed_policy = policy.clone();
        changed_policy.policy_id = "another-policy".into();
        let changed_publisher = PublishPatchSetCapability::new(config.clone(), changed_policy);
        assert!(changed_publisher
            .recover(Some(&replay), &runtime, &payload, Some(&context))
            .await
            .is_err());

        std::fs::remove_dir_all(root.path()).unwrap();
        assert_eq!(
            publisher
                .recover(Some(&replay), &runtime, &payload, Some(&context))
                .await
                .unwrap()
                .unwrap()
                .emitted_artifacts,
            original.emitted_artifacts
        );
        assert_eq!(events.watermark_capability().snapshot().unwrap(), watermark);
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

    #[cfg(unix)]
    #[test]
    fn publication_refuses_symlink_destinations_without_changing_foreign_files() {
        for original in [Some("foreign content\n"), None] {
            let root = tempfile::tempdir().unwrap();
            let foreign = tempfile::tempdir().unwrap();
            let target = foreign.path().join("private.txt");
            if let Some(content) = original {
                std::fs::write(&target, content).unwrap();
            }
            std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
            let source = inspect_scope(root.path()).unwrap();
            std::os::unix::fs::symlink(&target, root.path().join("README.md")).unwrap();
            let patches = validated_patch_set(
                source.source_fingerprint,
                "# Tool\n\n`run` exists.\n".into(),
            );
            let result = publish_patch_set(root.path(), &claim_policy(), &patches);
            assert_eq!(
                std::fs::read_to_string(&target).ok().as_deref(),
                original,
                "README publication must not alter a symlink target"
            );
            assert!(
                result.is_err(),
                "a nonregular README must remain unresolved"
            );
            assert!(std::fs::symlink_metadata(root.path().join("README.md"))
                .unwrap()
                .file_type()
                .is_symlink());
        }
    }

    #[cfg(unix)]
    #[test]
    fn publication_replaces_one_readme_without_mutating_other_hard_links() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let root = tempfile::tempdir().unwrap();
        let foreign = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("lib.rs"), "pub fn run() {}\n").unwrap();
        let readme = root.path().join("README.md");
        let target = foreign.path().join("prior.txt");
        std::fs::write(&target, "original\n").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640)).unwrap();
        std::fs::hard_link(&target, &readme).unwrap();
        let source = inspect_scope(root.path()).unwrap();
        let patches = validated_patch_set(
            source.source_fingerprint.clone(),
            "# Tool\n\n`run` exists.\n".into(),
        );
        let receipt = publish_patch_set(root.path(), &claim_policy(), &patches).unwrap();
        assert!(receipt.published[0].changed);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "original\n");
        assert_eq!(
            std::fs::read_to_string(&readme).unwrap(),
            patches.patches[0].content
        );
        let installed = std::fs::metadata(&readme).unwrap();
        assert_ne!(installed.ino(), std::fs::metadata(&target).unwrap().ino());
        assert_eq!(installed.permissions().mode() & 0o777, 0o640);
        let second = publish_patch_set(root.path(), &claim_policy(), &patches).unwrap();
        assert!(!second.published[0].changed);
        assert_eq!(std::fs::metadata(&readme).unwrap().ino(), installed.ino());
        assert_eq!(
            inspect_scope(root.path()).unwrap().source_fingerprint,
            source.source_fingerprint
        );
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 2);
    }

    #[test]
    fn changed_source_is_rejected_before_publication_writes() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("lib.rs"), "original\n").unwrap();
        let inspection = inspect_scope(root.path()).unwrap();
        let patches = validated_patch_set(inspection.source_fingerprint, "# Tool\n".into());
        std::fs::write(root.path().join("lib.rs"), "successor\n").unwrap();
        assert!(publish_patch_set(root.path(), &claim_policy(), &patches)
            .unwrap_err()
            .to_string()
            .contains("source changed"));
        assert!(!root.path().join("README.md").exists());
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
    fn publication_rejects_paths_outside_the_target() {
        assert!(safe_document_path(
            "../README.md",
            claim_policy().semantics().unwrap().scope().unwrap()
        )
        .is_err());
        assert!(safe_document_path(
            "docs/notes.md",
            claim_policy().semantics().unwrap().scope().unwrap()
        )
        .is_err());
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
        assert!(matches!(terminal, ApiError::TerminalCapabilityFailure(_)));

        let retryable = terminalize_docs_error(ApiError::ProviderRequestFailed(
            "upstream says Terminal capability failure: Workflow gate 'foreign' failed".into(),
        ));
        assert!(matches!(retryable, ApiError::ProviderRequestFailed(_)));
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
    fn readme_input_keeps_source_and_unvalidated_descendant_drafts_separate() {
        let directory = DirectoryEvidence {
            path: "production".into(),
            direct_files: vec![],
            child_directories: vec!["production/scripts".into()],
            evidence: String::new(),
        };
        let input = readme_input(
            &directory,
            "",
            "child draft",
            Some("Existing documented behavior."),
            MAX_DIRECTORY_EVIDENCE_BYTES + MAX_CHILD_README_BYTES,
        );
        assert_eq!(input["directory"], "production");
        assert_eq!(input["direct_files"], serde_json::json!([]));
        assert_eq!(input["descendant_drafts"], "child draft");
        assert_eq!(input["current_readme"], "Existing documented behavior.");
    }
}
