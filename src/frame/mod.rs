//! Context Frames
//!
//! Immutable containers for context information associated with filesystem nodes.
//! Each frame is content-addressed and append-only.

pub mod set;
pub mod storage;

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
    pub metadata: HashMap<String, String>, // Non-hashed
    pub timestamp: std::time::SystemTime,
}
