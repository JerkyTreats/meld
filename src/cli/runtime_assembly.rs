//! Root runtime assembly for CLI execution.

use crate::api::ContextApi;
use crate::config::MerkleConfig;
use crate::context::head::backfill_legacy_heads_into_spine;
use crate::error::ApiError;
use crate::heads::HeadIndex;
use crate::store::persistence::SledNodeRecordStore;
use crate::telemetry::ProgressRuntime;
use crate::workflow::registry::WorkflowRegistry;
use crate::world_state::graph::runtime::GraphRuntime;
use crate::world_state::WorldModelQueries;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct CliRuntimeAssembly {
    api: Arc<ContextApi>,
    workflow_registry: Arc<parking_lot::RwLock<WorkflowRegistry>>,
    progress: Arc<ProgressRuntime>,
    graph_runtime: Arc<GraphRuntime>,
}

impl CliRuntimeAssembly {
    pub fn load(workspace_root: &PathBuf, config: &MerkleConfig) -> Result<Self, ApiError> {
        let (store_path, frame_storage_path, artifact_storage_path) =
            config.system.storage.resolve_paths(workspace_root)?;
        let workflow_registry = Arc::new(parking_lot::RwLock::new(WorkflowRegistry::load(
            &config.workflows,
        )?));

        std::fs::create_dir_all(&store_path)
            .map_err(|e| ApiError::StorageError(crate::error::StorageError::IoError(e)))?;

        let db = sled::open(&store_path).map_err(|e| {
            ApiError::StorageError(crate::error::StorageError::IoError(std::io::Error::other(
                format!("Failed to open sled database: {}", e),
            )))
        })?;
        let node_store = Arc::new(SledNodeRecordStore::from_db(db.clone()));
        let progress = Arc::new(ProgressRuntime::new(db.clone()).map_err(ApiError::StorageError)?);
        let graph_runtime = Arc::new(GraphRuntime::new(db).map_err(ApiError::StorageError)?);
        let world_model_queries = Arc::new(WorldModelQueries::new(Arc::clone(&graph_runtime)));

        std::fs::create_dir_all(&frame_storage_path)
            .map_err(|e| ApiError::StorageError(crate::error::StorageError::IoError(e)))?;
        std::fs::create_dir_all(&artifact_storage_path)
            .map_err(|e| ApiError::StorageError(crate::error::StorageError::IoError(e)))?;
        let frame_storage = Arc::new(
            crate::context::frame::open_storage(&frame_storage_path)
                .map_err(ApiError::StorageError)?,
        );
        let prompt_context_storage = Arc::new(
            crate::prompt_context::PromptContextArtifactStorage::new(&artifact_storage_path)
                .map_err(ApiError::StorageError)?,
        );
        let head_index_path = HeadIndex::persistence_path(workspace_root);
        let head_index = Arc::new(parking_lot::RwLock::new(
            HeadIndex::load_from_disk(&head_index_path).unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to load head index from disk: {}, starting with empty index",
                    e
                );
                HeadIndex::new()
            }),
        ));
        {
            let head_index_guard = head_index.read();
            if let Err(err) = backfill_legacy_heads_into_spine(
                &progress,
                &head_index_guard,
                frame_storage.as_ref(),
                "context_head_backfill",
            ) {
                tracing::warn!(error = %err, "failed to backfill legacy heads into spine");
            }
        }

        let mut agent_registry = crate::agent::AgentRegistry::new();
        agent_registry.load_from_config(config)?;
        agent_registry.load_from_xdg()?;

        let mut provider_registry = crate::provider::ProviderRegistry::new();
        provider_registry.load_from_config(config)?;
        provider_registry.load_from_xdg()?;

        {
            let registry = workflow_registry.read();
            for agent in agent_registry.list_all() {
                crate::workflow::binding::validate_agent_binding(agent, &registry)?;
            }
        }

        let api = ContextApi::with_workspace_root(
            node_store,
            frame_storage,
            head_index,
            prompt_context_storage,
            Arc::new(parking_lot::RwLock::new(agent_registry)),
            Arc::new(parking_lot::RwLock::new(provider_registry)),
            Arc::new(crate::concurrency::NodeLockManager::new()),
            workspace_root.clone(),
        );
        api.set_world_model_queries(world_model_queries);
        api.set_workflow_registry(Arc::clone(&workflow_registry));

        Ok(Self {
            api: Arc::new(api),
            workflow_registry,
            progress,
            graph_runtime,
        })
    }

    pub fn api(&self) -> &Arc<ContextApi> {
        &self.api
    }

    pub fn workflow_registry(&self) -> &Arc<parking_lot::RwLock<WorkflowRegistry>> {
        &self.workflow_registry
    }

    pub fn progress(&self) -> &Arc<ProgressRuntime> {
        &self.progress
    }

    pub fn graph_runtime(&self) -> &Arc<GraphRuntime> {
        &self.graph_runtime
    }
}
