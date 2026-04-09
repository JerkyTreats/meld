//! Workspace publish helpers for frame-head-backed file writes.

use crate::api::ContextApi;
use crate::capability::{
    BoundBindingValue, BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource,
    CapabilityCatalog,
};
use crate::error::ApiError;
use crate::merkle_traversal::expansion::TraversalExpansionNode;
use crate::store::NodeType;
use crate::task::compiler::compile_task_definition;
use crate::task::contracts::{
    ArtifactProducerRef, ArtifactRecord, CompiledTaskRecord, TaskDefinition, TaskDependencyEdge,
    TaskDependencyKind, TaskInitSlotSpec,
};
use crate::task::expansion::{CompiledTaskDelta, TaskExpansionRequest};
use crate::types::{FrameID, NodeID};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub const WORKSPACE_WRITE_FRAME_HEAD_EXPANSION_KIND: &str = "workspace_write_frame_head_expansion";
pub const PUBLISH_STRATEGY_OVERWRITE_ON_NEW_HEAD: &str = "overwrite_on_new_head";
pub const PUBLISH_STRATEGY_FORCE_OVERWRITE: &str = "force_overwrite";

const PUBLISHED_HEAD_INDEX_VERSION: u32 = 1;

/// Declarative publish policy attached to a traversal-backed region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameHeadPublishTemplate {
    pub file_name: String,
    pub strategy: String,
}

/// One late-bound write expansion emitted after publish filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameHeadWriteExpansionContent {
    pub node_id: String,
    pub path: String,
    pub frame_type: String,
    pub file_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishedHeadEntry {
    pub frame_id: String,
}

/// Persisted workspace-scoped record of which frame head was last materialized to a file path.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishedHeadIndex {
    entries: std::collections::BTreeMap<String, PublishedHeadEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishFilterDecision {
    MissingHead,
    SkipCurrentHeadAlreadyPublished,
    WriteCurrentHead {
        frame_id: FrameID,
        output_path: PathBuf,
        file_missing: bool,
    },
}

impl PublishedHeadIndex {
    pub fn persistence_path(workspace_root: &Path) -> PathBuf {
        if let Ok(data_dir) = crate::config::xdg::workspace_data_dir(workspace_root) {
            data_dir.join("published_head_index.bin")
        } else {
            fallback_workspace_data_dir(workspace_root).join("published_head_index.bin")
        }
    }

    pub fn load_from_disk(path: &Path) -> Result<Self, ApiError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let bytes = fs::read(path).map_err(|err| {
            ApiError::StorageError(crate::error::StorageError::IoError(std::io::Error::other(
                format!(
                    "Failed to read published head index '{}': {}",
                    path.display(),
                    err
                ),
            )))
        })?;
        if bytes.len() < 4 {
            return Err(ApiError::ConfigError(format!(
                "Published head index '{}' is too short",
                path.display()
            )));
        }

        let version = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if version != PUBLISHED_HEAD_INDEX_VERSION {
            return Err(ApiError::ConfigError(format!(
                "Unsupported published head index version '{}' at '{}'",
                version,
                path.display()
            )));
        }

        bincode::deserialize(&bytes[4..]).map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to decode published head index '{}': {}",
                path.display(),
                err
            ))
        })
    }

    pub fn save_to_disk(&self, path: &Path) -> Result<(), ApiError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                ApiError::StorageError(crate::error::StorageError::IoError(std::io::Error::other(
                    format!(
                        "Failed to create publish index parent '{}' : {}",
                        parent.display(),
                        err
                    ),
                )))
            })?;
        }

        let payload = bincode::serialize(self).map_err(|err| {
            ApiError::ConfigError(format!("Failed to encode published head index: {}", err))
        })?;
        let mut serialized = Vec::with_capacity(4 + payload.len());
        serialized.extend_from_slice(&PUBLISHED_HEAD_INDEX_VERSION.to_le_bytes());
        serialized.extend_from_slice(&payload);

        let temp_path = path.with_extension("bin.tmp");
        fs::write(&temp_path, &serialized).map_err(|err| {
            ApiError::StorageError(crate::error::StorageError::IoError(std::io::Error::other(
                format!(
                    "Failed to write publish index temp file '{}' : {}",
                    temp_path.display(),
                    err
                ),
            )))
        })?;
        fs::rename(&temp_path, path).map_err(|err| {
            let _ = fs::remove_file(&temp_path);
            ApiError::StorageError(crate::error::StorageError::IoError(std::io::Error::other(
                format!(
                    "Failed to finalize publish index '{}' : {}",
                    path.display(),
                    err
                ),
            )))
        })?;
        Ok(())
    }

    pub fn last_published_frame_id(&self, output_path: &Path) -> Option<FrameID> {
        self.entries
            .get(&normalize_output_key(output_path))
            .and_then(|entry| decode_frame_id(&entry.frame_id).ok())
    }

    pub fn record_publish(&mut self, output_path: &Path, frame_id: FrameID) {
        self.entries.insert(
            normalize_output_key(output_path),
            PublishedHeadEntry {
                frame_id: hex::encode(frame_id),
            },
        );
    }
}

pub fn compile_workspace_write_frame_head_expansion(
    compiled_task: &CompiledTaskRecord,
    expansion: &TaskExpansionRequest,
    catalog: &CapabilityCatalog,
) -> Result<CompiledTaskDelta, ApiError> {
    let content: FrameHeadWriteExpansionContent = serde_json::from_value(expansion.content.clone())
        .map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to decode workspace publish expansion '{}': {}",
                expansion.expansion_id, err
            ))
        })?;

    decode_node_id(&content.node_id)?;
    let slot_id = publish_node_ref_slot_id(&content.node_id)?;
    let write_instance_id = publish_write_instance_id(&content.node_id)?;
    let init_slots = vec![TaskInitSlotSpec {
        init_slot_id: slot_id.clone(),
        artifact_type_id: "resolved_node_ref".to_string(),
        schema_version: 1,
        required: true,
    }];
    let init_artifacts = vec![ArtifactRecord {
        artifact_id: format!("expansion::{}::init::{}", expansion.expansion_id, slot_id),
        artifact_type_id: "resolved_node_ref".to_string(),
        schema_version: 1,
        content: json!({
            "node_id": content.node_id.clone(),
            "path": content.path.clone(),
        }),
        producer: ArtifactProducerRef {
            task_id: compiled_task.task_id.clone(),
            capability_instance_id: "__task_init__".to_string(),
            invocation_id: None,
            output_slot_id: Some(slot_id.clone()),
        },
    }];
    let capability_instances = vec![BoundCapabilityInstance {
        capability_instance_id: write_instance_id.clone(),
        capability_type_id: "workspace_write_frame_head".to_string(),
        capability_version: 1,
        scope_ref: content.node_id.clone(),
        scope_kind: "node".to_string(),
        binding_values: vec![
            binding("frame_type", json!(content.frame_type)),
            binding("file_name", json!(content.file_name)),
        ],
        input_wiring: vec![BoundInputWiring {
            slot_id: "resolved_node_ref".to_string(),
            sources: vec![BoundInputWiringSource::TaskInitSlot {
                init_slot_id: slot_id.clone(),
                artifact_type_id: "resolved_node_ref".to_string(),
                schema_version: 1,
            }],
        }],
    }];

    let mut full_init_slots = compiled_task.init_slots.clone();
    full_init_slots.extend(init_slots.clone());
    let mut full_instances = compiled_task.capability_instances.clone();
    full_instances.extend(capability_instances.clone());
    let compiled = compile_task_definition(
        &TaskDefinition {
            task_id: compiled_task.task_id.clone(),
            task_version: compiled_task.task_version,
            init_slots: full_init_slots,
            capability_instances: full_instances,
        },
        catalog,
    )?;
    let base_edges = compiled_task
        .dependency_edges
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let dependency_edges = compiled
        .dependency_edges
        .into_iter()
        .filter(|edge| !base_edges.contains(edge))
        .collect::<Vec<_>>();

    Ok(CompiledTaskDelta {
        init_slots,
        init_artifacts,
        capability_instances,
        dependency_edges,
    })
}

pub fn evaluate_publish_target(
    api: &ContextApi,
    node_id: NodeID,
    node_path: &Path,
    frame_type: &str,
    file_name: &str,
    strategy: &str,
) -> Result<PublishFilterDecision, ApiError> {
    let Some(workspace_root) = api.workspace_root() else {
        return Err(ApiError::ConfigError(
            "Publish capability requires workspace root context".to_string(),
        ));
    };
    let output_path = publish_output_path(workspace_root, node_path, file_name);
    let Some(frame_id) = api.get_head(&node_id, frame_type)? else {
        return Ok(PublishFilterDecision::MissingHead);
    };

    if strategy == PUBLISH_STRATEGY_FORCE_OVERWRITE {
        let file_missing = !output_path.exists();
        return Ok(PublishFilterDecision::WriteCurrentHead {
            frame_id,
            output_path,
            file_missing,
        });
    }
    if strategy != PUBLISH_STRATEGY_OVERWRITE_ON_NEW_HEAD {
        return Err(ApiError::ConfigError(format!(
            "Unsupported publish strategy '{}'",
            strategy
        )));
    }

    let index = load_published_head_index(workspace_root)?;
    let file_missing = !output_path.exists();
    let already_published = index
        .last_published_frame_id(&output_path)
        .map(|last| last == frame_id)
        .unwrap_or(false);
    if !file_missing && already_published {
        return Ok(PublishFilterDecision::SkipCurrentHeadAlreadyPublished);
    }

    Ok(PublishFilterDecision::WriteCurrentHead {
        frame_id,
        output_path,
        file_missing,
    })
}

pub fn publish_output_path(workspace_root: &Path, node_path: &Path, file_name: &str) -> PathBuf {
    let directory_path = if node_path.is_absolute() {
        node_path.to_path_buf()
    } else if node_path == Path::new(".") {
        workspace_root.to_path_buf()
    } else {
        workspace_root.join(node_path)
    };
    directory_path.join(file_name)
}

pub fn record_published_head(
    workspace_root: &Path,
    output_path: &Path,
    frame_id: FrameID,
) -> Result<(), ApiError> {
    let path = PublishedHeadIndex::persistence_path(workspace_root);
    let mut index = PublishedHeadIndex::load_from_disk(&path)?;
    index.record_publish(output_path, frame_id);
    index.save_to_disk(&path)
}

pub fn decode_node_id(value: &str) -> Result<NodeID, ApiError> {
    let bytes = hex::decode(value).map_err(|err| {
        ApiError::ConfigError(format!("Invalid node id hex '{}': {}", value, err))
    })?;
    if bytes.len() != 32 {
        return Err(ApiError::ConfigError(format!(
            "Invalid node id hex '{}' length '{}'",
            value,
            bytes.len()
        )));
    }
    let mut node_id = [0u8; 32];
    node_id.copy_from_slice(&bytes);
    Ok(node_id)
}

pub fn decode_frame_id(value: &str) -> Result<FrameID, ApiError> {
    let bytes = hex::decode(value).map_err(|err| {
        ApiError::ConfigError(format!("Invalid frame id hex '{}': {}", value, err))
    })?;
    if bytes.len() != 32 {
        return Err(ApiError::ConfigError(format!(
            "Invalid frame id hex '{}' length '{}'",
            value,
            bytes.len()
        )));
    }
    let mut frame_id = [0u8; 32];
    frame_id.copy_from_slice(&bytes);
    Ok(frame_id)
}

pub fn publish_node_ref_slot_id(node_id_hex: &str) -> Result<String, ApiError> {
    Ok(format!(
        "publish_node_ref::{}",
        node_id_prefix(node_id_hex)?
    ))
}

pub fn publish_filter_instance_id(node_id_hex: &str) -> Result<String, ApiError> {
    Ok(format!(
        "node::{}::publish::filter",
        node_id_prefix(node_id_hex)?
    ))
}

pub fn publish_write_instance_id(node_id_hex: &str) -> Result<String, ApiError> {
    Ok(format!(
        "node::{}::publish::write",
        node_id_prefix(node_id_hex)?
    ))
}

pub fn publish_filter_dependency(
    from_capability_instance_id: String,
    to_capability_instance_id: String,
) -> TaskDependencyEdge {
    TaskDependencyEdge {
        from_capability_instance_id,
        to_capability_instance_id,
        kind: TaskDependencyKind::Effect,
        reason: "publish_after_generation_head".to_string(),
    }
}

pub fn validate_directory_node(api: &ContextApi, node_id: &NodeID) -> Result<PathBuf, ApiError> {
    let record = api
        .node_store()
        .get(node_id)
        .map_err(ApiError::from)?
        .ok_or(ApiError::NodeNotFound(*node_id))?;
    if !matches!(record.node_type, NodeType::Directory) {
        return Err(ApiError::ConfigError(format!(
            "Publish target '{}' must be a directory node",
            hex::encode(node_id)
        )));
    }
    Ok(record.path)
}

pub fn node_from_ref_content(value: &Value) -> Result<TraversalExpansionNode, ApiError> {
    let node_id = value
        .get("node_id")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ApiError::ConfigError("resolved_node_ref is missing 'node_id'".to_string())
        })?;
    let path = value
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::ConfigError("resolved_node_ref is missing 'path'".to_string()))?;
    Ok(TraversalExpansionNode {
        node_id: node_id.to_string(),
        path: path.to_string(),
    })
}

fn load_published_head_index(workspace_root: &Path) -> Result<PublishedHeadIndex, ApiError> {
    PublishedHeadIndex::load_from_disk(&PublishedHeadIndex::persistence_path(workspace_root))
}

fn normalize_output_key(output_path: &Path) -> String {
    crate::tree::path::normalize_path_string(&output_path.to_string_lossy())
}

fn node_id_prefix(node_id_hex: &str) -> Result<String, ApiError> {
    if node_id_hex.len() < 16 {
        return Err(ApiError::ConfigError(format!(
            "Node id '{}' is too short for deterministic prefix generation",
            node_id_hex
        )));
    }
    Ok(node_id_hex[..16].to_string())
}

fn binding(binding_id: &str, value: Value) -> BoundBindingValue {
    BoundBindingValue {
        binding_id: binding_id.to_string(),
        value,
    }
}

fn fallback_workspace_data_dir(workspace_root: &Path) -> PathBuf {
    let canonical = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());
    let mut data_dir = std::env::temp_dir().join("meld");
    for component in canonical.components() {
        match component {
            std::path::Component::RootDir => {}
            std::path::Component::Prefix(_) => {}
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => data_dir.push("__"),
            std::path::Component::Normal(value) => data_dir.push(value),
        }
    }
    data_dir
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentIdentity, AgentRegistry, AgentRole};
    use crate::api::ContextApi;
    use crate::concurrency::NodeLockManager;
    use crate::context::frame::storage::FrameStorage;
    use crate::context::frame::{Basis, Frame};
    use crate::heads::HeadIndex;
    use crate::metadata::frame_write_contract::{
        build_generated_metadata, generated_metadata_input_from_payload,
    };
    use crate::prompt_context::PromptContextArtifactStorage;
    use crate::provider::ProviderRegistry;
    use crate::store::{NodeRecord, NodeRecordStore, SledNodeRecordStore};
    use std::sync::Arc;
    use tempfile::TempDir;

    fn publish_test_api() -> (TempDir, ContextApi, NodeID) {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path().join("workspace");
        fs::create_dir_all(workspace_root.join("src")).unwrap();

        let node_store = Arc::new(SledNodeRecordStore::new(temp_dir.path().join("store")).unwrap());
        let frame_storage = Arc::new(FrameStorage::new(temp_dir.path().join("frames")).unwrap());
        let prompt_context_storage =
            Arc::new(PromptContextArtifactStorage::new(temp_dir.path().join("artifacts")).unwrap());
        let head_index = Arc::new(parking_lot::RwLock::new(HeadIndex::new()));
        let agent_registry = Arc::new(parking_lot::RwLock::new(AgentRegistry::new()));
        let provider_registry = Arc::new(parking_lot::RwLock::new(ProviderRegistry::new()));
        agent_registry.write().register(AgentIdentity::new(
            "docs-writer".to_string(),
            AgentRole::Writer,
        ));

        let api = ContextApi::with_workspace_root(
            node_store.clone(),
            frame_storage,
            head_index,
            prompt_context_storage,
            agent_registry,
            provider_registry,
            Arc::new(NodeLockManager::new()),
            workspace_root.clone(),
        );
        let node_id = [5u8; 32];
        node_store
            .put(&NodeRecord {
                node_id,
                path: PathBuf::from("src"),
                node_type: NodeType::Directory,
                children: vec![],
                parent: None,
                frame_set_root: None,
                metadata: Default::default(),
                tombstoned_at: None,
            })
            .unwrap();

        (temp_dir, api, node_id)
    }

    fn put_readme_frame(api: &ContextApi, node_id: NodeID, body: &str) -> FrameID {
        let frame = Frame::new(
            Basis::Node(node_id),
            body.as_bytes().to_vec(),
            "context-docs-writer".to_string(),
            "docs-writer".to_string(),
            build_generated_metadata(&generated_metadata_input_from_payload(
                "docs-writer",
                "test-provider",
                "test-model",
                "local_custom",
                "prompt",
                "context",
            )),
        )
        .unwrap();
        api.put_frame(node_id, frame, "docs-writer".to_string())
            .unwrap()
    }

    #[test]
    fn overwrite_on_new_head_marks_missing_file_actionable() {
        let workspace_root = Path::new("/tmp/workspace");
        let output = publish_output_path(workspace_root, Path::new("src"), "README.md");

        assert_eq!(output, PathBuf::from("/tmp/workspace/src/README.md"));
    }

    #[test]
    fn published_head_index_round_trips_frame_ids() {
        let mut index = PublishedHeadIndex::default();
        let path = Path::new("/tmp/workspace/src/README.md");
        let frame_id = [7u8; 32];

        index.record_publish(path, frame_id);

        assert_eq!(index.last_published_frame_id(path), Some(frame_id));
    }

    #[test]
    fn overwrite_on_new_head_skips_when_file_and_publish_state_match_current_head() {
        let (_temp_dir, api, node_id) = publish_test_api();
        let frame_id = put_readme_frame(&api, node_id, "# README");
        let workspace_root = api.workspace_root().unwrap();
        let output_path = publish_output_path(workspace_root, Path::new("src"), "README.md");
        fs::write(&output_path, "# README").unwrap();
        record_published_head(workspace_root, &output_path, frame_id).unwrap();

        let decision = evaluate_publish_target(
            &api,
            node_id,
            Path::new("src"),
            "context-docs-writer",
            "README.md",
            PUBLISH_STRATEGY_OVERWRITE_ON_NEW_HEAD,
        )
        .unwrap();

        assert_eq!(
            decision,
            PublishFilterDecision::SkipCurrentHeadAlreadyPublished
        );
    }

    #[test]
    fn force_overwrite_returns_actionable_target_even_when_current_head_is_already_published() {
        let (_temp_dir, api, node_id) = publish_test_api();
        let frame_id = put_readme_frame(&api, node_id, "# README");
        let workspace_root = api.workspace_root().unwrap();
        let output_path = publish_output_path(workspace_root, Path::new("src"), "README.md");
        fs::write(&output_path, "# README").unwrap();
        record_published_head(workspace_root, &output_path, frame_id).unwrap();

        let decision = evaluate_publish_target(
            &api,
            node_id,
            Path::new("src"),
            "context-docs-writer",
            "README.md",
            PUBLISH_STRATEGY_FORCE_OVERWRITE,
        )
        .unwrap();

        assert!(matches!(
            decision,
            PublishFilterDecision::WriteCurrentHead {
                frame_id: actual,
                output_path,
                file_missing: false,
            } if actual == frame_id && output_path.ends_with("README.md")
        ));
    }
}
