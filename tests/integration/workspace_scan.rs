//! Workspace scan products through the canonical domain port.

use crate::integration::test_utils::open_authority_progress;
use meld::agent::AgentRegistry;
use meld::api::ContextApi;
use meld::concurrency::NodeLockManager;
use meld::heads::HeadIndex;
use meld::prompt_context::PromptContextArtifactStorage;
use meld::store::SledNodeRecordStore;
use meld::telemetry::ProgressRuntime;
use meld::workspace::scan::{
    execute_workspace_scan, WorkspaceScanOutcome, WorkspaceScanPolicy, WorkspaceScanRequest,
    WorkspaceScanStatus,
};
use meld_events::events::test_support::EventStore;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

fn create_test_api(
    workspace_root: &Path,
) -> (
    ContextApi,
    Arc<ProgressRuntime>,
    Arc<EventStore>,
    String,
    tempfile::TempDir,
) {
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
    let head_index = HeadIndex::new();
    let agent_registry = Arc::new(parking_lot::RwLock::new(AgentRegistry::new()));
    let provider_registry = Arc::new(parking_lot::RwLock::new(
        meld::provider::ProviderRegistry::new(),
    ));
    let lock_manager = Arc::new(NodeLockManager::new());
    let fixture = open_authority_progress(db);
    let event_store = Arc::clone(&fixture.store);
    let progress = Arc::new(fixture.progress);
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

    (api, progress, event_store, session_id, temp_dir)
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

fn scan(api: &ContextApi, root: &Path, session: &str, force: bool) -> WorkspaceScanOutcome {
    execute_workspace_scan(
        api,
        &WorkspaceScanRequest {
            workspace_root: root.into(),
            policy: WorkspaceScanPolicy { force },
            session_id: Some(session.into()),
            collect_observed: true,
        },
    )
    .unwrap()
}

#[test]
fn scan_reports_the_stored_tree_and_owner_publication_without_appending_it() {
    let workspace = fixture_workspace();
    let (api, progress, events, session, _store) = create_test_api(workspace.path());
    api.set_progress_context(progress, session.clone());
    let result = scan(&api, workspace.path(), &session, false);
    assert_eq!(result.summary.status, WorkspaceScanStatus::Scanned);
    assert_eq!(result.summary.node_count, FIXTURE_NODE_COUNT);
    assert!(result.summary.previous_root_node_id.is_none());
    assert_eq!(result.root_node_ref.node_id, result.summary.root_node_id);
    assert!(result.root_node_ref.parent_node_id.is_none());
    assert_eq!(result.snapshot_ref.domain_id, "workspace_fs");
    assert_eq!(result.snapshot_ref.object_kind, "snapshot");
    assert_eq!(result.snapshot_ref.object_id, result.summary.root_node_id);
    let observed: BTreeSet<_> = result
        .observed_nodes
        .iter()
        .map(|node| node.node_id.clone())
        .collect();
    let stored: BTreeSet<_> = api
        .node_store()
        .list_active()
        .unwrap()
        .iter()
        .map(|node| hex::encode(node.node_id))
        .collect();
    assert_eq!(observed, stored);
    assert_eq!(observed.len(), FIXTURE_NODE_COUNT);
    for path in ["pkg-a/README.md", "pkg-b/README.md"] {
        assert!(result
            .observed_nodes
            .iter()
            .any(|node| node.path.ends_with(path)));
    }
    let kinds: Vec<_> = result
        .publication_candidates
        .iter()
        .map(|event| event.event_type.as_str())
        .collect();
    for kind in [
        "workspace_fs.source_attached",
        "workspace_fs.snapshot_materialized",
        "workspace_fs.snapshot_selected",
        "workspace_fs.scan_completed",
        "world_state.owner_publication.v1",
    ] {
        assert!(kinds.contains(&kind));
    }
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == "workspace_fs.node_observed")
            .count(),
        FIXTURE_NODE_COUNT
    );
    let events = events.read_events_after(&session, 0).unwrap();
    assert!(events
        .iter()
        .all(|event| event.domain_id != "workspace_fs" && !event.event_type.starts_with("scan_")));
}

#[test]
fn unchanged_scan_retains_identity_and_returns_its_retryable_owner_publication() {
    let workspace = fixture_workspace();
    let (api, _, _, session, _store) = create_test_api(workspace.path());
    let first = scan(&api, workspace.path(), &session, false);
    let second = scan(&api, workspace.path(), &session, false);
    assert_eq!(first.summary.status, WorkspaceScanStatus::Scanned);
    assert_eq!(second.summary.status, WorkspaceScanStatus::UpToDate);
    assert_eq!(first.summary.root_node_id, second.summary.root_node_id);
    assert_eq!(second.observed_nodes.len(), FIXTURE_NODE_COUNT);
    assert_eq!(second.publication_candidates.len(), 1);
    assert_eq!(
        second.publication_candidates[0].event_type,
        "world_state.owner_publication.v1"
    );
    let forced = scan(&api, workspace.path(), &session, true);
    assert_eq!(forced.summary.status, WorkspaceScanStatus::Scanned);
    assert!(forced.summary.force);
    assert_eq!(forced.summary.root_node_id, first.summary.root_node_id);
}

#[test]
fn scan_publication_products_are_deterministic_for_the_same_tree() {
    let workspace = fixture_workspace();
    let observe = || {
        let (api, _, _, _, _store) = create_test_api(workspace.path());
        let result = scan(&api, workspace.path(), "scan-determinism", false);
        (
            result.summary,
            result.observed_nodes,
            result
                .publication_candidates
                .into_iter()
                .map(|event| (event.event_type, event.data))
                .collect::<Vec<_>>(),
        )
    };
    let first = observe();
    assert_eq!(first, observe());
    assert_eq!(first.2.len(), FIXTURE_NODE_COUNT + 5);
}

#[test]
fn workspace_scan_without_observed_collection_resolves_root_on_both_paths() {
    let workspace = fixture_workspace();
    let (api, _progress, _event_store, _session_id, _store_dir) = create_test_api(workspace.path());
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
