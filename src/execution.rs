//! Execution owned contracts and runtime ports.

pub mod contracts;
pub mod goal_handoff;
pub mod ports;

pub use contracts::{ProviderExecutionBinding, ProviderRuntimeOverrides};
pub use goal_handoff::{
    accept_agent_goal_command, build_add_goal_command, AgentGoalHandoffError,
    AgentGoalHandoffRequest,
};
pub use ports::{
    ContextReadPort, ContextWritePort, EventPublicationPort, ExecutionContext,
    ExecutionEventContext, ExecutionFrame, ExecutionNodeContext, ExecutionNodeKind,
    ExecutionNodeRecord, ExecutionProgressPort, ExecutionRuntimeContext, GeneratedMetadataPort,
    NodeResolutionPort, PromptArtifactReadPort, PromptLineagePort, ProviderExecutionPort,
    ProviderPreparationView, ProviderValidationPort, SystemPromptPort, TaskRunArtifactAnchor,
    WorkflowProfileLoadPort, WorldModelQueryPort,
};
