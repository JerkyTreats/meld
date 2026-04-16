//! Roots domain: root identity, migration bookkeeping, and operator status.

pub mod catalog;
pub mod contracts;
pub mod format;
pub mod ledger;
pub mod locator;
pub mod manifest;
pub mod runtime;
pub mod tooling;

pub use contracts::{
    ResolvedRoot, RootAttachmentStatus, RootCatalog, RootCatalogEntry, RootInspectionStatus,
    RootManifest, RootMigrationLane, RootMigrationLedgerEntry, RootMigrationStatus,
    RootMigrationStepStatus, RootStatusRow, RootsStatusOutput,
};
pub use runtime::RootRuntime;
