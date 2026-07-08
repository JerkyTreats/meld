//! Execution owned contracts and runtime ports.

pub mod contracts;
pub mod goal_mutation;
pub mod outcome_evidence;
pub mod ports;

pub use contracts::{ProviderExecutionBinding, ProviderRuntimeOverrides};
pub use goal_mutation::{
    satisfy_request_from_agent_mutation, GoalMutationError, GoalMutationRequest,
};
pub use outcome_evidence::{
    build_docs_task_success_evidence, DocsTaskSuccessEvidenceError, DocsTaskSuccessEvidenceRequest,
};
pub use ports::{
    BeliefContextReadPort, BeliefStatusLabel, BeliefSubjectSignal, ContextReadPort,
    ContextWritePort, EventPublicationPort, ExecutionContext, ExecutionEventContext,
    ExecutionFrame, ExecutionNodeContext, ExecutionNodeKind, ExecutionNodeRecord,
    ExecutionProgressPort, ExecutionRuntimeContext, GeneratedMetadataPort, NodeResolutionPort,
    PromptArtifactReadPort, PromptLineagePort, ProviderExecutionPort, ProviderPreparationView,
    ProviderValidationPort, SystemPromptPort, TaskRunArtifactAnchor, WorkflowProfileLoadPort,
    WorkspaceScanPort, WorldModelQueryPort,
};
