//! Task contracts and initialization payloads.

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
    artifact_matches_input_slot, ArtifactLinkRecord, ArtifactLinkRelation, ArtifactProducerRef,
    ArtifactRecord, ArtifactRepoRecord, CapabilityInvocationRecord, CompiledTaskRecord,
    TaskDefinition, TaskDependencyEdge, TaskDependencyKind, TaskInitSlotSpec,
};
pub use events::{
    build_execution_task_envelope, canonical_task_event_type, ExecutionTaskEventData, TaskEvent,
};
pub use executor::TaskExecutor;
pub use expansion::{
    parse_task_expansion_request_artifact, CompiledTaskDelta, TaskExpansionRecord,
    TaskExpansionRequest, TaskExpansionTemplate, TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID,
    TASK_EXPANSION_SCHEMA_VERSION, TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID,
};
pub use init::{
    validate_task_initialization, InitArtifactValue, TaskInitializationPayload, TaskRunContext,
};
pub use invocation::assemble_invocation_payload;
pub use readiness::compute_ready_capability_instances;
pub use runtime::{execute_task_to_completion, TaskRunSummary, WorkflowTaskTelemetry};
pub use templates::{
    prepare_registered_workflow_task_run, workflow_task_run_id_for_target,
    workflow_uses_task_package_path,
};
