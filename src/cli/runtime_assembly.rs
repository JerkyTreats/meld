//! Root runtime assembly for CLI execution.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::api::ContextApi;
use crate::branches::ResolvedBranch;
use crate::config::MerkleConfig;
use crate::config::PhysicalBinding;
use crate::context::head::backfill_legacy_heads_into_ledger;
use crate::error::{ApiError, StorageError};
use crate::events::binding::{resolve_product_event_authority, ProductEventBindingError};
use crate::heads::HeadIndex;
use crate::runtime::assembly::{
    PlanningTheoryBinding, ProductRuntimeAssembly, ProductRuntimeConfig, StewardshipComposition,
    StewardshipTheoryBindings,
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
        active_branch: &ResolvedBranch,
        enable_runtime_ids: &[String],
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
        // Stage 0 resolution followed by composed machine hydration: when a
        // docs freshness stewardship expression targets this workspace, the
        // product assembly derives its registration set and actor bindings
        // from the validated physical binding, and the selected theory
        // bodies compose from the XDG theory root. A selection that targets
        // a different workspace leaves this invocation on the plain
        // composition — that stewardship expression is not this runtime's.
        let stewardship = match config.stewardship.docs_freshness.as_ref() {
            Some(_) => {
                let binding = PhysicalBinding::resolve(config)?;
                let canonical_workspace = workspace_root
                    .canonicalize()
                    .unwrap_or_else(|_| workspace_root.to_path_buf());
                (binding.workspace_root == canonical_workspace).then(|| {
                    let theory = compose_stewardship_theory(&binding);
                    StewardshipComposition { binding, theory }
                })
            }
            None => None,
        };
        // Operator runtime enablement opens the default valve: each named
        // runtime leaves the default-disabled set. Naming a runtime that is
        // not disabled by default is an error rather than a silent no-op.
        let mut runtime_config = ProductRuntimeConfig::for_product_root(product_root.clone());
        // The physical binding validated its provider against the root
        // provider map, so provider-requiring runtimes may compose.
        // Reachability stays a runtime concern: an unreachable endpoint
        // fails invocations retryably, it does not fail the boot.
        if stewardship.is_some() {
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

/// Compose the selected theory bodies for one stewardship binding.
///
/// Composition is best-effort by design, mirroring the production dispatch
/// route composition: a theory kind that cannot load leaves its dependent
/// actor a truthful unresolved required binding — with the assembly
/// diagnostic naming the gap — instead of failing the invocation. Theory
/// bodies resolve from the XDG theory root only, where `meld world init
/// --theory-source` provisions them.
fn compose_stewardship_theory(binding: &PhysicalBinding) -> StewardshipTheoryBindings {
    let outcome_mapping = loaded(
        "outcome mapping",
        crate::init::world::theory::load_outcome_mapping_config(
            &binding.package.evidence_mapping_id,
        ),
    );
    StewardshipTheoryBindings {
        outcome_mapping,
        strategy: None,
        planning: compose_planning_theory(binding),
        dispatch: None,
    }
}

/// Compose the production planning theory for one stewardship binding.
///
/// Methods, the available-action set, and the method realizations are
/// authored planning theory loaded by expression; the capability catalog is
/// the workflow task-path set — the same catalog the production dispatch
/// routes execute through, so planning resolves operators against exactly
/// the contracts dispatch runs. Requested dimensions derive from the loaded
/// methods' trigger dimensions: the dimensions planning asks the projection
/// for are the ones its theory can act on.
fn compose_planning_theory(binding: &PhysicalBinding) -> Option<PlanningTheoryBinding> {
    use meld_lang::{Proposition, Term};

    let expression = &binding.package.expression;
    let methods = loaded(
        "planning methods",
        crate::init::world::theory::load_planning_methods(expression),
    )?;
    let available_actions = loaded(
        "available actions",
        crate::init::world::theory::load_available_actions(expression),
    )?;
    let method_realizations = loaded(
        "method realizations",
        crate::init::world::theory::load_method_realizations(expression),
    )?;
    let catalog = loaded(
        "planning capability catalog",
        crate::workflow::build_workflow_task_path_runtime(),
    )?;

    let mut requested_dimensions: Vec<String> = Vec::new();
    for method in &methods {
        if let Proposition::Holds {
            dimension: Term::Dimension(dimension),
            ..
        } = &method.trigger
        {
            if !requested_dimensions.contains(dimension) {
                requested_dimensions.push(dimension.clone());
            }
        }
    }
    if requested_dimensions.is_empty() {
        tracing::warn!(
            expression,
            "planning theory composition skipped: no method declares a trigger dimension"
        );
        return None;
    }

    Some(PlanningTheoryBinding {
        methods,
        capability_catalog: catalog.catalog,
        available_actions,
        method_realizations,
        requested_dimensions,
    })
}

/// Unwrap one composed theory load, downgrading failure to a warning.
fn loaded<T, E: std::fmt::Display>(label: &str, result: Result<T, E>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            tracing::warn!(error = %error, "{label} composition skipped");
            None
        }
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
