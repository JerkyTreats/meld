use crate::agent::profile::prompt_contract::PromptContract;
use crate::api::ContextApi;
use crate::context::frame::{Basis, Frame};
use crate::context::generation::contracts::{
    GeneratedMetadataBuilder, GenerationOrchestrationRequest,
};
use crate::context::generation::metadata_construction::{
    build_and_validate_generated_metadata, load_previous_metadata_snapshot,
};
use crate::context::generation::prompt_collection::build_prompt_messages;
use crate::context::generation::provider_execution::{
    execute_completion, prepare_provider_for_request,
};
use crate::context::queue::QueueEventContext;
use crate::error::ApiError;
use crate::prompt_context::{prepare_generated_lineage, PromptContextLineageInput};
use crate::telemetry::{FrameMetadataValidationEventData, PromptContextLineageEventData};
use crate::types::FrameID;
use serde_json::json;
use tracing::{debug, info};

pub async fn execute_generation_request(
    request: &GenerationOrchestrationRequest,
    api: &ContextApi,
    metadata_builder: &GeneratedMetadataBuilder,
    event_context: Option<&QueueEventContext>,
) -> Result<FrameID, ApiError> {
    debug!(
        request_id = request.request_id,
        node_id = %hex::encode(request.node_id),
        agent_id = %request.agent_id,
        attempt = request.retry_count + 1,
        "Processing generation request"
    );

    if !request.force {
        if let Some(existing_head) = api.get_head(&request.node_id, &request.frame_type)? {
            return Ok(existing_head);
        }
    }

    let agent = api.get_agent(&request.agent_id)?;
    let node_record = api
        .node_store()
        .get(&request.node_id)
        .map_err(ApiError::from)?
        .ok_or(ApiError::NodeNotFound(request.node_id))?;

    let prompt_contract = PromptContract::from_agent(&agent)?;
    let prompt_output = build_prompt_messages(api, request, &node_record, &prompt_contract)?;

    let provider_preparation = prepare_provider_for_request(api, request)?;

    let prepared_lineage = prepare_generated_lineage(
        api.prompt_context_storage(),
        &PromptContextLineageInput {
            system_prompt: prompt_output.system_prompt.clone(),
            user_prompt_template: prompt_output.user_prompt_template.clone(),
            rendered_prompt: prompt_output.rendered_prompt.clone(),
            context_payload: prompt_output.context_payload.clone(),
        },
        &request.agent_id,
        &request.provider.provider_name,
        provider_preparation.client.model_name(),
        &provider_preparation.provider_type,
    )?;
    if let Some(ctx) = event_context {
        let lineage_event = PromptContextLineageEventData {
            node_id: hex::encode(request.node_id),
            agent_id: request.agent_id.clone(),
            provider_name: request.provider.provider_name.clone(),
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

    if request.force {
        api.tombstone_head(request.node_id, &request.frame_type)?;
    }

    let previous_metadata = load_previous_metadata_snapshot(api, request)?;
    emit_metadata_validation_event(
        event_context,
        "frame_metadata_validation_started",
        FrameMetadataValidationEventData {
            node_id: hex::encode(request.node_id),
            path: node_record.path.to_string_lossy().to_string(),
            agent_id: request.agent_id.clone(),
            provider_name: request.provider.provider_name.clone(),
            frame_type: request.frame_type.clone(),
            prompt_digest: prepared_lineage.metadata_input.prompt_digest.clone(),
            context_digest: prepared_lineage.metadata_input.context_digest.clone(),
            prompt_link_id: prepared_lineage.metadata_input.prompt_link_id.clone(),
            previous_frame_id: previous_metadata.frame_id.clone(),
            previous_prompt_digest: previous_metadata.prompt_digest.clone(),
            previous_context_digest: previous_metadata.context_digest.clone(),
            previous_prompt_link_id: previous_metadata.prompt_link_id.clone(),
            workflow_id: None,
            thread_id: None,
            turn_id: None,
            turn_seq: None,
            attempt: Some(request.retry_count + 1),
            plan_id: None,
            level_index: None,
            error: None,
        },
    );

    let generated_metadata = match build_and_validate_generated_metadata(
        api,
        request,
        &prepared_lineage.metadata_input,
        metadata_builder,
    ) {
        Ok(metadata) => {
            emit_metadata_validation_event(
                event_context,
                "frame_metadata_validation_succeeded",
                FrameMetadataValidationEventData {
                    node_id: hex::encode(request.node_id),
                    path: node_record.path.to_string_lossy().to_string(),
                    agent_id: request.agent_id.clone(),
                    provider_name: request.provider.provider_name.clone(),
                    frame_type: request.frame_type.clone(),
                    prompt_digest: prepared_lineage.metadata_input.prompt_digest.clone(),
                    context_digest: prepared_lineage.metadata_input.context_digest.clone(),
                    prompt_link_id: prepared_lineage.metadata_input.prompt_link_id.clone(),
                    previous_frame_id: previous_metadata.frame_id.clone(),
                    previous_prompt_digest: previous_metadata.prompt_digest.clone(),
                    previous_context_digest: previous_metadata.context_digest.clone(),
                    previous_prompt_link_id: previous_metadata.prompt_link_id.clone(),
                    workflow_id: None,
                    thread_id: None,
                    turn_id: None,
                    turn_seq: None,
                    attempt: Some(request.retry_count + 1),
                    plan_id: None,
                    level_index: None,
                    error: None,
                },
            );
            metadata
        }
        Err(err) => {
            emit_metadata_validation_event(
                event_context,
                "frame_metadata_validation_failed",
                FrameMetadataValidationEventData {
                    node_id: hex::encode(request.node_id),
                    path: node_record.path.to_string_lossy().to_string(),
                    agent_id: request.agent_id.clone(),
                    provider_name: request.provider.provider_name.clone(),
                    frame_type: request.frame_type.clone(),
                    prompt_digest: prepared_lineage.metadata_input.prompt_digest.clone(),
                    context_digest: prepared_lineage.metadata_input.context_digest.clone(),
                    prompt_link_id: prepared_lineage.metadata_input.prompt_link_id.clone(),
                    previous_frame_id: previous_metadata.frame_id.clone(),
                    previous_prompt_digest: previous_metadata.prompt_digest.clone(),
                    previous_context_digest: previous_metadata.context_digest.clone(),
                    previous_prompt_link_id: previous_metadata.prompt_link_id.clone(),
                    workflow_id: None,
                    thread_id: None,
                    turn_id: None,
                    turn_seq: None,
                    attempt: Some(request.retry_count + 1),
                    plan_id: None,
                    level_index: None,
                    error: Some(err.to_string()),
                },
            );
            return Err(err);
        }
    };

    let response = execute_completion(
        request,
        &provider_preparation,
        prompt_output.messages,
        event_context,
    )
    .await?;

    let frame = Frame::new(
        Basis::Node(request.node_id),
        response.content.into_bytes(),
        request.frame_type.clone(),
        request.agent_id.clone(),
        generated_metadata,
    )?;

    let frame_id = api.put_frame(request.node_id, frame, request.agent_id.clone())?;

    info!(
        request_id = request.request_id,
        node_id = %hex::encode(request.node_id),
        agent_id = %request.agent_id,
        frame_id = %hex::encode(frame_id),
        "Frame generation completed"
    );

    Ok(frame_id)
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
