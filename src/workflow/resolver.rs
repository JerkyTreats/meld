//! Workflow turn input and prompt resolution contracts.

use crate::api::ContextApi;
use crate::error::ApiError;
use crate::store::NodeType;
use crate::types::NodeID;
use crate::workflow::builtin::builtin_prompt_text;
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

    if !values.contains_key("target_context") {
        values.insert(
            "target_context".to_string(),
            collect_target_context(api, node_id, frame_type)?,
        );
    }

    let mut ordered_keys: Vec<String> = values.keys().cloned().collect();
    ordered_keys.sort();
    let context_payload = ordered_keys
        .into_iter()
        .map(|key| {
            let value = values.get(&key).cloned().unwrap_or_default();
            format!("{}:\n{}", key, value)
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
    workspace_root: &Path,
    profile_source_path: Option<&Path>,
    prompt_ref: &str,
) -> Result<String, ApiError> {
    match PromptRefKind::parse(prompt_ref) {
        PromptRefKind::Builtin(key) => builtin_prompt_text(&key)
            .map(str::to_string)
            .ok_or_else(|| ApiError::ConfigError(format!("Unknown builtin prompt_ref '{}'", key))),
        PromptRefKind::ArtifactId(artifact_id) => read_artifact_prompt(api, &artifact_id),
        PromptRefKind::FilePath(path) => {
            let resolved = resolve_prompt_path(&path, workspace_root, profile_source_path)
                .ok_or_else(|| {
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
        "{}\n\nTurn ID: {}\nOutput Type: {}\n\nInputs:\n{}",
        template, turn.turn_id, turn.output_type, inputs.context_payload
    )
}

fn collect_target_context(
    api: &ContextApi,
    node_id: NodeID,
    frame_type: &str,
) -> Result<String, ApiError> {
    let context = api.context_by_type(node_id, frame_type, 8)?;
    if !context.frames.is_empty() {
        return Ok(context
            .frames
            .iter()
            .map(|frame| String::from_utf8_lossy(&frame.content).to_string())
            .collect::<Vec<_>>()
            .join("\n\n"));
    }

    match context.node_record.node_type {
        NodeType::File { .. } => {
            let bytes = std::fs::read(&context.node_record.path).map_err(|err| {
                ApiError::ConfigError(format!(
                    "Failed to read file context '{}': {}",
                    context.node_record.path.display(),
                    err
                ))
            })?;
            Ok(String::from_utf8_lossy(&bytes).to_string())
        }
        NodeType::Directory => Ok(format!(
            "Directory: {}\nChildren: {}",
            context.node_record.path.display(),
            context.node_record.children.len()
        )),
    }
}

fn resolve_prompt_path(
    prompt_path: &str,
    workspace_root: &Path,
    profile_source_path: Option<&Path>,
) -> Option<PathBuf> {
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

    let workspace_candidate = workspace_root.join(&raw);
    if workspace_candidate.exists() {
        return Some(workspace_candidate);
    }

    None
}

fn read_artifact_prompt(api: &ContextApi, artifact_id: &str) -> Result<String, ApiError> {
    if artifact_id.len() != 64 || !artifact_id.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(ApiError::ConfigError(format!(
            "artifact prompt_ref id '{}' must be a 64 char hex digest",
            artifact_id
        )));
    }

    let artifact_path = api
        .prompt_context_storage()
        .root()
        .join(&artifact_id[0..2])
        .join(&artifact_id[2..4])
        .join(format!("{}.blob", artifact_id));

    let bytes = std::fs::read(&artifact_path).map_err(|err| {
        ApiError::ConfigError(format!(
            "Failed to read artifact prompt '{}' from {}: {}",
            artifact_id,
            artifact_path.display(),
            err
        ))
    })?;

    let digest = blake3::hash(&bytes).to_hex().to_string();
    if digest != artifact_id {
        return Err(ApiError::PromptContextArtifactDigestMismatch {
            artifact_id: artifact_id.to_string(),
            expected_digest: artifact_id.to_string(),
            actual_digest: digest,
        });
    }

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
    fn resolve_prompt_template_reads_builtin() {
        let (api, temp, _) = create_test_api();
        let resolved = resolve_prompt_template(
            &api,
            temp.path(),
            None,
            "builtin:docs_writer/evidence_gather",
        )
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
            prompt_ref: "builtin:docs_writer/evidence_gather".to_string(),
            input_refs: vec!["target_context".to_string()],
            output_type: "o1".to_string(),
            gate_id: "g1".to_string(),
            retry_limit: 1,
            timeout_ms: 1000,
        };

        let inputs =
            resolve_turn_inputs(&api, node_id, "context-writer", &turn, &HashMap::new()).unwrap();
        assert!(inputs.context_payload.contains("target_context"));
    }
}
