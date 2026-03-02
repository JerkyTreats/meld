use crate::context::generation::contracts::{
    GeneratedMetadataBuilder, GenerationOrchestrationRequest,
};
use crate::error::ApiError;
use crate::metadata::frame_types::FrameMetadata;
use crate::metadata::frame_write_contract::validate_frame_metadata;

pub fn build_and_validate_generated_metadata(
    request: &GenerationOrchestrationRequest,
    model: &str,
    provider_type: &str,
    user_prompt: &str,
    metadata_builder: &GeneratedMetadataBuilder,
) -> Result<FrameMetadata, ApiError> {
    let generated_metadata = metadata_builder(
        &request.agent_id,
        &request.provider_name,
        model,
        provider_type,
        user_prompt,
    );
    validate_frame_metadata(&generated_metadata, &request.agent_id)?;
    Ok(generated_metadata)
}
