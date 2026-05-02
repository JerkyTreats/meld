use serde_json::json;
use std::fmt::Display;

use crate::error::ExecutionInvariantError;
use crate::execution::{
    ContextWritePort, ExecutionEventContext, GeneratedMetadataPort, ProviderPreparationView,
    ProviderValidationPort,
};
use crate::generation::{
    ChatMessage, GenerationOrchestrationRequest, MessageRole, NodeId,
    PromptContextLineageProgressEventData, PromptLineageRequest,
};
use crate::workflow::{
    evaluate_gate, prompt_link_record_from_contract_v1, workflow_turn_frame_type, GateOutcome,
    PromptLinkRecordInputV1, ResolvedTurnInputs, ThreadTurnGateRecordV1, WorkflowExecutionRequest,
    WorkflowGate, WorkflowProfile, WorkflowTurn, WorkflowTurnRecord, WorkflowTurnStatus,
};

use super::{
    emission::{
        emit_metadata_validation_event, emit_workflow_turn_event, metadata_event_payload,
        workflow_turn_payload,
    },
    errors::generation_failed,
    lifecycle::mark_thread_failed,
    WorkflowExecutorContext, WorkflowExecutorRuntime,
};

pub(super) struct CompletedTurn {
    pub frame_id: NodeId,
    pub content: String,
}

pub(super) struct TurnAttemptContext<'a, A, E>
where
    A: ContextWritePort + GeneratedMetadataPort,
{
    pub api: &'a A,
    pub profile: &'a WorkflowProfile,
    pub request: &'a WorkflowExecutionRequest,
    pub runtime: &'a WorkflowExecutorRuntime<'a, A, E>,
    pub event_context: Option<&'a ExecutionEventContext>,
    pub thread_id: &'a str,
    pub target_path: &'a str,
}

pub(super) struct TurnExecutionInput<'a> {
    pub turn: &'a WorkflowTurn,
    pub gate: &'a WorkflowGate,
    pub system_prompt: &'a str,
    pub prompt_template: &'a str,
    pub rendered_prompt: &'a str,
    pub resolved_inputs: &'a ResolvedTurnInputs,
    pub final_turn_seq: u32,
}

pub(super) async fn execute_turn_with_retries<A, E>(
    context: TurnAttemptContext<'_, A, E>,
    input: TurnExecutionInput<'_>,
    final_frame_id: Option<NodeId>,
) -> Result<CompletedTurn, E>
where
    A: WorkflowExecutorContext<E> + 'static,
    E: From<ExecutionInvariantError> + Display + Clone + Send + Sync + 'static,
    <A as ProviderValidationPort>::ProviderPreparation: ProviderPreparationView + Sync,
{
    let api = context.api;
    let profile = context.profile;
    let request = context.request;
    let runtime = context.runtime;
    let event_context = context.event_context;
    let thread_id = context.thread_id;
    let target_path = context.target_path;
    let turn = input.turn;
    let gate = input.gate;
    let state_store = &runtime.state_store;
    let mut attempt = 0usize;
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
                system_prompt: input.system_prompt.to_string(),
                user_prompt_template: input.prompt_template.to_string(),
                rendered_prompt: input.rendered_prompt.to_string(),
                context_payload: input.resolved_inputs.context_payload.clone(),
            },
            &request.agent_id,
            &request.provider.provider_name,
            provider_preparation.model_name(),
            provider_preparation.provider_type_slug(),
        )?;
        let turn_frame_type = workflow_turn_frame_type(
            &request.frame_type,
            turn,
            &prepared_lineage.prompt_link_contract.prompt_link_id,
            turn.seq == input.final_turn_seq,
        );

        emit_workflow_turn_event(
            api,
            event_context,
            "execution.workflow.turn_started",
            workflow_turn_payload(
                request,
                profile.workflow_id.clone(),
                thread_id.to_string(),
                turn.turn_id.clone(),
                turn.seq,
                target_path.to_string(),
                turn_frame_type.clone(),
                attempt,
                None,
                None,
            ),
        );

        state_store.upsert_turn(&WorkflowTurnRecord {
            thread_id: thread_id.to_string(),
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
                    prompt_link_id: prepared_lineage.prompt_link_contract.prompt_link_id.clone(),
                    prompt_digest: prepared_lineage.prompt_link_contract.prompt_digest.clone(),
                    context_digest: prepared_lineage.prompt_link_contract.context_digest.clone(),
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
                target_path,
                &turn_frame_type,
                &prepared_lineage,
                &previous_metadata,
                &profile.workflow_id,
                thread_id,
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
                        target_path,
                        &turn_frame_type,
                        &prepared_lineage,
                        &previous_metadata,
                        &profile.workflow_id,
                        thread_id,
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
                    thread_id: thread_id.to_string(),
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
                        target_path,
                        &turn_frame_type,
                        &prepared_lineage,
                        &previous_metadata,
                        &profile.workflow_id,
                        thread_id,
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
                        thread_id.to_string(),
                        turn.turn_id.clone(),
                        turn.seq,
                        target_path.to_string(),
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
                    thread_id,
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
                        content: input.system_prompt.to_string(),
                    },
                    ChatMessage {
                        role: MessageRole::User,
                        content: input.rendered_prompt.to_string(),
                    },
                ],
                event_context,
            )
            .await
        {
            Ok(response) => response,
            Err(err) => {
                state_store.upsert_turn(&WorkflowTurnRecord {
                    thread_id: thread_id.to_string(),
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
                        thread_id.to_string(),
                        turn.turn_id.clone(),
                        turn.seq,
                        target_path.to_string(),
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
                    thread_id,
                    request,
                    turn.seq,
                    final_frame_id,
                    runtime.now_millis,
                )?;
                return Err(err);
            }
        };

        let gate_result =
            evaluate_gate(gate, &response.content, Some(&input.resolved_inputs.values));
        let gate_record = ThreadTurnGateRecordV1::new(
            thread_id.to_string(),
            format!("turn-{}", turn.seq),
            gate.gate_type.clone(),
            gate_result.outcome.clone(),
            gate_result.reasons.clone(),
            (runtime.now_millis)(),
        );
        state_store.upsert_gate(thread_id, &turn.turn_id, &gate_record)?;

        if gate_result.outcome == GateOutcome::Fail {
            let gate_error: E = generation_failed(format!(
                "Workflow '{}' turn '{}' failed gate '{}': {}",
                profile.workflow_id,
                turn.turn_id,
                gate.gate_id,
                gate_result.reasons.join(" | ")
            ));
            state_store.upsert_turn(&WorkflowTurnRecord {
                thread_id: thread_id.to_string(),
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
                    thread_id.to_string(),
                    turn.turn_id.clone(),
                    turn.seq,
                    target_path.to_string(),
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
                    thread_id,
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
                thread_id: thread_id.to_string(),
                turn_id: format!("turn-{}", turn.seq),
                node_id: hex::encode(request.node_id),
                frame_id: hex::encode(frame_id),
                created_at_ms: (runtime.now_millis)(),
            },
        );
        state_store.upsert_prompt_link(thread_id, &turn.turn_id, &prompt_link_record)?;

        state_store.upsert_turn(&WorkflowTurnRecord {
            thread_id: thread_id.to_string(),
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
                thread_id.to_string(),
                turn.turn_id.clone(),
                turn.seq,
                target_path.to_string(),
                turn_frame_type.clone(),
                attempt,
                Some(hex::encode(frame_id)),
                None,
            ),
        );

        state_store.upsert_thread(&crate::workflow::WorkflowThreadRecord {
            thread_id: thread_id.to_string(),
            workflow_id: profile.workflow_id.clone(),
            node_id: hex::encode(request.node_id),
            frame_type: request.frame_type.clone(),
            status: crate::workflow::WorkflowThreadStatus::Running,
            next_turn_seq: turn.seq + 1,
            updated_at_ms: (runtime.now_millis)(),
            final_frame_id: Some(hex::encode(frame_id)),
        })?;

        return Ok(CompletedTurn {
            frame_id,
            content: response.content,
        });
    }

    Err(last_error.unwrap_or_else(|| {
        generation_failed(format!(
            "Workflow '{}' turn '{}' failed with no retryable error",
            profile.workflow_id, turn.turn_id
        ))
    }))
}
