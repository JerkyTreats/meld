//! Task package compatibility facade and preparation surfaces.

pub mod prepare;
pub mod registry;

use crate::context::belief_context::hydrate_belief_context_bundle;
use crate::error::ApiError;
use crate::execution::{
    BeliefContextReadPort, ContextReadPort, NodeResolutionPort, PromptArtifactReadPort,
};
use crate::workflow::registry::RegisteredWorkflowProfile;
use crate::{capability::CapabilityCatalog, types::NodeID};
use std::path::Path;
use std::path::PathBuf;

pub use meld_execution::task::package::{
    lower_traversal_prerequisite_expansion_template, lower_workflow_region_template,
    InitialSeedSpec, PackageExpansionSpec, PackageRunResolvers, PreparedTaskRun,
    PreparedWorkflowPackageContext, PrerequisiteTemplateSpec, RepeatedRegionSpec, SeedArtifactSpec,
    SeedSourceSpec, StageChainSpec, StageSpec, TargetSelectorKind, TaskPackageSpec,
    TaskTriggerSpec, TraversalPrerequisitePackageExpansionSpec, TraversalPublishSpec,
    TurnOutputPolicySpec, TurnSpec, WorkflowPackageTriggerRequest,
};
pub use prepare::{
    build_initial_task_definition, build_task_initialization_payload,
    find_traversal_prerequisite_expansion, gate_map, prepare_workflow_package_context, prompt_map,
    resolve_package_target_node_id, validate_workflow_package_trigger, workflow_task_run_id,
};
pub use registry::load_task_package_spec_for_workflow;

pub(crate) fn default_user_task_package_dir() -> Result<PathBuf, ApiError> {
    Ok(crate::config::WorkflowConfig::default()
        .resolve_user_profile_dir()?
        .join("packages"))
}

pub(crate) fn resolve_workflow_package_prompt_template(
    api: &(impl PromptArtifactReadPort + ?Sized),
    registered_profile: &RegisteredWorkflowProfile,
    prompt_ref: &str,
) -> Result<String, ApiError> {
    crate::workflow::resolver::resolve_prompt_template(
        api,
        registered_profile.source_path.as_deref(),
        prompt_ref,
    )
}

pub fn workflow_uses_task_package_path(
    registered_profile: &RegisteredWorkflowProfile,
) -> Result<bool, ApiError> {
    meld_execution::task::workflow_uses_task_package_path(
        registered_profile,
        Some(default_user_task_package_dir()?.as_path()),
    )
}

pub fn workflow_task_run_id_for_target(
    registered_profile: &RegisteredWorkflowProfile,
    node_id: NodeID,
) -> String {
    meld_execution::task::workflow_task_run_id_for_target(registered_profile, node_id)
}

pub fn prepare_registered_workflow_task_run(
    api: &(impl ContextReadPort
          + NodeResolutionPort
          + PromptArtifactReadPort
          + BeliefContextReadPort
          + ?Sized),
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    catalog: &CapabilityCatalog,
) -> Result<PreparedTaskRun, ApiError> {
    meld_execution::task::prepare_registered_workflow_task_run(
        api,
        workspace_root,
        registered_profile,
        request,
        catalog,
        Some(default_user_task_package_dir()?.as_path()),
        meld_execution::task::traversal_package_run_resolvers(
            registered_profile,
            request,
            |prompt_ref| {
                resolve_workflow_package_prompt_template(api, registered_profile, prompt_ref)
            },
            // Hydrates the trigger target's belief view into the seed bundle.
            // Only invoked for belief_context-enabled workflows.
            |node_id: NodeID| {
                let family_id = request.belief_family_id.as_deref().ok_or_else(|| {
                    ApiError::ConfigError(format!(
                        "Workflow '{}' enables belief context but its trigger has no belief family binding",
                        request.workflow_id
                    ))
                })?;
                let node_record = api
                    .read_node_record(&node_id)?
                    .ok_or(ApiError::NodeNotFound(node_id))?;
                let subject_path =
                    workspace_relative_subject_path(workspace_root, &node_record.path);
                let bundle = hydrate_belief_context_bundle(api, node_id, &subject_path, family_id)?;
                serde_json::to_value(&bundle).map_err(|err| {
                    ApiError::ConfigError(format!(
                        "Failed to encode belief context bundle artifact: {}",
                        err
                    ))
                })
            },
        ),
    )
}

/// Strips the workspace root from one canonical node path so seeded bundles
/// carry workspace-relative subject paths and bundle digests stay stable
/// across checkout locations. Node paths are canonicalized at scan time, so
/// the root is canonicalized the same way for the prefix strip to match.
fn workspace_relative_subject_path(workspace_root: &Path, node_path: &Path) -> String {
    let canonical_root = crate::tree::path::canonicalize_path(workspace_root)
        .unwrap_or_else(|_| workspace_root.to_path_buf());
    match node_path.strip_prefix(&canonical_root) {
        Ok(relative) if !relative.as_os_str().is_empty() => relative.to_string_lossy().into_owned(),
        // The workspace root itself is the subject.
        Ok(_) => ".".to_string(),
        Err(_) => node_path.to_string_lossy().into_owned(),
    }
}
