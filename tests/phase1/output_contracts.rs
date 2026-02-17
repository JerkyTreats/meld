use std::fs;

use merkle::tooling::cli::{
    AgentCommands, CliContext, Commands, ContextCommands, ProviderCommands, WorkspaceCommands,
};
use tempfile::TempDir;

use crate::phase1::support::{create_test_agent, create_test_provider, with_xdg_env};

#[test]
fn workspace_validate_json_contract_has_required_fields() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        fs::write(workspace_root.join("a.txt"), "a").unwrap();

        let cli = CliContext::new(workspace_root.clone(), None).unwrap();
        cli.execute(&Commands::Scan { force: true }).unwrap();

        let output = cli
            .execute(&Commands::Workspace {
                command: WorkspaceCommands::Validate {
                    format: "json".to_string(),
                },
            })
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.get("valid").and_then(|v| v.as_bool()).is_some());
        assert!(parsed.get("root_hash").and_then(|v| v.as_str()).is_some());
        assert!(parsed.get("node_count").and_then(|v| v.as_u64()).is_some());
        assert!(parsed.get("frame_count").and_then(|v| v.as_u64()).is_some());
        assert!(parsed.get("errors").and_then(|v| v.as_array()).is_some());
    });
}

#[test]
fn agent_status_json_contract_has_required_fields() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        create_test_agent("phase1-agent");

        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();

        let cli = CliContext::new(workspace_root, None).unwrap();
        let output = cli
            .execute(&Commands::Agent {
                command: AgentCommands::Status {
                    format: "json".to_string(),
                },
            })
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.get("total").and_then(|v| v.as_u64()).is_some());
        assert!(parsed.get("valid_count").and_then(|v| v.as_u64()).is_some());
        let agents = parsed
            .get("agents")
            .and_then(|v| v.as_array())
            .expect("agents array should exist");
        assert!(!agents.is_empty());

        let entry = agents
            .iter()
            .find(|item| {
                item.get("agent_id") == Some(&serde_json::Value::String("phase1-agent".to_string()))
            })
            .expect("phase1-agent should appear in status output");
        assert!(entry.get("role").and_then(|v| v.as_str()).is_some());
        assert!(entry.get("valid").and_then(|v| v.as_bool()).is_some());
        assert!(entry
            .get("prompt_path_exists")
            .and_then(|v| v.as_bool())
            .is_some());
    });
}

#[test]
fn provider_list_json_contract_has_required_fields() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        create_test_provider("phase1-provider");

        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();

        let cli = CliContext::new(workspace_root, None).unwrap();
        let output = cli
            .execute(&Commands::Provider {
                command: ProviderCommands::List {
                    format: "json".to_string(),
                    type_filter: None,
                },
            })
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.get("total").and_then(|v| v.as_u64()).is_some());

        let providers = parsed
            .get("providers")
            .and_then(|v| v.as_array())
            .expect("providers array should exist");
        assert!(!providers.is_empty());

        let entry = providers
            .iter()
            .find(|item| {
                item.get("provider_name")
                    == Some(&serde_json::Value::String("phase1-provider".to_string()))
            })
            .expect("phase1-provider should appear in provider list output");

        assert!(entry
            .get("provider_type")
            .and_then(|v| v.as_str())
            .is_some());
        assert!(entry.get("model").and_then(|v| v.as_str()).is_some());
        assert!(entry.get("endpoint").is_some());
    });
}

#[test]
fn context_get_json_contract_has_required_fields() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();

        let target = workspace_root.join("context.txt");
        fs::write(&target, "context sample").unwrap();

        let cli = CliContext::new(workspace_root.clone(), None).unwrap();
        cli.execute(&Commands::Scan { force: true }).unwrap();

        let output = cli
            .execute(&Commands::Context {
                command: ContextCommands::Get {
                    node: None,
                    path: Some(target),
                    agent: None,
                    frame_type: None,
                    max_frames: 10,
                    ordering: "recency".to_string(),
                    combine: false,
                    separator: "\n\n---\n\n".to_string(),
                    format: "json".to_string(),
                    include_metadata: false,
                    include_deleted: false,
                },
            })
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.get("node_id").and_then(|v| v.as_str()).is_some());
        assert!(parsed.get("frame_count").and_then(|v| v.as_u64()).is_some());
        assert!(parsed.get("frames").and_then(|v| v.as_array()).is_some());
    });
}
