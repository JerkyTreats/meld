//! Workflow command service and adapter contracts.

use crate::config::WorkflowConfig;
use crate::error::ApiError;
use crate::workflow::profile::WorkflowProfile;
use crate::workflow::registry::WorkflowRegistry;
use serde::{Deserialize, Serialize};

pub struct WorkflowCommandService;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowListItem {
    pub workflow_id: String,
    pub version: u32,
    pub title: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    pub profile: WorkflowProfile,
}

impl WorkflowCommandService {
    pub fn run_list(registry: &WorkflowRegistry) -> WorkflowListResult {
        let mut workflows: Vec<WorkflowListItem> = registry
            .iter()
            .map(|(workflow_id, registered)| WorkflowListItem {
                workflow_id: workflow_id.clone(),
                version: registered.profile.version,
                title: registered.profile.title.clone(),
                source_path: registered
                    .source_path
                    .as_ref()
                    .map(|path| path.display().to_string()),
            })
            .collect();
        workflows.sort_by(|left, right| left.workflow_id.cmp(&right.workflow_id));
        WorkflowListResult { workflows }
    }

    pub fn run_validate(config: &WorkflowConfig) -> Result<WorkflowValidateResult, ApiError> {
        let registry = WorkflowRegistry::load(config)?;
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
            source_path: registered
                .source_path
                .as_ref()
                .map(|path| path.display().to_string()),
            profile: registered.profile.clone(),
        })
    }
}
