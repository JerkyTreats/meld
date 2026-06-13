//! Execution owned contracts and runtime ports.

pub mod contracts;
pub mod outcome_evidence;
pub mod ports;

pub use contracts::{ProviderExecutionBinding, ProviderRuntimeOverrides};
pub use outcome_evidence::{
    build_docs_task_success_evidence, DocsTaskSuccessEvidenceError, DocsTaskSuccessEvidenceRequest,
};
pub use ports::{
    ContextReadPort, ContextWritePort, EventPublicationPort, ExecutionContext,
    ExecutionEventContext, ExecutionFrame, ExecutionNodeContext, ExecutionNodeKind,
    ExecutionNodeRecord, ExecutionProgressPort, ExecutionRuntimeContext, GeneratedMetadataPort,
    NodeResolutionPort, PromptArtifactReadPort, PromptLineagePort, ProviderExecutionPort,
    ProviderPreparationView, ProviderValidationPort, SystemPromptPort, TaskRunArtifactAnchor,
    WorkflowProfileLoadPort, WorldModelQueryPort,
};
