//! Product storage assembly for the durable runtime.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use meld_execution::goals::PersistentGoalSetStore;
use meld_execution::task::TaskArtifactRepoFactory;
use meld_execution::task_network::store::TaskNetworkStoreFactory;
use meld_world_model::agent::AgentStore;
use meld_world_model::belief::{
    BeliefAuthorityMigrationIdentity, BeliefStore, LegacyBeliefCompatibilityPosture,
};
use meld_world_model::error::StorageError as WorldModelStorageError;
use meld_world_model::planner::PlannerProjectionStore;
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::world_state::store::WorldStateStore;
use thiserror::Error;

use crate::context::frame::FrameStorage;
use crate::events::binding::ProductEventBinding;
use crate::prompt_context::PromptContextArtifactStorage;
use crate::store::SledNodeRecordStore;

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
    /// Shared world model database for graph, belief, agent, planner, and compatibility state.
    pub world_model_db: PathBuf,
    /// Execution goal set database owned by `meld-execution`.
    pub execution_goals_db: PathBuf,
    /// Shared task artifact database opened through execution-owned repo factories.
    pub task_artifacts_db: PathBuf,
    /// Directory containing one task network database per network storage key.
    pub task_networks_root: PathBuf,
    /// Context frame blob root owned by root context storage.
    pub frame_blob_root: PathBuf,
    /// Prompt artifact blob root owned by root prompt context storage.
    pub prompt_artifact_root: PathBuf,
}

/// Opened product stores and execution-owned store factories.
pub struct OpenProductStores {
    /// Workspace node record store.
    pub node_store: Arc<SledNodeRecordStore>,
    /// World model graph reducer state and traversal indexes.
    pub traversal_store: Arc<TraversalStore>,
    /// World model belief configuration, evidence, and revision state.
    pub belief_store: Arc<BeliefStore>,
    /// World model agent state.
    pub agent_store: Arc<AgentStore>,
    /// World model planner request and frame authority.
    pub planner_projection_store: Arc<PlannerProjectionStore>,
    /// Compatibility store for legacy world state claims while migration remains active.
    pub legacy_world_state_store: Arc<WorldStateStore>,
    /// Execution-owned goal set store.
    pub goal_store: Arc<PersistentGoalSetStore>,
    // TODO compat-shim: remove after W3B planning and publication consume
    // TaskNetworkAuthorityHostPort command and query capabilities and the
    // direct-store characterization tests have equivalent authority coverage.
    /// Execution-owned factory retained only for the active W3B migration.
    pub task_networks: TaskNetworkStoreFactory,
    /// Execution-owned factory for task-scoped artifact repositories.
    pub task_artifacts: TaskArtifactRepoFactory,
    /// Context frame blob storage.
    pub frame_storage: Arc<FrameStorage>,
    /// Prompt context artifact blob storage.
    pub prompt_artifacts: Arc<PromptContextArtifactStorage>,
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
    #[error("world model store backpressure: {0}")]
    Backpressure(String),
    #[error("world model store unavailable: {0}")]
    Unavailable(String),
    #[error("storage durability is indeterminate: {0}")]
    DurabilityIndeterminate(String),
    #[error("storage migration conflict: {0}")]
    MigrationConflict(String),
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
            execution_goals_db: root.join("execution").join("goals.sled"),
            task_artifacts_db: root.join("execution").join("task_artifacts.sled"),
            task_networks_root: root.join("execution").join("task_networks"),
            frame_blob_root: root.join("context").join("frames"),
            prompt_artifact_root: root.join("context").join("prompt_artifacts"),
            root,
        }
    }

    /// Create parent directories needed before opening stores.
    pub fn create_dirs(&self) -> Result<(), ProductStorageError> {
        create_dir_all(&self.root)?;
        create_dir_all(self.execution_root())?;
        create_dir_all(&self.task_networks_root)?;
        create_dir_all(self.context_root())?;
        create_dir_all(&self.frame_blob_root)?;
        create_dir_all(&self.prompt_artifact_root)?;
        Ok(())
    }

    fn execution_root(&self) -> &Path {
        self.execution_goals_db
            .parent()
            .expect("execution goals path has a parent")
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
        layout.create_dirs()?;

        let workspace_db = open_db(&layout.workspace_db)?;
        let world_model_db = open_db(&layout.world_model_db)?;
        let execution_goals_db = open_db(&layout.execution_goals_db)?;
        let task_artifacts_db = open_db(&layout.task_artifacts_db)?;

        Ok(Self {
            node_store: Arc::new(SledNodeRecordStore::from_db(workspace_db)),
            traversal_store: Arc::new(
                TraversalStore::new(world_model_db.clone()).map_err(to_world_model)?,
            ),
            belief_store: Arc::new(
                BeliefStore::new(world_model_db.clone()).map_err(to_world_model)?,
            ),
            agent_store: Arc::new(AgentStore::new(world_model_db.clone()).map_err(to_world_model)?),
            planner_projection_store: Arc::new(
                PlannerProjectionStore::new(world_model_db.clone()).map_err(to_world_model)?,
            ),
            legacy_world_state_store: Arc::new(
                WorldStateStore::new(world_model_db).map_err(to_world_model)?,
            ),
            goal_store: Arc::new(
                PersistentGoalSetStore::new(execution_goals_db).map_err(to_execution)?,
            ),
            task_networks: TaskNetworkStoreFactory::new(layout.task_networks_root.clone()),
            task_artifacts: TaskArtifactRepoFactory::new(task_artifacts_db),
            frame_storage: Arc::new(
                FrameStorage::new(&layout.frame_blob_root).map_err(to_context)?,
            ),
            prompt_artifacts: Arc::new(
                PromptContextArtifactStorage::new(&layout.prompt_artifact_root)
                    .map_err(to_context)?,
            ),
        })
    }

    /// Flush stores that are opened for every product runtime.
    ///
    /// Task network stores are opened per network and must be flushed by the
    /// caller before this boundary is used as a checkpoint.
    /// A successful flush means storage accepted pending writes. It does not
    /// prove semantic convergence, release supervisor leases, or repair domain
    /// records after a failed checkpoint.
    pub fn flush_boundary(&self) -> Result<(), ProductStorageError> {
        self.node_store.flush().map_err(to_sled)?;
        self.traversal_store.flush().map_err(to_world_model)?;
        self.belief_store.flush().map_err(to_world_model)?;
        self.agent_store.flush().map_err(to_world_model)?;
        self.planner_projection_store
            .flush()
            .map_err(to_world_model)?;
        self.legacy_world_state_store
            .flush()
            .map_err(to_world_model)?;
        self.goal_store.flush().map_err(to_execution)?;
        self.task_artifacts.flush().map_err(to_execution)?;
        self.frame_storage.flush().map_err(to_context)?;
        self.prompt_artifacts.flush().map_err(to_context)?;
        Ok(())
    }
}

/// Migrate the compatibility belief database before product stores are opened.
///
/// The event binding is the durable root authority product. Its branch,
/// ledger identity, and generation derive every belief migration identity so
/// retries cannot silently select a different source or target authority.
// TODO compat-shim: remove after all supported workspaces have durable product
// belief cutover markers and no CLI release can write legacy belief trees. The
// legacy view parity, source fence, interrupted-stage reopen, and root assembly
// characterization tests must remain green before this seam is deleted.
pub(crate) fn migrate_legacy_belief_authority(
    layout: &ProductStorageLayout,
    legacy_store_path: &Path,
    binding: &ProductEventBinding,
) -> Result<LegacyBeliefCompatibilityPosture, ProductStorageError> {
    layout.create_dirs()?;
    create_dir_all(&layout.world_model_db)?;
    create_dir_all(legacy_store_path)?;

    let product_path = canonical_storage_path(&layout.world_model_db)?;
    let legacy_path = canonical_storage_path(legacy_store_path)?;
    if product_path == legacy_path {
        return Err(ProductStorageError::WorldModel(format!(
            "legacy and product belief databases resolve to the same path {}",
            product_path.display()
        )));
    }

    let legacy = BeliefStore::new(open_db(&legacy_path)?).map_err(to_world_model)?;
    let product = BeliefStore::new(open_db(&product_path)?).map_err(to_world_model)?;
    let posture = product
        .migrate_legacy_authority(&legacy, belief_migration_identity(binding)?)
        .map_err(to_belief_migration)?;

    // Both the legacy fence and product marker must be durable before caller
    // assembly is allowed to construct any product reader or writer.
    legacy.flush().map_err(to_belief_durability)?;
    product.flush().map_err(to_belief_durability)?;
    Ok(posture)
}

fn belief_migration_identity(
    binding: &ProductEventBinding,
) -> Result<BeliefAuthorityMigrationIdentity, ProductStorageError> {
    let authority = binding.ledger_identity.to_string();
    BeliefAuthorityMigrationIdentity::try_new(
        format!(
            "belief-authority-migration:{}:{authority}:{}",
            binding.branch_id, binding.generation
        ),
        format!("legacy-belief-authority:{}", binding.branch_id),
        format!("product-belief-authority:{}:{authority}", binding.branch_id),
        binding.generation,
    )
    .map_err(to_world_model)
}

fn canonical_storage_path(path: &Path) -> Result<PathBuf, ProductStorageError> {
    path.canonicalize()
        .map_err(|error| ProductStorageError::Io(error.to_string()))
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

fn to_world_model(error: WorldModelStorageError) -> ProductStorageError {
    match error {
        WorldModelStorageError::Backpressure(message) => ProductStorageError::Backpressure(message),
        WorldModelStorageError::Unavailable(message) => ProductStorageError::Unavailable(message),
        WorldModelStorageError::DurabilityIndeterminate(message) => {
            ProductStorageError::DurabilityIndeterminate(message)
        }
        WorldModelStorageError::MigrationConflict(message) => {
            ProductStorageError::MigrationConflict(message)
        }
        WorldModelStorageError::IoError(error) => ProductStorageError::Io(error.to_string()),
        error => ProductStorageError::WorldModel(error.to_string()),
    }
}

fn to_belief_migration(error: WorldModelStorageError) -> ProductStorageError {
    match error {
        WorldModelStorageError::Backpressure(message) => ProductStorageError::Backpressure(message),
        WorldModelStorageError::Unavailable(message) => ProductStorageError::Unavailable(message),
        WorldModelStorageError::DurabilityIndeterminate(message) => {
            ProductStorageError::DurabilityIndeterminate(message)
        }
        WorldModelStorageError::IoError(error) => {
            ProductStorageError::DurabilityIndeterminate(error.to_string())
        }
        error => ProductStorageError::MigrationConflict(error.to_string()),
    }
}

fn to_belief_durability(error: WorldModelStorageError) -> ProductStorageError {
    ProductStorageError::DurabilityIndeterminate(error.to_string())
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
    use crate::events::binding::{ProductEventBinding, ProductEventBindingState};
    use meld_events::LedgerIdentity;
    use meld_world_model::belief::BeliefAuthorityMigrationProgress;

    fn binding() -> ProductEventBinding {
        ProductEventBinding {
            schema_version: 1,
            branch_id: "branch-a".to_string(),
            ledger_path: PathBuf::from("/product/ledger.sled"),
            ledger_identity: LedgerIdentity::new(),
            generation: 7,
            state: ProductEventBindingState::Active,
            source: None,
        }
    }

    fn open_belief(path: &Path) -> BeliefStore {
        BeliefStore::new(sled::open(path).unwrap()).unwrap()
    }

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
            layout.execution_goals_db,
            PathBuf::from("/tmp/meld-runtime/execution/goals.sled")
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
    fn legacy_only_belief_state_migrates_with_full_parity() {
        let temp = tempfile::tempdir().unwrap();
        let layout = ProductStorageLayout::from_root(temp.path().join("product"));
        let legacy_path = temp.path().join("legacy.sled");
        let legacy = open_belief(&legacy_path);
        legacy
            .put_config_snapshot("legacy-config", r#"{"enabled":true}"#)
            .unwrap();
        legacy.flush().unwrap();
        drop(legacy);

        let posture = migrate_legacy_belief_authority(&layout, &legacy_path, &binding()).unwrap();

        assert_eq!(
            posture,
            LegacyBeliefCompatibilityPosture::ProductAuthoritative
        );
        let product = open_belief(&layout.world_model_db);
        assert_eq!(
            product
                .get_config_snapshot("legacy-config")
                .unwrap()
                .as_deref(),
            Some(r#"{"enabled":true}"#)
        );
        assert!(matches!(
            product
                .authority_migration_marker()
                .unwrap()
                .unwrap()
                .progress(),
            BeliefAuthorityMigrationProgress::Cutover { .. }
        ));
    }

    #[test]
    fn empty_legacy_belief_source_still_reaches_durable_cutover() {
        let temp = tempfile::tempdir().unwrap();
        let layout = ProductStorageLayout::from_root(temp.path().join("product"));
        let legacy_path = temp.path().join("legacy.sled");

        let posture = migrate_legacy_belief_authority(&layout, &legacy_path, &binding()).unwrap();

        assert_eq!(posture, LegacyBeliefCompatibilityPosture::NoLegacyState);
        let product = open_belief(&layout.world_model_db);
        let marker = product.authority_migration_marker().unwrap().unwrap();
        assert!(matches!(
            marker.progress(),
            BeliefAuthorityMigrationProgress::Cutover { .. }
        ));
    }

    #[test]
    fn repeated_belief_migration_reuses_the_product_binding_identity() {
        let temp = tempfile::tempdir().unwrap();
        let layout = ProductStorageLayout::from_root(temp.path().join("product"));
        let legacy_path = temp.path().join("legacy.sled");
        let binding = binding();

        let first = migrate_legacy_belief_authority(&layout, &legacy_path, &binding).unwrap();
        let second = migrate_legacy_belief_authority(&layout, &legacy_path, &binding).unwrap();

        assert_eq!(first, LegacyBeliefCompatibilityPosture::NoLegacyState);
        assert_eq!(second, first);
        let product = open_belief(&layout.world_model_db);
        let marker = product.authority_migration_marker().unwrap().unwrap();
        assert_eq!(
            marker.migration(),
            &belief_migration_identity(&binding).unwrap()
        );
    }

    #[test]
    fn source_mutation_after_preparation_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let legacy_path = temp.path().join("legacy.sled");
        let product_path = temp.path().join("product.sled");
        let legacy_db = sled::open(&legacy_path).unwrap();
        let legacy = BeliefStore::new(legacy_db.clone()).unwrap();
        legacy.put_config_snapshot("config-a", "one").unwrap();
        let product = open_belief(&product_path);
        let identity = belief_migration_identity(&binding()).unwrap();
        product
            .advance_legacy_authority_migration(&legacy, identity.clone())
            .unwrap();

        legacy_db
            .open_tree("belief_config_snapshots")
            .unwrap()
            .insert(b"config-b", b"two")
            .unwrap();
        legacy_db.flush().unwrap();

        let error = product
            .migrate_legacy_authority(&legacy, identity)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("legacy belief authority mutated"));
    }

    #[test]
    fn divergent_product_record_fails_migration_parity() {
        let temp = tempfile::tempdir().unwrap();
        let layout = ProductStorageLayout::from_root(temp.path().join("product"));
        let legacy_path = temp.path().join("legacy.sled");
        let legacy = open_belief(&legacy_path);
        legacy.put_config_snapshot("config-a", "legacy").unwrap();
        legacy.flush().unwrap();
        drop(legacy);
        create_dir_all(&layout.world_model_db).unwrap();
        let product = open_belief(&layout.world_model_db);
        product.put_config_snapshot("config-a", "product").unwrap();
        product.flush().unwrap();
        drop(product);

        let error = migrate_legacy_belief_authority(&layout, &legacy_path, &binding()).unwrap_err();

        assert!(error.to_string().contains("migration target conflict"));
    }

    #[test]
    fn every_pre_cutover_marker_state_resumes_after_reopen() {
        for advance_count in 1..=4 {
            let temp = tempfile::tempdir().unwrap();
            let layout = ProductStorageLayout::from_root(temp.path().join("product"));
            let legacy_path = temp.path().join("legacy.sled");
            let binding = binding();
            create_dir_all(&layout.world_model_db).unwrap();
            {
                let legacy = open_belief(&legacy_path);
                legacy.put_config_snapshot("config-a", "legacy").unwrap();
                let product = open_belief(&layout.world_model_db);
                let identity = belief_migration_identity(&binding).unwrap();
                for _ in 0..advance_count {
                    product
                        .advance_legacy_authority_migration(&legacy, identity.clone())
                        .unwrap();
                }
                product.flush().unwrap();
                legacy.flush().unwrap();
            }

            let posture = migrate_legacy_belief_authority(&layout, &legacy_path, &binding).unwrap();
            assert_eq!(
                posture,
                LegacyBeliefCompatibilityPosture::ProductAuthoritative
            );
            let reopened = open_belief(&layout.world_model_db);
            assert!(matches!(
                reopened
                    .authority_migration_marker()
                    .unwrap()
                    .unwrap()
                    .progress(),
                BeliefAuthorityMigrationProgress::Cutover { .. }
            ));
        }
    }

    #[test]
    fn forward_repair_only_reopen_never_rolls_back_product_state() {
        let temp = tempfile::tempdir().unwrap();
        let layout = ProductStorageLayout::from_root(temp.path().join("product"));
        let legacy_path = temp.path().join("legacy.sled");
        let binding = binding();
        migrate_legacy_belief_authority(&layout, &legacy_path, &binding).unwrap();
        {
            let product = open_belief(&layout.world_model_db);
            product.put_config_snapshot("product-write", "new").unwrap();
            product.flush().unwrap();
        }

        let first = migrate_legacy_belief_authority(&layout, &legacy_path, &binding).unwrap();
        let reopened = migrate_legacy_belief_authority(&layout, &legacy_path, &binding).unwrap();

        assert_eq!(first, LegacyBeliefCompatibilityPosture::ForwardRepairOnly);
        assert_eq!(reopened, first);
        let product = open_belief(&layout.world_model_db);
        assert_eq!(
            product
                .get_config_snapshot("product-write")
                .unwrap()
                .as_deref(),
            Some("new")
        );
    }

    #[cfg(unix)]
    #[test]
    fn canonical_path_alias_to_product_belief_database_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let layout = ProductStorageLayout::from_root(temp.path().join("product"));
        layout.create_dirs().unwrap();
        create_dir_all(&layout.world_model_db).unwrap();
        let alias = temp.path().join("legacy-alias.sled");
        std::os::unix::fs::symlink(&layout.world_model_db, &alias).unwrap();

        let error = migrate_legacy_belief_authority(&layout, &alias, &binding()).unwrap_err();

        assert!(error
            .to_string()
            .contains("legacy and product belief databases resolve to the same path"));
    }

    #[test]
    fn belief_migration_error_mapping_preserves_retry_and_durability_classes() {
        assert!(matches!(
            to_belief_migration(WorldModelStorageError::Backpressure("retry".to_string())),
            ProductStorageError::Backpressure(message) if message == "retry"
        ));
        assert!(matches!(
            to_belief_migration(WorldModelStorageError::IoError(std::io::Error::other(
                "flush failed"
            ))),
            ProductStorageError::DurabilityIndeterminate(message)
                if message == "flush failed"
        ));
        assert!(matches!(
            to_belief_migration(WorldModelStorageError::InvalidPath(
                "fence conflict".to_string()
            )),
            ProductStorageError::MigrationConflict(message)
                if message.contains("fence conflict")
        ));
    }
}
