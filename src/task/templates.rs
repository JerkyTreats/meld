//! Task template materialization entrypoints.

use crate::api::ContextApi;
use crate::capability::CapabilityCatalog;
use crate::error::ApiError;
use crate::task::package::{
    load_task_package_spec_for_workflow, lower_traversal_prerequisite_expansion_template,
    prepare_workflow_task_run, workflow_task_run_id, PreparedTaskRun,
    WorkflowPackageTriggerRequest,
};
use crate::types::NodeID;
use crate::workflow::registry::RegisteredWorkflowProfile;
use std::path::Path;

/// Returns true when a registered workflow has a task package route.
pub fn workflow_uses_task_package_path(
    registered_profile: &RegisteredWorkflowProfile,
) -> Result<bool, ApiError> {
    Ok(load_task_package_spec_for_workflow(registered_profile)?.is_some())
}

/// Returns the deterministic task run id for one workflow target.
pub fn workflow_task_run_id_for_target(
    registered_profile: &RegisteredWorkflowProfile,
    node_id: NodeID,
) -> String {
    workflow_task_run_id(&registered_profile.profile.workflow_id, node_id)
}

/// Prepares one registered workflow through the generic task package path.
pub fn prepare_registered_workflow_task_run(
    api: &ContextApi,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    catalog: &CapabilityCatalog,
) -> Result<PreparedTaskRun, ApiError> {
    let package_spec =
        load_task_package_spec_for_workflow(registered_profile)?.ok_or_else(|| {
            ApiError::ConfigError(format!(
                "Workflow '{}' does not have a task package route",
                registered_profile.profile.workflow_id
            ))
        })?;

    prepare_workflow_task_run(
        api,
        workspace_root,
        registered_profile,
        request,
        catalog,
        &package_spec,
        |context| {
            lower_traversal_prerequisite_expansion_template(
                &registered_profile.profile,
                request,
                &context.traversal_expansion,
                context,
            )
        },
    )
}
