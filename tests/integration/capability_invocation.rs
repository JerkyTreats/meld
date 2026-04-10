use crate::integration::with_xdg_env;
use meld::agent::{AgentRole, AgentStorage, XdgAgentStorage};
use meld::capability::{
    ArtifactValueRef, BoundBindingValue, BoundCapabilityInstance, CapabilityCatalog,
    CapabilityExecutionContext, CapabilityExecutorRegistry, CapabilityInvocationPayload,
    InputValueSource, SuppliedInputValue, SuppliedValueRef, UpstreamLineage,
};
use meld::cli::{Commands, RunContext};
use meld::config::{xdg, AgentConfig, ProviderConfig, ProviderType};
use meld::context::frame::{Basis, Frame};
use meld::context::query::ContextView;
use meld::merkle_traversal::capability::MerkleTraversalCapability;
use meld::metadata::frame_write_contract::{
    build_generated_metadata, generated_metadata_input_from_payload,
};
use meld::provider::capability::ProviderExecuteChatCapability;
use meld::provider::{ProviderExecutionBinding, ProviderRuntimeOverrides};
use meld::workspace::capability::WorkspaceResolveNodeIdCapability;
use serde_json::json;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use tempfile::TempDir;

fn create_test_agent(agent_id: &str) {
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
        workflow_id: None,
        metadata: metadata.into(),
    };

    let toml = toml::to_string(&agent_config).unwrap();
    fs::write(&config_path, toml).unwrap();
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

    let toml = toml::to_string(&provider_config).unwrap();
    fs::write(&config_path, toml).unwrap();
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
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
                if buffer.len() >= end + content_length {
                    break;
                }
            }
        }

        tx.send(String::from_utf8_lossy(&buffer).to_string())
            .unwrap();

        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });

    (endpoint, rx, handle)
}

fn register_phase_four_capabilities(
    catalog: &mut CapabilityCatalog,
    registry: &mut CapabilityExecutorRegistry,
) {
    registry
        .register(catalog, WorkspaceResolveNodeIdCapability)
        .unwrap();
    registry
        .register(catalog, MerkleTraversalCapability)
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

fn single_input_payload(
    capability_instance_id: &str,
    invocation_id: &str,
    slot_id: &str,
    artifact_type_id: &str,
    content: serde_json::Value,
) -> CapabilityInvocationPayload {
    CapabilityInvocationPayload {
        invocation_id: invocation_id.to_string(),
        capability_instance_id: capability_instance_id.to_string(),
        supplied_inputs: vec![SuppliedInputValue {
            slot_id: slot_id.to_string(),
            source: InputValueSource::ArtifactHandoff,
            value: SuppliedValueRef::Artifact(ArtifactValueRef {
                artifact_id: format!("{invocation_id}::{slot_id}"),
                artifact_type_id: artifact_type_id.to_string(),
                schema_version: 1,
                content,
            }),
        }],
        upstream_lineage: Some(UpstreamLineage {
            task_id: "task_docs_writer".to_string(),
            task_run_id: "taskrun_phase4".to_string(),
            capability_path: vec![capability_instance_id.to_string()],
            batch_index: None,
            node_index: None,
            repair_scope: None,
        }),
        execution_context: CapabilityExecutionContext {
            attempt: 1,
            trace_id: Some("trace_phase4".to_string()),
            deadline_ms: None,
            cancellation_key: None,
            dispatch_priority: None,
        },
    }
}

#[test]
fn workspace_and_traversal_capabilities_follow_scanned_tree() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(workspace_root.join("pkg-a")).unwrap();
        fs::create_dir_all(workspace_root.join("pkg-b")).unwrap();
        fs::write(workspace_root.join("pkg-a").join("README.md"), "# pkg a").unwrap();
        fs::write(workspace_root.join("pkg-b").join("README.md"), "# pkg b").unwrap();

        create_test_agent("docs-writer");
        create_test_provider("test-provider", "http://127.0.0.1:9");

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let mut catalog = CapabilityCatalog::new();
        let mut registry = CapabilityExecutorRegistry::new();
        register_phase_four_capabilities(&mut catalog, &mut registry);

        let resolve_instance = BoundCapabilityInstance {
            capability_instance_id: "capinst_workspace_resolve".to_string(),
            capability_type_id: "workspace_resolve_node_id".to_string(),
            capability_version: 1,
            scope_ref: workspace_root.display().to_string(),
            scope_kind: "workspace".to_string(),
            binding_values: Vec::new(),
            input_wiring: Vec::new(),
        };
        let resolve_runtime = registry.runtime_init_for(&resolve_instance).unwrap();
        let resolve_payload = single_input_payload(
            &resolve_instance.capability_instance_id,
            "invk_workspace_resolve",
            "target_selector",
            "target_selector",
            json!({
                "path": ".",
            }),
        );

        let rt = tokio::runtime::Runtime::new().unwrap();
        let resolve_result = rt
            .block_on(
                registry
                    .get("workspace_resolve_node_id", 1)
                    .unwrap()
                    .invoke(run_context.api(), &resolve_runtime, &resolve_payload, None),
            )
            .unwrap();
        let resolved = resolve_result
            .emitted_artifacts
            .iter()
            .find(|artifact| artifact.artifact_type_id == "resolved_node_ref")
            .unwrap();

        let traversal_instance = BoundCapabilityInstance {
            capability_instance_id: "capinst_merkle_traversal".to_string(),
            capability_type_id: "merkle_traversal".to_string(),
            capability_version: 1,
            scope_ref: resolved
                .content
                .get("node_id")
                .and_then(serde_json::Value::as_str)
                .unwrap()
                .to_string(),
            scope_kind: "node".to_string(),
            binding_values: vec![BoundBindingValue {
                binding_id: "strategy".to_string(),
                value: json!("bottom_up"),
            }],
            input_wiring: Vec::new(),
        };
        let traversal_runtime = registry.runtime_init_for(&traversal_instance).unwrap();
        let traversal_payload = single_input_payload(
            &traversal_instance.capability_instance_id,
            "invk_merkle_traversal",
            "resolved_node_ref",
            "resolved_node_ref",
            resolved.content.clone(),
        );
        let traversal_result = rt
            .block_on(registry.get("merkle_traversal", 1).unwrap().invoke(
                run_context.api(),
                &traversal_runtime,
                &traversal_payload,
                None,
            ))
            .unwrap();
        let ordered = traversal_result
            .emitted_artifacts
            .iter()
            .find(|artifact| artifact.artifact_type_id == "ordered_merkle_node_batches")
            .unwrap();

        let batches = ordered
            .content
            .get("batches")
            .and_then(serde_json::Value::as_array)
            .unwrap();
        assert!(!batches.is_empty());
        assert!(batches[0].as_array().unwrap().len() >= 2);
    });
}

#[test]
fn context_provider_finalize_capabilities_materialize_frame() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(workspace_root.join("src")).unwrap();
        let file_path = workspace_root.join("src").join("lib.rs");
        fs::write(
            &file_path,
            "pub fn greet(name: &str) -> String { format!(\"hello {}\", name) }",
        )
        .unwrap();

        create_test_agent("docs-writer");

        let response_body = r##"{"id":"test","object":"chat.completion","created":0,"model":"test-model","choices":[{"index":0,"message":{"role":"assistant","content":"# Library\n\n## Purpose\n\nProvides greeting helpers."},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"##;
        let (endpoint, body_rx, handle) = spawn_capture_server(response_body);
        create_test_provider("test-provider", &endpoint);

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let node_id = meld::workspace::resolve_workspace_node_id(
            run_context.api(),
            &workspace_root,
            Some(Path::new("src/lib.rs")),
            None,
            false,
        )
        .unwrap();

        let mut catalog = CapabilityCatalog::new();
        let mut registry = CapabilityExecutorRegistry::new();
        register_phase_four_capabilities(&mut catalog, &mut registry);
        let rt = tokio::runtime::Runtime::new().unwrap();

        let prepare_instance = BoundCapabilityInstance {
            capability_instance_id: "capinst_prepare".to_string(),
            capability_type_id: "context_generate_prepare".to_string(),
            capability_version: 1,
            scope_ref: hex::encode(node_id),
            scope_kind: "node".to_string(),
            binding_values: vec![
                BoundBindingValue {
                    binding_id: "agent_id".to_string(),
                    value: json!("docs-writer"),
                },
                BoundBindingValue {
                    binding_id: "provider_binding".to_string(),
                    value: serde_json::to_value(
                        ProviderExecutionBinding::new(
                            "test-provider",
                            ProviderRuntimeOverrides::default(),
                        )
                        .unwrap(),
                    )
                    .unwrap(),
                },
                BoundBindingValue {
                    binding_id: "frame_type".to_string(),
                    value: json!("context-docs-writer"),
                },
                BoundBindingValue {
                    binding_id: "prompt_text".to_string(),
                    value: json!("Write a README in markdown only."),
                },
                BoundBindingValue {
                    binding_id: "turn_id".to_string(),
                    value: json!("style_refine"),
                },
                BoundBindingValue {
                    binding_id: "workflow_id".to_string(),
                    value: json!("docs_writer_thread_v1"),
                },
                BoundBindingValue {
                    binding_id: "output_type".to_string(),
                    value: json!("readme_final"),
                },
            ],
            input_wiring: Vec::new(),
        };
        let prepare_runtime = registry.runtime_init_for(&prepare_instance).unwrap();
        let prepare_payload = CapabilityInvocationPayload {
            invocation_id: "invk_prepare".to_string(),
            capability_instance_id: prepare_instance.capability_instance_id.clone(),
            supplied_inputs: vec![SuppliedInputValue {
                slot_id: "resolved_node_ref".to_string(),
                source: InputValueSource::ArtifactHandoff,
                value: SuppliedValueRef::Artifact(ArtifactValueRef {
                    artifact_id: "artifact_node".to_string(),
                    artifact_type_id: "resolved_node_ref".to_string(),
                    schema_version: 1,
                    content: json!({
                        "node_id": hex::encode(node_id),
                        "path": "src/lib.rs",
                    }),
                }),
            }],
            upstream_lineage: Some(UpstreamLineage {
                task_id: "task_docs_writer".to_string(),
                task_run_id: "taskrun_phase4".to_string(),
                capability_path: vec!["capinst_prepare".to_string()],
                batch_index: None,
                node_index: None,
                repair_scope: None,
            }),
            execution_context: CapabilityExecutionContext {
                attempt: 1,
                trace_id: Some("trace_phase4".to_string()),
                deadline_ms: None,
                cancellation_key: None,
                dispatch_priority: None,
            },
        };

        let prepare_result = rt
            .block_on(registry.get("context_generate_prepare", 1).unwrap().invoke(
                run_context.api(),
                &prepare_runtime,
                &prepare_payload,
                None,
            ))
            .unwrap();
        let provider_request = prepare_result
            .emitted_artifacts
            .iter()
            .find(|artifact| artifact.artifact_type_id == "provider_execute_request")
            .unwrap()
            .clone();
        let preparation_summary = prepare_result
            .emitted_artifacts
            .iter()
            .find(|artifact| artifact.artifact_type_id == "preparation_summary")
            .unwrap()
            .clone();

        let provider_instance = BoundCapabilityInstance {
            capability_instance_id: "capinst_provider".to_string(),
            capability_type_id: "provider_execute_chat".to_string(),
            capability_version: 1,
            scope_ref: hex::encode(node_id),
            scope_kind: "node".to_string(),
            binding_values: Vec::new(),
            input_wiring: Vec::new(),
        };
        let provider_runtime = registry.runtime_init_for(&provider_instance).unwrap();
        let provider_payload = single_input_payload(
            &provider_instance.capability_instance_id,
            "invk_provider",
            "provider_execute_request",
            "provider_execute_request",
            provider_request.content.clone(),
        );
        let provider_result = rt
            .block_on(registry.get("provider_execute_chat", 1).unwrap().invoke(
                run_context.api(),
                &provider_runtime,
                &provider_payload,
                None,
            ))
            .unwrap();
        let provider_output = provider_result
            .emitted_artifacts
            .iter()
            .find(|artifact| artifact.artifact_type_id == "provider_execute_result")
            .unwrap()
            .clone();

        let finalize_instance = BoundCapabilityInstance {
            capability_instance_id: "capinst_finalize".to_string(),
            capability_type_id: "context_generate_finalize".to_string(),
            capability_version: 1,
            scope_ref: hex::encode(node_id),
            scope_kind: "node".to_string(),
            binding_values: vec![
                BoundBindingValue {
                    binding_id: "persist_frame".to_string(),
                    value: json!(true),
                },
                BoundBindingValue {
                    binding_id: "output_type".to_string(),
                    value: json!("readme_final"),
                },
            ],
            input_wiring: Vec::new(),
        };
        let finalize_runtime = registry.runtime_init_for(&finalize_instance).unwrap();
        let finalize_payload = CapabilityInvocationPayload {
            invocation_id: "invk_finalize".to_string(),
            capability_instance_id: finalize_instance.capability_instance_id.clone(),
            supplied_inputs: vec![
                SuppliedInputValue {
                    slot_id: "provider_execute_result".to_string(),
                    source: InputValueSource::ArtifactHandoff,
                    value: SuppliedValueRef::Artifact(ArtifactValueRef {
                        artifact_id: provider_output.artifact_id.clone(),
                        artifact_type_id: provider_output.artifact_type_id.clone(),
                        schema_version: provider_output.schema_version,
                        content: provider_output.content.clone(),
                    }),
                },
                SuppliedInputValue {
                    slot_id: "preparation_summary".to_string(),
                    source: InputValueSource::ArtifactHandoff,
                    value: SuppliedValueRef::Artifact(ArtifactValueRef {
                        artifact_id: preparation_summary.artifact_id.clone(),
                        artifact_type_id: preparation_summary.artifact_type_id.clone(),
                        schema_version: preparation_summary.schema_version,
                        content: preparation_summary.content.clone(),
                    }),
                },
            ],
            upstream_lineage: Some(UpstreamLineage {
                task_id: "task_docs_writer".to_string(),
                task_run_id: "taskrun_phase4".to_string(),
                capability_path: vec!["capinst_finalize".to_string()],
                batch_index: None,
                node_index: None,
                repair_scope: None,
            }),
            execution_context: CapabilityExecutionContext {
                attempt: 1,
                trace_id: Some("trace_phase4".to_string()),
                deadline_ms: None,
                cancellation_key: None,
                dispatch_priority: None,
            },
        };
        let finalize_result = rt
            .block_on(
                registry
                    .get("context_generate_finalize", 1)
                    .unwrap()
                    .invoke(
                        run_context.api(),
                        &finalize_runtime,
                        &finalize_payload,
                        None,
                    ),
            )
            .unwrap();

        let request_body = body_rx.recv().unwrap();
        handle.join().unwrap();
        assert!(request_body.contains("Write a README in markdown only."));

        let generation_output = finalize_result
            .emitted_artifacts
            .iter()
            .find(|artifact| artifact.artifact_type_id == "readme_final")
            .unwrap();
        assert!(generation_output
            .content
            .get("content")
            .and_then(serde_json::Value::as_str)
            .unwrap()
            .contains("# Library"));

        let view = ContextView::builder()
            .max_frames(1)
            .recent()
            .by_type("context-docs-writer".to_string())
            .by_agent("docs-writer".to_string())
            .build();
        let context = run_context.api().get_node(node_id, view).unwrap();
        assert_eq!(context.frames.len(), 1);
        assert!(String::from_utf8_lossy(&context.frames[0].content).contains("# Library"));
    });
}

#[test]
fn context_prepare_dereferences_frame_ref_supporting_input() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(workspace_root.join("src").join("child")).unwrap();
        fs::write(
            workspace_root.join("src").join("lib.rs"),
            "pub fn greet() {}",
        )
        .unwrap();
        fs::write(
            workspace_root.join("src").join("child").join("lib.rs"),
            "pub fn child() {}",
        )
        .unwrap();

        create_test_agent("docs-writer");
        create_test_provider("test-provider", "http://127.0.0.1:9");

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let node_id = meld::workspace::resolve_workspace_node_id(
            run_context.api(),
            &workspace_root,
            Some(Path::new("src")),
            None,
            false,
        )
        .unwrap();
        let child_node_id = meld::workspace::resolve_workspace_node_id(
            run_context.api(),
            &workspace_root,
            Some(Path::new("src/child")),
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
        let child_frame_id = run_context
            .api()
            .put_frame(child_node_id, child_frame, "docs-writer".to_string())
            .unwrap();

        let mut catalog = CapabilityCatalog::new();
        let mut registry = CapabilityExecutorRegistry::new();
        register_phase_four_capabilities(&mut catalog, &mut registry);
        let rt = tokio::runtime::Runtime::new().unwrap();

        let prepare_instance = BoundCapabilityInstance {
            capability_instance_id: "capinst_prepare".to_string(),
            capability_type_id: "context_generate_prepare".to_string(),
            capability_version: 1,
            scope_ref: hex::encode(node_id),
            scope_kind: "node".to_string(),
            binding_values: vec![
                BoundBindingValue {
                    binding_id: "agent_id".to_string(),
                    value: json!("docs-writer"),
                },
                BoundBindingValue {
                    binding_id: "provider_binding".to_string(),
                    value: serde_json::to_value(
                        ProviderExecutionBinding::new(
                            "test-provider",
                            ProviderRuntimeOverrides::default(),
                        )
                        .unwrap(),
                    )
                    .unwrap(),
                },
                BoundBindingValue {
                    binding_id: "frame_type".to_string(),
                    value: json!("context-docs-writer"),
                },
                BoundBindingValue {
                    binding_id: "prompt_text".to_string(),
                    value: json!("Build evidence for README generation"),
                },
                BoundBindingValue {
                    binding_id: "turn_id".to_string(),
                    value: json!("evidence_gather"),
                },
                BoundBindingValue {
                    binding_id: "workflow_id".to_string(),
                    value: json!("docs_writer_thread_v1"),
                },
                BoundBindingValue {
                    binding_id: "output_type".to_string(),
                    value: json!("evidence_map"),
                },
            ],
            input_wiring: Vec::new(),
        };
        let prepare_runtime = registry.runtime_init_for(&prepare_instance).unwrap();
        let prepare_payload = CapabilityInvocationPayload {
            invocation_id: "invk_prepare_with_frame_ref".to_string(),
            capability_instance_id: prepare_instance.capability_instance_id.clone(),
            supplied_inputs: vec![
                SuppliedInputValue {
                    slot_id: "resolved_node_ref".to_string(),
                    source: InputValueSource::ArtifactHandoff,
                    value: SuppliedValueRef::Artifact(ArtifactValueRef {
                        artifact_id: "artifact_node".to_string(),
                        artifact_type_id: "resolved_node_ref".to_string(),
                        schema_version: 1,
                        content: json!({
                            "node_id": hex::encode(node_id),
                            "path": "src",
                        }),
                    }),
                },
                SuppliedInputValue {
                    slot_id: "upstream_artifact".to_string(),
                    source: InputValueSource::ArtifactHandoff,
                    value: SuppliedValueRef::Artifact(ArtifactValueRef {
                        artifact_id: "artifact_child_frame_ref".to_string(),
                        artifact_type_id: "frame_ref".to_string(),
                        schema_version: 1,
                        content: json!({
                            "frame_id": hex::encode(child_frame_id),
                            "node_id": hex::encode(child_node_id),
                            "frame_type": "context-docs-writer",
                        }),
                    }),
                },
            ],
            upstream_lineage: Some(UpstreamLineage {
                task_id: "task_docs_writer".to_string(),
                task_run_id: "taskrun_phase4".to_string(),
                capability_path: vec!["capinst_prepare".to_string()],
                batch_index: None,
                node_index: None,
                repair_scope: None,
            }),
            execution_context: CapabilityExecutionContext {
                attempt: 1,
                trace_id: Some("trace_phase4".to_string()),
                deadline_ms: None,
                cancellation_key: None,
                dispatch_priority: None,
            },
        };

        let prepare_result = rt
            .block_on(registry.get("context_generate_prepare", 1).unwrap().invoke(
                run_context.api(),
                &prepare_runtime,
                &prepare_payload,
                None,
            ))
            .unwrap();

        let provider_request = prepare_result
            .emitted_artifacts
            .iter()
            .find(|artifact| artifact.artifact_type_id == "provider_execute_request")
            .unwrap();
        let request_json = serde_json::to_string(&provider_request.content).unwrap();
        assert!(request_json.contains("# Existing Child README"));
    });
}
