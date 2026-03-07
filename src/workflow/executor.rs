//! Workflow runtime executor for bound agent turn workflows.

use crate::agent::profile::prompt_contract::PromptContract;
use crate::api::ContextApi;
use crate::context::frame::{Basis, Frame};
use crate::context::generation::contracts::{
    GeneratedMetadataBuilder, GenerationOrchestrationRequest,
};
use crate::context::generation::metadata_construction::build_and_validate_generated_metadata;
use crate::context::generation::provider_execution::{execute_completion, prepare_provider};
use crate::context::queue::QueueEventContext;
use crate::error::ApiError;
use crate::metadata::frame_write_contract::build_generated_metadata;
use crate::prompt_context::{prepare_generated_lineage, PromptContextLineageInput};
use crate::telemetry::{now_millis, PromptContextLineageEventData};
use crate::types::{FrameID, NodeID};
use crate::workflow::gates::evaluate_gate;
use crate::workflow::profile::WorkflowProfile;
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
    pub provider_name: String,
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

    let mut start_seq = 1u32;
    let mut turn_outputs: HashMap<String, String> = HashMap::new();
    let mut completed_turns = 0usize;

    if let Some(existing) = state_store.load_thread(&thread_id)? {
        match existing.status {
            WorkflowThreadStatus::Completed => {
                if !request.force {
                    let head = api.get_head(&request.node_id, &request.frame_type)?;
                    return Ok(WorkflowExecutionSummary {
                        workflow_id: profile.workflow_id.clone(),
                        thread_id,
                        turns_completed: 0,
                        final_frame_id: head,
                    });
                }
            }
            WorkflowThreadStatus::Failed | WorkflowThreadStatus::Running => {
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

    let agent = api.get_agent(&request.agent_id)?;
    let prompt_contract = PromptContract::from_agent(&agent)?;

    let provider_preparation = prepare_provider(api, &request.provider_name)?;
    let metadata_builder: &GeneratedMetadataBuilder = &build_generated_metadata;

    let mut final_frame_id: Option<FrameID> =
        api.get_head(&request.node_id, &request.frame_type)?;

    for turn in profile.ordered_turns() {
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
            workspace_root,
            registered_profile.source_path.as_deref(),
            &turn.prompt_ref,
        )?;
        let rendered_prompt = render_turn_prompt(&prompt_template, &turn, &resolved_inputs);

        let mut attempt = 0usize;
        let mut success = false;
        let mut last_error: Option<ApiError> = None;

        while attempt < turn.retry_limit {
            attempt += 1;

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

            let orchestration_request = GenerationOrchestrationRequest {
                request_id: ((turn.seq as u64) * 1000) + attempt as u64,
                node_id: request.node_id,
                agent_id: request.agent_id.clone(),
                provider_name: request.provider_name.clone(),
                frame_type: request.frame_type.clone(),
                retry_count: attempt.saturating_sub(1),
                force: request.force,
            };

            let prepared_lineage = prepare_generated_lineage(
                api.prompt_context_storage(),
                &PromptContextLineageInput {
                    system_prompt: prompt_contract.system_prompt.clone(),
                    user_prompt_template: prompt_template.clone(),
                    rendered_prompt: rendered_prompt.clone(),
                    context_payload: resolved_inputs.context_payload.clone(),
                },
                &request.agent_id,
                &request.provider_name,
                provider_preparation.client.model_name(),
                &provider_preparation.provider_type,
            )?;

            if let Some(ctx) = event_context {
                let lineage_event = PromptContextLineageEventData {
                    node_id: hex::encode(request.node_id),
                    agent_id: request.agent_id.clone(),
                    provider_name: request.provider_name.clone(),
                    frame_type: request.frame_type.clone(),
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

            let generated_metadata = build_and_validate_generated_metadata(
                api,
                &orchestration_request,
                &prepared_lineage.metadata_input,
                metadata_builder,
            )?;

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

            let frame = Frame::new(
                Basis::Node(request.node_id),
                response.content.as_bytes().to_vec(),
                request.frame_type.clone(),
                request.agent_id.clone(),
                generated_metadata,
            )?;
            let frame_id = api.put_frame(request.node_id, frame, request.agent_id.clone())?;

            let gate_result = evaluate_gate(gate, &response.content);
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
                    frame_id: Some(hex::encode(frame_id)),
                    output_text: Some(response.content.clone()),
                    updated_at_ms: now_millis(),
                })?;
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

    Ok(WorkflowExecutionSummary {
        workflow_id: profile.workflow_id.clone(),
        thread_id,
        turns_completed: completed_turns,
        final_frame_id,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::builtin::builtin_profiles;

    #[test]
    fn thread_id_is_deterministic_for_same_inputs() {
        let profile = builtin_profiles()
            .into_iter()
            .find(|profile| profile.workflow_id == "docs_writer_thread_v1")
            .unwrap();
        let node_id = crate::types::Hash::from([1u8; 32]);

        let left = build_thread_id(&profile, node_id, "context-docs-writer");
        let right = build_thread_id(&profile, node_id, "context-docs-writer");

        assert_eq!(left, right);
        assert!(left.starts_with("thread-"));
    }
}
