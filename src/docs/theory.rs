//! Routed installation of the canonical docs stewardship package.

use std::path::Path;

use crate::capability::product_capability_inventory;
use crate::init::world::routes::current_product_route_catalog;
use crate::runtime::storage::OpenProductStores;
use crate::theory::{
    PdsPackageInstallationReceiptV1, PdsPackageManifestV1, PdsPackageStore, TheoryRouter,
    TheoryRouterDiagnostic, TheoryRouterError,
};

pub const DOCS_PACKAGE_ID: &str = "meld.docs-freshness";

pub fn install_package(
    stores: &OpenProductStores,
    package_root: &Path,
    installed_at_seq: u64,
) -> Result<PdsPackageInstallationReceiptV1, TheoryRouterError> {
    let manifest_path = package_root.join("pds-package.json");
    let manifest: PdsPackageManifestV1 = serde_json::from_slice(
        &std::fs::read(&manifest_path).map_err(|failure| source_error(failure.to_string()))?,
    )
    .map_err(|failure| source_error(failure.to_string()))?;
    if manifest.package_id != DOCS_PACKAGE_ID {
        return Err(source_error(
            "docs package id differs from canonical identity",
        ));
    }
    let inventory =
        product_capability_inventory().map_err(|failure| source_error(failure.to_string()))?;
    let package = manifest.materialize_with_published(package_root, Some(&inventory))?;
    let catalog = current_product_route_catalog(stores)?;
    let theory_db = stores
        .theory_db
        .opened()
        .expect("docs package installation requires the theory store")
        .clone();
    let package_store = PdsPackageStore::new(theory_db)?;
    let prior = package_store.head(DOCS_PACKAGE_ID)?;
    TheoryRouter::new(catalog, package_store).install(
        &package,
        installed_at_seq,
        true,
        prior.as_ref(),
    )
}

fn source_error(message: impl Into<String>) -> TheoryRouterError {
    TheoryRouterDiagnostic::new("package_source_invalid", message).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SelectedStewardshipPackage;
    use crate::runtime::storage::ProductStorageLayout;
    use crate::runtime::theory::ResolvedStewardshipTheory;
    use crate::theory::{PdsPackageResolver, TheoryRouteId};
    use meld_events::DomainObjectRef;
    use std::path::PathBuf;

    fn package_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/docs_freshness")
    }

    #[test]
    fn routed_docs_package_round_trips_all_exact_owner_revisions() {
        let root = tempfile::tempdir().unwrap();
        let stores =
            OpenProductStores::open(&ProductStorageLayout::from_root(root.path())).unwrap();
        let receipt = install_package(&stores, &package_root(), 1).unwrap();
        assert_eq!(receipt.package_id, DOCS_PACKAGE_ID);
        assert_eq!(receipt.components.len(), 12);

        let catalog = current_product_route_catalog(&stores).unwrap();
        let package_store =
            PdsPackageStore::new(stores.theory_db.opened().unwrap().clone()).unwrap();
        let resolved = PdsPackageResolver::new(catalog, package_store)
            .resolve(&receipt.receipt_id)
            .unwrap();
        assert_eq!(resolved.receipt, receipt);
        assert_eq!(resolved.components_by_route.len(), 8);
        assert_eq!(
            resolved
                .components_by_route
                .get(&TheoryRouteId::new("execution", "capability-contract", 1))
                .unwrap()
                .len(),
            5
        );
    }

    #[test]
    fn exact_reinstall_is_idempotent_and_does_not_advance_the_head() {
        let root = tempfile::tempdir().unwrap();
        let stores =
            OpenProductStores::open(&ProductStorageLayout::from_root(root.path())).unwrap();
        let first = install_package(&stores, &package_root(), 1).unwrap();
        let second = install_package(&stores, &package_root(), 99).unwrap();
        assert_eq!(first.receipt_id, second.receipt_id);
        let store = PdsPackageStore::new(stores.theory_db.opened().unwrap().clone()).unwrap();
        let head = store.head(DOCS_PACKAGE_ID).unwrap().unwrap();
        assert_eq!(head.receipt_id, first.receipt_id);
        assert_eq!(head.head_revision, 1);
    }

    #[test]
    fn routed_receipt_resolves_into_the_current_runtime_owner_view() {
        let root = tempfile::tempdir().unwrap();
        let stores =
            OpenProductStores::open(&ProductStorageLayout::from_root(root.path())).unwrap();
        let receipt = install_package(&stores, &package_root(), 1).unwrap();
        let selection = SelectedStewardshipPackage {
            expression: "docs_freshness".to_string(),
            principal_id: "workspace-owner".to_string(),
            belief_family_id: "docs_freshness".to_string(),
            evidence_mapping_id: "docs_freshness_outcome_interpretation_v1".to_string(),
            curation_rule_id: "docs_freshness".to_string(),
            maintained_condition_id: "docs_freshness".to_string(),
            strategy_theory_id: "docs_freshness".to_string(),
            authority_policy_id: "docs_workspace_local".to_string(),
            claim_policy_id: "docs-claims-strict-v1".to_string(),
        };
        let subject = DomainObjectRef::new("workspace_fs", "node", "docs").unwrap();
        let resolved = ResolvedStewardshipTheory::resolve(&stores, &selection, &subject).unwrap();
        assert_eq!(resolved.package_receipt_id, receipt.receipt_id);
        assert_eq!(resolved.executable_contracts.len(), 5);
        assert_eq!(resolved.receipt.selection, selection);
    }
}
