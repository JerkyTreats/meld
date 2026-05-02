use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

use super::contracts::ProviderExecutionBinding;
use crate::generation::{
    GeneratedFrameMetadataInput, PreviousMetadataSnapshotView, PromptLineageRequest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionEventContext {
    pub session_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionNodeKind {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionFrame<F> {
    pub frame_id: F,
    pub frame_type: String,
    pub agent_id: String,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionNodeRecord<N> {
    pub node_id: N,
    pub path: String,
    pub node_kind: ExecutionNodeKind,
    pub children: Vec<N>,
    pub tombstoned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionNodeContext<N, F> {
    pub node_record: ExecutionNodeRecord<N>,
    pub frames: Vec<ExecutionFrame<F>>,
    pub frame_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRunArtifactAnchor {
    pub target_domain_id: String,
    pub target_object_kind: String,
    pub target_object_id: String,
}

pub trait ProviderPreparationView {
    fn provider_type_slug(&self) -> &str;
    fn model_name(&self) -> &str;
}

pub trait ContextReadPort: Send + Sync {
    type Error;
    type AgentIdentity;
    type NodeId;
    type FrameId;
    type ContextView;
    type NodeContext;
    type Frame;
    type NodeRecord;

    fn get_agent(&self, agent_id: &str) -> Result<Self::AgentIdentity, Self::Error>;
    fn get_head(
        &self,
        node_id: &Self::NodeId,
        frame_type: &str,
    ) -> Result<Option<Self::FrameId>, Self::Error>;
    fn find_frame_head(
        &self,
        node_id: &Self::NodeId,
        frame_type: &str,
        include_tombstoned: bool,
    ) -> Result<Option<Self::FrameId>, Self::Error>;
    fn get_node(
        &self,
        node_id: Self::NodeId,
        view: Self::ContextView,
    ) -> Result<Self::NodeContext, Self::Error>;
    fn context_by_type(
        &self,
        node_id: Self::NodeId,
        frame_type: &str,
        max_frames: usize,
    ) -> Result<Self::NodeContext, Self::Error>;
    fn read_frame(&self, frame_id: &Self::FrameId) -> Result<Option<Self::Frame>, Self::Error>;
    fn read_node_record(
        &self,
        node_id: &Self::NodeId,
    ) -> Result<Option<Self::NodeRecord>, Self::Error>;
    fn read_node_record_by_path(
        &self,
        path: &Path,
        include_tombstoned: bool,
    ) -> Result<Option<Self::NodeRecord>, Self::Error>;
    fn list_node_records(
        &self,
        include_tombstoned: bool,
    ) -> Result<Vec<Self::NodeRecord>, Self::Error>;
    fn workspace_root(&self) -> Option<&Path>;
    fn read_execution_frame(
        &self,
        frame_id: &Self::FrameId,
    ) -> Result<Option<ExecutionFrame<Self::FrameId>>, Self::Error>;
    fn read_execution_node_record(
        &self,
        node_id: &Self::NodeId,
    ) -> Result<Option<ExecutionNodeRecord<Self::NodeId>>, Self::Error>;
    fn context_frames_by_type(
        &self,
        node_id: Self::NodeId,
        frame_type: &str,
        max_frames: usize,
    ) -> Result<ExecutionNodeContext<Self::NodeId, Self::FrameId>, Self::Error>;
}

pub trait ContextWritePort: Send + Sync {
    type Error;
    type NodeId;
    type FrameId;
    type Frame;

    fn put_frame(
        &self,
        node_id: Self::NodeId,
        frame: Self::Frame,
        agent_id: String,
    ) -> Result<Self::FrameId, Self::Error>;
    fn tombstone_head(
        &self,
        node_id: Self::NodeId,
        frame_type: &str,
    ) -> Result<Option<Self::FrameId>, Self::Error>;
}

pub trait PromptArtifactReadPort: Send + Sync {
    type Error;
    type ArtifactKind;
    type ArtifactRef;

    fn read_prompt_artifact_bytes(&self, artifact_id: &str) -> Result<Vec<u8>, Self::Error>;
    fn write_prompt_artifact_utf8(
        &self,
        kind: Self::ArtifactKind,
        value: &str,
    ) -> Result<Self::ArtifactRef, Self::Error>;
}

pub trait SystemPromptPort: Send + Sync {
    type Error;

    fn load_system_prompt(&self, agent_id: &str) -> Result<String, Self::Error>;
}

pub trait NodeResolutionPort: Send + Sync {
    type Error;
    type NodeId;

    fn resolve_workspace_node_id(
        &self,
        workspace_root: &Path,
        path: Option<&Path>,
        node: Option<&str>,
        include_tombstoned: bool,
    ) -> Result<Self::NodeId, Self::Error>;
}

pub trait ProviderValidationPort: Send + Sync {
    type Error;
    type GenerationRequest;
    type ProviderPreparation: ProviderPreparationView;

    fn prepare_provider_for_request(
        &self,
        request: &Self::GenerationRequest,
    ) -> Result<Self::ProviderPreparation, Self::Error>;

    fn validate_provider_binding(
        &self,
        binding: &ProviderExecutionBinding,
    ) -> Result<(), Self::Error>;
}

#[async_trait]
pub trait ProviderExecutionPort: Send + Sync {
    type Error;
    type GenerationRequest: Sync;
    type ProviderPreparation: Sync;
    type ChatMessage: Send;
    type CompletionResponse;

    async fn execute_completion(
        &self,
        request: &Self::GenerationRequest,
        preparation: &Self::ProviderPreparation,
        messages: Vec<Self::ChatMessage>,
        event_context: Option<&ExecutionEventContext>,
    ) -> Result<Self::CompletionResponse, Self::Error>;
}

pub trait PromptLineagePort: Send + Sync {
    type Error;
    type PromptLineageRequest;
    type PreparedPromptLineage;

    fn prepare_prompt_lineage(
        &self,
        input: &PromptLineageRequest,
        agent_id: &str,
        provider: &str,
        model: &str,
        provider_type: &str,
    ) -> Result<Self::PreparedPromptLineage, Self::Error>;
}

pub trait GeneratedMetadataPort: Send + Sync {
    type Error;
    type GenerationRequest;
    type GeneratedMetadataInput;
    type PreviousMetadataSnapshotView;
    type FrameMetadata;
    type GeneratedMetadataBuilder: ?Sized;

    fn load_previous_metadata_snapshot(
        &self,
        request: &Self::GenerationRequest,
    ) -> Result<PreviousMetadataSnapshotView, Self::Error>;

    fn build_and_validate_generated_metadata(
        &self,
        request: &Self::GenerationRequest,
        input: &GeneratedFrameMetadataInput,
        metadata_builder: &Self::GeneratedMetadataBuilder,
    ) -> Result<Self::FrameMetadata, Self::Error>;
}

pub trait EventPublicationPort: Send + Sync {
    type Error;
    type EventEnvelope;

    fn publish_execution_envelope(
        &self,
        event_context: &ExecutionEventContext,
        envelope: Self::EventEnvelope,
    ) -> Result<(), Self::Error>;
}

pub trait ExecutionProgressPort: Send + Sync {
    type Error;

    fn emit_progress_event(
        &self,
        event_context: &ExecutionEventContext,
        event_type: &str,
        payload: Value,
    ) -> Result<(), Self::Error>;
}

pub trait WorldModelQueryPort: Send + Sync {
    type Error;

    fn current_artifact_for_task_run(
        &self,
        task_run_id: &str,
        artifact_type_id: &str,
    ) -> Result<Option<TaskRunArtifactAnchor>, Self::Error>;
}

pub trait WorkflowProfileLoadPort: Send + Sync {
    type Error;
    type WorkflowProfile;

    fn load_workflow_profile(
        &self,
        workflow_id: &str,
    ) -> Result<Self::WorkflowProfile, Self::Error>;
}

pub trait ExecutionContext:
    ContextReadPort
    + ContextWritePort
    + PromptArtifactReadPort
    + SystemPromptPort
    + NodeResolutionPort
    + ProviderValidationPort
    + ProviderExecutionPort
    + PromptLineagePort
    + GeneratedMetadataPort
{
}

impl<T> ExecutionContext for T where
    T: ContextReadPort
        + ContextWritePort
        + PromptArtifactReadPort
        + SystemPromptPort
        + NodeResolutionPort
        + ProviderValidationPort
        + ProviderExecutionPort
        + PromptLineagePort
        + GeneratedMetadataPort
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
