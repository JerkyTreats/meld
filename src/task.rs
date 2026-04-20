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
pub(crate) mod reducer;
pub mod runtime;
pub mod templates;

pub use artifact_repo::TaskArtifactRepo;
pub use compiler::{TaskCompiler, compile_task_definition};
pub use contracts::{
    ArtifactLinkRecord, ArtifactLinkRelation, ArtifactProducerRef, ArtifactRecord,
    ArtifactRepoRecord, CapabilityInvocationRecord, CompiledTaskRecord, TaskDefinition,
    TaskDependencyEdge, TaskDependencyKind, TaskInitSlotSpec,
};
pub use events::{
    ExecutionTaskEventData, TaskEvent, build_execution_task_envelope, canonical_task_event_type,
};
pub use executor::TaskExecutor;
pub use expansion::{
    CompiledTaskDelta, TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID, TASK_EXPANSION_SCHEMA_VERSION,
    TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID, TRAVERSAL_PREREQUISITE_EXPANSION_KIND,
    TaskExpansionRecord, TaskExpansionRequest, TaskExpansionTemplate,
    WORKSPACE_WRITE_FRAME_HEAD_EXPANSION_KIND, compile_task_expansion_request,
    parse_task_expansion_request_artifact,
};
pub use init::{
    InitArtifactValue, TaskInitializationPayload, TaskRunContext, validate_task_initialization,
};
pub use invocation::assemble_invocation_payload;
pub use package::{
    InitialSeedSpec, PackageExpansionSpec, PreparedTaskRun, PreparedWorkflowPackageContext,
    SeedArtifactSpec, SeedSourceSpec, TargetSelectorKind, TaskPackageSpec, TaskTriggerSpec,
    WorkflowPackageTriggerRequest, build_initial_task_definition,
    build_task_initialization_payload, find_traversal_prerequisite_expansion, gate_map,
    load_builtin_task_package_spec, load_builtin_task_package_spec_for_workflow,
    load_task_package_spec_for_workflow, lower_traversal_prerequisite_expansion_template,
    lower_workflow_region_template, prepare_workflow_package_context, prepare_workflow_task_run,
    prompt_map, resolve_package_target_node_id, validate_workflow_package_trigger,
};
pub use readiness::compute_ready_capability_instances;
pub use runtime::{TaskRunSummary, WorkflowTaskTelemetry, execute_task_to_completion};
