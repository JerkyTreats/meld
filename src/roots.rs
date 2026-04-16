//! Roots domain: compatibility facade over the canonical `branches` domain.

pub use crate::branches::catalog;
pub use crate::branches::contracts;
pub use crate::branches::format;
pub use crate::branches::ledger;
pub use crate::branches::locator;
pub use crate::branches::manifest;
pub use crate::branches::runtime;
pub use crate::branches::tooling;

pub use crate::branches::{
    BranchAttachmentStatus, BranchCatalog, BranchCatalogEntry, BranchHandle,
    BranchInspectionStatus, BranchKind, BranchManifest, BranchMigrationStatus, BranchRuntime,
    ResolvedBranch, ResolvedRoot, RootAttachmentStatus, RootCatalog, RootCatalogEntry,
    RootInspectionStatus, RootManifest, RootMigrationLane, RootMigrationLedgerEntry,
    RootMigrationStatus, RootMigrationStepStatus, RootRuntime, RootStatusRow, RootsStatusOutput,
};
