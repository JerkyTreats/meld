//! Agent–context adapter contract.
//!
//! Trait for agent access to the context engine. Uses context facade types
//! where possible; read/write types remain from api and types until context
//! exposes a thin read/write abstraction.

use crate::api::{ContextView, NodeContext};
use crate::context::frame::Frame;
use crate::context::queue::Priority;
use crate::error::ApiError;
use crate::types::{FrameID, NodeID};
use async_trait::async_trait;
use std::time::Duration;

/// Adapter for agents to interact with the context engine.
///
/// Provides read context, write frame, and optional frame generation via
/// context facade contracts only.
#[async_trait]
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

    /// Generate a frame using an LLM provider.
    ///
    /// Async because it may make network requests. All generation goes
    /// through the queue with configurable wait policy.
    async fn generate_frame(
        &self,
        node_id: NodeID,
        prompt: String,
        frame_type: String,
        agent_id: String,
        provider_name: String,
    ) -> Result<FrameID, ApiError>;
}

/// Default queue wait timeout for generate_frame (5 minutes).
pub const GENERATE_FRAME_TIMEOUT: Duration = Duration::from_secs(300);

/// Priority used when enqueueing generate requests from the adapter.
pub const GENERATE_FRAME_PRIORITY: Priority = Priority::Urgent;
