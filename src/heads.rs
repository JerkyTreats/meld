//! Frame Heads
//!
//! Provides O(1) access to the "latest" frame for a given node and frame type.

use crate::error::StorageError;
use crate::types::{FrameID, NodeID};
use std::collections::HashMap;

/// Head index: (NodeID, frame_type) -> FrameID
pub struct HeadIndex {
    heads: HashMap<(NodeID, String), FrameID>,
}

impl Default for HeadIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadIndex {
    pub fn new() -> Self {
        HeadIndex {
            heads: HashMap::new(),
        }
    }

    pub fn get_head(
        &self,
        node_id: &NodeID,
        frame_type: &str,
    ) -> Result<Option<FrameID>, StorageError> {
        Ok(self.heads.get(&(*node_id, frame_type.to_string())).copied())
    }

    pub fn update_head(
        &mut self,
        node_id: &NodeID,
        frame_type: &str,
        frame_id: &FrameID,
    ) -> Result<(), StorageError> {
        self.heads.insert((*node_id, frame_type.to_string()), *frame_id);
        Ok(())
    }
}
