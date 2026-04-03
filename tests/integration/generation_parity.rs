use meld::agent::profile::prompt_contract::PromptContract;
use meld::agent::{AgentIdentity, AgentRole};
use meld::api::ContextApi;
use meld::context::frame::{storage::FrameStorage, Basis, Frame};
use meld::context::generation::contracts::GenerationOrchestrationRequest;
use meld::context::generation::metadata_construction::build_and_validate_generated_metadata;
use meld::context::generation::prompt_collection::build_prompt_messages;
use meld::context::queue::{FrameGenerationQueue, GenerationConfig, Priority, QueueEventContext};
use meld::error::ApiError;
use meld::heads::HeadIndex;
use meld::metadata::frame_write_contract::{
    build_generated_metadata, generated_metadata_input_from_payload,
};
use meld::prompt_context::PromptContextArtifactStorage;
use meld::store::persistence::SledNodeRecordStore;
use meld::store::{NodeRecord, NodeType};
use meld::telemetry::ProgressRuntime;
use meld::types::{FrameID, Hash, NodeID};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

#[derive(Serialize)]
struct ParityArtifact {
    schema_version: u32,
    scenario_id: String,
    provider_request: Option<Value>,
    frame_output: Option<FrameOutput>,
    retry: RetryOutput,
}

#[derive(Serialize)]
struct FrameOutput {
    content: String,
    frame_type: String,
    metadata: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct RetryOutput {
    attempt_count: usize,
    retry_event_count: usize,
    backoff_class: String,
    terminal_error_class: Option<String>,
}

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
    let head_index = Arc::new(parking_lot::RwLock::new(HeadIndex::new()));
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

fn register_writer_agent(api: &ContextApi, agent_id: &str, include_prompt_templates: bool) {
    let mut registry = api.agent_registry().write();
    let mut identity = AgentIdentity::new(agent_id.to_string(), AgentRole::Writer);
    identity
        .metadata
        .insert("system_prompt".to_string(), "system prompt".to_string());
    if include_prompt_templates {
        identity
            .metadata
            .insert("user_prompt_file".to_string(), "summarize file".to_string());
        identity.metadata.insert(
            "user_prompt_directory".to_string(),
            "summarize directory".to_string(),
        );
    }
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

fn terminal_error_class(error: &ApiError) -> String {
    match error {
        ApiError::NodeNotFound(_) => "NodeNotFound",
        ApiError::FrameNotFound(_) => "FrameNotFound",
        ApiError::Unauthorized(_) => "Unauthorized",
        ApiError::InvalidFrame(_) => "InvalidFrame",
        ApiError::FrameMetadataPolicyViolation(_) => "FrameMetadataPolicyViolation",
        ApiError::FrameMetadataUnknownKey { .. } => "FrameMetadataUnknownKey",
        ApiError::FrameMetadataForbiddenKey { .. } => "FrameMetadataForbiddenKey",
        ApiError::FrameMetadataMissingRequiredKey { .. } => "FrameMetadataMissingRequiredKey",
        ApiError::FrameMetadataPerKeyBudgetExceeded { .. } => "FrameMetadataPerKeyBudgetExceeded",
        ApiError::FrameMetadataTotalBudgetExceeded { .. } => "FrameMetadataTotalBudgetExceeded",
        ApiError::FrameMetadataMutabilityViolation { .. } => "FrameMetadataMutabilityViolation",
        ApiError::PromptContextArtifactBudgetExceeded { .. } => {
            "PromptContextArtifactBudgetExceeded"
        }
        ApiError::PromptContextArtifactNotFound { .. } => "PromptContextArtifactNotFound",
        ApiError::PromptContextArtifactDigestMismatch { .. } => {
            "PromptContextArtifactDigestMismatch"
        }
        ApiError::PromptContextArtifactSizeMismatch { .. } => "PromptContextArtifactSizeMismatch",
        ApiError::PromptLinkContractInvalid { .. } => "PromptLinkContractInvalid",
        ApiError::WorkflowRecordContractInvalid { .. } => "WorkflowRecordContractInvalid",
        ApiError::WorkflowRecordReferenceInvalid { .. } => "WorkflowRecordReferenceInvalid",
        ApiError::MissingPromptContractField { .. } => "MissingPromptContractField",
        ApiError::ProviderError(_) => "ProviderError",
        ApiError::ProviderNotConfigured(_) => "ProviderNotConfigured",
        ApiError::ProviderRequestFailed(_) => "ProviderRequestFailed",
        ApiError::ProviderAuthFailed(_) => "ProviderAuthFailed",
        ApiError::ProviderRateLimit(_) => "ProviderRateLimit",
        ApiError::ProviderModelNotFound(_) => "ProviderModelNotFound",
        ApiError::StorageError(_) => "StorageError",
        ApiError::ConfigError(_) => "ConfigError",
        ApiError::GenerationFailed(_) => "GenerationFailed",
        ApiError::PathNotInTree(_) => "PathNotInTree",
    }
    .to_string()
}

fn normalize_value(value: &mut Value, workspace_root: &Path) {
    let root = workspace_root.to_string_lossy();
    match value {
        Value::String(s) => {
            if s.contains(root.as_ref()) {
                *s = s.replace(root.as_ref(), "<ROOT>");
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_value(item, workspace_root);
            }
        }
        Value::Object(map) => {
            for (_, item) in map {
                normalize_value(item, workspace_root);
            }
        }
        _ => {}
    }
}

fn fixture_path(scenario_id: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("generation_parity")
        .join(format!("{}.json", scenario_id))
}

fn assert_matches_fixture(scenario_id: &str, artifact: &ParityArtifact) {
    let path = fixture_path(scenario_id);
    let mut actual = serde_json::to_value(artifact).unwrap();
    normalize_context_digest(&mut actual);

    if std::env::var_os("MELD_REGEN_PARITY_FIXTURES").is_some() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, serde_json::to_string_pretty(&actual).unwrap()).unwrap();
        return;
    }

    let mut expected: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    normalize_context_digest(&mut expected);
    assert_eq!(
        actual,
        expected,
        "parity fixture mismatch for {}\nactual:\n{}\nexpected:\n{}",
        scenario_id,
        serde_json::to_string_pretty(&actual).unwrap(),
        serde_json::to_string_pretty(&expected).unwrap(),
    );
}

fn normalize_context_digest(value: &mut Value) {
    let metadata = value
        .get_mut("frame_output")
        .and_then(|frame_output| frame_output.get_mut("metadata"))
        .and_then(Value::as_object_mut);

    if let Some(metadata) = metadata {
        if metadata.contains_key("context_digest") {
            metadata.insert(
                "context_digest".to_string(),
                Value::String("<CONTEXT_DIGEST>".to_string()),
            );
        }
    }
}

fn event_counts(progress: &ProgressRuntime, session_id: &str) -> (usize, usize) {
    let events = progress.store().read_events(session_id).unwrap();
    let attempts = events
        .iter()
        .filter(|event| event.event_type == "request_processing")
        .count();
    let retries = events
        .iter()
        .filter(|event| event.event_type == "provider_request_retrying")
        .count();
    (attempts, retries)
}

fn frame_output(api: &ContextApi, frame_id: FrameID) -> FrameOutput {
    let frame = api.frame_storage().get(&frame_id).unwrap().unwrap();
    let metadata = frame
        .metadata
        .into_iter()
        .collect::<BTreeMap<String, String>>();
    FrameOutput {
        content: String::from_utf8(frame.content).unwrap(),
        frame_type: frame.frame_type,
        metadata,
    }
}

fn direct_generation_artifact(
    api: &ContextApi,
    temp_dir: &TempDir,
    request: &GenerationOrchestrationRequest,
    generated_content: &str,
    scenario_id: &str,
) -> ParityArtifact {
    let agent = api.get_agent(&request.agent_id).unwrap();
    let node_record = api.node_store().get(&request.node_id).unwrap().unwrap();
    let prompt_contract = PromptContract::from_agent(&agent).unwrap();
    let prompt_output =
        build_prompt_messages(api, request, &node_record, &prompt_contract).unwrap();

    let metadata_input = generated_metadata_input_from_payload(
        &request.agent_id,
        &request.provider.provider_name,
        "mock-model",
        "local",
        &prompt_output.rendered_prompt,
        &prompt_output.context_payload,
    );
    let metadata = build_and_validate_generated_metadata(
        api,
        request,
        &metadata_input,
        &build_generated_metadata,
    )
    .unwrap();

    let mut provider_request = json!({
        "model": "mock-model",
        "messages": prompt_output.messages,
    });
    normalize_value(&mut provider_request, temp_dir.path());

    let frame = Frame::new(
        Basis::Node(request.node_id),
        generated_content.as_bytes().to_vec(),
        request.frame_type.clone(),
        request.agent_id.clone(),
        metadata,
    )
    .unwrap();
    let frame_id = api
        .put_frame(request.node_id, frame, request.agent_id.clone())
        .unwrap();

    ParityArtifact {
        schema_version: 1,
        scenario_id: scenario_id.to_string(),
        provider_request: Some(provider_request),
        frame_output: Some(frame_output(api, frame_id)),
        retry: RetryOutput {
            attempt_count: 1,
            retry_event_count: 0,
            backoff_class: "none".to_string(),
            terminal_error_class: None,
        },
    }
}

#[test]
fn generation_parity_file_success_matches_fixture() {
    let (api, temp_dir) = create_test_api();
    register_writer_agent(&api, "writer", true);

    let file_path = temp_dir.path().join("input.txt");
    let node_id = Hash::from([1u8; 32]);
    put_file_node(&api, node_id, &file_path, b"alpha");

    let request = GenerationOrchestrationRequest {
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
        force: true,
    };

    let artifact =
        direct_generation_artifact(&api, &temp_dir, &request, "generated file", "file_success");
    assert_matches_fixture("file_success", &artifact);
}

#[test]
fn generation_parity_directory_success_matches_fixture() {
    let (api, temp_dir) = create_test_api();
    register_writer_agent(&api, "writer", true);

    let root_dir = temp_dir.path().join("root");
    let child_file = root_dir.join("child.txt");
    let dir_node = Hash::from([2u8; 32]);
    let child_node = Hash::from([3u8; 32]);

    put_file_node(&api, child_node, &child_file, b"child data");
    put_directory_node(&api, dir_node, &root_dir, vec![child_node]);

    let seed_frame = Frame::new(
        Basis::Node(child_node),
        b"child frame context".to_vec(),
        "context-writer".to_string(),
        "writer".to_string(),
        build_generated_metadata(&generated_metadata_input_from_payload(
            "writer",
            "mock-provider",
            "mock-model",
            "local",
            "seed-prompt",
            "seed-context",
        )),
    )
    .unwrap();
    api.put_frame(child_node, seed_frame, "writer".to_string())
        .unwrap();

    let request = GenerationOrchestrationRequest {
        request_id: 1,
        node_id: dir_node,
        agent_id: "writer".to_string(),
        provider: meld::provider::ProviderExecutionBinding::new(
            "mock-provider",
            meld::provider::ProviderRuntimeOverrides::default(),
        )
        .unwrap(),
        frame_type: "context-writer".to_string(),
        retry_count: 0,
        force: true,
    };

    let artifact = direct_generation_artifact(
        &api,
        &temp_dir,
        &request,
        "generated directory",
        "directory_success",
    );
    assert_matches_fixture("directory_success", &artifact);
}

#[tokio::test]
async fn generation_parity_retryable_failure_matches_fixture() {
    let (api, temp_dir) = create_test_api();
    let api = Arc::new(api);
    register_writer_agent(api.as_ref(), "writer", true);

    let db = sled::open(temp_dir.path().join("progress")).unwrap();
    let progress = Arc::new(ProgressRuntime::new(db).unwrap());
    let session_id = progress
        .start_command_session("generation.parity.retryable".to_string())
        .unwrap();

    let mut config = GenerationConfig::default();
    config.max_retry_attempts = 2;
    config.retry_delay_ms = 5;
    let queue = FrameGenerationQueue::with_event_context(
        Arc::clone(&api),
        config,
        Some(QueueEventContext {
            session_id: session_id.clone(),
            progress: Arc::clone(&progress),
        }),
    );

    queue.start().unwrap();
    let result = queue
        .enqueue_and_wait(
            Hash::from([9u8; 32]),
            "writer".to_string(),
            "mock-provider".to_string(),
            Some("context-writer".to_string()),
            Priority::Normal,
            Some(Duration::from_secs(5)),
        )
        .await;
    queue.stop().await.unwrap();

    progress
        .finish_command_session(
            &session_id,
            result.is_ok(),
            result.as_ref().err().map(|e| e.to_string()),
        )
        .unwrap();

    let error = result.unwrap_err();
    let (attempt_count, retry_event_count) = event_counts(&progress, &session_id);

    let artifact = ParityArtifact {
        schema_version: 1,
        scenario_id: "retryable_failure".to_string(),
        provider_request: None,
        frame_output: None,
        retry: RetryOutput {
            attempt_count,
            retry_event_count,
            backoff_class: if retry_event_count > 0 {
                "fixed_delay".to_string()
            } else {
                "none".to_string()
            },
            terminal_error_class: Some(terminal_error_class(&error)),
        },
    };

    assert_matches_fixture("retryable_failure", &artifact);
}

#[tokio::test]
async fn generation_parity_non_retryable_failure_matches_fixture() {
    let (api, temp_dir) = create_test_api();
    let api = Arc::new(api);
    register_writer_agent(api.as_ref(), "writer", false);

    let file_path = temp_dir.path().join("input.txt");
    let node_id = Hash::from([5u8; 32]);
    put_file_node(api.as_ref(), node_id, &file_path, b"alpha");

    let db = sled::open(temp_dir.path().join("progress")).unwrap();
    let progress = Arc::new(ProgressRuntime::new(db).unwrap());
    let session_id = progress
        .start_command_session("generation.parity.non_retryable".to_string())
        .unwrap();

    let mut config = GenerationConfig::default();
    config.max_retry_attempts = 4;
    config.retry_delay_ms = 5;
    let queue = FrameGenerationQueue::with_event_context(
        Arc::clone(&api),
        config,
        Some(QueueEventContext {
            session_id: session_id.clone(),
            progress: Arc::clone(&progress),
        }),
    );

    queue.start().unwrap();
    let result = queue
        .enqueue_and_wait(
            node_id,
            "writer".to_string(),
            "mock-provider".to_string(),
            Some("context-writer".to_string()),
            Priority::Normal,
            Some(Duration::from_secs(5)),
        )
        .await;
    queue.stop().await.unwrap();

    progress
        .finish_command_session(
            &session_id,
            result.is_ok(),
            result.as_ref().err().map(|e| e.to_string()),
        )
        .unwrap();

    let error = result.unwrap_err();
    let (attempt_count, retry_event_count) = event_counts(&progress, &session_id);

    let artifact = ParityArtifact {
        schema_version: 1,
        scenario_id: "non_retryable_failure".to_string(),
        provider_request: None,
        frame_output: None,
        retry: RetryOutput {
            attempt_count,
            retry_event_count,
            backoff_class: if retry_event_count > 0 {
                "fixed_delay".to_string()
            } else {
                "none".to_string()
            },
            terminal_error_class: Some(terminal_error_class(&error)),
        },
    };

    assert_matches_fixture("non_retryable_failure", &artifact);
}
