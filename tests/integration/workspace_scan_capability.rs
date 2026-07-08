//! Characterization tests for the `workspace_scan` capability.
//!
//! Pin current scan behavior as observed through the typed capability:
//! summary counts, root and observed node refs, incremental reruns, and the
//! rule that the capability reports publication candidates without appending
//! canonical events itself.

use meld::agent::AgentRegistry;
use meld::capability::{
    BoundBindingValue, BoundCapabilityInstance, CapabilityCatalog, CapabilityExecutionContext,
    CapabilityExecutorRegistry, CapabilityInvocationPayload, CapabilityInvocationResult,
    InputValueSource, SuppliedInputValue, SuppliedValueRef,
};
use meld::compat::ContextApi;
use meld::concurrency::NodeLockManager;
use meld::heads::HeadIndex;
use meld::prompt_context::PromptContextArtifactStorage;
use meld::store::SledNodeRecordStore;
use meld::telemetry::ProgressRuntime;
use meld::workflow::build_workflow_task_path_runtime;
use meld::workspace::capability::WorkspaceScanCapability;
use meld::workspace::scan::{
    execute_workspace_scan, WorkspaceScanPolicy, WorkspaceScanRequest, WorkspaceScanStatus,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

fn create_test_api(
    workspace_root: &Path,
) -> (ContextApi, Arc<ProgressRuntime>, String, tempfile::TempDir) {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let store_path = temp_dir.path().join("store");
    let frame_storage_path = temp_dir.path().join("frames");
    let artifact_storage_path = temp_dir.path().join("artifacts");
    std::fs::create_dir_all(&frame_storage_path).unwrap();
    std::fs::create_dir_all(&artifact_storage_path).unwrap();

    let db = sled::open(&store_path).unwrap();
    let node_store = Arc::new(SledNodeRecordStore::from_db(db.clone()));
    let frame_storage = Arc::new(meld::context::frame::open_storage(&frame_storage_path).unwrap());
    let prompt_context_storage =
        Arc::new(PromptContextArtifactStorage::new(&artifact_storage_path).unwrap());
    let head_index = Arc::new(parking_lot::RwLock::new(HeadIndex::new()));
    let agent_registry = Arc::new(parking_lot::RwLock::new(AgentRegistry::new()));
    let provider_registry = Arc::new(parking_lot::RwLock::new(
        meld::provider::ProviderRegistry::new(),
    ));
    let lock_manager = Arc::new(NodeLockManager::new());
    let progress = Arc::new(ProgressRuntime::new(db).unwrap());
    let session_id = progress
        .start_command_session("workspace.scan.capability".to_string())
        .unwrap();

    let api = ContextApi::with_workspace_root(
        node_store,
        frame_storage,
        head_index,
        prompt_context_storage,
        agent_registry,
        provider_registry,
        lock_manager,
        workspace_root.to_path_buf(),
    );

    (api, progress, session_id, temp_dir)
}

fn fixture_workspace() -> tempfile::TempDir {
    let workspace_root = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(workspace_root.path().join("pkg-a")).unwrap();
    std::fs::create_dir_all(workspace_root.path().join("pkg-b")).unwrap();
    std::fs::write(workspace_root.path().join("pkg-a").join("README.md"), "# a").unwrap();
    std::fs::write(workspace_root.path().join("pkg-b").join("README.md"), "# b").unwrap();
    workspace_root
}

const FIXTURE_NODE_COUNT: usize = 5;

fn scan_registry() -> (CapabilityCatalog, CapabilityExecutorRegistry) {
    let mut catalog = CapabilityCatalog::new();
    let mut registry = CapabilityExecutorRegistry::new();
    registry
        .register(&mut catalog, WorkspaceScanCapability)
        .unwrap();
    (catalog, registry)
}

fn invoke_scan(
    api: &ContextApi,
    registry: &CapabilityExecutorRegistry,
    workspace_root: &Path,
    invocation_id: &str,
    binding_values: Vec<BoundBindingValue>,
    supplied_inputs: Vec<SuppliedInputValue>,
) -> CapabilityInvocationResult {
    let instance = BoundCapabilityInstance {
        capability_instance_id: "capinst_workspace_scan".to_string(),
        capability_type_id: "workspace_scan".to_string(),
        capability_version: 1,
        scope_ref: workspace_root.display().to_string(),
        scope_kind: "workspace".to_string(),
        binding_values,
        input_wiring: Vec::new(),
    };
    let runtime_init = registry.runtime_init_for(&instance).unwrap();
    let payload = CapabilityInvocationPayload {
        invocation_id: invocation_id.to_string(),
        capability_instance_id: instance.capability_instance_id.clone(),
        supplied_inputs,
        upstream_lineage: None,
        execution_context: CapabilityExecutionContext {
            attempt: 1,
            ..CapabilityExecutionContext::default()
        },
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(registry.get("workspace_scan", 1).unwrap().invoke(
        api,
        &runtime_init,
        &payload,
        None,
    ))
    .unwrap()
}

fn artifact<'a>(
    result: &'a CapabilityInvocationResult,
    artifact_type_id: &str,
) -> &'a meld::task::ArtifactRecord {
    result
        .emitted_artifacts
        .iter()
        .find(|artifact| artifact.artifact_type_id == artifact_type_id)
        .unwrap_or_else(|| panic!("missing artifact '{artifact_type_id}'"))
}

fn session_binding(session_id: &str) -> BoundBindingValue {
    BoundBindingValue {
        binding_id: "session_id".to_string(),
        value: json!(session_id),
    }
}

#[test]
fn workspace_scan_contract_registers_in_task_path_runtime() {
    let runtime = build_workflow_task_path_runtime().unwrap();

    assert!(runtime.catalog.contains("workspace_scan", 1));
    assert!(runtime.registry.get("workspace_scan", 1).is_some());

    let contract = runtime.catalog.get("workspace_scan", 1).unwrap();
    assert_eq!(contract.owning_domain, "workspace");
    assert_eq!(contract.scope_contract.scope_kind, "workspace");
    assert_eq!(contract.scope_contract.scope_ref_kind, "workspace_root");

    let input_slots: Vec<&str> = contract
        .input_contract
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect();
    assert_eq!(input_slots, vec!["target_selector"]);
    assert!(!contract.input_contract[0].required);

    let output_slots: BTreeSet<&str> = contract
        .output_contract
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect();
    assert_eq!(
        output_slots,
        BTreeSet::from([
            "workspace_scan_summary",
            "workspace_snapshot_ref",
            "root_node_ref",
            "observed_node_refs",
            "publication_candidates",
        ])
    );

    let effect_targets: BTreeSet<&str> = contract
        .effect_contract
        .iter()
        .map(|effect| effect.target.as_str())
        .collect();
    assert_eq!(
        effect_targets,
        BTreeSet::from([
            "workspace_filesystem",
            "workspace_tree",
            "workspace_ignore_policy",
            "event_publication"
        ])
    );
}

#[test]
fn workspace_scan_capability_scans_fixture_tree() {
    let workspace = fixture_workspace();
    let (api, progress, session_id, _store_dir) = create_test_api(workspace.path());
    api.set_progress_context(progress.clone(), session_id.clone());
    let (_catalog, registry) = scan_registry();

    let result = invoke_scan(
        &api,
        &registry,
        workspace.path(),
        "invk_scan_initial",
        vec![session_binding(&session_id)],
        Vec::new(),
    );

    let summary = artifact(&result, "workspace_scan_summary");
    assert_eq!(
        summary.content.get("status").and_then(Value::as_str),
        Some("scanned")
    );
    assert_eq!(summary.content["node_count"], json!(FIXTURE_NODE_COUNT));
    assert_eq!(summary.content["previous_root_node_id"], Value::Null);
    let root_hex = summary.content["root_node_id"]
        .as_str()
        .unwrap()
        .to_string();

    let root_ref = artifact(&result, "resolved_node_ref");
    assert_eq!(root_ref.content["node_id"].as_str().unwrap(), root_hex);
    let root_bytes = hex::decode(&root_hex).unwrap();
    let mut root_node_id = [0u8; 32];
    root_node_id.copy_from_slice(&root_bytes);
    let root_record = api.node_store().get(&root_node_id).unwrap().unwrap();
    assert!(root_record.parent.is_none());

    let snapshot = artifact(&result, "workspace_snapshot_ref");
    assert_eq!(snapshot.content["domain_id"], json!("workspace_fs"));
    assert_eq!(snapshot.content["object_kind"], json!("snapshot"));
    assert_eq!(snapshot.content["object_id"].as_str().unwrap(), root_hex);

    let observed = artifact(&result, "workspace_observed_node_refs");
    let nodes = observed.content["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), FIXTURE_NODE_COUNT);
    let observed_ids: BTreeSet<String> = nodes
        .iter()
        .map(|node| node["node_id"].as_str().unwrap().to_string())
        .collect();
    let stored_ids: BTreeSet<String> = api
        .node_store()
        .list_active()
        .unwrap()
        .iter()
        .map(|record| hex::encode(record.node_id))
        .collect();
    assert_eq!(observed_ids, stored_ids);
    assert!(nodes
        .iter()
        .any(|node| node["path"].as_str().unwrap().ends_with("pkg-a/README.md")));
    assert!(nodes
        .iter()
        .any(|node| node["path"].as_str().unwrap().ends_with("pkg-b/README.md")));

    let candidates = artifact(&result, "workspace_event_candidates");
    let candidate_list = candidates.content["candidates"].as_array().unwrap();
    let candidate_types: Vec<&str> = candidate_list
        .iter()
        .map(|candidate| candidate["type"].as_str().unwrap())
        .collect();
    assert!(candidate_types.contains(&"workspace_fs.source_attached"));
    assert!(candidate_types.contains(&"workspace_fs.snapshot_materialized"));
    assert!(candidate_types.contains(&"workspace_fs.snapshot_selected"));
    assert!(candidate_types.contains(&"workspace_fs.scan_completed"));
    assert_eq!(
        candidate_types
            .iter()
            .filter(|event_type| **event_type == "workspace_fs.node_observed")
            .count(),
        FIXTURE_NODE_COUNT
    );
    // Candidates are time-free descriptors: append-time fields are stamped
    // by the publication runtime, keeping the artifact deterministic for a
    // fixed tree.
    for candidate in candidate_list {
        let fields = candidate.as_object().unwrap();
        for excluded in ["ts", "recorded_at", "session", "record_id", "occurred_at"] {
            assert!(
                !fields.contains_key(excluded),
                "candidate descriptor must not carry '{excluded}'"
            );
        }
        assert_eq!(candidate["domain_id"], json!("workspace_fs"));
        assert!(candidate["data"].is_object());
    }

    // Publication authority stays with the runtime: the capability reported
    // candidates above but appended nothing to the event store.
    let events = progress.store().read_events_after(&session_id, 0).unwrap();
    assert!(events.iter().all(|event| event.domain_id != "workspace_fs"));
    assert!(events
        .iter()
        .all(|event| !event.event_type.starts_with("scan_")));
}

#[test]
fn workspace_scan_capability_second_scan_is_incremental() {
    let workspace = fixture_workspace();
    let (api, _progress, session_id, _store_dir) = create_test_api(workspace.path());
    let (_catalog, registry) = scan_registry();

    let first = invoke_scan(
        &api,
        &registry,
        workspace.path(),
        "invk_scan_first",
        vec![session_binding(&session_id)],
        Vec::new(),
    );
    let first_summary = artifact(&first, "workspace_scan_summary").content.clone();

    let second = invoke_scan(
        &api,
        &registry,
        workspace.path(),
        "invk_scan_second",
        vec![session_binding(&session_id)],
        Vec::new(),
    );
    let second_summary = artifact(&second, "workspace_scan_summary").content.clone();

    assert_eq!(first_summary["status"], json!("scanned"));
    assert_eq!(second_summary["status"], json!("up_to_date"));
    assert_eq!(
        second_summary["root_node_id"],
        first_summary["root_node_id"]
    );
    assert_eq!(second_summary["node_count"], json!(FIXTURE_NODE_COUNT));

    // An incremental rerun writes nothing, so it has no publication candidates.
    assert!(second
        .emitted_artifacts
        .iter()
        .all(|artifact| artifact.artifact_type_id != "workspace_event_candidates"));
    let observed = artifact(&second, "workspace_observed_node_refs");
    assert_eq!(
        observed.content["nodes"].as_array().unwrap().len(),
        FIXTURE_NODE_COUNT
    );

    let forced = invoke_scan(
        &api,
        &registry,
        workspace.path(),
        "invk_scan_forced",
        vec![
            session_binding(&session_id),
            BoundBindingValue {
                binding_id: "scan_policy".to_string(),
                value: json!({ "force": true }),
            },
        ],
        Vec::new(),
    );
    let forced_summary = artifact(&forced, "workspace_scan_summary").content.clone();
    assert_eq!(forced_summary["status"], json!("scanned"));
    assert_eq!(forced_summary["force"], json!(true));
    assert_eq!(
        forced_summary["root_node_id"],
        first_summary["root_node_id"]
    );
}

#[test]
fn workspace_scan_capability_filters_observed_refs_by_target_selector() {
    let workspace = fixture_workspace();
    let (api, _progress, _session_id, _store_dir) = create_test_api(workspace.path());
    let (_catalog, registry) = scan_registry();

    let result = invoke_scan(
        &api,
        &registry,
        workspace.path(),
        "invk_scan_selector",
        Vec::new(),
        vec![SuppliedInputValue {
            slot_id: "target_selector".to_string(),
            source: InputValueSource::InitPayload,
            value: SuppliedValueRef::StructuredValue(json!({ "path": "pkg-a" })),
        }],
    );

    let summary = artifact(&result, "workspace_scan_summary");
    assert_eq!(summary.content["node_count"], json!(FIXTURE_NODE_COUNT));

    let observed = artifact(&result, "workspace_observed_node_refs");
    assert!(observed.content["target_path"]
        .as_str()
        .unwrap()
        .ends_with("pkg-a"));
    let nodes = observed.content["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 2);
    assert!(nodes
        .iter()
        .all(|node| node["path"].as_str().unwrap().contains("pkg-a")));

    // Root ref still points at the workspace root even under a selector.
    let root_ref = artifact(&result, "resolved_node_ref");
    assert_eq!(root_ref.content["node_id"], summary.content["root_node_id"]);

    // No session was bound, so scan reports no publication candidates.
    assert!(result
        .emitted_artifacts
        .iter()
        .all(|artifact| artifact.artifact_type_id != "workspace_event_candidates"));
}

#[test]
fn workspace_scan_publication_candidates_are_deterministic_for_a_fixed_tree() {
    let workspace = fixture_workspace();
    // Scans over the same tree from independent fresh stores must report
    // identical descriptors: node ids, path ordering, and candidate payloads
    // are pure functions of the tree and the bound session.
    let scan = || {
        let (api, _progress, _session_id, _store_dir) = create_test_api(workspace.path());
        let (_catalog, registry) = scan_registry();
        let result = invoke_scan(
            &api,
            &registry,
            workspace.path(),
            "invk_scan_determinism",
            vec![session_binding("session-scan-determinism")],
            Vec::new(),
        );
        (
            artifact(&result, "workspace_scan_summary").content.clone(),
            artifact(&result, "workspace_observed_node_refs")
                .content
                .clone(),
            artifact(&result, "workspace_event_candidates")
                .content
                .clone(),
        )
    };

    let first = scan();
    let second = scan();

    assert_eq!(first, second);
    // Guard against vacuous equality: the full first-scan candidate sequence
    // (attach, materialize, select, one node_observed per node, complete).
    assert_eq!(
        first.2["candidates"].as_array().unwrap().len(),
        FIXTURE_NODE_COUNT + 4
    );
}

#[test]
fn workspace_scan_without_observed_collection_resolves_root_on_both_paths() {
    let workspace = fixture_workspace();
    let (api, _progress, _session_id, _store_dir) = create_test_api(workspace.path());
    // The CLI scan path opts out of observed-ref collection.
    let request = WorkspaceScanRequest {
        workspace_root: workspace.path().to_path_buf(),
        policy: WorkspaceScanPolicy::default(),
        session_id: None,
        collect_observed: false,
    };

    let scanned = execute_workspace_scan(&api, &request).unwrap();
    assert_eq!(scanned.summary.status, WorkspaceScanStatus::Scanned);
    assert_eq!(scanned.summary.node_count, FIXTURE_NODE_COUNT);
    assert!(scanned.observed_nodes.is_empty());
    assert_eq!(scanned.root_node_ref.node_id, scanned.summary.root_node_id);
    assert!(scanned.root_node_ref.parent_node_id.is_none());
    assert!(scanned.publication_candidates.is_empty());

    // The up-to-date rerun builds only a root record internally; the root
    // ref must still resolve without observed-ref collection.
    let rerun = execute_workspace_scan(&api, &request).unwrap();
    assert_eq!(rerun.summary.status, WorkspaceScanStatus::UpToDate);
    assert_eq!(rerun.summary.node_count, FIXTURE_NODE_COUNT);
    assert!(rerun.observed_nodes.is_empty());
    assert_eq!(rerun.root_node_ref.node_id, scanned.summary.root_node_id);
}
