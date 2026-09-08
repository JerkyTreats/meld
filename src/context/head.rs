pub mod store;

use crate::error::ApiError;
use crate::events::DomainObjectRef;
use crate::heads::HeadIndex;
use crate::types::{FrameID, NodeID};

pub trait CurrentFrameHeadRead {
    fn current_frame_head(
        &self,
        node_id: &NodeID,
        frame_type: &str,
    ) -> Result<Option<FrameID>, ApiError>;

    fn current_frame_heads_for_node(&self, node_id: &NodeID) -> Result<Vec<FrameID>, ApiError>;

    fn count_nodes_for_frame_type(&self, frame_type: &str) -> Result<usize, ApiError>;
}

impl CurrentFrameHeadRead for HeadIndex {
    fn current_frame_head(
        &self,
        node_id: &NodeID,
        frame_type: &str,
    ) -> Result<Option<FrameID>, ApiError> {
        self.get_head(node_id, frame_type).map_err(ApiError::from)
    }

    fn current_frame_heads_for_node(&self, node_id: &NodeID) -> Result<Vec<FrameID>, ApiError> {
        Ok(self.active_heads_for_node(node_id))
    }

    fn count_nodes_for_frame_type(&self, frame_type: &str) -> Result<usize, ApiError> {
        Ok(self.count_nodes_for_frame_type(frame_type))
    }
}

pub fn head_ref(node_id: NodeID, frame_type: &str) -> DomainObjectRef {
    DomainObjectRef::new("context", "head", head_object_id(node_id, frame_type))
        .expect("head ref should be valid")
}

pub fn node_ref(node_id: NodeID) -> DomainObjectRef {
    DomainObjectRef::new("workspace_fs", "node", hex::encode(node_id))
        .expect("node ref should be valid")
}

pub fn frame_ref(frame_id: FrameID) -> DomainObjectRef {
    DomainObjectRef::new("context", "frame", hex::encode(frame_id))
        .expect("frame ref should be valid")
}

fn head_object_id(node_id: NodeID, frame_type: &str) -> String {
    format!("{}::{}", hex::encode(node_id), frame_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_head_ref_is_stable_for_node_and_frame_type() {
        let node_id = [7u8; 32];
        let head_ref = head_ref(node_id, "analysis");
        assert_eq!(head_ref.domain_id, "context");
        assert_eq!(head_ref.object_kind, "head");
        assert_eq!(
            head_ref.object_id,
            format!("{}::analysis", hex::encode(node_id))
        );
    }
}
