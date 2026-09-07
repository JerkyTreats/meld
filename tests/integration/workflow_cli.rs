use super::test_utils::with_xdg_env;
use meld::cli::{Commands, RunContext, WorkflowCommands};
use meld::error::ApiError;
use std::fs;
use tempfile::TempDir;

fn initialize_default_workflows() {
    super::install_legacy_workflow_fixture().unwrap();
}

#[test]
fn workflow_list_includes_initialized_profile() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        initialize_default_workflows();

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
fn workflow_execute_rejects_before_resolving_target_agent_or_provider() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        initialize_default_workflows();
        let legacy_root = meld::config::xdg::workspace_data_dir(&workspace_root)
            .unwrap()
            .join("workflow");
        let run_context = RunContext::new(workspace_root, None).unwrap();
        let result = run_context.execute(&Commands::Workflow {
            command: WorkflowCommands::Execute {
                workflow_id: "docs_writer_thread_v1".into(),
                node: None,
                path: None,
                path_positional: None,
                agent: "unregistered-agent".into(),
                provider: "unregistered-provider".into(),
                frame_type: None,
                force: true,
            },
        });
        let error = result.unwrap_err().to_string();
        assert!(
            error.contains("Legacy Workflow execution is retired"),
            "{error}"
        );
        assert!(!legacy_root.exists());
    });
}
