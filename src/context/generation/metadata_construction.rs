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
