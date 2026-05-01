//! Root wrappers for task template materialization.

use crate::capability::CapabilityCatalog;
use crate::error::ApiError;
use crate::execution::{ContextReadPort, NodeResolutionPort, PromptArtifactReadPort};
use crate::task::package::{PreparedTaskRun, WorkflowPackageTriggerRequest};
use crate::types::NodeID;
use crate::workflow::registry::RegisteredWorkflowProfile;
use std::path::{Path, PathBuf};

/// Returns true when a registered workflow has a task package route.
pub fn workflow_uses_task_package_path(
    registered_profile: &RegisteredWorkflowProfile,
) -> Result<bool, ApiError> {
    let default_package_dir = default_task_package_dir()?;
    meld_execution::task::workflow_uses_task_package_path(
        registered_profile,
        Some(default_package_dir.as_path()),
    )
}

/// Returns the deterministic task run id for one workflow target.
pub fn workflow_task_run_id_for_target(
    registered_profile: &RegisteredWorkflowProfile,
    node_id: NodeID,
) -> String {
    meld_execution::task::workflow_task_run_id_for_target(registered_profile, node_id)
}

/// Prepares one registered workflow through the generic task package path.
pub fn prepare_registered_workflow_task_run(
    api: &(impl ContextReadPort + NodeResolutionPort + PromptArtifactReadPort + ?Sized),
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowPackageTriggerRequest,
    catalog: &CapabilityCatalog,
) -> Result<PreparedTaskRun, ApiError> {
    let default_package_dir = default_task_package_dir()?;
    meld_execution::task::prepare_registered_workflow_task_run(
        api,
        workspace_root,
        registered_profile,
        request,
        catalog,
        Some(default_package_dir.as_path()),
        |prompt_ref| {
            crate::workflow::resolver::resolve_prompt_template(
                api,
                registered_profile.source_path.as_deref(),
                prompt_ref,
            )
        },
    )
}

fn default_task_package_dir() -> Result<PathBuf, ApiError> {
    Ok(crate::config::WorkflowConfig::default()
        .resolve_user_profile_dir()?
        .join("packages"))
}
