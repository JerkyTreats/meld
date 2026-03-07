//! Integration tests for workflow CLI commands.

use meld::agent::{AgentRole, AgentStorage, XdgAgentStorage};
use meld::cli::{Commands, RunContext, WorkflowCommands};
use meld::config::{xdg, AgentConfig, ProviderConfig, ProviderType};
use meld::context::frame::{Basis, Frame};
use meld::error::ApiError;
use meld::metadata::frame_write_contract::{
    build_generated_metadata, generated_metadata_input_from_payload,
};
use meld::workflow::state_store::{WorkflowStateStore, WorkflowThreadRecord, WorkflowThreadStatus};
use meld::workflow::{build_target_execution_request, execute_workflow_target};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use crate::integration::with_xdg_env;

fn create_test_agent(
    agent_id: &str,
    role: AgentRole,
    workflow_id: Option<&str>,
) -> Result<PathBuf, ApiError> {
    let agents_dir = XdgAgentStorage::new().agents_dir()?;
    fs::create_dir_all(&agents_dir)
        .map_err(|err| ApiError::ConfigError(format!("Failed to create agents dir: {}", err)))?;
    let config_path = agents_dir.join(format!("{}.toml", agent_id));

    let agent_config = AgentConfig {
        agent_id: agent_id.to_string(),
        role,
        system_prompt: None,
        system_prompt_path: None,
        workflow_id: workflow_id.map(ToString::to_string),
        metadata: Default::default(),
    };

    let toml = toml::to_string(&agent_config)
        .map_err(|err| ApiError::ConfigError(format!("Failed to encode agent config: {}", err)))?;
    fs::write(&config_path, toml)
        .map_err(|err| ApiError::ConfigError(format!("Failed to write agent config: {}", err)))?;
    Ok(config_path)
}

fn create_test_provider(
    provider_name: &str,
    provider_type: ProviderType,
) -> Result<PathBuf, ApiError> {
    let providers_dir = xdg::providers_dir()?;
    fs::create_dir_all(&providers_dir).map_err(|err| {
        ApiError::ConfigError(format!("Failed to create providers directory: {}", err))
    })?;
    let config_path = providers_dir.join(format!("{}.toml", provider_name));

    let provider_config = ProviderConfig {
        provider_name: Some(provider_name.to_string()),
        provider_type,
        model: "test-model".to_string(),
        api_key: None,
        endpoint: Some("http://127.0.0.1:9".to_string()),
        default_options: meld::provider::CompletionOptions::default(),
    };

    let toml = toml::to_string(&provider_config).map_err(|err| {
        ApiError::ConfigError(format!("Failed to serialize provider config: {}", err))
    })?;
    fs::write(&config_path, toml).map_err(|err| {
        ApiError::ConfigError(format!("Failed to write provider config: {}", err))
    })?;
    Ok(config_path)
}

#[test]
fn workflow_list_includes_builtin_profile() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();

        let run_context = RunContext::new(workspace_root, None).unwrap();
        let output = run_context
            .execute(&Commands::Workflow {
                command: WorkflowCommands::List {
                    format: "text".to_string(),
                },
            })
            .unwrap();
        assert!(output.contains("docs_writer_thread_v1"));
    });
}

#[test]
fn workflow_inspect_returns_error_for_unknown_workflow() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();

        let run_context = RunContext::new(workspace_root, None).unwrap();
        let result = run_context.execute(&Commands::Workflow {
            command: WorkflowCommands::Inspect {
                workflow_id: "missing".to_string(),
                format: "text".to_string(),
            },
        });
        assert!(result.is_err());
        match result {
            Err(ApiError::ConfigError(message)) => {
                assert!(message.contains("Workflow not found"));
            }
            _ => panic!("Expected ConfigError for missing workflow"),
        }
    });
}

#[test]
fn workflow_execute_uses_workflow_runtime_and_can_skip_completed_thread() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        let file_path = workspace_root.join("doc.md");
        fs::write(&file_path, "# hello").unwrap();

        create_test_agent(
            "docs-writer",
            AgentRole::Writer,
            Some("docs_writer_thread_v1"),
        )
        .unwrap();
        create_test_provider("test-provider", ProviderType::LocalCustom).unwrap();

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let node_id = meld::workspace::resolve_workspace_node_id(
            run_context.api(),
            &workspace_root,
            Some(file_path.as_path()),
            None,
            false,
        )
        .unwrap();

        let frame_type = "context-docs-writer".to_string();
        let frame = Frame::new(
            Basis::Node(node_id),
            b"seed".to_vec(),
            frame_type.clone(),
            "docs-writer".to_string(),
            build_generated_metadata(&generated_metadata_input_from_payload(
                "docs-writer",
                "test-provider",
                "test-model",
                "local",
                "seed prompt",
                "seed context",
            )),
        )
        .unwrap();
        let _frame_id = run_context
            .api()
            .put_frame(node_id, frame, "docs-writer".to_string())
            .unwrap();

        let thread_payload = format!(
            "{}:{}:{}",
            "docs_writer_thread_v1",
            hex::encode(node_id),
            frame_type
        );
        let thread_digest = blake3::hash(thread_payload.as_bytes()).to_hex().to_string();
        let thread_id = format!("thread-{}", &thread_digest[..16]);
        let state_store = WorkflowStateStore::new(&workspace_root).unwrap();
        state_store
            .upsert_thread(&WorkflowThreadRecord {
                thread_id: thread_id.clone(),
                workflow_id: "docs_writer_thread_v1".to_string(),
                node_id: hex::encode(node_id),
                frame_type: "context-docs-writer".to_string(),
                status: WorkflowThreadStatus::Completed,
                next_turn_seq: 5,
                updated_at_ms: 0,
            })
            .unwrap();

        let output = run_context
            .execute(&Commands::Workflow {
                command: WorkflowCommands::Execute {
                    workflow_id: "docs_writer_thread_v1".to_string(),
                    node: Some(hex::encode(node_id)),
                    path: None,
                    path_positional: None,
                    agent: "docs-writer".to_string(),
                    provider: "test-provider".to_string(),
                    frame_type: None,
                    force: false,
                },
            })
            .unwrap();
        assert!(output.contains("workflow_id=docs_writer_thread_v1"));
        assert!(output.contains("skipped=true"));
        assert!(output.contains(&thread_id));
    });
}

#[test]
fn workflow_facade_maps_completed_thread_reuse_to_target_result() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        let file_path = workspace_root.join("doc.md");
        fs::write(&file_path, "# hello").unwrap();

        create_test_agent(
            "docs-writer",
            AgentRole::Writer,
            Some("docs_writer_thread_v1"),
        )
        .unwrap();
        create_test_provider("test-provider", ProviderType::LocalCustom).unwrap();

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let node_id = meld::workspace::resolve_workspace_node_id(
            run_context.api(),
            &workspace_root,
            Some(file_path.as_path()),
            None,
            false,
        )
        .unwrap();

        let frame = Frame::new(
            Basis::Node(node_id),
            b"seed".to_vec(),
            "context-docs-writer".to_string(),
            "docs-writer".to_string(),
            build_generated_metadata(&generated_metadata_input_from_payload(
                "docs-writer",
                "test-provider",
                "test-model",
                "local",
                "seed prompt",
                "seed context",
            )),
        )
        .unwrap();
        let frame_id = run_context
            .api()
            .put_frame(node_id, frame, "docs-writer".to_string())
            .unwrap();

        let thread_payload = format!(
            "{}:{}:{}",
            "docs_writer_thread_v1",
            hex::encode(node_id),
            "context-docs-writer"
        );
        let thread_digest = blake3::hash(thread_payload.as_bytes()).to_hex().to_string();
        let thread_id = format!("thread-{}", &thread_digest[..16]);
        let state_store = WorkflowStateStore::new(&workspace_root).unwrap();
        state_store
            .upsert_thread(&WorkflowThreadRecord {
                thread_id: thread_id.clone(),
                workflow_id: "docs_writer_thread_v1".to_string(),
                node_id: hex::encode(node_id),
                frame_type: "context-docs-writer".to_string(),
                status: WorkflowThreadStatus::Completed,
                next_turn_seq: 5,
                updated_at_ms: 0,
            })
            .unwrap();

        let request = build_target_execution_request(
            run_context.api(),
            node_id,
            "docs-writer".to_string(),
            "test-provider".to_string(),
            "context-docs-writer".to_string(),
            false,
            meld::context::TargetExecutionProgram::workflow("docs_writer_thread_v1"),
            None,
            None,
            None,
        )
        .unwrap();

        let result =
            execute_workflow_target(run_context.api(), &workspace_root, &request, None).unwrap();

        assert_eq!(result.final_frame_id, frame_id);
        assert!(result.reused_existing_head);
        assert_eq!(result.workflow_id.as_deref(), Some("docs_writer_thread_v1"));
        assert_eq!(result.thread_id.as_deref(), Some(thread_id.as_str()));
        assert_eq!(result.turns_completed, 0);
    });
}
