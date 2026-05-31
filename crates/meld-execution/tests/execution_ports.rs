use async_trait::async_trait;
use meld_execution::{
    ContextReadPort, ContextWritePort, EventPublicationPort, ExecutionContext,
    ExecutionEventContext, ExecutionFrame, ExecutionNodeContext, ExecutionNodeKind,
    ExecutionNodeRecord, ExecutionProgressPort, ExecutionRuntimeContext,
    GeneratedFrameMetadataInput, GeneratedMetadataPort, NodeResolutionPort, PreparedPromptLineage,
    PreviousMetadataSnapshotView, PromptArtifactReadPort, PromptLineagePort, PromptLineageRequest,
    PromptLinkContractView, ProviderExecutionBinding, ProviderExecutionPort,
    ProviderPreparationView, ProviderRuntimeOverrides, ProviderValidationPort, SystemPromptPort,
    TaskRunArtifactAnchor, WorkflowProfileLoadPort, WorldModelQueryPort,
};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct FakeExecutionContext;

struct FakeProviderPreparation;

impl ProviderPreparationView for FakeProviderPreparation {
    fn provider_type_slug(&self) -> &str {
        "fake"
    }

    fn model_name(&self) -> &str {
        "fake-model"
    }
}

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

    fn read_execution_frame(
        &self,
        frame_id: &Self::FrameId,
    ) -> Result<Option<ExecutionFrame<Self::FrameId>>, Self::Error> {
        Ok(Some(ExecutionFrame {
            frame_id: *frame_id,
            frame_type: "summary".to_string(),
            agent_id: "agent".to_string(),
            content: frame_id.to_be_bytes().to_vec(),
        }))
    }

    fn read_execution_node_record(
        &self,
        node_id: &Self::NodeId,
    ) -> Result<Option<ExecutionNodeRecord<Self::NodeId>>, Self::Error> {
        Ok(Some(ExecutionNodeRecord {
            node_id: *node_id,
            path: format!("node-{node_id}"),
            node_kind: ExecutionNodeKind::File,
            children: Vec::new(),
            tombstoned: false,
        }))
    }

    fn context_frames_by_type(
        &self,
        node_id: Self::NodeId,
        frame_type: &str,
        _max_frames: usize,
    ) -> Result<ExecutionNodeContext<Self::NodeId, Self::FrameId>, Self::Error> {
        Ok(ExecutionNodeContext {
            node_record: ExecutionNodeRecord {
                node_id,
                path: format!("node-{node_id}"),
                node_kind: ExecutionNodeKind::File,
                children: Vec::new(),
                tombstoned: false,
            },
            frames: vec![ExecutionFrame {
                frame_id: 1,
                frame_type: frame_type.to_string(),
                agent_id: "agent".to_string(),
                content: b"context".to_vec(),
            }],
            frame_count: 1,
        })
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
    type ProviderPreparation = FakeProviderPreparation;

    fn prepare_provider_for_request(
        &self,
        _request: &Self::GenerationRequest,
    ) -> Result<Self::ProviderPreparation, Self::Error> {
        Ok(FakeProviderPreparation)
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
    type ProviderPreparation = FakeProviderPreparation;

    async fn execute_completion(
        &self,
        request: &Self::GenerationRequest,
        _preparation: &Self::ProviderPreparation,
        messages: Vec<Self::ChatMessage>,
        _event_context: Option<&ExecutionEventContext>,
    ) -> Result<Self::CompletionResponse, Self::Error> {
        Ok(format!("prepared:{request}:{}", messages.len()))
    }
}

impl SystemPromptPort for FakeExecutionContext {
    type Error = String;

    fn load_system_prompt(&self, agent_id: &str) -> Result<String, Self::Error> {
        Ok(format!("system:{agent_id}"))
    }
}

impl PromptLineagePort for FakeExecutionContext {
    type Error = String;
    type PreparedPromptLineage = PreparedPromptLineage;
    type PromptLineageRequest = PromptLineageRequest;

    fn prepare_prompt_lineage(
        &self,
        input: &PromptLineageRequest,
        _agent_id: &str,
        _provider: &str,
        _model: &str,
        _provider_type: &str,
    ) -> Result<Self::PreparedPromptLineage, Self::Error> {
        Ok(PreparedPromptLineage {
            prompt_link_contract: PromptLinkContractView {
                prompt_link_id: "prompt-link-1".to_string(),
                prompt_digest: input.rendered_prompt.clone(),
                context_digest: input.context_payload.clone(),
                system_prompt_artifact_id: "system".to_string(),
                user_prompt_template_artifact_id: "template".to_string(),
                rendered_prompt_artifact_id: "rendered".to_string(),
                context_artifact_id: "context".to_string(),
            },
            metadata_input: GeneratedFrameMetadataInput {
                agent_id: "agent".to_string(),
                provider: "fake".to_string(),
                model: "fake-model".to_string(),
                provider_type: "fake".to_string(),
                prompt_digest: input.rendered_prompt.clone(),
                context_digest: input.context_payload.clone(),
                prompt_link_id: "prompt-link-1".to_string(),
            },
        })
    }
}

impl GeneratedMetadataPort for FakeExecutionContext {
    type Error = String;
    type FrameMetadata = String;
    type GeneratedMetadataBuilder = dyn Fn(&GeneratedFrameMetadataInput) -> String + Send + Sync;
    type GeneratedMetadataInput = GeneratedFrameMetadataInput;
    type GenerationRequest = String;
    type PreviousMetadataSnapshotView = PreviousMetadataSnapshotView;

    fn load_previous_metadata_snapshot(
        &self,
        _request: &Self::GenerationRequest,
    ) -> Result<PreviousMetadataSnapshotView, Self::Error> {
        Ok(PreviousMetadataSnapshotView {
            frame_id: None,
            prompt_digest: None,
            context_digest: None,
            prompt_link_id: None,
        })
    }

    fn build_and_validate_generated_metadata(
        &self,
        _request: &Self::GenerationRequest,
        input: &GeneratedFrameMetadataInput,
        metadata_builder: &Self::GeneratedMetadataBuilder,
    ) -> Result<Self::FrameMetadata, Self::Error> {
        Ok(metadata_builder(input))
    }
}

impl EventPublicationPort for FakeExecutionContext {
    type Error = String;
    type EventEnvelope = String;

    fn publish_execution_envelope(
        &self,
        event_context: &ExecutionEventContext,
        envelope: Self::EventEnvelope,
    ) -> Result<(), Self::Error> {
        if event_context.session_id.trim().is_empty() {
            return Err("session required".to_string());
        }
        if envelope.trim().is_empty() {
            return Err("envelope required".to_string());
        }
        Ok(())
    }
}

impl ExecutionProgressPort for FakeExecutionContext {
    type Error = String;

    fn emit_progress_event(
        &self,
        event_context: &ExecutionEventContext,
        event_type: &str,
        _payload: Value,
    ) -> Result<(), Self::Error> {
        if event_context.session_id.trim().is_empty() || event_type.trim().is_empty() {
            return Err("progress context required".to_string());
        }
        Ok(())
    }
}

impl WorldModelQueryPort for FakeExecutionContext {
    type Error = String;

    fn current_artifact_for_task_run(
        &self,
        task_run_id: &str,
        artifact_type_id: &str,
    ) -> Result<Option<TaskRunArtifactAnchor>, Self::Error> {
        Ok(Some(TaskRunArtifactAnchor {
            target_domain_id: "execution".to_string(),
            target_object_kind: artifact_type_id.to_string(),
            target_object_id: task_run_id.to_string(),
        }))
    }
}

impl WorkflowProfileLoadPort for FakeExecutionContext {
    type Error = String;
    type WorkflowProfile = String;

    fn load_workflow_profile(
        &self,
        workflow_id: &str,
    ) -> Result<Self::WorkflowProfile, Self::Error> {
        Ok(format!("profile:{workflow_id}"))
    }
}

fn assert_execution_context<T: ExecutionContext>(_context: &T) {}
fn assert_execution_runtime_context<T: ExecutionRuntimeContext>(_context: &T) {}
fn assert_world_model_query_port<T: WorldModelQueryPort>(_context: &T) {}
fn assert_workflow_profile_load_port<T: WorkflowProfileLoadPort>(_context: &T) {}

#[test]
fn blanket_execution_context_impl_accepts_port_bundle() {
    let context = FakeExecutionContext;
    assert_execution_context(&context);

    let binding =
        ProviderExecutionBinding::new("local", ProviderRuntimeOverrides::default()).unwrap();
    context.validate_provider_binding(&binding).unwrap();
}

#[test]
fn runtime_and_query_port_contracts_compile_against_port_bundle() {
    let context = FakeExecutionContext;
    let event_context = ExecutionEventContext {
        session_id: "session-1".to_string(),
    };

    assert_execution_runtime_context(&context);
    assert_world_model_query_port(&context);
    assert_workflow_profile_load_port(&context);
    context
        .publish_execution_envelope(&event_context, "envelope".to_string())
        .unwrap();
    context
        .emit_progress_event(&event_context, "execution.progress", serde_json::json!({}))
        .unwrap();
    assert_eq!(
        context
            .current_artifact_for_task_run("taskrun-1", "summary")
            .unwrap()
            .unwrap()
            .target_object_kind,
        "summary"
    );
    assert_eq!(
        context.load_workflow_profile("workflow-docs").unwrap(),
        "profile:workflow-docs"
    );
}
