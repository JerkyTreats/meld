//! Agent–context adapter contract.
//!
//! Trait for agent access to the context engine. Uses context facade types
//! where possible; read/write types remain from api and types until context
//! exposes a thin read/write abstraction.

use crate::api::{ContextView, NodeContext};
use crate::context::frame::Frame;
use crate::error::ApiError;
use crate::types::{FrameID, NodeID};

/// Adapter for agents to interact with the context engine.
///
/// Provides context reads and frame writes through
/// context facade contracts only.
pub trait AgentAdapter: Send + Sync {
    /// Read context for a node using a view policy.
    fn read_context(&self, node_id: NodeID, view: ContextView) -> Result<NodeContext, ApiError>;

    /// Write a context frame to a node.
    fn write_context(
        &self,
        node_id: NodeID,
        frame: Frame,
        agent_id: String,
    ) -> Result<FrameID, ApiError>;
}
