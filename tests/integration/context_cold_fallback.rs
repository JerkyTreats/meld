//! Cold-run context parity for the capability prompt path.
//!
//! Sibling contract: the workflow resolver falls back to reading file source
//! from disk when a child carries no frames, and renders the literal
//! insufficient-context marker when a directory yields nothing. The capability
//! path must agree, or cold runs on deep directories send prompts with no
//! context and the writer truthfully produces empty evidence.

use meld::agent::profile::prompt_contract::PromptContract;
use meld::agent::{AgentIdentity, AgentRole};
use meld::api::ContextApi;
use meld::context::frame::storage::FrameStorage;
use meld::context::generation::contracts::GenerationOrchestrationRequest;
use meld::context::generation::prompt_collection::{
    build_prompt_messages, INSUFFICIENT_CONTEXT_MARKER,
};
use meld::heads::HeadIndex;
use meld::prompt_context::PromptContextArtifactStorage;
use meld::store::persistence::SledNodeRecordStore;
use meld::store::{NodeRecord, NodeType};
use meld::types::{Hash, NodeID};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

fn create_test_api() -> (ContextApi, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let store_path = temp_dir.path().join("store");
    let node_store = Arc::new(SledNodeRecordStore::new(&store_path).unwrap());
    let frame_storage_path = temp_dir.path().join("frames");
    let artifact_storage_path = temp_dir.path().join("artifacts");
    fs::create_dir_all(&frame_storage_path).unwrap();
    fs::create_dir_all(&artifact_storage_path).unwrap();
    let frame_storage = Arc::new(FrameStorage::new(&frame_storage_path).unwrap());
    let prompt_context_storage =
        Arc::new(PromptContextArtifactStorage::new(&artifact_storage_path).unwrap());
    let head_index = HeadIndex::new();
    let agent_registry = Arc::new(parking_lot::RwLock::new(meld::agent::AgentRegistry::new()));
    let provider_registry = Arc::new(parking_lot::RwLock::new(
        meld::provider::ProviderRegistry::new(),
    ));
    let lock_manager = Arc::new(meld::concurrency::NodeLockManager::new());
    let api = ContextApi::new(
        node_store,
        frame_storage,
        head_index,
        prompt_context_storage,
        agent_registry,
        provider_registry,
        lock_manager,
    );
    (api, temp_dir)
}

fn register_writer_agent(api: &ContextApi) {
    let mut registry = api.agent_registry().write();
    let mut identity = AgentIdentity::new("writer".to_string(), AgentRole::Writer);
    identity
        .metadata
        .insert("system_prompt".to_string(), "system prompt".to_string());
    identity
        .metadata
        .insert("user_prompt_file".to_string(), "summarize file".to_string());
    identity.metadata.insert(
        "user_prompt_directory".to_string(),
        "summarize directory".to_string(),
    );
    registry.register(identity);
}

fn put_file_node(api: &ContextApi, node_id: NodeID, path: &Path, content: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
    api.node_store()
        .put(&NodeRecord {
            node_id,
            path: path.to_path_buf(),
            node_type: NodeType::File {
                size: content.len() as u64,
                content_hash: [7u8; 32],
            },
            children: vec![],
            parent: None,
            frame_set_root: None,
            metadata: Default::default(),
            tombstoned_at: None,
        })
        .unwrap();
}

fn put_directory_node(api: &ContextApi, node_id: NodeID, path: &Path, children: Vec<NodeID>) {
    fs::create_dir_all(path).unwrap();
    api.node_store()
        .put(&NodeRecord {
            node_id,
            path: path.to_path_buf(),
            node_type: NodeType::Directory,
            children,
            parent: None,
            frame_set_root: None,
            metadata: Default::default(),
            tombstoned_at: None,
        })
        .unwrap();
}

fn directory_request(node_id: NodeID) -> GenerationOrchestrationRequest {
    GenerationOrchestrationRequest {
        request_id: 1,
        node_id,
        agent_id: "writer".to_string(),
        provider: meld::provider::ProviderExecutionBinding::new(
            "mock-provider",
            meld::provider::ProviderRuntimeOverrides::default(),
        )
        .unwrap(),
        frame_type: "context-writer".to_string(),
        retry_count: 0,
        force: false,
    }
}

fn assemble_directory_prompt(
    api: &ContextApi,
    node_id: NodeID,
) -> meld::context::generation::contracts::PromptAssemblyOutput {
    let request = directory_request(node_id);
    let agent = api.get_agent(&request.agent_id).unwrap();
    let node_record = api.node_store().get(&request.node_id).unwrap().unwrap();
    let prompt_contract = PromptContract::from_agent(&agent).unwrap();
    build_prompt_messages(api, &request, &node_record, &prompt_contract).unwrap()
}

#[test]
fn directory_prompt_includes_file_child_source_when_no_frames_exist() {
    let (api, temp_dir) = create_test_api();
    register_writer_agent(&api);

    let root = temp_dir.path().join("root");
    let alpha_id = Hash::from([1u8; 32]);
    let beta_id = Hash::from([2u8; 32]);
    let dir_id = Hash::from([3u8; 32]);
    put_file_node(
        &api,
        alpha_id,
        &root.join("alpha.rs"),
        b"fn alpha_entry() { unique_alpha_marker(); }",
    );
    put_file_node(
        &api,
        beta_id,
        &root.join("beta.rs"),
        b"fn beta_entry() { unique_beta_marker(); }",
    );
    put_directory_node(&api, dir_id, &root, vec![alpha_id, beta_id]);

    let output = assemble_directory_prompt(&api, dir_id);

    assert!(
        output.context_payload.contains("unique_alpha_marker"),
        "file child source missing from cold directory context: {}",
        output.context_payload
    );
    assert!(output.context_payload.contains("unique_beta_marker"));
    assert!(
        output.messages[1].content.starts_with("Context:"),
        "cold directory prompt lost its context block"
    );
}

#[test]
fn empty_directory_prompt_carries_insufficient_context_marker() {
    let (api, temp_dir) = create_test_api();
    register_writer_agent(&api);

    let dir_id = Hash::from([4u8; 32]);
    put_directory_node(&api, dir_id, &temp_dir.path().join("empty"), vec![]);

    let output = assemble_directory_prompt(&api, dir_id);

    assert_eq!(output.context_payload, INSUFFICIENT_CONTEXT_MARKER);
    assert!(output.messages[1]
        .content
        .contains(&format!("Context:\n{}", INSUFFICIENT_CONTEXT_MARKER)));
}

#[test]
fn frameless_directory_child_is_still_skipped() {
    let (api, temp_dir) = create_test_api();
    register_writer_agent(&api);

    let root = temp_dir.path().join("root");
    let subdir_id = Hash::from([5u8; 32]);
    let file_id = Hash::from([6u8; 32]);
    let dir_id = Hash::from([7u8; 32]);
    put_directory_node(&api, subdir_id, &root.join("sub"), vec![]);
    put_file_node(
        &api,
        file_id,
        &root.join("gamma.rs"),
        b"fn gamma_entry() { unique_gamma_marker(); }",
    );
    put_directory_node(&api, dir_id, &root, vec![subdir_id, file_id]);

    let output = assemble_directory_prompt(&api, dir_id);

    assert!(output.context_payload.contains("unique_gamma_marker"));
    assert!(
        !output.context_payload.contains("root/sub"),
        "frameless directory child unexpectedly rendered: {}",
        output.context_payload
    );
}

#[test]
fn oversized_file_child_source_is_truncated_with_marker() {
    let (api, temp_dir) = create_test_api();
    register_writer_agent(&api);

    let root = temp_dir.path().join("root");
    let big_id = Hash::from([8u8; 32]);
    let dir_id = Hash::from([9u8; 32]);
    let big_content = vec![b'x'; 200 * 1024];
    put_file_node(&api, big_id, &root.join("big.txt"), &big_content);
    put_directory_node(&api, dir_id, &root, vec![big_id]);

    let output = assemble_directory_prompt(&api, dir_id);

    assert!(output.context_payload.contains("Truncated to"));
    assert!(output.context_payload.len() < big_content.len());
}
