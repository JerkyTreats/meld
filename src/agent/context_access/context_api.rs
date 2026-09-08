//! Context owner adapter for Agent context reads and frame writes.

use super::contract::AgentAdapter;
use crate::api::{ContextApi, ContextView, NodeContext};
use crate::context::frame::Frame;
use crate::error::ApiError;
use crate::types::{FrameID, NodeID};
use std::sync::Arc;

/// Adapter implementation that delegates to ContextApi .
pub struct ContextApiAdapter {
    api: Arc<ContextApi>,
}

impl ContextApiAdapter {
    /// Create an adapter wrapping the Context owner.
    pub fn new(api: ContextApi) -> Self {
        Self { api: Arc::new(api) }
    }

    /// Create from a shared Context owner.
    pub fn from_arc(api: Arc<ContextApi>) -> Self {
        Self { api }
    }

    /// Reference to the underlying ContextApi.
    pub fn api(&self) -> &ContextApi {
        &self.api
    }
}

impl AgentAdapter for ContextApiAdapter {
    fn read_context(&self, node_id: NodeID, view: ContextView) -> Result<NodeContext, ApiError> {
        self.api.get_node(node_id, view)
    }

    fn write_context(
        &self,
        node_id: NodeID,
        frame: Frame,
        agent_id: String,
    ) -> Result<FrameID, ApiError> {
        self.api.put_frame(node_id, frame, agent_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heads::HeadIndex;
    use crate::prompt_context::PromptContextArtifactStorage;
    use crate::store::persistence::SledNodeRecordStore;
    use crate::types::Hash;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_api() -> (ContextApi, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("store");
        let node_store = Arc::new(SledNodeRecordStore::new(&store_path).unwrap());
        let frame_storage_path = temp_dir.path().join("frames");
        let artifact_storage_path = temp_dir.path().join("artifacts");
        std::fs::create_dir_all(&frame_storage_path).unwrap();
        std::fs::create_dir_all(&artifact_storage_path).unwrap();
        let frame_storage = Arc::new(
            crate::context::frame::storage::FrameStorage::new(&frame_storage_path).unwrap(),
        );
        let prompt_context_storage =
            Arc::new(PromptContextArtifactStorage::new(&artifact_storage_path).unwrap());
        let head_index = HeadIndex::new();
        let agent_registry = Arc::new(parking_lot::RwLock::new(crate::agent::AgentRegistry::new()));
        let provider_registry = Arc::new(parking_lot::RwLock::new(
            crate::provider::ProviderRegistry::new(),
        ));
        let lock_manager = Arc::new(crate::concurrency::NodeLockManager::new());

        let api = ContextApi::new(
            node_store,
            frame_storage,
            head_index,
            prompt_context_storage,
            agent_registry,
            provider_registry,
            lock_manager,
        );

        (api, temp_dir)
    }

    #[test]
    fn test_adapter_creation() {
        let (api, _temp_dir) = create_test_api();
        let adapter = ContextApiAdapter::new(api);
        assert!(adapter
            .api()
            .get_node(
                Hash::from([0u8; 32]),
                ContextView {
                    max_frames: 10,
                    ordering: crate::views::OrderingPolicy::Recency,
                    filters: vec![],
                }
            )
            .is_err());
    }
}
