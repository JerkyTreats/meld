//! Task contracts and initialization payloads.

pub mod contracts;
pub mod init;

pub use contracts::{
    artifact_matches_input_slot, ArtifactLinkRecord, ArtifactLinkRelation, ArtifactProducerRef,
    ArtifactRecord, ArtifactRepoRecord, CapabilityInvocationRecord, CompiledTaskRecord,
    TaskDefinition, TaskDependencyEdge, TaskDependencyKind, TaskInitSlotSpec,
};
pub use init::{
    validate_task_initialization, InitArtifactValue, TaskInitializationPayload, TaskRunContext,
};
