//! Context Frames
//!
//! Immutable containers for context information associated with filesystem nodes.
//! Each frame is content-addressed and append-only.

pub mod id;
pub mod queue;
pub mod set;
pub mod storage;

pub use queue::{FrameGenerationQueue, GenerationConfig, GenerationRequest, Priority, QueueStats};
pub use set::FrameMerkleSet;
pub use storage::FrameStorage;

use crate::types::{FrameID, NodeID};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Basis for a context frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Basis {
    Node(NodeID),
    Frame(FrameID),
    Both { node: NodeID, frame: FrameID },
}

/// Context frame
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub frame_id: FrameID,
    pub basis: Basis,
    pub content: Vec<u8>, // Blob
    pub frame_type: String, // Frame type identifier
    pub metadata: HashMap<String, String>, // Non-hashed
    pub timestamp: std::time::SystemTime,
}

impl Frame {
    /// Create a new frame with computed FrameID
    ///
    /// The FrameID is computed deterministically from the basis, agent_id, content, and frame_type.
    /// The agent_id is included in both the FrameID computation and the metadata (Phase 2A requirement).
    pub fn new(
        basis: Basis,
        content: Vec<u8>,
        frame_type: String,
        agent_id: String,
        metadata: HashMap<String, String>,
    ) -> Result<Self, crate::error::StorageError> {
        // Ensure agent_id is in metadata (Phase 2A: agent identity preserved in all frames)
        let mut metadata = metadata;
        metadata.insert("agent_id".to_string(), agent_id.clone());

        // Compute FrameID with agent_id included in hash
        let frame_id = id::compute_frame_id(&basis, &content, &frame_type, &agent_id)?;

        Ok(Frame {
            frame_id,
            basis,
            content,
            frame_type,
            metadata,
            timestamp: std::time::SystemTime::now(),
        })
    }
}
