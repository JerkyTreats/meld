use crate::integration::with_xdg_env;
use meld::agent::{AgentRole, AgentStorage, XdgAgentStorage};
use meld::capability::{CapabilityCatalog, CapabilityExecutionContext, CapabilityExecutorRegistry};
use meld::cli::{Commands, RunContext};
use meld::config::{xdg, AgentConfig, ProviderConfig, ProviderType};
use meld::context::frame::{Basis, Frame};
use meld::context::query::ContextView;
use meld::metadata::frame_write_contract::{
    build_generated_metadata, generated_metadata_input_from_payload,
};
use meld::provider::capability::ProviderExecuteChatCapability;
use meld::provider::{ProviderExecutionBinding, ProviderRuntimeOverrides};
use meld::task::templates::prepare_registered_workflow_task_run;
use meld::task::{
    compile_task_expansion_request, execute_task_to_completion,
    parse_task_expansion_request_artifact, TaskExecutor, WorkflowPackageTriggerRequest,
};
use meld::workspace::capability::WorkspaceResolveNodeIdCapability;
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
    spawn_docs_writer_server_with_shape(expected_requests, false)
}

fn spawn_wrapped_docs_writer_server(
    expected_requests: usize,
) -> (String, thread::JoinHandle<usize>) {
    spawn_docs_writer_server_with_shape(expected_requests, true)
}

fn spawn_docs_writer_server_with_shape(
    expected_requests: usize,
    wrap_structured_output: bool,
) -> (String, thread::JoinHandle<usize>) {
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
                if wrap_structured_output {
                    "```json\n{\"claims\":[{\"claim_id\":\"c1\",\"statement\":\"Provides greeting helpers.\",\"evidence_path\":\"src/lib.rs\",\"evidence_symbol\":\"greet\",\"evidence_quote\":\"pub fn greet(name: &str) -> String\"}]}\n```"
                } else {
                    r#"{"claims":[{"claim_id":"c1","statement":"Provides greeting helpers.","evidence_path":"src/lib.rs","evidence_symbol":"greet","evidence_quote":"pub fn greet(name: &str) -> String"}]}"#
                }
            } else if request_body.contains("Validate each claim against the provided evidence") {
                if wrap_structured_output {
                    "Here is the JSON you requested.\n{\"verified_claims\":[{\"claim_id\":\"c1\",\"statement\":\"Provides greeting helpers.\",\"evidence_path\":\"src/lib.rs\",\"evidence_symbol\":\"greet\",\"evidence_quote\":\"pub fn greet(name: &str) -> String\"}],\"rejected_claims\":[],\"reasons\":[]}"
                } else {
                    r#"{"verified_claims":[{"claim_id":"c1","statement":"Provides greeting helpers.","evidence_path":"src/lib.rs","evidence_symbol":"greet","evidence_quote":"pub fn greet(name: &str) -> String"}],"rejected_claims":[],"reasons":[]}"#
                }
            } else if request_body.contains("Build a structured README draft") {
                if wrap_structured_output {
                    "```json\n{\"title\":\"Workspace Library\",\"purpose\":\"Provides greeting helpers.\",\"usage\":\"Call greet with a user name.\"}\n```"
                } else {
                    r#"{"title":"Workspace Library","purpose":"Provides greeting helpers.","usage":"Call greet with a user name."}"#
                }
            } else {
                "# Workspace Library\n\n## Purpose\n\nProvides greeting helpers.\n\n## Usage\n\nCall `greet` with a user name."
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

fn register_phase_four_capabilities(
    catalog: &mut CapabilityCatalog,
    registry: &mut CapabilityExecutorRegistry,
) {
    registry
        .register(catalog, WorkspaceResolveNodeIdCapability)
        .unwrap();
    registry
        .register(
            catalog,
            meld::workspace::capability::WorkspaceFilterFrameHeadPublishCapability,
        )
        .unwrap();
    registry
        .register(
            catalog,
            meld::workspace::capability::WorkspaceWriteFrameHeadCapability,
        )
        .unwrap();
    registry
        .register(
            catalog,
            meld::merkle_traversal::capability::MerkleTraversalCapability,
        )
        .unwrap();
    registry
        .register(
            catalog,
            meld::context::capability::ContextGeneratePrepareCapability,
        )
        .unwrap();
    registry
        .register(catalog, ProviderExecuteChatCapability)
        .unwrap();
    registry
        .register(
            catalog,
            meld::context::capability::ContextGenerateFinalizeCapability,
        )
        .unwrap();
}

fn apply_initial_expansion(
    api: &meld::api::ContextApi,
    executor: &mut TaskExecutor,
    catalog: &CapabilityCatalog,
    registry: &CapabilityExecutorRegistry,
) {
    let payload = executor
        .release_ready_invocations(CapabilityExecutionContext::default())
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    let traversal_instance = executor
        .compiled_task()
        .capability_instances
        .iter()
        .find(|instance| instance.capability_instance_id == payload.capability_instance_id)
        .unwrap()
        .clone();
    let runtime_init = registry.runtime_init_for(&traversal_instance).unwrap();
    let invoker = registry
        .get(
            &traversal_instance.capability_type_id,
            traversal_instance.capability_version,
        )
        .unwrap()
        .clone();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt
        .block_on(invoker.invoke(api, &runtime_init, &payload, None))
        .unwrap();
    let mut expansion_requests = Vec::new();
    for artifact in &result.emitted_artifacts {
        if let Some(request) = parse_task_expansion_request_artifact(artifact).unwrap() {
            expansion_requests.push((artifact.artifact_id.clone(), request));
        }
    }
    executor
        .record_success(&payload.invocation_id, result.emitted_artifacts)
        .unwrap();
    for (source_artifact_id, request) in expansion_requests {
        let delta =
            compile_task_expansion_request(api, executor.compiled_task(), &request, catalog)
                .unwrap();
        executor
            .apply_task_expansion(
                &request.expansion_id,
                &request.expansion_kind,
                &source_artifact_id,
                delta,
            )
            .unwrap();
    }
}

#[test]
fn docs_writer_task_compiles_bottom_up_dependencies() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        meld::init::initialize_workflows(false).unwrap();

        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(workspace_root.join("src")).unwrap();
        fs::write(
            workspace_root.join("src").join("lib.rs"),
            "pub fn greet() {}",
        )
        .unwrap();

        create_test_agent("docs-writer", Some("docs_writer_thread_v1"));
        create_test_provider("test-provider", "http://127.0.0.1:9");

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let registered_profile = run_context
            .workflow_registry()
            .read()
            .get("docs_writer_thread_v1")
            .unwrap()
            .clone();
        let mut catalog = CapabilityCatalog::new();
        let mut registry = CapabilityExecutorRegistry::new();
        register_phase_four_capabilities(&mut catalog, &mut registry);

        let prepared = prepare_registered_workflow_task_run(
            run_context.api(),
            &workspace_root,
            &registered_profile,
            &WorkflowPackageTriggerRequest {
                package_id: "docs_writer".to_string(),
                workflow_id: "docs_writer_thread_v1".to_string(),
                node_id: None,
                path: Some(PathBuf::from("src")),
                agent_id: "docs-writer".to_string(),
                provider: ProviderExecutionBinding::new(
                    "test-provider",
                    ProviderRuntimeOverrides::default(),
                )
                .unwrap(),
                frame_type: "context-docs-writer".to_string(),
                force: true,
                session_id: None,
            },
            &catalog,
        )
        .unwrap();

        assert_eq!(prepared.compiled_task.capability_instances.len(), 1);
        assert!(prepared
            .compiled_task
            .init_slots
            .iter()
            .any(|slot| slot.artifact_type_id == "task_expansion_template"));

        let mut executor = TaskExecutor::new(
            prepared.compiled_task.clone(),
            prepared.init_payload.clone(),
            "repo_docs_writer_compile",
        )
        .unwrap();
        apply_initial_expansion(run_context.api(), &mut executor, &catalog, &registry);

        assert!(executor
            .compiled_task()
            .dependency_edges
            .iter()
            .any(|edge| edge.reason == "publish_after_generation_head"));
        assert!(executor
            .compiled_task()
            .init_slots
            .iter()
            .any(|slot| slot.init_slot_id.starts_with("node_ref::")));
    });
}

#[test]
fn docs_writer_task_runs_to_completion() {
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

        let registered_profile = run_context
            .workflow_registry()
            .read()
            .get("docs_writer_thread_v1")
            .unwrap()
            .clone();
        let mut catalog = CapabilityCatalog::new();
        let mut registry = CapabilityExecutorRegistry::new();
        register_phase_four_capabilities(&mut catalog, &mut registry);

        let prepared = prepare_registered_workflow_task_run(
            run_context.api(),
            &workspace_root,
            &registered_profile,
            &WorkflowPackageTriggerRequest {
                package_id: "docs_writer".to_string(),
                workflow_id: "docs_writer_thread_v1".to_string(),
                node_id: None,
                path: Some(PathBuf::from("src")),
                agent_id: "docs-writer".to_string(),
                provider: ProviderExecutionBinding::new(
                    "test-provider",
                    ProviderRuntimeOverrides::default(),
                )
                .unwrap(),
                frame_type: "context-docs-writer".to_string(),
                force: true,
                session_id: Some("session_docs_writer".to_string()),
            },
            &catalog,
        )
        .unwrap();

        let mut executor = TaskExecutor::new(
            prepared.compiled_task.clone(),
            prepared.init_payload.clone(),
            "repo_docs_writer",
        )
        .unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let summary = rt
            .block_on(execute_task_to_completion(
                run_context.api(),
                &mut executor,
                &catalog,
                &registry,
                None,
                None,
            ))
            .unwrap();

        let handled = server_handle.join().unwrap();
        assert_eq!(handled, 4);
        assert_eq!(
            summary.completed_instances,
            executor.compiled_task().capability_instances.len()
        );
        assert!(executor.compiled_task().capability_instances.len() > 1);
        assert_eq!(executor.expansion_records().len(), 2);

        let view = ContextView::builder()
            .max_frames(1)
            .recent()
            .by_type("context-docs-writer".to_string())
            .by_agent("docs-writer".to_string())
            .build();
        let context = run_context
            .api()
            .get_node(prepared.target_node_id, view)
            .unwrap();
        assert_eq!(context.frames.len(), 1);
        let body = String::from_utf8_lossy(&context.frames[0].content);
        assert!(body.contains("# Workspace Library"));
    });
}

#[test]
fn docs_writer_task_accepts_wrapped_structured_output() {
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
        let (endpoint, server_handle) = spawn_wrapped_docs_writer_server(4);
        create_test_provider("test-provider", &endpoint);

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let registered_profile = run_context
            .workflow_registry()
            .read()
            .get("docs_writer_thread_v1")
            .unwrap()
            .clone();
        let mut catalog = CapabilityCatalog::new();
        let mut registry = CapabilityExecutorRegistry::new();
        register_phase_four_capabilities(&mut catalog, &mut registry);

        let prepared = prepare_registered_workflow_task_run(
            run_context.api(),
            &workspace_root,
            &registered_profile,
            &WorkflowPackageTriggerRequest {
                package_id: "docs_writer".to_string(),
                workflow_id: "docs_writer_thread_v1".to_string(),
                node_id: None,
                path: Some(PathBuf::from("src")),
                agent_id: "docs-writer".to_string(),
                provider: ProviderExecutionBinding::new(
                    "test-provider",
                    ProviderRuntimeOverrides::default(),
                )
                .unwrap(),
                frame_type: "context-docs-writer".to_string(),
                force: true,
                session_id: Some("session_docs_writer_wrapped".to_string()),
            },
            &catalog,
        )
        .unwrap();

        let mut executor = TaskExecutor::new(
            prepared.compiled_task.clone(),
            prepared.init_payload.clone(),
            "repo_docs_writer_wrapped",
        )
        .unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(execute_task_to_completion(
            run_context.api(),
            &mut executor,
            &catalog,
            &registry,
            None,
            None,
        ))
        .unwrap();

        let handled = server_handle.join().unwrap();
        assert_eq!(handled, 4);

        let view = ContextView::builder()
            .max_frames(1)
            .recent()
            .by_type("context-docs-writer".to_string())
            .by_agent("docs-writer".to_string())
            .build();
        let context = run_context
            .api()
            .get_node(prepared.target_node_id, view)
            .unwrap();
        assert_eq!(context.frames.len(), 1);
        let body = String::from_utf8_lossy(&context.frames[0].content);
        assert!(body.contains("# Workspace Library"));
    });
}

#[test]
fn docs_writer_task_expansion_is_idempotent() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        meld::init::initialize_workflows(false).unwrap();

        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(workspace_root.join("src")).unwrap();
        fs::write(
            workspace_root.join("src").join("lib.rs"),
            "pub fn greet() {}",
        )
        .unwrap();

        create_test_agent("docs-writer", Some("docs_writer_thread_v1"));
        create_test_provider("test-provider", "http://127.0.0.1:9");

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let registered_profile = run_context
            .workflow_registry()
            .read()
            .get("docs_writer_thread_v1")
            .unwrap()
            .clone();
        let mut catalog = CapabilityCatalog::new();
        let mut registry = CapabilityExecutorRegistry::new();
        register_phase_four_capabilities(&mut catalog, &mut registry);

        let prepared = prepare_registered_workflow_task_run(
            run_context.api(),
            &workspace_root,
            &registered_profile,
            &WorkflowPackageTriggerRequest {
                package_id: "docs_writer".to_string(),
                workflow_id: "docs_writer_thread_v1".to_string(),
                node_id: None,
                path: Some(PathBuf::from("src")),
                agent_id: "docs-writer".to_string(),
                provider: ProviderExecutionBinding::new(
                    "test-provider",
                    ProviderRuntimeOverrides::default(),
                )
                .unwrap(),
                frame_type: "context-docs-writer".to_string(),
                force: true,
                session_id: None,
            },
            &catalog,
        )
        .unwrap();

        let mut executor = TaskExecutor::new(
            prepared.compiled_task.clone(),
            prepared.init_payload.clone(),
            "repo_docs_writer_idempotent",
        )
        .unwrap();
        let payload = executor
            .release_ready_invocations(CapabilityExecutionContext::default())
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let traversal_instance = executor
            .compiled_task()
            .capability_instances
            .iter()
            .find(|instance| instance.capability_instance_id == payload.capability_instance_id)
            .unwrap()
            .clone();
        let runtime_init = registry.runtime_init_for(&traversal_instance).unwrap();
        let invoker = registry
            .get(
                &traversal_instance.capability_type_id,
                traversal_instance.capability_version,
            )
            .unwrap()
            .clone();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt
            .block_on(invoker.invoke(run_context.api(), &runtime_init, &payload, None))
            .unwrap();
        let (source_artifact_id, request) = result
            .emitted_artifacts
            .iter()
            .find_map(|artifact| {
                parse_task_expansion_request_artifact(artifact)
                    .unwrap()
                    .map(|request| (artifact.artifact_id.clone(), request))
            })
            .unwrap();
        executor
            .record_success(&payload.invocation_id, result.emitted_artifacts)
            .unwrap();

        let delta = compile_task_expansion_request(
            run_context.api(),
            executor.compiled_task(),
            &request,
            &catalog,
        )
        .unwrap();
        assert!(executor
            .apply_task_expansion(
                &request.expansion_id,
                &request.expansion_kind,
                &source_artifact_id,
                delta.clone(),
            )
            .unwrap());

        let instance_count = executor.compiled_task().capability_instances.len();
        let edge_count = executor.compiled_task().dependency_edges.len();
        let init_slot_count = executor.compiled_task().init_slots.len();

        assert!(!executor
            .apply_task_expansion(
                &request.expansion_id,
                &request.expansion_kind,
                &source_artifact_id,
                delta,
            )
            .unwrap());
        assert_eq!(executor.expansion_records().len(), 1);
        assert_eq!(
            executor.compiled_task().capability_instances.len(),
            instance_count
        );
        assert_eq!(executor.compiled_task().dependency_edges.len(), edge_count);
        assert_eq!(executor.compiled_task().init_slots.len(), init_slot_count);
    });
}

#[test]
fn docs_writer_task_reuses_existing_child_readme_outputs() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        meld::init::initialize_workflows(false).unwrap();

        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(workspace_root.join("src").join("record_contracts")).unwrap();
        fs::write(
            workspace_root.join("src").join("record_contracts").join("lib.rs"),
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

        let child_node_id = meld::workspace::resolve_workspace_node_id(
            run_context.api(),
            &workspace_root,
            Some(PathBuf::from("src/record_contracts").as_path()),
            None,
            false,
        )
        .unwrap();
        let child_frame = Frame::new(
            Basis::Node(child_node_id),
            b"# Existing Child README".to_vec(),
            "context-docs-writer".to_string(),
            "docs-writer".to_string(),
            build_generated_metadata(&generated_metadata_input_from_payload(
                "docs-writer",
                "test-provider",
                "test-model",
                "local_custom",
                "child prompt",
                "child context",
            )),
        )
        .unwrap();
        run_context
            .api()
            .put_frame(child_node_id, child_frame, "docs-writer".to_string())
            .unwrap();

        let registered_profile = run_context
            .workflow_registry()
            .read()
            .get("docs_writer_thread_v1")
            .unwrap()
            .clone();
        let mut catalog = CapabilityCatalog::new();
        let mut registry = CapabilityExecutorRegistry::new();
        register_phase_four_capabilities(&mut catalog, &mut registry);

        let prepared = prepare_registered_workflow_task_run(
            run_context.api(),
            &workspace_root,
            &registered_profile,
            &WorkflowPackageTriggerRequest {
                package_id: "docs_writer".to_string(),
                workflow_id: "docs_writer_thread_v1".to_string(),
                node_id: None,
                path: Some(PathBuf::from("src")),
                agent_id: "docs-writer".to_string(),
                provider: ProviderExecutionBinding::new(
                    "test-provider",
                    ProviderRuntimeOverrides::default(),
                )
                .unwrap(),
                frame_type: "context-docs-writer".to_string(),
                force: false,
                session_id: Some("session_docs_writer".to_string()),
            },
            &catalog,
        )
        .unwrap();

        assert_eq!(prepared.compiled_task.capability_instances.len(), 1);

        let mut executor = TaskExecutor::new(
            prepared.compiled_task.clone(),
            prepared.init_payload.clone(),
            "repo_docs_writer_existing_child",
        )
        .unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let summary = rt
            .block_on(execute_task_to_completion(
                run_context.api(),
                &mut executor,
                &catalog,
                &registry,
                None,
                None,
            ))
            .unwrap();

        let handled = server_handle.join().unwrap();
        assert_eq!(handled, 4);
        assert_eq!(
            summary.completed_instances,
            executor.compiled_task().capability_instances.len()
        );
        assert!(executor
            .compiled_task()
            .init_slots
            .iter()
            .any(|slot| slot.artifact_type_id == "readme_final"));

        let root_node_id = meld::workspace::resolve_workspace_node_id(
            run_context.api(),
            &workspace_root,
            Some(PathBuf::from("src").as_path()),
            None,
            false,
        )
        .unwrap();
        let root_context = run_context
            .api()
            .get_node(
                root_node_id,
                ContextView::builder()
                    .max_frames(1)
                    .recent()
                    .by_type("context-docs-writer".to_string())
                    .by_agent("docs-writer".to_string())
                    .build(),
            )
            .unwrap();
        assert_eq!(root_context.frames.len(), 1);

        let child_context = run_context
            .api()
            .get_node(
                child_node_id,
                ContextView::builder()
                    .max_frames(1)
                    .recent()
                    .by_type("context-docs-writer".to_string())
                    .by_agent("docs-writer".to_string())
                    .build(),
            )
            .unwrap();
        assert_eq!(child_context.frames.len(), 1);
        let body = String::from_utf8_lossy(&child_context.frames[0].content);
        assert_eq!(body, "# Existing Child README");
    });
}
