//! Owner-authored semantic policy routing.

use crate::theory::*;
use std::sync::Arc;

pub fn route_contract() -> TheoryRouteContract {
    contract("dependency-security", "policy", RouteCardinality::Many)
}

pub fn route_handler(
    store: crate::dependency_security::theory::DependencySecurityPolicyRegistry,
) -> Arc<PortBackedTheoryRouteHandler> {
    let validate = Arc::new(|owner_id: &str, bytes: &[u8]| {
        let body: crate::dependency_security::policy::DependencySecurityPolicyV2 = decode(bytes)?;
        body.validate().map_err(owner_failure)?;
        require_id(owner_id, &body.policy_id)
    });
    let install_store = store.clone();
    let install = Arc::new(move |owner_id: &str, bytes: &[u8], seq| {
        let body: crate::dependency_security::policy::DependencySecurityPolicyV2 = decode(bytes)?;
        require_id(owner_id, &body.policy_id)?;
        install_store.install(body, seq).map_err(owner_failure)
    });
    let verify = Arc::new(move |reference: &TheoryRevisionRef| {
        require_registry(reference, crate::dependency_security::policy::REGISTRY)?;
        require_found(store.resolve(reference).map_err(owner_failure)?.is_some())
    });
    Arc::new(PortBackedTheoryRouteHandler::new(
        route_contract(),
        validate,
        install,
        verify,
        no_semantic_links(),
    ))
}

fn contract(owner: &str, kind: &str, cardinality: RouteCardinality) -> TheoryRouteContract {
    TheoryRouteContract {
        route: TheoryRouteId::new(owner, kind, 2),
        accepted_component_schema: VersionRange { min: 2, max: 2 },
        package_cardinality: cardinality,
        handler_contract_version: 2,
    }
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, OwnerRouteDiagnostic> {
    serde_json::from_slice(bytes).map_err(owner_failure)
}

fn require_id(expected: &str, actual: &str) -> Result<(), OwnerRouteDiagnostic> {
    if expected == actual {
        Ok(())
    } else {
        Err(owner(
            "owner_id_mismatch",
            format!("routed owner id '{expected}' differs from body id '{actual}'"),
        ))
    }
}

fn require_registry(
    reference: &TheoryRevisionRef,
    expected: &str,
) -> Result<(), OwnerRouteDiagnostic> {
    if reference.registry == expected {
        Ok(())
    } else {
        Err(owner(
            "owner_ref_invalid",
            format!(
                "owner ref registry '{}' differs from '{expected}'",
                reference.registry
            ),
        ))
    }
}

fn require_found(found: bool) -> Result<(), OwnerRouteDiagnostic> {
    if found {
        Ok(())
    } else {
        Err(owner(
            "owner_revision_missing",
            "exact owner revision is absent",
        ))
    }
}

fn owner(code: impl Into<String>, message: impl Into<String>) -> OwnerRouteDiagnostic {
    OwnerRouteDiagnostic::new(code, message)
}

fn owner_failure(failure: impl ToString) -> OwnerRouteDiagnostic {
    owner("owner_contract_invalid", failure.to_string())
}
