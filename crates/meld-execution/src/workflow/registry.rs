use std::path::PathBuf;

use super::profile::WorkflowProfile;

#[derive(Debug, Clone)]
pub struct RegisteredWorkflowProfile {
    pub profile: WorkflowProfile,
    pub source_path: Option<PathBuf>,
}
