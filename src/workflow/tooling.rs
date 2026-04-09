use crate::api::ContextApi;
use crate::cli::WorkflowCommands;
use crate::config::ConfigLoader;
use crate::context::queue::QueueEventContext;
use crate::error::ApiError;
use crate::telemetry::ProgressRuntime;
use crate::workflow::commands::{WorkflowCommandService, WorkflowExecuteRequest};
use crate::workflow::registry::WorkflowRegistry;
use std::path::Path;
use std::sync::Arc;

pub fn handle_cli_command(
    api: &ContextApi,
    workspace_root: &Path,
    config_path: Option<&Path>,
    workflow_registry: &parking_lot::RwLock<WorkflowRegistry>,
    progress: &Arc<ProgressRuntime>,
    command: &WorkflowCommands,
    session_id: &str,
) -> Result<String, ApiError> {
    match command {
        WorkflowCommands::List { format } => {
            let registry = workflow_registry.read();
            let result = WorkflowCommandService::run_list(&registry);
            if format == "json" {
                serde_json::to_string_pretty(&result).map_err(|err| {
                    ApiError::ConfigError(format!(
                        "Failed to serialize workflow list result: {}",
                        err
                    ))
                })
            } else {
                let mut out = String::new();
                for item in result.workflows {
                    let path = item.source_path.unwrap_or_else(|| "-".to_string());
                    out.push_str(&format!(
                        "{} | v{} | {} | {}\n",
                        item.workflow_id, item.version, item.title, path
                    ));
                }
                if out.is_empty() {
                    out.push_str("No workflows resolved.\n");
                }
                Ok(out.trim_end().to_string())
            }
        }
        WorkflowCommands::Validate { format } => {
            let config = load_runtime_config(workspace_root, config_path)?;
            let result = WorkflowCommandService::run_validate(&config.workflows)?;
            if format == "json" {
                serde_json::to_string_pretty(&result).map_err(|err| {
                    ApiError::ConfigError(format!(
                        "Failed to serialize workflow validate result: {}",
                        err
                    ))
                })
            } else {
                Ok(format!(
                    "Workflow registry is valid with {} profile(s).",
                    result.workflow_count
                ))
            }
        }
        WorkflowCommands::Inspect {
            workflow_id,
            format,
        } => {
            let registry = workflow_registry.read();
            let result = WorkflowCommandService::run_inspect(&registry, workflow_id)?;
            if format == "json" {
                serde_json::to_string_pretty(&result).map_err(|err| {
                    ApiError::ConfigError(format!(
                        "Failed to serialize workflow inspect result: {}",
                        err
                    ))
                })
            } else {
                let mut out = String::new();
                out.push_str(&format!("workflow_id: {}\n", result.workflow_id));
                if let Some(path) = result.source_path {
                    out.push_str(&format!("source_path: {}\n", path));
                }
                out.push_str(&format!("version: {}\n", result.profile.version));
                out.push_str(&format!("title: {}\n", result.profile.title));
                out.push_str(&format!("description: {}\n", result.profile.description));
                out.push_str(&format!("turn_count: {}\n", result.profile.turns.len()));
                out.push_str(&format!("gate_count: {}\n", result.profile.gates.len()));
                for turn in result.profile.ordered_turns() {
                    out.push_str(&format!(
                        "turn:{} seq:{} gate:{} prompt:{}\n",
                        turn.turn_id, turn.seq, turn.gate_id, turn.prompt_ref
                    ));
                }
                Ok(out.trim_end().to_string())
            }
        }
        WorkflowCommands::Execute {
            workflow_id,
            node,
            path,
            path_positional,
            agent,
            provider,
            frame_type,
            force,
        } => {
            let path_merged = path.as_ref().or(path_positional.as_ref()).cloned();
            let event_context = QueueEventContext {
                session_id: session_id.to_string(),
                progress: Arc::clone(progress),
            };
            let execute_request = WorkflowExecuteRequest {
                workflow_id: workflow_id.clone(),
                node: node.clone(),
                path: path_merged,
                agent_id: agent.clone(),
                provider_name: provider.clone(),
                frame_type: frame_type.clone(),
                force: *force,
            };
            let registry = workflow_registry.read();
            let result = WorkflowCommandService::run_execute(
                api,
                workspace_root,
                &registry,
                &execute_request,
                Some(&event_context),
            )?;
            Ok(format!(
                "Workflow execution completed: workflow_id={}, thread_id={}, turns_completed={}, final_frame_id={}, skipped={}",
                result.workflow_id,
                result.thread_id,
                result.turns_completed,
                result.final_frame_id.as_deref().unwrap_or("none"),
                result.skipped
            ))
        }
    }
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
