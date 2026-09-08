use crate::api::ContextApi;
use crate::cli::{format_context_json_output, format_context_text_output, ContextCommands};
use crate::context::query::get_node_for_cli;
use crate::error::ApiError;
use crate::telemetry::ProgressRuntime;
use crate::workflow::WorkflowRegistry;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

pub fn handle_cli_command(
    api: Arc<ContextApi>,
    workspace_root: &Path,
    workflow_config: &crate::config::WorkflowConfig,
    progress: &Arc<ProgressRuntime>,
    command: &ContextCommands,
    session_id: &str,
) -> Result<String, ApiError> {
    match command {
        ContextCommands::Generate { .. } | ContextCommands::Regenerate { .. } => Err(ApiError::ConfigError(
            "Direct Context generation is retired. Use a runtime-loaded theory with a Goal and authorized Context generation capabilities. A replacement Context generation package is not yet provided.".into(),
        )),
        ContextCommands::Get {
            node,
            path,
            agent,
            frame_type,
            max_frames,
            ordering,
            combine,
            separator,
            format,
            include_metadata,
            include_deleted,
        } => {
            let effective_frame_type = resolve_context_get_frame_type(
                &api,
                workflow_config,
                agent.as_deref(),
                frame_type.as_deref(),
            )?;
            let context = get_node_for_cli(
                &api,
                workspace_root,
                node.as_deref(),
                path.as_deref(),
                agent.as_deref(),
                effective_frame_type.as_deref(),
                *max_frames,
                ordering,
                *include_deleted,
            )?;
            let formatted = match format.as_str() {
                "text" => format_context_text_output(
                    &context.context,
                    &context.warnings,
                    *include_metadata,
                    *combine,
                    separator,
                    *include_deleted,
                ),
                "json" => format_context_json_output(
                    &context.context,
                    &context.warnings,
                    *include_metadata,
                    *include_deleted,
                ),
                _ => Err(ApiError::ConfigError(format!(
                    "Invalid format: '{}'. Must be 'text' or 'json'.",
                    format
                ))),
            }?;
            progress.emit_event_best_effort(
                session_id,
                "context_read_summary",
                json!({
                    "node_id": hex::encode(context.context.node_id()),
                    "frame_count": context.context.frames.len(),
                    "max_frames": max_frames,
                    "ordering": ordering,
                    "combine": combine,
                    "format": format
                }),
            );
            Ok(formatted)
        }
    }
}

fn resolve_context_get_frame_type(
    api: &ContextApi,
    workflow_config: &crate::config::WorkflowConfig,
    agent_id: Option<&str>,
    frame_type: Option<&str>,
) -> Result<Option<String>, ApiError> {
    if let Some(frame_type) = frame_type {
        return Ok(Some(frame_type.to_string()));
    }

    let Some(agent_id) = agent_id else {
        return Ok(None);
    };

    let agent = api.get_agent(agent_id)?;
    let Some(workflow_id) = agent.workflow_binding() else {
        return Ok(None);
    };

    let workflow_registry = WorkflowRegistry::load(workflow_config)?;
    let registered_workflow = workflow_registry.get(workflow_id).ok_or_else(|| {
        ApiError::ConfigError(format!(
            "Agent '{}' references unknown workflow_id '{}'",
            agent_id, workflow_id
        ))
    })?;

    Ok(Some(
        registered_workflow
            .profile
            .target_frame_type
            .clone()
            .unwrap_or_else(|| format!("context-{}", agent_id)),
    ))
}
