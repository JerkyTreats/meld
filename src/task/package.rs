//! Task package contracts and authored package surfaces.

pub mod contracts;
pub mod lower;
pub mod output;
pub mod prepare;
pub mod prerequisite;
pub mod region;
pub mod registry;
pub mod seed;
pub mod trigger;

pub use contracts::{
    PackageExpansionSpec, PreparedTaskRun, PreparedWorkflowPackageContext, TaskPackageSpec,
    TraversalPrerequisitePackageExpansionSpec, TraversalPublishSpec, WorkflowPackageTriggerRequest,
};
pub use lower::{lower_traversal_prerequisite_expansion_template, lower_workflow_region_template};
pub use output::TurnOutputPolicySpec;
pub use prepare::{
    build_initial_task_definition, build_task_initialization_payload,
    find_traversal_prerequisite_expansion, gate_map, prepare_workflow_package_context,
    prepare_workflow_task_run, prompt_map, resolve_package_target_node_id,
    validate_workflow_package_trigger, workflow_task_run_id,
};
pub use prerequisite::PrerequisiteTemplateSpec;
pub use region::{RepeatedRegionSpec, StageChainSpec, StageSpec, TurnSpec};
pub use registry::{
    load_builtin_task_package_spec, load_builtin_task_package_spec_for_workflow,
    load_task_package_spec_for_workflow,
};
pub use seed::{InitialSeedSpec, SeedArtifactSpec, SeedSourceSpec};
pub use trigger::{TargetSelectorKind, TaskTriggerSpec};
