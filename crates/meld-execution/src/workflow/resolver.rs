use crate::error::ApiError;
use crate::execution::{
    ContextReadPort, ExecutionFrame, ExecutionNodeKind, ExecutionNodeRecord, PromptArtifactReadPort,
};
use crate::generation::NodeId;
use crate::workflow::profile::{PromptRefKind, WorkflowTurn};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTurnInputs {
    pub context_payload: String,
    pub values: HashMap<String, String>,
}

pub fn resolve_turn_inputs<E>(
    api: &(impl ContextReadPort<Error = E, NodeId = NodeId> + ?Sized),
    node_id: NodeId,
    frame_type: &str,
    turn: &WorkflowTurn,
    prior_outputs: &HashMap<String, String>,
) -> Result<ResolvedTurnInputs, E>
where
    E: From<ApiError>,
{
    let mut values = HashMap::new();

    for input_ref in &turn.input_refs {
        let resolved = if input_ref == "target_context" {
            collect_target_context(api, node_id, frame_type)?
        } else {
            prior_outputs.get(input_ref).cloned().ok_or_else(|| {
                E::from(ApiError::ConfigError(format!(
                    "Turn '{}' missing required input_ref '{}'",
                    turn.turn_id, input_ref
                )))
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

pub fn resolve_prompt_template<E>(
    api: &(impl PromptArtifactReadPort<Error = E> + ?Sized),
    profile_source_path: Option<&Path>,
    prompt_ref: &str,
) -> Result<String, E>
where
    E: From<ApiError>,
{
    match PromptRefKind::parse(prompt_ref) {
        PromptRefKind::ArtifactId(artifact_id) => read_artifact_prompt(api, &artifact_id),
        PromptRefKind::FilePath(path) => {
            let resolved = resolve_prompt_path(&path, profile_source_path).ok_or_else(|| {
                E::from(ApiError::ConfigError(format!(
                    "Unable to resolve prompt path '{}'",
                    path
                )))
            })?;
            std::fs::read_to_string(&resolved).map_err(|err| {
                E::from(ApiError::ConfigError(format!(
                    "Failed to read prompt path '{}': {}",
                    resolved.display(),
                    err
                )))
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

fn collect_target_context<E>(
    api: &(impl ContextReadPort<Error = E, NodeId = NodeId> + ?Sized),
    node_id: NodeId,
    frame_type: &str,
) -> Result<String, E>
where
    E: From<ApiError>,
{
    let context = api.context_frames_by_type(node_id, frame_type, 8)?;
    match context.node_record.node_kind {
        ExecutionNodeKind::File => {
            collect_file_target_context(&context.node_record.path, &context.frames)
        }
        ExecutionNodeKind::Directory => {
            collect_directory_target_context(api, &context.node_record, frame_type)
        }
    }
}

fn collect_file_target_context<F, E>(path: &str, frames: &[ExecutionFrame<F>]) -> Result<String, E>
where
    E: From<ApiError>,
{
    let content = if !frames.is_empty() {
        frames
            .iter()
            .map(|frame| String::from_utf8_lossy(&frame.content).to_string())
            .collect::<Vec<_>>()
            .join("\n\n")
    } else {
        let bytes = std::fs::read(path).map_err(|err| {
            E::from(ApiError::ConfigError(format!(
                "Failed to read file context '{}': {}",
                path, err
            )))
        })?;
        String::from_utf8_lossy(&bytes).to_string()
    };

    Ok(format_context_block(path, "File", &content))
}

fn collect_directory_target_context<E>(
    api: &(impl ContextReadPort<Error = E, NodeId = NodeId> + ?Sized),
    node_record: &ExecutionNodeRecord<NodeId>,
    frame_type: &str,
) -> Result<String, E>
where
    E: From<ApiError>,
{
    let mut child_records = node_record
        .children
        .iter()
        .map(|child_id| {
            api.read_execution_node_record(child_id)?
                .ok_or(E::from(ApiError::ConfigError(format!(
                    "Directory context is missing child node '{}'",
                    hex::encode(child_id),
                ))))
        })
        .collect::<Result<Vec<_>, E>>()?;
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

fn child_context_priority(path: &str) -> u8 {
    match Path::new(path).file_name().and_then(|name| name.to_str()) {
        Some("README.md") => 0,
        Some("mod.rs") => 1,
        Some("lib.rs") => 2,
        _ => 3,
    }
}

fn collect_child_context_block<E>(
    api: &(impl ContextReadPort<Error = E, NodeId = NodeId> + ?Sized),
    child_record: &ExecutionNodeRecord<NodeId>,
    frame_type: &str,
) -> Result<String, E>
where
    E: From<ApiError>,
{
    match child_record.node_kind {
        ExecutionNodeKind::File => {
            let context = api.context_frames_by_type(child_record.node_id, frame_type, 1)?;
            collect_file_target_context(&child_record.path, &context.frames)
        }
        ExecutionNodeKind::Directory => {
            let context = api.context_frames_by_type(child_record.node_id, frame_type, 1)?;
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
    node_record: &ExecutionNodeRecord<NodeId>,
    child_records: &[ExecutionNodeRecord<NodeId>],
) -> String {
    let children = if child_records.is_empty() {
        "none".to_string()
    } else {
        child_records
            .iter()
            .map(|child| format!("- {}", child.path))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "Path: {}\nType: Directory\nContent:\nChild count: {}\nChild paths:\n{}",
        node_record.path,
        child_records.len(),
        children
    )
}

fn format_context_block(path: &str, node_type: &str, content: &str) -> String {
    format!("Path: {}\nType: {}\nContent:\n{}", path, node_type, content)
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

fn read_artifact_prompt<E>(
    api: &(impl PromptArtifactReadPort<Error = E> + ?Sized),
    artifact_id: &str,
) -> Result<String, E>
where
    E: From<ApiError>,
{
    let bytes = api.read_prompt_artifact_bytes(artifact_id)?;

    String::from_utf8(bytes).map_err(|err| {
        E::from(ApiError::ConfigError(format!(
            "Artifact prompt '{}' is not valid utf8: {}",
            artifact_id, err
        )))
    })
}
