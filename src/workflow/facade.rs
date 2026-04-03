//! Workflow public facade for target scoped execution.

use crate::api::ContextApi;
use crate::config::ConfigLoader;
use crate::context::generation::{
    TargetExecutionProgramKind, TargetExecutionRequest, TargetExecutionResult,
};
use crate::context::queue::QueueEventContext;
use crate::error::ApiError;
use crate::provider::ProviderExecutionBinding;
use crate::store::NodeType;
use crate::workflow::executor::{
    execute_registered_workflow, execute_registered_workflow_async, WorkflowExecutionRequest,
};
use crate::workflow::registry::{RegisteredWorkflowProfile, WorkflowRegistry};
use std::path::Path;

pub fn execute_workflow_target(
    api: &ContextApi,
    workspace_root: &Path,
    request: &TargetExecutionRequest,
    event_context: Option<&QueueEventContext>,
) -> Result<TargetExecutionResult, ApiError> {
    let workflow_id = request.program.workflow_id().ok_or_else(|| {
        ApiError::ConfigError(
            "Workflow target execution requires a workflow backed execution program".to_string(),
        )
    })?;
    let config = ConfigLoader::load(workspace_root)?;
    let registry = WorkflowRegistry::load(&config.workflows)?;
    let registered_profile = registry
        .get(workflow_id)
        .ok_or_else(|| ApiError::ConfigError(format!("Workflow not found: {}", workflow_id)))?;
    execute_registered_workflow_target(
        api,
        workspace_root,
        registered_profile,
        request,
        event_context,
    )
}

pub async fn execute_workflow_target_async(
    api: &ContextApi,
    workspace_root: &Path,
    request: &TargetExecutionRequest,
    event_context: Option<&QueueEventContext>,
) -> Result<TargetExecutionResult, ApiError> {
    let workflow_id = request.program.workflow_id().ok_or_else(|| {
        ApiError::ConfigError(
            "Workflow target execution requires a workflow backed execution program".to_string(),
        )
    })?;
    let config = ConfigLoader::load(workspace_root)?;
    let registry = WorkflowRegistry::load(&config.workflows)?;
    let registered_profile = registry
        .get(workflow_id)
        .ok_or_else(|| ApiError::ConfigError(format!("Workflow not found: {}", workflow_id)))?;
    execute_registered_workflow_target_async(
        api,
        workspace_root,
        registered_profile,
        request,
        event_context,
    )
    .await
}

pub fn execute_registered_workflow_target(
    api: &ContextApi,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &TargetExecutionRequest,
    event_context: Option<&QueueEventContext>,
) -> Result<TargetExecutionResult, ApiError> {
    if request.program.kind != TargetExecutionProgramKind::Workflow {
        return Err(ApiError::ConfigError(
            "Registered workflow target execution requires workflow program kind".to_string(),
        ));
    }

    let summary = execute_registered_workflow(
        api,
        workspace_root,
        registered_profile,
        &WorkflowExecutionRequest {
            node_id: request.node_id,
            agent_id: request.agent_id.clone(),
            provider: request.provider.clone(),
            frame_type: request.frame_type.clone(),
            force: request.force,
            path: Some(request.path.clone()),
            plan_id: request.plan_id.clone(),
            level_index: request.level_index,
        },
        event_context,
    )?;

    let final_frame_id = summary.final_frame_id.ok_or_else(|| {
        ApiError::GenerationFailed(format!(
            "Workflow '{}' completed without a final frame",
            summary.workflow_id
        ))
    })?;

    Ok(TargetExecutionResult {
        final_frame_id,
        reused_existing_head: summary.turns_completed == 0,
        program: request.program.clone(),
        workflow_id: Some(summary.workflow_id),
        thread_id: Some(summary.thread_id),
        turns_completed: summary.turns_completed,
    })
}

pub async fn execute_registered_workflow_target_async(
    api: &ContextApi,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &TargetExecutionRequest,
    event_context: Option<&QueueEventContext>,
) -> Result<TargetExecutionResult, ApiError> {
    if request.program.kind != TargetExecutionProgramKind::Workflow {
        return Err(ApiError::ConfigError(
            "Registered workflow target execution requires workflow program kind".to_string(),
        ));
    }

    let summary = execute_registered_workflow_async(
        api,
        workspace_root,
        registered_profile,
        &WorkflowExecutionRequest {
            node_id: request.node_id,
            agent_id: request.agent_id.clone(),
            provider: request.provider.clone(),
            frame_type: request.frame_type.clone(),
            force: request.force,
            path: Some(request.path.clone()),
            plan_id: request.plan_id.clone(),
            level_index: request.level_index,
        },
        event_context,
    )
    .await?;

    let final_frame_id = summary.final_frame_id.ok_or_else(|| {
        ApiError::GenerationFailed(format!(
            "Workflow '{}' completed without a final frame",
            summary.workflow_id
        ))
    })?;

    Ok(TargetExecutionResult {
        final_frame_id,
        reused_existing_head: summary.turns_completed == 0,
        program: request.program.clone(),
        workflow_id: Some(summary.workflow_id),
        thread_id: Some(summary.thread_id),
        turns_completed: summary.turns_completed,
    })
}

pub fn build_target_execution_request(
    api: &ContextApi,
    node_id: crate::types::NodeID,
    agent_id: String,
    provider: ProviderExecutionBinding,
    frame_type: String,
    force: bool,
    program: crate::context::generation::TargetExecutionProgram,
    plan_id: Option<String>,
    session_id: Option<String>,
    level_index: Option<usize>,
) -> Result<TargetExecutionRequest, ApiError> {
    let record = api
        .node_store()
        .get(&node_id)
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::NodeNotFound(node_id))?;
    Ok(TargetExecutionRequest {
        node_id,
        path: record.path.to_string_lossy().to_string(),
        node_type: match record.node_type {
            NodeType::File { .. } => crate::context::generation::GenerationNodeType::File,
            NodeType::Directory => crate::context::generation::GenerationNodeType::Directory,
        },
        agent_id,
        provider,
        frame_type,
        force,
        program,
        plan_id,
        session_id,
        level_index,
    })
}
