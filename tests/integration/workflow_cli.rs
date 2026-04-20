//! Integration tests for workflow CLI commands.

use meld::agent::{AgentRole, AgentStorage, XdgAgentStorage};
use meld::cli::{Commands, RunContext, WorkflowCommands};
use meld::config::{xdg, AgentConfig, ProviderConfig, ProviderType};
use meld::error::ApiError;
use meld::workflow::{build_target_execution_request, execute_workflow_target};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

use crate::integration::with_xdg_env;

fn initialize_default_workflows() {
    meld::init::initialize_workflows(false).unwrap();
}

fn create_test_provider_with_endpoint(
    provider_name: &str,
    provider_type: ProviderType,
    endpoint: &str,
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
        endpoint: Some(endpoint.to_string()),
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

fn create_workflow_test_agent(
    agent_id: &str,
    workflow_id: Option<&str>,
) -> Result<PathBuf, ApiError> {
    let agents_dir = XdgAgentStorage::new().agents_dir()?;
    fs::create_dir_all(&agents_dir)
        .map_err(|err| ApiError::ConfigError(format!("Failed to create agents dir: {}", err)))?;
    let config_path = agents_dir.join(format!("{}.toml", agent_id));

    let mut metadata = std::collections::HashMap::new();
    metadata.insert(
        "user_prompt_file".to_string(),
        "Analyze file {path}".to_string(),
    );
    metadata.insert(
        "user_prompt_directory".to_string(),
        "Analyze directory {path}".to_string(),
    );

    let agent_config = AgentConfig {
        agent_id: agent_id.to_string(),
        role: AgentRole::Writer,
        system_prompt: Some("You are a workflow test writer.".to_string()),
        system_prompt_path: None,
        workflow_id: workflow_id.map(ToString::to_string),
        metadata: metadata.into(),
    };

    let toml = toml::to_string(&agent_config)
        .map_err(|err| ApiError::ConfigError(format!("Failed to encode agent config: {}", err)))?;
    fs::write(&config_path, toml)
        .map_err(|err| ApiError::ConfigError(format!("Failed to write agent config: {}", err)))?;
    Ok(config_path)
}

fn write_runtime_workflow(workflow_id: &str, prompt_text: &str) -> Result<(), ApiError> {
    let workflow_dir = meld::config::WorkflowConfig::default().resolve_user_profile_dir()?;
    fs::create_dir_all(&workflow_dir)
        .map_err(|err| ApiError::ConfigError(format!("Failed to create workflow dir: {}", err)))?;
    let prompt_rel = format!("prompts/{}/turn.md", workflow_id);
    let prompt_path = workflow_dir.join(&prompt_rel);
    if let Some(parent) = prompt_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            ApiError::ConfigError(format!("Failed to create workflow prompt dir: {}", err))
        })?;
    }
    fs::write(&prompt_path, prompt_text).map_err(|err| {
        ApiError::ConfigError(format!("Failed to write workflow prompt: {}", err))
    })?;

    let workflow_path = workflow_dir.join(format!("{}.yaml", workflow_id));
    let workflow_yaml = format!(
        r#"workflow_id: {workflow_id}
version: 1
title: Runtime Workflow
description: Runtime verification workflow
thread_policy:
  start_conditions: {{}}
  dedupe_key_fields:
    - workflow_id
    - target_node_id
  max_turn_retries: 1
turns:
  - turn_id: runtime_turn
    seq: 1
    title: Runtime Turn
    prompt_ref: {prompt_rel}
    input_refs:
      - target_context
    output_type: readme_final
    gate_id: runtime_gate
    retry_limit: 1
    timeout_ms: 60000
gates:
  - gate_id: runtime_gate
    gate_type: no_semantic_drift
    required_fields: []
    rules: null
    fail_on_violation: false
artifact_policy:
  store_output: true
  store_prompt_render: true
  store_context_payload: true
  max_output_bytes: 262144
failure_policy:
  mode: fail_fast
  resume_from_failed_turn: true
  stop_on_gate_fail: false
target_agent_id: docs-writer
target_frame_type: context-docs-writer
final_artifact_type: readme_final
"#
    );
    fs::write(&workflow_path, workflow_yaml).map_err(|err| {
        ApiError::ConfigError(format!("Failed to write workflow profile: {}", err))
    })?;
    Ok(())
}

fn spawn_capture_server(
    response_body: &str,
) -> (String, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let response_body = response_body.to_string();
    let (tx, rx) = mpsc::channel();

    let handle = thread::spawn(move || {
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
                let body_start = end + 4;
                while buffer.len() < body_start + content_length {
                    let read = stream.read(&mut chunk).unwrap();
                    if read == 0 {
                        break;
                    }
                    buffer.extend_from_slice(&chunk[..read]);
                }
                let body =
                    String::from_utf8_lossy(&buffer[body_start..body_start + content_length])
                        .to_string();
                tx.send(body).unwrap();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                stream.write_all(response.as_bytes()).unwrap();
                stream.flush().unwrap();
                break;
            }
        }
    });

    (endpoint, rx, handle)
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
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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

fn workflow_thread_id(node_id: meld::types::NodeID, frame_type: &str) -> String {
    let thread_payload = format!(
        "{}:{}:{}",
        "docs_writer_thread_v1",
        hex::encode(node_id),
        frame_type
    );
    let thread_digest = blake3::hash(thread_payload.as_bytes()).to_hex().to_string();
    format!("thread-{}", &thread_digest[..16])
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
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
fn workflow_execute_uses_workflow_runtime_and_can_skip_completed_thread() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(workspace_root.join("src")).unwrap();
        let target_path = workspace_root.join("src");
        fs::write(
            target_path.join("lib.rs"),
            "pub fn greet(name: &str) -> String { format!(\"hello {}\", name) }",
        )
        .unwrap();

        initialize_default_workflows();
        create_workflow_test_agent("docs-writer", Some("docs_writer_thread_v1")).unwrap();
        let (endpoint, server_handle) = spawn_docs_writer_server(4);
        create_test_provider_with_endpoint("test-provider", ProviderType::LocalCustom, &endpoint)
            .unwrap();

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let node_id = meld::workspace::resolve_workspace_node_id(
            run_context.api(),
            &workspace_root,
            Some(target_path.as_path()),
            None,
            false,
        )
        .unwrap();

        let thread_id = workflow_thread_id(node_id, "context-docs-writer");

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
        assert_eq!(server_handle.join().unwrap(), 4);

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
        fs::create_dir_all(workspace_root.join("src")).unwrap();
        let target_path = workspace_root.join("src");
        fs::write(
            target_path.join("lib.rs"),
            "pub fn greet(name: &str) -> String { format!(\"hello {}\", name) }",
        )
        .unwrap();

        initialize_default_workflows();
        create_workflow_test_agent("docs-writer", Some("docs_writer_thread_v1")).unwrap();
        let (endpoint, server_handle) = spawn_docs_writer_server(4);
        create_test_provider_with_endpoint("test-provider", ProviderType::LocalCustom, &endpoint)
            .unwrap();

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let node_id = meld::workspace::resolve_workspace_node_id(
            run_context.api(),
            &workspace_root,
            Some(target_path.as_path()),
            None,
            false,
        )
        .unwrap();

        let thread_id = workflow_thread_id(node_id, "context-docs-writer");
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
        assert_eq!(server_handle.join().unwrap(), 4);
        let first_frame_id = run_context
            .api()
            .get_head(&node_id, "context-docs-writer")
            .unwrap()
            .unwrap();

        let request = build_target_execution_request(
            run_context.api(),
            node_id,
            "docs-writer".to_string(),
            meld::provider::ProviderExecutionBinding::new(
                "test-provider",
                meld::provider::ProviderRuntimeOverrides::default(),
            )
            .unwrap(),
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

        assert_eq!(result.final_frame_id, first_frame_id);
        assert!(result.reused_existing_head);
        assert_eq!(result.workflow_id.as_deref(), Some("docs_writer_thread_v1"));
        assert_eq!(result.thread_id.as_deref(), Some(thread_id.as_str()));
        assert_eq!(result.turns_completed, 0);
    });
}

#[test]
fn workflow_execute_uses_runtime_modified_xdg_prompt_in_provider_request() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        let file_path = workspace_root.join("doc.md");
        fs::write(&file_path, "# hello").unwrap();

        let workflow_id = "runtime_modified_workflow";
        let prompt_marker = "RUNTIME WORKFLOW PROMPT MARKER";
        write_runtime_workflow(workflow_id, prompt_marker).unwrap();
        create_workflow_test_agent("docs-writer", Some(workflow_id)).unwrap();

        let response_body = r##"{"id":"test","object":"chat.completion","created":0,"model":"test-model","choices":[{"index":0,"message":{"role":"assistant","content":"# Runtime Workflow\n\nGenerated body"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"##;
        let (endpoint, body_rx, handle) = spawn_capture_server(response_body);
        create_test_provider_with_endpoint("test-provider", ProviderType::LocalCustom, &endpoint)
            .unwrap();

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let output = run_context
            .execute(&Commands::Workflow {
                command: WorkflowCommands::Execute {
                    workflow_id: workflow_id.to_string(),
                    node: None,
                    path: Some(file_path.clone()),
                    path_positional: None,
                    agent: "docs-writer".to_string(),
                    provider: "test-provider".to_string(),
                    frame_type: None,
                    force: true,
                },
            })
            .unwrap();

        let request_body = body_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        handle.join().unwrap();

        assert!(output.contains("workflow_id=runtime_modified_workflow"));
        assert!(request_body.contains(prompt_marker));
        assert!(request_body.contains("Complete workflow turn 'runtime_turn'"));
    });
}
