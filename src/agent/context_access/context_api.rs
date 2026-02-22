//! Context API–backed implementation of AgentAdapter.
//!
//! Depends on context's public API: `ContextApi` for get_node/put_frame and
//! context facade types (Frame, FrameGenerationQueue, Priority). Queue wait
//! policy is defined here; behavior matches the legacy tooling adapter.

use super::contract::{AgentAdapter, GENERATE_FRAME_PRIORITY, GENERATE_FRAME_TIMEOUT};
use crate::api::{ContextApi, ContextView, NodeContext};
use crate::context::frame::Frame;
use crate::context::queue::FrameGenerationQueue;
use crate::error::ApiError;
use crate::types::{FrameID, NodeID};
use async_trait::async_trait;
use std::sync::Arc;

/// Adapter implementation that delegates to ContextApi and optional queue.
pub struct ContextApiAdapter {
    api: Arc<ContextApi>,
    queue: Option<Arc<FrameGenerationQueue>>,
}

impl ContextApiAdapter {
    /// Create an adapter wrapping ContextApi; generation will fail until with_queue is used.
    pub fn new(api: ContextApi) -> Self {
        Self {
            api: Arc::new(api),
            queue: None,
        }
    }

    /// Create from an Arc<ContextApi> without queue.
    pub fn from_arc(api: Arc<ContextApi>) -> Self {
        Self { api, queue: None }
    }

    /// Create with a queue so generate_frame can enqueue and wait.
    pub fn with_queue(api: Arc<ContextApi>, queue: Arc<FrameGenerationQueue>) -> Self {
        Self {
            api,
            queue: Some(queue),
        }
    }

    /// Reference to the underlying ContextApi.
    pub fn api(&self) -> &ContextApi {
        &self.api
    }
}

#[async_trait]
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

    async fn generate_frame(
        &self,
        node_id: NodeID,
        _prompt: String,
        frame_type: String,
        agent_id: String,
        provider_name: String,
    ) -> Result<FrameID, ApiError> {
        let queue = self.queue.as_ref().ok_or_else(|| {
            ApiError::ConfigError(
                "Generation queue not available. All generation requests must go through the queue."
                    .to_string(),
            )
        })?;

        queue
            .enqueue_and_wait(
                node_id,
                agent_id,
                provider_name,
                Some(frame_type),
                GENERATE_FRAME_PRIORITY,
                Some(GENERATE_FRAME_TIMEOUT),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heads::HeadIndex;
    use crate::store::persistence::SledNodeRecordStore;
    use crate::types::Hash;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_test_api() -> (ContextApi, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store_path = temp_dir.path().join("store");
        let node_store = Arc::new(SledNodeRecordStore::new(&store_path).unwrap());
        let frame_storage_path = temp_dir.path().join("frames");
        std::fs::create_dir_all(&frame_storage_path).unwrap();
        let frame_storage = Arc::new(
            crate::context::frame::storage::FrameStorage::new(&frame_storage_path).unwrap(),
        );
        let head_index = Arc::new(parking_lot::RwLock::new(HeadIndex::new()));
        let agent_registry = Arc::new(parking_lot::RwLock::new(crate::agent::AgentRegistry::new()));
        let provider_registry = Arc::new(parking_lot::RwLock::new(
            crate::provider::ProviderRegistry::new(),
        ));
        let lock_manager = Arc::new(crate::concurrency::NodeLockManager::new());

        let api = ContextApi::new(
            node_store,
            frame_storage,
            head_index,
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
