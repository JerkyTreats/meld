//! Workflow runtime executor for bound agent turn workflows.

use crate::agent::profile::prompt_contract::PromptContract;
use crate::api::ContextApi;
use crate::capability::{CapabilityCatalog, CapabilityExecutorRegistry};
use crate::context::frame::{Basis, Frame};
use crate::context::generation::contracts::{
    GeneratedMetadataBuilder, GenerationOrchestrationRequest,
};
use crate::context::generation::metadata_construction::{
    build_and_validate_generated_metadata, load_previous_metadata_snapshot,
};
use crate::context::generation::provider_execution::{
    execute_completion, prepare_provider_for_request,
};
use crate::context::queue::QueueEventContext;
use crate::error::ApiError;
use crate::metadata::frame_write_contract::build_generated_metadata;
use crate::prompt_context::{prepare_generated_lineage, PromptContextLineageInput};
use crate::provider::ProviderExecutionBinding;
use crate::task::templates::{
    prepare_registered_workflow_task_run, workflow_uses_task_package_path,
};
use crate::task::{
    execute_task_to_completion, load_task_package_spec_for_workflow, TaskExecutor,
    WorkflowPackageTriggerRequest, WorkflowTaskTelemetry,
};
use crate::telemetry::{
    now_millis, FrameMetadataValidationEventData, PromptContextLineageEventData,
    WorkflowForceResetEventData, WorkflowTargetEventData, WorkflowTurnEventData,
};
use crate::types::{FrameID, NodeID};
use crate::workflow::gates::evaluate_gate;
use crate::workflow::events::{
    workflow_turn_completed_envelope, workflow_turn_failed_envelope,
    workflow_turn_started_envelope, ExecutionWorkflowTurnEventData,
};
use crate::workflow::profile::{WorkflowProfile, WorkflowTurn};
use crate::workflow::record_contracts::{
    prompt_link_record_from_contract_v1, GateOutcome, PromptLinkRecordInputV1,
    ThreadTurnGateRecordV1,
};
use crate::workflow::registry::RegisteredWorkflowProfile;
use crate::workflow::resolver::{render_turn_prompt, resolve_prompt_template, resolve_turn_inputs};
use crate::workflow::state_store::{
    WorkflowStateStore, WorkflowThreadRecord, WorkflowThreadStatus, WorkflowTurnRecord,
    WorkflowTurnStatus,
};
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct WorkflowExecutionRequest {
    pub node_id: NodeID,
    pub agent_id: String,
    pub provider: ProviderExecutionBinding,
    pub frame_type: String,
    pub force: bool,
    pub path: Option<String>,
    pub plan_id: Option<String>,
    pub level_index: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecutionSummary {
    pub workflow_id: String,
    pub thread_id: String,
    pub turns_completed: usize,
    pub final_frame_id: Option<FrameID>,
}

pub fn execute_registered_workflow(
    api: &ContextApi,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowExecutionRequest,
    event_context: Option<&QueueEventContext>,
) -> Result<WorkflowExecutionSummary, ApiError> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|err| ApiError::ProviderError(format!("Failed to create runtime: {}", err)))?;

    rt.block_on(async move {
        execute_registered_workflow_async(
            api,
            workspace_root,
            registered_profile,
            request,
            event_context,
        )
        .await
    })
}

pub(crate) async fn execute_registered_workflow_async(
    api: &ContextApi,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowExecutionRequest,
    event_context: Option<&QueueEventContext>,
) -> Result<WorkflowExecutionSummary, ApiError> {
    let profile = &registered_profile.profile;
    let thread_id = build_thread_id(profile, request.node_id, &request.frame_type);
    let state_store = WorkflowStateStore::new(workspace_root)?;
    let node_record = api
        .node_store()
        .get(&request.node_id)
        .map_err(ApiError::from)?
        .ok_or(ApiError::NodeNotFound(request.node_id))?;
    let target_path = request
        .path
        .clone()
        .unwrap_or_else(|| node_record.path.to_string_lossy().to_string());

    let mut start_seq = 1u32;
    let mut turn_outputs: HashMap<String, String> = HashMap::new();
    let mut completed_turns = 0usize;
    let task_path_final_head =
        if !request.force && workflow_uses_task_package_path(registered_profile)? {
            api.get_head(&request.node_id, &request.frame_type)?
        } else {
            None
        };

    if let Some(existing) = state_store.load_thread(&thread_id)? {
        match existing.status {
            WorkflowThreadStatus::Completed => {
                if !request.force {
                    let head = api.get_head(&request.node_id, &request.frame_type)?;
                    emit_workflow_target_event(
                        event_context,
                        "workflow_target_completed",
                        WorkflowTargetEventData {
                            workflow_id: profile.workflow_id.clone(),
                            thread_id: thread_id.clone(),
                            node_id: hex::encode(request.node_id),
                            path: target_path.clone(),
                            agent_id: request.agent_id.clone(),
                            provider_name: request.provider.provider_name.clone(),
                            frame_type: request.frame_type.clone(),
                            plan_id: request.plan_id.clone(),
                            level_index: request.level_index,
                            final_frame_id: head.map(hex::encode),
                            turns_completed: Some(0),
                            reused_existing_head: Some(true),
                        },
                    );
                    return Ok(WorkflowExecutionSummary {
                        workflow_id: profile.workflow_id.clone(),
                        thread_id,
                        turns_completed: 0,
                        final_frame_id: head,
                    });
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
                        updated_at_ms: now_millis(),
                    })?;
                    emit_workflow_target_event(
                        event_context,
                        "workflow_target_completed",
                        WorkflowTargetEventData {
                            workflow_id: profile.workflow_id.clone(),
                            thread_id: thread_id.clone(),
                            node_id: hex::encode(request.node_id),
                            path: target_path.clone(),
                            agent_id: request.agent_id.clone(),
                            provider_name: request.provider.provider_name.clone(),
                            frame_type: request.frame_type.clone(),
                            plan_id: request.plan_id.clone(),
                            level_index: request.level_index,
                            final_frame_id: Some(hex::encode(head)),
                            turns_completed: Some(0),
                            reused_existing_head: Some(true),
                        },
                    );
                    return Ok(WorkflowExecutionSummary {
                        workflow_id: profile.workflow_id.clone(),
                        thread_id,
                        turns_completed: 0,
                        final_frame_id: Some(head),
                    });
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
            updated_at_ms: now_millis(),
        })?;
        emit_workflow_target_event(
            event_context,
            "workflow_target_completed",
            WorkflowTargetEventData {
                workflow_id: profile.workflow_id.clone(),
                thread_id: thread_id.clone(),
                node_id: hex::encode(request.node_id),
                path: target_path.clone(),
                agent_id: request.agent_id.clone(),
                provider_name: request.provider.provider_name.clone(),
                frame_type: request.frame_type.clone(),
                plan_id: request.plan_id.clone(),
                level_index: request.level_index,
                final_frame_id: Some(hex::encode(head)),
                turns_completed: Some(0),
                reused_existing_head: Some(true),
            },
        );
        return Ok(WorkflowExecutionSummary {
            workflow_id: profile.workflow_id.clone(),
            thread_id,
            turns_completed: 0,
            final_frame_id: Some(head),
        });
    }

    if request.force {
        let previous_head = api.tombstone_head(request.node_id, &request.frame_type)?;
        if let Some(previous_frame_id) = previous_head {
            emit_workflow_force_reset_event(
                event_context,
                "workflow_target_force_reset",
                WorkflowForceResetEventData {
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
        updated_at_ms: now_millis(),
    })?;

    emit_workflow_target_event(
        event_context,
        "workflow_target_started",
        WorkflowTargetEventData {
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

    let agent = api.get_agent(&request.agent_id)?;
    let prompt_contract = PromptContract::from_agent(&agent)?;

    let metadata_builder: &GeneratedMetadataBuilder = &build_generated_metadata;

    let mut final_frame_id: Option<FrameID> =
        api.get_head(&request.node_id, &request.frame_type)?;
    let ordered_turns = profile.ordered_turns();
    let final_turn_seq = ordered_turns
        .last()
        .map(|turn| turn.seq)
        .unwrap_or_default();

    if workflow_uses_task_package_path(registered_profile)? {
        return execute_registered_workflow_via_task_async(
            api,
            workspace_root,
            registered_profile,
            request,
            event_context,
            &state_store,
            &thread_id,
            &target_path,
            final_turn_seq,
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
                ApiError::ConfigError(format!(
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
        let mut last_error: Option<ApiError> = None;

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
            let provider_preparation = prepare_provider_for_request(api, &orchestration_request)?;

            let prepared_lineage = prepare_generated_lineage(
                api.prompt_context_storage(),
                &PromptContextLineageInput {
                    system_prompt: prompt_contract.system_prompt.clone(),
                    user_prompt_template: prompt_template.clone(),
                    rendered_prompt: rendered_prompt.clone(),
                    context_payload: resolved_inputs.context_payload.clone(),
                },
                &request.agent_id,
                &request.provider.provider_name,
                provider_preparation.client.model_name(),
                &provider_preparation.provider_type,
            )?;
            let turn_frame_type = workflow_turn_frame_type(
                &request.frame_type,
                &turn,
                &prepared_lineage.prompt_link_contract.prompt_link_id,
                turn.seq == final_turn_seq,
            );

            emit_workflow_turn_event(
                event_context,
                "execution.workflow.turn_started",
                WorkflowTurnEventData {
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
                updated_at_ms: now_millis(),
            })?;

            let mut orchestration_request = orchestration_request;
            orchestration_request.frame_type = turn_frame_type.clone();
            if let Some(ctx) = event_context {
                let lineage_event = PromptContextLineageEventData {
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
                };
                ctx.progress.emit_event_best_effort(
                    &ctx.session_id,
                    "prompt_context_lineage_prepared",
                    json!(lineage_event),
                );
            }

            let previous_metadata = load_previous_metadata_snapshot(api, &orchestration_request)?;
            emit_metadata_validation_event(
                event_context,
                "frame_metadata_validation_started",
                FrameMetadataValidationEventData {
                    node_id: hex::encode(request.node_id),
                    path: target_path.clone(),
                    agent_id: request.agent_id.clone(),
                    provider_name: request.provider.provider_name.clone(),
                    frame_type: turn_frame_type.clone(),
                    prompt_digest: prepared_lineage.metadata_input.prompt_digest.clone(),
                    context_digest: prepared_lineage.metadata_input.context_digest.clone(),
                    prompt_link_id: prepared_lineage.metadata_input.prompt_link_id.clone(),
                    previous_frame_id: previous_metadata.frame_id.clone(),
                    previous_prompt_digest: previous_metadata.prompt_digest.clone(),
                    previous_context_digest: previous_metadata.context_digest.clone(),
                    previous_prompt_link_id: previous_metadata.prompt_link_id.clone(),
                    workflow_id: Some(profile.workflow_id.clone()),
                    thread_id: Some(thread_id.clone()),
                    turn_id: Some(turn.turn_id.clone()),
                    turn_seq: Some(turn.seq),
                    attempt: Some(attempt),
                    plan_id: request.plan_id.clone(),
                    level_index: request.level_index,
                    error: None,
                },
            );

            let generated_metadata = match build_and_validate_generated_metadata(
                api,
                &orchestration_request,
                &prepared_lineage.metadata_input,
                metadata_builder,
            ) {
                Ok(metadata) => {
                    emit_metadata_validation_event(
                        event_context,
                        "frame_metadata_validation_succeeded",
                        FrameMetadataValidationEventData {
                            node_id: hex::encode(request.node_id),
                            path: target_path.clone(),
                            agent_id: request.agent_id.clone(),
                            provider_name: request.provider.provider_name.clone(),
                            frame_type: turn_frame_type.clone(),
                            prompt_digest: prepared_lineage.metadata_input.prompt_digest.clone(),
                            context_digest: prepared_lineage.metadata_input.context_digest.clone(),
                            prompt_link_id: prepared_lineage.metadata_input.prompt_link_id.clone(),
                            previous_frame_id: previous_metadata.frame_id.clone(),
                            previous_prompt_digest: previous_metadata.prompt_digest.clone(),
                            previous_context_digest: previous_metadata.context_digest.clone(),
                            previous_prompt_link_id: previous_metadata.prompt_link_id.clone(),
                            workflow_id: Some(profile.workflow_id.clone()),
                            thread_id: Some(thread_id.clone()),
                            turn_id: Some(turn.turn_id.clone()),
                            turn_seq: Some(turn.seq),
                            attempt: Some(attempt),
                            plan_id: request.plan_id.clone(),
                            level_index: request.level_index,
                            error: None,
                        },
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
                        updated_at_ms: now_millis(),
                    })?;
                    emit_metadata_validation_event(
                        event_context,
                        "frame_metadata_validation_failed",
                        FrameMetadataValidationEventData {
                            node_id: hex::encode(request.node_id),
                            path: target_path.clone(),
                            agent_id: request.agent_id.clone(),
                            provider_name: request.provider.provider_name.clone(),
                            frame_type: turn_frame_type.clone(),
                            prompt_digest: prepared_lineage.metadata_input.prompt_digest.clone(),
                            context_digest: prepared_lineage.metadata_input.context_digest.clone(),
                            prompt_link_id: prepared_lineage.metadata_input.prompt_link_id.clone(),
                            previous_frame_id: previous_metadata.frame_id.clone(),
                            previous_prompt_digest: previous_metadata.prompt_digest.clone(),
                            previous_context_digest: previous_metadata.context_digest.clone(),
                            previous_prompt_link_id: previous_metadata.prompt_link_id.clone(),
                            workflow_id: Some(profile.workflow_id.clone()),
                            thread_id: Some(thread_id.clone()),
                            turn_id: Some(turn.turn_id.clone()),
                            turn_seq: Some(turn.seq),
                            attempt: Some(attempt),
                            plan_id: request.plan_id.clone(),
                            level_index: request.level_index,
                            error: Some(err.to_string()),
                        },
                    );
                    emit_workflow_turn_event(
                        event_context,
                        "execution.workflow.turn_failed",
                        WorkflowTurnEventData {
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
                            error: Some(err.to_string()),
                        },
                    );
                    last_error = Some(err.clone());
                    if attempt < turn.retry_limit {
                        continue;
                    }
                    state_store.upsert_thread(&WorkflowThreadRecord {
                        thread_id: thread_id.clone(),
                        workflow_id: profile.workflow_id.clone(),
                        node_id: hex::encode(request.node_id),
                        frame_type: request.frame_type.clone(),
                        status: WorkflowThreadStatus::Failed,
                        next_turn_seq: turn.seq,
                        updated_at_ms: now_millis(),
                    })?;
                    return Err(err);
                }
            };

            let response = match execute_completion(
                &orchestration_request,
                &provider_preparation,
                vec![
                    crate::provider::ChatMessage {
                        role: crate::provider::MessageRole::System,
                        content: prompt_contract.system_prompt.clone(),
                    },
                    crate::provider::ChatMessage {
                        role: crate::provider::MessageRole::User,
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
                        updated_at_ms: now_millis(),
                    })?;
                    emit_workflow_turn_event(
                        event_context,
                        "execution.workflow.turn_failed",
                        WorkflowTurnEventData {
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
                            error: Some(err.to_string()),
                        },
                    );
                    last_error = Some(err.clone());
                    if attempt < turn.retry_limit {
                        continue;
                    }
                    state_store.upsert_thread(&WorkflowThreadRecord {
                        thread_id: thread_id.clone(),
                        workflow_id: profile.workflow_id.clone(),
                        node_id: hex::encode(request.node_id),
                        frame_type: request.frame_type.clone(),
                        status: WorkflowThreadStatus::Failed,
                        next_turn_seq: turn.seq,
                        updated_at_ms: now_millis(),
                    })?;
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
                now_millis(),
            );
            state_store.upsert_gate(&thread_id, &turn.turn_id, &gate_record)?;

            if gate_result.outcome == GateOutcome::Fail {
                let gate_error = ApiError::GenerationFailed(format!(
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
                    updated_at_ms: now_millis(),
                })?;
                emit_workflow_turn_event(
                    event_context,
                    "execution.workflow.turn_failed",
                    WorkflowTurnEventData {
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
                        error: Some(gate_error.to_string()),
                    },
                );
                last_error = Some(gate_error.clone());
                if attempt < turn.retry_limit {
                    continue;
                }
                if gate.fail_on_violation || profile.failure_policy.stop_on_gate_fail {
                    state_store.upsert_thread(&WorkflowThreadRecord {
                        thread_id: thread_id.clone(),
                        workflow_id: profile.workflow_id.clone(),
                        node_id: hex::encode(request.node_id),
                        frame_type: request.frame_type.clone(),
                        status: WorkflowThreadStatus::Failed,
                        next_turn_seq: turn.seq,
                        updated_at_ms: now_millis(),
                    })?;
                    return Err(gate_error);
                }
            }

            let frame = Frame::new(
                Basis::Node(request.node_id),
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
                    created_at_ms: now_millis(),
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
                updated_at_ms: now_millis(),
            })?;

            emit_workflow_turn_event(
                event_context,
                "execution.workflow.turn_completed",
                WorkflowTurnEventData {
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
                    final_frame_id: Some(hex::encode(frame_id)),
                    error: None,
                },
            );

            state_store.upsert_thread(&WorkflowThreadRecord {
                thread_id: thread_id.clone(),
                workflow_id: profile.workflow_id.clone(),
                node_id: hex::encode(request.node_id),
                frame_type: request.frame_type.clone(),
                status: WorkflowThreadStatus::Running,
                next_turn_seq: turn.seq + 1,
                updated_at_ms: now_millis(),
            })?;

            turn_outputs.insert(turn.output_type.clone(), response.content.clone());
            turn_outputs.insert(turn.turn_id.clone(), response.content.clone());
            completed_turns += 1;
            final_frame_id = Some(frame_id);
            success = true;
            break;
        }

        if !success {
            state_store.upsert_thread(&WorkflowThreadRecord {
                thread_id: thread_id.clone(),
                workflow_id: profile.workflow_id.clone(),
                node_id: hex::encode(request.node_id),
                frame_type: request.frame_type.clone(),
                status: WorkflowThreadStatus::Failed,
                next_turn_seq: turn.seq,
                updated_at_ms: now_millis(),
            })?;
            return Err(last_error.unwrap_or_else(|| {
                ApiError::GenerationFailed(format!(
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
        updated_at_ms: now_millis(),
    })?;

    emit_workflow_target_event(
        event_context,
        "workflow_target_completed",
        WorkflowTargetEventData {
            workflow_id: profile.workflow_id.clone(),
            thread_id: thread_id.clone(),
            node_id: hex::encode(request.node_id),
            path: target_path,
            agent_id: request.agent_id.clone(),
            provider_name: request.provider.provider_name.clone(),
            frame_type: request.frame_type.clone(),
            plan_id: request.plan_id.clone(),
            level_index: request.level_index,
            final_frame_id: final_frame_id.map(hex::encode),
            turns_completed: Some(completed_turns),
            reused_existing_head: Some(false),
        },
    );

    Ok(WorkflowExecutionSummary {
        workflow_id: profile.workflow_id.clone(),
        thread_id,
        turns_completed: completed_turns,
        final_frame_id,
    })
}

#[allow(clippy::too_many_arguments)]
async fn execute_registered_workflow_via_task_async(
    api: &ContextApi,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowExecutionRequest,
    event_context: Option<&QueueEventContext>,
    state_store: &WorkflowStateStore,
    thread_id: &str,
    target_path: &str,
    final_turn_seq: u32,
) -> Result<WorkflowExecutionSummary, ApiError> {
    let mut catalog = CapabilityCatalog::new();
    let mut registry = CapabilityExecutorRegistry::new();
    register_task_path_capabilities(&mut catalog, &mut registry)?;
    let package_spec =
        load_task_package_spec_for_workflow(registered_profile)?.ok_or_else(|| {
            ApiError::ConfigError(format!(
                "Workflow '{}' does not have a task package route",
                registered_profile.profile.workflow_id
            ))
        })?;

    let prepared = prepare_registered_workflow_task_run(
        api,
        workspace_root,
        registered_profile,
        &WorkflowPackageTriggerRequest {
            package_id: package_spec.package_id,
            workflow_id: registered_profile.profile.workflow_id.clone(),
            node_id: Some(request.node_id),
            path: None,
            agent_id: request.agent_id.clone(),
            provider: request.provider.clone(),
            frame_type: request.frame_type.clone(),
            force: request.force,
            session_id: event_context.map(|ctx| ctx.session_id.clone()),
        },
        &catalog,
    )?;
    let mut executor = TaskExecutor::new(
        prepared.compiled_task,
        prepared.init_payload,
        format!("task_repo::{thread_id}"),
    )?;
    let task_summary = match execute_task_to_completion(
        api,
        &mut executor,
        &catalog,
        &registry,
        event_context,
        Some(&WorkflowTaskTelemetry {
            workflow_id: registered_profile.profile.workflow_id.clone(),
            thread_id: thread_id.to_string(),
            agent_id: request.agent_id.clone(),
            provider_name: request.provider.provider_name.clone(),
            frame_type: request.frame_type.clone(),
            plan_id: request.plan_id.clone(),
            level_index: request.level_index,
            turn_seq_by_id: registered_profile
                .profile
                .ordered_turns()
                .into_iter()
                .map(|turn| (turn.turn_id, turn.seq))
                .collect(),
        }),
    )
    .await
    {
        Ok(summary) => summary,
        Err(err) => {
            state_store.upsert_thread(&WorkflowThreadRecord {
                thread_id: thread_id.to_string(),
                workflow_id: registered_profile.profile.workflow_id.clone(),
                node_id: hex::encode(request.node_id),
                frame_type: request.frame_type.clone(),
                status: WorkflowThreadStatus::Failed,
                next_turn_seq: 1,
                updated_at_ms: now_millis(),
            })?;
            return Err(err);
        }
    };

    let final_frame_id = api.get_head(&request.node_id, &request.frame_type)?;
    state_store.upsert_thread(&WorkflowThreadRecord {
        thread_id: thread_id.to_string(),
        workflow_id: registered_profile.profile.workflow_id.clone(),
        node_id: hex::encode(request.node_id),
        frame_type: request.frame_type.clone(),
        status: WorkflowThreadStatus::Completed,
        next_turn_seq: final_turn_seq.saturating_add(1),
        updated_at_ms: now_millis(),
    })?;

    emit_workflow_target_event(
        event_context,
        "workflow_target_completed",
        WorkflowTargetEventData {
            workflow_id: registered_profile.profile.workflow_id.clone(),
            thread_id: thread_id.to_string(),
            node_id: hex::encode(request.node_id),
            path: target_path.to_string(),
            agent_id: request.agent_id.clone(),
            provider_name: request.provider.provider_name.clone(),
            frame_type: request.frame_type.clone(),
            plan_id: request.plan_id.clone(),
            level_index: request.level_index,
            final_frame_id: final_frame_id.map(hex::encode),
            turns_completed: Some(task_summary.completed_instances / 3),
            reused_existing_head: Some(false),
        },
    );

    Ok(WorkflowExecutionSummary {
        workflow_id: registered_profile.profile.workflow_id.clone(),
        thread_id: thread_id.to_string(),
        turns_completed: task_summary.completed_instances / 3,
        final_frame_id,
    })
}

fn register_task_path_capabilities(
    catalog: &mut CapabilityCatalog,
    registry: &mut CapabilityExecutorRegistry,
) -> Result<(), ApiError> {
    registry.register(
        catalog,
        crate::workspace::capability::WorkspaceResolveNodeIdCapability,
    )?;
    registry.register(
        catalog,
        crate::workspace::capability::WorkspaceFilterFrameHeadPublishCapability,
    )?;
    registry.register(
        catalog,
        crate::workspace::capability::WorkspaceWriteFrameHeadCapability,
    )?;
    registry.register(
        catalog,
        crate::merkle_traversal::capability::MerkleTraversalCapability,
    )?;
    registry.register(
        catalog,
        crate::context::capability::ContextGeneratePrepareCapability,
    )?;
    registry.register(
        catalog,
        crate::provider::capability::ProviderExecuteChatCapability,
    )?;
    registry.register(
        catalog,
        crate::context::capability::ContextGenerateFinalizeCapability,
    )?;
    Ok(())
}

fn emit_workflow_target_event(
    event_context: Option<&QueueEventContext>,
    event_type: &str,
    payload: WorkflowTargetEventData,
) {
    if let Some(ctx) = event_context {
        ctx.progress
            .emit_event_best_effort(&ctx.session_id, event_type, json!(payload));
    }
}

fn emit_workflow_turn_event(
    event_context: Option<&QueueEventContext>,
    event_type: &str,
    payload: WorkflowTurnEventData,
) {
    if let Some(ctx) = event_context {
        let envelope = match event_type {
            "execution.workflow.turn_started" => workflow_turn_started_envelope(
                &ctx.session_id,
                ExecutionWorkflowTurnEventData::from(payload),
            ),
            "execution.workflow.turn_completed" => workflow_turn_completed_envelope(
                &ctx.session_id,
                ExecutionWorkflowTurnEventData::from(payload),
            ),
            "execution.workflow.turn_failed" => workflow_turn_failed_envelope(
                &ctx.session_id,
                ExecutionWorkflowTurnEventData::from(payload),
            ),
            _ => return,
        };
        ctx.progress.emit_envelope_best_effort(envelope);
    }
}

fn emit_workflow_force_reset_event(
    event_context: Option<&QueueEventContext>,
    event_type: &str,
    payload: WorkflowForceResetEventData,
) {
    if let Some(ctx) = event_context {
        ctx.progress
            .emit_event_best_effort(&ctx.session_id, event_type, json!(payload));
    }
}

fn emit_metadata_validation_event(
    event_context: Option<&QueueEventContext>,
    event_type: &str,
    payload: FrameMetadataValidationEventData,
) {
    if let Some(ctx) = event_context {
        ctx.progress
            .emit_event_best_effort(&ctx.session_id, event_type, json!(payload));
    }
}

fn build_thread_id(profile: &WorkflowProfile, node_id: NodeID, frame_type: &str) -> String {
    let payload = format!(
        "{}:{}:{}",
        profile.workflow_id,
        hex::encode(node_id),
        frame_type
    );
    let digest = blake3::hash(payload.as_bytes()).to_hex().to_string();
    format!("thread-{}", &digest[..16])
}

fn workflow_turn_frame_type(
    requested_frame_type: &str,
    turn: &WorkflowTurn,
    prompt_link_id: &str,
    is_final_turn: bool,
) -> String {
    if is_final_turn {
        return requested_frame_type.to_string();
    }

    format!(
        "{}--workflow-turn-{}-{}",
        requested_frame_type, turn.seq, prompt_link_id
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::profile::{
        WorkflowArtifactPolicy, WorkflowFailurePolicy, WorkflowGate, WorkflowProfile,
        WorkflowThreadPolicy,
    };

    fn test_profile() -> WorkflowProfile {
        WorkflowProfile {
            workflow_id: "docs_writer_thread_v1".to_string(),
            version: 1,
            title: "Docs Writer".to_string(),
            description: "Test workflow".to_string(),
            thread_policy: WorkflowThreadPolicy {
                start_conditions: serde_json::Value::Null,
                dedupe_key_fields: vec!["workflow_id".to_string()],
                max_turn_retries: 1,
            },
            turns: vec![],
            gates: vec![WorkflowGate {
                gate_id: "style_gate".to_string(),
                gate_type: "no_semantic_drift".to_string(),
                required_fields: vec![],
                rules: serde_json::Value::Null,
                fail_on_violation: false,
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
                stop_on_gate_fail: false,
            },
            thread_profile: None,
            target_agent_id: None,
            target_frame_type: None,
            final_artifact_type: None,
        }
    }

    #[test]
    fn thread_id_is_deterministic_for_same_inputs() {
        let profile = test_profile();
        let node_id = crate::types::Hash::from([1u8; 32]);

        let left = build_thread_id(&profile, node_id, "context-docs-writer");
        let right = build_thread_id(&profile, node_id, "context-docs-writer");

        assert_eq!(left, right);
        assert!(left.starts_with("thread-"));
    }

    #[test]
    fn intermediate_turns_use_distinct_frame_types() {
        let turn = WorkflowTurn {
            turn_id: "evidence_gather".to_string(),
            seq: 1,
            title: "Gather Evidence".to_string(),
            prompt_ref: "prompts/docs_writer/evidence_gather.md".to_string(),
            input_refs: vec!["target_context".to_string()],
            output_type: "evidence_map".to_string(),
            gate_id: "evidence_gate".to_string(),
            retry_limit: 1,
            timeout_ms: 60000,
        };

        let frame_type =
            workflow_turn_frame_type("context-docs-writer", &turn, "prompt-link-abc", false);

        assert_eq!(
            frame_type,
            "context-docs-writer--workflow-turn-1-prompt-link-abc"
        );
    }

    #[test]
    fn final_turn_uses_requested_frame_type() {
        let turn = WorkflowTurn {
            turn_id: "style_refine".to_string(),
            seq: 4,
            title: "Refine Style".to_string(),
            prompt_ref: "prompts/docs_writer/style_refine.md".to_string(),
            input_refs: vec!["readme_struct".to_string()],
            output_type: "readme_final".to_string(),
            gate_id: "style_gate".to_string(),
            retry_limit: 1,
            timeout_ms: 60000,
        };

        let frame_type =
            workflow_turn_frame_type("context-docs-writer", &turn, "prompt-link-ignored", true);

        assert_eq!(frame_type, "context-docs-writer");
    }
}
