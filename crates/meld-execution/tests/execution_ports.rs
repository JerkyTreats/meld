use async_trait::async_trait;
use meld_execution::{
    ContextReadPort, ContextWritePort, ExecutionContext, NodeResolutionPort,
    PromptArtifactReadPort, ProviderExecutionBinding, ProviderExecutionPort,
    ProviderRuntimeOverrides, ProviderValidationPort,
};
use std::path::{Path, PathBuf};

#[derive(Default)]
struct FakeExecutionContext;

impl ContextReadPort for FakeExecutionContext {
    type AgentIdentity = String;
    type ContextView = ();
    type Error = String;
    type Frame = Vec<u8>;
    type FrameId = u64;
    type NodeContext = String;
    type NodeId = u64;
    type NodeRecord = PathBuf;

    fn get_agent(&self, agent_id: &str) -> Result<Self::AgentIdentity, Self::Error> {
        Ok(agent_id.to_string())
    }

    fn get_head(
        &self,
        _node_id: &Self::NodeId,
        _frame_type: &str,
    ) -> Result<Option<Self::FrameId>, Self::Error> {
        Ok(Some(1))
    }

    fn find_frame_head(
        &self,
        node_id: &Self::NodeId,
        frame_type: &str,
        _include_tombstoned: bool,
    ) -> Result<Option<Self::FrameId>, Self::Error> {
        self.get_head(node_id, frame_type)
    }

    fn get_node(
        &self,
        node_id: Self::NodeId,
        _view: Self::ContextView,
    ) -> Result<Self::NodeContext, Self::Error> {
        Ok(format!("node-{node_id}"))
    }

    fn context_by_type(
        &self,
        node_id: Self::NodeId,
        frame_type: &str,
        _max_frames: usize,
    ) -> Result<Self::NodeContext, Self::Error> {
        Ok(format!("node-{node_id}:{frame_type}"))
    }

    fn read_frame(&self, frame_id: &Self::FrameId) -> Result<Option<Self::Frame>, Self::Error> {
        Ok(Some(frame_id.to_be_bytes().to_vec()))
    }

    fn read_node_record(
        &self,
        node_id: &Self::NodeId,
    ) -> Result<Option<Self::NodeRecord>, Self::Error> {
        Ok(Some(PathBuf::from(format!("node-{node_id}"))))
    }

    fn read_node_record_by_path(
        &self,
        path: &Path,
        _include_tombstoned: bool,
    ) -> Result<Option<Self::NodeRecord>, Self::Error> {
        Ok(Some(path.to_path_buf()))
    }

    fn list_node_records(
        &self,
        _include_tombstoned: bool,
    ) -> Result<Vec<Self::NodeRecord>, Self::Error> {
        Ok(vec![PathBuf::from("node-1")])
    }

    fn workspace_root(&self) -> Option<&Path> {
        None
    }
}

impl ContextWritePort for FakeExecutionContext {
    type Error = String;
    type Frame = Vec<u8>;
    type FrameId = u64;
    type NodeId = u64;

    fn put_frame(
        &self,
        node_id: Self::NodeId,
        _frame: Self::Frame,
        _agent_id: String,
    ) -> Result<Self::FrameId, Self::Error> {
        Ok(node_id + 1)
    }

    fn tombstone_head(
        &self,
        node_id: Self::NodeId,
        _frame_type: &str,
    ) -> Result<Option<Self::FrameId>, Self::Error> {
        Ok(Some(node_id))
    }
}

impl PromptArtifactReadPort for FakeExecutionContext {
    type ArtifactKind = String;
    type ArtifactRef = String;
    type Error = String;

    fn read_prompt_artifact_bytes(&self, artifact_id: &str) -> Result<Vec<u8>, Self::Error> {
        Ok(artifact_id.as_bytes().to_vec())
    }

    fn write_prompt_artifact_utf8(
        &self,
        kind: Self::ArtifactKind,
        value: &str,
    ) -> Result<Self::ArtifactRef, Self::Error> {
        Ok(format!("{kind}:{value}"))
    }
}

impl NodeResolutionPort for FakeExecutionContext {
    type Error = String;
    type NodeId = u64;

    fn resolve_workspace_node_id(
        &self,
        _workspace_root: &Path,
        _path: Option<&Path>,
        _node: Option<&str>,
        _include_tombstoned: bool,
    ) -> Result<Self::NodeId, Self::Error> {
        Ok(42)
    }
}

impl ProviderValidationPort for FakeExecutionContext {
    type Error = String;
    type GenerationRequest = String;
    type ProviderPreparation = String;

    fn prepare_provider_for_request(
        &self,
        request: &Self::GenerationRequest,
    ) -> Result<Self::ProviderPreparation, Self::Error> {
        Ok(format!("prepared:{request}"))
    }

    fn validate_provider_binding(
        &self,
        binding: &ProviderExecutionBinding,
    ) -> Result<(), Self::Error> {
        if binding.provider_name.trim().is_empty() {
            return Err("provider name required".to_string());
        }
        Ok(())
    }
}

#[async_trait]
impl ProviderExecutionPort for FakeExecutionContext {
    type ChatMessage = String;
    type CompletionResponse = String;
    type Error = String;
    type GenerationRequest = String;
    type ProviderPreparation = String;
    type QueueEventContext = String;

    async fn execute_completion(
        &self,
        request: &Self::GenerationRequest,
        preparation: &Self::ProviderPreparation,
        messages: Vec<Self::ChatMessage>,
        _event_context: Option<&Self::QueueEventContext>,
    ) -> Result<Self::CompletionResponse, Self::Error> {
        Ok(format!("{preparation}:{request}:{}", messages.len()))
    }
}

fn assert_execution_context<T: ExecutionContext>(_context: &T) {}

#[test]
fn blanket_execution_context_impl_accepts_port_bundle() {
    let context = FakeExecutionContext;
    assert_execution_context(&context);

    let binding =
        ProviderExecutionBinding::new("local", ProviderRuntimeOverrides::default()).unwrap();
    context.validate_provider_binding(&binding).unwrap();
}
