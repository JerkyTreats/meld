//! Canonical execution activation inputs and validation receipts.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::planning::MethodLibrary;
use crate::task::package::TaskPackageSpec;

/// Exact capability contract required by an activated method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityContractRef {
    /// Stable capability type identifier.
    pub capability_type_id: String,
    /// Exact published capability version.
    pub capability_version: u32,
}

/// Explicit binding from one method step to one authored task package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MethodTaskPackageBinding {
    /// Stable binding identifier.
    pub binding_id: String,
    /// Method selected by product activation.
    pub method_id: String,
    /// Method step lowered through the task-package path.
    pub package_step_id: String,
    /// Authored task package selected for that step.
    pub package_id: String,
    /// Workflow profile required by the selected package.
    pub workflow_id: String,
}

/// Artifact contract required from the activated docs-freshness path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredArtifactContract {
    /// Stable artifact type identifier.
    pub artifact_type_id: String,
    /// Exact artifact schema version.
    pub schema_version: u32,
}

/// Execution-owned mapping from task outcomes into publication semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationMapping {
    /// Stable mapping identifier.
    pub mapping_id: String,
    /// Event type emitted for successful task outcomes.
    pub success_event_type: String,
    /// Event type emitted for failed task outcomes.
    pub failure_event_type: String,
    /// Source kind used by the receiving belief evidence mapper.
    pub content_source_kind: String,
}

/// Existing-output posture selected for activated task-package execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionForcePolicy {
    /// Reuse an existing compatible output.
    ReuseExisting,
    /// Replace an existing output after execution-owned validation.
    ReplaceExisting,
}

/// Canonical target selector kind accepted by execution activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionTargetKind {
    /// Canonical workspace path.
    Path,
    /// Stable workspace node identifier.
    NodeId,
}

/// Source-neutral canonical target selected for execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionTargetSelector {
    /// Target selector kind.
    pub kind: ExecutionTargetKind,
    /// Canonical target value resolved before semantic stores open.
    pub canonical_value: String,
}

/// Source-neutral repository configuration visible to activation validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionActivationValidationContext {
    /// Configured provider binding identities available to execution.
    pub configured_provider_binding_refs: BTreeSet<String>,
}

impl ExecutionActivationValidationContext {
    /// Build validation context from repository-owned provider identities.
    pub fn from_provider_binding_refs(
        provider_binding_refs: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            configured_provider_binding_refs: provider_binding_refs.into_iter().collect(),
        }
    }

    /// Return true when the repository configuration contains this provider.
    pub fn contains_provider_binding(&self, provider_binding_ref: &str) -> bool {
        self.configured_provider_binding_refs
            .contains(provider_binding_ref)
    }
}

/// Source-neutral activation selection that root assembly may construct.
///
/// This value names execution products but does not contain loaded assets or
/// source provenance. An execution-owned binder resolves it into
/// [`ExecutionActivationInput`] before pure validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionActivationSelection {
    /// Hash binding normalized activation content and resolved deployment coordinates.
    pub activation_hash: String,
    /// Stable logical product activation identifier.
    pub activation_id: String,
    /// Method selected by product activation.
    pub method_id: String,
    /// Authored task package selected by product activation.
    pub task_package_id: String,
    /// Workflow profile required by the selected task package.
    pub workflow_id: String,
    /// Exact method step that must resolve the workspace scan capability.
    pub workspace_scan_step_id: String,
    /// Real workspace scan capability required by the method.
    pub workspace_scan: CapabilityContractRef,
    /// Stable task network identity used by later execution actors.
    pub task_network_id: String,
    /// Artifact required from successful docs-freshness work.
    pub required_artifact: RequiredArtifactContract,
    /// Repository provider binding resolved by execution adapters later.
    pub provider_binding_ref: String,
    /// Frame type supplied to the docs-writer package path.
    pub frame_type: String,
    /// Existing-output posture for package execution.
    pub force_policy: ExecutionForcePolicy,
    /// Canonical target resolved by root assembly.
    pub target: ExecutionTargetSelector,
    /// Canonical publication mapping for task outcomes.
    pub publication: PublicationMapping,
}

/// Execution-bound activation input produced by the execution-owned binder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionActivationInput {
    /// Root-supplied source-neutral execution selection.
    pub selection: ExecutionActivationSelection,
    /// Verified method library supplied as a typed execution product.
    pub method_library: MethodLibrary,
    /// Binding from the selected method step to the selected task package.
    pub method_binding: MethodTaskPackageBinding,
    /// Typed authored task package selected by the binding.
    pub task_package: TaskPackageSpec,
    /// Repository-owned identities used for source-neutral validation.
    pub validation_context: ExecutionActivationValidationContext,
}

/// Deterministic pure-validation acknowledgement for execution activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionActivationValidationReceipt {
    /// Stable receipt id derived from the normalized owner input.
    pub receipt_id: String,
    /// Root hash binding normalized content and resolved deployment coordinates.
    pub activation_hash: String,
    /// Stable logical product activation identifier.
    pub activation_id: String,
    /// Canonical hash of the normalized execution-owned input projection.
    pub input_hash: String,
    /// Explicit method-to-package binding identity.
    pub method_binding_id: String,
    /// Semantic digest of the complete method-to-package binding.
    pub method_binding_digest: String,
    /// Selected method identifier.
    pub method_id: String,
    /// Semantic method-library digest that excludes source provenance.
    pub method_library_digest: String,
    /// Selected task package identifier.
    pub task_package_id: String,
    /// Semantic digest of the typed task package.
    pub task_package_digest: String,
    /// Configured task network identifier.
    pub task_network_id: String,
    /// Filesystem-safe execution-owned storage key for the task network.
    pub task_network_storage_key: String,
    /// Digest of the configured network identity and derived storage key.
    ///
    /// Schema one has no authored topology. Its configured network contract
    /// covers only the network identity and the derived storage key.
    pub task_network_identity_digest: String,
    /// Digest of the required artifact contract.
    pub artifact_contract_digest: String,
    /// Digest of the exact workspace scan capability contract reference.
    pub workspace_scan_contract_digest: String,
    /// Digest of provider, frame, force, and canonical target coordinates.
    pub execution_coordinates_digest: String,
    /// Stable publication mapping identifier.
    pub publication_mapping_id: String,
    /// Digest of the complete publication mapping.
    pub publication_mapping_digest: String,
}
