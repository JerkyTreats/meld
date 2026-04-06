use crate::integration::with_xdg_env;
use meld::agent::{AgentRole, AgentStorage, XdgAgentStorage};
use meld::cli::{Commands, RunContext, WorkflowCommands};
use meld::config::{xdg, AgentConfig, ProviderConfig, ProviderType};
use meld::workflow::state_store::{WorkflowStateStore, WorkflowThreadStatus};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use tempfile::TempDir;

fn create_test_agent(agent_id: &str, workflow_id: Option<&str>) {
    let agents_dir = XdgAgentStorage::new().agents_dir().unwrap();
    fs::create_dir_all(&agents_dir).unwrap();
    let config_path = agents_dir.join(format!("{agent_id}.toml"));

    let mut metadata = std::collections::HashMap::new();
    metadata.insert(
        "user_prompt_file".to_string(),
        "Summarize file context".to_string(),
    );
    metadata.insert(
        "user_prompt_directory".to_string(),
        "Summarize directory context".to_string(),
    );

    let agent_config = AgentConfig {
        agent_id: agent_id.to_string(),
        role: AgentRole::Writer,
        system_prompt: Some("You are a careful docs writer.".to_string()),
        system_prompt_path: None,
        workflow_id: workflow_id.map(ToString::to_string),
        metadata: metadata.into(),
    };

    fs::write(&config_path, toml::to_string(&agent_config).unwrap()).unwrap();
}

fn create_test_provider(provider_name: &str, endpoint: &str) {
    let providers_dir = xdg::providers_dir().unwrap();
    fs::create_dir_all(&providers_dir).unwrap();
    let config_path = providers_dir.join(format!("{provider_name}.toml"));

    let provider_config = ProviderConfig {
        provider_name: Some(provider_name.to_string()),
        provider_type: ProviderType::LocalCustom,
        model: "test-model".to_string(),
        api_key: None,
        endpoint: Some(endpoint.to_string()),
        default_options: meld::provider::CompletionOptions::default(),
    };

    fs::write(&config_path, toml::to_string(&provider_config).unwrap()).unwrap();
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn spawn_docs_writer_server(expected_requests: usize) -> (String, thread::JoinHandle<usize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());

    let handle = thread::spawn(move || {
        let mut handled = 0usize;
        while handled < expected_requests {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = Vec::new();
            let mut chunk = [0u8; 4096];
            let mut header_end = None;

            loop {
                let read = stream.read(&mut chunk).unwrap();
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
                if header_end.is_none() {
                    header_end = find_header_end(&buffer);
                }
                if let Some(end) = header_end {
                    let headers = String::from_utf8_lossy(&buffer[..end]);
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            let lower = line.to_ascii_lowercase();
                            lower
                                .strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if buffer.len() >= end + content_length {
                        break;
                    }
                }
            }

            let request_body = String::from_utf8_lossy(&buffer);
            let completion = if request_body.contains("Build evidence for README generation") {
                r#"{"claims":[{"claim_id":"c1","statement":"Provides greeting helpers.","evidence_path":"src/lib.rs","evidence_symbol":"greet","evidence_quote":"pub fn greet(name: &str) -> String"}]}"#
            } else if request_body.contains("Validate each claim against the provided evidence") {
                r#"{"verified_claims":[{"claim_id":"c1","statement":"Provides greeting helpers.","evidence_path":"src/lib.rs","evidence_symbol":"greet","evidence_quote":"pub fn greet(name: &str) -> String"}],"rejected_claims":[],"reasons":[]}"#
            } else if request_body.contains("Build a structured README draft") {
                r#"{"title":"Workflow Library","purpose":"Provides greeting helpers.","usage":"Call greet with a user name."}"#
            } else {
                "# Workflow Library\n\n## Purpose\n\nProvides greeting helpers.\n\n## Usage\n\nCall `greet` with a user name."
            };
            let response_body = format!(
                r#"{{"id":"test","object":"chat.completion","created":0,"model":"test-model","choices":[{{"index":0,"message":{{"role":"assistant","content":{}}},"finish_reason":"stop"}}],"usage":{{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}}}"#,
                serde_json::to_string(completion).unwrap()
            );
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            stream.write_all(response.as_bytes()).unwrap();
            handled += 1;
        }

        handled
    });

    (endpoint, handle)
}

#[test]
fn workflow_execute_routes_docs_writer_through_task_path() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        meld::init::initialize_workflows(false).unwrap();

        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(workspace_root.join("src")).unwrap();
        fs::write(
            workspace_root.join("src").join("lib.rs"),
            "pub fn greet(name: &str) -> String { format!(\"hello {}\", name) }",
        )
        .unwrap();

        create_test_agent("docs-writer", Some("docs_writer_thread_v1"));
        let (endpoint, server_handle) = spawn_docs_writer_server(4);
        create_test_provider("test-provider", &endpoint);

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let output = run_context
            .execute(&Commands::Workflow {
                command: WorkflowCommands::Execute {
                    workflow_id: "docs_writer_thread_v1".to_string(),
                    node: None,
                    path: Some(PathBuf::from("src")),
                    path_positional: None,
                    agent: "docs-writer".to_string(),
                    provider: "test-provider".to_string(),
                    frame_type: None,
                    force: true,
                },
            })
            .unwrap();

        let handled = server_handle.join().unwrap();
        assert_eq!(handled, 4);
        assert!(output.contains("workflow_id=docs_writer_thread_v1"));
        assert!(output.contains("skipped=false"));

        let node_id = meld::workspace::resolve_workspace_node_id(
            run_context.api(),
            &workspace_root,
            Some(PathBuf::from("src").as_path()),
            None,
            false,
        )
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
        let thread = state_store.load_thread(&thread_id).unwrap().unwrap();
        assert_eq!(thread.status, WorkflowThreadStatus::Completed);
    });
}

#[test]
fn workflow_execute_reuses_existing_final_head_when_task_thread_state_drifted() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        meld::init::initialize_workflows(false).unwrap();

        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(workspace_root.join("src")).unwrap();
        fs::write(
            workspace_root.join("src").join("lib.rs"),
            "pub fn greet(name: &str) -> String { format!(\"hello {}\", name) }",
        )
        .unwrap();

        create_test_agent("docs-writer", Some("docs_writer_thread_v1"));
        let (endpoint, first_server_handle) = spawn_docs_writer_server(4);
        create_test_provider("test-provider", &endpoint);

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let first_output = run_context
            .execute(&Commands::Workflow {
                command: WorkflowCommands::Execute {
                    workflow_id: "docs_writer_thread_v1".to_string(),
                    node: None,
                    path: Some(PathBuf::from("src")),
                    path_positional: None,
                    agent: "docs-writer".to_string(),
                    provider: "test-provider".to_string(),
                    frame_type: None,
                    force: true,
                },
            })
            .unwrap();

        assert!(first_output.contains("skipped=false"));
        assert_eq!(first_server_handle.join().unwrap(), 4);

        let node_id = meld::workspace::resolve_workspace_node_id(
            run_context.api(),
            &workspace_root,
            Some(PathBuf::from("src").as_path()),
            None,
            false,
        )
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
            .upsert_thread(&meld::workflow::state_store::WorkflowThreadRecord {
                thread_id: thread_id.clone(),
                workflow_id: "docs_writer_thread_v1".to_string(),
                node_id: hex::encode(node_id),
                frame_type: "context-docs-writer".to_string(),
                status: WorkflowThreadStatus::Failed,
                next_turn_seq: 1,
                updated_at_ms: 0,
            })
            .unwrap();

        let second_output = run_context
            .execute(&Commands::Workflow {
                command: WorkflowCommands::Execute {
                    workflow_id: "docs_writer_thread_v1".to_string(),
                    node: None,
                    path: Some(PathBuf::from("src")),
                    path_positional: None,
                    agent: "docs-writer".to_string(),
                    provider: "test-provider".to_string(),
                    frame_type: None,
                    force: false,
                },
            })
            .unwrap();

        assert!(second_output.contains("skipped=true"));

        let repaired_thread = state_store.load_thread(&thread_id).unwrap().unwrap();
        assert_eq!(repaired_thread.status, WorkflowThreadStatus::Completed);
    });
}
