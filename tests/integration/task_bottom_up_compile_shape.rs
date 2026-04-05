use crate::integration::with_xdg_env;
use meld::agent::{AgentRole, AgentStorage, XdgAgentStorage};
use meld::capability::{CapabilityCatalog, CapabilityExecutionContext, CapabilityExecutorRegistry};
use meld::cli::{Commands, RunContext};
use meld::config::{xdg, AgentConfig, ProviderConfig, ProviderType};
use meld::provider::capability::ProviderExecuteChatCapability;
use meld::provider::ProviderExecutionBinding;
use meld::provider::ProviderRuntimeOverrides;
use meld::task::templates::prepare_registered_workflow_task_run;
use meld::task::{
    compile_task_expansion_request, parse_task_expansion_request_artifact, TaskExecutor,
    WorkflowPackageTriggerRequest,
};
use meld::workspace::capability::WorkspaceResolveNodeIdCapability;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::PathBuf;
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

fn create_test_provider(provider_name: &str) {
    let providers_dir = xdg::providers_dir().unwrap();
    fs::create_dir_all(&providers_dir).unwrap();
    let config_path = providers_dir.join(format!("{provider_name}.toml"));

    let provider_config = ProviderConfig {
        provider_name: Some(provider_name.to_string()),
        provider_type: ProviderType::LocalCustom,
        model: "test-model".to_string(),
        api_key: None,
        endpoint: Some("http://127.0.0.1:9".to_string()),
        default_options: meld::provider::CompletionOptions::default(),
    };

    fs::write(&config_path, toml::to_string(&provider_config).unwrap()).unwrap();
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

#[derive(Debug)]
struct BottomUpCompileCase {
    files: &'static [&'static str],
    target_path: &'static str,
    expected_active_paths: &'static [&'static str],
    expected_cross_node_edges: &'static [ExpectedEdge],
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ExpectedEdge {
    from_path: &'static str,
    from_turn: &'static str,
    from_stage: &'static str,
    to_path: &'static str,
    to_turn: &'static str,
    to_stage: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct NormalizedEdge {
    from_path: String,
    from_turn: String,
    from_stage: String,
    to_path: String,
    to_turn: String,
    to_stage: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NormalizedCompileShape {
    active_paths: BTreeSet<String>,
    cross_node_edges: BTreeSet<NormalizedEdge>,
}

fn compiled_shape_for_case(case: &BottomUpCompileCase) -> NormalizedCompileShape {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        meld::init::initialize_workflows(false).unwrap();

        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(&workspace_root).unwrap();
        for relative_path in case.files {
            let full_path = workspace_root.join(relative_path);
            fs::create_dir_all(full_path.parent().unwrap()).unwrap();
            fs::write(
                &full_path,
                format!("pub const MARKER: &str = \"{}\";", relative_path),
            )
            .unwrap();
        }

        create_test_agent("docs-writer", Some("docs_writer_thread_v1"));
        create_test_provider("test-provider");

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
                path: Some(PathBuf::from(case.target_path)),
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
            "repo_compile_shape",
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
            let delta = compile_task_expansion_request(
                run_context.api(),
                executor.compiled_task(),
                &request,
                &catalog,
            )
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

        let path_by_prefix = executor
            .artifact_repo()
            .record()
            .artifacts
            .iter()
            .filter_map(|artifact| {
                artifact
                    .producer
                    .output_slot_id
                    .as_deref()
                    .and_then(|slot_id| slot_id.strip_prefix("node_ref::"))
                    .map(|prefix| {
                        let absolute_path = artifact
                            .content
                            .get("path")
                            .and_then(|value| value.as_str())
                            .unwrap()
                            .to_string();
                        let path = PathBuf::from(&absolute_path)
                            .strip_prefix(&workspace_root)
                            .unwrap()
                            .to_string_lossy()
                            .to_string();
                        (prefix.to_string(), path)
                    })
            })
            .collect::<HashMap<_, _>>();

        let mut active_paths = BTreeSet::new();
        for instance in &executor.compiled_task().capability_instances {
            if let Some(node_ref) =
                parse_instance_id(&instance.capability_instance_id, &path_by_prefix)
            {
                active_paths.insert(node_ref.path);
            }
        }

        let cross_node_edges = executor
            .compiled_task()
            .dependency_edges
            .iter()
            .filter_map(|edge| {
                let from = parse_instance_id(&edge.from_capability_instance_id, &path_by_prefix)?;
                let to = parse_instance_id(&edge.to_capability_instance_id, &path_by_prefix)?;
                if from.path == to.path {
                    return None;
                }
                Some(NormalizedEdge {
                    from_path: from.path,
                    from_turn: from.turn_id,
                    from_stage: from.stage,
                    to_path: to.path,
                    to_turn: to.turn_id,
                    to_stage: to.stage,
                })
            })
            .collect::<BTreeSet<_>>();

        NormalizedCompileShape {
            active_paths,
            cross_node_edges,
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedInstanceRef {
    path: String,
    turn_id: String,
    stage: String,
}

fn parse_instance_id(
    capability_instance_id: &str,
    path_by_prefix: &HashMap<String, String>,
) -> Option<ParsedInstanceRef> {
    let parts = capability_instance_id.split("::").collect::<Vec<_>>();
    if parts.len() < 5 || parts[0] != "node" || parts[2] != "turn" {
        return None;
    }

    Some(ParsedInstanceRef {
        path: path_by_prefix.get(parts[1])?.clone(),
        turn_id: parts[3].to_string(),
        stage: parts[4].to_string(),
    })
}

fn assert_case(case: &BottomUpCompileCase) {
    let actual = compiled_shape_for_case(case);
    let expected_paths = case
        .expected_active_paths
        .iter()
        .map(|path| path.to_string())
        .collect::<BTreeSet<_>>();
    let expected_edges = case
        .expected_cross_node_edges
        .iter()
        .map(|edge| NormalizedEdge {
            from_path: edge.from_path.to_string(),
            from_turn: edge.from_turn.to_string(),
            from_stage: edge.from_stage.to_string(),
            to_path: edge.to_path.to_string(),
            to_turn: edge.to_turn.to_string(),
            to_stage: edge.to_stage.to_string(),
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(actual.active_paths, expected_paths);
    assert_eq!(actual.cross_node_edges, expected_edges);
}

#[test]
fn bottom_up_compile_shape_matches_nested_tree_case_data() {
    let case = BottomUpCompileCase {
        files: &["src/foo/bar/wow.rs", "src/foo/bar/baz.rs"],
        target_path: "src/foo",
        expected_active_paths: &[
            "src/foo",
            "src/foo/bar",
            "src/foo/bar/baz.rs",
            "src/foo/bar/wow.rs",
        ],
        expected_cross_node_edges: &[
            ExpectedEdge {
                from_path: "src/foo/bar/baz.rs",
                from_turn: "style_refine",
                from_stage: "finalize",
                to_path: "src/foo/bar",
                to_turn: "evidence_gather",
                to_stage: "prepare",
            },
            ExpectedEdge {
                from_path: "src/foo/bar/wow.rs",
                from_turn: "style_refine",
                from_stage: "finalize",
                to_path: "src/foo/bar",
                to_turn: "evidence_gather",
                to_stage: "prepare",
            },
            ExpectedEdge {
                from_path: "src/foo/bar",
                from_turn: "style_refine",
                from_stage: "finalize",
                to_path: "src/foo",
                to_turn: "evidence_gather",
                to_stage: "prepare",
            },
        ],
    };

    assert_case(&case);
}

#[test]
fn bottom_up_compile_shape_matches_two_branch_tree_case_data() {
    let case = BottomUpCompileCase {
        files: &["src/foo/bar/a.rs", "src/foo/baz/b.rs"],
        target_path: "src/foo",
        expected_active_paths: &[
            "src/foo",
            "src/foo/bar",
            "src/foo/bar/a.rs",
            "src/foo/baz",
            "src/foo/baz/b.rs",
        ],
        expected_cross_node_edges: &[
            ExpectedEdge {
                from_path: "src/foo/bar/a.rs",
                from_turn: "style_refine",
                from_stage: "finalize",
                to_path: "src/foo/bar",
                to_turn: "evidence_gather",
                to_stage: "prepare",
            },
            ExpectedEdge {
                from_path: "src/foo/baz/b.rs",
                from_turn: "style_refine",
                from_stage: "finalize",
                to_path: "src/foo/baz",
                to_turn: "evidence_gather",
                to_stage: "prepare",
            },
            ExpectedEdge {
                from_path: "src/foo/bar",
                from_turn: "style_refine",
                from_stage: "finalize",
                to_path: "src/foo",
                to_turn: "evidence_gather",
                to_stage: "prepare",
            },
            ExpectedEdge {
                from_path: "src/foo/baz",
                from_turn: "style_refine",
                from_stage: "finalize",
                to_path: "src/foo",
                to_turn: "evidence_gather",
                to_stage: "prepare",
            },
        ],
    };

    assert_case(&case);
}
