//! Root runtime assembly for CLI execution.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::api::ContextApi;
use crate::branches::ResolvedBranch;
use crate::config::MerkleConfig;
use crate::context::head::backfill_legacy_heads_into_ledger;
use crate::error::{ApiError, StorageError};
use crate::events::binding::{resolve_product_event_authority, ProductEventBindingError};
use crate::heads::HeadIndex;
use crate::runtime::assembly::ProductRuntimeAssembly;
use crate::runtime::storage::ProductStorageLayout;
use crate::session::{SessionRuntime, SessionStore};
use crate::store::persistence::SledNodeRecordStore;
use crate::telemetry::ProgressRuntime;
use crate::workflow::WorkflowRegistry;
use crate::world_state::belief::BeliefStore;

#[derive(Clone)]
pub struct CliRuntimeAssembly {
    api: Arc<ContextApi>,
    workflow_registry: Arc<parking_lot::RwLock<WorkflowRegistry>>,
    progress: Arc<ProgressRuntime>,
    product_runtime: Arc<ProductRuntimeAssembly>,
    legacy_store_path: PathBuf,
    frame_storage_path: PathBuf,
    artifact_storage_path: PathBuf,
}

impl CliRuntimeAssembly {
    pub fn load(
        workspace_root: &Path,
        config: &MerkleConfig,
        active_branch: &ResolvedBranch,
    ) -> Result<Self, ApiError> {
        let (legacy_store_path, frame_storage_path, artifact_storage_path) =
            config.system.storage.resolve_paths(workspace_root)?;
        let product_root = config.system.storage.resolve_product_root(workspace_root)?;
        let product_layout = ProductStorageLayout::from_root(&product_root);
        let resolved = resolve_product_event_authority(
            active_branch,
            &product_layout.ledger_db,
            &legacy_store_path,
        )
        .map_err(binding_error)?;
        let product_runtime = Arc::new(
            ProductRuntimeAssembly::load_for_workspace_with_authority(
                workspace_root,
                config,
                resolved.authority,
            )
            .map_err(|error| ApiError::ConfigError(error.to_string()))?,
        );

        let workflow_registry = Arc::new(parking_lot::RwLock::new(WorkflowRegistry::load(
            &config.workflows,
        )?));

        // Existing CLI-owned node, frame, prompt, belief, and session state
        // stays on its characterized compatibility paths. Only canonical
        // events and the graph projection move to the product authority in E5.
        std::fs::create_dir_all(&legacy_store_path)
            .map_err(|error| ApiError::StorageError(StorageError::IoError(error)))?;
        let compatibility_db = sled::open(&legacy_store_path).map_err(|error| {
            ApiError::StorageError(StorageError::IoError(std::io::Error::other(format!(
                "failed to open compatibility CLI storage: {error}"
            ))))
        })?;
        let node_store = Arc::new(SledNodeRecordStore::from_db(compatibility_db.clone()));
        let belief_store = BeliefStore::shared(compatibility_db.clone()).map_err(ApiError::from)?;
        let session_store = SessionStore::shared(compatibility_db).map_err(ApiError::from)?;
        let session_runtime = Arc::new(SessionRuntime::new(session_store));
        std::fs::create_dir_all(&frame_storage_path)
            .map_err(|error| ApiError::StorageError(StorageError::IoError(error)))?;
        std::fs::create_dir_all(&artifact_storage_path)
            .map_err(|error| ApiError::StorageError(StorageError::IoError(error)))?;
        let frame_storage = Arc::new(
            crate::context::frame::open_storage(&frame_storage_path).map_err(ApiError::from)?,
        );
        let prompt_artifacts = Arc::new(
            crate::prompt_context::PromptContextArtifactStorage::new(&artifact_storage_path)
                .map_err(ApiError::from)?,
        );
        let authority = product_runtime.event_authority();
        let progress = Arc::new(ProgressRuntime::from_capabilities(
            authority.append_capability(),
            session_runtime,
        ));
        let graph_runtime = product_runtime.graph_runtime();
        let world_model_queries = Arc::new(crate::world_state::WorldModelQueries::new(Arc::clone(
            &graph_runtime,
        )));

        let head_index_path = HeadIndex::persistence_path(workspace_root);
        let head_index = Arc::new(parking_lot::RwLock::new(
            HeadIndex::load_from_disk(&head_index_path).unwrap_or_else(|error| {
                tracing::warn!(
                    "Failed to load head index from disk: {}, starting with empty index",
                    error
                );
                HeadIndex::new()
            }),
        ));
        {
            let head_index_guard = head_index.read();
            if let Err(error) = backfill_legacy_heads_into_ledger(
                &progress,
                &head_index_guard,
                frame_storage.as_ref(),
                "context_head_backfill",
            ) {
                tracing::warn!(error = %error, "failed to backfill legacy heads into ledger");
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
            prompt_artifacts,
            Arc::new(parking_lot::RwLock::new(agent_registry)),
            Arc::new(parking_lot::RwLock::new(provider_registry)),
            Arc::new(crate::concurrency::NodeLockManager::new()),
            workspace_root.to_path_buf(),
        );
        api.set_world_model_queries(world_model_queries);
        api.set_belief_store(belief_store);
        api.set_workflow_registry(Arc::clone(&workflow_registry));

        Ok(Self {
            api: Arc::new(api),
            workflow_registry,
            progress,
            product_runtime,
            legacy_store_path,
            frame_storage_path,
            artifact_storage_path,
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

    pub fn product_runtime(&self) -> &Arc<ProductRuntimeAssembly> {
        &self.product_runtime
    }

    pub fn legacy_store_path(&self) -> &Path {
        &self.legacy_store_path
    }

    pub fn frame_storage_path(&self) -> &Path {
        &self.frame_storage_path
    }

    pub fn artifact_storage_path(&self) -> &Path {
        &self.artifact_storage_path
    }

    pub fn graph_runtime(&self) -> Arc<crate::world_state::graph::runtime::GraphRuntime> {
        self.product_runtime.graph_runtime()
    }
}

fn binding_error(error: ProductEventBindingError) -> ApiError {
    match error {
        ProductEventBindingError::Io(error) => ApiError::StorageError(StorageError::IoError(error)),
        ProductEventBindingError::Json(error) => ApiError::StorageError(
            StorageError::MigrationConflict(format!("invalid product event binding JSON: {error}")),
        ),
        ProductEventBindingError::Authority(error) => {
            crate::events::tooling::surface_authority_error("product event binding", error)
        }
        ProductEventBindingError::Mismatch(message) => {
            ApiError::StorageError(StorageError::MigrationConflict(message))
        }
        ProductEventBindingError::SourceUnavailable(message) => {
            ApiError::StorageError(StorageError::EventAuthorityUnavailable(message))
        }
    }
}
