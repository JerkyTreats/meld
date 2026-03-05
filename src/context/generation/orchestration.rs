use crate::agent::profile::prompt_contract::PromptContract;
use crate::api::ContextApi;
use crate::context::frame::{Basis, Frame};
use crate::context::generation::contracts::{
    GeneratedMetadataBuilder, GenerationOrchestrationRequest,
};
use crate::context::generation::metadata_construction::build_and_validate_generated_metadata;
use crate::context::generation::prompt_collection::build_prompt_messages;
use crate::context::generation::provider_execution::{execute_completion, prepare_provider};
use crate::context::queue::QueueEventContext;
use crate::error::ApiError;
use crate::metadata::frame_write_contract::GeneratedFrameMetadataInput;
use crate::metadata::prompt_link_contract::PromptLinkContractV1;
use crate::prompt_context::{persist_prompt_context_lineage, PromptContextLineageInput};
use crate::telemetry::PromptContextLineageEventData;
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
        .ok_or_else(|| ApiError::NodeNotFound(request.node_id))?;

    let prompt_contract = PromptContract::from_agent(&agent)?;
    let prompt_output = build_prompt_messages(api, request, &node_record, &prompt_contract)?;

    let provider_preparation = prepare_provider(api, &request.provider_name)?;

    let lineage = persist_prompt_context_lineage(
        api.prompt_context_storage(),
        &PromptContextLineageInput {
            system_prompt: prompt_output.system_prompt.clone(),
            user_prompt_template: prompt_output.user_prompt_template.clone(),
            rendered_prompt: prompt_output.rendered_prompt.clone(),
            context_payload: prompt_output.context_payload.clone(),
        },
    )?;
    let prompt_link_contract = PromptLinkContractV1::from_lineage(&lineage);
    prompt_link_contract.validate()?;
    if let Some(ctx) = event_context {
        let lineage_event = PromptContextLineageEventData {
            node_id: hex::encode(request.node_id),
            agent_id: request.agent_id.clone(),
            provider_name: request.provider_name.clone(),
            frame_type: request.frame_type.clone(),
            prompt_link_id: prompt_link_contract.prompt_link_id.clone(),
            prompt_digest: prompt_link_contract.prompt_digest.clone(),
            context_digest: prompt_link_contract.context_digest.clone(),
            system_prompt_artifact_id: prompt_link_contract.system_prompt_artifact_id.clone(),
            user_prompt_template_artifact_id: prompt_link_contract
                .user_prompt_template_artifact_id
                .clone(),
            rendered_prompt_artifact_id: prompt_link_contract.rendered_prompt_artifact_id.clone(),
            context_artifact_id: prompt_link_contract.context_artifact_id.clone(),
            lineage_failure_policy: "deterministic_orphan_keep".to_string(),
        };
        ctx.progress.emit_event_best_effort(
            &ctx.session_id,
            "prompt_context_lineage_prepared",
            json!(lineage_event),
        );
    }

    let metadata_input = GeneratedFrameMetadataInput {
        agent_id: request.agent_id.clone(),
        provider: request.provider_name.clone(),
        model: provider_preparation.client.model_name().to_string(),
        provider_type: provider_preparation.provider_type.clone(),
        prompt_digest: prompt_link_contract.prompt_digest.clone(),
        context_digest: prompt_link_contract.context_digest.clone(),
        prompt_link_id: prompt_link_contract.prompt_link_id.clone(),
    };

    let generated_metadata =
        build_and_validate_generated_metadata(api, request, &metadata_input, metadata_builder)?;

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
