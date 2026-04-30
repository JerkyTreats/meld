//! Task package compatibility facade and preparation surfaces.

pub mod prepare;
pub mod registry;

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
