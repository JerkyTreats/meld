//! Owner-authored semantic policy routing.

use crate::docs::claim_validation::{
    DocsClaimPolicy, DocsClaimPolicyRegistryStore, DocsClaimPolicyRevisionRef,
};
use crate::theory::*;
use std::sync::Arc;

pub fn route_contract() -> TheoryRouteContract {
    contract("docs", "claim-policy", RouteCardinality::AtMostOne)
}

pub fn route_handler(
    store: Arc<DocsClaimPolicyRegistryStore>,
) -> Arc<PortBackedTheoryRouteHandler> {
    let validate = Arc::new(|owner_id: &str, bytes: &[u8]| {
        let body: DocsClaimPolicy = decode(bytes)?;
        body.validate().map_err(owner_failure)?;
        require_id(owner_id, &body.policy_id)
    });
    let install_store = store.clone();
    let install = Arc::new(move |owner_id: &str, bytes: &[u8], seq| {
        let body: DocsClaimPolicy = decode(bytes)?;
        require_id(owner_id, &body.policy_id)?;
        let (_, revision) = install_store.install(body, seq).map_err(owner_failure)?;
        let reference = revision.revision_ref();
        Ok(TheoryRevisionRef {
            registry: "docs_claim_policy".to_string(),
            id: reference.policy_id,
            content_hash: reference.content_identity,
        })
    });
    let verify = Arc::new(move |reference: &TheoryRevisionRef| {
        require_registry(reference, "docs_claim_policy")?;
        require_found(
            store
                .resolve(&DocsClaimPolicyRevisionRef {
                    policy_id: reference.id.clone(),
                    content_identity: reference.content_hash.clone(),
                })
                .map_err(owner_failure)?
                .is_some(),
        )
    });
    let links = Arc::new(
        |reference: &InstalledTheoryComponentRef,
         _package: &InstalledPackageLinkView|
         -> Result<(), OwnerRouteDiagnostic> {
            if reference.owner_revision.id != reference.owner_revision.id.trim() {
                return Err(owner(
                    "policy_id_mismatch",
                    "claim policy id is not normalized",
                ));
            }
            Ok(())
        },
    );
    Arc::new(PortBackedTheoryRouteHandler::new(
        route_contract(),
        validate,
        install,
        verify,
        links,
    ))
}

fn contract(owner: &str, kind: &str, cardinality: RouteCardinality) -> TheoryRouteContract {
    TheoryRouteContract {
        route: TheoryRouteId::new(owner, kind, 1),
        accepted_component_schema: VersionRange { min: 1, max: 1 },
        package_cardinality: cardinality,
        handler_contract_version: 1,
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
