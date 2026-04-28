use crate::agent::AgentIdentity;
use crate::api::ContextApi;
use crate::context::frame::Frame;
use crate::context::query::view::{ContextView, NodeContext};
use crate::context::queue::QueueEventContext;
use crate::context::CurrentFrameHeadRead;
use crate::error::ApiError;
use crate::events::EventEnvelope;
use crate::execution::contracts::ProviderExecutionBinding;
use crate::prompt_context::PromptContextArtifactStorage;
use crate::prompt_context::{PromptContextArtifactKind, PromptContextArtifactRef};
use crate::provider::executor::ProviderPreparation;
use crate::provider::{ChatMessage, CompletionResponse};
use crate::store::NodeRecord;
use crate::types::{FrameID, NodeID};
use crate::workflow::registry::RegisteredWorkflowProfile;
use crate::world_state::WorldModelQueries;
use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

pub use meld_execution::ExecutionEventContext;

pub trait ContextReadPort:
    meld_execution::ContextReadPort<
    Error = ApiError,
    AgentIdentity = AgentIdentity,
    NodeId = NodeID,
    FrameId = FrameID,
    ContextView = ContextView,
    NodeContext = NodeContext,
    Frame = Frame,
    NodeRecord = NodeRecord,
>
{
}

impl<T> ContextReadPort for T where
    T: meld_execution::ContextReadPort<
        Error = ApiError,
        AgentIdentity = AgentIdentity,
        NodeId = NodeID,
        FrameId = FrameID,
        ContextView = ContextView,
        NodeContext = NodeContext,
        Frame = Frame,
        NodeRecord = NodeRecord,
    >
{
}

pub trait ContextWritePort:
    meld_execution::ContextWritePort<
    Error = ApiError,
    NodeId = NodeID,
    FrameId = FrameID,
    Frame = Frame,
>
{
}

impl<T> ContextWritePort for T where
    T: meld_execution::ContextWritePort<
        Error = ApiError,
        NodeId = NodeID,
        FrameId = FrameID,
        Frame = Frame,
    >
{
}

pub trait PromptArtifactReadPort:
    meld_execution::PromptArtifactReadPort<
    Error = ApiError,
    ArtifactKind = PromptContextArtifactKind,
    ArtifactRef = PromptContextArtifactRef,
>
{
}

impl<T> PromptArtifactReadPort for T where
    T: meld_execution::PromptArtifactReadPort<
        Error = ApiError,
        ArtifactKind = PromptContextArtifactKind,
        ArtifactRef = PromptContextArtifactRef,
    >
{
}

pub trait NodeResolutionPort:
    meld_execution::NodeResolutionPort<Error = ApiError, NodeId = NodeID>
{
}

impl<T> NodeResolutionPort for T where
    T: meld_execution::NodeResolutionPort<Error = ApiError, NodeId = NodeID>
{
}

pub trait ProviderValidationPort:
    meld_execution::ProviderValidationPort<
    Error = ApiError,
    GenerationRequest = crate::context::generation::contracts::GenerationOrchestrationRequest,
    ProviderPreparation = ProviderPreparation,
>
{
}

impl<T> ProviderValidationPort for T where
    T: meld_execution::ProviderValidationPort<
        Error = ApiError,
        GenerationRequest = crate::context::generation::contracts::GenerationOrchestrationRequest,
        ProviderPreparation = ProviderPreparation,
    >
{
}

pub trait ProviderExecutionPort:
    meld_execution::ProviderExecutionPort<
    Error = ApiError,
    GenerationRequest = crate::context::generation::contracts::GenerationOrchestrationRequest,
    ProviderPreparation = ProviderPreparation,
    ChatMessage = ChatMessage,
    CompletionResponse = CompletionResponse,
>
{
}

impl<T> ProviderExecutionPort for T where
    T: meld_execution::ProviderExecutionPort<
        Error = ApiError,
        GenerationRequest = crate::context::generation::contracts::GenerationOrchestrationRequest,
        ProviderPreparation = ProviderPreparation,
        ChatMessage = ChatMessage,
        CompletionResponse = CompletionResponse,
    >
{
}

pub trait EventPublicationPort:
    meld_execution::EventPublicationPort<Error = ApiError, EventEnvelope = EventEnvelope>
{
}

impl<T> EventPublicationPort for T where
    T: meld_execution::EventPublicationPort<Error = ApiError, EventEnvelope = EventEnvelope>
{
}

pub trait ExecutionProgressPort: meld_execution::ExecutionProgressPort<Error = ApiError> {}

impl<T> ExecutionProgressPort for T where T: meld_execution::ExecutionProgressPort<Error = ApiError> {}

pub trait WorldModelQueryPort:
    meld_execution::WorldModelQueryPort<WorldModelQueries = WorldModelQueries>
{
}

impl<T> WorldModelQueryPort for T where
    T: meld_execution::WorldModelQueryPort<WorldModelQueries = WorldModelQueries>
{
}

pub trait WorkflowProfileLoadPort:
    meld_execution::WorkflowProfileLoadPort<
    Error = ApiError,
    WorkflowProfile = RegisteredWorkflowProfile,
>
{
}

impl<T> WorkflowProfileLoadPort for T where
    T: meld_execution::WorkflowProfileLoadPort<
        Error = ApiError,
        WorkflowProfile = RegisteredWorkflowProfile,
    >
{
}

pub trait ExecutionContext:
    ContextReadPort
    + ContextWritePort
    + PromptArtifactReadPort
    + NodeResolutionPort
    + ProviderValidationPort
    + ProviderExecutionPort
{
}

impl<T> ExecutionContext for T where
    T: ContextReadPort
        + ContextWritePort
        + PromptArtifactReadPort
        + NodeResolutionPort
        + ProviderValidationPort
        + ProviderExecutionPort
{
}

pub trait ExecutionRuntimeContext:
    ExecutionContext + EventPublicationPort + ExecutionProgressPort
{
}

impl<T> ExecutionRuntimeContext for T where
    T: ExecutionContext + EventPublicationPort + ExecutionProgressPort
{
}

impl From<&QueueEventContext> for meld_execution::ExecutionEventContext {
    fn from(value: &QueueEventContext) -> Self {
        Self {
            session_id: value.session_id.clone(),
        }
    }
}

impl meld_execution::ContextReadPort for ContextApi {
    type AgentIdentity = AgentIdentity;
    type ContextView = ContextView;
    type Error = ApiError;
    type Frame = Frame;
    type FrameId = FrameID;
    type NodeContext = NodeContext;
    type NodeId = NodeID;
    type NodeRecord = NodeRecord;

    fn get_agent(&self, agent_id: &str) -> Result<AgentIdentity, ApiError> {
        ContextApi::get_agent(self, agent_id)
    }

    fn get_head(&self, node_id: &NodeID, frame_type: &str) -> Result<Option<FrameID>, ApiError> {
        CurrentFrameHeadRead::current_frame_head(self, node_id, frame_type)
    }

    fn find_frame_head(
        &self,
        node_id: &NodeID,
        frame_type: &str,
        include_tombstoned: bool,
    ) -> Result<Option<FrameID>, ApiError> {
        if include_tombstoned {
            let head_index = self.head_index().read();
            return Ok(head_index
                .entries_for_node(node_id)
                .into_iter()
                .find(|entry| entry.frame_type == frame_type)
                .map(|entry| entry.frame_id));
        }
        self.get_head(node_id, frame_type)
    }

    fn get_node(&self, node_id: NodeID, view: ContextView) -> Result<NodeContext, ApiError> {
        ContextApi::get_node(self, node_id, view)
    }

    fn context_by_type(
        &self,
        node_id: NodeID,
        frame_type: &str,
        max_frames: usize,
    ) -> Result<NodeContext, ApiError> {
        ContextApi::context_by_type(self, node_id, frame_type, max_frames)
    }

    fn read_frame(&self, frame_id: &FrameID) -> Result<Option<Frame>, ApiError> {
        self.frame_storage().get(frame_id).map_err(ApiError::from)
    }

    fn read_node_record(&self, node_id: &NodeID) -> Result<Option<NodeRecord>, ApiError> {
        self.node_store().get(node_id).map_err(ApiError::from)
    }

    fn read_node_record_by_path(
        &self,
        path: &Path,
        include_tombstoned: bool,
    ) -> Result<Option<NodeRecord>, ApiError> {
        if include_tombstoned {
            return self.node_store().get_by_path(path).map_err(ApiError::from);
        }
        self.node_store().find_by_path(path).map_err(ApiError::from)
    }

    fn list_node_records(&self, include_tombstoned: bool) -> Result<Vec<NodeRecord>, ApiError> {
        if include_tombstoned {
            return self.node_store().list_all().map_err(ApiError::from);
        }
        self.node_store().list_active().map_err(ApiError::from)
    }

    fn workspace_root(&self) -> Option<&Path> {
        ContextApi::workspace_root(self)
    }
}

impl meld_execution::ContextWritePort for ContextApi {
    type Error = ApiError;
    type Frame = Frame;
    type FrameId = FrameID;
    type NodeId = NodeID;

    fn put_frame(
        &self,
        node_id: NodeID,
        frame: Frame,
        agent_id: String,
    ) -> Result<FrameID, ApiError> {
        ContextApi::put_frame(self, node_id, frame, agent_id)
    }

    fn tombstone_head(
        &self,
        node_id: NodeID,
        frame_type: &str,
    ) -> Result<Option<FrameID>, ApiError> {
        ContextApi::tombstone_head(self, node_id, frame_type)
    }
}

impl meld_execution::PromptArtifactReadPort for ContextApi {
    type ArtifactKind = PromptContextArtifactKind;
    type ArtifactRef = PromptContextArtifactRef;
    type Error = ApiError;

    fn read_prompt_artifact_bytes(&self, artifact_id: &str) -> Result<Vec<u8>, ApiError> {
        self.prompt_context_storage()
            .read_by_artifact_id_verified(artifact_id)
    }

    fn write_prompt_artifact_utf8(
        &self,
        kind: PromptContextArtifactKind,
        value: &str,
    ) -> Result<PromptContextArtifactRef, ApiError> {
        self.prompt_context_storage().write_utf8(kind, value)
    }
}

impl meld_execution::PromptArtifactReadPort for PromptContextArtifactStorage {
    type ArtifactKind = PromptContextArtifactKind;
    type ArtifactRef = PromptContextArtifactRef;
    type Error = ApiError;

    fn read_prompt_artifact_bytes(&self, artifact_id: &str) -> Result<Vec<u8>, ApiError> {
        self.read_by_artifact_id_verified(artifact_id)
    }

    fn write_prompt_artifact_utf8(
        &self,
        kind: PromptContextArtifactKind,
        value: &str,
    ) -> Result<PromptContextArtifactRef, ApiError> {
        self.write_utf8(kind, value)
    }
}

impl meld_execution::NodeResolutionPort for ContextApi {
    type Error = ApiError;
    type NodeId = NodeID;

    fn resolve_workspace_node_id(
        &self,
        workspace_root: &Path,
        path: Option<&Path>,
        node: Option<&str>,
        include_tombstoned: bool,
    ) -> Result<NodeID, ApiError> {
        crate::workspace::resolve_workspace_node_id(
            self,
            workspace_root,
            path,
            node,
            include_tombstoned,
        )
    }
}

impl meld_execution::ProviderValidationPort for ContextApi {
    type Error = ApiError;
    type GenerationRequest = crate::context::generation::contracts::GenerationOrchestrationRequest;
    type ProviderPreparation = ProviderPreparation;

    fn prepare_provider_for_request(
        &self,
        request: &crate::context::generation::contracts::GenerationOrchestrationRequest,
    ) -> Result<ProviderPreparation, ApiError> {
        crate::provider::executor::prepare_provider_for_request_from_api(self, request)
    }

    fn validate_provider_binding(
        &self,
        binding: &ProviderExecutionBinding,
    ) -> Result<(), ApiError> {
        let registry = self.provider_registry().read();
        let _ = registry.get_or_error(&binding.provider_name)?;
        binding.runtime_overrides.validate()?;
        Ok(())
    }
}

#[async_trait]
impl meld_execution::ProviderExecutionPort for ContextApi {
    type ChatMessage = ChatMessage;
    type CompletionResponse = CompletionResponse;
    type Error = ApiError;
    type GenerationRequest = crate::context::generation::contracts::GenerationOrchestrationRequest;
    type ProviderPreparation = ProviderPreparation;

    async fn execute_completion(
        &self,
        request: &crate::context::generation::contracts::GenerationOrchestrationRequest,
        preparation: &ProviderPreparation,
        messages: Vec<ChatMessage>,
        event_context: Option<&meld_execution::ExecutionEventContext>,
    ) -> Result<CompletionResponse, ApiError> {
        crate::provider::executor::execute_completion_from_api(
            self,
            request,
            preparation,
            messages,
            event_context,
        )
        .await
    }
}

impl meld_execution::EventPublicationPort for ContextApi {
    type Error = ApiError;
    type EventEnvelope = EventEnvelope;

    fn publish_execution_envelope(
        &self,
        _event_context: &meld_execution::ExecutionEventContext,
        envelope: EventEnvelope,
    ) -> Result<(), ApiError> {
        self.emit_envelope_best_effort(envelope);
        Ok(())
    }
}

impl meld_execution::ExecutionProgressPort for ContextApi {
    type Error = ApiError;

    fn emit_progress_event(
        &self,
        event_context: &meld_execution::ExecutionEventContext,
        event_type: &str,
        payload: Value,
    ) -> Result<(), ApiError> {
        self.emit_progress_event_best_effort(&event_context.session_id, event_type, payload);
        Ok(())
    }
}

impl meld_execution::WorkflowProfileLoadPort for ContextApi {
    type Error = ApiError;
    type WorkflowProfile = RegisteredWorkflowProfile;

    fn load_workflow_profile(
        &self,
        workflow_id: &str,
    ) -> Result<RegisteredWorkflowProfile, ApiError> {
        ContextApi::load_workflow_profile(self, workflow_id)
    }
}

impl meld_execution::WorldModelQueryPort for ContextApi {
    type WorldModelQueries = WorldModelQueries;

    fn world_model_queries(&self) -> Option<Arc<WorldModelQueries>> {
        ContextApi::world_model_queries(self)
    }
}
