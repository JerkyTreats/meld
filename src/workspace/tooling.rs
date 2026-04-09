use crate::agent::registry::AgentRegistry;
use crate::api::ContextApi;
use crate::cli::{
    format_ignore_result, format_list_deleted_result, format_validate_result_text,
    WorkspaceCommands,
};
use crate::config::ConfigLoader;
use crate::error::ApiError;
use crate::ignore;
use crate::telemetry::ProgressRuntime;
use crate::workflow::binding::validate_agent_binding;
use crate::workflow::registry::WorkflowRegistry;
use crate::workspace::{
    format_unified_status_text, format_workspace_status_text, WatchConfig, WatchDaemon,
    WorkspaceCommandService, WorkspaceStatusRequest,
};
use std::path::Path;
use std::sync::Arc;

pub fn handle_scan_command(
    api: &ContextApi,
    workspace_root: &Path,
    progress: &Arc<ProgressRuntime>,
    force: bool,
    session_id: &str,
) -> Result<String, ApiError> {
    progress.emit_event_best_effort(
        session_id,
        "scan_started",
        serde_json::json!({ "force": force }),
    );
    WorkspaceCommandService::scan(api, workspace_root, force, Some(progress), Some(session_id))
}

pub fn handle_cli_command(
    api: &ContextApi,
    workspace_root: &Path,
    store_path: &Path,
    frame_storage_path: &Path,
    command: &WorkspaceCommands,
) -> Result<String, ApiError> {
    match command {
        WorkspaceCommands::Status { format, breakdown } => {
            let registry = api.agent_registry().read();
            let request = WorkspaceStatusRequest {
                workspace_root: workspace_root.to_path_buf(),
                store_path: store_path.to_path_buf(),
                include_breakdown: *breakdown,
            };
            let status = WorkspaceCommandService::status(api, &request, &registry)?;
            if format == "json" {
                serde_json::to_string_pretty(&status).map_err(|e| {
                    ApiError::StorageError(crate::error::StorageError::InvalidPath(e.to_string()))
                })
            } else {
                Ok(format_workspace_status_text(
                    &status,
                    request.include_breakdown,
                ))
            }
        }
        WorkspaceCommands::Validate { format } => {
            let result = WorkspaceCommandService::validate(
                api,
                workspace_root,
                &frame_storage_path.to_path_buf(),
            )?;
            if format == "json" {
                serde_json::to_string_pretty(&result).map_err(|e| {
                    ApiError::StorageError(crate::error::StorageError::InvalidPath(e.to_string()))
                })
            } else {
                Ok(format_validate_result_text(&result))
            }
        }
        WorkspaceCommands::Ignore {
            path,
            dry_run,
            format,
        } => {
            let result =
                WorkspaceCommandService::ignore(workspace_root, path.as_deref(), *dry_run)?;
            format_ignore_result(&result, format.as_str())
        }
        WorkspaceCommands::Delete {
            path,
            node,
            dry_run,
            no_ignore,
        } => WorkspaceCommandService::delete(
            api,
            workspace_root,
            path.as_deref(),
            node.as_deref(),
            *dry_run,
            *no_ignore,
        ),
        WorkspaceCommands::Restore {
            path,
            node,
            dry_run,
        } => WorkspaceCommandService::restore(
            api,
            workspace_root,
            path.as_deref(),
            node.as_deref(),
            *dry_run,
        ),
        WorkspaceCommands::Compact {
            ttl,
            all,
            keep_frames,
            dry_run,
        } => WorkspaceCommandService::compact(api, *ttl, *all, *keep_frames, *dry_run),
        WorkspaceCommands::ListDeleted { older_than, format } => {
            let result = WorkspaceCommandService::list_deleted(api, *older_than)?;
            format_list_deleted_result(&result, format.as_str())
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_status_command(
    api: &ContextApi,
    workspace_root: &Path,
    store_path: &Path,
    format: &str,
    workspace_only: bool,
    agents_only: bool,
    providers_only: bool,
    breakdown: bool,
    test_connectivity: bool,
) -> Result<String, ApiError> {
    let include_all = !workspace_only && !agents_only && !providers_only;
    let include_workspace = include_all || workspace_only;
    let include_agents = include_all || agents_only;
    let include_providers = include_all || providers_only;
    let registry_agent = api.agent_registry().read();
    let registry_provider = api.provider_registry().read();
    let unified = WorkspaceCommandService::unified_status(
        api,
        workspace_root,
        store_path,
        &registry_agent,
        &registry_provider,
        include_workspace,
        include_agents,
        include_providers,
        breakdown,
        test_connectivity,
    )?;

    if format == "json" {
        serde_json::to_string_pretty(&unified).map_err(|e| {
            ApiError::StorageError(crate::error::StorageError::InvalidPath(e.to_string()))
        })
    } else {
        Ok(format_unified_status_text(
            &unified,
            breakdown,
            test_connectivity,
        ))
    }
}

pub fn handle_validate_command(
    api: &ContextApi,
    workspace_root: &Path,
    frame_storage_path: &Path,
) -> Result<String, ApiError> {
    let result =
        WorkspaceCommandService::validate(api, workspace_root, &frame_storage_path.to_path_buf())?;
    Ok(format_validate_result_text(&result))
}

pub fn handle_watch_command(
    api: Arc<ContextApi>,
    workspace_root: &Path,
    config_path: Option<&Path>,
    workflow_registry: &Arc<parking_lot::RwLock<WorkflowRegistry>>,
    progress: &Arc<ProgressRuntime>,
    debounce_ms: u64,
    batch_window_ms: u64,
    session_id: &str,
) -> Result<String, ApiError> {
    let config = load_runtime_config(workspace_root, config_path)?;
    let loaded_workflow_registry = WorkflowRegistry::load(&config.workflows)?;

    {
        let mut registry = api.agent_registry().write();
        registry.load_from_config(&config).map_err(|e| {
            ApiError::ConfigError(format!("Failed to load agents from config: {}", e))
        })?;
        validate_bindings(&registry, &loaded_workflow_registry)?;
    }

    {
        let mut shared = workflow_registry.write();
        *shared = loaded_workflow_registry;
    }

    let ignore_patterns = ignore::load_ignore_patterns(workspace_root)
        .unwrap_or_else(|_| crate::tree::walker::WalkerConfig::default().ignore_patterns);

    let watch_config = WatchConfig {
        workspace_root: workspace_root.to_path_buf(),
        debounce_ms,
        batch_window_ms,
        ignore_patterns,
        session_id: Some(session_id.to_string()),
        progress: Some(Arc::clone(progress)),
        workflow_registry: Some(Arc::clone(workflow_registry)),
        ..WatchConfig::default()
    };

    let daemon = WatchDaemon::new(api, watch_config)?;
    tracing::info!("Starting watch mode daemon");
    daemon.start()?;
    Ok("Watch daemon stopped".to_string())
}

fn validate_bindings(
    registry: &AgentRegistry,
    workflow_registry: &WorkflowRegistry,
) -> Result<(), ApiError> {
    for agent in registry.list_all() {
        validate_agent_binding(agent, workflow_registry)?;
    }
    Ok(())
}

fn load_runtime_config(
    workspace_root: &Path,
    config_path: Option<&Path>,
) -> Result<crate::config::MerkleConfig, ApiError> {
    if let Some(config_path) = config_path {
        ConfigLoader::load_from_file(config_path).map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to load config from {}: {}",
                config_path.display(),
                err
            ))
        })
    } else {
        ConfigLoader::load(workspace_root)
            .map_err(|err| ApiError::ConfigError(format!("Failed to load config: {}", err)))
    }
}
