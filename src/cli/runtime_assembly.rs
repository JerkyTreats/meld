//! Root runtime assembly for CLI execution.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::api::ContextApi;
use crate::branches::ResolvedBranch;
use crate::config::MerkleConfig;
use crate::config::PhysicalBinding;
use crate::context::head::backfill_legacy_heads_into_ledger;
use crate::error::{ApiError, StorageError};
use crate::events::binding::{
    resolve_product_event_authority, resolve_product_event_authority_at, ProductEventBindingError,
};
use crate::heads::HeadIndex;
use crate::runtime::assembly::{
    ProductRuntimeAssembly, ProductRuntimeConfig, StewardshipComposition,
};
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
        active_branch: Option<&ResolvedBranch>,
        enable_runtime_ids: &[String],
    ) -> Result<Self, ApiError> {
        let stewardship = PhysicalBinding::resolve_for_target(config, workspace_root)?
            .map(|binding| StewardshipComposition { binding });
        let runtime_workspace = stewardship.as_ref().map_or_else(
            || Some(workspace_root.to_path_buf()),
            |selected| selected.binding.workspace_root.clone(),
        );
        let product_root = match &stewardship {
            Some(selected) => selected.binding.storage_root.clone(),
            None => config.system.storage.resolve_product_root(workspace_root)?,
        };
        let (legacy_store_path, frame_storage_path, artifact_storage_path) =
            match &runtime_workspace {
                Some(workspace) => config.system.storage.resolve_paths(workspace)?,
                None => {
                    let root = product_root.join("compatibility");
                    (
                        root.join("store"),
                        root.join("frames"),
                        root.join("artifacts"),
                    )
                }
            };
        let product_layout = ProductStorageLayout::from_root(&product_root);
        let resolved = match active_branch {
            Some(branch) => resolve_product_event_authority(
                branch,
                &product_layout.ledger_db,
                &legacy_store_path,
            ),
            None => {
                let selected = stewardship.as_ref().ok_or_else(|| {
                    ApiError::ConfigError(
                        "a product without a branch requires a stewardship binding".to_string(),
                    )
                })?;
                resolve_product_event_authority_at(
                    &format!("subject:{}", selected.binding.subject.index_key()),
                    &product_root,
                    &product_layout.ledger_db,
                    &legacy_store_path,
                )
            }
        }
        .map_err(binding_error)?;
        // Operator runtime enablement opens the default valve: each named
        // runtime leaves the default-disabled set. Naming a runtime that is
        // not disabled by default is an error rather than a silent no-op.
        let mut runtime_config = ProductRuntimeConfig::for_product_root(product_root.clone());
        // The physical binding validated its provider against the root
        // provider map, so provider-requiring runtimes may compose.
        // Reachability stays a runtime concern: an unreachable endpoint
        // fails invocations retryably, it does not fail the boot.
        if stewardship
            .as_ref()
            .is_some_and(|selected| selected.binding.provider_id.is_some())
        {
            runtime_config.provider.provider_available = true;
        }
        let default_disabled = runtime_config.disabled_runtime_ids.clone();
        for runtime_id in enable_runtime_ids {
            let before = runtime_config.disabled_runtime_ids.len();
            runtime_config
                .disabled_runtime_ids
                .retain(|disabled| disabled != runtime_id);
            if runtime_config.disabled_runtime_ids.len() == before {
                return Err(ApiError::ConfigError(format!(
                    "runtime id '{runtime_id}' is not disabled by default; \
                     --enable-runtime accepts: {}",
                    default_disabled.join(", ")
                )));
            }
        }
        let product_runtime = Arc::new(
            ProductRuntimeAssembly::load_composed(runtime_config, resolved.authority, stewardship)
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

        let head_index = Arc::new(parking_lot::RwLock::new(match &runtime_workspace {
            Some(workspace) => HeadIndex::load_from_disk(HeadIndex::persistence_path(workspace))
                .unwrap_or_else(|error| {
                    tracing::warn!(
                        "Failed to load head index from disk: {}, starting with empty index",
                        error
                    );
                    HeadIndex::new()
                }),
            None => HeadIndex::new(),
        }));
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

        let api = ContextApi::new(
            node_store,
            frame_storage,
            head_index,
            prompt_artifacts,
            Arc::new(parking_lot::RwLock::new(agent_registry)),
            Arc::new(parking_lot::RwLock::new(provider_registry)),
            Arc::new(crate::concurrency::NodeLockManager::new()),
        )
        .with_optional_workspace(runtime_workspace);
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
