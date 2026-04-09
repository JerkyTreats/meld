use crate::agent::commands::AgentCommandService;
use crate::api::ContextApi;
use crate::cli::{
    format_agent_list_result_json, format_agent_list_result_text, format_agent_show_result_json,
    format_agent_show_result_text, format_validation_result, format_validation_results_all,
    AgentCommands, AgentPromptCommands,
};
use crate::error::ApiError;
use crate::workspace::{format_agent_status_text, AgentStatusEntry, AgentStatusOutput};
use std::path::{Path, PathBuf};

pub fn handle_cli_command(api: &ContextApi, command: &AgentCommands) -> Result<String, ApiError> {
    match command {
        AgentCommands::Status { format } => handle_status(api, format),
        AgentCommands::List { format, role } => handle_list(api, format, role.as_deref()),
        AgentCommands::Show {
            agent_id,
            format,
            include_prompt,
        } => handle_show(api, agent_id, format, *include_prompt),
        AgentCommands::Validate {
            agent_id,
            all,
            verbose,
        } => handle_validate(api, agent_id.as_deref(), *all, *verbose),
        AgentCommands::Create {
            agent_id,
            role,
            prompt_path,
            interactive,
            non_interactive,
        } => handle_create(
            api,
            agent_id,
            role.as_deref(),
            prompt_path.as_deref(),
            *interactive,
            *non_interactive,
        ),
        AgentCommands::Edit {
            agent_id,
            prompt_path,
            role,
            editor,
        } => handle_edit(
            api,
            agent_id,
            prompt_path.as_deref(),
            role.as_deref(),
            editor.as_deref(),
        ),
        AgentCommands::Prompt { command } => handle_prompt_command(api, command),
        AgentCommands::Remove { agent_id, force } => handle_remove(api, agent_id, *force),
    }
}

fn handle_prompt_command(
    api: &ContextApi,
    command: &AgentPromptCommands,
) -> Result<String, ApiError> {
    match command {
        AgentPromptCommands::Show { agent_id } => handle_prompt_show(api, agent_id),
        AgentPromptCommands::Edit { agent_id, editor } => {
            handle_prompt_edit(api, agent_id, editor.as_deref())
        }
    }
}

fn handle_list(
    api: &ContextApi,
    format: &str,
    role_filter: Option<&str>,
) -> Result<String, ApiError> {
    let registry = api.agent_registry().read();
    let result = AgentCommandService::list(&registry, role_filter)?;
    match format {
        "json" => Ok(format_agent_list_result_json(&result)),
        _ => Ok(format_agent_list_result_text(&result)),
    }
}

fn handle_show(
    api: &ContextApi,
    agent_id: &str,
    format: &str,
    include_prompt: bool,
) -> Result<String, ApiError> {
    let registry = api.agent_registry().read();
    let result = AgentCommandService::show(&registry, agent_id, include_prompt)?;
    match format {
        "json" => Ok(format_agent_show_result_json(&result)),
        _ => Ok(format_agent_show_result_text(&result)),
    }
}

fn handle_validate(
    api: &ContextApi,
    agent_id: Option<&str>,
    all: bool,
    verbose: bool,
) -> Result<String, ApiError> {
    let registry = api.agent_registry().read();
    if all {
        let result = AgentCommandService::validate_all(&registry)?;
        if result.results.is_empty() {
            return Ok("No agents found to validate.".to_string());
        }
        Ok(format_validation_results_all(&result.results, verbose))
    } else {
        let id = agent_id.ok_or_else(|| {
            ApiError::ConfigError("Agent ID required unless --all is specified".to_string())
        })?;
        let result = AgentCommandService::validate_single(&registry, id)?;
        Ok(format_validation_result(&result.result, verbose))
    }
}

fn handle_create(
    api: &ContextApi,
    agent_id: &str,
    role: Option<&str>,
    prompt_path: Option<&str>,
    interactive: bool,
    non_interactive: bool,
) -> Result<String, ApiError> {
    let is_interactive = interactive || (!non_interactive && role.is_none());

    let (final_role, final_prompt_path) = if is_interactive {
        create_interactive()?
    } else {
        let role_str = role.ok_or_else(|| {
            ApiError::ConfigError(
                "Role is required in non-interactive mode. Use --role <role>".to_string(),
            )
        })?;
        let parsed_role = AgentCommandService::parse_role(role_str)?;
        let prompt = if parsed_role != crate::agent::AgentRole::Reader {
            Some(
                prompt_path
                    .ok_or_else(|| {
                        ApiError::ConfigError(
                            "Prompt path is required for Writer agents. Use --prompt-path <path>"
                                .to_string(),
                        )
                    })?
                    .to_string(),
            )
        } else {
            None
        };
        (parsed_role, prompt)
    };

    let mut registry = api.agent_registry().write();
    let result =
        AgentCommandService::create(&mut registry, agent_id, final_role, final_prompt_path)?;
    let mut output = format!(
        "Agent created: {}\nConfiguration file: {}",
        result.agent_id,
        result.config_path.display()
    );
    if let Some(prompt_path) = result.prompt_path {
        output.push_str(&format!("\nPrompt file: {}", prompt_path.display()));
    }
    Ok(output)
}

fn create_interactive() -> Result<(crate::agent::AgentRole, Option<String>), ApiError> {
    use dialoguer::{Input, Select};

    let role_selection = Select::new()
        .with_prompt("Agent role")
        .items(&["Reader", "Writer"])
        .default(1)
        .interact()
        .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

    let role = match role_selection {
        0 => crate::agent::AgentRole::Reader,
        1 => crate::agent::AgentRole::Writer,
        _ => unreachable!(),
    };

    let prompt_path = if role != crate::agent::AgentRole::Reader {
        let path: String = Input::new()
            .with_prompt("Prompt file path")
            .interact_text()
            .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;
        Some(path)
    } else {
        None
    };

    Ok((role, prompt_path))
}

fn handle_edit(
    api: &ContextApi,
    agent_id: &str,
    prompt_path: Option<&str>,
    role: Option<&str>,
    editor: Option<&str>,
) -> Result<String, ApiError> {
    if prompt_path.is_some() || role.is_some() {
        let mut registry = api.agent_registry().write();
        let _ = AgentCommandService::update_flags(&mut registry, agent_id, prompt_path, role)?;
    } else {
        edit_with_editor(api, agent_id, editor)?;
    }
    Ok(format!("Agent updated: {}", agent_id))
}

fn edit_with_editor(
    api: &ContextApi,
    agent_id: &str,
    editor: Option<&str>,
) -> Result<(), ApiError> {
    let config_path = api.agent_registry().read().agent_config_path(agent_id)?;
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| ApiError::ConfigError(format!("Failed to read config: {}", e)))?;

    let temp_path = std::env::temp_dir().join(format!("meld-agent-{}.toml", agent_id));
    std::fs::write(&temp_path, content.as_bytes())
        .map_err(|e| ApiError::ConfigError(format!("Failed to write temp file: {}", e)))?;
    open_editor_for_path(&temp_path, editor)?;

    let edited_content = std::fs::read_to_string(&temp_path)
        .map_err(|e| ApiError::ConfigError(format!("Failed to read edited file: {}", e)))?;
    let agent_config: crate::agent::AgentConfig = toml::from_str(&edited_content)
        .map_err(|e| ApiError::ConfigError(format!("Invalid config after editing: {}", e)))?;

    let mut registry = api.agent_registry().write();
    AgentCommandService::persist_edited_config(&mut registry, agent_id, agent_config)?;

    let _ = std::fs::remove_file(&temp_path);
    Ok(())
}

fn resolve_prompt_file_path(api: &ContextApi, agent_id: &str) -> Result<PathBuf, ApiError> {
    let prompt_path = {
        let registry = api.agent_registry().read();
        let result = AgentCommandService::show(&registry, agent_id, false)?;
        result.prompt_path.ok_or_else(|| {
            ApiError::ConfigError(format!(
                "Agent '{}' has no prompt file path configured",
                agent_id
            ))
        })?
    };
    Ok(PathBuf::from(prompt_path))
}

fn handle_prompt_show(api: &ContextApi, agent_id: &str) -> Result<String, ApiError> {
    let prompt_path = resolve_prompt_file_path(api, agent_id)?;
    let content = std::fs::read_to_string(&prompt_path).map_err(|e| {
        ApiError::ConfigError(format!(
            "Failed to read prompt file {}: {}",
            prompt_path.display(),
            e
        ))
    })?;

    Ok(format!(
        "Agent: {}\nPrompt file: {}\n\n{}",
        agent_id,
        prompt_path.display(),
        content
    ))
}

fn handle_prompt_edit(
    api: &ContextApi,
    agent_id: &str,
    editor: Option<&str>,
) -> Result<String, ApiError> {
    let prompt_path = resolve_prompt_file_path(api, agent_id)?;
    if let Some(parent) = prompt_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            ApiError::ConfigError(format!(
                "Failed to create prompt directory {}: {}",
                parent.display(),
                e
            ))
        })?;
    }
    if !prompt_path.exists() {
        std::fs::write(&prompt_path, b"").map_err(|e| {
            ApiError::ConfigError(format!(
                "Failed to create prompt file {}: {}",
                prompt_path.display(),
                e
            ))
        })?;
    }

    open_editor_for_path(&prompt_path, editor)?;
    Ok(format!("Prompt updated: {}", prompt_path.display()))
}

fn open_editor_for_path(path: &Path, editor: Option<&str>) -> Result<(), ApiError> {
    use std::process::Command;

    let editor_cmd = match editor {
        Some(editor) => editor.to_string(),
        None => std::env::var("EDITOR").map_err(|_| {
            ApiError::ConfigError(
                "No editor specified and $EDITOR not set. Use --editor <editor>".to_string(),
            )
        })?,
    };

    let status = Command::new(&editor_cmd)
        .arg(path)
        .status()
        .map_err(|e| ApiError::ConfigError(format!("Failed to open editor: {}", e)))?;

    if !status.success() {
        return Err(ApiError::ConfigError(
            "Editor exited with non-zero status".to_string(),
        ));
    }

    Ok(())
}

fn handle_remove(api: &ContextApi, agent_id: &str, force: bool) -> Result<String, ApiError> {
    if !force {
        use dialoguer::Confirm;
        let confirmed = Confirm::new()
            .with_prompt(format!("Remove agent '{}'?", agent_id))
            .interact()
            .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

        if !confirmed {
            return Ok("Removal cancelled".to_string());
        }
    }

    let mut registry = api.agent_registry().write();
    let result = AgentCommandService::remove(&mut registry, agent_id)?;
    Ok(format!(
        "Removed agent: {}\nConfiguration file deleted: {}",
        result.agent_id,
        result.config_path.display()
    ))
}

fn handle_status(api: &ContextApi, format: &str) -> Result<String, ApiError> {
    let registry = api.agent_registry().read();
    let entries_result = AgentCommandService::status(&registry)?;
    let entries: Vec<AgentStatusEntry> = entries_result
        .into_iter()
        .map(|entry| AgentStatusEntry {
            agent_id: entry.agent_id,
            role: entry.role,
            valid: entry.valid,
            prompt_path_exists: entry.prompt_path_exists,
        })
        .collect();
    let valid_count = entries.iter().filter(|entry| entry.valid).count();

    if format == "json" {
        serde_json::to_string_pretty(&AgentStatusOutput {
            agents: entries.clone(),
            total: entries.len(),
            valid_count,
        })
        .map_err(|e| ApiError::StorageError(crate::error::StorageError::InvalidPath(e.to_string())))
    } else {
        Ok(format_agent_status_text(&entries))
    }
}
