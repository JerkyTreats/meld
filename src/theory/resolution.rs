use std::collections::{BTreeMap, BTreeSet};

use super::contracts::{InstalledTheoryComponentRef, TheoryRouteId};
use super::error::{error, TheoryRouterError};
use super::receipt::PdsPackageInstallationReceiptV1;
use super::registry::{PdsPackageStore, TheoryRouteCatalog};

#[derive(Debug, Clone)]
pub struct ResolvedPdsPackage {
    pub receipt: PdsPackageInstallationReceiptV1,
    pub components_by_route: BTreeMap<TheoryRouteId, Vec<InstalledTheoryComponentRef>>,
    /// Exact transitive receipts, including the selected root once.
    pub receipt_closure: Vec<PdsPackageInstallationReceiptV1>,
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
        let receipt_closure = self.store.resolve_closure(&[receipt_id.to_string()])?;
        let receipt = receipt_closure
            .iter()
            .find(|receipt| receipt.receipt_id == receipt_id)
            .expect("resolved closure contains its requested root")
            .clone();
        let mut components_by_route: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for package in &receipt_closure {
            for component in &package.components {
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
                    super::error::TheoryRouterDiagnostic::new(
                        "owner_revision_missing",
                        owner.message,
                    )
                    .package(package.package_id.clone())
                    .component(component.component_id.clone())
                    .route(component.route.clone())
                    .owner(owner.code)
                })?;
                components_by_route
                    .entry(component.route.clone())
                    .or_default()
                    .push(component.clone());
            }
        }
        Ok(ResolvedPdsPackage {
            receipt,
            components_by_route,
            receipt_closure,
        })
    }
}

impl PdsPackageStore {
    /// Resolve exact transitive receipt identity without consulting mutable heads.
    /// A shared import contributes once, while each import edge still checks its
    /// declared package identity. Owner semantic verification stays in the resolver.
    pub fn resolve_closure(
        &self,
        receipt_ids: &[String],
    ) -> Result<Vec<PdsPackageInstallationReceiptV1>, TheoryRouterError> {
        enum Visit {
            Enter(String),
            Leave(PdsPackageInstallationReceiptV1),
        }
        let mut resolved = BTreeMap::<String, PdsPackageInstallationReceiptV1>::new();
        let mut resolving = BTreeSet::new();
        let mut pending = receipt_ids
            .iter()
            .cloned()
            .map(Visit::Enter)
            .collect::<Vec<_>>();
        while let Some(visit) = pending.pop() {
            match visit {
                Visit::Enter(receipt_id) => {
                    if resolved.contains_key(&receipt_id) {
                        continue;
                    }
                    if !resolving.insert(receipt_id.clone()) {
                        return Err(error(
                            "component_requirement_cycle",
                            "exact import receipt cycle",
                        ));
                    }
                    let receipt = self.resolve_receipt(&receipt_id)?.ok_or_else(|| {
                        error("exact_import_missing", "package receipt is absent")
                    })?;
                    let imports = receipt
                        .exact_imports
                        .iter()
                        .map(|import| Visit::Enter(import.receipt_id.clone()))
                        .collect::<Vec<_>>();
                    pending.push(Visit::Leave(receipt));
                    pending.extend(imports);
                }
                Visit::Leave(receipt) => {
                    for import in &receipt.exact_imports {
                        if resolved[&import.receipt_id].package_id != import.package_id {
                            return Err(error(
                                "exact_import_mismatch",
                                "resolved import package identity differs",
                            ));
                        }
                    }
                    resolving.remove(&receipt.receipt_id);
                    resolved.insert(receipt.receipt_id.clone(), receipt);
                }
            }
        }
        let mut receipts = resolved.into_values().collect::<Vec<_>>();
        receipts.sort_by(|left, right| {
            left.package_id
                .cmp(&right.package_id)
                .then(left.receipt_id.cmp(&right.receipt_id))
        });
        Ok(receipts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::InstalledExactPackageImport;

    fn receipt(
        id: &str,
        imports: &[&PdsPackageInstallationReceiptV1],
    ) -> PdsPackageInstallationReceiptV1 {
        PdsPackageInstallationReceiptV1::new(
            id.into(),
            "1".into(),
            format!("content-{id}"),
            1,
            imports
                .iter()
                .map(|package| InstalledExactPackageImport {
                    package_id: package.package_id.clone(),
                    receipt_id: package.receipt_id.clone(),
                })
                .collect(),
            Vec::new(),
            1,
        )
        .unwrap()
    }

    #[test]
    fn exact_closure_shares_diamond_imports_and_survives_reopen() {
        let root = tempfile::tempdir().unwrap();
        let leaf = receipt("leaf", &[]);
        let left = receipt("left", &[&leaf]);
        let right = receipt("right", &[&leaf]);
        let root_receipt = receipt("root", &[&left, &right]);
        let expected = vec![
            leaf.clone(),
            left.clone(),
            right.clone(),
            root_receipt.clone(),
        ];
        {
            let store = PdsPackageStore::new(sled::open(root.path()).unwrap()).unwrap();
            for package in &expected {
                store.install_receipt(package).unwrap();
            }
            assert_eq!(
                store
                    .resolve_closure(&[root_receipt.receipt_id.clone(), left.receipt_id.clone()])
                    .unwrap(),
                expected
            );
            store.flush().unwrap();
        }
        let store = PdsPackageStore::new(sled::open(root.path()).unwrap()).unwrap();
        assert_eq!(
            store.resolve_closure(&[root_receipt.receipt_id]).unwrap(),
            expected
        );
    }

    #[test]
    fn exact_closure_refuses_missing_and_misnamed_imports() {
        let store =
            PdsPackageStore::new(sled::Config::new().temporary(true).open().unwrap()).unwrap();
        let leaf = receipt("leaf", &[]);
        let parent = receipt("parent", &[&leaf]);
        store.install_receipt(&parent).unwrap();
        assert_eq!(
            store
                .resolve_closure(std::slice::from_ref(&parent.receipt_id))
                .unwrap_err()
                .diagnostic_code,
            "exact_import_missing"
        );
        store.install_receipt(&leaf).unwrap();
        let mut misnamed = leaf.clone();
        misnamed.package_id = "foreign".into();
        let parent = receipt("parent", &[&misnamed]);
        store.install_receipt(&parent).unwrap();
        assert_eq!(
            store
                .resolve_closure(&[parent.receipt_id])
                .unwrap_err()
                .diagnostic_code,
            "exact_import_mismatch"
        );
    }
}
