//! Context currentness and Graph publication use one owner source across replay.
use crate::integration::test_utils::open_authority_progress;
use crate::integration::with_xdg_env;
use meld::agent::{AgentIdentity, AgentRegistry, AgentRole};
use meld::compat::ContextApi;
use meld::concurrency::NodeLockManager;
use meld::context::frame::{Basis, Frame, FrameStorage};
use meld::context::head::CurrentFrameHeadRead;
use meld::context::publication::{head_publication, publish_heads};
use meld::heads::HeadIndex;
use meld::prompt_context::PromptContextArtifactStorage;
use meld::store::{NodeRecord, NodeType, SledNodeRecordStore};
use meld::telemetry::ProgressRuntime;
use meld::types::NodeID;
use meld_events::events::test_support::{EventStore, EventStoreTestSupport as _};
use meld_world_model::graph::contracts::*;
use meld_world_model::TraversalQuery;
use std::path::Path;
use std::sync::Arc;

fn create_context_api(
    workspace_root: &Path,
    progress: Arc<ProgressRuntime>,
    event_store: &EventStore,
    session_id: &str,
    temp_dir: &tempfile::TempDir,
) -> ContextApi {
    let db = event_store.db().clone();
    let node_store = Arc::new(SledNodeRecordStore::from_db(db));
    let frame_storage = Arc::new(FrameStorage::new(temp_dir.path().join("frames")).unwrap());
    let prompt_context_storage =
        Arc::new(PromptContextArtifactStorage::new(temp_dir.path().join("artifacts")).unwrap());
    let head_index = HeadIndex::new();
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

fn selected_frames(
    graph: &meld_world_model::graph::runtime::GraphRuntime,
    operation: &OwnerPublicationOperation,
) -> Vec<String> {
    graph.catch_up().unwrap();
    let store = graph.traversal_store();
    let query = TraversalQuery::new(&store);
    let cut = query
        .cut(&TraversalCutRequest {
            owners: vec![TraversalOwnerRequirement {
                owner_id: "context".into(),
                scope: operation.batch.scope.clone(),
                required: true,
                event_source: None,
            }],
            scope: operation.batch.scope.clone(),
            event_position: graph.durable_event_cursor().unwrap(),
            currentness: OwnerCurrentnessPolicy::LatestComplete,
        })
        .unwrap();
    assert_eq!(cut.receipts[0].revision_id, operation.batch.revision_id);
    let roots = operation
        .batch
        .objects
        .iter()
        .filter(|object| object.object_ref.object_kind == "head")
        .map(|object| object.object_ref.clone())
        .collect();
    query
        .traverse(
            &cut,
            &BoundedTraversalRequest {
                roots,
                direction: TraversalDirection::Outgoing,
                relation_types: Some(vec!["selected".into()]),
                bounds: TraversalBounds {
                    max_depth: 1,
                    max_objects: 32,
                    max_occurrences: 32,
                    max_paths: 32,
                },
            },
        )
        .unwrap()
        .objects
        .into_iter()
        .filter(|object| {
            object.object_ref.object_kind == "frame"
                && object.state == OwnerPublicationState::Observed
        })
        .map(|object| object.object_ref.object_id)
        .collect()
}

#[test]
fn context_source_controls_selection_withdrawal_restore_and_replay() {
    let temp = tempfile::tempdir().unwrap();
    with_xdg_env(&temp, || {
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let fixture = open_authority_progress(sled::open(temp.path().join("events")).unwrap());
        let graph = fixture.graph_runtime(sled::open(temp.path().join("graph")).unwrap());
        let progress = Arc::new(fixture.progress.clone());
        let api = create_context_api(
            &workspace,
            progress.clone(),
            &fixture.store,
            "context-test",
            &temp,
        );
        api.bind_event_append(fixture.append_capability()).unwrap();
        // Telemetry session lifetime must not gate canonical owner publication.
        api.clear_progress_context();
        register_writer(&api, "writer");
        let node = [16; 32];
        put_test_node(&api, &workspace, node);
        let frame = Frame::new(
            Basis::Node(node),
            b"hello".to_vec(),
            "analysis".into(),
            "writer".into(),
            frame_metadata("writer"),
        )
        .unwrap();
        let frame_id = api.put_frame(node, frame, "writer".into()).unwrap();
        let first = head_publication(&api.heads(), api.frame_storage()).unwrap();
        assert_eq!(selected_frames(&graph, &first), vec![hex::encode(frame_id)]);
        api.tombstone_head(node, "analysis").unwrap();
        let withdrawn = head_publication(&api.heads(), api.frame_storage()).unwrap();
        assert!(selected_frames(&graph, &withdrawn).is_empty());
        assert!(api.current_frame_heads_for_node(&node).unwrap().is_empty());
        // Re-selecting identical content is a new source revision, not a replay
        // of the initial operation that would leave Graph on the withdrawal.
        let frame = api.frame_storage().get(&frame_id).unwrap().unwrap();
        api.put_frame(node, frame, "writer".into()).unwrap();
        let restored = head_publication(&api.heads(), api.frame_storage()).unwrap();
        assert_ne!(first.operation_id, restored.operation_id);
        assert_eq!(
            selected_frames(&graph, &restored),
            vec![hex::encode(frame_id)]
        );
        let reopened = HeadIndex::load_from_disk(HeadIndex::persistence_path(&workspace)).unwrap();
        assert_eq!(
            head_publication(&reopened, api.frame_storage()).unwrap(),
            restored
        );
        let before = fixture.replay_all().len();
        publish_heads(
            &fixture.append_capability(),
            &reopened,
            api.frame_storage(),
            "context-test",
        )
        .unwrap();
        assert_eq!(fixture.replay_all().len(), before);
        let replayed =
            fixture.graph_runtime(sled::open(temp.path().join("replayed-graph")).unwrap());
        assert_eq!(
            selected_frames(&replayed, &restored),
            vec![hex::encode(frame_id)]
        );
    });
}

#[test]
fn independent_context_sources_do_not_replace_each_other_in_a_shared_cut() {
    let temp = tempfile::tempdir().unwrap();
    let frames = FrameStorage::new(temp.path().join("frames")).unwrap();
    let frame = Frame::new(
        Basis::Node([1; 32]),
        b"hello".to_vec(),
        "analysis".into(),
        "writer".into(),
        frame_metadata("writer"),
    )
    .unwrap();
    frames.store(&frame).unwrap();
    let mut first = HeadIndex::new();
    let mut second = HeadIndex::new();
    first
        .update_head(&[1; 32], "analysis", &frame.frame_id)
        .unwrap();
    second
        .update_head(&[1; 32], "analysis", &frame.frame_id)
        .unwrap();
    let first = head_publication(&first, &frames).unwrap();
    let second = head_publication(&second, &frames).unwrap();
    let fixture = open_authority_progress(sled::open(temp.path().join("events")).unwrap());
    let graph = fixture.graph_runtime(sled::open(temp.path().join("graph")).unwrap());
    for operation in [&first, &second] {
        fixture
            .progress
            .emit_envelope_idempotent(
                meld_world_model::graph::events::owner_publication_envelope("test", operation)
                    .unwrap(),
            )
            .unwrap();
    }
    graph.catch_up().unwrap();
    let store = graph.traversal_store();
    let query = TraversalQuery::new(&store);
    let cut = query
        .cut(&TraversalCutRequest {
            owners: [&first, &second]
                .into_iter()
                .map(|operation| TraversalOwnerRequirement {
                    owner_id: "context".into(),
                    scope: operation.batch.scope.clone(),
                    required: true,
                    event_source: None,
                })
                .collect(),
            scope: OwnerPublicationScope {
                scope_id: "shared-inspection".into(),
                branch_id: None,
                perspective_id: None,
                valid_at: None,
            },
            event_position: graph.durable_event_cursor().unwrap(),
            currentness: OwnerCurrentnessPolicy::LatestComplete,
        })
        .unwrap();
    let result = query
        .traverse(
            &cut,
            &BoundedTraversalRequest {
                roots: [&first, &second]
                    .into_iter()
                    .flat_map(|operation| operation.batch.objects.iter())
                    .filter(|object| object.object_ref.object_kind == "head")
                    .map(|object| object.object_ref.clone())
                    .collect(),
                direction: TraversalDirection::Outgoing,
                relation_types: Some(vec!["selected".into()]),
                bounds: TraversalBounds {
                    max_depth: 1,
                    max_objects: 16,
                    max_occurrences: 16,
                    max_paths: 16,
                },
            },
        )
        .unwrap();
    assert_eq!(result.receipts.len(), 2);
    assert_eq!(result.occurrences.len(), 2);
    assert_eq!(result.objects.len(), 4);
}
