//! Product storage assembly for the durable runtime.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use meld_execution::authority::AuthorityPolicyRegistryStore;
use meld_execution::capability::CapabilityContractRegistryStore;
use meld_execution::task::TaskArtifactRepoFactory;
use meld_execution::task_network::store::TaskNetworkStoreFactory;
use meld_world_model::agent::{
    AgentCurationRuleRegistryStore, AgentMaintainedConditionRegistryStore, AgentStore,
};
use meld_world_model::belief::{
    BeliefFamilyRegistryStore, BeliefStore, OutcomeMappingRegistryStore,
};
use meld_world_model::strategy::StrategyTheoryRegistryStore;
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::world_state::store::WorldStateStore;
use meld_world_model::CurationStore;
use thiserror::Error;

use crate::context::frame::FrameStorage;
use crate::docs::claim_validation::DocsClaimPolicyRegistryStore;
use crate::prompt_context::PromptContextArtifactStorage;
use crate::runtime::theory::TheoryInstallationReceiptStore;
use crate::store::SledNodeRecordStore;
use crate::theory::{PdsPackageStore, PdsProductStore};

/// Product storage root for durable runtime state.
///
/// Root `meld` owns this physical location. Domain crates own the record
/// meaning, validation, replay rules, and durable cursor semantics inside the
/// stores opened below it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductStorageRoot {
    /// Filesystem root that contains every product runtime store.
    pub root: PathBuf,
}

/// Concrete product storage paths for each durable owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductStorageLayout {
    /// Filesystem root that all product storage paths are derived from.
    pub root: PathBuf,
    /// Canonical event ledger database owned by `meld-events`.
    pub ledger_db: PathBuf,
    /// Root workspace node record database.
    pub workspace_db: PathBuf,
    /// Shared world model database for graph, belief, agent, and compatibility state.
    pub world_model_db: PathBuf,
    /// Shared physical database for execution, docs, and root theory records.
    pub theory_db: PathBuf,
    /// Shared task artifact database opened through execution-owned repo factories.
    pub task_artifacts_db: PathBuf,
    /// Directory containing one task network database per network storage key.
    pub task_networks_root: PathBuf,
    /// Context frame blob root owned by root context storage.
    pub frame_blob_root: PathBuf,
    /// Prompt artifact blob root owned by root prompt context storage.
    pub prompt_artifact_root: PathBuf,
}

/// Store groups one composed registration set may require.
///
/// Registration-scoped resource opening (Runtime Initialization stage 1):
/// a scope names the durable store groups the composed registration set
/// requires, and [`OpenProductStores::open_scoped`] opens nothing outside
/// it. The event ledger is not part of this scope because the event
/// authority is resolved and supplied by product binding, never opened
/// here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StoreScope {
    /// Workspace node record database.
    pub workspace: bool,
    /// Shared world model database, including the belief family registry.
    pub world_model: bool,
    /// Shared theory database and its typed owner stores.
    pub theory: bool,
    /// Task execution databases: artifacts, package progress, task networks.
    pub task_execution: bool,
    /// Context frame blob storage.
    pub context_frames: bool,
    /// Prompt artifact blob storage.
    pub prompt_artifacts: bool,
}

impl StoreScope {
    /// Scope covering every product store group.
    pub fn all() -> Self {
        Self {
            workspace: true,
            world_model: true,
            theory: true,
            task_execution: true,
            context_frames: true,
            prompt_artifacts: true,
        }
    }

    /// Scope covering no store group.
    pub fn none() -> Self {
        Self::default()
    }

    /// Union of two scopes.
    pub fn union(self, other: Self) -> Self {
        Self {
            workspace: self.workspace || other.workspace,
            world_model: self.world_model || other.world_model,
            theory: self.theory || other.theory,
            task_execution: self.task_execution || other.task_execution,
            context_frames: self.context_frames || other.context_frames,
            prompt_artifacts: self.prompt_artifacts || other.prompt_artifacts,
        }
    }
}

/// One store handle that may be closed under a scoped composition.
///
/// Full-scope compositions keep the existing field-access ergonomics
/// through `Deref`; scoped-aware composition code must use [`Self::get`]
/// so an out-of-scope store is an explicit `None`, never a panic. A
/// `Deref` on a closed store is a composition contract violation and
/// panics with the owning field's diagnostic label.
pub struct ScopedResource<T> {
    label: &'static str,
    inner: Option<T>,
}

impl<T> ScopedResource<T> {
    pub(crate) fn open(label: &'static str, inner: T) -> Self {
        Self {
            label,
            inner: Some(inner),
        }
    }

    pub(crate) fn closed(label: &'static str) -> Self {
        Self { label, inner: None }
    }

    /// The store when its group is inside the composed scope.
    pub fn opened(&self) -> Option<&T> {
        self.inner.as_ref()
    }

    /// Whether the store group was opened for this composition.
    pub fn is_open(&self) -> bool {
        self.inner.is_some()
    }
}

impl<T> std::ops::Deref for ScopedResource<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.inner.as_ref().unwrap_or_else(|| {
            panic!(
                "store '{}' was not opened for the composed registration scope",
                self.label
            )
        })
    }
}

/// Opened product stores and execution-owned store factories.
///
/// Fields are scoped: a composition built from an explicit registration
/// set opens only the store groups that set requires, and out-of-scope
/// fields stay closed. The default product composition opens everything.
pub struct OpenProductStores {
    /// Workspace node record store.
    pub node_store: ScopedResource<Arc<SledNodeRecordStore>>,
    /// World model graph reducer state and traversal indexes.
    pub traversal_store: ScopedResource<Arc<TraversalStore>>,
    /// Standing Curation operations, acceptances, results, and publication receipts.
    pub curation_store: ScopedResource<Arc<CurationStore>>,
    /// World model belief configuration, evidence, and revision state.
    pub belief_store: ScopedResource<Arc<BeliefStore>>,
    /// World model belief-family theory registry (Runtime Initialization stage 2 home).
    pub belief_family_registry: ScopedResource<Arc<BeliefFamilyRegistryStore>>,
    /// Agent-owned exact curation-rule registry.
    pub curation_rule_registry: ScopedResource<Arc<AgentCurationRuleRegistryStore>>,
    /// Agent-owned exact maintained-condition registry.
    pub maintained_condition_registry: ScopedResource<Arc<AgentMaintainedConditionRegistryStore>>,
    /// Belief-owned exact outcome-mapping registry.
    pub outcome_mapping_registry: ScopedResource<Arc<OutcomeMappingRegistryStore>>,
    /// Strategy-owned exact theory-package registry.
    pub strategy_theory_registry: ScopedResource<Arc<StrategyTheoryRegistryStore>>,
    /// World model agent state.
    pub agent_store: ScopedResource<Arc<AgentStore>>,
    /// Compatibility store for legacy world state claims while migration remains active.
    pub legacy_world_state_store: ScopedResource<Arc<WorldStateStore>>,
    /// Execution-owned factory for per-network task network stores.
    pub task_networks: ScopedResource<TaskNetworkStoreFactory>,
    /// Execution-owned factory for task-scoped artifact repositories.
    pub task_artifacts: ScopedResource<TaskArtifactRepoFactory>,
    /// Shared execution database holding package progress, dispatch claim
    /// artifact repositories, and the aggregate publication outbox. Same
    /// database as `task_artifacts`.
    pub execution_db: ScopedResource<sled::Db>,
    /// Execution-owned exact capability-contract registry.
    pub capability_contract_registry: ScopedResource<Arc<CapabilityContractRegistryStore>>,
    /// Execution-owned exact effective-authority policy registry.
    pub authority_policy_registry: ScopedResource<Arc<AuthorityPolicyRegistryStore>>,
    /// Docs-owned exact claim-policy registry.
    pub claim_policy_registry: ScopedResource<Arc<DocsClaimPolicyRegistryStore>>,
    /// Read-only historical root installation receipts resolved by exact id.
    pub theory_receipts: ScopedResource<Arc<TheoryInstallationReceiptStore>>,
    /// Generic append-only PDS package receipts and selection heads.
    pub pds_packages: ScopedResource<Arc<PdsPackageStore>>,
    /// Canonical PDS product declarations, compilations, and inert closures.
    pub pds_products: ScopedResource<Arc<PdsProductStore>>,
    /// Shared physical theory database for checkpoint flushing only.
    pub theory_db: ScopedResource<sled::Db>,
    /// Context frame blob storage.
    pub frame_storage: ScopedResource<Arc<FrameStorage>>,
    /// Prompt context artifact blob storage.
    pub prompt_artifacts: ScopedResource<Arc<PromptContextArtifactStorage>>,
}

/// Error returned while opening or flushing product storage.
#[derive(Debug, Error)]
pub enum ProductStorageError {
    #[error("storage io error: {0}")]
    Io(String),
    #[error("sled storage error: {0}")]
    Sled(String),
    #[error("world model store error: {0}")]
    WorldModel(String),
    #[error("execution store error: {0}")]
    Execution(String),
    #[error("context storage error: {0}")]
    Context(String),
}

impl ProductStorageRoot {
    /// Create a product storage root wrapper.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Derive the standard product storage layout from this root.
    pub fn layout(&self) -> ProductStorageLayout {
        ProductStorageLayout::from_root(self.root.clone())
    }
}

impl ProductStorageLayout {
    /// Build the standard product storage layout from one root.
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            ledger_db: root.join("ledger.sled"),
            workspace_db: root.join("workspace.sled"),
            world_model_db: root.join("world_model.sled"),
            theory_db: root.join("theory.sled"),
            task_artifacts_db: root.join("execution").join("task_artifacts.sled"),
            task_networks_root: root.join("execution").join("task_networks"),
            frame_blob_root: root.join("context").join("frames"),
            prompt_artifact_root: root.join("context").join("prompt_artifacts"),
            root,
        }
    }

    /// Create parent directories needed before opening every store.
    pub fn create_dirs(&self) -> Result<(), ProductStorageError> {
        self.create_dirs_scoped(&StoreScope::all())
    }

    /// Create only the parent directories the scoped store groups need.
    pub fn create_dirs_scoped(&self, scope: &StoreScope) -> Result<(), ProductStorageError> {
        create_dir_all(&self.root)?;
        if scope.task_execution {
            create_dir_all(self.execution_root())?;
        }
        if scope.task_execution {
            create_dir_all(&self.task_networks_root)?;
        }
        if scope.context_frames || scope.prompt_artifacts {
            create_dir_all(self.context_root())?;
        }
        if scope.context_frames {
            create_dir_all(&self.frame_blob_root)?;
        }
        if scope.prompt_artifacts {
            create_dir_all(&self.prompt_artifact_root)?;
        }
        Ok(())
    }

    fn execution_root(&self) -> &Path {
        self.task_artifacts_db
            .parent()
            .expect("task artifacts path has a parent")
    }

    fn context_root(&self) -> &Path {
        self.frame_blob_root
            .parent()
            .expect("frame blob path has a parent")
    }
}

impl OpenProductStores {
    /// Open all product-level stores and execution-owned factories.
    pub fn open(layout: &ProductStorageLayout) -> Result<Self, ProductStorageError> {
        Self::open_scoped(layout, &StoreScope::all())
    }

    /// Open only the store groups named by the composed registration scope.
    ///
    /// Idempotent by construction: opening an existing world changes
    /// nothing, and store groups outside the scope are neither created on
    /// disk nor opened.
    pub fn open_scoped(
        layout: &ProductStorageLayout,
        scope: &StoreScope,
    ) -> Result<Self, ProductStorageError> {
        layout.create_dirs_scoped(scope)?;

        let node_store = if scope.workspace {
            let workspace_db = open_db(&layout.workspace_db)?;
            ScopedResource::open(
                "node_store",
                Arc::new(SledNodeRecordStore::from_db(workspace_db)),
            )
        } else {
            ScopedResource::closed("node_store")
        };

        let (
            traversal,
            curation,
            belief,
            family_registry,
            curation_registry,
            maintained_condition_registry,
            outcome_registry,
            strategy_registry,
            agent,
            legacy,
        ) = if scope.world_model {
            let world_model_db = open_db(&layout.world_model_db)?;
            (
                ScopedResource::open(
                    "traversal_store",
                    Arc::new(TraversalStore::new(world_model_db.clone()).map_err(to_world_model)?),
                ),
                ScopedResource::open(
                    "curation_store",
                    Arc::new(CurationStore::new(world_model_db.clone()).map_err(to_world_model)?),
                ),
                ScopedResource::open(
                    "belief_store",
                    Arc::new(BeliefStore::new(world_model_db.clone()).map_err(to_world_model)?),
                ),
                ScopedResource::open(
                    "belief_family_registry",
                    Arc::new(
                        BeliefFamilyRegistryStore::new(world_model_db.clone())
                            .map_err(to_world_model)?,
                    ),
                ),
                ScopedResource::open(
                    "curation_rule_registry",
                    Arc::new(
                        AgentCurationRuleRegistryStore::new(world_model_db.clone())
                            .map_err(to_world_model)?,
                    ),
                ),
                ScopedResource::open(
                    "maintained_condition_registry",
                    Arc::new(
                        AgentMaintainedConditionRegistryStore::new(world_model_db.clone())
                            .map_err(to_world_model)?,
                    ),
                ),
                ScopedResource::open(
                    "outcome_mapping_registry",
                    Arc::new(
                        OutcomeMappingRegistryStore::new(world_model_db.clone())
                            .map_err(to_world_model)?,
                    ),
                ),
                ScopedResource::open(
                    "strategy_theory_registry",
                    Arc::new(
                        StrategyTheoryRegistryStore::new(world_model_db.clone())
                            .map_err(to_world_model)?,
                    ),
                ),
                ScopedResource::open(
                    "agent_store",
                    Arc::new(AgentStore::new(world_model_db.clone()).map_err(to_world_model)?),
                ),
                ScopedResource::open(
                    "legacy_world_state_store",
                    Arc::new(WorldStateStore::new(world_model_db).map_err(to_world_model)?),
                ),
            )
        } else {
            (
                ScopedResource::closed("traversal_store"),
                ScopedResource::closed("curation_store"),
                ScopedResource::closed("belief_store"),
                ScopedResource::closed("belief_family_registry"),
                ScopedResource::closed("curation_rule_registry"),
                ScopedResource::closed("maintained_condition_registry"),
                ScopedResource::closed("outcome_mapping_registry"),
                ScopedResource::closed("strategy_theory_registry"),
                ScopedResource::closed("agent_store"),
                ScopedResource::closed("legacy_world_state_store"),
            )
        };

        let (
            capability_contract_registry,
            authority_policy_registry,
            claim_policy_registry,
            theory_receipts,
            pds_packages,
            pds_products,
            theory_db,
        ) = if scope.theory {
            let theory_db = open_db(&layout.theory_db)?;
            (
                ScopedResource::open(
                    "capability_contract_registry",
                    Arc::new(
                        CapabilityContractRegistryStore::new(theory_db.clone())
                            .map_err(to_execution)?,
                    ),
                ),
                ScopedResource::open(
                    "authority_policy_registry",
                    Arc::new(
                        AuthorityPolicyRegistryStore::new(theory_db.clone())
                            .map_err(to_execution)?,
                    ),
                ),
                ScopedResource::open(
                    "claim_policy_registry",
                    Arc::new(
                        DocsClaimPolicyRegistryStore::new(theory_db.clone()).map_err(to_context)?,
                    ),
                ),
                ScopedResource::open(
                    "theory_receipts",
                    Arc::new(
                        TheoryInstallationReceiptStore::new(theory_db.clone())
                            .map_err(to_context)?,
                    ),
                ),
                ScopedResource::open(
                    "pds_packages",
                    Arc::new(PdsPackageStore::new(theory_db.clone()).map_err(to_context)?),
                ),
                ScopedResource::open(
                    "pds_products",
                    Arc::new(PdsProductStore::new(theory_db.clone()).map_err(to_context)?),
                ),
                ScopedResource::open("theory_db", theory_db),
            )
        } else {
            (
                ScopedResource::closed("capability_contract_registry"),
                ScopedResource::closed("authority_policy_registry"),
                ScopedResource::closed("claim_policy_registry"),
                ScopedResource::closed("theory_receipts"),
                ScopedResource::closed("pds_packages"),
                ScopedResource::closed("pds_products"),
                ScopedResource::closed("theory_db"),
            )
        };

        let (task_networks, task_artifacts, execution_db) = if scope.task_execution {
            let task_artifacts_db = open_db(&layout.task_artifacts_db)?;
            (
                ScopedResource::open(
                    "task_networks",
                    TaskNetworkStoreFactory::new(layout.task_networks_root.clone()),
                ),
                ScopedResource::open(
                    "task_artifacts",
                    TaskArtifactRepoFactory::new(task_artifacts_db.clone()),
                ),
                ScopedResource::open("execution_db", task_artifacts_db),
            )
        } else {
            (
                ScopedResource::closed("task_networks"),
                ScopedResource::closed("task_artifacts"),
                ScopedResource::closed("execution_db"),
            )
        };

        let frame_storage = if scope.context_frames {
            ScopedResource::open(
                "frame_storage",
                Arc::new(FrameStorage::new(&layout.frame_blob_root).map_err(to_context)?),
            )
        } else {
            ScopedResource::closed("frame_storage")
        };

        let prompt_artifacts = if scope.prompt_artifacts {
            ScopedResource::open(
                "prompt_artifacts",
                Arc::new(
                    PromptContextArtifactStorage::new(&layout.prompt_artifact_root)
                        .map_err(to_context)?,
                ),
            )
        } else {
            ScopedResource::closed("prompt_artifacts")
        };

        Ok(Self {
            node_store,
            traversal_store: traversal,
            curation_store: curation,
            belief_store: belief,
            belief_family_registry: family_registry,
            curation_rule_registry: curation_registry,
            maintained_condition_registry,
            outcome_mapping_registry: outcome_registry,
            strategy_theory_registry: strategy_registry,
            agent_store: agent,
            legacy_world_state_store: legacy,
            task_networks,
            task_artifacts,
            execution_db,
            capability_contract_registry,
            authority_policy_registry,
            claim_policy_registry,
            theory_receipts,
            pds_packages,
            pds_products,
            theory_db,
            frame_storage,
            prompt_artifacts,
        })
    }

    /// Flush stores that are opened for this composition.
    ///
    /// Task network stores are opened per network and must be flushed by the
    /// caller before this boundary is used as a checkpoint.
    /// A successful flush means storage accepted pending writes. It does not
    /// prove semantic convergence, release supervisor leases, or repair domain
    /// records after a failed checkpoint.
    pub fn flush_boundary(&self) -> Result<(), ProductStorageError> {
        if let Some(store) = self.node_store.opened() {
            store.flush().map_err(to_sled)?;
        }
        if let Some(store) = self.traversal_store.opened() {
            store.flush().map_err(to_world_model)?;
        }
        if let Some(store) = self.curation_store.opened() {
            store.flush().map_err(to_world_model)?;
        }
        if let Some(store) = self.belief_store.opened() {
            store.flush().map_err(to_world_model)?;
        }
        if let Some(store) = self.agent_store.opened() {
            store.flush().map_err(to_world_model)?;
        }
        if let Some(store) = self.legacy_world_state_store.opened() {
            store.flush().map_err(to_world_model)?;
        }
        if let Some(db) = self.theory_db.opened() {
            db.flush().map_err(to_sled)?;
        }
        if let Some(factory) = self.task_artifacts.opened() {
            factory.flush().map_err(to_execution)?;
        }
        if let Some(storage) = self.frame_storage.opened() {
            storage.flush().map_err(to_context)?;
        }
        if let Some(storage) = self.prompt_artifacts.opened() {
            storage.flush().map_err(to_context)?;
        }
        Ok(())
    }
}

fn create_dir_all(path: impl AsRef<Path>) -> Result<(), ProductStorageError> {
    std::fs::create_dir_all(path.as_ref())
        .map_err(|error| ProductStorageError::Io(error.to_string()))
}

fn open_db(path: &Path) -> Result<sled::Db, ProductStorageError> {
    sled::open(path).map_err(|error| ProductStorageError::Sled(error.to_string()))
}

fn to_sled(error: impl ToString) -> ProductStorageError {
    ProductStorageError::Sled(error.to_string())
}

fn to_world_model(error: impl ToString) -> ProductStorageError {
    ProductStorageError::WorldModel(error.to_string())
}

fn to_execution(error: impl ToString) -> ProductStorageError {
    ProductStorageError::Execution(error.to_string())
}

fn to_context(error: impl ToString) -> ProductStorageError {
    ProductStorageError::Context(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_storage_layout_derives_expected_paths() {
        let root = PathBuf::from("/tmp/meld-runtime");

        let layout = ProductStorageLayout::from_root(root.clone());

        assert_eq!(layout.root, root);
        assert_eq!(
            layout.ledger_db,
            PathBuf::from("/tmp/meld-runtime/ledger.sled")
        );
        assert_eq!(
            layout.workspace_db,
            PathBuf::from("/tmp/meld-runtime/workspace.sled")
        );
        assert_eq!(
            layout.world_model_db,
            PathBuf::from("/tmp/meld-runtime/world_model.sled")
        );
        assert_eq!(
            layout.theory_db,
            PathBuf::from("/tmp/meld-runtime/theory.sled")
        );
        assert_eq!(
            layout.task_artifacts_db,
            PathBuf::from("/tmp/meld-runtime/execution/task_artifacts.sled")
        );
        assert_eq!(
            layout.task_networks_root,
            PathBuf::from("/tmp/meld-runtime/execution/task_networks")
        );
        assert_eq!(
            layout.frame_blob_root,
            PathBuf::from("/tmp/meld-runtime/context/frames")
        );
        assert_eq!(
            layout.prompt_artifact_root,
            PathBuf::from("/tmp/meld-runtime/context/prompt_artifacts")
        );
    }

    #[test]
    fn scoped_open_creates_only_scoped_store_groups() {
        let temp = tempfile::tempdir().unwrap();
        let layout = ProductStorageLayout::from_root(temp.path());
        let scope = StoreScope {
            world_model: true,
            ..StoreScope::none()
        };

        let stores = OpenProductStores::open_scoped(&layout, &scope).unwrap();

        assert!(stores.belief_store.is_open());
        assert!(stores.belief_family_registry.is_open());
        assert!(stores.traversal_store.is_open());
        assert!(!stores.node_store.is_open());
        assert!(!stores.task_networks.is_open());
        assert!(!stores.frame_storage.is_open());
        assert!(!stores.prompt_artifacts.is_open());
        assert!(layout.world_model_db.exists());
        assert!(!layout.workspace_db.exists());
        assert!(!layout.theory_db.exists());
        assert!(!layout.task_artifacts_db.exists());
        assert!(!layout.task_networks_root.exists());
        assert!(!layout.frame_blob_root.exists());
        assert!(!layout.prompt_artifact_root.exists());
        stores.flush_boundary().unwrap();
    }

    #[test]
    fn full_scope_open_matches_all_groups() {
        let temp = tempfile::tempdir().unwrap();
        let layout = ProductStorageLayout::from_root(temp.path());

        let stores = OpenProductStores::open(&layout).unwrap();

        assert!(stores.node_store.is_open());
        assert!(stores.execution_db.is_open());
        assert!(stores.legacy_world_state_store.is_open());
        assert!(stores.theory_receipts.is_open());
        stores.flush_boundary().unwrap();
    }

    #[test]
    fn theory_scope_opens_only_the_shared_theory_group() {
        let temp = tempfile::tempdir().unwrap();
        let layout = ProductStorageLayout::from_root(temp.path());
        let scope = StoreScope {
            theory: true,
            ..StoreScope::none()
        };

        let stores = OpenProductStores::open_scoped(&layout, &scope).unwrap();

        assert!(stores.capability_contract_registry.is_open());
        assert!(stores.claim_policy_registry.is_open());
        assert!(stores.theory_receipts.is_open());
        assert!(stores.theory_db.is_open());
        assert!(!stores.belief_family_registry.is_open());
        assert!(!stores.node_store.is_open());
        assert!(layout.theory_db.exists());
        assert!(!layout.world_model_db.exists());
        assert!(!layout.workspace_db.exists());
    }
}
