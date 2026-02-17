use std::fs;

use merkle::tooling::cli::{
    AgentCommands, CliContext, Commands, ContextCommands, ProviderCommands, WorkspaceCommands,
};
use tempfile::TempDir;

use crate::phase1::support::{
    create_test_agent, create_test_provider, latest_session_events, with_xdg_env,
};

#[test]
fn command_families_emit_typed_and_command_summary_events() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        create_test_agent("summary-agent");
        create_test_provider("summary-provider");

        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        let target = workspace_root.join("a.txt");
        fs::write(&target, "hello").unwrap();

        let cli = CliContext::new(workspace_root.clone(), None).unwrap();
        cli.execute(&Commands::Scan { force: true }).unwrap();

        let checks: Vec<(Commands, &str, &str)> = vec![
            (
                Commands::Workspace {
                    command: WorkspaceCommands::Status {
                        format: "text".to_string(),
                        breakdown: false,
                    },
                },
                "workspace.status",
                "status_summary",
            ),
            (
                Commands::Agent {
                    command: AgentCommands::List {
                        format: "text".to_string(),
                        role: None,
                    },
                },
                "agent.list",
                "config_mutation_summary",
            ),
            (
                Commands::Provider {
                    command: ProviderCommands::List {
                        format: "text".to_string(),
                        type_filter: None,
                    },
                },
                "provider.list",
                "config_mutation_summary",
            ),
            (
                Commands::Status {
                    format: "text".to_string(),
                    workspace_only: true,
                    agents_only: false,
                    providers_only: false,
                    breakdown: false,
                    test_connectivity: false,
                },
                "status",
                "status_summary",
            ),
            (
                Commands::Init {
                    force: false,
                    list: true,
                },
                "init",
                "init_summary",
            ),
        ];

        for (command, session_command, typed_event) in checks {
            cli.execute(&command).unwrap();
            let events = latest_session_events(&cli.progress_runtime(), session_command);

            assert!(
                events.iter().any(|e| e.event_type == typed_event),
                "missing typed summary {typed_event} for {session_command}"
            );
            assert!(
                events.iter().any(|e| e.event_type == "command_summary"),
                "missing command_summary for {session_command}"
            );
            assert!(
                events.windows(2).all(|w| w[1].seq == w[0].seq + 1),
                "event sequence should be monotonic for {session_command}"
            );
        }

        cli.execute(&Commands::Context {
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

        let context_events = latest_session_events(&cli.progress_runtime(), "context.get");
        assert!(context_events
            .iter()
            .any(|e| e.event_type == "context_read_summary"));
        assert!(context_events
            .iter()
            .any(|e| e.event_type == "command_summary"));
    });
}

#[test]
fn failing_command_summary_payload_is_bounded() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        let target = workspace_root.join("missing-provider.txt");
        fs::write(&target, "hello").unwrap();

        let cli = CliContext::new(workspace_root.clone(), None).unwrap();
        cli.execute(&Commands::Scan { force: true }).unwrap();

        let result = cli.execute(&Commands::Context {
            command: ContextCommands::Generate {
                node: None,
                path: Some(target),
                path_positional: None,
                agent: None,
                provider: None,
                frame_type: None,
                force: false,
                no_recursive: false,
            },
        });
        assert!(result.is_err());

        let events = latest_session_events(&cli.progress_runtime(), "context.generate");
        let summary = events
            .iter()
            .find(|e| e.event_type == "command_summary")
            .expect("command_summary should be present for failed command");

        let message = summary
            .data
            .get("message")
            .and_then(|v| v.as_str())
            .expect("failed summary should include a message");

        assert!(message.chars().count() <= 256);
        assert!(summary
            .data
            .get("error_chars")
            .and_then(|v| v.as_u64())
            .is_some());
        assert!(summary.data.get("output_chars").is_none());
    });
}
