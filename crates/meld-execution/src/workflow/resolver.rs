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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::{ContextReadPort, ExecutionNodeContext, PromptArtifactReadPort};
    use crate::workflow::profile::WorkflowTurn;
    use std::fs;
    use tempfile::tempdir;

    #[derive(Default)]
    struct FakeApi;

    impl PromptArtifactReadPort for FakeApi {
        type ArtifactKind = String;
        type ArtifactRef = String;
        type Error = ApiError;

        fn read_prompt_artifact_bytes(&self, artifact_id: &str) -> Result<Vec<u8>, Self::Error> {
            if artifact_id == "bad_utf8" {
                return Ok(vec![0xff]);
            }
            Ok(format!("artifact prompt {artifact_id}").into_bytes())
        }

        fn write_prompt_artifact_utf8(
            &self,
            kind: Self::ArtifactKind,
            value: &str,
        ) -> Result<Self::ArtifactRef, Self::Error> {
            Ok(format!("{kind}:{value}"))
        }
    }

    impl ContextReadPort for FakeApi {
        type AgentIdentity = String;
        type ContextView = ();
        type Error = ApiError;
        type Frame = Vec<u8>;
        type FrameId = u64;
        type NodeContext = String;
        type NodeId = NodeId;
        type NodeRecord = ExecutionNodeRecord<NodeId>;

        fn get_agent(&self, agent_id: &str) -> Result<Self::AgentIdentity, Self::Error> {
            Ok(agent_id.to_string())
        }

        fn get_head(
            &self,
            _node_id: &Self::NodeId,
            _frame_type: &str,
        ) -> Result<Option<Self::FrameId>, Self::Error> {
            Ok(None)
        }

        fn find_frame_head(
            &self,
            node_id: &Self::NodeId,
            frame_type: &str,
            _include_tombstoned: bool,
        ) -> Result<Option<Self::FrameId>, Self::Error> {
            self.get_head(node_id, frame_type)
        }

        fn get_node(
            &self,
            node_id: Self::NodeId,
            _view: Self::ContextView,
        ) -> Result<Self::NodeContext, Self::Error> {
            Ok(format!("node-{}", hex::encode(node_id)))
        }

        fn context_by_type(
            &self,
            node_id: Self::NodeId,
            frame_type: &str,
            _max_frames: usize,
        ) -> Result<Self::NodeContext, Self::Error> {
            Ok(format!("node-{}:{frame_type}", hex::encode(node_id)))
        }

        fn read_frame(
            &self,
            _frame_id: &Self::FrameId,
        ) -> Result<Option<Self::Frame>, Self::Error> {
            Ok(None)
        }

        fn read_node_record(
            &self,
            node_id: &Self::NodeId,
        ) -> Result<Option<Self::NodeRecord>, Self::Error> {
            Ok(Some(ExecutionNodeRecord {
                node_id: *node_id,
                path: "README.md".to_string(),
                node_kind: ExecutionNodeKind::File,
                children: vec![],
                tombstoned: false,
            }))
        }

        fn read_node_record_by_path(
            &self,
            _path: &Path,
            _include_tombstoned: bool,
        ) -> Result<Option<Self::NodeRecord>, Self::Error> {
            Ok(None)
        }

        fn list_node_records(
            &self,
            _include_tombstoned: bool,
        ) -> Result<Vec<Self::NodeRecord>, Self::Error> {
            Ok(vec![])
        }

        fn workspace_root(&self) -> Option<&Path> {
            None
        }

        fn read_execution_frame(
            &self,
            frame_id: &Self::FrameId,
        ) -> Result<Option<ExecutionFrame<Self::FrameId>>, Self::Error> {
            Ok(Some(ExecutionFrame {
                frame_id: *frame_id,
                frame_type: "summary".to_string(),
                agent_id: "agent".to_string(),
                content: b"context".to_vec(),
            }))
        }

        fn read_execution_node_record(
            &self,
            node_id: &Self::NodeId,
        ) -> Result<Option<ExecutionNodeRecord<Self::NodeId>>, Self::Error> {
            self.read_node_record(node_id)
        }

        fn context_frames_by_type(
            &self,
            node_id: Self::NodeId,
            frame_type: &str,
            _max_frames: usize,
        ) -> Result<ExecutionNodeContext<Self::NodeId, Self::FrameId>, Self::Error> {
            Ok(ExecutionNodeContext {
                node_record: ExecutionNodeRecord {
                    node_id,
                    path: "README.md".to_string(),
                    node_kind: ExecutionNodeKind::File,
                    children: vec![],
                    tombstoned: false,
                },
                frames: vec![ExecutionFrame {
                    frame_id: 1,
                    frame_type: frame_type.to_string(),
                    agent_id: "agent".to_string(),
                    content: b"frame context".to_vec(),
                }],
                frame_count: 1,
            })
        }
    }

    fn turn(input_refs: Vec<&str>) -> WorkflowTurn {
        WorkflowTurn {
            turn_id: "turn-1".to_string(),
            seq: 1,
            title: "First".to_string(),
            prompt_ref: "prompt.md".to_string(),
            input_refs: input_refs.into_iter().map(ToString::to_string).collect(),
            output_type: "summary".to_string(),
            gate_id: "gate-1".to_string(),
            retry_limit: 1,
            timeout_ms: 1000,
        }
    }

    #[test]
    fn resolves_prompt_templates_from_artifact_and_profile_relative_path() {
        let api = FakeApi;
        let from_artifact = resolve_prompt_template(&api, None, "artifact:prompt_1").unwrap();
        let dir = tempdir().unwrap();
        let profile_path = dir.path().join("workflow.yaml");
        let prompt_path = dir.path().join("prompt.md");
        fs::write(&prompt_path, "file prompt").unwrap();

        let from_file = resolve_prompt_template(&api, Some(&profile_path), "prompt.md").unwrap();

        assert_eq!(from_artifact, "artifact prompt prompt_1");
        assert_eq!(from_file, "file prompt");
    }

    #[test]
    fn reports_missing_prompt_template_and_invalid_artifact_prompt() {
        let api = FakeApi;

        let missing = resolve_prompt_template(&api, None, "missing.md").unwrap_err();
        let invalid = resolve_prompt_template(&api, None, "artifact:bad_utf8").unwrap_err();

        assert!(missing
            .to_string()
            .contains("Unable to resolve prompt path"));
        assert!(invalid.to_string().contains("not valid utf8"));
    }

    #[test]
    fn resolves_turn_inputs_from_target_context_and_prior_outputs_in_key_order() {
        let api = FakeApi;
        let prior_outputs = HashMap::from([
            ("zeta".to_string(), "last".to_string()),
            ("alpha".to_string(), "first".to_string()),
        ]);

        let resolved = resolve_turn_inputs(
            &api,
            [1; 32],
            "summary",
            &turn(vec!["zeta", "target_context", "alpha"]),
            &prior_outputs,
        )
        .unwrap();

        assert!(resolved.context_payload.starts_with("Input: alpha"));
        assert!(resolved.context_payload.contains("Path: README.md"));
        assert_eq!(resolved.values["zeta"], "last");
    }

    #[test]
    fn reports_missing_prior_output_ref() {
        let api = FakeApi;

        let error = resolve_turn_inputs(
            &api,
            [1; 32],
            "summary",
            &turn(vec!["missing"]),
            &HashMap::new(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("missing required input_ref"));
    }
}
