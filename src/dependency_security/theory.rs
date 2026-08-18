//! Dependency-security owned exact policy registry.

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use super::policy::DependencySecurityPolicyV1;
use crate::theory::TheoryRevisionRef;

pub const PACKAGE_ID: &str = "meld.dependency-security-fixture";

pub fn install_package(
    stores: &crate::runtime::storage::OpenProductStores,
    package_root: &std::path::Path,
    installed_at_seq: u64,
) -> Result<crate::theory::PdsPackageInstallationReceiptV1, crate::theory::TheoryRouterError> {
    use crate::theory::{
        PdsPackageManifestV1, PdsPackageStore, TheoryRouter, TheoryRouterDiagnostic,
    };
    let fail = |message: String| -> crate::theory::TheoryRouterError {
        TheoryRouterDiagnostic::new("package_source_invalid", message).into()
    };
    let manifest: PdsPackageManifestV1 = serde_json::from_slice(
        &std::fs::read(package_root.join("pds-package.json"))
            .map_err(|failure| fail(failure.to_string()))?,
    )
    .map_err(|failure| fail(failure.to_string()))?;
    if manifest.package_id != PACKAGE_ID {
        return Err(fail("dependency-security package id differs".into()));
    }
    let inventory = crate::capability::product_capability_inventory()
        .map_err(|failure| fail(failure.to_string()))?;
    let package = manifest.materialize_with_published(package_root, Some(&inventory))?;
    let catalog = crate::init::world::routes::current_product_route_catalog(stores)?;
    let package_store = PdsPackageStore::new(
        stores
            .theory_db
            .opened()
            .expect("security package requires theory store")
            .clone(),
    )?;
    let prior = package_store.head(PACKAGE_ID)?;
    TheoryRouter::new(catalog, package_store).install(
        &package,
        installed_at_seq,
        true,
        prior.as_ref(),
    )
}

const TREE: &str = "dependency_security_policy_revisions_v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencySecurityPolicyRevision {
    pub reference: TheoryRevisionRef,
    pub policy: DependencySecurityPolicyV1,
    pub installed_at_seq: u64,
}

#[derive(Clone)]
pub struct DependencySecurityPolicyRegistry {
    db: Db,
    revisions: Tree,
}

impl DependencySecurityPolicyRegistry {
    pub fn new(db: Db) -> Result<Self, String> {
        let revisions = db.open_tree(TREE).map_err(|e| e.to_string())?;
        Ok(Self { db, revisions })
    }
    pub fn install(
        &self,
        policy: DependencySecurityPolicyV1,
        seq: u64,
    ) -> Result<TheoryRevisionRef, String> {
        let reference = policy.revision_ref()?;
        let revision = DependencySecurityPolicyRevision {
            reference: reference.clone(),
            policy,
            installed_at_seq: seq,
        };
        let key = key(&reference);
        let bytes = bincode::serialize(&revision).map_err(|e| e.to_string())?;
        if let Some(existing) = self.revisions.get(&key).map_err(|e| e.to_string())? {
            let existing: DependencySecurityPolicyRevision =
                bincode::deserialize(&existing).map_err(|e| e.to_string())?;
            if existing.reference != revision.reference || existing.policy != revision.policy {
                return Err("exact policy revision differs".into());
            }
        } else {
            self.revisions
                .insert(&key, bytes)
                .map_err(|e| e.to_string())?;
            self.db.flush().map_err(|e| e.to_string())?;
        }
        Ok(reference)
    }
    pub fn resolve(
        &self,
        reference: &TheoryRevisionRef,
    ) -> Result<Option<DependencySecurityPolicyRevision>, String> {
        self.revisions
            .get(key(reference))
            .map_err(|e| e.to_string())?
            .map(|bytes| bincode::deserialize(&bytes).map_err(|e| e.to_string()))
            .transpose()
    }
}
fn key(reference: &TheoryRevisionRef) -> String {
    format!(
        "{}:{}:{}",
        reference.registry, reference.id, reference.content_hash
    )
}

#[cfg(test)]
mod tests {
    use crate::runtime::storage::{OpenProductStores, ProductStorageLayout};
    #[test]
    fn shipped_source_hashes_are_visible_for_manifest_review() {
        for name in [
            "policy.cargo_fixture.json",
            "belief_family.security_coverage.json",
            "belief_family.security_posture.json",
            "outcome_mapping.security.json",
            "curation_rule.security.json",
            "maintained_condition.security.json",
            "authority_policy.security_read_only.json",
            "strategy_theory.security.json",
        ] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("theory/dependency_security")
                .join(name);
            println!(
                "{} {name}",
                blake3::hash(&std::fs::read(path).unwrap()).to_hex()
            );
        }
    }

    #[test]
    fn package_installs_across_security_world_and_execution_routes() {
        let root = tempfile::tempdir().unwrap();
        let stores =
            OpenProductStores::open(&ProductStorageLayout::from_root(root.path())).unwrap();
        let package_root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/dependency_security");
        let receipt = super::install_package(&stores, &package_root, 1).unwrap();
        assert_eq!(receipt.package_id, super::PACKAGE_ID);
        assert_eq!(receipt.components.len(), 12);
        let owners: std::collections::BTreeSet<_> = receipt
            .components
            .iter()
            .map(|component| component.route.owner_domain.as_str())
            .collect();
        assert_eq!(
            owners,
            std::collections::BTreeSet::from(["dependency-security", "execution", "world-model"])
        );
        let repeated = super::install_package(&stores, &package_root, 99).unwrap();
        assert_eq!(receipt.receipt_id, repeated.receipt_id);
    }
}
