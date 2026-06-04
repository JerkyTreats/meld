use std::path::PathBuf;

use super::profile::WorkflowProfile;

/// Registered workflow profile contract used by execution runtimes.
#[derive(Debug, Clone)]
pub struct RegisteredWorkflowProfile {
    /// Parsed and validated workflow profile.
    pub profile: WorkflowProfile,
    /// Filesystem path used to resolve relative prompt references.
    pub source_path: Option<PathBuf>,
}
