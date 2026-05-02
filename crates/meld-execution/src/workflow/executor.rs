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

pub type NodeNotFoundBuilder<'a, E> = dyn Fn(NodeId) -> E + Send + Sync + 'a;

pub struct WorkflowExecutorRuntime<'a, A, E>
where
    A: ContextWritePort + GeneratedMetadataPort,
{
    pub state_store: WorkflowStateStore,
    pub metadata_builder: &'a A::GeneratedMetadataBuilder,
    pub build_frame: &'a FrameBuilder<'a, A, E>,
    pub node_not_found: &'a NodeNotFoundBuilder<'a, E>,
    pub task_path_executor: &'a dyn WorkflowTaskPathExecutor<A, E>,
    pub now_millis: fn() -> u64,
}

pub struct WorkflowTaskPathExecution<'a, A> {
    pub api: &'a A,
    pub workspace_root: &'a Path,
    pub registered_profile: &'a RegisteredWorkflowProfile,
    pub request: &'a WorkflowExecutionRequest,
    pub event_context: Option<&'a ExecutionEventContext>,
    pub state_store: &'a WorkflowStateStore,
    pub thread_id: &'a str,
    pub target_path: &'a str,
    pub final_turn_seq: u32,
    pub now_millis: fn() -> u64,
}

#[async_trait]
pub trait WorkflowTaskPathExecutor<A, E>: Send + Sync {
    fn uses_task_package_path(
        &self,
        registered_profile: &RegisteredWorkflowProfile,
    ) -> Result<bool, E>;

    fn resolve_completed_task_path_final_frame(
        &self,
        api: &A,
        registered_profile: &RegisteredWorkflowProfile,
        request: &WorkflowExecutionRequest,
        existing: &WorkflowThreadRecord,
    ) -> Result<NodeId, E>;

    async fn execute_task_path(
        &self,
        execution: WorkflowTaskPathExecution<'_, A>,
    ) -> Result<WorkflowExecutionSummary, E>;
}

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
