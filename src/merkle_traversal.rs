pub mod capability;
pub mod expansion;

use crate::api::ContextApi;
use crate::error::ApiError;
use crate::store::NodeType;
use crate::types::NodeID;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TraversalStrategy {
    BottomUp,
    TopDown,
    DirectoriesBottomUp,
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
        TraversalStrategy::DirectoriesBottomUp => ordered_depths.sort_by(|(a, _), (b, _)| b.cmp(a)),
    }

    let batches = ordered_depths
        .into_iter()
        .map(|(_, nodes)| nodes)
        .map(|nodes| match strategy {
            TraversalStrategy::DirectoriesBottomUp => nodes
                .into_iter()
                .filter_map(|node_id| match api.node_store().get(&node_id) {
                    Ok(Some(record)) if matches!(record.node_type, NodeType::Directory) => {
                        Some(Ok(node_id))
                    }
                    Ok(Some(_)) => None,
                    Ok(None) => Some(Err(ApiError::NodeNotFound(node_id))),
                    Err(err) => Some(Err(ApiError::from(err))),
                })
                .collect::<Result<Vec<_>, ApiError>>(),
            _ => Ok(nodes),
        })
        .collect::<Result<Vec<_>, ApiError>>()?
        .into_iter()
        .filter(|batch| !batch.is_empty())
        .collect();

    Ok(OrderedMerkleNodeBatches::new(batches))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentIdentity, AgentRegistry, AgentRole};
    use crate::api::ContextApi;
    use crate::concurrency::NodeLockManager;
    use crate::context::frame::storage::FrameStorage;
    use crate::heads::HeadIndex;
    use crate::prompt_context::PromptContextArtifactStorage;
    use crate::provider::ProviderRegistry;
    use crate::store::{NodeRecord, NodeRecordStore, SledNodeRecordStore};
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn test_api() -> (TempDir, ContextApi, NodeID, NodeID, NodeID) {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path().to_path_buf();
        let store_root = temp_dir.path().join("store");
        let frames_root = temp_dir.path().join("frames");
        let artifact_storage_root = temp_dir.path().join("artifacts");

        let node_store = Arc::new(SledNodeRecordStore::new(&store_root).unwrap());
        let frame_storage = Arc::new(FrameStorage::new(&frames_root).unwrap());
        let head_index = Arc::new(parking_lot::RwLock::new(HeadIndex::new()));
        let prompt_context_storage =
            Arc::new(PromptContextArtifactStorage::new(&artifact_storage_root).unwrap());
        let agent_registry = Arc::new(parking_lot::RwLock::new(AgentRegistry::new()));
        let provider_registry = Arc::new(parking_lot::RwLock::new(ProviderRegistry::new()));

        agent_registry
            .write()
            .register(AgentIdentity::new("writer".to_string(), AgentRole::Writer));

        let api = ContextApi::with_workspace_root(
            node_store.clone(),
            frame_storage,
            head_index,
            prompt_context_storage,
            agent_registry,
            provider_registry,
            Arc::new(NodeLockManager::new()),
            workspace_root.clone(),
        );

        let root_id = [1u8; 32];
        let dir_id = [2u8; 32];
        let file_id = [3u8; 32];

        node_store
            .put(&NodeRecord {
                node_id: root_id,
                path: PathBuf::from("."),
                node_type: NodeType::Directory,
                children: vec![dir_id],
                parent: None,
                frame_set_root: None,
                metadata: Default::default(),
                tombstoned_at: None,
            })
            .unwrap();
        node_store
            .put(&NodeRecord {
                node_id: dir_id,
                path: PathBuf::from("src"),
                node_type: NodeType::Directory,
                children: vec![file_id],
                parent: Some(root_id),
                frame_set_root: None,
                metadata: Default::default(),
                tombstoned_at: None,
            })
            .unwrap();
        node_store
            .put(&NodeRecord {
                node_id: file_id,
                path: PathBuf::from("src/lib.rs"),
                node_type: NodeType::File {
                    size: 5,
                    content_hash: [9u8; 32],
                },
                children: vec![],
                parent: Some(dir_id),
                frame_set_root: None,
                metadata: Default::default(),
                tombstoned_at: None,
            })
            .unwrap();

        (temp_dir, api, root_id, dir_id, file_id)
    }

    #[test]
    fn directories_bottom_up_filters_out_files() {
        let (_temp_dir, api, root_id, dir_id, _file_id) = test_api();

        let batches = traverse(&api, root_id, TraversalStrategy::DirectoriesBottomUp).unwrap();

        assert_eq!(batches.as_slice(), &[vec![dir_id], vec![root_id]]);
    }
}
