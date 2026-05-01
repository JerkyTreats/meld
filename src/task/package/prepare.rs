//! Root wrappers for extracted workflow-backed task package preparation.

use crate::capability::CapabilityCatalog;
use crate::error::ApiError;
use crate::execution::{ContextReadPort, NodeResolutionPort, PromptArtifactReadPort};
use crate::task::expansion::TaskExpansionTemplate;
use crate::task::package::{
    PreparedTaskRun, PreparedWorkflowPackageContext, TaskPackageSpec,
    TraversalPrerequisitePackageExpansionSpec, TurnSpec, WorkflowPackageTriggerRequest,
};
use crate::task::{TaskDefinition, TaskInitializationPayload};
use crate::types::NodeID;
use crate::workflow::profile::{WorkflowGate, WorkflowProfile};
use crate::workflow::registry::RegisteredWorkflowProfile;
use std::collections::HashMap;
use std::path::Path;

/// Returns the deterministic task run id used by workflow-backed task packages.
pub fn workflow_task_run_id(workflow_id: &str, target_node_id: NodeID) -> String {
    meld_execution::task::package::workflow_task_run_id(workflow_id, target_node_id)
}

/// Resolves a workflow package request into shared package context.
pub fn prepare_workflow_package_context(
    api: &(impl ContextReadPort + NodeResolutionPort + PromptArtifactReadPort + ?Sized),
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    package_spec: &TaskPackageSpec,
) -> Result<PreparedWorkflowPackageContext, ApiError> {
    meld_execution::task::package::prepare_workflow_package_context(
        api,
        workspace_root,
        registered_profile,
        request,
        package_spec,
        |prompt_ref| {
            crate::workflow::resolver::resolve_prompt_template(
                api,
                registered_profile.source_path.as_deref(),
                prompt_ref,
            )
        },
    )
}

/// Prepares one workflow-backed task run from a package spec and package-specific expansion lowering.
pub fn prepare_workflow_task_run<F>(
    api: &(impl ContextReadPort + NodeResolutionPort + PromptArtifactReadPort + ?Sized),
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    catalog: &CapabilityCatalog,
    package_spec: &TaskPackageSpec,
    build_expansion_template: F,
) -> Result<PreparedTaskRun, ApiError>
where
    F: FnOnce(&PreparedWorkflowPackageContext) -> Result<TaskExpansionTemplate, ApiError>,
{
    meld_execution::task::package::prepare_workflow_task_run(
        api,
        workspace_root,
        registered_profile,
        request,
        catalog,
        package_spec,
        |prompt_ref| {
            crate::workflow::resolver::resolve_prompt_template(
                api,
                registered_profile.source_path.as_deref(),
                prompt_ref,
            )
        },
        build_expansion_template,
    )
}

/// Validates that a workflow package trigger request satisfies the authored trigger contract.
pub fn validate_workflow_package_trigger(
    package_spec: &TaskPackageSpec,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
) -> Result<(), ApiError> {
    meld_execution::task::package::validate_workflow_package_trigger(
        package_spec,
        registered_profile,
        request,
    )
}

/// Finds the authored traversal prerequisite expansion on a package.
pub fn find_traversal_prerequisite_expansion(
    package_spec: &TaskPackageSpec,
) -> Result<&TraversalPrerequisitePackageExpansionSpec, ApiError> {
    meld_execution::task::package::find_traversal_prerequisite_expansion(package_spec)
}

/// Resolves the trigger target node for one package request.
pub fn resolve_package_target_node_id(
    api: &(impl NodeResolutionPort + ?Sized),
    workspace_root: &Path,
    request: &WorkflowPackageTriggerRequest,
) -> Result<NodeID, ApiError> {
    meld_execution::task::package::resolve_package_target_node_id(api, workspace_root, request)
}

/// Resolves workflow prompt text for the authored package turns.
pub fn prompt_map(
    api: &(impl PromptArtifactReadPort + ?Sized),
    registered_profile: &RegisteredWorkflowProfile,
    turns: &[TurnSpec],
) -> Result<HashMap<String, String>, ApiError> {
    meld_execution::task::package::prompt_map(turns, |prompt_ref| {
        crate::workflow::resolver::resolve_prompt_template(
            api,
            registered_profile.source_path.as_deref(),
            prompt_ref,
        )
    })
}

/// Indexes workflow gates by gate id.
pub fn gate_map(profile: &WorkflowProfile) -> HashMap<String, WorkflowGate> {
    meld_execution::task::package::gate_map(profile)
}

/// Builds the initial traversal-seeding task definition from package authroing data.
pub fn build_initial_task_definition(
    profile: &WorkflowProfile,
    package_spec: &TaskPackageSpec,
) -> TaskDefinition {
    meld_execution::task::package::build_initial_task_definition(profile, package_spec)
}

/// Builds the initialization payload from package seed contracts and one expansion template.
pub fn build_task_initialization_payload(
    profile: &WorkflowProfile,
    package_spec: &TaskPackageSpec,
    request: &WorkflowPackageTriggerRequest,
    target_node_id: NodeID,
    target_path: &str,
    expansion_template: TaskExpansionTemplate,
) -> Result<TaskInitializationPayload, ApiError> {
    Ok(
        meld_execution::task::package::build_task_initialization_payload(
            profile,
            package_spec,
            request,
            target_node_id,
            target_path,
            expansion_template,
        )?,
    )
}
