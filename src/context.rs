//! Context domain: frame model, query, mutation, generation, and queue.
//! Owns context behavior; CLI, agent adapter, and workspace watch consume via explicit contracts.

pub mod capability;
pub mod facade;
pub mod frame;
pub(crate) mod frame_metadata_keys;
pub mod generation;
pub mod query;
pub mod queue;
pub mod summary;
pub mod types;

pub use facade::ContextFacade;
pub use frame::{Basis, Frame, FrameMerkleSet, FrameStorage};
pub use generation::{
    FailurePolicy, GenerationExecutor, GenerationItem, GenerationNodeType, GenerationPlan,
    GenerationResult, PlanPriority, QueueSubmitter, TargetExecutionProgram,
    TargetExecutionProgramKind, TargetExecutionRequest, TargetExecutionResult,
};
pub use queue::{
    FrameGenerationQueue, GenerationConfig, GenerationRequest, GenerationRequestOptions, Priority,
    QueueEventContext, QueueStats,
};
pub use types::{CompactResult, RestoreResult, TombstoneResult};
