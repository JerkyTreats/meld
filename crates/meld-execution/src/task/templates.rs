//! Task template materialization entrypoints.

use crate::capability::CapabilityCatalog;
use crate::error::ApiError;
use crate::execution::{ContextReadPort, NodeResolutionPort};
use crate::generation::NodeId;
use crate::task::package::{
    load_task_package_spec_for_workflow, lower_traversal_prerequisite_expansion_template,
    prepare_workflow_task_run, workflow_task_run_id, PreparedTaskRun,
    WorkflowPackageTriggerRequest,
};
use crate::workflow::registry::RegisteredWorkflowProfile;
use std::path::Path;

/// Returns true when a registered workflow has a task package route.
pub fn workflow_uses_task_package_path<E>(
    registered_profile: &RegisteredWorkflowProfile,
    default_package_dir: Option<&Path>,
) -> Result<bool, E>
where
    E: From<ApiError>,
{
    Ok(load_task_package_spec_for_workflow(registered_profile, default_package_dir)?.is_some())
}

/// Returns the deterministic task run id for one workflow target.
pub fn workflow_task_run_id_for_target(
    registered_profile: &RegisteredWorkflowProfile,
    node_id: NodeId,
) -> String {
    workflow_task_run_id(&registered_profile.profile.workflow_id, node_id)
}

/// Prepares one registered workflow through the generic task package path.
pub fn prepare_registered_workflow_task_run<E, A, R>(
    api: &A,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    catalog: &CapabilityCatalog,
    default_package_dir: Option<&Path>,
    resolve_prompt: R,
) -> Result<PreparedTaskRun, E>
where
    E: From<ApiError>,
    A: ContextReadPort<Error = E, NodeId = NodeId>
        + NodeResolutionPort<Error = E, NodeId = NodeId>
        + ?Sized,
    R: FnMut(&str) -> Result<String, E>,
{
    let package_spec =
        load_task_package_spec_for_workflow(registered_profile, default_package_dir)?.ok_or_else(
            || {
                ApiError::ConfigError(format!(
                    "Workflow '{}' does not have a task package route",
                    registered_profile.profile.workflow_id
                ))
            },
        )?;

    prepare_workflow_task_run(
        api,
        workspace_root,
        registered_profile,
        request,
        catalog,
        &package_spec,
        resolve_prompt,
        |context| {
            Ok(lower_traversal_prerequisite_expansion_template(
                &registered_profile.profile,
                request,
                &context.traversal_expansion,
                context,
            )?)
        },
    )
}
