use std::path::PathBuf;
use std::sync::Arc;

use meld::agent::{AgentIdentity, AgentRegistry, AgentRole};
use meld::api::ContextApi;
use meld::concurrency::NodeLockManager;
use meld::context::frame::{Basis, Frame, FrameStorage};
use meld::heads::HeadIndex;
use meld::prompt_context::PromptContextArtifactStorage;
use meld::store::{NodeRecord, NodeType, SledNodeRecordStore};
use meld::telemetry::ProgressRuntime;
use meld::types::NodeID;

fn create_test_api() -> (ContextApi, Arc<ProgressRuntime>, String, tempfile::TempDir) {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let store_path = temp_dir.path().join("store");
    let frame_storage_path = temp_dir.path().join("frames");
    let artifact_storage_path = temp_dir.path().join("artifacts");
    std::fs::create_dir_all(&frame_storage_path).unwrap();
    std::fs::create_dir_all(&artifact_storage_path).unwrap();

    let db = sled::open(&store_path).unwrap();
    let node_store = Arc::new(SledNodeRecordStore::from_db(db.clone()));
    let frame_storage = Arc::new(FrameStorage::new(&frame_storage_path).unwrap());
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
        .start_command_session("context.traversal".to_string())
        .unwrap();

    let api = ContextApi::new(
        node_store,
        frame_storage,
        head_index,
        prompt_context_storage,
        agent_registry,
        provider_registry,
        lock_manager,
    );
    api.set_progress_context(Arc::clone(&progress), session_id.clone());

    (api, progress, session_id, temp_dir)
}

fn create_test_node_record(node_id: NodeID) -> NodeRecord {
    NodeRecord {
        node_id,
        path: PathBuf::from("/test/file.txt"),
        node_type: NodeType::File {
            size: 100,
            content_hash: [0u8; 32],
        },
        children: vec![],
        parent: None,
        frame_set_root: None,
        metadata: Default::default(),
        tombstoned_at: None,
    }
}

fn register_agent(api: &ContextApi, agent_id: &str) {
    let mut registry = api.agent_registry().write();
    registry.register(AgentIdentity::new(
        agent_id.to_string(),
        AgentRole::Writer,
    ));
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

#[test]
fn put_frame_emits_frame_and_head_events() {
    let (api, progress, session_id, _temp_dir) = create_test_api();
    let node_id: NodeID = [9u8; 32];
    api.node_store().put(&create_test_node_record(node_id)).unwrap();
    register_agent(&api, "writer-1");

    let frame = Frame::new(
        Basis::Node(node_id),
        b"hello".to_vec(),
        "analysis".to_string(),
        "writer-1".to_string(),
        frame_metadata("writer-1"),
    )
    .unwrap();
    let frame_id = api.put_frame(node_id, frame, "writer-1".to_string()).unwrap();

    let events = progress.store().read_events_after(&session_id, 0).unwrap();
    assert!(events.iter().any(|event| event.event_type == "context.frame_added"));
    let head_event = events
        .iter()
        .find(|event| event.event_type == "context.head_selected")
        .unwrap();
    assert!(head_event
        .objects
        .iter()
        .any(|object| object.object_kind == "head"));
    assert!(head_event
        .objects
        .iter()
        .any(|object| object.object_kind == "frame" && object.object_id == hex::encode(frame_id)));
}

#[test]
fn tombstone_head_emits_head_tombstoned() {
    let (api, progress, session_id, _temp_dir) = create_test_api();
    let node_id: NodeID = [10u8; 32];
    api.node_store().put(&create_test_node_record(node_id)).unwrap();
    register_agent(&api, "writer-1");

    let frame = Frame::new(
        Basis::Node(node_id),
        b"hello".to_vec(),
        "analysis".to_string(),
        "writer-1".to_string(),
        frame_metadata("writer-1"),
    )
    .unwrap();
    api.put_frame(node_id, frame, "writer-1".to_string()).unwrap();
    api.tombstone_head(node_id, "analysis").unwrap();

    let events = progress.store().read_events_after(&session_id, 0).unwrap();
    assert!(events
        .iter()
        .any(|event| event.event_type == "context.head_tombstoned"));
}
