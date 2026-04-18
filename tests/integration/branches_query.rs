use meld::branches::{BranchQueryRuntime, BranchQueryScope, BranchRuntime};
use meld::context::events::{frame_added_envelope, head_selected_envelope};
use meld::context::frame::Basis;
use meld::task::{build_execution_task_envelope, TaskEvent};
use meld::telemetry::events::{ProgressEnvelope, ProgressEvent};
use meld::telemetry::{DomainObjectRef, ProgressRuntime};
use meld::workflow::events::{workflow_turn_completed_envelope, ExecutionWorkflowTurnEventData};
use meld::world_state::{GraphRuntime, GraphWalkSpec, TraversalDirection, TraversalQuery};
use tempfile::TempDir;

use crate::integration::with_xdg_data_home;

fn append(runtime: &ProgressRuntime, envelope: ProgressEnvelope, seq: u64) {
    runtime
        .store()
        .append_event(&ProgressEvent::from_envelope(envelope, seq))
        .unwrap();
}

fn node_ref(node_id: [u8; 32]) -> DomainObjectRef {
    DomainObjectRef::new("workspace_fs", "node", hex::encode(node_id)).unwrap()
}

fn workflow_turn_data(
    node_id: [u8; 32],
    frame_id: [u8; 32],
    turn_id: &str,
) -> ExecutionWorkflowTurnEventData {
    ExecutionWorkflowTurnEventData {
        workflow_id: "wf_a".to_string(),
        thread_id: "thread_a".to_string(),
        turn_id: turn_id.to_string(),
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

fn task_artifact_event(
    task_run_id: &str,
    target_node_id: [u8; 32],
    artifact_id: &str,
) -> ProgressEnvelope {
    let mut event = TaskEvent::new("task_artifact_emitted", "task_a", task_run_id);
    event.target_node_id = Some(hex::encode(target_node_id));
    event.artifact_id = Some(artifact_id.to_string());
    event.artifact_type_id = Some("summary".to_string());
    build_execution_task_envelope("session_a", &event).unwrap()
}

fn load_branch_graph(
    store_path: &std::path::Path,
    session_id: &str,
    node_id: [u8; 32],
    frame_id: [u8; 32],
    turn_id: &str,
    task_run_id: &str,
    artifact_id: &str,
) {
    let db = sled::open(store_path).unwrap();
    let progress = ProgressRuntime::new(db.clone()).unwrap();
    append(
        &progress,
        frame_added_envelope(
            session_id,
            node_id,
            &Basis::Node(node_id),
            frame_id,
            "analysis",
            "writer",
        ),
        1,
    );
    append(
        &progress,
        head_selected_envelope(session_id, node_id, "analysis", frame_id, None),
        2,
    );
    append(
        &progress,
        workflow_turn_completed_envelope(
            session_id,
            workflow_turn_data(node_id, frame_id, turn_id),
        ),
        3,
    );
    append(
        &progress,
        task_artifact_event(task_run_id, node_id, artifact_id),
        4,
    );

    let graph_runtime = GraphRuntime::new(db).unwrap();
    assert_eq!(graph_runtime.catch_up().unwrap(), 4);
}

fn attach_branch(workspace: &TempDir) -> meld::branches::ResolvedBranch {
    let branch_runtime = BranchRuntime::new();
    let resolved = branch_runtime
        .resolve_active_branch(workspace.path())
        .unwrap();
    branch_runtime.attach_branch(workspace.path()).unwrap();
    resolved.resolved().clone()
}

#[test]
fn federated_neighbors_match_single_branch_traversal_query() {
    let test_dir = TempDir::new().unwrap();
    let workspace = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let branch = attach_branch(&workspace);
        let store_path = meld::branches::locator::branch_store_path(&branch.data_home_path);
        let node_id = [1u8; 32];
        let frame_id = [2u8; 32];
        load_branch_graph(
            &store_path,
            "session_single",
            node_id,
            frame_id,
            "turn_a",
            "run_a",
            "artifact_a",
        );

        let local_neighbors = {
            let local_db = sled::open(&store_path).unwrap();
            let local_traversal = meld::world_state::TraversalStore::new(local_db).unwrap();
            TraversalQuery::new(&local_traversal)
                .neighbors(&node_ref(node_id), TraversalDirection::Both, None, true)
                .unwrap()
        };

        let federated = BranchQueryRuntime::new()
            .neighbors(
                BranchQueryScope::BranchIds(vec![branch.branch_id.clone()]),
                None,
                &node_ref(node_id),
                TraversalDirection::Both,
                None,
                true,
            )
            .unwrap();

        let federated_neighbors: Vec<_> = federated
            .neighbors
            .iter()
            .map(|presence| presence.object.clone())
            .collect();
        assert_eq!(federated_neighbors, local_neighbors);
        assert_eq!(
            federated.metadata.readable_branch_ids,
            vec![branch.branch_id]
        );
    });
}

#[test]
fn federated_walk_matches_single_branch_traversal_query() {
    let test_dir = TempDir::new().unwrap();
    let workspace = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let branch = attach_branch(&workspace);
        let store_path = meld::branches::locator::branch_store_path(&branch.data_home_path);
        let node_id = [3u8; 32];
        let frame_id = [4u8; 32];
        load_branch_graph(
            &store_path,
            "session_walk",
            node_id,
            frame_id,
            "turn_walk",
            "run_walk",
            "artifact_walk",
        );

        let spec = GraphWalkSpec {
            direction: TraversalDirection::Both,
            relation_types: None,
            max_depth: 2,
            current_only: true,
            include_facts: true,
        };

        let local_walk = {
            let local_db = sled::open(&store_path).unwrap();
            let local_traversal = meld::world_state::TraversalStore::new(local_db).unwrap();
            TraversalQuery::new(&local_traversal)
                .walk(&node_ref(node_id), &spec)
                .unwrap()
        };

        let federated = BranchQueryRuntime::new()
            .walk(
                BranchQueryScope::BranchIds(vec![branch.branch_id.clone()]),
                None,
                &node_ref(node_id),
                &spec,
            )
            .unwrap();

        let federated_objects: Vec<_> = federated
            .walk
            .visited_objects
            .iter()
            .map(|presence| presence.object.clone())
            .collect();
        let federated_facts: Vec<_> = federated
            .walk
            .visited_facts
            .iter()
            .map(|fact| fact.fact.clone())
            .collect();
        let federated_relations: Vec<_> = federated
            .walk
            .traversed_relations
            .iter()
            .map(|relation| relation.relation.clone())
            .collect();

        assert_eq!(federated_objects, local_walk.visited_objects);
        assert_eq!(federated_facts, local_walk.visited_facts);
        assert_eq!(federated_relations, local_walk.traversed_relations);
        assert_eq!(
            federated.metadata.readable_branch_ids,
            vec![branch.branch_id]
        );
    });
}

#[test]
fn federated_neighbors_merge_many_branches_deterministically() {
    let test_dir = TempDir::new().unwrap();
    let workspace_a = TempDir::new().unwrap();
    let workspace_b = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let branch_a = attach_branch(&workspace_a);
        let branch_b = attach_branch(&workspace_b);
        let node_id = [5u8; 32];

        load_branch_graph(
            &meld::branches::locator::branch_store_path(&branch_a.data_home_path),
            "session_merge_a",
            node_id,
            [6u8; 32],
            "turn_a",
            "run_a",
            "artifact_a",
        );
        load_branch_graph(
            &meld::branches::locator::branch_store_path(&branch_b.data_home_path),
            "session_merge_b",
            node_id,
            [7u8; 32],
            "turn_b",
            "run_b",
            "artifact_b",
        );

        let federated = BranchQueryRuntime::new()
            .neighbors(
                BranchQueryScope::All,
                None,
                &node_ref(node_id),
                TraversalDirection::Both,
                None,
                true,
            )
            .unwrap();

        let keys: Vec<String> = federated
            .neighbors
            .iter()
            .map(|presence| format!("{}::{}", presence.branch_id, presence.object.index_key()))
            .collect();
        let mut sorted_keys = keys.clone();
        sorted_keys.sort();

        assert_eq!(keys, sorted_keys);
        assert!(keys
            .iter()
            .any(|key| key.ends_with("execution::task_run::run_a")));
        assert!(keys
            .iter()
            .any(|key| key.ends_with("execution::task_run::run_b")));
        let mut expected_branch_ids = vec![branch_a.branch_id, branch_b.branch_id];
        expected_branch_ids.sort();
        assert_eq!(federated.metadata.readable_branch_ids, expected_branch_ids);
    });
}

#[test]
fn federated_neighbors_return_branch_annotated_presence() {
    let test_dir = TempDir::new().unwrap();
    let workspace = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let branch = attach_branch(&workspace);
        let store_path = meld::branches::locator::branch_store_path(&branch.data_home_path);
        let node_id = [10u8; 32];
        let frame_id = [11u8; 32];
        load_branch_graph(
            &store_path,
            "session_presence",
            node_id,
            frame_id,
            "turn_presence",
            "run_presence",
            "artifact_presence",
        );

        let federated = BranchQueryRuntime::new()
            .neighbors(
                BranchQueryScope::BranchIds(vec![branch.branch_id.clone()]),
                None,
                &node_ref(node_id),
                TraversalDirection::Both,
                None,
                true,
            )
            .unwrap();

        assert!(federated.neighbors.iter().all(|presence| {
            presence.branch_id == branch.branch_id
                && presence.canonical_locator == branch.canonical_locator.to_string_lossy()
                && presence.current_in_branch
                && presence.last_seen_seq >= presence.first_seen_seq
        }));
    });
}

#[test]
fn federated_walk_preserves_branch_scoped_fact_identity() {
    let test_dir = TempDir::new().unwrap();
    let workspace_a = TempDir::new().unwrap();
    let workspace_b = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let branch_a = attach_branch(&workspace_a);
        let branch_b = attach_branch(&workspace_b);
        let node_id = [12u8; 32];
        let frame_id = [13u8; 32];

        load_branch_graph(
            &meld::branches::locator::branch_store_path(&branch_a.data_home_path),
            "session_identity_a",
            node_id,
            frame_id,
            "turn_same",
            "run_same",
            "artifact_same",
        );
        load_branch_graph(
            &meld::branches::locator::branch_store_path(&branch_b.data_home_path),
            "session_identity_b",
            node_id,
            frame_id,
            "turn_same",
            "run_same",
            "artifact_same",
        );

        let walk = BranchQueryRuntime::new()
            .walk(
                BranchQueryScope::All,
                None,
                &node_ref(node_id),
                &GraphWalkSpec {
                    direction: TraversalDirection::Both,
                    relation_types: None,
                    max_depth: 2,
                    current_only: true,
                    include_facts: true,
                },
            )
            .unwrap();

        let matching_local_ids: Vec<_> = walk
            .walk
            .visited_facts
            .iter()
            .filter(|fact| fact.fact.fact_id == "traversal::fact::2")
            .collect();
        assert_eq!(matching_local_ids.len(), 2);
        assert_ne!(
            matching_local_ids[0].federated_fact_id,
            matching_local_ids[1].federated_fact_id
        );
    });
}

#[test]
fn same_object_in_many_branches_keeps_distinct_presence_rows() {
    let test_dir = TempDir::new().unwrap();
    let workspace_a = TempDir::new().unwrap();
    let workspace_b = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let branch_a = attach_branch(&workspace_a);
        let branch_b = attach_branch(&workspace_b);
        let node_id = [14u8; 32];

        load_branch_graph(
            &meld::branches::locator::branch_store_path(&branch_a.data_home_path),
            "session_object_a",
            node_id,
            [15u8; 32],
            "turn_a",
            "run_a",
            "artifact_a",
        );
        load_branch_graph(
            &meld::branches::locator::branch_store_path(&branch_b.data_home_path),
            "session_object_b",
            node_id,
            [16u8; 32],
            "turn_b",
            "run_b",
            "artifact_b",
        );

        let walk = BranchQueryRuntime::new()
            .walk(
                BranchQueryScope::All,
                None,
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

        let node_rows: Vec<_> = walk
            .walk
            .visited_objects
            .iter()
            .filter(|presence| presence.object == node_ref(node_id))
            .collect();
        assert_eq!(node_rows.len(), 2);
        assert_ne!(node_rows[0].branch_id, node_rows[1].branch_id);
        assert!(node_rows.iter().all(|presence| presence.current_in_branch));
    });
}

#[test]
fn federated_graph_status_and_neighbors_isolate_unreadable_branches() {
    let test_dir = TempDir::new().unwrap();
    let healthy_workspace = TempDir::new().unwrap();
    let missing_workspace = TempDir::new().unwrap();

    with_xdg_data_home(&test_dir, || {
        let healthy_branch = attach_branch(&healthy_workspace);
        let missing_branch = attach_branch(&missing_workspace);
        let node_id = [8u8; 32];

        load_branch_graph(
            &meld::branches::locator::branch_store_path(&healthy_branch.data_home_path),
            "session_isolation",
            node_id,
            [9u8; 32],
            "turn_ok",
            "run_ok",
            "artifact_ok",
        );

        let status = BranchQueryRuntime::new()
            .graph_status(BranchQueryScope::All, None)
            .unwrap();
        assert!(status
            .branches
            .iter()
            .any(|branch| branch.branch_id == healthy_branch.branch_id
                && branch.read_status == "ready"));
        assert!(status
            .branches
            .iter()
            .any(|branch| branch.branch_id == missing_branch.branch_id
                && branch.read_status == "unreadable"));

        let neighbors = BranchQueryRuntime::new()
            .neighbors(
                BranchQueryScope::All,
                None,
                &node_ref(node_id),
                TraversalDirection::Both,
                None,
                true,
            )
            .unwrap();

        assert_eq!(
            neighbors.metadata.readable_branch_ids,
            vec![healthy_branch.branch_id]
        );
        assert_eq!(neighbors.metadata.skipped_branches.len(), 1);
        assert_eq!(
            neighbors.metadata.skipped_branches[0].branch_id,
            missing_branch.branch_id
        );
    });
}
