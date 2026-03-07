use crate::api::ContextApi;
use crate::context::generation::contracts::{
    GeneratedMetadataBuilder, GenerationOrchestrationRequest,
};
use crate::error::ApiError;
use crate::metadata::frame_types::FrameMetadata;
use crate::metadata::frame_write_contract::GeneratedFrameMetadataInput;
use crate::metadata::frame_write_contract::{
    validate_frame_metadata, FrameMetadataValidationInput,
};
use crate::metadata::owned_frame_metadata_keys::{
    KEY_CONTEXT_DIGEST, KEY_PROMPT_DIGEST, KEY_PROMPT_LINK_ID,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviousMetadataSnapshot {
    pub frame_id: Option<String>,
    pub prompt_digest: Option<String>,
    pub context_digest: Option<String>,
    pub prompt_link_id: Option<String>,
}

pub fn load_previous_metadata_snapshot(
    api: &ContextApi,
    request: &GenerationOrchestrationRequest,
) -> Result<PreviousMetadataSnapshot, ApiError> {
    let Some(previous_frame_id) = api.get_head(&request.node_id, &request.frame_type)? else {
        return Ok(PreviousMetadataSnapshot {
            frame_id: None,
            prompt_digest: None,
            context_digest: None,
            prompt_link_id: None,
        });
    };

    let previous_metadata = api
        .frame_storage()
        .get(&previous_frame_id)
        .map_err(ApiError::from)?
        .map(|frame| frame.metadata);

    Ok(PreviousMetadataSnapshot {
        frame_id: Some(hex::encode(previous_frame_id)),
        prompt_digest: previous_metadata
            .as_ref()
            .and_then(|metadata| metadata.get(KEY_PROMPT_DIGEST).cloned()),
        context_digest: previous_metadata
            .as_ref()
            .and_then(|metadata| metadata.get(KEY_CONTEXT_DIGEST).cloned()),
        prompt_link_id: previous_metadata
            .as_ref()
            .and_then(|metadata| metadata.get(KEY_PROMPT_LINK_ID).cloned()),
    })
}

pub fn build_and_validate_generated_metadata(
    api: &ContextApi,
    request: &GenerationOrchestrationRequest,
    input: &GeneratedFrameMetadataInput,
    metadata_builder: &GeneratedMetadataBuilder,
) -> Result<FrameMetadata, ApiError> {
    let generated_metadata = metadata_builder(input);
    let previous_metadata =
        if let Some(previous_frame_id) = api.get_head(&request.node_id, &request.frame_type)? {
            api.frame_storage()
                .get(&previous_frame_id)
                .map_err(ApiError::from)?
                .map(|frame| frame.metadata)
        } else {
            None
        };

    validate_frame_metadata(FrameMetadataValidationInput {
        metadata: &generated_metadata,
        actor_agent_id: &request.agent_id,
        previous_metadata: previous_metadata.as_ref(),
    })?;
    Ok(generated_metadata)
}
