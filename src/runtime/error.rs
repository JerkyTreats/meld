//! Typed errors for root runtime assembly.

use thiserror::Error;

use crate::runtime::storage::ProductStorageError;
use crate::runtime::supervisor::SupervisorStoreError;

/// Error returned while building root product runtime infrastructure.
#[derive(Debug, Error)]
pub enum RuntimeAssemblyError {
    /// Product or workspace configuration is invalid or incomplete.
    #[error("runtime config error: {0}")]
    Config(String),
    /// Required process environment input is unavailable.
    #[error("runtime environment error: {0}")]
    Environment(String),
    /// Product storage layout selection failed.
    #[error("runtime storage layout error: {0}")]
    StorageLayout(String),
    /// Product domain storage failed to open or flush.
    #[error("product storage error: {0}")]
    ProductStorage(#[from] ProductStorageError),
    /// Supervisor lifecycle storage failed to open or flush.
    #[error("supervisor store error: {0}")]
    SupervisorStore(#[from] SupervisorStoreError),
    /// A direct handoff port could not be built.
    #[error("runtime port construction error: {0}")]
    PortConstruction(String),
    /// Provider construction failed before supervisor start.
    #[error("provider construction error: {0}")]
    ProviderConstruction(String),
    /// Runtime factory registry validation failed.
    #[error("runtime registry error: {0}")]
    RuntimeRegistry(#[from] RuntimeRegistryError),
    /// Runtime handle construction failed before lifecycle handoff.
    #[error("runtime handle construction error: {0}")]
    RuntimeHandleConstruction(String),
    /// A configured runtime id has no supported factory.
    #[error("unsupported runtime id: {0}")]
    UnsupportedRuntimeId(String),
    /// Supervisor handoff failed after assembly completed.
    #[error("supervisor handoff error: {0}")]
    SupervisorHandoff(String),
}

/// Error returned by direct handoff ports assembled in root `meld`.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RuntimePortError {
    /// The caller supplied an invalid command or request.
    #[error("invalid port request: {0}")]
    InvalidRequest(String),
    /// The owning domain rejected or failed a storage operation.
    #[error("port storage error: {0}")]
    Storage(String),
    /// The owning domain returned a projection error.
    #[error("planner projection error: {0}")]
    PlannerProjection(String),
    /// The event append contract failed.
    #[error("event append error: {0}")]
    EventAppend(String),
    /// The event replay contract failed.
    #[error("event replay error: {0}")]
    EventReplay(String),
}

/// Error returned while validating runtime factory registry metadata.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RuntimeRegistryError {
    /// Runtime id is empty or uses an invalid shape.
    #[error("invalid runtime id: {0}")]
    InvalidRuntimeId(String),
    /// Runtime id appears more than once in the registry.
    #[error("duplicate runtime id: {0}")]
    DuplicateRuntimeId(String),
}
