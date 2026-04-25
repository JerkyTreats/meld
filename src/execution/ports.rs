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
use std::path::Path;
use std::sync::Arc;

pub trait ContextReadPort: Send + Sync {
    fn get_agent(&self, agent_id: &str) -> Result<AgentIdentity, ApiError>;
    fn get_head(&self, node_id: &NodeID, frame_type: &str) -> Result<Option<FrameID>, ApiError>;
    fn find_frame_head(
        &self,
        node_id: &NodeID,
        frame_type: &str,
        include_tombstoned: bool,
    ) -> Result<Option<FrameID>, ApiError>;
    fn get_node(&self, node_id: NodeID, view: ContextView) -> Result<NodeContext, ApiError>;
    fn context_by_type(
        &self,
        node_id: NodeID,
        frame_type: &str,
        max_frames: usize,
    ) -> Result<NodeContext, ApiError>;
    fn read_frame(&self, frame_id: &FrameID) -> Result<Option<Frame>, ApiError>;
    fn read_node_record(&self, node_id: &NodeID) -> Result<Option<NodeRecord>, ApiError>;
    fn read_node_record_by_path(
        &self,
        path: &Path,
        include_tombstoned: bool,
    ) -> Result<Option<NodeRecord>, ApiError>;
    fn list_node_records(&self, include_tombstoned: bool) -> Result<Vec<NodeRecord>, ApiError>;
    fn workspace_root(&self) -> Option<&Path>;
}

pub trait ContextWritePort: Send + Sync {
    fn put_frame(
        &self,
        node_id: NodeID,
        frame: Frame,
        agent_id: String,
    ) -> Result<FrameID, ApiError>;
    fn tombstone_head(
        &self,
        node_id: NodeID,
        frame_type: &str,
    ) -> Result<Option<FrameID>, ApiError>;
}

pub trait PromptArtifactReadPort: Send + Sync {
    fn read_prompt_artifact_bytes(&self, artifact_id: &str) -> Result<Vec<u8>, ApiError>;
    fn write_prompt_artifact_utf8(
        &self,
        kind: PromptContextArtifactKind,
        value: &str,
    ) -> Result<PromptContextArtifactRef, ApiError>;
}

pub trait NodeResolutionPort: Send + Sync {
    fn resolve_workspace_node_id(
        &self,
        workspace_root: &Path,
        path: Option<&Path>,
        node: Option<&str>,
        include_tombstoned: bool,
    ) -> Result<NodeID, ApiError>;
}

pub trait ProviderValidationPort: Send + Sync {
    fn prepare_provider_for_request(
        &self,
        request: &crate::context::generation::contracts::GenerationOrchestrationRequest,
    ) -> Result<ProviderPreparation, ApiError>;

    fn validate_provider_binding(&self, binding: &ProviderExecutionBinding)
        -> Result<(), ApiError>;
}

#[async_trait]
pub trait ProviderExecutionPort: Send + Sync {
    async fn execute_completion(
        &self,
        request: &crate::context::generation::contracts::GenerationOrchestrationRequest,
        preparation: &ProviderPreparation,
        messages: Vec<ChatMessage>,
        event_context: Option<&QueueEventContext>,
    ) -> Result<CompletionResponse, ApiError>;
}

pub trait EventPublicationPort: Send + Sync {
    fn publish_execution_envelope(&self, envelope: EventEnvelope) -> Result<(), ApiError>;
}

pub trait WorldModelQueryPort: Send + Sync {
    fn world_model_queries(&self) -> Option<Arc<WorldModelQueries>>;
}

pub trait WorkflowProfileLoadPort: Send + Sync {
    fn load_workflow_profile(
        &self,
        workflow_id: &str,
    ) -> Result<RegisteredWorkflowProfile, ApiError>;
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

impl ContextReadPort for ContextApi {
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

impl ContextWritePort for ContextApi {
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

impl PromptArtifactReadPort for ContextApi {
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

impl PromptArtifactReadPort for PromptContextArtifactStorage {
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

impl NodeResolutionPort for ContextApi {
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

impl ProviderValidationPort for ContextApi {
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
impl ProviderExecutionPort for ContextApi {
    async fn execute_completion(
        &self,
        request: &crate::context::generation::contracts::GenerationOrchestrationRequest,
        preparation: &ProviderPreparation,
        messages: Vec<ChatMessage>,
        event_context: Option<&QueueEventContext>,
    ) -> Result<CompletionResponse, ApiError> {
        crate::provider::executor::execute_completion_from_api(
            request,
            preparation,
            messages,
            event_context,
        )
        .await
    }
}

impl WorkflowProfileLoadPort for ContextApi {
    fn load_workflow_profile(
        &self,
        workflow_id: &str,
    ) -> Result<RegisteredWorkflowProfile, ApiError> {
        ContextApi::load_workflow_profile(self, workflow_id)
    }
}

impl WorldModelQueryPort for ContextApi {
    fn world_model_queries(&self) -> Option<Arc<WorldModelQueries>> {
        ContextApi::world_model_queries(self)
    }
}
