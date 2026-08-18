use std::collections::{BTreeMap, BTreeSet};

use super::contracts::{InstalledTheoryComponentRef, TheoryRouteId};
use super::error::{error, TheoryRouterError};
use super::receipt::PdsPackageInstallationReceiptV1;
use super::registry::{PdsPackageStore, TheoryRouteCatalog};

#[derive(Debug, Clone)]
pub struct ResolvedImportedPdsPackage {
    pub package: Box<ResolvedPdsPackage>,
}

#[derive(Debug, Clone)]
pub struct ResolvedPdsPackage {
    pub receipt: PdsPackageInstallationReceiptV1,
    pub components_by_route: BTreeMap<TheoryRouteId, Vec<InstalledTheoryComponentRef>>,
    pub exact_imports: BTreeMap<String, ResolvedImportedPdsPackage>,
}

/// Exact receipt resolver that never consults package or owner heads.
#[derive(Clone)]
pub struct PdsPackageResolver {
    catalog: TheoryRouteCatalog,
    store: PdsPackageStore,
}

impl PdsPackageResolver {
    pub fn new(catalog: TheoryRouteCatalog, store: PdsPackageStore) -> Self {
        Self { catalog, store }
    }

    pub fn resolve(&self, receipt_id: &str) -> Result<ResolvedPdsPackage, TheoryRouterError> {
        self.resolve_inner(receipt_id, &mut BTreeSet::new())
    }

    fn resolve_inner(
        &self,
        receipt_id: &str,
        resolving: &mut BTreeSet<String>,
    ) -> Result<ResolvedPdsPackage, TheoryRouterError> {
        if !resolving.insert(receipt_id.to_string()) {
            return Err(error(
                "component_requirement_cycle",
                "exact import receipt cycle",
            ));
        }
        let receipt = self
            .store
            .resolve_receipt(receipt_id)?
            .ok_or_else(|| error("exact_import_missing", "package receipt is absent"))?;
        let mut components_by_route: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for component in &receipt.components {
            let handler = self.catalog.handler(&component.route).ok_or_else(|| {
                error(
                    "route_unavailable",
                    format!(
                        "historical route '{}' is unavailable",
                        component.route.key()
                    ),
                )
            })?;
            handler.verify(component).map_err(|owner| {
                super::error::TheoryRouterDiagnostic::new("owner_revision_missing", owner.message)
                    .package(receipt.package_id.clone())
                    .component(component.component_id.clone())
                    .route(component.route.clone())
                    .owner(owner.code)
            })?;
            components_by_route
                .entry(component.route.clone())
                .or_default()
                .push(component.clone());
        }
        let mut exact_imports = BTreeMap::new();
        for import in &receipt.exact_imports {
            let resolved = self.resolve_inner(&import.receipt_id, resolving)?;
            if resolved.receipt.package_id != import.package_id {
                return Err(error(
                    "exact_import_mismatch",
                    "resolved import package identity differs",
                ));
            }
            exact_imports.insert(
                import.package_id.clone(),
                ResolvedImportedPdsPackage {
                    package: Box::new(resolved),
                },
            );
        }
        resolving.remove(receipt_id);
        Ok(ResolvedPdsPackage {
            receipt,
            components_by_route,
            exact_imports,
        })
    }
}
