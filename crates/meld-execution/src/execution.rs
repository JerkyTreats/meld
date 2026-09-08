//! Execution boundary contracts shared by workflow, task, and workspace adapters.

/// Provider execution binding and runtime override contracts.
pub mod contracts;
/// Adapter port traits required by execution runtimes.
pub mod ports;

pub use contracts::{ProviderExecutionBinding, ProviderRuntimeOverrides};
pub use ports::{
    BeliefStatusLabel, ContextReadPort, ContextWritePort, EventPublicationPort, ExecutionContext,
    ExecutionEffectAuthority, ExecutionEventContext, ExecutionFrame, ExecutionNodeContext,
    ExecutionNodeKind, ExecutionNodeRecord, ExecutionProgressPort, ExecutionRuntimeContext,
    GeneratedMetadataPort, NodeResolutionPort, PromptArtifactReadPort, PromptLineagePort,
    ProviderExecutionPort, ProviderPreparationView, ProviderValidationPort, SystemPromptPort,
    WorkspaceScanPort,
};
