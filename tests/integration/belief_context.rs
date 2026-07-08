//! Contract tests for the flag-gated generation read path:
//! `belief_context_bundle` seeding and belief-endorsed frame selection.

use crate::integration::{
    create_test_agent, create_test_provider, register_docs_writer_capabilities,
    spawn_docs_writer_server, with_xdg_env,
};
use meld::agent::profile::prompt_contract::PromptContract;
use meld::capability::{CapabilityCatalog, CapabilityExecutorRegistry};
use meld::cli::{Commands, RunContext};
use meld::context::belief_context::{
    hydrate_belief_context_bundle, BeliefContextBundle, BELIEF_CONTEXT_FAMILY_ID,
};
use meld::context::frame::{Basis, Frame};
use meld::context::generation::contracts::{GenerationOrchestrationRequest, PromptAssemblyOutput};
use meld::context::generation::prompt_collection::{
    build_prompt_messages, build_prompt_messages_with_belief,
};
use meld::events::DomainObjectRef;
use meld::metadata::frame_write_contract::{
    build_generated_metadata, generated_metadata_input_from_payload,
};
use meld::prompt_context::{prepare_generated_lineage, PromptContextLineageInput};
use meld::provider::{ProviderExecutionBinding, ProviderRuntimeOverrides};
use meld::task::{
    execute_task_to_completion, prepare_registered_workflow_task_run, TaskExecutor,
    WorkflowPackageTriggerRequest,
};
use meld::types::NodeID;
use meld::world_state::belief::{
    BeliefKey, BeliefProvenanceSummary, BeliefStatus, BeliefStore, BeliefView, BranchScope,
    ContradictionState, FreshnessState, HydrationRefs, PlannerProjectionSummary, PosteriorSummary,
};
use meld::world_state::PerspectiveKey;
use meld_execution::workflow::profile::BELIEF_CONTEXT_ENV_VAR;
use meld_execution::BeliefStatusLabel;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const FRAME_TYPE: &str = "context-docs-writer";
const AGENT_ID: &str = "docs-writer";
const WORKFLOW_ID: &str = "docs_writer_thread_v1";

fn belief_key(subject_hex: &str) -> BeliefKey {
    BeliefKey {
        subject: DomainObjectRef::new("workspace_fs", "node", subject_hex).unwrap(),
        dimension_id: BELIEF_CONTEXT_FAMILY_ID.to_string(),
        predicate_id: "confidence".to_string(),
        perspective: PerspectiveKey::new("default", "default").unwrap(),
        branch_scope: BranchScope::main(),
        evidence_policy_id: "default_policy".to_string(),
    }
}

/// Authored belief facts for one seeded docs_freshness view.
struct SeededBelief {
    status: BeliefStatus,
    confidence: f64,
    stale: bool,
    contradicted: bool,
    contradicted_evidence_ids: Vec<String>,
    high_water_seq: u64,
}

/// Builds one docs_freshness belief view mirroring the world-model
/// projection shape.
fn belief_view(subject: NodeID, seed: SeededBelief) -> BeliefView {
    let SeededBelief {
        status,
        confidence,
        stale,
        contradicted,
        contradicted_evidence_ids,
        high_water_seq,
    } = seed;
    let subject_hex = hex::encode(subject);
    let key = belief_key(&subject_hex);
    BeliefView {
        view_id: format!("view-{}", &subject_hex[..8]),
        key,
        current_revision_id: Some(format!("rev-{}", &subject_hex[..8])),
        status,
        posterior: PosteriorSummary {
            probability: confidence,
            meaning: "stale_probability".to_string(),
        },
        planner_projection: PlannerProjectionSummary {
            confidence_field: "confidence".to_string(),
            confidence,
            threshold: 0.7,
        },
        uncertainty: 0.1,
        precision: 1.0,
        freshness: FreshnessState {
            stale,
            reasons: Vec::new(),
            high_water_seq,
        },
        contradiction: ContradictionState {
            contradicted,
            reasons: Vec::new(),
            supporting_evidence_ids: vec!["ev-support".to_string()],
            contradicted_evidence_ids,
        },
        observation: None,
        assessment_state: "complete".to_string(),
        advisory_posture: "ready".to_string(),
        provenance: BeliefProvenanceSummary::empty(),
        hydration: HydrationRefs {
            evidence_ids: vec!["ev-support".to_string()],
            source_fact_ids: vec!["fact-1".to_string()],
            graph_anchor_ids: Vec::new(),
            revision_id: Some(format!("rev-{}", &subject_hex[..8])),
        },
    }
}

/// Seeds docs_freshness belief views by opening the workspace belief store
/// directly, staying on the world-model side of the read-only belief
/// boundary: `ContextApi` exposes no belief write surface. Callers must not
/// hold an open `RunContext` on the same workspace while this runs, because
/// the sled store takes an exclusive lock; the handle drops before returning
/// so the next `RunContext` can open the store.
fn seed_belief_views(workspace_root: &Path, seeds: Vec<(NodeID, SeededBelief)>) {
    let store_path = meld::config::xdg::workspace_data_dir(workspace_root)
        .unwrap()
        .join("store");
    let db = sled::open(&store_path).unwrap();
    let store = BeliefStore::new(db).unwrap();
    for (subject, seed) in seeds {
        store.put_view(&belief_view(subject, seed)).unwrap();
    }
    store.flush().unwrap();
}

fn put_child_frame(api: &meld::compat::ContextApi, node_id: NodeID, content: &str) {
    let frame = Frame::new(
        Basis::Node(node_id),
        content.as_bytes().to_vec(),
        FRAME_TYPE.to_string(),
        AGENT_ID.to_string(),
        build_generated_metadata(&generated_metadata_input_from_payload(
            AGENT_ID,
            "test-provider",
            "test-model",
            "local_custom",
            "child prompt",
            "child context",
        )),
    )
    .unwrap();
    api.put_frame(node_id, frame, AGENT_ID.to_string()).unwrap();
}

fn generation_request(node_id: NodeID) -> GenerationOrchestrationRequest {
    GenerationOrchestrationRequest {
        request_id: 1,
        node_id,
        agent_id: AGENT_ID.to_string(),
        provider: ProviderExecutionBinding::new(
            "test-provider",
            ProviderRuntimeOverrides::default(),
        )
        .unwrap(),
        frame_type: FRAME_TYPE.to_string(),
        retry_count: 0,
        force: true,
    }
}

fn prompt_contract() -> PromptContract {
    PromptContract {
        system_prompt: "You are a careful docs writer.".to_string(),
        user_prompt_file: "Summarize file context".to_string(),
        user_prompt_directory: "Summarize directory context".to_string(),
    }
}

struct DocsWriterRunOutput {
    prompts: Vec<String>,
    lineage_summaries: Vec<serde_json::Value>,
    bundle_artifacts: Vec<Vec<u8>>,
    target_node_id: NodeID,
}

/// Runs the docs_writer task package to completion over `<workspace>/src`
/// with one belief seeded on the target subject, then extracts rendered
/// prompts and lineage summaries from the task artifact repo.
fn run_docs_writer_with_seeded_belief(temp_dir: &TempDir, repo_id: &str) -> DocsWriterRunOutput {
    meld::init::initialize_workflows(false).unwrap();

    let workspace_root = temp_dir.path().join("workspace");
    fs::create_dir_all(workspace_root.join("src")).unwrap();
    fs::write(
        workspace_root.join("src").join("lib.rs"),
        "pub fn greet(name: &str) -> String { format!(\"hello {}\", name) }",
    )
    .unwrap();

    create_test_agent(AGENT_ID, Some(WORKFLOW_ID));
    let (endpoint, server_handle) = spawn_docs_writer_server(4);
    create_test_provider("test-provider", &endpoint);

    // Scan first so the subject node exists, then release the runtime so
    // the belief store can be opened directly for seeding.
    let target_node_id = {
        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();
        meld::workspace::resolve_workspace_node_id(
            run_context.api(),
            &workspace_root,
            Some(PathBuf::from("src").as_path()),
            None,
            false,
        )
        .unwrap()
    };
    seed_belief_views(
        &workspace_root,
        vec![(
            target_node_id,
            SeededBelief {
                status: BeliefStatus::Settled,
                confidence: 0.93,
                stale: false,
                contradicted: false,
                contradicted_evidence_ids: vec!["ev-contra".to_string()],
                high_water_seq: 42,
            },
        )],
    );

    let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
    let registered_profile = run_context
        .workflow_registry()
        .read()
        .get("docs_writer_thread_v1")
        .unwrap()
        .clone();
    let mut catalog = CapabilityCatalog::new();
    let mut registry = CapabilityExecutorRegistry::new();
    register_docs_writer_capabilities(&mut catalog, &mut registry);

    let prepared = prepare_registered_workflow_task_run(
        run_context.api(),
        &workspace_root,
        &registered_profile,
        &WorkflowPackageTriggerRequest {
            package_id: "docs_writer".to_string(),
            workflow_id: "docs_writer_thread_v1".to_string(),
            node_id: None,
            path: Some(PathBuf::from("src")),
            agent_id: AGENT_ID.to_string(),
            provider: ProviderExecutionBinding::new(
                "test-provider",
                ProviderRuntimeOverrides::default(),
            )
            .unwrap(),
            frame_type: FRAME_TYPE.to_string(),
            force: true,
            session_id: None,
        },
        &catalog,
    )
    .unwrap();

    let mut executor = TaskExecutor::new(
        prepared.compiled_task.clone(),
        prepared.init_payload.clone(),
        repo_id,
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
    assert_eq!(server_handle.join().unwrap(), 4);

    let workspace_display = workspace_root.display().to_string();
    let mut prompts = Vec::new();
    let mut lineage_summaries = Vec::new();
    let mut bundle_digests = Vec::new();
    for artifact in &executor.artifact_repo().record().artifacts {
        match artifact.artifact_type_id.as_str() {
            "provider_execute_request" => {
                let content = artifact.content["messages"][1]["content"]
                    .as_str()
                    .expect("user message content")
                    .replace(&workspace_display, "<workspace>");
                prompts.push(content);
            }
            "prompt_context_lineage_summary" => {
                lineage_summaries.push(artifact.content.clone());
                if let Some(digest) = artifact.content.get("belief_bundle_digest") {
                    bundle_digests.push(digest.as_str().unwrap().to_string());
                }
            }
            _ => {}
        }
    }
    prompts.sort();

    let bundle_artifacts = bundle_digests
        .iter()
        .map(|digest| {
            run_context
                .api()
                .prompt_context_storage()
                .read_by_artifact_id_verified(digest)
                .expect("belief bundle artifact readable by digest")
        })
        .collect();

    DocsWriterRunOutput {
        prompts,
        lineage_summaries,
        bundle_artifacts,
        target_node_id,
    }
}

/// Runs `f` with the belief_context environment override set, restoring the
/// prior value afterwards. Restoration lives in a drop guard so a panic
/// inside `f` cannot leak the override into later tests sharing this
/// process. Callers already hold the XDG env mutex.
fn with_belief_context_env<R>(value: &str, f: impl FnOnce() -> R) -> R {
    struct RestoreBeliefContextEnv {
        previous: Option<String>,
    }
    impl Drop for RestoreBeliefContextEnv {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(previous) => std::env::set_var(BELIEF_CONTEXT_ENV_VAR, previous),
                None => std::env::remove_var(BELIEF_CONTEXT_ENV_VAR),
            }
        }
    }
    let _restore = RestoreBeliefContextEnv {
        previous: std::env::var(BELIEF_CONTEXT_ENV_VAR).ok(),
    };
    std::env::set_var(BELIEF_CONTEXT_ENV_VAR, value);
    f()
}

#[test]
fn belief_context_ab_contract_flag_off_unchanged_flag_on_adds_brief_and_lineage() {
    // Flag off: same fixture, same seeded belief; prompts and lineage must
    // show no belief conditioning.
    let off_dir = TempDir::new().unwrap();
    let off = with_xdg_env(&off_dir, || {
        run_docs_writer_with_seeded_belief(&off_dir, "repo_belief_ab_off")
    });

    // Flag on via environment override over the same authored profile.
    let on_dir = TempDir::new().unwrap();
    let on = with_xdg_env(&on_dir, || {
        with_belief_context_env("1", || {
            run_docs_writer_with_seeded_belief(&on_dir, "repo_belief_ab_on")
        })
    });

    assert_eq!(off.prompts.len(), 4);
    assert_eq!(on.prompts.len(), 4);

    // Flag off ignores the seeded belief entirely.
    for prompt in &off.prompts {
        assert!(!prompt.contains("Belief Context"));
        assert!(!prompt.contains("docs_freshness"));
    }
    for summary in &off.lineage_summaries {
        assert!(summary.get("belief_bundle_digest").is_none());
    }
    assert!(off.bundle_artifacts.is_empty());

    // Flag on renders the belief brief into every prepared prompt.
    for prompt in &on.prompts {
        assert!(prompt.contains("Belief Context (subject: src)"), "{prompt}");
        // The brief carries both the bundle high-water sequence and the
        // trigger assertion's own per-subject currency.
        assert!(prompt.contains("docs_freshness (high-water sequence 42)"));
        assert!(prompt.contains("status=Settled, confidence=0.93 (as of sequence 42)"));
        assert!(prompt.contains("authoritative over model recall"));
        assert!(prompt.contains("ev-contra (unresolved)"));
    }
    // Workspace-normalized prompts differ between the two variants.
    assert_ne!(off.prompts, on.prompts);

    // The on-variant lineage record links the bundle digest, and the digest
    // resolves to the canonical bundle for the target subject.
    assert_eq!(on.lineage_summaries.len(), 4);
    for summary in &on.lineage_summaries {
        assert!(summary.get("belief_bundle_digest").is_some());
    }
    assert_eq!(on.bundle_artifacts.len(), 4);
    for bytes in &on.bundle_artifacts {
        let bundle: BeliefContextBundle = serde_json::from_slice(bytes).unwrap();
        assert_eq!(bundle.subject_node_id, hex::encode(on.target_node_id));
        // The seeded subject path is workspace-relative, not absolute.
        assert_eq!(bundle.subject_path, "src");
        assert_eq!(bundle.family_id, BELIEF_CONTEXT_FAMILY_ID);
        assert_eq!(bundle.as_of_seq, 42);
        let assertion = bundle
            .assertion_for_subject(&bundle.subject_node_id)
            .expect("seeded belief hydrated");
        assert_eq!(assertion.confidence, 0.93);
        assert_eq!(assertion.status, BeliefStatusLabel::Settled);
        assert_eq!(assertion.contradicted_claims, vec!["ev-contra".to_string()]);
        // Only the target subject carries a belief in this fixture, so the
        // subtree assertion map holds exactly that subject.
        assert_eq!(bundle.subject_assertions.len(), 1);
        assert!(bundle
            .subject_assertions
            .contains_key(&bundle.subject_node_id));
    }
}

#[test]
fn belief_context_bundle_and_selection_deterministic_across_reopen() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        meld::init::initialize_workflows(false).unwrap();

        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(workspace_root.join("src")).unwrap();
        fs::write(workspace_root.join("src").join("a.rs"), "pub fn a() {}").unwrap();
        fs::write(workspace_root.join("src").join("b.rs"), "pub fn b() {}").unwrap();
        create_test_agent(AGENT_ID, Some(WORKFLOW_ID));

        let compute = |run_context: &RunContext| -> (String, String) {
            let api = run_context.api();
            let src_node = meld::workspace::resolve_workspace_node_id(
                api,
                &workspace_root,
                Some(PathBuf::from("src").as_path()),
                None,
                false,
            )
            .unwrap();
            let bundle = hydrate_belief_context_bundle(api, src_node, "src").unwrap();
            let node_record = api.node_store().get(&src_node).unwrap().unwrap();
            let output = build_prompt_messages_with_belief(
                api,
                &generation_request(src_node),
                &node_record,
                &prompt_contract(),
                Some(&bundle),
            )
            .unwrap();
            (bundle.canonical_json().unwrap(), output.context_payload)
        };

        // Scan and author frames, then release the runtime so beliefs can
        // seed through a directly opened store.
        let (src_node, child_a, child_b) = {
            let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
            run_context
                .execute(&Commands::Scan { force: true })
                .unwrap();
            let api = run_context.api();
            let src_node = meld::workspace::resolve_workspace_node_id(
                api,
                &workspace_root,
                Some(PathBuf::from("src").as_path()),
                None,
                false,
            )
            .unwrap();
            let child_a = meld::workspace::resolve_workspace_node_id(
                api,
                &workspace_root,
                Some(PathBuf::from("src/a.rs").as_path()),
                None,
                false,
            )
            .unwrap();
            let child_b = meld::workspace::resolve_workspace_node_id(
                api,
                &workspace_root,
                Some(PathBuf::from("src/b.rs").as_path()),
                None,
                false,
            )
            .unwrap();
            put_child_frame(api, child_a, "# Frame A");
            put_child_frame(api, child_b, "# Frame B");
            (src_node, child_a, child_b)
        };
        seed_belief_views(
            &workspace_root,
            vec![
                (
                    src_node,
                    SeededBelief {
                        status: BeliefStatus::Settled,
                        confidence: 0.9,
                        stale: false,
                        contradicted: false,
                        contradicted_evidence_ids: Vec::new(),
                        high_water_seq: 11,
                    },
                ),
                (
                    child_a,
                    SeededBelief {
                        status: BeliefStatus::Settled,
                        confidence: 0.88,
                        stale: false,
                        contradicted: false,
                        contradicted_evidence_ids: Vec::new(),
                        high_water_seq: 12,
                    },
                ),
                (
                    child_b,
                    SeededBelief {
                        status: BeliefStatus::Stale,
                        confidence: 0.4,
                        stale: true,
                        contradicted: false,
                        contradicted_evidence_ids: Vec::new(),
                        high_water_seq: 13,
                    },
                ),
            ],
        );

        // First open after seeding: hydrate and select twice.
        let (first_bundle, first_selection) = {
            let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
            let first = compute(&run_context);
            let second = compute(&run_context);
            // Repeated hydration and selection over the same open store are
            // byte-identical.
            assert_eq!(first, second);
            first
        };

        // Reopen the same workspace stores and recompute.
        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        let (reopened_bundle, reopened_selection) = compute(&run_context);

        assert_eq!(first_bundle, reopened_bundle);
        assert_eq!(first_selection, reopened_selection);
        // Sanity: the selection actually carries belief annotations, so the
        // determinism assertion covers the belief-governed path.
        assert!(reopened_selection.contains("Belief Context (subject: src)"));
        assert!(reopened_selection.contains("endorsed"));
        assert!(reopened_selection.contains("stale"));

        // Regression: the seeded bundle, not the live belief store, governs
        // prompt assembly. Mutating docs_freshness beliefs after hydration
        // must leave the rendered prompt and lineage digests unchanged.
        let bundle = hydrate_belief_context_bundle(run_context.api(), src_node, "src").unwrap();
        let node_record = run_context
            .api()
            .node_store()
            .get(&src_node)
            .unwrap()
            .unwrap();
        let assemble = |api: &meld::compat::ContextApi| {
            build_prompt_messages_with_belief(
                api,
                &generation_request(src_node),
                &node_record,
                &prompt_contract(),
                Some(&bundle),
            )
            .unwrap()
        };
        let prepare_lineage = |api: &meld::compat::ContextApi, output: &PromptAssemblyOutput| {
            prepare_generated_lineage(
                api.prompt_context_storage(),
                &PromptContextLineageInput {
                    system_prompt: output.system_prompt.clone(),
                    user_prompt_template: output.user_prompt_template.clone(),
                    rendered_prompt: output.rendered_prompt.clone(),
                    context_payload: output.context_payload.clone(),
                    belief_context_bundle: Some(bundle.canonical_json().unwrap()),
                },
            )
            .unwrap()
            .lineage
        };
        let before = assemble(run_context.api());
        let lineage_before = prepare_lineage(run_context.api(), &before);

        // Live mutations flip every seeded classification: contradict the
        // endorsed child, settle the stale child, and downgrade the subject.
        // The runtime is released around the direct-store writes.
        drop(run_context);
        seed_belief_views(
            &workspace_root,
            vec![
                (
                    child_a,
                    SeededBelief {
                        status: BeliefStatus::Settled,
                        confidence: 0.2,
                        stale: false,
                        contradicted: true,
                        contradicted_evidence_ids: vec!["ev-live-a".to_string()],
                        high_water_seq: 99,
                    },
                ),
                (
                    child_b,
                    SeededBelief {
                        status: BeliefStatus::Settled,
                        confidence: 0.95,
                        stale: false,
                        contradicted: false,
                        contradicted_evidence_ids: Vec::new(),
                        high_water_seq: 100,
                    },
                ),
                (
                    src_node,
                    SeededBelief {
                        status: BeliefStatus::NeedsAssessment,
                        confidence: 0.1,
                        stale: true,
                        contradicted: false,
                        contradicted_evidence_ids: Vec::new(),
                        high_water_seq: 101,
                    },
                ),
            ],
        );
        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();

        let after = assemble(run_context.api());
        assert_eq!(before.context_payload, after.context_payload);
        assert_eq!(before.rendered_prompt, after.rendered_prompt);
        assert_eq!(before.messages.len(), after.messages.len());
        for (before_message, after_message) in before.messages.iter().zip(&after.messages) {
            assert_eq!(before_message.content, after_message.content);
        }
        let lineage_after = prepare_lineage(run_context.api(), &after);
        assert_eq!(lineage_before.prompt_digest, lineage_after.prompt_digest);
        assert_eq!(lineage_before.context_digest, lineage_after.context_digest);
        assert_eq!(lineage_before.prompt_link_id, lineage_after.prompt_link_id);
        assert_eq!(
            lineage_before
                .belief_context_bundle
                .as_ref()
                .map(|artifact| artifact.digest.clone()),
            lineage_after
                .belief_context_bundle
                .as_ref()
                .map(|artifact| artifact.digest.clone())
        );
    });
}

#[test]
fn belief_endorsed_selection_excludes_contradicted_child_while_recency_unchanged() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        meld::init::initialize_workflows(false).unwrap();

        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(workspace_root.join("src")).unwrap();
        fs::write(workspace_root.join("src").join("a.rs"), "pub fn a() {}").unwrap();
        fs::write(workspace_root.join("src").join("b.rs"), "pub fn b() {}").unwrap();
        fs::write(workspace_root.join("src").join("c.rs"), "pub fn c() {}").unwrap();
        create_test_agent(AGENT_ID, Some(WORKFLOW_ID));

        // Scan, author frames, and capture the recency selection with no
        // belief seeded, then release the runtime so the belief store can be
        // opened directly for seeding.
        let request;
        let contract = prompt_contract();
        let (src_node, child_a, child_b, child_c, node_record, recency_before) = {
            let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
            run_context
                .execute(&Commands::Scan { force: true })
                .unwrap();
            let api = run_context.api();
            let src_node = meld::workspace::resolve_workspace_node_id(
                api,
                &workspace_root,
                Some(PathBuf::from("src").as_path()),
                None,
                false,
            )
            .unwrap();
            let child_a = meld::workspace::resolve_workspace_node_id(
                api,
                &workspace_root,
                Some(PathBuf::from("src/a.rs").as_path()),
                None,
                false,
            )
            .unwrap();
            let child_b = meld::workspace::resolve_workspace_node_id(
                api,
                &workspace_root,
                Some(PathBuf::from("src/b.rs").as_path()),
                None,
                false,
            )
            .unwrap();
            let child_c = meld::workspace::resolve_workspace_node_id(
                api,
                &workspace_root,
                Some(PathBuf::from("src/c.rs").as_path()),
                None,
                false,
            )
            .unwrap();
            put_child_frame(api, child_a, "# Frame A");
            put_child_frame(api, child_b, "# Frame B");
            put_child_frame(api, child_c, "# Frame C");

            let node_record = api.node_store().get(&src_node).unwrap().unwrap();
            request = generation_request(src_node);
            let recency_before =
                build_prompt_messages(api, &request, &node_record, &contract).unwrap();
            (
                src_node,
                child_a,
                child_b,
                child_c,
                node_record,
                recency_before,
            )
        };
        assert!(recency_before.context_payload.contains("# Frame A"));
        assert!(recency_before.context_payload.contains("# Frame B"));
        assert!(recency_before.context_payload.contains("# Frame C"));

        // Contradicted belief on child A, stale belief on child B; child C
        // stays uncovered so the recency-fallback rank sits between the
        // seeded classes.
        seed_belief_views(
            &workspace_root,
            vec![
                (
                    child_a,
                    SeededBelief {
                        status: BeliefStatus::Settled,
                        confidence: 0.3,
                        stale: false,
                        contradicted: true,
                        contradicted_evidence_ids: vec!["ev-contra-a".to_string()],
                        high_water_seq: 21,
                    },
                ),
                (
                    child_b,
                    SeededBelief {
                        status: BeliefStatus::Stale,
                        confidence: 0.5,
                        stale: true,
                        contradicted: false,
                        contradicted_evidence_ids: Vec::new(),
                        high_water_seq: 22,
                    },
                ),
            ],
        );

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        let api = run_context.api();

        // Recency selection ignores belief state entirely.
        let recency_after = build_prompt_messages(api, &request, &node_record, &contract).unwrap();
        assert_eq!(
            recency_before.context_payload,
            recency_after.context_payload
        );
        assert_eq!(
            recency_before.messages[1].content,
            recency_after.messages[1].content
        );

        // BeliefEndorsed selection excludes the contradicted child's content
        // and flags it as unresolved; the stale child stays, annotated and
        // ranked ahead of the flagged subject.
        let bundle = hydrate_belief_context_bundle(api, src_node, "src").unwrap();
        assert!(
            bundle
                .assertion_for_subject(&bundle.subject_node_id)
                .is_none(),
            "no belief seeded on src itself"
        );
        // Both covered children are hydrated into the subtree assertion map;
        // the uncovered child contributes no entry.
        assert_eq!(bundle.subject_assertions.len(), 2);
        assert!(bundle
            .subject_assertions
            .contains_key(&hex::encode(child_a)));
        assert!(bundle
            .subject_assertions
            .contains_key(&hex::encode(child_b)));
        assert!(!bundle
            .subject_assertions
            .contains_key(&hex::encode(child_c)));
        let endorsed = build_prompt_messages_with_belief(
            api,
            &request,
            &node_record,
            &contract,
            Some(&bundle),
        )
        .unwrap();

        assert!(!endorsed.context_payload.contains("# Frame A"));
        assert!(endorsed
            .context_payload
            .contains("docs_freshness contradicted (unresolved as of sequence 21)"));
        assert!(endorsed.context_payload.contains("# Frame B"));
        assert!(endorsed.context_payload.contains("docs_freshness stale"));
        assert!(endorsed.context_payload.contains("# Frame C"));
        // Section order pins the selection ranks: the uncovered child sorts
        // ahead of the stale one (the stale belief demotes its section below
        // an unseeded sibling despite later authored order), and the
        // contradicted flag line sorts last.
        let uncovered_index = endorsed.context_payload.find("# Frame C").unwrap();
        let stale_index = endorsed.context_payload.find("# Frame B").unwrap();
        let contradicted_index = endorsed
            .context_payload
            .find("contradicted (unresolved")
            .unwrap();
        assert!(uncovered_index < stale_index);
        assert!(stale_index < contradicted_index);
        // No subject belief means no belief brief in the prompt.
        assert!(!endorsed
            .context_payload
            .contains("Belief Context (subject:"));
        // Selection changed relative to recency.
        assert_ne!(recency_after.context_payload, endorsed.context_payload);
    });
}

#[test]
fn belief_empty_bundle_keeps_flag_on_prompt_byte_identical() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        meld::init::initialize_workflows(false).unwrap();

        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(workspace_root.join("src")).unwrap();
        fs::write(workspace_root.join("src").join("a.rs"), "pub fn a() {}").unwrap();
        create_test_agent(AGENT_ID, Some(WORKFLOW_ID));

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();
        let api = run_context.api();
        let src_node = meld::workspace::resolve_workspace_node_id(
            api,
            &workspace_root,
            Some(PathBuf::from("src").as_path()),
            None,
            false,
        )
        .unwrap();
        let child_a = meld::workspace::resolve_workspace_node_id(
            api,
            &workspace_root,
            Some(PathBuf::from("src/a.rs").as_path()),
            None,
            false,
        )
        .unwrap();
        put_child_frame(api, child_a, "# Frame A");
        let node_record = api.node_store().get(&src_node).unwrap().unwrap();
        let request = generation_request(src_node);
        let contract = prompt_contract();

        // No belief exists anywhere in the workspace: hydration yields an
        // explicitly empty bundle.
        let bundle = hydrate_belief_context_bundle(api, src_node, "src").unwrap();
        assert!(bundle.subject_assertions.is_empty());
        assert_eq!(bundle.as_of_seq, 0);
        assert_eq!(bundle.omitted_subject_count, 0);

        // The empty bundle enables belief-endorsed selection but must leave
        // the rendered prompt byte-identical to the recency path.
        let recency = build_prompt_messages(api, &request, &node_record, &contract).unwrap();
        let endorsed = build_prompt_messages_with_belief(
            api,
            &request,
            &node_record,
            &contract,
            Some(&bundle),
        )
        .unwrap();
        assert_eq!(recency.context_payload, endorsed.context_payload);
        assert_eq!(recency.rendered_prompt, endorsed.rendered_prompt);
        assert_eq!(recency.messages.len(), endorsed.messages.len());
        for (recency_message, endorsed_message) in recency.messages.iter().zip(&endorsed.messages) {
            assert_eq!(recency_message.content, endorsed_message.content);
        }
        assert!(!endorsed.context_payload.contains("Belief Context"));
    });
}
