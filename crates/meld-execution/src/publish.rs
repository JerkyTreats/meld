//! Publish templates for frame head write expansion.

use serde::{Deserialize, Serialize};

/// Publish template for writing a selected frame head to a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameHeadPublishTemplate {
    /// Publish file name selected for frame head output.
    pub file_name: String,
    /// Publish strategy selected for frame head output.
    pub strategy: String,
}

/// Expansion payload that writes a generated frame head to publish output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameHeadWriteExpansionContent {
    /// Workspace node identifier carried across execution boundaries.
    pub node_id: String,
    /// Workspace path associated with this execution record.
    pub path: String,
    /// Context frame type produced or consumed by this execution path.
    pub frame_type: String,
    /// Publish file name selected for frame head output.
    pub file_name: String,
}
