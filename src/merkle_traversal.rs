pub mod capability;

use crate::api::ContextApi;
use crate::error::ApiError;
use crate::types::NodeID;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TraversalStrategy {
    BottomUp,
    TopDown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderedMerkleNodeBatches {
    batches: Vec<Vec<NodeID>>,
}

impl OrderedMerkleNodeBatches {
    pub fn new(batches: Vec<Vec<NodeID>>) -> Self {
        Self { batches }
    }

    pub fn as_slice(&self) -> &[Vec<NodeID>] {
        &self.batches
    }

    pub fn into_batches(self) -> Vec<Vec<NodeID>> {
        self.batches
    }
}

pub fn traverse(
    api: &ContextApi,
    target_node_id: NodeID,
    strategy: TraversalStrategy,
) -> Result<OrderedMerkleNodeBatches, ApiError> {
    let mut levels: HashMap<usize, Vec<NodeID>> = HashMap::new();
    let mut visited: HashSet<NodeID> = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back((target_node_id, 0usize));

    while let Some((node_id, depth)) = queue.pop_front() {
        if !visited.insert(node_id) {
            continue;
        }
        levels.entry(depth).or_default().push(node_id);
        let record = api
            .node_store()
            .get(&node_id)
            .map_err(ApiError::from)?
            .ok_or(ApiError::NodeNotFound(node_id))?;
        for child in &record.children {
            queue.push_back((*child, depth + 1));
        }
    }

    let mut ordered_depths: Vec<_> = levels.into_iter().collect();
    match strategy {
        TraversalStrategy::BottomUp => ordered_depths.sort_by(|(a, _), (b, _)| b.cmp(a)),
        TraversalStrategy::TopDown => ordered_depths.sort_by(|(a, _), (b, _)| a.cmp(b)),
    }

    Ok(OrderedMerkleNodeBatches::new(
        ordered_depths.into_iter().map(|(_, nodes)| nodes).collect(),
    ))
}
