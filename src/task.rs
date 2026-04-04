//! Task contracts, artifact persistence, compilation, and initialization.
//!
//! This domain owns the durable data plane above capabilities.
//! It stores compiled task structure, task-scoped artifacts, initialization inputs,
//! and invocation records without taking over domain execution internals.

pub mod artifact_repo;
pub mod compiler;
pub mod contracts;
pub mod init;

pub use artifact_repo::TaskArtifactRepo;
pub use compiler::{compile_task_definition, TaskCompiler};
pub use contracts::{
    ArtifactLinkRecord, ArtifactLinkRelation, ArtifactProducerRef, ArtifactRecord,
    ArtifactRepoRecord, CapabilityInvocationRecord, CompiledTaskRecord, TaskDefinition,
    TaskDependencyEdge, TaskDependencyKind, TaskInitSlotSpec,
};
pub use init::{
    validate_task_initialization, InitArtifactValue, TaskInitializationPayload, TaskRunContext,
};
