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
    TaskExpansionRecord, TaskExpansionRequest, TaskExpansionTemplate, TraversalExpansionNode,
    TraversalExpansionRelation, TraversalPrerequisiteExpansionContent,
    TraversalPrerequisiteExpansionTemplate, TraversalPrerequisiteTemplate, WorkflowRegionTemplate,
    WorkflowTurnTemplate, TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID, TASK_EXPANSION_SCHEMA_VERSION,
    TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID, TRAVERSAL_PREREQUISITE_EXPANSION_KIND,
};
pub use init::{
    validate_task_initialization, InitArtifactValue, TaskInitializationPayload, TaskRunContext,
};
pub use invocation::assemble_invocation_payload;
pub use package::{PreparedTaskRun, WorkflowPackageTriggerRequest};
pub use readiness::compute_ready_capability_instances;
pub use runtime::{execute_task_to_completion, TaskRunSummary, WorkflowTaskTelemetry};
