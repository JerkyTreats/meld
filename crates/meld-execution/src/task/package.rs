//! Task package authored contracts and lowering surfaces.

pub mod contracts;
pub mod lower;
pub mod output;
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
pub use prerequisite::PrerequisiteTemplateSpec;
pub use region::{RepeatedRegionSpec, StageChainSpec, StageSpec, TurnSpec};
pub use registry::{
    load_builtin_task_package_spec, load_builtin_task_package_spec_for_workflow,
    load_task_package_spec_for_workflow,
};
pub use seed::{InitialSeedSpec, SeedArtifactSpec, SeedSourceSpec};
pub use trigger::{TargetSelectorKind, TaskTriggerSpec};
