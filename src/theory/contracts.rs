use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// Exact semantic route owned by one domain.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TheoryRouteId {
    pub owner_domain: String,
    pub component_kind: String,
    pub route_version: u32,
}

impl TheoryRouteId {
    pub fn new(
        owner_domain: impl Into<String>,
        component_kind: impl Into<String>,
        route_version: u32,
    ) -> Self {
        Self {
            owner_domain: owner_domain.into(),
            component_kind: component_kind.into(),
            route_version,
        }
    }

    pub fn key(&self) -> String {
        format!(
            "{}.{}.v{}",
            self.owner_domain, self.component_kind, self.route_version
        )
    }
}

/// Inclusive owner-schema range accepted by a route handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionRange {
    pub min: u32,
    pub max: u32,
}

impl VersionRange {
    pub fn contains(&self, version: u32) -> bool {
        self.min <= version && version <= self.max
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteCardinality {
    AtMostOne,
    Many,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TheoryRouteContract {
    pub route: TheoryRouteId,
    pub accepted_component_schema: VersionRange,
    pub package_cardinality: RouteCardinality,
    pub handler_contract_version: u32,
}

impl TheoryRouteContract {
    pub fn identity(&self) -> String {
        let bytes = serde_json::to_vec(self).expect("route contract serializes");
        blake3::hash(&bytes).to_hex().to_string()
    }
}

/// Opaque owner validation failure preserved by the router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerRouteDiagnostic {
    pub code: String,
    pub message: String,
}

impl OwnerRouteDiagnostic {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

/// Materialized component bytes supplied to an owner handler.
#[derive(Debug, Clone)]
pub struct RoutedComponentSource {
    pub component_id: String,
    pub owner_component_id: String,
    pub route: TheoryRouteId,
    pub component_schema_version: u32,
    pub source_content_hash: String,
    pub canonical_bytes: Arc<[u8]>,
}

/// Structural package view available during owner validation.
#[derive(Debug, Clone)]
pub struct PackageLinkView {
    pub package_id: String,
    pub package_content_hash: String,
    pub exact_import_receipts: BTreeMap<String, String>,
    pub component_routes: BTreeMap<String, TheoryRouteId>,
}

/// Exact installed package view available during semantic-link validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackageLinkView {
    pub package_id: String,
    pub package_content_hash: String,
    pub exact_import_receipts: BTreeMap<String, String>,
    pub components: BTreeMap<String, InstalledTheoryComponentRef>,
}

/// Generic exact owner revision identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TheoryRevisionRef {
    pub registry: String,
    pub id: String,
    pub content_hash: String,
}

/// Exact owner result retained by a package receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledTheoryComponentRef {
    pub component_id: String,
    pub route: TheoryRouteId,
    pub component_schema_version: u32,
    pub source_content_hash: String,
    pub owner_revision: TheoryRevisionRef,
}

/// Process-local proof returned by one exact handler validation.
#[derive(Debug)]
pub struct OwnerValidationToken {
    handler_identity: String,
    package_content_hash: String,
    component_id: String,
    source_content_hash: String,
    opaque: Vec<u8>,
}

impl OwnerValidationToken {
    pub fn new(
        contract: &TheoryRouteContract,
        package: &PackageLinkView,
        source: &RoutedComponentSource,
        opaque: Vec<u8>,
    ) -> Self {
        Self {
            handler_identity: contract.identity(),
            package_content_hash: package.package_content_hash.clone(),
            component_id: source.component_id.clone(),
            source_content_hash: source.source_content_hash.clone(),
            opaque,
        }
    }

    pub fn opaque(&self) -> &[u8] {
        &self.opaque
    }

    pub(crate) fn matches(
        &self,
        contract: &TheoryRouteContract,
        package: &PackageLinkView,
        source: &RoutedComponentSource,
    ) -> bool {
        self.handler_identity == contract.identity()
            && self.package_content_hash == package.package_content_hash
            && self.component_id == source.component_id
            && self.source_content_hash == source.source_content_hash
    }
}

/// Narrow owner boundary used by the generic router.
pub trait TheoryRouteHandler: Send + Sync {
    fn contract(&self) -> TheoryRouteContract;

    fn validate(
        &self,
        source: &RoutedComponentSource,
        package: &PackageLinkView,
    ) -> Result<OwnerValidationToken, OwnerRouteDiagnostic>;

    fn install(
        &self,
        source: &RoutedComponentSource,
        validation: OwnerValidationToken,
        installed_at_seq: u64,
    ) -> Result<InstalledTheoryComponentRef, OwnerRouteDiagnostic>;

    fn verify(&self, reference: &InstalledTheoryComponentRef) -> Result<(), OwnerRouteDiagnostic>;

    fn validate_links(
        &self,
        reference: &InstalledTheoryComponentRef,
        package: &InstalledPackageLinkView,
    ) -> Result<(), OwnerRouteDiagnostic>;
}
