//! Branches domain: canonical branch identity, migration bookkeeping,
//! operator status, and federated branch reads.

pub mod catalog;
pub mod contracts;
pub mod format;
pub mod ledger;
pub mod locator;
pub mod manifest;
pub mod query;
pub mod runtime;
pub mod tooling;

pub use contracts::{
    BranchAttachmentStatus, BranchCatalog, BranchCatalogEntry, BranchHandle,
    BranchInspectionStatus, BranchKind, BranchManifest, BranchMigrationLane,
    BranchMigrationLedgerEntry, BranchMigrationStatus, BranchMigrationStepStatus, BranchStatusRow,
    BranchesStatusOutput, ResolvedBranch,
};
pub use query::{
    BranchGraphStatusOutput, BranchGraphStatusRow, BranchQueryRuntime, BranchQueryScope,
    BranchReadFailure, FederatedGraphWalkResult, FederatedNeighborsOutput, FederatedObjectPresence,
    FederatedReadMetadata, FederatedRelationRecord, FederatedTraversalFact, FederatedWalkOutput,
};
pub use runtime::BranchRuntime;
