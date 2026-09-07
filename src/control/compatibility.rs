use crate::api::ContextApi;
use crate::context::generation::contracts::{
    GeneratedMetadataBuilder, GenerationOrchestrationRequest,
};
use crate::context::generation::orchestration::execute_generation_request;
use crate::context::queue::{GenerationRequest, QueueEventContext};
use crate::error::ApiError;
use crate::types::FrameID;

pub async fn execute_target_request(
    request: &GenerationRequest,
    api: &ContextApi,
    event_context: Option<&QueueEventContext>,
    metadata_builder: &GeneratedMetadataBuilder,
) -> Result<FrameID, ApiError> {
    request.program().validate_execution()?;

    let orchestration_request = GenerationOrchestrationRequest {
        request_id: request.request_id.as_u64(),
        node_id: request.node_id(),
        agent_id: request.agent_id().to_string(),
        provider: request.provider().clone(),
        frame_type: request.frame_type().to_string(),
        retry_count: request.retry_count,
        force: request.options.force,
    };
    execute_generation_request(&orchestration_request, api, metadata_builder, event_context).await
}
