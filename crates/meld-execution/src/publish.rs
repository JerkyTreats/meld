use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameHeadPublishTemplate {
    pub file_name: String,
    pub strategy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameHeadWriteExpansionContent {
    pub node_id: String,
    pub path: String,
    pub frame_type: String,
    pub file_name: String,
}
