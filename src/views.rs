//! Context Views
//!
//! Selects and orders a bounded set of frames based on policies
//! (recency, type, agent). Ensures deterministic, bounded context retrieval.

use crate::types::FrameID;
use serde::{Deserialize, Serialize};

/// Ordering policy for frame selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderingPolicy {
    Recency,
    Type,
    Agent,
}

/// Frame filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FrameFilter {
    ByType(String),
    ByAgent(String),
}

/// Context view policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewPolicy {
    pub max_frames: usize,
    pub ordering: OrderingPolicy,
    pub filters: Vec<FrameFilter>,
}

/// Get context view for a node
pub fn get_context_view(
    _node_id: &crate::types::NodeID,
    _policy: &ViewPolicy,
) -> Result<Vec<FrameID>, crate::error::StorageError> {
    // TODO: Implement policy-driven frame selection
    Ok(Vec::new())
}
