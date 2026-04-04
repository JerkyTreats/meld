//! Workflow turn input and prompt resolution contracts.

use crate::api::ContextApi;
use crate::context::frame::Frame;
use crate::error::ApiError;
use crate::store::{NodeRecord, NodeType};
use crate::types::NodeID;
use crate::workflow::profile::{PromptRefKind, WorkflowTurn};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTurnInputs {
    pub context_payload: String,
    pub values: HashMap<String, String>,
}

pub fn resolve_turn_inputs(
    api: &ContextApi,
    node_id: NodeID,
    frame_type: &str,
    turn: &WorkflowTurn,
    prior_outputs: &HashMap<String, String>,
) -> Result<ResolvedTurnInputs, ApiError> {
    let mut values = HashMap::new();

    for input_ref in &turn.input_refs {
        let resolved = if input_ref == "target_context" {
            collect_target_context(api, node_id, frame_type)?
        } else {
            prior_outputs.get(input_ref).cloned().ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Turn '{}' missing required input_ref '{}'",
                    turn.turn_id, input_ref
                ))
            })?
        };
        values.insert(input_ref.clone(), resolved);
    }

    let mut ordered_keys: Vec<String> = values.keys().cloned().collect();
    ordered_keys.sort();
    let context_payload = ordered_keys
        .into_iter()
        .map(|key| {
            let value = values.get(&key).cloned().unwrap_or_default();
            format_input_payload(&key, &value)
        })
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    Ok(ResolvedTurnInputs {
        context_payload,
        values,
    })
}

pub fn resolve_prompt_template(
    api: &ContextApi,
    profile_source_path: Option<&Path>,
    prompt_ref: &str,
) -> Result<String, ApiError> {
    match PromptRefKind::parse(prompt_ref) {
        PromptRefKind::ArtifactId(artifact_id) => read_artifact_prompt(api, &artifact_id),
        PromptRefKind::FilePath(path) => {
            let resolved = resolve_prompt_path(&path, profile_source_path).ok_or_else(|| {
                ApiError::ConfigError(format!("Unable to resolve prompt path '{}'", path))
            })?;
            std::fs::read_to_string(&resolved).map_err(|err| {
                ApiError::ConfigError(format!(
                    "Failed to read prompt path '{}': {}",
                    resolved.display(),
                    err
                ))
            })
        }
    }
}

pub fn render_turn_prompt(
    template: &str,
    turn: &WorkflowTurn,
    inputs: &ResolvedTurnInputs,
) -> String {
    format!(
        "{}\n\nTask:\nComplete workflow turn '{}' and return the '{}' artifact only.\n\nContext:\n{}",
        template,
        turn.turn_id,
        turn.output_type,
        if inputs.context_payload.trim().is_empty() {
            "Insufficient context".to_string()
        } else {
            inputs.context_payload.clone()
        }
    )
}

fn format_input_payload(key: &str, value: &str) -> String {
    if key == "target_context" {
        return value.to_string();
    }

    format!("Input: {}\nContent:\n{}", key, value)
}

fn collect_target_context(
    api: &ContextApi,
    node_id: NodeID,
    frame_type: &str,
) -> Result<String, ApiError> {
    let context = api.context_by_type(node_id, frame_type, 8)?;
    match context.node_record.node_type {
        NodeType::File { .. } => {
            collect_file_target_context(&context.node_record.path, &context.frames)
        }
        NodeType::Directory => {
            collect_directory_target_context(api, &context.node_record, frame_type)
        }
    }
}

fn collect_file_target_context(path: &Path, frames: &[Frame]) -> Result<String, ApiError> {
    let content = if !frames.is_empty() {
        frames
            .iter()
            .map(|frame| String::from_utf8_lossy(&frame.content).to_string())
            .collect::<Vec<_>>()
            .join("\n\n")
    } else {
        let bytes = std::fs::read(path).map_err(|err| {
            ApiError::ConfigError(format!(
                "Failed to read file context '{}': {}",
                path.display(),
                err
            ))
        })?;
        String::from_utf8_lossy(&bytes).to_string()
    };

    Ok(format_context_block(path, "File", &content))
}

fn collect_directory_target_context(
    api: &ContextApi,
    node_record: &NodeRecord,
    frame_type: &str,
) -> Result<String, ApiError> {
    let mut child_records = node_record
        .children
        .iter()
        .map(|child_id| {
            api.node_store()
                .get(child_id)
                .map_err(ApiError::from)?
                .ok_or(ApiError::NodeNotFound(*child_id))
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    child_records.sort_by(|left, right| {
        child_context_priority(&left.path)
            .cmp(&child_context_priority(&right.path))
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut blocks = vec![format_directory_summary_block(node_record, &child_records)];
    for child_record in child_records {
        blocks.push(collect_child_context_block(api, &child_record, frame_type)?);
    }

    Ok(blocks.join("\n\n---\n\n"))
}

fn child_context_priority(path: &Path) -> u8 {
    match path.file_name().and_then(|name| name.to_str()) {
        Some("README.md") => 0,
        Some("mod.rs") => 1,
        Some("lib.rs") => 2,
        _ => 3,
    }
}

fn collect_child_context_block(
    api: &ContextApi,
    child_record: &NodeRecord,
    frame_type: &str,
) -> Result<String, ApiError> {
    match child_record.node_type {
        NodeType::File { .. } => {
            let context = api.context_by_type(child_record.node_id, frame_type, 1)?;
            collect_file_target_context(&child_record.path, &context.frames)
        }
        NodeType::Directory => {
            let context = api.context_by_type(child_record.node_id, frame_type, 1)?;
            let content = if let Some(frame) = context.frames.first() {
                String::from_utf8_lossy(&frame.content).to_string()
            } else {
                format!(
                    "Child entries: {}\nStatus: Insufficient context",
                    child_record.children.len()
                )
            };
            Ok(format_context_block(
                &child_record.path,
                "Directory",
                &content,
            ))
        }
    }
}

fn format_directory_summary_block(
    node_record: &NodeRecord,
    child_records: &[NodeRecord],
) -> String {
    let children = if child_records.is_empty() {
        "none".to_string()
    } else {
        child_records
            .iter()
            .map(|child| format!("- {}", child.path.display()))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "Path: {}\nType: Directory\nContent:\nChild count: {}\nChild paths:\n{}",
        node_record.path.display(),
        child_records.len(),
        children
    )
}

fn format_context_block(path: &Path, node_type: &str, content: &str) -> String {
    format!(
        "Path: {}\nType: {}\nContent:\n{}",
        path.display(),
        node_type,
        content
    )
}

fn resolve_prompt_path(prompt_path: &str, profile_source_path: Option<&Path>) -> Option<PathBuf> {
    let raw = PathBuf::from(prompt_path);
    if raw.is_absolute() {
        if raw.exists() {
            return Some(raw);
        }
        return None;
    }

    if let Some(source) = profile_source_path {
        if let Some(parent) = source.parent() {
            let candidate = parent.join(&raw);
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    None
}

fn read_artifact_prompt(api: &ContextApi, artifact_id: &str) -> Result<String, ApiError> {
    let bytes = api
        .prompt_context_storage()
        .read_by_artifact_id_verified(artifact_id)?;

    String::from_utf8(bytes).map_err(|err| {
        ApiError::ConfigError(format!(
            "Artifact prompt '{}' is not valid utf8: {}",
            artifact_id, err
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentIdentity, AgentRole};
    use crate::context::frame::storage::FrameStorage;
    use crate::heads::HeadIndex;
    use crate::prompt_context::PromptContextArtifactStorage;
    use crate::store::persistence::SledNodeRecordStore;
    use crate::store::{NodeRecord, NodeType};
    use tempfile::TempDir;

    fn create_test_api() -> (ContextApi, TempDir, NodeID) {
        let temp_dir = TempDir::new().unwrap();
        let node_store = Arc::new(SledNodeRecordStore::new(temp_dir.path().join("store")).unwrap());
        let frame_storage = Arc::new(FrameStorage::new(temp_dir.path().join("frames")).unwrap());
        let prompt_context_storage =
            Arc::new(PromptContextArtifactStorage::new(temp_dir.path().join("artifacts")).unwrap());
        let head_index = Arc::new(parking_lot::RwLock::new(HeadIndex::new()));
        let agent_registry = Arc::new(parking_lot::RwLock::new(crate::agent::AgentRegistry::new()));
        agent_registry
            .write()
            .register(AgentIdentity::new("writer".to_string(), AgentRole::Writer));
        let provider_registry = Arc::new(parking_lot::RwLock::new(
            crate::provider::ProviderRegistry::new(),
        ));
        let lock_manager = Arc::new(crate::concurrency::NodeLockManager::new());
        let api = ContextApi::new(
            node_store,
            frame_storage,
            head_index,
            prompt_context_storage,
            agent_registry,
            provider_registry,
            lock_manager,
        );

        let node_id = crate::types::Hash::from([7u8; 32]);
        let file_path = temp_dir.path().join("README.md");
        std::fs::write(&file_path, "source context").unwrap();
        api.node_store()
            .put(&NodeRecord {
                node_id,
                path: file_path,
                node_type: NodeType::File {
                    size: 14,
                    content_hash: [9u8; 32],
                },
                children: vec![],
                parent: None,
                frame_set_root: None,
                metadata: Default::default(),
                tombstoned_at: None,
            })
            .unwrap();

        (api, temp_dir, node_id)
    }

    use std::sync::Arc;

    #[test]
    fn resolve_prompt_template_reads_file_prompt_ref() {
        let (api, temp, _) = create_test_api();
        let profile_path = temp
            .path()
            .join("workflows")
            .join("docs_writer_thread_v1.yaml");
        let prompt_path = temp
            .path()
            .join("workflows")
            .join("prompts")
            .join("evidence_gather.md");
        std::fs::create_dir_all(prompt_path.parent().unwrap()).unwrap();
        std::fs::write(&prompt_path, "evidence prompt").unwrap();
        std::fs::write(&profile_path, "workflow_id: docs_writer_thread_v1").unwrap();

        let resolved =
            resolve_prompt_template(&api, Some(&profile_path), "prompts/evidence_gather.md")
                .unwrap();
        assert!(resolved.contains("evidence"));
    }

    #[test]
    fn resolve_turn_inputs_includes_target_context() {
        let (api, _temp, node_id) = create_test_api();
        let turn = WorkflowTurn {
            turn_id: "t1".to_string(),
            seq: 1,
            title: "Turn one".to_string(),
            prompt_ref: "prompts/docs_writer/evidence_gather.md".to_string(),
            input_refs: vec!["target_context".to_string()],
            output_type: "o1".to_string(),
            gate_id: "g1".to_string(),
            retry_limit: 1,
            timeout_ms: 1000,
        };

        let inputs =
            resolve_turn_inputs(&api, node_id, "context-writer", &turn, &HashMap::new()).unwrap();
        assert!(inputs.context_payload.contains("Path:"));
        assert!(inputs.context_payload.contains("Type: File"));
        assert!(inputs.context_payload.contains("Content:"));
    }

    #[test]
    fn resolve_turn_inputs_only_includes_declared_inputs() {
        let (api, _temp, node_id) = create_test_api();
        let mut prior_outputs = HashMap::new();
        prior_outputs.insert(
            "verification_report".to_string(),
            "{\"verified_claims\":[]}".to_string(),
        );
        let turn = WorkflowTurn {
            turn_id: "t2".to_string(),
            seq: 2,
            title: "Turn two".to_string(),
            prompt_ref: "prompts/docs_writer/verification.md".to_string(),
            input_refs: vec!["verification_report".to_string()],
            output_type: "verification_report".to_string(),
            gate_id: "g1".to_string(),
            retry_limit: 1,
            timeout_ms: 1000,
        };

        let inputs =
            resolve_turn_inputs(&api, node_id, "context-writer", &turn, &prior_outputs).unwrap();

        assert!(inputs
            .context_payload
            .contains("Input: verification_report"));
        assert!(!inputs.context_payload.contains("Path:"));
    }

    #[test]
    fn resolve_turn_inputs_for_directory_uses_child_source_context() {
        let temp_dir = TempDir::new().unwrap();
        let node_store = Arc::new(SledNodeRecordStore::new(temp_dir.path().join("store")).unwrap());
        let frame_storage = Arc::new(FrameStorage::new(temp_dir.path().join("frames")).unwrap());
        let prompt_context_storage =
            Arc::new(PromptContextArtifactStorage::new(temp_dir.path().join("artifacts")).unwrap());
        let head_index = Arc::new(parking_lot::RwLock::new(HeadIndex::new()));
        let agent_registry = Arc::new(parking_lot::RwLock::new(crate::agent::AgentRegistry::new()));
        agent_registry
            .write()
            .register(AgentIdentity::new("writer".to_string(), AgentRole::Writer));
        let provider_registry = Arc::new(parking_lot::RwLock::new(
            crate::provider::ProviderRegistry::new(),
        ));
        let lock_manager = Arc::new(crate::concurrency::NodeLockManager::new());
        let api = ContextApi::new(
            node_store,
            frame_storage,
            head_index,
            prompt_context_storage,
            agent_registry,
            provider_registry,
            lock_manager,
        );

        let directory_id = crate::types::Hash::from([1u8; 32]);
        let child_id = crate::types::Hash::from([2u8; 32]);
        let directory_path = temp_dir.path().join("src").join("tree");
        std::fs::create_dir_all(&directory_path).unwrap();
        let child_path = directory_path.join("builder.rs");
        std::fs::write(&child_path, "pub struct Tree;\npub struct TreeBuilder;\n").unwrap();

        api.node_store()
            .put(&NodeRecord {
                node_id: child_id,
                path: child_path.clone(),
                node_type: NodeType::File {
                    size: 40,
                    content_hash: [3u8; 32],
                },
                children: vec![],
                parent: Some(directory_id),
                frame_set_root: None,
                metadata: Default::default(),
                tombstoned_at: None,
            })
            .unwrap();
        api.node_store()
            .put(&NodeRecord {
                node_id: directory_id,
                path: directory_path,
                node_type: NodeType::Directory,
                children: vec![child_id],
                parent: None,
                frame_set_root: None,
                metadata: Default::default(),
                tombstoned_at: None,
            })
            .unwrap();

        let stale_frame = crate::context::frame::Frame::new(
            crate::context::frame::Basis::Node(directory_id),
            b"stale directory readme".to_vec(),
            "context-docs-writer".to_string(),
            "writer".to_string(),
            crate::metadata::frame_write_contract::build_generated_metadata(
                &crate::metadata::frame_write_contract::GeneratedFrameMetadataInput {
                    agent_id: "writer".to_string(),
                    provider: "test-provider".to_string(),
                    model: "test-model".to_string(),
                    provider_type: "local".to_string(),
                    prompt_digest: "prompt-digest".to_string(),
                    context_digest: "context-digest".to_string(),
                    prompt_link_id: "prompt-link-prompt-digest".to_string(),
                },
            ),
        )
        .unwrap();
        api.put_frame(directory_id, stale_frame, "writer".to_string())
            .unwrap();

        let turn = WorkflowTurn {
            turn_id: "t1".to_string(),
            seq: 1,
            title: "Turn one".to_string(),
            prompt_ref: "prompts/docs_writer/evidence_gather.md".to_string(),
            input_refs: vec!["target_context".to_string()],
            output_type: "evidence_map".to_string(),
            gate_id: "g1".to_string(),
            retry_limit: 1,
            timeout_ms: 1000,
        };

        let inputs = resolve_turn_inputs(
            &api,
            directory_id,
            "context-docs-writer",
            &turn,
            &HashMap::new(),
        )
        .unwrap();

        assert!(inputs.context_payload.contains("Type: Directory"));
        assert!(inputs.context_payload.contains("builder.rs"));
        assert!(inputs.context_payload.contains("pub struct Tree"));
        assert!(!inputs.context_payload.contains("stale directory readme"));
    }

    #[test]
    fn resolve_prompt_template_reads_artifact_prompt_ref() {
        let (api, _temp, _) = create_test_api();
        let artifact = api
            .prompt_context_storage()
            .write_utf8(
                crate::prompt_context::PromptContextArtifactKind::RenderedPrompt,
                "artifact prompt body",
            )
            .unwrap();

        let prompt =
            resolve_prompt_template(&api, None, &format!("artifact:{}", artifact.artifact_id))
                .unwrap();
        assert_eq!(prompt, "artifact prompt body");
    }
}
