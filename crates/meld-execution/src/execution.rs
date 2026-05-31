//! Execution boundary contracts shared by workflow, task, and workspace adapters.

pub mod contracts;
pub mod ports;

pub use contracts::{ProviderExecutionBinding, ProviderRuntimeOverrides};
pub use ports::{
    ContextReadPort, ContextWritePort, EventPublicationPort, ExecutionContext,
    ExecutionEventContext, ExecutionFrame, ExecutionNodeContext, ExecutionNodeKind,
    ExecutionNodeRecord, ExecutionProgressPort, ExecutionRuntimeContext, GeneratedMetadataPort,
    NodeResolutionPort, PromptArtifactReadPort, PromptLineagePort, ProviderExecutionPort,
    ProviderPreparationView, ProviderValidationPort, SystemPromptPort, TaskRunArtifactAnchor,
    WorkflowProfileLoadPort, WorldModelQueryPort,
};
