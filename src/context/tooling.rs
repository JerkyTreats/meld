use crate::api::ContextApi;
use crate::cli::{
    format_context_json_output, format_context_text_output, parse_provider_additional_json_file,
    ContextCommands,
};
use crate::context::generation::run::{run_generate, GenerateRequest};
use crate::context::query::get_node_for_cli;
use crate::error::ApiError;
use crate::provider::{ProviderExecutionBinding, ProviderRuntimeOverrides};
use crate::telemetry::ProgressRuntime;
use crate::workflow::registry::WorkflowRegistry;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn handle_cli_command(
    api: Arc<ContextApi>,
    workspace_root: &Path,
    workflow_registry: &WorkflowRegistry,
    progress: &Arc<ProgressRuntime>,
    command: &ContextCommands,
    session_id: &str,
) -> Result<String, ApiError> {
    match command {
        ContextCommands::Generate {
            node,
            path,
            path_positional,
            agent,
            provider,
            workflow_id,
            provider_model,
            provider_additional_json_file,
            frame_type,
            force,
            no_recursive,
        } => {
            let path_merged = path.as_ref().or(path_positional.as_ref());
            let provider_binding = build_generate_provider_binding(
                provider.as_deref(),
                provider_model.as_deref(),
                provider_additional_json_file.as_ref(),
            )?;
            let request = GenerateRequest {
                node: node.clone(),
                path: path_merged.cloned(),
                agent: agent.clone(),
                provider: provider_binding,
                workflow_id: workflow_id.clone(),
                frame_type: frame_type.clone(),
                force: *force,
                no_recursive: *no_recursive,
            };
            run_generate(
                api,
                workspace_root,
                Some(Arc::clone(progress)),
                Some(session_id),
                &request,
            )
        }
        ContextCommands::Regenerate {
            node,
            path,
            path_positional,
            agent,
            provider,
            workflow_id,
            provider_model,
            provider_additional_json_file,
            frame_type,
            recursive,
        } => {
            let path_merged = path.as_ref().or(path_positional.as_ref());
            let provider_binding = build_generate_provider_binding(
                provider.as_deref(),
                provider_model.as_deref(),
                provider_additional_json_file.as_ref(),
            )?;
            let request = GenerateRequest {
                node: node.clone(),
                path: path_merged.cloned(),
                agent: agent.clone(),
                provider: provider_binding,
                workflow_id: workflow_id.clone(),
                frame_type: frame_type.clone(),
                force: true,
                no_recursive: !*recursive,
            };
            run_generate(
                api,
                workspace_root,
                Some(Arc::clone(progress)),
                Some(session_id),
                &request,
            )
        }
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
                workflow_registry,
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
                    "node_id": hex::encode(context.context.node_id),
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
    workflow_registry: &WorkflowRegistry,
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

fn build_generate_provider_binding(
    provider_name: Option<&str>,
    provider_model: Option<&str>,
    provider_additional_json_file: Option<&PathBuf>,
) -> Result<ProviderExecutionBinding, ApiError> {
    let provider_name = provider_name.ok_or_else(|| {
        ApiError::ProviderNotConfigured(
            "Provider is required. Use `--provider <provider_name>` to specify a provider. Use `meld provider list` to see available providers.".to_string(),
        )
    })?;
    let provider_additional_json =
        parse_provider_additional_json_file(provider_additional_json_file)
            .map_err(ApiError::ConfigError)?;
    let provider_runtime_overrides = ProviderRuntimeOverrides::new(
        provider_model.map(str::to_string),
        provider_additional_json.unwrap_or_default(),
    )?;
    ProviderExecutionBinding::new(provider_name, provider_runtime_overrides)
}
