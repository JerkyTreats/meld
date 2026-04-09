use crate::api::ContextApi;
use crate::cli::{
    format_provider_list_result_json, format_provider_list_result_text,
    format_provider_show_result_json, format_provider_show_result_text,
    format_provider_test_result, format_provider_validation_result, ProviderCommands,
};
use crate::error::ApiError;
use crate::provider::commands::ProviderCommandService;
use crate::telemetry::{ProgressRuntime, ProviderLifecycleEventData};
use crate::workspace::{format_provider_status_text, ProviderStatusEntry, ProviderStatusOutput};
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

type ProviderCreationDialogResult = (
    crate::provider::ProviderType,
    String,
    Option<String>,
    Option<String>,
    crate::provider::CompletionOptions,
);

pub fn handle_cli_command(
    api: &ContextApi,
    progress: &Arc<ProgressRuntime>,
    command: &ProviderCommands,
    session_id: &str,
) -> Result<String, ApiError> {
    match command {
        ProviderCommands::Status {
            format,
            test_connectivity,
        } => handle_status(api, format, *test_connectivity),
        ProviderCommands::List {
            format,
            type_filter,
        } => handle_list(api, format, type_filter.as_deref()),
        ProviderCommands::Show {
            provider_name,
            format,
            include_credentials,
        } => handle_show(api, provider_name, format, *include_credentials),
        ProviderCommands::Validate {
            provider_name,
            test_connectivity,
            check_model,
            verbose,
        } => handle_validate(
            api,
            provider_name,
            *test_connectivity,
            *check_model,
            *verbose,
        ),
        ProviderCommands::Test {
            provider_name,
            model,
            timeout,
        } => handle_test(
            api,
            progress,
            provider_name,
            model.as_deref(),
            *timeout,
            session_id,
        ),
        ProviderCommands::Create {
            provider_name,
            type_,
            model,
            endpoint,
            api_key,
            interactive,
            non_interactive,
        } => handle_create(
            api,
            provider_name,
            type_.as_deref(),
            model.as_deref(),
            endpoint.as_deref(),
            api_key.as_deref(),
            *interactive,
            *non_interactive,
        ),
        ProviderCommands::Edit {
            provider_name,
            model,
            endpoint,
            api_key,
            editor,
        } => handle_edit(
            api,
            provider_name,
            model.as_deref(),
            endpoint.as_deref(),
            api_key.as_deref(),
            editor.as_deref(),
        ),
        ProviderCommands::Remove {
            provider_name,
            force,
        } => handle_remove(api, provider_name, *force),
    }
}

fn handle_list(
    api: &ContextApi,
    format: &str,
    type_filter: Option<&str>,
) -> Result<String, ApiError> {
    let registry = api.provider_registry().read();
    let result = ProviderCommandService::run_list(&registry, type_filter)?;
    match format {
        "json" => Ok(format_provider_list_result_json(&result)),
        _ => Ok(format_provider_list_result_text(&result)),
    }
}

fn handle_show(
    api: &ContextApi,
    provider_name: &str,
    format: &str,
    include_credentials: bool,
) -> Result<String, ApiError> {
    let registry = api.provider_registry().read();
    let result = ProviderCommandService::run_show(&registry, provider_name, include_credentials)?;
    match format {
        "json" => Ok(format_provider_show_result_json(&result)),
        _ => Ok(format_provider_show_result_text(&result)),
    }
}

fn handle_validate(
    api: &ContextApi,
    provider_name: &str,
    test_connectivity: bool,
    check_model: bool,
    verbose: bool,
) -> Result<String, ApiError> {
    let registry = api.provider_registry().read();
    let result = ProviderCommandService::run_validate(
        &registry,
        provider_name,
        test_connectivity,
        check_model,
    )?;
    Ok(format_provider_validation_result(&result, verbose))
}

fn handle_status(
    api: &ContextApi,
    format: &str,
    test_connectivity: bool,
) -> Result<String, ApiError> {
    let registry = api.provider_registry().read();
    let entries_result = ProviderCommandService::run_status(&registry, test_connectivity)?;
    let entries: Vec<ProviderStatusEntry> = entries_result
        .into_iter()
        .map(|entry| ProviderStatusEntry {
            provider_name: entry.provider_name,
            provider_type: entry.provider_type,
            model: entry.model,
            connectivity: entry.connectivity,
        })
        .collect();

    if format == "json" {
        serde_json::to_string_pretty(&ProviderStatusOutput {
            providers: entries.clone(),
            total: entries.len(),
        })
        .map_err(|e| ApiError::StorageError(crate::error::StorageError::InvalidPath(e.to_string())))
    } else {
        Ok(format_provider_status_text(&entries, test_connectivity))
    }
}

fn handle_test(
    api: &ContextApi,
    progress: &Arc<ProgressRuntime>,
    provider_name: &str,
    model_override: Option<&str>,
    timeout: u64,
    session_id: &str,
) -> Result<String, ApiError> {
    let registry = api.provider_registry().read();
    let model_for_event = model_override.unwrap_or_else(|| {
        registry
            .get(provider_name)
            .map(|provider| provider.model.as_str())
            .unwrap_or("")
    });

    progress.emit_event_best_effort(
        session_id,
        "provider_request_sent",
        json!(ProviderLifecycleEventData {
            node_id: "provider_test".to_string(),
            agent_id: "provider_test".to_string(),
            provider_name: provider_name.to_string(),
            frame_type: model_for_event.to_string(),
            duration_ms: None,
            error: None,
            retry_count: Some(0),
        }),
    );

    let start = std::time::Instant::now();
    let result =
        ProviderCommandService::run_test(&registry, provider_name, model_override, timeout)?;
    let elapsed_ms = start.elapsed().as_millis();

    let event_name = if result.connectivity_ok {
        "provider_response_received"
    } else {
        "provider_request_failed"
    };
    progress.emit_event_best_effort(
        session_id,
        event_name,
        json!(ProviderLifecycleEventData {
            node_id: "provider_test".to_string(),
            agent_id: "provider_test".to_string(),
            provider_name: result.provider_name.clone(),
            frame_type: result.model_checked.clone(),
            duration_ms: Some(elapsed_ms),
            error: result.error_message.clone(),
            retry_count: Some(0),
        }),
    );

    Ok(format_provider_test_result(&result, Some(elapsed_ms)))
}

#[allow(clippy::too_many_arguments)]
fn handle_create(
    api: &ContextApi,
    provider_name: &str,
    type_: Option<&str>,
    model: Option<&str>,
    endpoint: Option<&str>,
    api_key: Option<&str>,
    interactive: bool,
    non_interactive: bool,
) -> Result<String, ApiError> {
    let is_interactive = interactive || (!non_interactive && type_.is_none());

    let (provider_type, final_model, final_endpoint, final_api_key, default_options) =
        if is_interactive {
            create_interactive()?
        } else {
            let type_str = type_.ok_or_else(|| {
                ApiError::ConfigError(
                    "Provider type is required in non-interactive mode. Use --type <type>"
                        .to_string(),
                )
            })?;

            let parsed_type = ProviderCommandService::parse_provider_type(type_str)?;
            let model_name = model.ok_or_else(|| {
                ApiError::ConfigError(
                    "Model is required in non-interactive mode. Use --model <model>".to_string(),
                )
            })?;

            (
                parsed_type,
                model_name.to_string(),
                endpoint.map(str::to_string),
                api_key.map(str::to_string),
                crate::provider::CompletionOptions::default(),
            )
        };

    let mut registry = api.provider_registry().write();
    let result = ProviderCommandService::run_create(
        &mut registry,
        provider_name,
        provider_type,
        final_model,
        final_endpoint,
        final_api_key,
        default_options,
    )?;

    Ok(format!(
        "Provider created: {}\nConfiguration file: {}",
        result.provider_name,
        result.config_path.display()
    ))
}

fn create_interactive() -> Result<ProviderCreationDialogResult, ApiError> {
    use dialoguer::{Input, Select};

    let type_selection = Select::new()
        .with_prompt("Provider type")
        .items(&["openai", "anthropic", "ollama", "local"])
        .default(0)
        .interact()
        .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

    let provider_type = match type_selection {
        0 => crate::provider::ProviderType::OpenAI,
        1 => crate::provider::ProviderType::Anthropic,
        2 => crate::provider::ProviderType::Ollama,
        3 => crate::provider::ProviderType::LocalCustom,
        _ => unreachable!(),
    };

    let model: String = Input::new()
        .with_prompt("Model name")
        .interact_text()
        .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

    let default_endpoint = ProviderCommandService::default_endpoint(provider_type);
    let endpoint = if provider_type == crate::provider::ProviderType::LocalCustom {
        Some(
            Input::new()
                .with_prompt("Endpoint URL required")
                .interact_text()
                .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?,
        )
    } else if let Some(default) = default_endpoint {
        let input: String = Input::new()
            .with_prompt(format!("Endpoint URL optional, default: {}", default))
            .default(default)
            .interact_text()
            .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;
        Some(input)
    } else {
        None
    };

    let env_var = ProviderCommandService::required_api_key_env_var(provider_type).unwrap_or("");
    let api_key = if provider_type == crate::provider::ProviderType::Ollama {
        None
    } else {
        let prompt = if env_var.is_empty() {
            "API key optional".to_string()
        } else {
            format!("API key optional, will use {} env var if not set", env_var)
        };

        let input: String = Input::new()
            .with_prompt(prompt)
            .allow_empty(true)
            .interact_text()
            .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

        if input.is_empty() {
            None
        } else {
            Some(input)
        }
    };

    let temperature: f32 = Input::new()
        .with_prompt("Default temperature 0.0-2.0, default: 1.0")
        .default(1.0)
        .interact_text()
        .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

    let max_tokens: Option<u32> = {
        let input: String = Input::new()
            .with_prompt("Default max tokens optional, press Enter to skip")
            .allow_empty(true)
            .interact_text()
            .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

        if input.is_empty() {
            None
        } else {
            input.parse().ok()
        }
    };

    let default_options = crate::provider::CompletionOptions {
        temperature: Some(temperature),
        max_tokens,
        ..Default::default()
    };

    Ok((provider_type, model, endpoint, api_key, default_options))
}

fn handle_edit(
    api: &ContextApi,
    provider_name: &str,
    model: Option<&str>,
    endpoint: Option<&str>,
    api_key: Option<&str>,
    editor: Option<&str>,
) -> Result<String, ApiError> {
    if model.is_some() || endpoint.is_some() || api_key.is_some() {
        let mut registry = api.provider_registry().write();
        ProviderCommandService::run_update_flags(
            &mut registry,
            provider_name,
            model,
            endpoint,
            api_key,
        )?;
    } else {
        edit_with_editor(api, provider_name, editor)?;
    }

    Ok(format!("Provider updated: {}", provider_name))
}

fn edit_with_editor(
    api: &ContextApi,
    provider_name: &str,
    editor: Option<&str>,
) -> Result<(), ApiError> {
    let config_path = {
        let registry = api.provider_registry().read();
        ProviderCommandService::provider_config_path(&registry, provider_name)?
    };

    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| ApiError::ConfigError(format!("Failed to read config: {}", e)))?;
    let temp_path = std::env::temp_dir().join(format!("meld-provider-{}.toml", provider_name));
    std::fs::write(&temp_path, content.as_bytes())
        .map_err(|e| ApiError::ConfigError(format!("Failed to write temp file: {}", e)))?;

    open_editor_for_path(&temp_path, editor)?;

    let edited_content = std::fs::read_to_string(&temp_path)
        .map_err(|e| ApiError::ConfigError(format!("Failed to read edited file: {}", e)))?;
    let provider_config: crate::config::ProviderConfig = toml::from_str(&edited_content)
        .map_err(|e| ApiError::ConfigError(format!("Invalid config after editing: {}", e)))?;

    if let Some(config_name) = provider_config.provider_name.as_deref() {
        if config_name != provider_name {
            return Err(ApiError::ConfigError(format!(
                "Provider name mismatch: config has '{}' but expected '{}'",
                config_name, provider_name
            )));
        }
    }

    let mut registry = api.provider_registry().write();
    ProviderCommandService::persist_provider_config(
        &mut registry,
        provider_name,
        &provider_config,
    )?;

    let _ = std::fs::remove_file(&temp_path);
    Ok(())
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

fn handle_remove(api: &ContextApi, provider_name: &str, force: bool) -> Result<String, ApiError> {
    {
        let registry = api.provider_registry().read();
        let provider = registry.get_or_error(provider_name)?;
        if provider.provider_type == crate::provider::ProviderType::OpenAI
            || provider.provider_type == crate::provider::ProviderType::Anthropic
        {
            eprintln!(
                "Warning: Provider '{}' may be in use by agents.",
                provider_name
            );
        }
    }

    if !force {
        use dialoguer::Confirm;
        let confirmed = Confirm::new()
            .with_prompt(format!("Remove provider '{}'?", provider_name))
            .interact()
            .map_err(|e| ApiError::ConfigError(format!("Failed to get user input: {}", e)))?;

        if !confirmed {
            return Ok("Removal cancelled".to_string());
        }
    }

    let mut registry = api.provider_registry().write();
    let result = ProviderCommandService::run_remove(&mut registry, provider_name)?;
    Ok(format!(
        "Removed provider: {}\nConfiguration file deleted: {}",
        result.provider_name,
        result.config_path.display()
    ))
}
