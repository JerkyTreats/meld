//! Workflow runtime executor for bound agent turn workflows.

mod attempt;
mod direct;
mod emission;
mod errors;
mod lifecycle;

use async_trait::async_trait;
use meld_events::EventEnvelope;
use std::collections::HashMap;
use std::fmt::Display;
use std::path::Path;

use crate::error::ExecutionInvariantError;
use crate::execution::{
    ContextReadPort, ContextWritePort, EventPublicationPort, ExecutionEventContext,
    ExecutionProgressPort, GeneratedMetadataPort, PromptArtifactReadPort, PromptLineagePort,
    ProviderExecutionPort, ProviderPreparationView, ProviderValidationPort, SystemPromptPort,
    WorldModelQueryPort,
};
use crate::generation::{
    ChatMessage, CompletionResponse, GeneratedFrameMetadataInput, GenerationOrchestrationRequest,
    NodeId, PreparedPromptLineage, PreviousMetadataSnapshotView, PromptLineageRequest,
};
use crate::workflow::{
    workflow_thread_id, RegisteredWorkflowProfile, WorkflowExecutionRequest,
    WorkflowExecutionSummary, WorkflowForceResetProgressEventData, WorkflowStateStore,
    WorkflowTargetProgressEventData, WorkflowThreadRecord, WorkflowThreadStatus,
    WorkflowTurnStatus,
};

use emission::{
    completed_target_summary, emit_workflow_force_reset_event, emit_workflow_target_event,
};

/// Builds an adapter frame from generated workflow content and metadata.
pub type FrameBuilder<'a, A, E> = dyn Fn(
        NodeId,
        Vec<u8>,
        String,
        String,
        <A as GeneratedMetadataPort>::FrameMetadata,
    ) -> Result<<A as ContextWritePort>::Frame, E>
    + Send
    + Sync
    + 'a;

/// Builds the adapter error returned when a workflow target node is missing.
pub type NodeNotFoundBuilder<'a, E> = dyn Fn(NodeId) -> E + Send + Sync + 'a;

/// Runtime dependencies that are stable across one workflow execution.
pub struct WorkflowExecutorRuntime<'a, A, E>
where
    A: ContextWritePort + GeneratedMetadataPort,
{
    /// Durable workflow state store used for resume and completion records.
    pub state_store: WorkflowStateStore,
    /// Adapter metadata builder used before frame writes.
    pub metadata_builder: &'a A::GeneratedMetadataBuilder,
    /// Adapter frame constructor used after provider completion.
    pub build_frame: &'a FrameBuilder<'a, A, E>,
    /// Adapter error constructor for missing target nodes.
    pub node_not_found: &'a NodeNotFoundBuilder<'a, E>,
    /// Task package path executor for workflows that compile into tasks.
    pub task_path_executor: &'a dyn WorkflowTaskPathExecutor<A, E>,
    /// Clock injected for deterministic record timestamps.
    pub now_millis: fn() -> u64,
}

/// Inputs supplied to task package execution for one workflow request.
pub struct WorkflowTaskPathExecution<'a, A> {
    /// Execution adapter used by the task path.
    pub api: &'a A,
    /// Workspace root used to resolve package authored target selectors.
    pub workspace_root: &'a Path,
    /// Registered workflow profile that selected the task package path.
    pub registered_profile: &'a RegisteredWorkflowProfile,
    /// Original workflow request.
    pub request: &'a WorkflowExecutionRequest,
    /// Optional event publication context.
    pub event_context: Option<&'a ExecutionEventContext>,
    /// Shared workflow state store.
    pub state_store: &'a WorkflowStateStore,
    /// Deterministic workflow thread id.
    pub thread_id: &'a str,
    /// Resolved target path for the workflow request.
    pub target_path: &'a str,
    /// Final turn sequence from the registered workflow profile.
    pub final_turn_seq: u32,
    /// Clock injected for deterministic record timestamps.
    pub now_millis: fn() -> u64,
}

/// Execution seam for workflows that route through task packages.
#[async_trait]
pub trait WorkflowTaskPathExecutor<A, E>: Send + Sync {
    /// Returns true when a profile should use the task package path.
    fn uses_task_package_path(
        &self,
        registered_profile: &RegisteredWorkflowProfile,
    ) -> Result<bool, E>;

    /// Resolves the final frame for an already completed task package thread.
    fn resolve_completed_task_path_final_frame(
        &self,
        api: &A,
        registered_profile: &RegisteredWorkflowProfile,
        request: &WorkflowExecutionRequest,
        existing: &WorkflowThreadRecord,
    ) -> Result<NodeId, E>;

    /// Executes one workflow request through the task package runtime path.
    async fn execute_task_path(
        &self,
        execution: WorkflowTaskPathExecution<'_, A>,
    ) -> Result<WorkflowExecutionSummary, E>;
}

/// Composite adapter contract required by the direct workflow executor.
pub trait WorkflowExecutorContext<E>:
    ContextReadPort<Error = E, NodeId = NodeId, FrameId = NodeId>
    + ContextWritePort<Error = E, NodeId = NodeId, FrameId = NodeId>
    + PromptArtifactReadPort<Error = E>
    + SystemPromptPort<Error = E>
    + ProviderValidationPort<Error = E, GenerationRequest = GenerationOrchestrationRequest>
    + ProviderExecutionPort<
        Error = E,
        GenerationRequest = GenerationOrchestrationRequest,
        ProviderPreparation = <Self as ProviderValidationPort>::ProviderPreparation,
        ChatMessage = ChatMessage,
        CompletionResponse = CompletionResponse,
    > + PromptLineagePort<
        Error = E,
        PromptLineageRequest = PromptLineageRequest,
        PreparedPromptLineage = PreparedPromptLineage,
    > + GeneratedMetadataPort<
        Error = E,
        GenerationRequest = GenerationOrchestrationRequest,
        GeneratedMetadataInput = GeneratedFrameMetadataInput,
        PreviousMetadataSnapshotView = PreviousMetadataSnapshotView,
    > + EventPublicationPort<Error = E, EventEnvelope = EventEnvelope>
    + ExecutionProgressPort<Error = E>
    + WorldModelQueryPort<Error = E>
{
}

impl<T, E> WorkflowExecutorContext<E> for T where
    T: ContextReadPort<Error = E, NodeId = NodeId, FrameId = NodeId>
        + ContextWritePort<Error = E, NodeId = NodeId, FrameId = NodeId>
        + PromptArtifactReadPort<Error = E>
        + SystemPromptPort<Error = E>
        + ProviderValidationPort<Error = E, GenerationRequest = GenerationOrchestrationRequest>
        + ProviderExecutionPort<
            Error = E,
            GenerationRequest = GenerationOrchestrationRequest,
            ProviderPreparation = <T as ProviderValidationPort>::ProviderPreparation,
            ChatMessage = ChatMessage,
            CompletionResponse = CompletionResponse,
        > + PromptLineagePort<
            Error = E,
            PromptLineageRequest = PromptLineageRequest,
            PreparedPromptLineage = PreparedPromptLineage,
        > + GeneratedMetadataPort<
            Error = E,
            GenerationRequest = GenerationOrchestrationRequest,
            GeneratedMetadataInput = GeneratedFrameMetadataInput,
            PreviousMetadataSnapshotView = PreviousMetadataSnapshotView,
        > + EventPublicationPort<Error = E, EventEnvelope = EventEnvelope>
        + ExecutionProgressPort<Error = E>
        + WorldModelQueryPort<Error = E>
{
}

/// Executes one registered workflow request through either direct or task path execution.
pub async fn execute_registered_workflow_async<A, E>(
    api: &A,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowExecutionRequest,
    runtime: &WorkflowExecutorRuntime<'_, A, E>,
    event_context: Option<&ExecutionEventContext>,
) -> Result<WorkflowExecutionSummary, E>
where
    A: WorkflowExecutorContext<E> + 'static,
    E: From<ExecutionInvariantError> + Display + Clone + Send + Sync + 'static,
    <A as ProviderValidationPort>::ProviderPreparation: ProviderPreparationView + Sync,
{
    execute_registered_workflow_impl(
        api,
        workspace_root,
        registered_profile,
        request,
        runtime,
        event_context,
    )
    .await
}

async fn execute_registered_workflow_impl<A, E>(
    api: &A,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowExecutionRequest,
    runtime: &WorkflowExecutorRuntime<'_, A, E>,
    event_context: Option<&ExecutionEventContext>,
) -> Result<WorkflowExecutionSummary, E>
where
    A: WorkflowExecutorContext<E> + 'static,
    E: From<ExecutionInvariantError> + Display + Clone + Send + Sync + 'static,
    <A as ProviderValidationPort>::ProviderPreparation: ProviderPreparationView + Sync,
{
    let profile = &registered_profile.profile;
    let uses_task_package_path = runtime
        .task_path_executor
        .uses_task_package_path(registered_profile)?;
    let thread_id = workflow_thread_id(profile, request.node_id, &request.frame_type);
    let state_store = &runtime.state_store;
    let node_record = api
        .read_execution_node_record(&request.node_id)?
        .ok_or_else(|| (runtime.node_not_found)(request.node_id))?;
    let target_path = request
        .path
        .clone()
        .unwrap_or_else(|| node_record.path.clone());

    let mut start_seq = 1u32;
    let mut turn_outputs: HashMap<String, String> = HashMap::new();
    let mut completed_turns = 0usize;
    let task_path_final_head = if !request.force && !uses_task_package_path {
        api.get_head(&request.node_id, &request.frame_type)?
    } else {
        None
    };

    if let Some(existing) = state_store.load_thread(&thread_id)? {
        match existing.status {
            WorkflowThreadStatus::Completed => {
                if !request.force {
                    let head = if uses_task_package_path {
                        Some(
                            runtime
                                .task_path_executor
                                .resolve_completed_task_path_final_frame(
                                    api,
                                    registered_profile,
                                    request,
                                    &existing,
                                )?,
                        )
                    } else {
                        api.get_head(&request.node_id, &request.frame_type)?
                    };
                    return Ok(completed_target_summary(
                        api,
                        event_context,
                        profile.workflow_id.clone(),
                        thread_id,
                        request,
                        target_path.clone(),
                        head,
                        0,
                        true,
                    ));
                }
            }
            WorkflowThreadStatus::Failed | WorkflowThreadStatus::Running => {
                if let Some(head) = task_path_final_head {
                    state_store.upsert_thread(&WorkflowThreadRecord {
                        thread_id: thread_id.clone(),
                        workflow_id: profile.workflow_id.clone(),
                        node_id: hex::encode(request.node_id),
                        frame_type: request.frame_type.clone(),
                        status: WorkflowThreadStatus::Completed,
                        next_turn_seq: profile.turns.len() as u32 + 1,
                        updated_at_ms: (runtime.now_millis)(),
                        final_frame_id: Some(hex::encode(head)),
                    })?;
                    return Ok(completed_target_summary(
                        api,
                        event_context,
                        profile.workflow_id.clone(),
                        thread_id,
                        request,
                        target_path.clone(),
                        Some(head),
                        0,
                        true,
                    ));
                }
                if profile.failure_policy.resume_from_failed_turn && !request.force {
                    start_seq = existing.next_turn_seq.max(1);
                    turn_outputs = state_store.completed_output_map(&thread_id)?;
                    completed_turns = state_store
                        .load_turns(&thread_id)?
                        .into_iter()
                        .filter(|turn| turn.status == WorkflowTurnStatus::Completed)
                        .count();
                }
            }
            WorkflowThreadStatus::Pending => {}
        }
    } else if let Some(head) = task_path_final_head {
        state_store.upsert_thread(&WorkflowThreadRecord {
            thread_id: thread_id.clone(),
            workflow_id: profile.workflow_id.clone(),
            node_id: hex::encode(request.node_id),
            frame_type: request.frame_type.clone(),
            status: WorkflowThreadStatus::Completed,
            next_turn_seq: profile.turns.len() as u32 + 1,
            updated_at_ms: (runtime.now_millis)(),
            final_frame_id: Some(hex::encode(head)),
        })?;
        return Ok(completed_target_summary(
            api,
            event_context,
            profile.workflow_id.clone(),
            thread_id,
            request,
            target_path.clone(),
            Some(head),
            0,
            true,
        ));
    }

    if request.force {
        let previous_head = api.tombstone_head(request.node_id, &request.frame_type)?;
        if let Some(previous_frame_id) = previous_head {
            emit_workflow_force_reset_event(
                api,
                event_context,
                "workflow_target_force_reset",
                WorkflowForceResetProgressEventData {
                    workflow_id: profile.workflow_id.clone(),
                    thread_id: thread_id.clone(),
                    node_id: hex::encode(request.node_id),
                    path: target_path.clone(),
                    agent_id: request.agent_id.clone(),
                    provider_name: request.provider.provider_name.clone(),
                    frame_type: request.frame_type.clone(),
                    previous_frame_id: Some(hex::encode(previous_frame_id)),
                    plan_id: request.plan_id.clone(),
                    level_index: request.level_index,
                },
            );
        }
    }

    state_store.upsert_thread(&WorkflowThreadRecord {
        thread_id: thread_id.clone(),
        workflow_id: profile.workflow_id.clone(),
        node_id: hex::encode(request.node_id),
        frame_type: request.frame_type.clone(),
        status: WorkflowThreadStatus::Running,
        next_turn_seq: start_seq,
        updated_at_ms: (runtime.now_millis)(),
        final_frame_id: None,
    })?;

    emit_workflow_target_event(
        api,
        event_context,
        "workflow_target_started",
        WorkflowTargetProgressEventData {
            workflow_id: profile.workflow_id.clone(),
            thread_id: thread_id.clone(),
            node_id: hex::encode(request.node_id),
            path: target_path.clone(),
            agent_id: request.agent_id.clone(),
            provider_name: request.provider.provider_name.clone(),
            frame_type: request.frame_type.clone(),
            plan_id: request.plan_id.clone(),
            level_index: request.level_index,
            final_frame_id: None,
            turns_completed: None,
            reused_existing_head: Some(false),
        },
    );

    let system_prompt = api.load_system_prompt(&request.agent_id)?;
    let mut final_frame_id: Option<NodeId> = api.get_head(&request.node_id, &request.frame_type)?;
    let ordered_turns = profile.ordered_turns();
    let final_turn_seq = ordered_turns
        .last()
        .map(|turn| turn.seq)
        .unwrap_or_default();

    if uses_task_package_path {
        return runtime
            .task_path_executor
            .execute_task_path(WorkflowTaskPathExecution {
                api,
                workspace_root,
                registered_profile,
                request,
                event_context,
                state_store,
                thread_id: &thread_id,
                target_path: &target_path,
                final_turn_seq,
                now_millis: runtime.now_millis,
            })
            .await;
    }

    let direct_result = direct::execute_direct_turns(
        direct::DirectExecutionContext {
            api,
            registered_profile,
            request,
            runtime,
            event_context,
            thread_id: &thread_id,
            target_path: &target_path,
            system_prompt,
            final_turn_seq,
        },
        direct::DirectExecutionState {
            start_seq,
            turn_outputs,
            completed_turns,
            final_frame_id,
        },
    )
    .await?;
    completed_turns = direct_result.completed_turns;
    final_frame_id = direct_result.final_frame_id;

    state_store.upsert_thread(&WorkflowThreadRecord {
        thread_id: thread_id.clone(),
        workflow_id: profile.workflow_id.clone(),
        node_id: hex::encode(request.node_id),
        frame_type: request.frame_type.clone(),
        status: WorkflowThreadStatus::Completed,
        next_turn_seq: profile.turns.len() as u32 + 1,
        updated_at_ms: (runtime.now_millis)(),
        final_frame_id: final_frame_id.map(hex::encode),
    })?;

    Ok(completed_target_summary(
        api,
        event_context,
        profile.workflow_id.clone(),
        thread_id,
        request,
        target_path,
        final_frame_id,
        completed_turns,
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::{
        ContextReadPort, ContextWritePort, ExecutionFrame, ExecutionNodeContext, ExecutionNodeKind,
        ExecutionNodeRecord, GeneratedMetadataPort, NodeResolutionPort, PromptArtifactReadPort,
        PromptLineagePort, ProviderExecutionBinding, ProviderRuntimeOverrides,
        ProviderValidationPort, SystemPromptPort, TaskRunArtifactAnchor,
    };
    use crate::generation::{
        ChatMessage, CompletionResponse, GeneratedFrameMetadataInput,
        GenerationOrchestrationRequest, NodeId, PreparedPromptLineage,
        PreviousMetadataSnapshotView, PromptLineageRequest, PromptLinkContractView, TokenUsage,
    };
    use crate::workflow::{
        WorkflowArtifactPolicy, WorkflowFailurePolicy, WorkflowGate, WorkflowProfile,
        WorkflowThreadPolicy, WorkflowTurn, WorkflowTurnRecord,
    };
    use futures::executor::block_on;
    use serde_json::{json, Value};
    use std::path::Path;
    use std::sync::Mutex;
    use tempfile::tempdir;

    #[derive(Debug, Clone)]
    struct FakeProviderPreparation;

    impl ProviderPreparationView for FakeProviderPreparation {
        fn provider_type_slug(&self) -> &str {
            "fake"
        }

        fn model_name(&self) -> &str {
            "fake-model"
        }
    }

    type PutFrameRecord = (NodeId, String, String, Vec<u8>);

    #[derive(Default)]
    struct FakeApi {
        head: Mutex<Option<NodeId>>,
        completions: Mutex<Vec<Result<String, ExecutionInvariantError>>>,
        completion_requests: Mutex<Vec<GenerationOrchestrationRequest>>,
        events: Mutex<Vec<EventEnvelope>>,
        progress: Mutex<Vec<(String, Value)>>,
        put_frames: Mutex<Vec<PutFrameRecord>>,
        metadata_failures: Mutex<Vec<ExecutionInvariantError>>,
    }

    impl FakeApi {
        fn with_completions(completions: Vec<Result<String, ExecutionInvariantError>>) -> Self {
            Self {
                completions: Mutex::new(completions),
                ..Self::default()
            }
        }

        fn set_head(&self, head: Option<NodeId>) {
            *self.head.lock().unwrap() = head;
        }

        fn fail_metadata_with(&self, errors: Vec<ExecutionInvariantError>) {
            *self.metadata_failures.lock().unwrap() = errors;
        }

        fn progress_events(&self) -> Vec<String> {
            self.progress
                .lock()
                .unwrap()
                .iter()
                .map(|(event_type, _)| event_type.clone())
                .collect()
        }

        fn workflow_events(&self) -> Vec<String> {
            self.events
                .lock()
                .unwrap()
                .iter()
                .map(|envelope| envelope.event_type.clone())
                .collect()
        }
    }

    impl ContextReadPort for FakeApi {
        type AgentIdentity = String;
        type ContextView = ();
        type Error = ExecutionInvariantError;
        type Frame = Vec<u8>;
        type FrameId = NodeId;
        type NodeContext = String;
        type NodeId = NodeId;
        type NodeRecord = ExecutionNodeRecord<NodeId>;

        fn get_agent(&self, agent_id: &str) -> Result<Self::AgentIdentity, Self::Error> {
            Ok(agent_id.to_string())
        }

        fn get_head(
            &self,
            _node_id: &Self::NodeId,
            _frame_type: &str,
        ) -> Result<Option<Self::FrameId>, Self::Error> {
            Ok(*self.head.lock().unwrap())
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
            Ok(format!("node:{}", hex::encode(node_id)))
        }

        fn context_by_type(
            &self,
            node_id: Self::NodeId,
            frame_type: &str,
            _max_frames: usize,
        ) -> Result<Self::NodeContext, Self::Error> {
            Ok(format!("node:{}:{frame_type}", hex::encode(node_id)))
        }

        fn read_frame(&self, frame_id: &Self::FrameId) -> Result<Option<Self::Frame>, Self::Error> {
            Ok(Some(frame_id.to_vec()))
        }

        fn read_node_record(
            &self,
            node_id: &Self::NodeId,
        ) -> Result<Option<Self::NodeRecord>, Self::Error> {
            Ok(Some(ExecutionNodeRecord {
                node_id: *node_id,
                path: "README.md".to_string(),
                node_kind: ExecutionNodeKind::File,
                children: Vec::new(),
                tombstoned: false,
            }))
        }

        fn read_node_record_by_path(
            &self,
            path: &Path,
            _include_tombstoned: bool,
        ) -> Result<Option<Self::NodeRecord>, Self::Error> {
            Ok(Some(ExecutionNodeRecord {
                node_id: node_id(),
                path: path.display().to_string(),
                node_kind: ExecutionNodeKind::File,
                children: Vec::new(),
                tombstoned: false,
            }))
        }

        fn list_node_records(
            &self,
            _include_tombstoned: bool,
        ) -> Result<Vec<Self::NodeRecord>, Self::Error> {
            Ok(Vec::new())
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
                content: frame_id.to_vec(),
            }))
        }

        fn read_execution_node_record(
            &self,
            node_id: &Self::NodeId,
        ) -> Result<Option<ExecutionNodeRecord<Self::NodeId>>, Self::Error> {
            self.read_node_record(node_id)
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
                    path: "README.md".to_string(),
                    node_kind: ExecutionNodeKind::File,
                    children: Vec::new(),
                    tombstoned: false,
                },
                frames: vec![ExecutionFrame {
                    frame_id: [7; 32],
                    frame_type: frame_type.to_string(),
                    agent_id: "agent".to_string(),
                    content: b"target context".to_vec(),
                }],
                frame_count: 1,
            })
        }
    }

    impl ContextWritePort for FakeApi {
        type Error = ExecutionInvariantError;
        type Frame = Vec<u8>;
        type FrameId = NodeId;
        type NodeId = NodeId;

        fn put_frame(
            &self,
            node_id: Self::NodeId,
            frame: Self::Frame,
            agent_id: String,
        ) -> Result<Self::FrameId, Self::Error> {
            let frame_id = [self.put_frames.lock().unwrap().len() as u8 + 20; 32];
            let frame_type = String::from_utf8_lossy(&frame).to_string();
            self.put_frames
                .lock()
                .unwrap()
                .push((node_id, frame_type, agent_id, frame));
            *self.head.lock().unwrap() = Some(frame_id);
            Ok(frame_id)
        }

        fn tombstone_head(
            &self,
            _node_id: Self::NodeId,
            _frame_type: &str,
        ) -> Result<Option<Self::FrameId>, Self::Error> {
            Ok(self.head.lock().unwrap().take())
        }
    }

    impl PromptArtifactReadPort for FakeApi {
        type ArtifactKind = String;
        type ArtifactRef = String;
        type Error = ExecutionInvariantError;

        fn read_prompt_artifact_bytes(&self, artifact_id: &str) -> Result<Vec<u8>, Self::Error> {
            Ok(format!("prompt:{artifact_id}").into_bytes())
        }

        fn write_prompt_artifact_utf8(
            &self,
            kind: Self::ArtifactKind,
            value: &str,
        ) -> Result<Self::ArtifactRef, Self::Error> {
            Ok(format!("{kind}:{value}"))
        }
    }

    impl SystemPromptPort for FakeApi {
        type Error = ExecutionInvariantError;

        fn load_system_prompt(&self, agent_id: &str) -> Result<String, Self::Error> {
            Ok(format!("system:{agent_id}"))
        }
    }

    impl NodeResolutionPort for FakeApi {
        type Error = ExecutionInvariantError;
        type NodeId = NodeId;

        fn resolve_workspace_node_id(
            &self,
            _workspace_root: &Path,
            _path: Option<&Path>,
            _node: Option<&str>,
            _include_tombstoned: bool,
        ) -> Result<Self::NodeId, Self::Error> {
            Ok(node_id())
        }
    }

    impl ProviderValidationPort for FakeApi {
        type Error = ExecutionInvariantError;
        type GenerationRequest = GenerationOrchestrationRequest;
        type ProviderPreparation = FakeProviderPreparation;

        fn prepare_provider_for_request(
            &self,
            _request: &Self::GenerationRequest,
        ) -> Result<Self::ProviderPreparation, Self::Error> {
            Ok(FakeProviderPreparation)
        }

        fn validate_provider_binding(
            &self,
            _binding: &ProviderExecutionBinding,
        ) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[async_trait]
    impl ProviderExecutionPort for FakeApi {
        type ChatMessage = ChatMessage;
        type CompletionResponse = CompletionResponse;
        type Error = ExecutionInvariantError;
        type GenerationRequest = GenerationOrchestrationRequest;
        type ProviderPreparation = FakeProviderPreparation;

        async fn execute_completion(
            &self,
            request: &Self::GenerationRequest,
            _preparation: &Self::ProviderPreparation,
            _messages: Vec<Self::ChatMessage>,
            _event_context: Option<&ExecutionEventContext>,
        ) -> Result<Self::CompletionResponse, Self::Error> {
            self.completion_requests
                .lock()
                .unwrap()
                .push(request.clone());
            let content = self.completions.lock().unwrap().remove(0)?;
            Ok(CompletionResponse {
                content,
                model: "fake-model".to_string(),
                usage: TokenUsage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                },
                finish_reason: Some("stop".to_string()),
            })
        }
    }

    impl PromptLineagePort for FakeApi {
        type Error = ExecutionInvariantError;
        type PreparedPromptLineage = PreparedPromptLineage;
        type PromptLineageRequest = PromptLineageRequest;

        fn prepare_prompt_lineage(
            &self,
            input: &PromptLineageRequest,
            _agent_id: &str,
            provider: &str,
            model: &str,
            provider_type: &str,
        ) -> Result<Self::PreparedPromptLineage, Self::Error> {
            let index = self.completion_requests.lock().unwrap().len() + 1;
            Ok(PreparedPromptLineage {
                prompt_link_contract: PromptLinkContractView {
                    prompt_link_id: format!("prompt-link-{index}"),
                    prompt_digest: input.rendered_prompt.clone(),
                    context_digest: input.context_payload.clone(),
                    system_prompt_artifact_id: hex64('a'),
                    user_prompt_template_artifact_id: hex64('b'),
                    rendered_prompt_artifact_id: hex64('c'),
                    context_artifact_id: hex64('d'),
                },
                metadata_input: GeneratedFrameMetadataInput {
                    agent_id: "agent".to_string(),
                    provider: provider.to_string(),
                    model: model.to_string(),
                    provider_type: provider_type.to_string(),
                    prompt_digest: input.rendered_prompt.clone(),
                    context_digest: input.context_payload.clone(),
                    prompt_link_id: format!("prompt-link-{index}"),
                },
            })
        }
    }

    impl GeneratedMetadataPort for FakeApi {
        type Error = ExecutionInvariantError;
        type FrameMetadata = String;
        type GeneratedMetadataBuilder =
            dyn Fn(&GeneratedFrameMetadataInput) -> String + Send + Sync;
        type GeneratedMetadataInput = GeneratedFrameMetadataInput;
        type GenerationRequest = GenerationOrchestrationRequest;
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
            if !self.metadata_failures.lock().unwrap().is_empty() {
                return Err(self.metadata_failures.lock().unwrap().remove(0));
            }
            Ok(metadata_builder(input))
        }
    }

    impl EventPublicationPort for FakeApi {
        type Error = ExecutionInvariantError;
        type EventEnvelope = EventEnvelope;

        fn publish_execution_envelope(
            &self,
            _event_context: &ExecutionEventContext,
            envelope: Self::EventEnvelope,
        ) -> Result<(), Self::Error> {
            self.events.lock().unwrap().push(envelope);
            Ok(())
        }
    }

    impl ExecutionProgressPort for FakeApi {
        type Error = ExecutionInvariantError;

        fn emit_progress_event(
            &self,
            _event_context: &ExecutionEventContext,
            event_type: &str,
            payload: Value,
        ) -> Result<(), Self::Error> {
            self.progress
                .lock()
                .unwrap()
                .push((event_type.to_string(), payload));
            Ok(())
        }
    }

    impl WorldModelQueryPort for FakeApi {
        type Error = ExecutionInvariantError;

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

    #[derive(Default)]
    struct FakeTaskPathExecutor {
        uses_task_package_path: bool,
        resolved_final_frame: NodeId,
    }

    #[async_trait]
    impl WorkflowTaskPathExecutor<FakeApi, ExecutionInvariantError> for FakeTaskPathExecutor {
        fn uses_task_package_path(
            &self,
            _registered_profile: &RegisteredWorkflowProfile,
        ) -> Result<bool, ExecutionInvariantError> {
            Ok(self.uses_task_package_path)
        }

        fn resolve_completed_task_path_final_frame(
            &self,
            _api: &FakeApi,
            _registered_profile: &RegisteredWorkflowProfile,
            _request: &WorkflowExecutionRequest,
            _existing: &WorkflowThreadRecord,
        ) -> Result<NodeId, ExecutionInvariantError> {
            Ok(self.resolved_final_frame)
        }

        async fn execute_task_path(
            &self,
            execution: WorkflowTaskPathExecution<'_, FakeApi>,
        ) -> Result<WorkflowExecutionSummary, ExecutionInvariantError> {
            Ok(WorkflowExecutionSummary {
                workflow_id: execution.registered_profile.profile.workflow_id.clone(),
                thread_id: execution.thread_id.to_string(),
                turns_completed: 1,
                final_frame_id: Some(self.resolved_final_frame),
            })
        }
    }

    fn hex64(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn node_id() -> NodeId {
        [1; 32]
    }

    fn provider() -> ProviderExecutionBinding {
        ProviderExecutionBinding::new("fake", ProviderRuntimeOverrides::default()).unwrap()
    }

    fn request(force: bool) -> WorkflowExecutionRequest {
        WorkflowExecutionRequest {
            node_id: node_id(),
            agent_id: "agent".to_string(),
            provider: provider(),
            frame_type: "summary".to_string(),
            force,
            path: Some("README.md".to_string()),
            plan_id: Some("plan-1".to_string()),
            level_index: Some(0),
        }
    }

    fn turn(turn_id: &str, seq: u32, input_refs: Vec<&str>) -> WorkflowTurn {
        WorkflowTurn {
            turn_id: turn_id.to_string(),
            seq,
            title: format!("Turn {seq}"),
            prompt_ref: format!("artifact:prompt_{seq}"),
            input_refs: input_refs.into_iter().map(ToString::to_string).collect(),
            output_type: format!("output_{seq}"),
            gate_id: "gate-1".to_string(),
            retry_limit: 2,
            timeout_ms: 1000,
        }
    }

    fn profile(required_fields: Vec<&str>, stop_on_gate_fail: bool) -> WorkflowProfile {
        WorkflowProfile {
            workflow_id: "workflow_docs".to_string(),
            version: 1,
            title: "Docs".to_string(),
            description: "Docs workflow".to_string(),
            thread_policy: WorkflowThreadPolicy {
                start_conditions: Value::Null,
                dedupe_key_fields: vec!["workflow_id".to_string()],
                max_turn_retries: 2,
            },
            turns: vec![
                turn("turn-1", 1, vec!["target_context"]),
                turn("turn-2", 2, vec!["output_1"]),
            ],
            gates: vec![WorkflowGate {
                gate_id: "gate-1".to_string(),
                gate_type: "schema_required_fields".to_string(),
                required_fields: required_fields
                    .into_iter()
                    .map(ToString::to_string)
                    .collect(),
                rules: json!({}),
                fail_on_violation: stop_on_gate_fail,
            }],
            artifact_policy: WorkflowArtifactPolicy {
                store_output: true,
                store_prompt_render: true,
                store_context_payload: true,
                max_output_bytes: 1024,
            },
            failure_policy: WorkflowFailurePolicy {
                mode: "fail_fast".to_string(),
                resume_from_failed_turn: true,
                stop_on_gate_fail,
            },
            thread_profile: None,
            target_agent_id: None,
            target_frame_type: None,
            final_artifact_type: None,
        }
    }

    fn registered_profile(
        required_fields: Vec<&str>,
        stop_on_gate_fail: bool,
    ) -> RegisteredWorkflowProfile {
        RegisteredWorkflowProfile {
            profile: profile(required_fields, stop_on_gate_fail),
            source_path: None,
        }
    }

    fn one_turn_registered_profile(
        required_fields: Vec<&str>,
        stop_on_gate_fail: bool,
    ) -> RegisteredWorkflowProfile {
        let mut registered_profile = registered_profile(required_fields, stop_on_gate_fail);
        registered_profile.profile.turns.truncate(1);
        registered_profile
    }

    fn runtime<'a>(
        state_store: WorkflowStateStore,
        task_path_executor: &'a FakeTaskPathExecutor,
        metadata_builder: &'a <FakeApi as GeneratedMetadataPort>::GeneratedMetadataBuilder,
        build_frame: &'a FrameBuilder<'a, FakeApi, ExecutionInvariantError>,
        node_not_found: &'a NodeNotFoundBuilder<'a, ExecutionInvariantError>,
    ) -> WorkflowExecutorRuntime<'a, FakeApi, ExecutionInvariantError> {
        WorkflowExecutorRuntime {
            state_store,
            metadata_builder,
            build_frame,
            node_not_found,
            task_path_executor,
            now_millis: || 1,
        }
    }

    fn run_workflow(
        api: &FakeApi,
        registered_profile: &RegisteredWorkflowProfile,
        request: &WorkflowExecutionRequest,
        state_store: WorkflowStateStore,
        task_path_executor: &FakeTaskPathExecutor,
    ) -> Result<WorkflowExecutionSummary, ExecutionInvariantError> {
        let metadata_builder = |input: &GeneratedFrameMetadataInput| input.prompt_link_id.clone();
        let build_frame = |_node_id: NodeId,
                           content: Vec<u8>,
                           frame_type: String,
                           _agent_id: String,
                           _metadata: String|
         -> Result<Vec<u8>, ExecutionInvariantError> {
            let mut frame = frame_type.into_bytes();
            frame.push(b'\n');
            frame.extend(content);
            Ok(frame)
        };
        let node_not_found = |node_id: NodeId| {
            ExecutionInvariantError::ConfigError(format!("missing node '{}'", hex::encode(node_id)))
        };
        let runtime = runtime(
            state_store,
            task_path_executor,
            &metadata_builder,
            &build_frame,
            &node_not_found,
        );
        let event_context = ExecutionEventContext {
            session_id: "session-1".to_string(),
        };
        block_on(execute_registered_workflow_async(
            api,
            Path::new("."),
            registered_profile,
            request,
            &runtime,
            Some(&event_context),
        ))
    }

    #[test]
    fn executor_runs_direct_workflow_and_records_completion() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::with_completions(vec![
            Ok(r#"{"ok":true}"#.to_string()),
            Ok(r#"{"ok":true}"#.to_string()),
        ]);
        let registered_profile = registered_profile(vec!["ok"], true);
        let task_path_executor = FakeTaskPathExecutor::default();

        let summary = run_workflow(
            &api,
            &registered_profile,
            &request(false),
            state_store.clone(),
            &task_path_executor,
        )
        .unwrap();

        assert_eq!(summary.turns_completed, 2);
        assert_eq!(
            state_store
                .load_thread(&summary.thread_id)
                .unwrap()
                .unwrap()
                .next_turn_seq,
            3
        );
        assert_eq!(
            state_store
                .load_thread(&summary.thread_id)
                .unwrap()
                .unwrap()
                .status,
            WorkflowThreadStatus::Completed
        );
        assert_eq!(state_store.load_turns(&summary.thread_id).unwrap().len(), 2);
        assert_eq!(
            api.workflow_events(),
            vec![
                "execution.workflow.turn_started",
                "execution.workflow.turn_completed",
                "execution.workflow.turn_started",
                "execution.workflow.turn_completed",
            ]
        );
        let frames = api.put_frames.lock().unwrap();
        assert!(frames[0].1.starts_with("summary--workflow-turn-1-"));
        assert!(frames[1].1.starts_with("summary\n"));
        assert!(api
            .progress_events()
            .iter()
            .any(|event_type| event_type == "frame_metadata_validation_started"));
        assert!(api
            .progress_events()
            .iter()
            .any(|event_type| event_type == "frame_metadata_validation_succeeded"));
    }

    #[test]
    fn executor_reuses_completed_thread_without_force() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::default();
        api.set_head(Some([9; 32]));
        let registered_profile = registered_profile(vec!["ok"], true);
        let thread_id = workflow_thread_id(&registered_profile.profile, node_id(), "summary");
        state_store
            .upsert_thread(&WorkflowThreadRecord {
                thread_id: thread_id.clone(),
                workflow_id: "workflow_docs".to_string(),
                node_id: hex::encode(node_id()),
                frame_type: "summary".to_string(),
                status: WorkflowThreadStatus::Completed,
                next_turn_seq: 3,
                updated_at_ms: 1,
                final_frame_id: Some(hex::encode([9; 32])),
            })
            .unwrap();

        let summary = run_workflow(
            &api,
            &registered_profile,
            &request(false),
            state_store,
            &FakeTaskPathExecutor::default(),
        )
        .unwrap();

        assert_eq!(summary.turns_completed, 0);
        assert_eq!(summary.final_frame_id, Some([9; 32]));
        assert_eq!(api.progress_events(), vec!["workflow_target_completed"]);
        assert!(api.completion_requests.lock().unwrap().is_empty());
    }

    #[test]
    fn executor_force_reset_ignores_reusable_head_and_runs_turns() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::with_completions(vec![
            Ok(r#"{"ok":true}"#.to_string()),
            Ok(r#"{"ok":true}"#.to_string()),
        ]);
        api.set_head(Some([9; 32]));
        let registered_profile = registered_profile(vec!["ok"], true);

        let summary = run_workflow(
            &api,
            &registered_profile,
            &request(true),
            state_store,
            &FakeTaskPathExecutor::default(),
        )
        .unwrap();

        assert_eq!(summary.turns_completed, 2);
        assert!(api
            .progress_events()
            .iter()
            .any(|event_type| event_type == "workflow_target_force_reset"));
    }

    #[test]
    fn executor_reuses_existing_head_when_no_thread_record_exists() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::default();
        api.set_head(Some([9; 32]));
        let registered_profile = registered_profile(vec!["ok"], true);

        let summary = run_workflow(
            &api,
            &registered_profile,
            &request(false),
            state_store.clone(),
            &FakeTaskPathExecutor::default(),
        )
        .unwrap();
        let thread = state_store
            .load_thread(&summary.thread_id)
            .unwrap()
            .unwrap();

        assert_eq!(summary.turns_completed, 0);
        assert_eq!(thread.status, WorkflowThreadStatus::Completed);
        assert_eq!(thread.next_turn_seq, 3);
        assert_eq!(summary.final_frame_id, Some([9; 32]));
    }

    #[test]
    fn executor_marks_failed_thread_completed_when_existing_head_is_reusable() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::default();
        api.set_head(Some([9; 32]));
        let registered_profile = registered_profile(vec!["ok"], true);
        let thread_id = workflow_thread_id(&registered_profile.profile, node_id(), "summary");
        state_store
            .upsert_thread(&WorkflowThreadRecord {
                thread_id: thread_id.clone(),
                workflow_id: "workflow_docs".to_string(),
                node_id: hex::encode(node_id()),
                frame_type: "summary".to_string(),
                status: WorkflowThreadStatus::Failed,
                next_turn_seq: 2,
                updated_at_ms: 1,
                final_frame_id: None,
            })
            .unwrap();

        let summary = run_workflow(
            &api,
            &registered_profile,
            &request(false),
            state_store.clone(),
            &FakeTaskPathExecutor::default(),
        )
        .unwrap();
        let thread = state_store
            .load_thread(&summary.thread_id)
            .unwrap()
            .unwrap();

        assert_eq!(summary.turns_completed, 0);
        assert_eq!(thread.status, WorkflowThreadStatus::Completed);
        assert_eq!(thread.next_turn_seq, 3);
        assert_eq!(summary.final_frame_id, Some([9; 32]));
    }

    #[test]
    fn executor_resumes_failed_thread_from_next_turn() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::with_completions(vec![Ok(r#"{"ok":true}"#.to_string())]);
        let registered_profile = registered_profile(vec!["ok"], true);
        let thread_id = workflow_thread_id(&registered_profile.profile, node_id(), "summary");
        state_store
            .upsert_thread(&WorkflowThreadRecord {
                thread_id: thread_id.clone(),
                workflow_id: "workflow_docs".to_string(),
                node_id: hex::encode(node_id()),
                frame_type: "summary".to_string(),
                status: WorkflowThreadStatus::Failed,
                next_turn_seq: 2,
                updated_at_ms: 1,
                final_frame_id: Some(hex::encode([8; 32])),
            })
            .unwrap();
        state_store
            .upsert_turn(&WorkflowTurnRecord {
                thread_id: thread_id.clone(),
                turn_id: "turn-1".to_string(),
                seq: 1,
                output_type: "output_1".to_string(),
                status: WorkflowTurnStatus::Completed,
                attempt_count: 1,
                frame_id: Some(hex::encode([8; 32])),
                output_text: Some(r#"{"ok":true}"#.to_string()),
                updated_at_ms: 1,
            })
            .unwrap();

        let summary = run_workflow(
            &api,
            &registered_profile,
            &request(false),
            state_store,
            &FakeTaskPathExecutor::default(),
        )
        .unwrap();

        assert_eq!(summary.turns_completed, 2);
        assert_eq!(api.completion_requests.lock().unwrap().len(), 1);
        assert_eq!(api.completion_requests.lock().unwrap()[0].request_id, 2001);
    }

    #[test]
    fn executor_force_request_does_not_resume_failed_thread_state() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::with_completions(vec![
            Ok(r#"{"ok":true}"#.to_string()),
            Ok(r#"{"ok":true}"#.to_string()),
        ]);
        let registered_profile = registered_profile(vec!["ok"], true);
        let thread_id = workflow_thread_id(&registered_profile.profile, node_id(), "summary");
        state_store
            .upsert_thread(&WorkflowThreadRecord {
                thread_id,
                workflow_id: "workflow_docs".to_string(),
                node_id: hex::encode(node_id()),
                frame_type: "summary".to_string(),
                status: WorkflowThreadStatus::Failed,
                next_turn_seq: 2,
                updated_at_ms: 1,
                final_frame_id: None,
            })
            .unwrap();

        let summary = run_workflow(
            &api,
            &registered_profile,
            &request(true),
            state_store,
            &FakeTaskPathExecutor::default(),
        )
        .unwrap();
        let requests = api.completion_requests.lock().unwrap().clone();

        assert_eq!(summary.turns_completed, 2);
        assert_eq!(requests[0].request_id, 1001);
        assert_eq!(requests[1].request_id, 2001);
    }

    #[test]
    fn executor_retries_provider_failure_and_records_retry_count() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::with_completions(vec![
            Err(ExecutionInvariantError::GenerationFailed(
                "provider failed".to_string(),
            )),
            Ok(r#"{"ok":true}"#.to_string()),
            Ok(r#"{"ok":true}"#.to_string()),
        ]);
        let registered_profile = registered_profile(vec!["ok"], true);

        let summary = run_workflow(
            &api,
            &registered_profile,
            &request(false),
            state_store,
            &FakeTaskPathExecutor::default(),
        )
        .unwrap();
        let requests = api.completion_requests.lock().unwrap().clone();

        assert_eq!(summary.turns_completed, 2);
        assert_eq!(requests[0].retry_count, 0);
        assert_eq!(requests[1].retry_count, 1);
        assert!(api
            .workflow_events()
            .iter()
            .any(|event_type| event_type == "execution.workflow.turn_failed"));
    }

    #[test]
    fn executor_exhausts_provider_retries_and_marks_thread_failed() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::with_completions(vec![
            Err(ExecutionInvariantError::GenerationFailed(
                "provider failed once".to_string(),
            )),
            Err(ExecutionInvariantError::GenerationFailed(
                "provider failed twice".to_string(),
            )),
            Err(ExecutionInvariantError::GenerationFailed(
                "provider failed extra".to_string(),
            )),
        ]);
        let registered_profile = one_turn_registered_profile(vec!["ok"], true);

        let error = run_workflow(
            &api,
            &registered_profile,
            &request(false),
            state_store.clone(),
            &FakeTaskPathExecutor::default(),
        )
        .unwrap_err();
        let thread_id = workflow_thread_id(&registered_profile.profile, node_id(), "summary");
        let requests = api.completion_requests.lock().unwrap().clone();

        assert!(error.to_string().contains("provider failed twice"));
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].retry_count, 0);
        assert_eq!(requests[1].retry_count, 1);
        assert_eq!(
            state_store.load_thread(&thread_id).unwrap().unwrap().status,
            WorkflowThreadStatus::Failed
        );
    }

    #[test]
    fn executor_zero_retry_limit_does_not_attempt_turn() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::with_completions(vec![Ok(r#"{"ok":true}"#.to_string())]);
        let mut registered_profile = one_turn_registered_profile(vec!["ok"], true);
        registered_profile.profile.turns[0].retry_limit = 0;

        let error = run_workflow(
            &api,
            &registered_profile,
            &request(false),
            state_store,
            &FakeTaskPathExecutor::default(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("failed with no retryable error"));
        assert!(api.completion_requests.lock().unwrap().is_empty());
    }

    #[test]
    fn executor_retries_metadata_failure_without_calling_provider() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::with_completions(vec![Ok(r#"{"ok":true}"#.to_string())]);
        api.fail_metadata_with(vec![
            ExecutionInvariantError::GenerationFailed("metadata failed once".to_string()),
            ExecutionInvariantError::GenerationFailed("metadata failed twice".to_string()),
        ]);
        let registered_profile = one_turn_registered_profile(vec!["ok"], true);

        let error = run_workflow(
            &api,
            &registered_profile,
            &request(false),
            state_store.clone(),
            &FakeTaskPathExecutor::default(),
        )
        .unwrap_err();
        let thread_id = workflow_thread_id(&registered_profile.profile, node_id(), "summary");

        assert!(error.to_string().contains("metadata failed twice"));
        assert!(api.completion_requests.lock().unwrap().is_empty());
        assert_eq!(
            state_store.load_thread(&thread_id).unwrap().unwrap().status,
            WorkflowThreadStatus::Failed
        );
        assert_eq!(
            api.progress_events()
                .into_iter()
                .filter(|event_type| event_type == "frame_metadata_validation_failed")
                .count(),
            2
        );
    }

    #[test]
    fn executor_preserves_running_next_turn_seq_when_later_turn_cannot_resolve_inputs() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::with_completions(vec![Ok(r#"{"ok":true}"#.to_string())]);
        let mut registered_profile = registered_profile(vec!["ok"], true);
        registered_profile.profile.turns[1].input_refs = vec!["missing".to_string()];

        let error = run_workflow(
            &api,
            &registered_profile,
            &request(false),
            state_store.clone(),
            &FakeTaskPathExecutor::default(),
        )
        .unwrap_err();
        let thread_id = workflow_thread_id(&registered_profile.profile, node_id(), "summary");
        let thread = state_store.load_thread(&thread_id).unwrap().unwrap();

        assert!(error.to_string().contains("missing required input_ref"));
        assert_eq!(thread.status, WorkflowThreadStatus::Running);
        assert_eq!(thread.next_turn_seq, 2);
    }

    #[test]
    fn executor_marks_thread_failed_when_gate_failure_stops_workflow() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::with_completions(vec![
            Ok(r#"{"missing":false}"#.to_string()),
            Ok(r#"{"missing":false}"#.to_string()),
        ]);
        let registered_profile = registered_profile(vec!["ok"], true);

        let error = run_workflow(
            &api,
            &registered_profile,
            &request(false),
            state_store.clone(),
            &FakeTaskPathExecutor::default(),
        )
        .unwrap_err();
        let thread_id = workflow_thread_id(&registered_profile.profile, node_id(), "summary");

        assert!(error.to_string().contains("failed gate"));
        assert_eq!(
            state_store.load_thread(&thread_id).unwrap().unwrap().status,
            WorkflowThreadStatus::Failed
        );
        assert!(api
            .workflow_events()
            .iter()
            .any(|event_type| event_type == "execution.workflow.turn_failed"));
    }

    #[test]
    fn executor_gate_fail_on_violation_stops_even_when_profile_policy_allows_continue() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::with_completions(vec![
            Ok(r#"{"missing":false}"#.to_string()),
            Ok(r#"{"missing":false}"#.to_string()),
        ]);
        let mut registered_profile = one_turn_registered_profile(vec!["ok"], false);
        registered_profile.profile.gates[0].fail_on_violation = true;

        let error = run_workflow(
            &api,
            &registered_profile,
            &request(false),
            state_store,
            &FakeTaskPathExecutor::default(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("failed gate"));
        assert!(api
            .workflow_events()
            .iter()
            .any(|event_type| event_type == "execution.workflow.turn_failed"));
    }

    #[test]
    fn executor_gate_failure_continues_as_completed_when_policy_allows() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::with_completions(vec![
            Ok(r#"{"missing":false}"#.to_string()),
            Ok(r#"{"missing":false}"#.to_string()),
            Ok(r#"{"missing":false}"#.to_string()),
            Ok(r#"{"missing":false}"#.to_string()),
        ]);
        let registered_profile = registered_profile(vec!["ok"], false);

        let summary = run_workflow(
            &api,
            &registered_profile,
            &request(false),
            state_store.clone(),
            &FakeTaskPathExecutor::default(),
        )
        .unwrap();
        let turns = state_store.load_turns(&summary.thread_id).unwrap();

        assert_eq!(summary.turns_completed, 2);
        assert!(turns
            .iter()
            .all(|turn| turn.status == WorkflowTurnStatus::Completed));
        assert_eq!(
            api.workflow_events(),
            vec![
                "execution.workflow.turn_started",
                "execution.workflow.turn_failed",
                "execution.workflow.turn_started",
                "execution.workflow.turn_failed",
                "execution.workflow.turn_completed",
                "execution.workflow.turn_started",
                "execution.workflow.turn_failed",
                "execution.workflow.turn_started",
                "execution.workflow.turn_failed",
                "execution.workflow.turn_completed",
            ]
        );
    }

    #[test]
    fn executor_routes_task_package_profiles_to_task_executor() {
        let dir = tempdir().unwrap();
        let state_store = WorkflowStateStore::from_root(dir.path()).unwrap();
        let api = FakeApi::default();
        api.set_head(Some([9; 32]));
        let registered_profile = registered_profile(vec!["ok"], true);
        let task_path_executor = FakeTaskPathExecutor {
            uses_task_package_path: true,
            resolved_final_frame: [4; 32],
        };

        let summary = run_workflow(
            &api,
            &registered_profile,
            &request(false),
            state_store,
            &task_path_executor,
        )
        .unwrap();

        assert_eq!(summary.turns_completed, 1);
        assert_eq!(summary.final_frame_id, Some([4; 32]));
        assert!(api.completion_requests.lock().unwrap().is_empty());
    }
}
