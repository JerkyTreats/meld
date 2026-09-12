//! Root structural product assembly over domain-owned package components.

use std::path::Path;

use crate::runtime::storage::OpenProductStores;
use crate::theory::{
    PdsPackageInstallationReceiptV1, PdsPackageManifestV1, ProductDeclarationV1, ProductTopologyV1,
    TheoryRouter, TheoryRouterDiagnostic, TheoryRouterError,
};

/// Install through the compiled owner routes without interpreting product vocabulary.
pub fn install_package(
    stores: &OpenProductStores,
    package_root: &Path,
    expected_package_id: Option<&str>,
    installed_at_seq: u64,
) -> Result<PdsPackageInstallationReceiptV1, TheoryRouterError> {
    let manifest: PdsPackageManifestV1 = serde_json::from_slice(
        &std::fs::read(package_root.join("pds-package.json"))
            .map_err(|error| source_error(error.to_string()))?,
    )
    .map_err(|error| source_error(error.to_string()))?;
    if expected_package_id.is_some_and(|expected| manifest.package_id != expected) {
        return Err(source_error(
            "package identity differs from the requested owner package",
        ));
    }
    let inventory = crate::capability::product_capability_inventory_with_owners(&stores.owners)
        .map_err(|error| source_error(error.to_string()))?;
    let package = manifest.materialize_with_published(package_root, Some(&inventory))?;
    let catalog = super::routes::current_product_route_catalog(stores)?;
    let store = stores.pds_packages.as_ref().clone();
    let prior = store.head(&manifest.package_id)?;
    TheoryRouter::new(catalog, store).install(&package, installed_at_seq, true, prior.as_ref())
}

fn source_error(message: impl Into<String>) -> TheoryRouterError {
    TheoryRouterDiagnostic::new("package_source_invalid", message).into()
}

/// Resolve the exact package-authored topology without synthesizing product roles.
pub fn product_declaration(
    product_id: &str,
    principal_id: &str,
    package: &crate::theory::ResolvedPdsPackage,
    products: &crate::theory::PdsProductStore,
) -> Result<ProductDeclarationV1, TheoryRouterError> {
    // A composition owns its topology. Imported packages retain their standalone
    // topology without becoming a second active composition declaration.
    let roots = package
        .receipt
        .components
        .iter()
        .filter(|c| c.route == ProductTopologyV1::route())
        .cloned()
        .collect::<Vec<_>>();
    let topologies = if roots.is_empty() {
        package
            .components_by_route
            .get(&ProductTopologyV1::route())
            .map(Vec::as_slice)
    } else {
        Some(roots.as_slice())
    };
    let component = match topologies {
        Some([component]) => component,
        _ => {
            return Err(source_error(
                "product requires exactly one installed topology in its package closure",
            ))
        }
    };
    let topology = products
        .topology(&component.owner_revision)?
        .ok_or_else(|| source_error("installed product topology is absent"))?;
    topology.declare(product_id, principal_id, &package.receipt)
}
