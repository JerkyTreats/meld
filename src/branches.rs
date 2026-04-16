//! Branches domain: canonical branch identity, migration bookkeeping,
//! operator status, and federated branch reads.

#[path = "roots/catalog.rs"]
pub mod catalog;
#[path = "roots/contracts.rs"]
pub mod contracts;
#[path = "roots/format.rs"]
pub mod format;
#[path = "roots/ledger.rs"]
pub mod ledger;
#[path = "roots/locator.rs"]
pub mod locator;
#[path = "roots/manifest.rs"]
pub mod manifest;
#[path = "roots/query.rs"]
pub mod query;
#[path = "roots/runtime.rs"]
pub mod runtime;
#[path = "roots/tooling.rs"]
pub mod tooling;

pub use contracts::{
    BranchAttachmentStatus, BranchCatalog, BranchCatalogEntry, BranchHandle,
    BranchInspectionStatus, BranchKind, BranchManifest, BranchMigrationStatus, ResolvedBranch,
    ResolvedRoot, RootAttachmentStatus, RootCatalog, RootCatalogEntry, RootInspectionStatus,
    RootManifest, RootMigrationLane, RootMigrationLedgerEntry, RootMigrationStatus,
    RootMigrationStepStatus, RootStatusRow, RootsStatusOutput,
};
pub use query::{
    BranchGraphStatusOutput, BranchGraphStatusRow, BranchQueryRuntime, BranchQueryScope,
    BranchReadFailure, FederatedNeighborsOutput, FederatedReadMetadata, FederatedWalkOutput,
};
pub use runtime::RootRuntime as BranchRuntime;
pub use runtime::RootRuntime;
