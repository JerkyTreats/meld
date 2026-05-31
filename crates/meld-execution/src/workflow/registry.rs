use std::path::PathBuf;

use super::profile::WorkflowProfile;

/// Registered workflow profile contract used by execution runtimes.
#[derive(Debug, Clone)]
pub struct RegisteredWorkflowProfile {
    /// Profile owned by this execution contract.
    pub profile: WorkflowProfile,
    /// Source path owned by this execution contract.
    pub source_path: Option<PathBuf>,
}
