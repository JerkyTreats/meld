use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use super::contracts::{TheoryRouteHandler, TheoryRouteId};
use super::error::{error, TheoryRouterError};
use super::package::validate_name;
use super::receipt::PdsPackageInstallationReceiptV1;

const TREE_RECEIPTS: &str = "pds_package_receipts_v1";
const TREE_HEADS: &str = "pds_package_heads_v1";

/// Immutable compiled route catalog.
#[derive(Clone)]
pub struct TheoryRouteCatalog {
    handlers: Arc<BTreeMap<TheoryRouteId, Arc<dyn TheoryRouteHandler>>>,
    assembly_identity: String,
}

impl TheoryRouteCatalog {
    pub fn build(handlers: Vec<Arc<dyn TheoryRouteHandler>>) -> Result<Self, TheoryRouterError> {
        let mut sorted: Vec<_> = handlers
            .into_iter()
            .map(|handler| (handler.contract(), handler))
            .collect();
        sorted.sort_by(|left, right| left.0.route.cmp(&right.0.route));
        let mut catalog = BTreeMap::new();
        let mut contracts = Vec::new();
        for (contract, handler) in sorted {
            validate_name(&contract.route.owner_domain)?;
            validate_name(&contract.route.component_kind)?;
            if contract.route.route_version == 0
                || contract.handler_contract_version == 0
                || contract.accepted_component_schema.min == 0
                || contract.accepted_component_schema.min > contract.accepted_component_schema.max
            {
                return Err(error("route_unavailable", "invalid route contract"));
            }
            if catalog.insert(contract.route.clone(), handler).is_some() {
                return Err(error(
                    "route_unavailable",
                    format!("duplicate route publication '{}'", contract.route.key()),
                ));
            }
            contracts.push(contract);
        }
        let bytes = serde_json::to_vec(&contracts)
            .map_err(|failure| error("route_unavailable", failure.to_string()))?;
        Ok(Self {
            handlers: Arc::new(catalog),
            assembly_identity: blake3::hash(&bytes).to_hex().to_string(),
        })
    }

    pub fn handler(&self, route: &TheoryRouteId) -> Option<&Arc<dyn TheoryRouteHandler>> {
        self.handlers.get(route)
    }

    pub fn routes(&self) -> impl Iterator<Item = &TheoryRouteId> {
        self.handlers.keys()
    }

    pub fn assembly_identity(&self) -> &str {
        &self.assembly_identity
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PdsPackageHeadV1 {
    pub package_id: String,
    pub package_version: String,
    pub receipt_id: String,
    pub head_revision: u64,
}

/// Append-only receipt store and compare-and-swap package heads.
#[derive(Clone)]
pub struct PdsPackageStore {
    db: Db,
    receipts: Tree,
    heads: Tree,
}

impl PdsPackageStore {
    pub fn new(db: Db) -> Result<Self, TheoryRouterError> {
        let receipts = db
            .open_tree(TREE_RECEIPTS)
            .map_err(|failure| error("package_receipt_conflict", failure.to_string()))?;
        let heads = db
            .open_tree(TREE_HEADS)
            .map_err(|failure| error("package_head_conflict", failure.to_string()))?;
        Ok(Self {
            db,
            receipts,
            heads,
        })
    }

    pub fn install_receipt(
        &self,
        receipt: &PdsPackageInstallationReceiptV1,
    ) -> Result<bool, TheoryRouterError> {
        receipt.verify_identity()?;
        let bytes = serde_json::to_vec(receipt)
            .map_err(|failure| error("package_receipt_conflict", failure.to_string()))?;
        match self
            .receipts
            .compare_and_swap(
                receipt.receipt_id.as_bytes(),
                None as Option<&[u8]>,
                Some(bytes.as_slice()),
            )
            .map_err(|failure| error("package_receipt_conflict", failure.to_string()))?
        {
            Ok(()) => Ok(true),
            Err(conflict) => {
                let existing: PdsPackageInstallationReceiptV1 =
                    serde_json::from_slice(conflict.current.as_deref().ok_or_else(|| {
                        error(
                            "package_receipt_conflict",
                            "receipt compare conflict is empty",
                        )
                    })?)
                    .map_err(|failure| error("package_receipt_conflict", failure.to_string()))?;
                existing.verify_identity()?;
                if existing.receipt_id == receipt.receipt_id {
                    Ok(false)
                } else {
                    Err(error(
                        "package_receipt_conflict",
                        "receipt id already contains different semantic content",
                    ))
                }
            }
        }
    }

    pub fn resolve_receipt(
        &self,
        receipt_id: &str,
    ) -> Result<Option<PdsPackageInstallationReceiptV1>, TheoryRouterError> {
        let Some(bytes) = self
            .receipts
            .get(receipt_id.as_bytes())
            .map_err(|failure| error("package_receipt_corrupt", failure.to_string()))?
        else {
            return Ok(None);
        };
        let receipt: PdsPackageInstallationReceiptV1 = serde_json::from_slice(&bytes)
            .map_err(|failure| error("package_receipt_corrupt", failure.to_string()))?;
        receipt.verify_identity()?;
        Ok(Some(receipt))
    }

    pub fn head(&self, package_id: &str) -> Result<Option<PdsPackageHeadV1>, TheoryRouterError> {
        let Some(bytes) = self
            .heads
            .get(package_id.as_bytes())
            .map_err(|failure| error("package_head_conflict", failure.to_string()))?
        else {
            return Ok(None);
        };
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|failure| error("package_head_conflict", failure.to_string()))
    }

    pub fn heads(&self) -> Result<Vec<PdsPackageHeadV1>, TheoryRouterError> {
        let mut heads: Vec<PdsPackageHeadV1> = Vec::new();
        for item in &self.heads {
            let (_, bytes) =
                item.map_err(|failure| error("package_head_conflict", failure.to_string()))?;
            heads.push(
                serde_json::from_slice(&bytes)
                    .map_err(|failure| error("package_head_conflict", failure.to_string()))?,
            );
        }
        heads.sort_by(|left, right| left.package_id.cmp(&right.package_id));
        Ok(heads)
    }

    pub fn advance_head(
        &self,
        receipt: &PdsPackageInstallationReceiptV1,
        expected_prior: Option<&PdsPackageHeadV1>,
    ) -> Result<PdsPackageHeadV1, TheoryRouterError> {
        let current = expected_prior
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|failure| error("package_head_conflict", failure.to_string()))?;
        let next = PdsPackageHeadV1 {
            package_id: receipt.package_id.clone(),
            package_version: receipt.package_version.clone(),
            receipt_id: receipt.receipt_id.clone(),
            head_revision: expected_prior.map_or(1, |head| head.head_revision + 1),
        };
        let next_bytes = serde_json::to_vec(&next)
            .map_err(|failure| error("package_head_conflict", failure.to_string()))?;
        self.heads
            .compare_and_swap(
                receipt.package_id.as_bytes(),
                current.as_deref(),
                Some(next_bytes.as_slice()),
            )
            .map_err(|failure| error("package_head_conflict", failure.to_string()))?
            .map_err(|_| {
                error(
                    "package_head_conflict",
                    "package head expected prior mismatch",
                )
            })?;
        Ok(next)
    }

    pub fn flush(&self) -> Result<(), TheoryRouterError> {
        self.db
            .flush()
            .map(|_| ())
            .map_err(|failure| error("package_receipt_conflict", failure.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::{
        InstalledPackageLinkView, InstalledTheoryComponentRef, OwnerRouteDiagnostic,
        OwnerValidationToken, PackageLinkView, RouteCardinality, RoutedComponentSource,
        TheoryRouteContract, VersionRange,
    };

    struct EmptyHandler;

    impl TheoryRouteHandler for EmptyHandler {
        fn contract(&self) -> TheoryRouteContract {
            TheoryRouteContract {
                route: TheoryRouteId::new("fake", "body", 1),
                accepted_component_schema: VersionRange { min: 1, max: 1 },
                package_cardinality: RouteCardinality::Many,
                handler_contract_version: 1,
            }
        }
        fn validate(
            &self,
            _: &RoutedComponentSource,
            _: &PackageLinkView,
        ) -> Result<OwnerValidationToken, OwnerRouteDiagnostic> {
            unreachable!()
        }
        fn install(
            &self,
            _: &RoutedComponentSource,
            _: OwnerValidationToken,
            _: u64,
        ) -> Result<InstalledTheoryComponentRef, OwnerRouteDiagnostic> {
            unreachable!()
        }
        fn verify(&self, _: &InstalledTheoryComponentRef) -> Result<(), OwnerRouteDiagnostic> {
            Ok(())
        }
        fn validate_links(
            &self,
            _: &InstalledTheoryComponentRef,
            _: &InstalledPackageLinkView,
        ) -> Result<(), OwnerRouteDiagnostic> {
            Ok(())
        }
    }

    #[test]
    fn duplicate_route_publication_is_rejected() {
        let handlers: Vec<Arc<dyn TheoryRouteHandler>> =
            vec![Arc::new(EmptyHandler), Arc::new(EmptyHandler)];
        assert_eq!(
            TheoryRouteCatalog::build(handlers)
                .err()
                .unwrap()
                .diagnostic_code,
            "route_unavailable"
        );
    }
}
