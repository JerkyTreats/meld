use std::path::Path;
use std::sync::Arc;

use meld::agent::{AgentIdentity, AgentRegistry, AgentRole};
use meld::api::ContextApi;
use meld::cli::{Commands, RunContext};
use meld::concurrency::NodeLockManager;
use meld::context::events::{frame_added_envelope, head_ref, head_selected_envelope};
use meld::context::frame::{Basis, Frame, FrameStorage};
use meld::heads::HeadIndex;
use meld::prompt_context::PromptContextArtifactStorage;
use meld::store::{NodeRecord, NodeType, SledNodeRecordStore};
use meld::task::{build_execution_task_envelope, TaskEvent};
use meld::telemetry::events::ProgressEvent;
use meld::telemetry::{DomainObjectRef, ProgressRuntime};
use meld::types::{FrameID, NodeID};
use meld::workflow::events::{workflow_turn_completed_envelope, ExecutionWorkflowTurnEventData};
use meld::workspace::events::source_ref;
use meld::workspace::{read_workspace_scan_state, WorkspaceCommandService};
use meld::world_state::graph::compat::LegacyClaimAdapter;
use meld::world_state::graph::events::AnchorSelectedEventData;
use meld::world_state::graph::reducer::TraversalReducer;
use meld::world_state::{
    GraphRuntime, GraphWalkSpec, TraversalDirection, TraversalQuery, TraversalStore,
};

use crate::integration::with_xdg_env;

fn create_runtime_and_traversal() -> (Arc<ProgressRuntime>, TraversalStore, tempfile::TempDir) {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let spine_db = sled::open(temp_dir.path().join("spine")).unwrap();
    let traversal_db = sled::open(temp_dir.path().join("traversal")).unwrap();
    let progress = Arc::new(ProgressRuntime::new(spine_db).unwrap());
    let traversal = TraversalStore::new(traversal_db).unwrap();
    (progress, traversal, temp_dir)
}

fn create_context_api(
    workspace_root: &Path,
    progress: Arc<ProgressRuntime>,
    session_id: &str,
    temp_dir: &tempfile::TempDir,
) -> ContextApi {
    let db = progress.store().db().clone();
    let node_store = Arc::new(SledNodeRecordStore::from_db(db));
    let frame_storage = Arc::new(FrameStorage::new(temp_dir.path().join("frames")).unwrap());
    let prompt_context_storage =
        Arc::new(PromptContextArtifactStorage::new(temp_dir.path().join("artifacts")).unwrap());
    let head_index = Arc::new(parking_lot::RwLock::new(HeadIndex::new()));
    let agent_registry = Arc::new(parking_lot::RwLock::new(AgentRegistry::new()));
    let provider_registry = Arc::new(parking_lot::RwLock::new(
        meld::provider::ProviderRegistry::new(),
    ));
    let lock_manager = Arc::new(NodeLockManager::new());

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
    api.set_progress_context(progress, session_id.to_string());
    api
}

fn register_writer(api: &ContextApi, agent_id: &str) {
    let mut registry = api.agent_registry().write();
    registry.register(AgentIdentity::new(agent_id.to_string(), AgentRole::Writer));
}

fn frame_metadata(agent_id: &str) -> std::collections::HashMap<String, String> {
    let mut metadata = std::collections::HashMap::new();
    metadata.insert("agent_id".to_string(), agent_id.to_string());
    metadata.insert("provider".to_string(), "provider-a".to_string());
    metadata.insert("model".to_string(), "model-a".to_string());
    metadata.insert("provider_type".to_string(), "local".to_string());
    metadata.insert("prompt_digest".to_string(), "prompt-digest-a".to_string());
    metadata.insert("context_digest".to_string(), "context-digest-a".to_string());
    metadata.insert("prompt_link_id".to_string(), "prompt-link-a".to_string());
    metadata
}

fn put_test_node(api: &ContextApi, workspace_root: &Path, node_id: NodeID) {
    let path = workspace_root.join("doc.txt");
    std::fs::write(&path, "hello").unwrap();
    api.node_store()
        .put(&NodeRecord {
            node_id,
            path,
            node_type: NodeType::File {
                size: 5,
                content_hash: [1u8; 32],
            },
            children: vec![],
            parent: None,
            frame_set_root: None,
            metadata: Default::default(),
            tombstoned_at: None,
        })
        .unwrap();
}

fn append(
    runtime: &ProgressRuntime,
    envelope: meld::telemetry::events::ProgressEnvelope,
    seq: u64,
) {
    runtime
        .store()
        .append_event(&ProgressEvent::from_envelope(envelope, seq))
        .unwrap();
}

fn replay(runtime: &ProgressRuntime, traversal: &TraversalStore) -> TraversalReducer {
    TraversalReducer::replay_from_spine(runtime.store(), traversal, 0).unwrap()
}

fn node_ref(node_id: NodeID) -> DomainObjectRef {
    DomainObjectRef::new("workspace_fs", "node", hex::encode(node_id)).unwrap()
}

fn frame_ref(frame_id: FrameID) -> DomainObjectRef {
    DomainObjectRef::new("context", "frame", hex::encode(frame_id)).unwrap()
}

fn task_run_ref(task_run_id: &str) -> DomainObjectRef {
    DomainObjectRef::new("execution", "task_run", task_run_id).unwrap()
}

fn sample_turn_data(node_id: NodeID, frame_id: FrameID) -> ExecutionWorkflowTurnEventData {
    ExecutionWorkflowTurnEventData {
        workflow_id: "wf_a".to_string(),
        thread_id: "thread_a".to_string(),
        turn_id: "turn_a".to_string(),
        turn_seq: 1,
        node_id: hex::encode(node_id),
        path: "/tmp/doc.txt".to_string(),
        agent_id: "writer".to_string(),
        provider_name: "mock".to_string(),
        frame_type: "analysis".to_string(),
        attempt: 1,
        plan_id: Some("plan_a".to_string()),
        level_index: Some(0),
        final_frame_id: Some(hex::encode(frame_id)),
        error: None,
    }
}

fn build_task_artifact_event(
    task_run_id: &str,
    target_node_id: NodeID,
    artifact_id: &str,
    artifact_type_id: &str,
) -> meld::telemetry::events::ProgressEnvelope {
    let mut event = TaskEvent::new("task_artifact_emitted", "task_a", task_run_id);
    event.target_node_id = Some(hex::encode(target_node_id));
    event.artifact_id = Some(artifact_id.to_string());
    event.artifact_type_id = Some(artifact_type_id.to_string());
    build_execution_task_envelope("session_a", &event).unwrap()
}

fn load_cross_domain_graph(runtime: &ProgressRuntime, node_id: NodeID, frame_id: FrameID) {
    append(
        runtime,
        frame_added_envelope(
            "session_a",
            node_id,
            &Basis::Node(node_id),
            frame_id,
            "analysis",
            "writer",
        ),
        1,
    );
    append(
        runtime,
        head_selected_envelope("session_a", node_id, "analysis", frame_id, None),
        2,
    );
    append(
        runtime,
        workflow_turn_completed_envelope("session_a", sample_turn_data(node_id, frame_id)),
        3,
    );
    append(
        runtime,
        build_task_artifact_event("run_a", node_id, "artifact_a", "summary"),
        4,
    );
}

#[test]
fn facts_for_object_are_seq_ordered() {
    let (progress, traversal, _temp_dir) = create_runtime_and_traversal();
    let node_id = [7u8; 32];
    let frame_a = [8u8; 32];
    let frame_b = [9u8; 32];

    append(
        &progress,
        frame_added_envelope(
            "session_a",
            node_id,
            &Basis::Node(node_id),
            frame_a,
            "analysis",
            "writer",
        ),
        1,
    );
    append(
        &progress,
        head_selected_envelope("session_a", node_id, "analysis", frame_b, Some(frame_a)),
        2,
    );

    replay(&progress, &traversal);
    let facts = TraversalQuery::new(&traversal)
        .facts_for_object(&node_ref(node_id), 0)
        .unwrap();

    assert_eq!(
        facts.iter().map(|fact| fact.seq).collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[test]
fn current_anchor_lookup_is_index_backed() {
    let (progress, traversal, _temp_dir) = create_runtime_and_traversal();
    let node_id = [10u8; 32];
    let frame_id = [11u8; 32];

    append(
        &progress,
        head_selected_envelope("session_a", node_id, "analysis", frame_id, None),
        1,
    );

    replay(&progress, &traversal);
    let query = TraversalQuery::new(&traversal);
    let head_anchor_ref = head_ref(node_id, "analysis");
    let current = query.current_anchor(&head_anchor_ref).unwrap().unwrap();

    assert_eq!(current.anchor_ref, head_anchor_ref);
    assert_eq!(current.target, frame_ref(frame_id));
}

#[test]
fn neighbors_are_index_backed() {
    let (progress, traversal, _temp_dir) = create_runtime_and_traversal();
    let node_id = [12u8; 32];
    let frame_id = [13u8; 32];

    load_cross_domain_graph(&progress, node_id, frame_id);
    replay(&progress, &traversal);

    let neighbors = TraversalQuery::new(&traversal)
        .neighbors(&node_ref(node_id), TraversalDirection::Both, None, true)
        .unwrap();

    assert!(neighbors
        .iter()
        .any(|object| object.domain_id == "context" && object.object_kind == "head"));
    assert!(neighbors
        .iter()
        .any(|object| object.domain_id == "execution" && object.object_kind == "workflow_turn"));
    assert!(neighbors
        .iter()
        .any(|object| object.domain_id == "execution" && object.object_kind == "task_run"));
}

#[test]
fn walk_returns_bounded_subgraph() {
    let (progress, traversal, _temp_dir) = create_runtime_and_traversal();
    let node_id = [14u8; 32];
    let frame_id = [15u8; 32];

    load_cross_domain_graph(&progress, node_id, frame_id);
    replay(&progress, &traversal);
    let query = TraversalQuery::new(&traversal);

    let shallow = query
        .walk(
            &node_ref(node_id),
            &GraphWalkSpec {
                direction: TraversalDirection::Both,
                relation_types: None,
                max_depth: 1,
                current_only: true,
                include_facts: false,
            },
        )
        .unwrap();
    let deep = query
        .walk(
            &node_ref(node_id),
            &GraphWalkSpec {
                direction: TraversalDirection::Both,
                relation_types: None,
                max_depth: 3,
                current_only: true,
                include_facts: false,
            },
        )
        .unwrap();

    assert!(!shallow
        .visited_objects
        .iter()
        .any(|object| object.object_kind == "plan"));
    assert!(deep
        .visited_objects
        .iter()
        .any(|object| object.object_kind == "plan"));
}

#[test]
fn replay_rebuilds_same_context_heads() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let workspace_root = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).unwrap();

    let spine_db = sled::open(temp_dir.path().join("spine")).unwrap();
    let progress = Arc::new(ProgressRuntime::new(spine_db).unwrap());
    let session_id = progress
        .start_command_session("traversal.context".to_string())
        .unwrap();
    let api = create_context_api(
        &workspace_root,
        Arc::clone(&progress),
        &session_id,
        &temp_dir,
    );
    register_writer(&api, "writer");

    let node_id = [16u8; 32];
    put_test_node(&api, &workspace_root, node_id);

    let frame_a = Frame::new(
        Basis::Node(node_id),
        b"hello".to_vec(),
        "analysis".to_string(),
        "writer".to_string(),
        frame_metadata("writer"),
    )
    .unwrap();
    api.put_frame(node_id, frame_a, "writer".to_string())
        .unwrap();

    let frame_b = Frame::new(
        Basis::Node(node_id),
        b"world".to_vec(),
        "analysis".to_string(),
        "writer".to_string(),
        frame_metadata("writer"),
    )
    .unwrap();
    let latest_frame = api
        .put_frame(node_id, frame_b, "writer".to_string())
        .unwrap();

    let traversal_a =
        TraversalStore::new(sled::open(temp_dir.path().join("traversal_a")).unwrap()).unwrap();
    replay(&progress, &traversal_a);
    let current_a = TraversalQuery::new(&traversal_a)
        .current_frame_head(&node_ref(node_id), "analysis")
        .unwrap()
        .unwrap();

    let traversal_b =
        TraversalStore::new(sled::open(temp_dir.path().join("traversal_b")).unwrap()).unwrap();
    replay(&progress, &traversal_b);
    let current_b = TraversalQuery::new(&traversal_b)
        .current_frame_head(&node_ref(node_id), "analysis")
        .unwrap()
        .unwrap();

    assert_eq!(current_a.target.object_id, hex::encode(latest_frame));
    assert_eq!(current_b.target.object_id, current_a.target.object_id);
}

#[test]
fn current_frame_head_matches_legacy_head_index() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let workspace_root = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).unwrap();

    let spine_db = sled::open(temp_dir.path().join("spine")).unwrap();
    let progress = Arc::new(ProgressRuntime::new(spine_db).unwrap());
    let session_id = progress
        .start_command_session("traversal.context".to_string())
        .unwrap();
    let api = create_context_api(
        &workspace_root,
        Arc::clone(&progress),
        &session_id,
        &temp_dir,
    );
    register_writer(&api, "writer");

    let node_id = [17u8; 32];
    put_test_node(&api, &workspace_root, node_id);

    let frame = Frame::new(
        Basis::Node(node_id),
        b"hello".to_vec(),
        "analysis".to_string(),
        "writer".to_string(),
        frame_metadata("writer"),
    )
    .unwrap();
    let frame_id = api.put_frame(node_id, frame, "writer".to_string()).unwrap();

    let traversal =
        TraversalStore::new(sled::open(temp_dir.path().join("traversal")).unwrap()).unwrap();
    replay(&progress, &traversal);
    let current = TraversalQuery::new(&traversal)
        .current_frame_head(&node_ref(node_id), "analysis")
        .unwrap()
        .unwrap();

    assert_eq!(
        api.get_head(&node_id, "analysis").unwrap().unwrap(),
        frame_id
    );
    assert_eq!(current.target.object_id, hex::encode(frame_id));
}

#[test]
fn current_snapshot_matches_workspace_root_hash() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let workspace_root = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).unwrap();
    std::fs::write(workspace_root.join("doc.txt"), "hello").unwrap();

    let spine_db = sled::open(temp_dir.path().join("spine")).unwrap();
    let progress = Arc::new(ProgressRuntime::new(spine_db.clone()).unwrap());
    let session_id = progress
        .start_command_session("traversal.scan".to_string())
        .unwrap();
    let api = create_context_api(
        &workspace_root,
        Arc::clone(&progress),
        &session_id,
        &temp_dir,
    );

    WorkspaceCommandService::scan(
        &api,
        &workspace_root,
        true,
        Some(&progress),
        Some(&session_id),
    )
    .unwrap();

    let scan_state = read_workspace_scan_state(&api, &workspace_root).unwrap();
    let traversal =
        TraversalStore::new(sled::open(temp_dir.path().join("traversal")).unwrap()).unwrap();
    replay(&progress, &traversal);
    let query = TraversalQuery::new(&traversal);
    let source = source_ref(&workspace_root).unwrap();
    let current = query.current_snapshot_for_source(&source).unwrap().unwrap();

    assert_eq!(current.target.object_id, scan_state.current_root_hash);
}

#[test]
fn artifact_slot_selects_latest_artifact() {
    let (progress, traversal, _temp_dir) = create_runtime_and_traversal();
    let node_id = [18u8; 32];

    append(
        &progress,
        build_task_artifact_event("run_a", node_id, "artifact_a", "summary"),
        1,
    );
    append(
        &progress,
        build_task_artifact_event("run_a", node_id, "artifact_b", "summary"),
        2,
    );

    replay(&progress, &traversal);
    let query = TraversalQuery::new(&traversal);
    let current = query
        .current_artifact_for_task_run(&task_run_ref("run_a"), "summary")
        .unwrap()
        .unwrap();

    assert_eq!(current.target.object_id, "artifact_b");
    assert_eq!(
        query
            .anchor_history(
                &DomainObjectRef::new("execution", "artifact_slot", "run_a::summary").unwrap(),
            )
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn walk_from_workspace_node_to_frame_to_turn_to_plan() {
    let (progress, traversal, _temp_dir) = create_runtime_and_traversal();
    let node_id = [19u8; 32];
    let frame_id = [20u8; 32];

    load_cross_domain_graph(&progress, node_id, frame_id);
    replay(&progress, &traversal);

    let result = TraversalQuery::new(&traversal)
        .walk(
            &node_ref(node_id),
            &GraphWalkSpec {
                direction: TraversalDirection::Both,
                relation_types: None,
                max_depth: 3,
                current_only: true,
                include_facts: true,
            },
        )
        .unwrap();

    assert!(result
        .visited_objects
        .iter()
        .any(|object| object.domain_id == "context" && object.object_kind == "frame"));
    assert!(result
        .visited_objects
        .iter()
        .any(|object| object.domain_id == "execution" && object.object_kind == "workflow_turn"));
    assert!(result
        .visited_objects
        .iter()
        .any(|object| object.domain_id == "execution" && object.object_kind == "plan"));
    assert!(!result.visited_facts.is_empty());
}

#[test]
fn walk_from_task_run_to_node_and_artifact() {
    let (progress, traversal, _temp_dir) = create_runtime_and_traversal();
    let node_id = [21u8; 32];

    append(
        &progress,
        build_task_artifact_event("run_a", node_id, "artifact_a", "summary"),
        1,
    );
    replay(&progress, &traversal);

    let result = TraversalQuery::new(&traversal)
        .walk(
            &task_run_ref("run_a"),
            &GraphWalkSpec {
                direction: TraversalDirection::Both,
                relation_types: None,
                max_depth: 2,
                current_only: true,
                include_facts: false,
            },
        )
        .unwrap();

    assert!(result
        .visited_objects
        .iter()
        .any(|object| object.domain_id == "workspace_fs" && object.object_kind == "node"));
    assert!(result
        .visited_objects
        .iter()
        .any(|object| object.domain_id == "execution" && object.object_kind == "artifact"));
}

#[test]
fn legacy_claim_query_reads_through_traversal_adapter() {
    let (progress, traversal, _temp_dir) = create_runtime_and_traversal();
    let node_id = [22u8; 32];
    let frame_id = [23u8; 32];

    append(
        &progress,
        head_selected_envelope("session_a", node_id, "review", frame_id, None),
        1,
    );
    replay(&progress, &traversal);

    let subject = node_ref(node_id);
    let claims = LegacyClaimAdapter::new(&traversal)
        .current_claims_for_object(&subject)
        .unwrap();

    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].subject, subject);
    assert_eq!(claims[0].created_at_seq, 1);
}

#[test]
fn graph_runtime_repeated_catch_up_is_idempotent() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(temp_dir.path().join("spine")).unwrap();
    let progress = Arc::new(ProgressRuntime::new(db.clone()).unwrap());
    let runtime = GraphRuntime::new(db).unwrap();
    let node_id = [24u8; 32];
    let frame_id = [25u8; 32];

    append(
        &progress,
        head_selected_envelope("session_a", node_id, "analysis", frame_id, None),
        1,
    );

    assert_eq!(runtime.catch_up().unwrap(), 1);
    assert_eq!(runtime.catch_up().unwrap(), 0);

    let derived_events: Vec<_> = progress
        .store()
        .read_all_events_after(0)
        .unwrap()
        .into_iter()
        .filter(|event| event.event_type == "world_state.anchor_selected")
        .collect();
    assert_eq!(derived_events.len(), 1);

    let traversal = runtime.traversal_store();
    let query = TraversalQuery::new(traversal.as_ref());
    let current = query
        .current_frame_head(&node_ref(node_id), "analysis")
        .unwrap()
        .unwrap();

    assert_eq!(current.target.object_id, hex::encode(frame_id));
    assert_eq!(
        query
            .anchor_history(&head_ref(node_id, "analysis"))
            .unwrap()
            .len(),
        1
    );
    assert_eq!(traversal.last_reduced_seq().unwrap(), 2);
}

#[test]
fn graph_runtime_persists_anchor_selected_events_idempotently() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let db = sled::open(temp_dir.path().join("spine")).unwrap();
    let progress = Arc::new(ProgressRuntime::new(db.clone()).unwrap());
    let runtime = GraphRuntime::new(db).unwrap();
    let node_id = [26u8; 32];
    let frame_id = [27u8; 32];

    append(
        &progress,
        head_selected_envelope("session_a", node_id, "analysis", frame_id, None),
        1,
    );

    assert_eq!(runtime.catch_up().unwrap(), 1);
    assert_eq!(runtime.catch_up().unwrap(), 0);

    let derived: Vec<_> = progress
        .store()
        .read_all_events_after(0)
        .unwrap()
        .into_iter()
        .filter(|event| event.event_type == "world_state.anchor_selected")
        .collect();
    assert_eq!(derived.len(), 1);

    let data: AnchorSelectedEventData = serde_json::from_value(derived[0].data.clone()).unwrap();
    assert_eq!(derived[0].record_id.as_deref(), Some(data.fact_id.as_str()));
}

#[test]
fn derived_anchor_events_are_readable_from_spine_after_restart() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let db_path = temp_dir.path().join("spine");
    let db = sled::open(&db_path).unwrap();
    let progress = Arc::new(ProgressRuntime::new(db.clone()).unwrap());
    let runtime = GraphRuntime::new(db.clone()).unwrap();
    let node_id = [28u8; 32];
    let frame_id = [29u8; 32];

    append(
        &progress,
        head_selected_envelope("session_a", node_id, "analysis", frame_id, None),
        1,
    );

    assert_eq!(runtime.catch_up().unwrap(), 1);
    drop(runtime);
    drop(progress);
    drop(db);

    let reopened_db = sled::open(&db_path).unwrap();
    let reopened_progress = Arc::new(ProgressRuntime::new(reopened_db.clone()).unwrap());
    let reopened_runtime = GraphRuntime::new(reopened_db).unwrap();

    let derived: Vec<_> = reopened_progress
        .store()
        .read_all_events_after(0)
        .unwrap()
        .into_iter()
        .filter(|event| event.event_type == "world_state.anchor_selected")
        .collect();
    assert_eq!(derived.len(), 1);
    assert_eq!(reopened_runtime.catch_up().unwrap(), 0);
}

#[test]
fn run_context_scan_bootstraps_graph_runtime() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = temp_dir.path().join("workspace");
        std::fs::create_dir_all(&workspace_root).unwrap();
        std::fs::write(workspace_root.join("doc.txt"), "hello").unwrap();

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();

        let traversal =
            TraversalStore::new(run_context.progress_runtime().store().db().clone()).unwrap();
        let source = source_ref(&workspace_root).unwrap();
        let current = TraversalQuery::new(&traversal)
            .current_snapshot_for_source(&source)
            .unwrap()
            .unwrap();

        assert_eq!(current.subject, source);
        assert!(traversal.last_reduced_seq().unwrap() > 0);
    });
}
