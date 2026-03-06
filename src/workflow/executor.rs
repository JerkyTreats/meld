//! Workflow runtime executor for bound agent turn workflows.

use crate::agent::profile::prompt_contract::PromptContract;
use crate::api::ContextApi;
use crate::context::frame::{Basis, Frame};
use crate::context::generation::contracts::{
    GeneratedMetadataBuilder, GenerationOrchestrationRequest,
};
use crate::context::generation::metadata_construction::build_and_validate_generated_metadata;
use crate::context::generation::provider_execution::{execute_completion, prepare_provider};
use crate::error::ApiError;
use crate::metadata::frame_write_contract::build_generated_metadata;
use crate::prompt_context::{prepare_generated_lineage, PromptContextLineageInput};
use crate::types::{FrameID, NodeID};
use crate::workflow::gates::evaluate_gate;
use crate::workflow::profile::WorkflowProfile;
use crate::workflow::record_contracts::GateOutcome;
use crate::workflow::registry::RegisteredWorkflowProfile;
use crate::workflow::resolver::{render_turn_prompt, resolve_prompt_template, resolve_turn_inputs};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct WorkflowExecutionRequest {
    pub node_id: NodeID,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: String,
    pub force: bool,
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
) -> Result<WorkflowExecutionSummary, ApiError> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|err| ApiError::ProviderError(format!("Failed to create runtime: {}", err)))?;

    rt.block_on(async move {
        execute_registered_workflow_async(api, workspace_root, registered_profile, request).await
    })
}

async fn execute_registered_workflow_async(
    api: &ContextApi,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowExecutionRequest,
) -> Result<WorkflowExecutionSummary, ApiError> {
    let profile = &registered_profile.profile;
    let thread_id = build_thread_id(profile, request.node_id, &request.frame_type);

    if !request.force {
        if let Some(existing_head) = api.get_head(&request.node_id, &request.frame_type)? {
            return Ok(WorkflowExecutionSummary {
                workflow_id: profile.workflow_id.clone(),
                thread_id,
                turns_completed: 0,
                final_frame_id: Some(existing_head),
            });
        }
    }

    let agent = api.get_agent(&request.agent_id)?;
    let prompt_contract = PromptContract::from_agent(&agent)?;

    let provider_preparation = prepare_provider(api, &request.provider_name)?;
    let metadata_builder: &GeneratedMetadataBuilder = &build_generated_metadata;

    let mut turn_outputs: HashMap<String, String> = HashMap::new();
    let mut completed_turns = 0usize;
    let mut final_frame_id: Option<FrameID> = None;

    for turn in profile.ordered_turns() {
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

            let orchestration_request = GenerationOrchestrationRequest {
                request_id: ((completed_turns as u64) + 1) * 1000 + attempt as u64,
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
                None,
            )
            .await
            {
                Ok(response) => response,
                Err(err) => {
                    last_error = Some(err.clone());
                    if attempt < turn.retry_limit {
                        continue;
                    }
                    return Err(err);
                }
            };

            let gate_result = evaluate_gate(gate, &response.content);
            if gate_result.outcome == GateOutcome::Fail {
                let gate_error = ApiError::GenerationFailed(format!(
                    "Workflow '{}' turn '{}' failed gate '{}': {}",
                    profile.workflow_id,
                    turn.turn_id,
                    gate.gate_id,
                    gate_result.reasons.join(" | ")
                ));
                last_error = Some(gate_error.clone());
                if attempt < turn.retry_limit {
                    continue;
                }
                if gate.fail_on_violation || profile.failure_policy.stop_on_gate_fail {
                    return Err(gate_error);
                }
            }

            let frame = Frame::new(
                Basis::Node(request.node_id),
                response.content.as_bytes().to_vec(),
                request.frame_type.clone(),
                request.agent_id.clone(),
                generated_metadata,
            )?;
            let frame_id = api.put_frame(request.node_id, frame, request.agent_id.clone())?;

            turn_outputs.insert(turn.output_type.clone(), response.content.clone());
            turn_outputs.insert(turn.turn_id.clone(), response.content.clone());
            completed_turns += 1;
            final_frame_id = Some(frame_id);
            success = true;
            break;
        }

        if !success {
            return Err(last_error.unwrap_or_else(|| {
                ApiError::GenerationFailed(format!(
                    "Workflow '{}' turn '{}' failed with no retryable error",
                    profile.workflow_id, turn.turn_id
                ))
            }));
        }
    }

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
