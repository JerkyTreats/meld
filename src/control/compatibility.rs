use crate::api::ContextApi;
use crate::context::generation::contracts::{
    GeneratedMetadataBuilder, GenerationOrchestrationRequest,
};
use crate::context::generation::orchestration::execute_generation_request;
use crate::context::generation::{TargetExecutionProgramKind, TargetExecutionRequest};
use crate::context::queue::{GenerationRequest, QueueEventContext};
use crate::error::ApiError;
use crate::types::FrameID;
use crate::workflow::{build_target_execution_request, execute_workflow_target_async};

pub async fn execute_target_request(
    request: &GenerationRequest,
    api: &ContextApi,
    event_context: Option<&QueueEventContext>,
    metadata_builder: &GeneratedMetadataBuilder,
) -> Result<FrameID, ApiError> {
    if request.program.kind == TargetExecutionProgramKind::Workflow {
        let workspace_root = api.workspace_root().ok_or_else(|| {
            ApiError::ConfigError(
                "Workflow target execution requires workspace root context".to_string(),
            )
        })?;
        let target_request = build_compatibility_target_request(api, request, event_context)?;
        let target_result =
            execute_workflow_target_async(api, workspace_root, &target_request, event_context)
                .await?;
        return Ok(target_result.final_frame_id);
    }

    let orchestration_request = GenerationOrchestrationRequest {
        request_id: request.request_id.as_u64(),
        node_id: request.node_id,
        agent_id: request.agent_id.clone(),
        provider: request.provider.clone(),
        frame_type: request.frame_type.clone(),
        retry_count: request.retry_count,
        force: request.options.force,
    };
    execute_generation_request(&orchestration_request, api, metadata_builder, event_context).await
}

fn build_compatibility_target_request(
    api: &ContextApi,
    request: &GenerationRequest,
    event_context: Option<&QueueEventContext>,
) -> Result<TargetExecutionRequest, ApiError> {
    build_target_execution_request(
        api,
        request.node_id,
        request.agent_id.clone(),
        request.provider.clone(),
        request.frame_type.clone(),
        request.options.force,
        request.program.clone(),
        request.options.plan_id.clone(),
        event_context.map(|ctx| ctx.session_id.clone()),
        None,
    )
}
