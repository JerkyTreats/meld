//! Task contracts, artifact persistence, compilation, and initialization.
//!
//! This domain owns the durable data plane above capabilities.
//! It stores compiled task structure, task-scoped artifacts, initialization inputs,
//! and invocation records without taking over domain execution internals.

pub mod artifact_repo;
pub mod compiler;
pub mod contracts;
pub mod events;
pub mod executor;
pub mod expansion;
pub mod init;
pub mod invocation;
pub mod package;
pub mod readiness;
pub mod runtime;
pub mod templates;

pub use artifact_repo::TaskArtifactRepo;
pub use compiler::{compile_task_definition, TaskCompiler};
pub use contracts::{
    ArtifactLinkRecord, ArtifactLinkRelation, ArtifactProducerRef, ArtifactRecord,
    ArtifactRepoRecord, CapabilityInvocationRecord, CompiledTaskRecord, TaskDefinition,
    TaskDependencyEdge, TaskDependencyKind, TaskInitSlotSpec,
};
pub use events::TaskEvent;
pub use executor::TaskExecutor;
pub use expansion::{
    compile_task_expansion_request, parse_task_expansion_request_artifact, CompiledTaskDelta,
    TaskExpansionRecord, TaskExpansionRequest, TaskExpansionTemplate,
    TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID, TASK_EXPANSION_SCHEMA_VERSION,
    TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID, TRAVERSAL_PREREQUISITE_EXPANSION_KIND,
    WORKSPACE_WRITE_FRAME_HEAD_EXPANSION_KIND,
};
pub use init::{
    validate_task_initialization, InitArtifactValue, TaskInitializationPayload, TaskRunContext,
};
pub use invocation::assemble_invocation_payload;
pub use package::{
    build_initial_task_definition, build_task_initialization_payload,
    find_traversal_prerequisite_expansion, gate_map, load_builtin_task_package_spec,
    load_builtin_task_package_spec_for_workflow, load_task_package_spec_for_workflow,
    lower_traversal_prerequisite_expansion_template, lower_workflow_region_template,
    prepare_workflow_package_context, prepare_workflow_task_run, prompt_map,
    resolve_package_target_node_id, validate_workflow_package_trigger, InitialSeedSpec,
    PackageExpansionSpec, PreparedTaskRun, PreparedWorkflowPackageContext, SeedArtifactSpec,
    SeedSourceSpec, TargetSelectorKind, TaskPackageSpec, TaskTriggerSpec,
    WorkflowPackageTriggerRequest,
};
pub use readiness::compute_ready_capability_instances;
pub use runtime::{execute_task_to_completion, TaskRunSummary, WorkflowTaskTelemetry};
