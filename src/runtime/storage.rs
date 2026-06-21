//! Product storage assembly for the durable runtime.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use meld_events::events::store::EventStore;
use meld_execution::goals::PersistentGoalSetStore;
use meld_execution::task::TaskArtifactRepoFactory;
use meld_execution::task_network::store::TaskNetworkStoreFactory;
use meld_world_model::agent::AgentStore;
use meld_world_model::belief::BeliefStore;
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::world_state::store::WorldStateStore;
use thiserror::Error;

use crate::context::frame::FrameStorage;
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
    /// Shared world model database for graph, belief, agent, and compatibility state.
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
    /// Canonical event ledger for promoted semantic facts.
    pub event_store: Arc<EventStore>,
    /// Workspace node record store.
    pub node_store: Arc<SledNodeRecordStore>,
    /// World model graph reducer state and traversal indexes.
    pub traversal_store: Arc<TraversalStore>,
    /// World model belief configuration, evidence, and revision state.
    pub belief_store: Arc<BeliefStore>,
    /// World model agent state.
    pub agent_store: Arc<AgentStore>,
    /// Compatibility store for legacy world state claims while migration remains active.
    pub legacy_world_state_store: Arc<WorldStateStore>,
    /// Execution-owned goal set store.
    pub goal_store: Arc<PersistentGoalSetStore>,
    /// Execution-owned factory for per-network task network stores.
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
    #[error("event store error: {0}")]
    Events(String),
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

        let ledger_db = open_db(&layout.ledger_db)?;
        let workspace_db = open_db(&layout.workspace_db)?;
        let world_model_db = open_db(&layout.world_model_db)?;
        let execution_goals_db = open_db(&layout.execution_goals_db)?;
        let task_artifacts_db = open_db(&layout.task_artifacts_db)?;

        Ok(Self {
            event_store: Arc::new(EventStore::new(ledger_db).map_err(to_events)?),
            node_store: Arc::new(SledNodeRecordStore::from_db(workspace_db)),
            traversal_store: Arc::new(
                TraversalStore::new(world_model_db.clone()).map_err(to_world_model)?,
            ),
            belief_store: Arc::new(
                BeliefStore::new(world_model_db.clone()).map_err(to_world_model)?,
            ),
            agent_store: Arc::new(AgentStore::new(world_model_db.clone()).map_err(to_world_model)?),
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
        self.event_store.flush().map_err(to_events)?;
        self.node_store.flush().map_err(to_sled)?;
        self.traversal_store.flush().map_err(to_world_model)?;
        self.belief_store.flush().map_err(to_world_model)?;
        self.agent_store.flush().map_err(to_world_model)?;
        self.legacy_world_state_store
            .flush()
            .map_err(to_world_model)?;
        self.goal_store.flush().map_err(to_execution)?;
        self.task_artifacts.flush().map_err(to_execution)?;
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

fn to_events(error: impl ToString) -> ProductStorageError {
    ProductStorageError::Events(error.to_string())
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
}
