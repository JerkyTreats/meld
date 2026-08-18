use serde::{Deserialize, Serialize};

use super::contracts::InstalledTheoryComponentRef;
use super::error::{error, TheoryRouterError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledExactPackageImport {
    pub package_id: String,
    pub receipt_id: String,
}

/// Immutable exact receipt for one completely installed package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdsPackageInstallationReceiptV1 {
    pub receipt_id: String,
    pub package_id: String,
    pub package_version: String,
    pub package_content_hash: String,
    pub manifest_schema_version: u32,
    pub exact_imports: Vec<InstalledExactPackageImport>,
    pub components: Vec<InstalledTheoryComponentRef>,
    pub installed_at_seq: u64,
}

#[derive(Serialize)]
struct ReceiptIdentity<'a> {
    package_id: &'a str,
    package_version: &'a str,
    package_content_hash: &'a str,
    manifest_schema_version: u32,
    exact_imports: &'a [InstalledExactPackageImport],
    components: &'a [InstalledTheoryComponentRef],
}

impl PdsPackageInstallationReceiptV1 {
    pub fn new(
        package_id: String,
        package_version: String,
        package_content_hash: String,
        manifest_schema_version: u32,
        mut exact_imports: Vec<InstalledExactPackageImport>,
        mut components: Vec<InstalledTheoryComponentRef>,
        installed_at_seq: u64,
    ) -> Result<Self, TheoryRouterError> {
        exact_imports.sort_by(|left, right| {
            left.package_id
                .cmp(&right.package_id)
                .then(left.receipt_id.cmp(&right.receipt_id))
        });
        components.sort_by(|left, right| {
            left.route
                .cmp(&right.route)
                .then(left.component_id.cmp(&right.component_id))
        });
        let identity = ReceiptIdentity {
            package_id: &package_id,
            package_version: &package_version,
            package_content_hash: &package_content_hash,
            manifest_schema_version,
            exact_imports: &exact_imports,
            components: &components,
        };
        let receipt_id = identity_hash(&identity)?;
        Ok(Self {
            receipt_id,
            package_id,
            package_version,
            package_content_hash,
            manifest_schema_version,
            exact_imports,
            components,
            installed_at_seq,
        })
    }

    pub fn verify_identity(&self) -> Result<(), TheoryRouterError> {
        let identity = ReceiptIdentity {
            package_id: &self.package_id,
            package_version: &self.package_version,
            package_content_hash: &self.package_content_hash,
            manifest_schema_version: self.manifest_schema_version,
            exact_imports: &self.exact_imports,
            components: &self.components,
        };
        if identity_hash(&identity)? != self.receipt_id {
            return Err(error(
                "package_receipt_corrupt",
                "package receipt identity mismatch",
            ));
        }
        if !self.exact_imports.windows(2).all(|pair| {
            (&pair[0].package_id, &pair[0].receipt_id) < (&pair[1].package_id, &pair[1].receipt_id)
        }) || !self.components.windows(2).all(|pair| {
            (&pair[0].route, &pair[0].component_id) < (&pair[1].route, &pair[1].component_id)
        }) {
            return Err(error(
                "package_receipt_corrupt",
                "package receipt collections are not canonical",
            ));
        }
        Ok(())
    }
}

fn identity_hash(identity: &ReceiptIdentity<'_>) -> Result<String, TheoryRouterError> {
    let bytes = serde_json::to_vec(identity)
        .map_err(|failure| error("package_receipt_corrupt", failure.to_string()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::{TheoryRevisionRef, TheoryRouteId};

    fn component(id: &str) -> InstalledTheoryComponentRef {
        InstalledTheoryComponentRef {
            component_id: id.to_string(),
            route: TheoryRouteId::new("fake", "body", 1),
            component_schema_version: 1,
            source_content_hash: format!("source-{id}"),
            owner_revision: TheoryRevisionRef {
                registry: "fake".to_string(),
                id: id.to_string(),
                content_hash: format!("owner-{id}"),
            },
        }
    }

    #[test]
    fn receipt_identity_is_order_independent() {
        let first = PdsPackageInstallationReceiptV1::new(
            "package".to_string(),
            "1".to_string(),
            "content".to_string(),
            1,
            vec![],
            vec![component("a"), component("b")],
            1,
        )
        .unwrap();
        let second = PdsPackageInstallationReceiptV1::new(
            "package".to_string(),
            "1".to_string(),
            "content".to_string(),
            1,
            vec![],
            vec![component("b"), component("a")],
            99,
        )
        .unwrap();
        assert_eq!(first.receipt_id, second.receipt_id);
    }

    #[test]
    fn corrupt_receipt_fails_closed() {
        let mut receipt = PdsPackageInstallationReceiptV1::new(
            "package".to_string(),
            "1".to_string(),
            "content".to_string(),
            1,
            vec![],
            vec![component("a")],
            1,
        )
        .unwrap();
        receipt.package_content_hash = "changed".to_string();
        assert_eq!(
            receipt.verify_identity().unwrap_err().diagnostic_code,
            "package_receipt_corrupt"
        );
    }
}
