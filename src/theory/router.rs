use std::collections::{BTreeMap, BTreeSet};

use super::contracts::{
    InstalledPackageLinkView, InstalledTheoryComponentRef, OwnerValidationToken, PackageLinkView,
    RouteCardinality, RoutedComponentSource, TheoryRouteHandler,
};
use super::error::{error, TheoryRouterDiagnostic, TheoryRouterError};
use super::package::{MaterializedPdsPackage, PackageRequirementTarget};
use super::receipt::{InstalledExactPackageImport, PdsPackageInstallationReceiptV1};
use super::registry::{PdsPackageHeadV1, PdsPackageStore, TheoryRouteCatalog};

struct ValidatedComponent {
    source: RoutedComponentSource,
    handler: std::sync::Arc<dyn TheoryRouteHandler>,
    token: OwnerValidationToken,
}

/// Structural package installer. Construction and installation are inert.
#[derive(Clone)]
pub struct TheoryRouter {
    catalog: TheoryRouteCatalog,
    store: PdsPackageStore,
}

impl TheoryRouter {
    pub fn new(catalog: TheoryRouteCatalog, store: PdsPackageStore) -> Self {
        Self { catalog, store }
    }

    pub fn catalog(&self) -> &TheoryRouteCatalog {
        &self.catalog
    }

    pub fn store(&self) -> &PdsPackageStore {
        &self.store
    }

    pub fn install(
        &self,
        package: &MaterializedPdsPackage,
        installed_at_seq: u64,
        advance_head: bool,
        expected_prior: Option<&PdsPackageHeadV1>,
    ) -> Result<PdsPackageInstallationReceiptV1, TheoryRouterError> {
        let imports = self.resolve_imports(package)?;
        let link_view = self.package_link_view(package, &imports);
        self.validate_cardinality_and_requirements(package, &imports)?;

        let mut validated = Vec::new();
        for source in &package.components {
            let handler = self
                .catalog
                .handler(&source.route)
                .cloned()
                .ok_or_else(|| {
                    error(
                        "route_unavailable",
                        format!("route '{}' is not published", source.route.key()),
                    )
                })?;
            let contract = handler.contract();
            if !contract
                .accepted_component_schema
                .contains(source.component_schema_version)
            {
                return Err(error(
                    "component_schema_unsupported",
                    format!(
                        "route '{}' does not accept component schema {}",
                        source.route.key(),
                        source.component_schema_version
                    ),
                ));
            }
            let token = handler
                .validate(source, &link_view)
                .map_err(|owner| owner_error("owner_validation_failed", package, source, owner))?;
            if !token.matches(&contract, &link_view, source) {
                return Err(error(
                    "owner_validation_failed",
                    "owner returned a validation token for another package or component",
                ));
            }
            validated.push(ValidatedComponent {
                source: source.clone(),
                handler,
                token,
            });
        }

        validated.sort_by(|left, right| {
            left.source
                .route
                .cmp(&right.source.route)
                .then(left.source.component_id.cmp(&right.source.component_id))
        });
        let mut installed = Vec::new();
        for item in validated {
            let reference = item
                .handler
                .install(&item.source, item.token, installed_at_seq)
                .map_err(|owner| {
                    owner_error("owner_install_failed", package, &item.source, owner)
                })?;
            validate_owner_reference(&item.source, &reference)?;
            item.handler.verify(&reference).map_err(|owner| {
                owner_error("owner_revision_missing", package, &item.source, owner)
            })?;
            installed.push(reference);
        }

        let installed_view = InstalledPackageLinkView {
            package_id: package.manifest.package_id.clone(),
            package_content_hash: package.package_content_hash.clone(),
            exact_import_receipts: imports
                .iter()
                .map(|item| (item.package_id.clone(), item.receipt_id.clone()))
                .collect(),
            components: installed
                .iter()
                .map(|item| (item.component_id.clone(), item.clone()))
                .collect(),
        };
        for reference in &installed {
            let handler = self.catalog.handler(&reference.route).ok_or_else(|| {
                error(
                    "route_unavailable",
                    "route disappeared from immutable catalog",
                )
            })?;
            handler
                .validate_links(reference, &installed_view)
                .map_err(|owner| {
                    TheoryRouterError::from(
                        TheoryRouterDiagnostic::new("owner_link_invalid", owner.message)
                            .package(package.manifest.package_id.clone())
                            .component(reference.component_id.clone())
                            .route(reference.route.clone())
                            .owner(owner.code),
                    )
                })?;
        }

        let receipt = PdsPackageInstallationReceiptV1::new(
            package.manifest.package_id.clone(),
            package.manifest.package_version.clone(),
            package.package_content_hash.clone(),
            package.manifest.schema_version,
            imports,
            installed,
            installed_at_seq,
        )?;
        self.store.install_receipt(&receipt)?;
        if advance_head {
            let current = self.store.head(&receipt.package_id)?;
            if current.as_ref().map(|head| head.receipt_id.as_str())
                != Some(receipt.receipt_id.as_str())
            {
                if current.as_ref() != expected_prior {
                    return Err(error(
                        "package_head_conflict",
                        "package head differs from expected prior",
                    ));
                }
                self.store.advance_head(&receipt, expected_prior)?;
            }
        }
        Ok(receipt)
    }

    fn resolve_imports(
        &self,
        package: &MaterializedPdsPackage,
    ) -> Result<Vec<InstalledExactPackageImport>, TheoryRouterError> {
        let mut imports = Vec::new();
        for import in &package.manifest.imports {
            let receipt = self
                .store
                .resolve_receipt(&import.receipt_id)?
                .ok_or_else(|| error("exact_import_missing", "exact import receipt is absent"))?;
            if receipt.package_id != import.package_id || receipt.receipt_id != import.receipt_id {
                return Err(error(
                    "exact_import_mismatch",
                    "exact import package or receipt identity mismatch",
                ));
            }
            imports.push(InstalledExactPackageImport {
                package_id: import.package_id.clone(),
                receipt_id: import.receipt_id.clone(),
            });
        }
        Ok(imports)
    }

    fn package_link_view(
        &self,
        package: &MaterializedPdsPackage,
        imports: &[InstalledExactPackageImport],
    ) -> PackageLinkView {
        PackageLinkView {
            package_id: package.manifest.package_id.clone(),
            package_content_hash: package.package_content_hash.clone(),
            exact_import_receipts: imports
                .iter()
                .map(|item| (item.package_id.clone(), item.receipt_id.clone()))
                .collect(),
            component_routes: package
                .components
                .iter()
                .map(|item| (item.component_id.clone(), item.route.clone()))
                .collect(),
        }
    }

    fn validate_cardinality_and_requirements(
        &self,
        package: &MaterializedPdsPackage,
        imports: &[InstalledExactPackageImport],
    ) -> Result<(), TheoryRouterError> {
        let mut counts = BTreeMap::new();
        for component in &package.components {
            let Some(handler) = self.catalog.handler(&component.route) else {
                return Err(error(
                    "route_unavailable",
                    format!("route '{}' is not published", component.route.key()),
                ));
            };
            *counts.entry(component.route.clone()).or_insert(0usize) += 1;
            if handler.contract().package_cardinality == RouteCardinality::AtMostOne
                && counts[&component.route] > 1
            {
                return Err(error(
                    "route_cardinality_invalid",
                    format!(
                        "route '{}' permits at most one component",
                        component.route.key()
                    ),
                ));
            }
        }
        let import_keys: BTreeSet<_> = imports
            .iter()
            .map(|item| (&item.package_id, &item.receipt_id))
            .collect();
        for entry in &package.manifest.components {
            for requirement in &entry.requires {
                if let PackageRequirementTarget::ExactImport {
                    package_id,
                    receipt_id,
                } = &requirement.package
                {
                    if !import_keys.contains(&(package_id, receipt_id)) {
                        return Err(error(
                            "component_requirement_missing",
                            "component requirement does not name a resolved exact import",
                        ));
                    }
                    let receipt = self
                        .store
                        .resolve_receipt(receipt_id)?
                        .ok_or_else(|| error("exact_import_missing", "import disappeared"))?;
                    let Some(component) = receipt
                        .components
                        .iter()
                        .find(|item| item.component_id == requirement.component_id)
                    else {
                        return Err(error(
                            "component_requirement_missing",
                            "required imported component is absent",
                        ));
                    };
                    if requirement
                        .required_route
                        .as_ref()
                        .is_some_and(|route| route != &component.route)
                    {
                        return Err(error(
                            "component_requirement_missing",
                            "required imported component route differs",
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

fn validate_owner_reference(
    source: &RoutedComponentSource,
    reference: &InstalledTheoryComponentRef,
) -> Result<(), TheoryRouterError> {
    if reference.component_id != source.component_id
        || reference.route != source.route
        || reference.component_schema_version != source.component_schema_version
        || reference.source_content_hash != source.source_content_hash
        || reference.owner_revision.registry.is_empty()
        || reference.owner_revision.id.is_empty()
        || reference.owner_revision.content_hash.is_empty()
    {
        return Err(error(
            "owner_install_failed",
            "owner returned a revision for another component or incomplete identity",
        ));
    }
    Ok(())
}

fn owner_error(
    code: &str,
    package: &MaterializedPdsPackage,
    source: &RoutedComponentSource,
    owner: super::contracts::OwnerRouteDiagnostic,
) -> TheoryRouterError {
    TheoryRouterDiagnostic::new(code, owner.message)
        .package(package.manifest.package_id.clone())
        .component(source.component_id.clone())
        .route(source.route.clone())
        .owner(owner.code)
        .into()
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::theory::{
        ComponentContentRef, PdsComponentEntry, PdsPackageManifestV1, PdsPackageResolver,
        RouteCardinality, TheoryRevisionRef, TheoryRouteContract, TheoryRouteId, VersionRange,
    };

    #[derive(Clone)]
    struct FakeHandler {
        contract: TheoryRouteContract,
        revisions: Arc<Mutex<BTreeMap<String, InstalledTheoryComponentRef>>>,
        install_writes: Arc<AtomicUsize>,
        reject_validation: Arc<AtomicBool>,
        reject_install: Arc<AtomicBool>,
        reject_links: Arc<AtomicBool>,
    }

    impl FakeHandler {
        fn new(owner: &str, kind: &str) -> Self {
            Self {
                contract: TheoryRouteContract {
                    route: TheoryRouteId::new(owner, kind, 1),
                    accepted_component_schema: VersionRange { min: 1, max: 1 },
                    package_cardinality: RouteCardinality::Many,
                    handler_contract_version: 1,
                },
                revisions: Arc::new(Mutex::new(BTreeMap::new())),
                install_writes: Arc::new(AtomicUsize::new(0)),
                reject_validation: Arc::new(AtomicBool::new(false)),
                reject_install: Arc::new(AtomicBool::new(false)),
                reject_links: Arc::new(AtomicBool::new(false)),
            }
        }

        fn remove(&self, component_id: &str) {
            self.revisions.lock().unwrap().remove(component_id);
        }
    }

    impl TheoryRouteHandler for FakeHandler {
        fn contract(&self) -> TheoryRouteContract {
            self.contract.clone()
        }

        fn validate(
            &self,
            source: &RoutedComponentSource,
            package: &PackageLinkView,
        ) -> Result<OwnerValidationToken, super::super::OwnerRouteDiagnostic> {
            if self.reject_validation.load(Ordering::SeqCst) {
                return Err(super::super::OwnerRouteDiagnostic::new(
                    "fake_invalid",
                    "rejected by fake owner",
                ));
            }
            Ok(OwnerValidationToken::new(
                &self.contract,
                package,
                source,
                source.canonical_bytes.to_vec(),
            ))
        }

        fn install(
            &self,
            source: &RoutedComponentSource,
            validation: OwnerValidationToken,
            _installed_at_seq: u64,
        ) -> Result<InstalledTheoryComponentRef, super::super::OwnerRouteDiagnostic> {
            if self.reject_install.load(Ordering::SeqCst) {
                return Err(super::super::OwnerRouteDiagnostic::new(
                    "fake_install_failed",
                    "rejected by fake owner",
                ));
            }
            let mut revisions = self.revisions.lock().unwrap();
            if let Some(existing) = revisions.get(&source.component_id) {
                return Ok(existing.clone());
            }
            let reference = InstalledTheoryComponentRef {
                component_id: source.component_id.clone(),
                route: source.route.clone(),
                component_schema_version: source.component_schema_version,
                source_content_hash: source.source_content_hash.clone(),
                owner_revision: TheoryRevisionRef {
                    registry: format!("{}_registry", source.route.owner_domain),
                    id: source.component_id.clone(),
                    content_hash: blake3::hash(validation.opaque()).to_hex().to_string(),
                },
            };
            revisions.insert(source.component_id.clone(), reference.clone());
            self.install_writes.fetch_add(1, Ordering::SeqCst);
            Ok(reference)
        }

        fn verify(
            &self,
            reference: &InstalledTheoryComponentRef,
        ) -> Result<(), super::super::OwnerRouteDiagnostic> {
            if self.revisions.lock().unwrap().get(&reference.component_id) == Some(reference) {
                Ok(())
            } else {
                Err(super::super::OwnerRouteDiagnostic::new(
                    "fake_missing",
                    "fake owner revision is absent",
                ))
            }
        }

        fn validate_links(
            &self,
            _reference: &InstalledTheoryComponentRef,
            _package: &InstalledPackageLinkView,
        ) -> Result<(), super::super::OwnerRouteDiagnostic> {
            if self.reject_links.load(Ordering::SeqCst) {
                Err(super::super::OwnerRouteDiagnostic::new(
                    "fake_link_invalid",
                    "fake semantic link is invalid",
                ))
            } else {
                Ok(())
            }
        }
    }

    fn package(handlers: &[FakeHandler]) -> MaterializedPdsPackage {
        PdsPackageManifestV1 {
            schema_version: 1,
            package_id: "fake.package".to_string(),
            package_version: "1.0.0".to_string(),
            description: None,
            imports: vec![],
            components: handlers
                .iter()
                .enumerate()
                .map(|(index, handler)| PdsComponentEntry {
                    component_id: format!("component-{index}"),
                    owner_component_id: format!("owner-component-{index}"),
                    route: handler.contract.route.clone(),
                    component_schema_version: 1,
                    content: ComponentContentRef::Embedded {
                        canonical_bytes: format!("body-{index}").into_bytes(),
                    },
                    requires: vec![],
                })
                .collect(),
        }
        .materialize(Path::new("."))
        .unwrap()
    }

    fn setup(handlers: &[FakeHandler]) -> (TheoryRouter, PdsPackageStore) {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = PdsPackageStore::new(db).unwrap();
        let publications = handlers
            .iter()
            .cloned()
            .map(|handler| Arc::new(handler) as Arc<dyn TheoryRouteHandler>)
            .collect();
        let catalog = TheoryRouteCatalog::build(publications).unwrap();
        (TheoryRouter::new(catalog, store.clone()), store)
    }

    #[test]
    fn all_validation_precedes_owner_install() {
        let handlers = vec![
            FakeHandler::new("alpha", "body"),
            FakeHandler::new("beta", "body"),
        ];
        handlers[1].reject_validation.store(true, Ordering::SeqCst);
        let package = package(&handlers);
        let (router, _) = setup(&handlers);

        assert_eq!(
            router
                .install(&package, 1, false, None)
                .unwrap_err()
                .diagnostic_code,
            "owner_validation_failed"
        );
        assert_eq!(
            handlers
                .iter()
                .map(|handler| handler.install_writes.load(Ordering::SeqCst))
                .sum::<usize>(),
            0
        );
    }

    #[test]
    fn partial_owner_install_remains_unselectable() {
        let handlers = vec![
            FakeHandler::new("alpha", "body"),
            FakeHandler::new("beta", "body"),
        ];
        handlers[1].reject_install.store(true, Ordering::SeqCst);
        let package = package(&handlers);
        let (router, store) = setup(&handlers);

        assert_eq!(
            router
                .install(&package, 1, true, None)
                .unwrap_err()
                .diagnostic_code,
            "owner_install_failed"
        );
        assert_eq!(handlers[0].install_writes.load(Ordering::SeqCst), 1);
        assert!(store.head("fake.package").unwrap().is_none());
    }

    #[test]
    fn retry_reuses_exact_owner_revision() {
        let handlers = vec![
            FakeHandler::new("alpha", "body"),
            FakeHandler::new("beta", "body"),
            FakeHandler::new("gamma", "body"),
        ];
        handlers[2].reject_install.store(true, Ordering::SeqCst);
        let package = package(&handlers);
        let (router, store) = setup(&handlers);
        assert!(router.install(&package, 1, false, None).is_err());
        handlers[2].reject_install.store(false, Ordering::SeqCst);

        let receipt = router.install(&package, 2, true, None).unwrap();
        assert_eq!(receipt.components.len(), 3);
        assert_eq!(handlers[0].install_writes.load(Ordering::SeqCst), 1);
        assert_eq!(handlers[1].install_writes.load(Ordering::SeqCst), 1);
        assert_eq!(handlers[2].install_writes.load(Ordering::SeqCst), 1);
        assert_eq!(
            store.head("fake.package").unwrap().unwrap().receipt_id,
            receipt.receipt_id
        );
    }

    #[test]
    fn semantic_link_failure_prevents_receipt_commit() {
        let handlers = vec![FakeHandler::new("alpha", "body")];
        handlers[0].reject_links.store(true, Ordering::SeqCst);
        let package = package(&handlers);
        let (router, store) = setup(&handlers);

        assert_eq!(
            router
                .install(&package, 1, true, None)
                .unwrap_err()
                .diagnostic_code,
            "owner_link_invalid"
        );
        assert!(store.head("fake.package").unwrap().is_none());
    }

    #[test]
    fn historical_resolution_uses_exact_refs_and_missing_ref_has_no_fallback() {
        let handlers = vec![FakeHandler::new("alpha", "body")];
        let package = package(&handlers);
        let (router, store) = setup(&handlers);
        let receipt = router.install(&package, 1, true, None).unwrap();
        let resolver = PdsPackageResolver::new(router.catalog().clone(), store);
        let resolved = resolver.resolve(&receipt.receipt_id).unwrap();
        assert_eq!(resolved.receipt, receipt);

        handlers[0].remove("component-0");
        assert_eq!(
            resolver
                .resolve(&receipt.receipt_id)
                .unwrap_err()
                .diagnostic_code,
            "owner_revision_missing"
        );
    }

    #[test]
    fn missing_historical_route_fails_closed() {
        let handlers = vec![FakeHandler::new("alpha", "body")];
        let package = package(&handlers);
        let (router, store) = setup(&handlers);
        let receipt = router.install(&package, 1, false, None).unwrap();
        let empty_catalog = TheoryRouteCatalog::build(vec![]).unwrap();
        let resolver = PdsPackageResolver::new(empty_catalog, store);
        assert_eq!(
            resolver
                .resolve(&receipt.receipt_id)
                .unwrap_err()
                .diagnostic_code,
            "route_unavailable"
        );
    }
}
