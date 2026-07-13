//! Task package authored contracts and lowering surfaces.

/// Top-level task package authoring contracts.
pub mod contracts;
/// Lowering from authored package specs into execution templates.
pub mod lower;
/// Output policy contracts for package-authored turns.
pub mod output;
/// Shared workflow package preparation helpers.
pub mod prepare;
/// Prerequisite authoring contracts for repeated regions.
pub mod prerequisite;
/// Repeated region and stage chain authoring contracts.
pub mod region;
/// Task package document registry and built-in package loader.
pub mod registry;
/// Initial seed artifact authoring contracts.
pub mod seed;
/// Task package trigger authoring contracts.
pub mod trigger;

pub use contracts::{
    PackageExpansionSpec, PreparedTaskRun, PreparedWorkflowPackageContext,
    TaskPackageOutputArtifactSpec, TaskPackageSpec, TraversalPrerequisitePackageExpansionSpec,
    TraversalPublishSpec, WorkflowPackageTriggerRequest,
};
pub use lower::{lower_traversal_prerequisite_expansion_template, lower_workflow_region_template};
pub use output::{
    map_task_package_output_artifact, resolve_task_package_output_mapping,
    MappedTaskPackageOutputArtifact, TaskPackageOutputMappingError, TurnOutputPolicySpec,
};
pub use prepare::{
    build_initial_task_definition, build_task_initialization_payload,
    find_traversal_prerequisite_expansion, gate_map, prepare_workflow_package_context,
    prepare_workflow_task_run, prompt_map, resolve_package_target_node_id,
    validate_workflow_package_trigger, workflow_task_run_id, PackageRunResolvers,
};
pub use prerequisite::PrerequisiteTemplateSpec;
pub use region::{RepeatedRegionSpec, StageChainSpec, StageSpec, TurnSpec};
pub use registry::{
    load_builtin_task_package_spec, load_builtin_task_package_spec_for_workflow,
    load_task_package_spec_for_workflow,
};
pub use seed::{InitialSeedSpec, SeedArtifactSpec, SeedSourceSpec};
pub use trigger::{TargetSelectorKind, TaskTriggerSpec};
