//! Task package compatibility facade and preparation surfaces.

pub mod prepare;
pub mod registry;

use crate::error::ApiError;
use crate::execution::PromptArtifactReadPort;
use crate::workflow::registry::RegisteredWorkflowProfile;
use std::path::PathBuf;

pub use meld_execution::task::package::{
    lower_traversal_prerequisite_expansion_template, lower_workflow_region_template,
    InitialSeedSpec, PackageExpansionSpec, PreparedTaskRun, PreparedWorkflowPackageContext,
    PrerequisiteTemplateSpec, RepeatedRegionSpec, SeedArtifactSpec, SeedSourceSpec, StageChainSpec,
    StageSpec, TargetSelectorKind, TaskPackageSpec, TaskTriggerSpec,
    TraversalPrerequisitePackageExpansionSpec, TraversalPublishSpec, TurnOutputPolicySpec,
    TurnSpec, WorkflowPackageTriggerRequest,
};
pub use prepare::{
    build_initial_task_definition, build_task_initialization_payload,
    find_traversal_prerequisite_expansion, gate_map, prepare_workflow_package_context,
    prepare_workflow_task_run, prompt_map, resolve_package_target_node_id,
    validate_workflow_package_trigger, workflow_task_run_id,
};
pub use registry::{
    load_builtin_task_package_spec, load_builtin_task_package_spec_for_workflow,
    load_task_package_spec_for_workflow,
};

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
