use std::sync::Arc;

use meld::agent::AgentRegistry;
use meld::api::ContextApi;
use meld::concurrency::NodeLockManager;
use meld::heads::HeadIndex;
use meld::prompt_context::PromptContextArtifactStorage;
use meld::store::SledNodeRecordStore;
use meld::telemetry::ProgressRuntime;
use meld::workspace::WorkspaceCommandService;

fn create_test_api(
    workspace_root: &std::path::Path,
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
        .start_command_session("workspace.traversal".to_string())
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

#[test]
fn brand_new_scan_emits_source_and_snapshot() {
    let workspace_root = tempfile::TempDir::new().unwrap();
    std::fs::write(workspace_root.path().join("a.txt"), "hello").unwrap();
    let (api, progress, session_id, _temp_dir) = create_test_api(workspace_root.path());

    WorkspaceCommandService::scan(
        &api,
        workspace_root.path(),
        false,
        Some(&progress),
        Some(&session_id),
    )
    .unwrap();

    let events = progress.store().read_events_after(&session_id, 0).unwrap();
    assert!(events
        .iter()
        .any(|event| event.event_type == "workspace_fs.source_attached"));
    assert!(events
        .iter()
        .any(|event| event.event_type == "workspace_fs.snapshot_materialized"));
    assert!(events
        .iter()
        .any(|event| event.event_type == "workspace_fs.snapshot_selected"));
}

#[test]
fn repeated_scan_reuses_source_identity() {
    let workspace_root = tempfile::TempDir::new().unwrap();
    std::fs::write(workspace_root.path().join("a.txt"), "hello").unwrap();
    let (api, progress, session_id, _temp_dir) = create_test_api(workspace_root.path());

    WorkspaceCommandService::scan(
        &api,
        workspace_root.path(),
        true,
        Some(&progress),
        Some(&session_id),
    )
    .unwrap();
    WorkspaceCommandService::scan(
        &api,
        workspace_root.path(),
        true,
        Some(&progress),
        Some(&session_id),
    )
    .unwrap();

    let events = progress.store().read_events_after(&session_id, 0).unwrap();
    let mut source_ids = std::collections::BTreeSet::new();
    for event in events
        .iter()
        .filter(|event| event.domain_id == "workspace_fs" && !event.objects.is_empty())
    {
        for object in &event.objects {
            if object.object_kind == "source" {
                source_ids.insert(object.object_id.clone());
            }
        }
    }
    assert_eq!(source_ids.len(), 1);
}

#[test]
fn snapshot_selected_changes_only_when_root_hash_changes() {
    let workspace_root = tempfile::TempDir::new().unwrap();
    let target = workspace_root.path().join("a.txt");
    std::fs::write(&target, "hello").unwrap();
    let (api, progress, session_id, _temp_dir) = create_test_api(workspace_root.path());

    WorkspaceCommandService::scan(
        &api,
        workspace_root.path(),
        true,
        Some(&progress),
        Some(&session_id),
    )
    .unwrap();
    let before = progress
        .store()
        .read_events_after(&session_id, 0)
        .unwrap()
        .into_iter()
        .filter(|event| event.event_type == "workspace_fs.snapshot_selected")
        .count();

    WorkspaceCommandService::scan(
        &api,
        workspace_root.path(),
        true,
        Some(&progress),
        Some(&session_id),
    )
    .unwrap();
    let same_root = progress
        .store()
        .read_events_after(&session_id, 0)
        .unwrap()
        .into_iter()
        .filter(|event| event.event_type == "workspace_fs.snapshot_selected")
        .count();
    assert_eq!(same_root, before);

    std::fs::write(&target, "changed").unwrap();
    WorkspaceCommandService::scan(
        &api,
        workspace_root.path(),
        true,
        Some(&progress),
        Some(&session_id),
    )
    .unwrap();
    let changed_root = progress
        .store()
        .read_events_after(&session_id, 0)
        .unwrap()
        .into_iter()
        .filter(|event| event.event_type == "workspace_fs.snapshot_selected")
        .count();
    assert!(changed_root > same_root);
}
