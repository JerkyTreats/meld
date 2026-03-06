//! Workflow command service and adapter contracts.

use crate::api::ContextApi;
use crate::config::WorkflowConfig;
use crate::error::ApiError;
use crate::types::NodeID;
use crate::workflow::executor::{execute_registered_workflow, WorkflowExecutionRequest};
use crate::workflow::profile::WorkflowProfile;
use crate::workflow::registry::WorkflowRegistry;
use crate::workspace;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub struct WorkflowCommandService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowListItem {
    pub workflow_id: String,
    pub version: u32,
    pub title: String,
    pub source_layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowListResult {
    pub workflows: Vec<WorkflowListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowValidateResult {
    pub valid: bool,
    pub workflow_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInspectResult {
    pub workflow_id: String,
    pub source_layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    pub profile: WorkflowProfile,
}

#[derive(Debug, Clone)]
pub struct WorkflowExecuteRequest {
    pub workflow_id: String,
    pub node: Option<String>,
    pub path: Option<PathBuf>,
    pub agent_id: String,
    pub provider_name: String,
    pub frame_type: Option<String>,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecuteResult {
    pub workflow_id: String,
    pub thread_id: String,
    pub turns_completed: usize,
    pub skipped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_frame_id: Option<String>,
}

impl WorkflowCommandService {
    pub fn run_list(registry: &WorkflowRegistry) -> WorkflowListResult {
        let mut workflows: Vec<WorkflowListItem> = registry
            .iter()
            .map(|(workflow_id, registered)| WorkflowListItem {
                workflow_id: workflow_id.clone(),
                version: registered.profile.version,
                title: registered.profile.title.clone(),
                source_layer: format!("{:?}", registered.source_layer).to_lowercase(),
                source_path: registered
                    .source_path
                    .as_ref()
                    .map(|path| path.display().to_string()),
            })
            .collect();
        workflows.sort_by(|left, right| left.workflow_id.cmp(&right.workflow_id));
        WorkflowListResult { workflows }
    }

    pub fn run_validate(
        workspace_root: &Path,
        config: &WorkflowConfig,
    ) -> Result<WorkflowValidateResult, ApiError> {
        let registry = WorkflowRegistry::load(workspace_root, config)?;
        Ok(WorkflowValidateResult {
            valid: true,
            workflow_count: registry.iter().count(),
        })
    }

    pub fn run_inspect(
        registry: &WorkflowRegistry,
        workflow_id: &str,
    ) -> Result<WorkflowInspectResult, ApiError> {
        let registered = registry
            .get(workflow_id)
            .ok_or_else(|| ApiError::ConfigError(format!("Workflow not found: {}", workflow_id)))?;

        Ok(WorkflowInspectResult {
            workflow_id: workflow_id.to_string(),
            source_layer: format!("{:?}", registered.source_layer).to_lowercase(),
            source_path: registered
                .source_path
                .as_ref()
                .map(|path| path.display().to_string()),
            profile: registered.profile.clone(),
        })
    }

    pub fn run_execute(
        api: &ContextApi,
        workspace_root: &Path,
        registry: &WorkflowRegistry,
        request: &WorkflowExecuteRequest,
    ) -> Result<WorkflowExecuteResult, ApiError> {
        let node_id = resolve_node_id(
            api,
            workspace_root,
            request.node.as_deref(),
            request.path.as_deref(),
        )?;

        let agent = api.get_agent(&request.agent_id)?;
        if !agent.can_write() {
            return Err(ApiError::Unauthorized(format!(
                "Agent '{}' cannot execute workflow because role is not writer",
                request.agent_id
            )));
        }
        if let Some(bound_id) = agent.workflow_binding() {
            if bound_id != request.workflow_id {
                return Err(ApiError::ConfigError(format!(
                    "Agent '{}' is bound to workflow '{}' and cannot execute '{}'",
                    request.agent_id, bound_id, request.workflow_id
                )));
            }
        }

        {
            let provider_registry = api.provider_registry().read();
            provider_registry.get_or_error(&request.provider_name)?;
        }

        let registered_profile = registry.get(&request.workflow_id).ok_or_else(|| {
            ApiError::ConfigError(format!("Workflow not found: {}", request.workflow_id))
        })?;

        let frame_type = request
            .frame_type
            .clone()
            .unwrap_or_else(|| format!("context-{}", request.agent_id));
        let summary = execute_registered_workflow(
            api,
            &workspace_root.to_path_buf(),
            registered_profile,
            &WorkflowExecutionRequest {
                node_id,
                agent_id: request.agent_id.clone(),
                provider_name: request.provider_name.clone(),
                frame_type,
                force: request.force,
            },
        )?;

        Ok(WorkflowExecuteResult {
            workflow_id: summary.workflow_id,
            thread_id: summary.thread_id,
            turns_completed: summary.turns_completed,
            skipped: summary.turns_completed == 0,
            final_frame_id: summary.final_frame_id.map(hex::encode),
        })
    }
}

fn parse_node_id(s: &str) -> Result<NodeID, ApiError> {
    let trimmed = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(trimmed)
        .map_err(|err| ApiError::InvalidFrame(format!("Invalid hex: {}", err)))?;
    if bytes.len() != 32 {
        return Err(ApiError::InvalidFrame(format!(
            "NodeID must be 32 bytes, got {}",
            bytes.len()
        )));
    }
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);
    Ok(crate::types::Hash::from(hash))
}

fn resolve_node_id(
    api: &ContextApi,
    workspace_root: &Path,
    node: Option<&str>,
    path: Option<&Path>,
) -> Result<NodeID, ApiError> {
    match (node, path) {
        (Some(node_id), None) => parse_node_id(node_id),
        (None, Some(path)) => workspace::resolve_workspace_node_id(
            api,
            &workspace_root.to_path_buf(),
            Some(path),
            None,
            false,
        ),
        (Some(_), Some(_)) => Err(ApiError::ConfigError(
            "Cannot specify both --node and --path".to_string(),
        )),
        (None, None) => Err(ApiError::ConfigError(
            "Must specify either --node or --path for workflow execute".to_string(),
        )),
    }
}
