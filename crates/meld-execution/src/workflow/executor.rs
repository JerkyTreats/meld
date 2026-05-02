//! Workflow runtime executor for bound agent turn workflows.

use async_trait::async_trait;
use meld_events::EventEnvelope;
use serde_json::json;
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
    ChatMessage, CompletionResponse, FrameMetadataValidationProgressEventData,
    GeneratedFrameMetadataInput, GenerationOrchestrationRequest, MessageRole, NodeId,
    PreparedPromptLineage, PreviousMetadataSnapshotView, PromptContextLineageProgressEventData,
    PromptLineageRequest,
};
use crate::workflow::{
    evaluate_gate, prompt_link_record_from_contract_v1, render_turn_prompt,
    resolve_prompt_template, resolve_turn_inputs, workflow_thread_id,
    workflow_turn_completed_envelope, workflow_turn_failed_envelope, workflow_turn_frame_type,
    workflow_turn_started_envelope, ExecutionWorkflowTurnEventData, GateOutcome,
    PromptLinkRecordInputV1, RegisteredWorkflowProfile, ThreadTurnGateRecordV1,
    WorkflowExecutionRequest, WorkflowExecutionSummary, WorkflowForceResetProgressEventData,
    WorkflowStateStore, WorkflowTargetProgressEventData, WorkflowThreadRecord,
    WorkflowThreadStatus, WorkflowTurnProgressEventData, WorkflowTurnRecord, WorkflowTurnStatus,
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

    #[allow(clippy::too_many_arguments)]
    async fn execute_task_path(
        &self,
        api: &A,
        workspace_root: &Path,
        registered_profile: &RegisteredWorkflowProfile,
        request: &WorkflowExecutionRequest,
        event_context: Option<&ExecutionEventContext>,
        state_store: &WorkflowStateStore,
        thread_id: &str,
        target_path: &str,
        final_turn_seq: u32,
        now_millis: fn() -> u64,
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
            .execute_task_path(
                api,
                workspace_root,
                registered_profile,
                request,
                event_context,
                state_store,
                &thread_id,
                &target_path,
                final_turn_seq,
                runtime.now_millis,
            )
            .await;
    }

    for turn in ordered_turns {
        if turn.seq < start_seq {
            continue;
        }

        let gate = profile
            .gates
            .iter()
            .find(|gate| gate.gate_id == turn.gate_id)
            .ok_or_else(|| {
                config_error(format!(
                    "Workflow '{}' missing gate '{}' for turn '{}'",
                    profile.workflow_id, turn.gate_id, turn.turn_id
                ))
            })?;

        let resolved_inputs = resolve_turn_inputs(
            api,
            request.node_id,
            &request.frame_type,
            &turn,
            &turn_outputs,
        )?;
        let prompt_template = resolve_prompt_template(
            api,
            registered_profile.source_path.as_deref(),
            &turn.prompt_ref,
        )?;
        let rendered_prompt = render_turn_prompt(&prompt_template, &turn, &resolved_inputs);

        let mut attempt = 0usize;
        let mut success = false;
        let mut last_error: Option<E> = None;

        while attempt < turn.retry_limit {
            attempt += 1;

            let orchestration_request = GenerationOrchestrationRequest {
                request_id: ((turn.seq as u64) * 1000) + attempt as u64,
                node_id: request.node_id,
                agent_id: request.agent_id.clone(),
                provider: request.provider.clone(),
                frame_type: request.frame_type.clone(),
                retry_count: attempt.saturating_sub(1),
                force: request.force,
            };
            let provider_preparation = api.prepare_provider_for_request(&orchestration_request)?;

            let prepared_lineage = api.prepare_prompt_lineage(
                &PromptLineageRequest {
                    system_prompt: system_prompt.clone(),
                    user_prompt_template: prompt_template.clone(),
                    rendered_prompt: rendered_prompt.clone(),
                    context_payload: resolved_inputs.context_payload.clone(),
                },
                &request.agent_id,
                &request.provider.provider_name,
                provider_preparation.model_name(),
                provider_preparation.provider_type_slug(),
            )?;
            let turn_frame_type = workflow_turn_frame_type(
                &request.frame_type,
                &turn,
                &prepared_lineage.prompt_link_contract.prompt_link_id,
                turn.seq == final_turn_seq,
            );

            emit_workflow_turn_event(
                api,
                event_context,
                "execution.workflow.turn_started",
                WorkflowTurnProgressEventData {
                    workflow_id: profile.workflow_id.clone(),
                    thread_id: thread_id.clone(),
                    turn_id: turn.turn_id.clone(),
                    turn_seq: turn.seq,
                    node_id: hex::encode(request.node_id),
                    path: target_path.clone(),
                    agent_id: request.agent_id.clone(),
                    provider_name: request.provider.provider_name.clone(),
                    frame_type: turn_frame_type.clone(),
                    attempt,
                    plan_id: request.plan_id.clone(),
                    level_index: request.level_index,
                    final_frame_id: None,
                    error: None,
                },
            );

            state_store.upsert_turn(&WorkflowTurnRecord {
                thread_id: thread_id.clone(),
                turn_id: turn.turn_id.clone(),
                seq: turn.seq,
                output_type: turn.output_type.clone(),
                status: WorkflowTurnStatus::Running,
                attempt_count: attempt,
                frame_id: None,
                output_text: None,
                updated_at_ms: (runtime.now_millis)(),
            })?;

            let mut orchestration_request = orchestration_request;
            orchestration_request.frame_type = turn_frame_type.clone();
            if let Some(ctx) = event_context {
                api.emit_progress_event(
                    ctx,
                    "prompt_context_lineage_prepared",
                    json!(PromptContextLineageProgressEventData {
                        node_id: hex::encode(request.node_id),
                        agent_id: request.agent_id.clone(),
                        provider_name: request.provider.provider_name.clone(),
                        frame_type: turn_frame_type.clone(),
                        prompt_link_id: prepared_lineage
                            .prompt_link_contract
                            .prompt_link_id
                            .clone(),
                        prompt_digest: prepared_lineage.prompt_link_contract.prompt_digest.clone(),
                        context_digest: prepared_lineage
                            .prompt_link_contract
                            .context_digest
                            .clone(),
                        system_prompt_artifact_id: prepared_lineage
                            .prompt_link_contract
                            .system_prompt_artifact_id
                            .clone(),
                        user_prompt_template_artifact_id: prepared_lineage
                            .prompt_link_contract
                            .user_prompt_template_artifact_id
                            .clone(),
                        rendered_prompt_artifact_id: prepared_lineage
                            .prompt_link_contract
                            .rendered_prompt_artifact_id
                            .clone(),
                        context_artifact_id: prepared_lineage
                            .prompt_link_contract
                            .context_artifact_id
                            .clone(),
                        lineage_failure_policy: "deterministic_orphan_keep".to_string(),
                    }),
                )?;
            }

            if request.force {
                api.tombstone_head(request.node_id, &turn_frame_type)?;
            }

            let previous_metadata = api.load_previous_metadata_snapshot(&orchestration_request)?;
            emit_metadata_validation_event(
                api,
                event_context,
                "frame_metadata_validation_started",
                metadata_event_payload(
                    request,
                    &target_path,
                    &turn_frame_type,
                    &prepared_lineage,
                    &previous_metadata,
                    &profile.workflow_id,
                    &thread_id,
                    &turn.turn_id,
                    turn.seq,
                    attempt,
                    None,
                ),
            );

            let generated_metadata = match api.build_and_validate_generated_metadata(
                &orchestration_request,
                &prepared_lineage.metadata_input,
                runtime.metadata_builder,
            ) {
                Ok(metadata) => {
                    emit_metadata_validation_event(
                        api,
                        event_context,
                        "frame_metadata_validation_succeeded",
                        metadata_event_payload(
                            request,
                            &target_path,
                            &turn_frame_type,
                            &prepared_lineage,
                            &previous_metadata,
                            &profile.workflow_id,
                            &thread_id,
                            &turn.turn_id,
                            turn.seq,
                            attempt,
                            None,
                        ),
                    );
                    metadata
                }
                Err(err) => {
                    state_store.upsert_turn(&WorkflowTurnRecord {
                        thread_id: thread_id.clone(),
                        turn_id: turn.turn_id.clone(),
                        seq: turn.seq,
                        output_type: turn.output_type.clone(),
                        status: WorkflowTurnStatus::Failed,
                        attempt_count: attempt,
                        frame_id: None,
                        output_text: None,
                        updated_at_ms: (runtime.now_millis)(),
                    })?;
                    emit_metadata_validation_event(
                        api,
                        event_context,
                        "frame_metadata_validation_failed",
                        metadata_event_payload(
                            request,
                            &target_path,
                            &turn_frame_type,
                            &prepared_lineage,
                            &previous_metadata,
                            &profile.workflow_id,
                            &thread_id,
                            &turn.turn_id,
                            turn.seq,
                            attempt,
                            Some(err.to_string()),
                        ),
                    );
                    emit_workflow_turn_event(
                        api,
                        event_context,
                        "execution.workflow.turn_failed",
                        workflow_turn_payload(
                            request,
                            profile.workflow_id.clone(),
                            thread_id.clone(),
                            turn.turn_id.clone(),
                            turn.seq,
                            target_path.clone(),
                            turn_frame_type.clone(),
                            attempt,
                            None,
                            Some(err.to_string()),
                        ),
                    );
                    last_error = Some(err.clone());
                    if attempt < turn.retry_limit {
                        continue;
                    }
                    mark_thread_failed(
                        state_store,
                        profile.workflow_id.clone(),
                        &thread_id,
                        request,
                        turn.seq,
                        final_frame_id,
                        runtime.now_millis,
                    )?;
                    return Err(err);
                }
            };

            let response = match api
                .execute_completion(
                    &orchestration_request,
                    &provider_preparation,
                    vec![
                        ChatMessage {
                            role: MessageRole::System,
                            content: system_prompt.clone(),
                        },
                        ChatMessage {
                            role: MessageRole::User,
                            content: rendered_prompt.clone(),
                        },
                    ],
                    event_context,
                )
                .await
            {
                Ok(response) => response,
                Err(err) => {
                    state_store.upsert_turn(&WorkflowTurnRecord {
                        thread_id: thread_id.clone(),
                        turn_id: turn.turn_id.clone(),
                        seq: turn.seq,
                        output_type: turn.output_type.clone(),
                        status: WorkflowTurnStatus::Failed,
                        attempt_count: attempt,
                        frame_id: None,
                        output_text: None,
                        updated_at_ms: (runtime.now_millis)(),
                    })?;
                    emit_workflow_turn_event(
                        api,
                        event_context,
                        "execution.workflow.turn_failed",
                        workflow_turn_payload(
                            request,
                            profile.workflow_id.clone(),
                            thread_id.clone(),
                            turn.turn_id.clone(),
                            turn.seq,
                            target_path.clone(),
                            turn_frame_type.clone(),
                            attempt,
                            None,
                            Some(err.to_string()),
                        ),
                    );
                    last_error = Some(err.clone());
                    if attempt < turn.retry_limit {
                        continue;
                    }
                    mark_thread_failed(
                        state_store,
                        profile.workflow_id.clone(),
                        &thread_id,
                        request,
                        turn.seq,
                        final_frame_id,
                        runtime.now_millis,
                    )?;
                    return Err(err);
                }
            };

            let gate_result = evaluate_gate(gate, &response.content, Some(&resolved_inputs.values));
            let gate_record = ThreadTurnGateRecordV1::new(
                thread_id.clone(),
                format!("turn-{}", turn.seq),
                gate.gate_type.clone(),
                gate_result.outcome.clone(),
                gate_result.reasons.clone(),
                (runtime.now_millis)(),
            );
            state_store.upsert_gate(&thread_id, &turn.turn_id, &gate_record)?;

            if gate_result.outcome == GateOutcome::Fail {
                let gate_error: E = generation_failed(format!(
                    "Workflow '{}' turn '{}' failed gate '{}': {}",
                    profile.workflow_id,
                    turn.turn_id,
                    gate.gate_id,
                    gate_result.reasons.join(" | ")
                ));
                state_store.upsert_turn(&WorkflowTurnRecord {
                    thread_id: thread_id.clone(),
                    turn_id: turn.turn_id.clone(),
                    seq: turn.seq,
                    output_type: turn.output_type.clone(),
                    status: WorkflowTurnStatus::Failed,
                    attempt_count: attempt,
                    frame_id: None,
                    output_text: Some(response.content.clone()),
                    updated_at_ms: (runtime.now_millis)(),
                })?;
                emit_workflow_turn_event(
                    api,
                    event_context,
                    "execution.workflow.turn_failed",
                    workflow_turn_payload(
                        request,
                        profile.workflow_id.clone(),
                        thread_id.clone(),
                        turn.turn_id.clone(),
                        turn.seq,
                        target_path.clone(),
                        turn_frame_type.clone(),
                        attempt,
                        None,
                        Some(gate_error.to_string()),
                    ),
                );
                last_error = Some(gate_error.clone());
                if attempt < turn.retry_limit {
                    continue;
                }
                if gate.fail_on_violation || profile.failure_policy.stop_on_gate_fail {
                    mark_thread_failed(
                        state_store,
                        profile.workflow_id.clone(),
                        &thread_id,
                        request,
                        turn.seq,
                        final_frame_id,
                        runtime.now_millis,
                    )?;
                    return Err(gate_error);
                }
            }

            let frame = (runtime.build_frame)(
                request.node_id,
                response.content.as_bytes().to_vec(),
                turn_frame_type.clone(),
                request.agent_id.clone(),
                generated_metadata,
            )?;
            let frame_id = api.put_frame(request.node_id, frame, request.agent_id.clone())?;

            let prompt_link_record = prompt_link_record_from_contract_v1(
                &prepared_lineage.prompt_link_contract,
                &PromptLinkRecordInputV1 {
                    thread_id: thread_id.clone(),
                    turn_id: format!("turn-{}", turn.seq),
                    node_id: hex::encode(request.node_id),
                    frame_id: hex::encode(frame_id),
                    created_at_ms: (runtime.now_millis)(),
                },
            );
            state_store.upsert_prompt_link(&thread_id, &turn.turn_id, &prompt_link_record)?;

            state_store.upsert_turn(&WorkflowTurnRecord {
                thread_id: thread_id.clone(),
                turn_id: turn.turn_id.clone(),
                seq: turn.seq,
                output_type: turn.output_type.clone(),
                status: WorkflowTurnStatus::Completed,
                attempt_count: attempt,
                frame_id: Some(hex::encode(frame_id)),
                output_text: Some(response.content.clone()),
                updated_at_ms: (runtime.now_millis)(),
            })?;

            emit_workflow_turn_event(
                api,
                event_context,
                "execution.workflow.turn_completed",
                workflow_turn_payload(
                    request,
                    profile.workflow_id.clone(),
                    thread_id.clone(),
                    turn.turn_id.clone(),
                    turn.seq,
                    target_path.clone(),
                    turn_frame_type.clone(),
                    attempt,
                    Some(hex::encode(frame_id)),
                    None,
                ),
            );

            state_store.upsert_thread(&WorkflowThreadRecord {
                thread_id: thread_id.clone(),
                workflow_id: profile.workflow_id.clone(),
                node_id: hex::encode(request.node_id),
                frame_type: request.frame_type.clone(),
                status: WorkflowThreadStatus::Running,
                next_turn_seq: turn.seq + 1,
                updated_at_ms: (runtime.now_millis)(),
                final_frame_id: Some(hex::encode(frame_id)),
            })?;

            turn_outputs.insert(turn.output_type.clone(), response.content.clone());
            turn_outputs.insert(turn.turn_id.clone(), response.content.clone());
            completed_turns += 1;
            final_frame_id = Some(frame_id);
            success = true;
            break;
        }

        if !success {
            mark_thread_failed(
                state_store,
                profile.workflow_id.clone(),
                &thread_id,
                request,
                turn.seq,
                final_frame_id,
                runtime.now_millis,
            )?;
            return Err(last_error.unwrap_or_else(|| {
                generation_failed(format!(
                    "Workflow '{}' turn '{}' failed with no retryable error",
                    profile.workflow_id, turn.turn_id
                ))
            }));
        }
    }

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

#[allow(clippy::too_many_arguments)]
fn metadata_event_payload(
    request: &WorkflowExecutionRequest,
    target_path: &str,
    turn_frame_type: &str,
    prepared_lineage: &PreparedPromptLineage,
    previous_metadata: &PreviousMetadataSnapshotView,
    workflow_id: &str,
    thread_id: &str,
    turn_id: &str,
    turn_seq: u32,
    attempt: usize,
    error: Option<String>,
) -> FrameMetadataValidationProgressEventData {
    FrameMetadataValidationProgressEventData {
        node_id: hex::encode(request.node_id),
        path: target_path.to_string(),
        agent_id: request.agent_id.clone(),
        provider_name: request.provider.provider_name.clone(),
        frame_type: turn_frame_type.to_string(),
        prompt_digest: prepared_lineage.metadata_input.prompt_digest.clone(),
        context_digest: prepared_lineage.metadata_input.context_digest.clone(),
        prompt_link_id: prepared_lineage.metadata_input.prompt_link_id.clone(),
        previous_frame_id: previous_metadata.frame_id.clone(),
        previous_prompt_digest: previous_metadata.prompt_digest.clone(),
        previous_context_digest: previous_metadata.context_digest.clone(),
        previous_prompt_link_id: previous_metadata.prompt_link_id.clone(),
        workflow_id: Some(workflow_id.to_string()),
        thread_id: Some(thread_id.to_string()),
        turn_id: Some(turn_id.to_string()),
        turn_seq: Some(turn_seq),
        attempt: Some(attempt),
        plan_id: request.plan_id.clone(),
        level_index: request.level_index,
        error,
    }
}

#[allow(clippy::too_many_arguments)]
fn workflow_turn_payload(
    request: &WorkflowExecutionRequest,
    workflow_id: String,
    thread_id: String,
    turn_id: String,
    turn_seq: u32,
    target_path: String,
    frame_type: String,
    attempt: usize,
    final_frame_id: Option<String>,
    error: Option<String>,
) -> WorkflowTurnProgressEventData {
    WorkflowTurnProgressEventData {
        workflow_id,
        thread_id,
        turn_id,
        turn_seq,
        node_id: hex::encode(request.node_id),
        path: target_path,
        agent_id: request.agent_id.clone(),
        provider_name: request.provider.provider_name.clone(),
        frame_type,
        attempt,
        plan_id: request.plan_id.clone(),
        level_index: request.level_index,
        final_frame_id,
        error,
    }
}

#[allow(clippy::too_many_arguments)]
fn completed_target_summary<E>(
    api: &(impl ExecutionProgressPort<Error = E> + ?Sized),
    event_context: Option<&ExecutionEventContext>,
    workflow_id: String,
    thread_id: String,
    request: &WorkflowExecutionRequest,
    target_path: String,
    final_frame_id: Option<NodeId>,
    turns_completed: usize,
    reused_existing_head: bool,
) -> WorkflowExecutionSummary {
    emit_workflow_target_event(
        api,
        event_context,
        "workflow_target_completed",
        WorkflowTargetProgressEventData {
            workflow_id: workflow_id.clone(),
            thread_id: thread_id.clone(),
            node_id: hex::encode(request.node_id),
            path: target_path,
            agent_id: request.agent_id.clone(),
            provider_name: request.provider.provider_name.clone(),
            frame_type: request.frame_type.clone(),
            plan_id: request.plan_id.clone(),
            level_index: request.level_index,
            final_frame_id: final_frame_id.map(hex::encode),
            turns_completed: Some(turns_completed),
            reused_existing_head: Some(reused_existing_head),
        },
    );

    WorkflowExecutionSummary {
        workflow_id,
        thread_id,
        turns_completed,
        final_frame_id,
    }
}

fn mark_thread_failed<E>(
    state_store: &WorkflowStateStore,
    workflow_id: String,
    thread_id: &str,
    request: &WorkflowExecutionRequest,
    next_turn_seq: u32,
    final_frame_id: Option<NodeId>,
    now_millis: fn() -> u64,
) -> Result<(), E>
where
    E: From<ExecutionInvariantError>,
{
    state_store.upsert_thread(&WorkflowThreadRecord {
        thread_id: thread_id.to_string(),
        workflow_id,
        node_id: hex::encode(request.node_id),
        frame_type: request.frame_type.clone(),
        status: WorkflowThreadStatus::Failed,
        next_turn_seq,
        updated_at_ms: now_millis(),
        final_frame_id: final_frame_id.map(hex::encode),
    })?;
    Ok(())
}

fn emit_workflow_target_event<E>(
    api: &(impl ExecutionProgressPort<Error = E> + ?Sized),
    event_context: Option<&ExecutionEventContext>,
    event_type: &str,
    payload: WorkflowTargetProgressEventData,
) {
    if let Some(ctx) = event_context {
        let _ = api.emit_progress_event(ctx, event_type, json!(payload));
    }
}

fn emit_workflow_turn_event<E>(
    api: &(impl EventPublicationPort<Error = E, EventEnvelope = EventEnvelope> + ?Sized),
    event_context: Option<&ExecutionEventContext>,
    event_type: &str,
    payload: WorkflowTurnProgressEventData,
) {
    if let Some(ctx) = event_context {
        let payload = ExecutionWorkflowTurnEventData {
            workflow_id: payload.workflow_id,
            thread_id: payload.thread_id,
            turn_id: payload.turn_id,
            turn_seq: payload.turn_seq,
            node_id: payload.node_id,
            path: payload.path,
            agent_id: payload.agent_id,
            provider_name: payload.provider_name,
            frame_type: payload.frame_type,
            attempt: payload.attempt,
            plan_id: payload.plan_id,
            level_index: payload.level_index,
            final_frame_id: payload.final_frame_id,
            error: payload.error,
        };
        let envelope = match event_type {
            "execution.workflow.turn_started" => {
                workflow_turn_started_envelope(&ctx.session_id, payload)
            }
            "execution.workflow.turn_completed" => {
                workflow_turn_completed_envelope(&ctx.session_id, payload)
            }
            "execution.workflow.turn_failed" => {
                workflow_turn_failed_envelope(&ctx.session_id, payload)
            }
            _ => return,
        };
        let _ = api.publish_execution_envelope(ctx, envelope);
    }
}

fn emit_workflow_force_reset_event<E>(
    api: &(impl ExecutionProgressPort<Error = E> + ?Sized),
    event_context: Option<&ExecutionEventContext>,
    event_type: &str,
    payload: WorkflowForceResetProgressEventData,
) {
    if let Some(ctx) = event_context {
        let _ = api.emit_progress_event(ctx, event_type, json!(payload));
    }
}

fn emit_metadata_validation_event<E>(
    api: &(impl ExecutionProgressPort<Error = E> + ?Sized),
    event_context: Option<&ExecutionEventContext>,
    event_type: &str,
    payload: FrameMetadataValidationProgressEventData,
) {
    if let Some(ctx) = event_context {
        let _ = api.emit_progress_event(ctx, event_type, json!(payload));
    }
}

fn config_error<E>(message: String) -> E
where
    E: From<ExecutionInvariantError>,
{
    E::from(ExecutionInvariantError::ConfigError(message))
}

fn generation_failed<E>(message: String) -> E
where
    E: From<ExecutionInvariantError>,
{
    E::from(ExecutionInvariantError::GenerationFailed(message))
}
