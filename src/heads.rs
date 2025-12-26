//! Frame Heads
//!
//! Provides O(1) access to the "latest" frame for a given node and frame type.

use crate::error::StorageError;
use crate::types::{FrameID, NodeID};
use std::collections::HashMap;

/// Head index: (NodeID, frame_type) -> FrameID
pub struct HeadIndex {
    pub(crate) heads: HashMap<(NodeID, String), FrameID>,
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

    /// Get all frame IDs for a given node
    ///
    /// Returns all FrameIDs that are heads for the specified node.
    pub fn get_all_heads_for_node(&self, node_id: &NodeID) -> Vec<FrameID> {
        self.heads
            .iter()
            .filter_map(|((nid, _), frame_id)| {
                if *nid == *node_id {
                    Some(*frame_id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get all unique node IDs that have heads
    pub fn get_all_node_ids(&self) -> Vec<NodeID> {
        let mut node_ids = std::collections::HashSet::new();
        for ((node_id, _), _) in &self.heads {
            node_ids.insert(*node_id);
        }
        node_ids.into_iter().collect()
    }
}
