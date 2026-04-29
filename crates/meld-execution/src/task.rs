//! Task contracts and initialization payloads.

pub mod contracts;
pub mod events;
pub mod expansion;
pub mod init;

pub use contracts::{
    artifact_matches_input_slot, ArtifactLinkRecord, ArtifactLinkRelation, ArtifactProducerRef,
    ArtifactRecord, ArtifactRepoRecord, CapabilityInvocationRecord, CompiledTaskRecord,
    TaskDefinition, TaskDependencyEdge, TaskDependencyKind, TaskInitSlotSpec,
};
pub use events::{
    build_execution_task_envelope, canonical_task_event_type, ExecutionTaskEventData, TaskEvent,
};
pub use expansion::{
    parse_task_expansion_request_artifact, CompiledTaskDelta, TaskExpansionRecord,
    TaskExpansionRequest, TaskExpansionTemplate, TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID,
    TASK_EXPANSION_SCHEMA_VERSION, TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID,
};
pub use init::{
    validate_task_initialization, InitArtifactValue, TaskInitializationPayload, TaskRunContext,
};
