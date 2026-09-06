//! Root structural product assembly over domain-owned package components.

use std::collections::BTreeSet;
use std::path::Path;

use crate::runtime::storage::OpenProductStores;
use crate::theory::{
    ActivationParticipantPlanV1, ActivationParticipantSpec, ParticipantKind,
    PdsPackageInstallationReceiptV1, PdsPackageManifestV1, ProductAgentPositionV1,
    ProductAgentSubscriptionV1, ProductDeclarationV1, ProductPackageSelectionV1, TheoryRouter,
    TheoryRouterDiagnostic, TheoryRouterError,
};

const PRODUCT_COMPILATION_POLICY: &str = "pds-product-compilation.v1";

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
    let inventory = crate::capability::product_capability_inventory()
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

/// Compose one assigned stewardship position over an exact routed package.
pub fn product_declaration(
    product_id: &str,
    principal_id: &str,
    receipt: &PdsPackageInstallationReceiptV1,
    observation_scope_component_id: &str,
    directive: &str,
    requested_authority_ref: &str,
    source_owners: &BTreeSet<String>,
) -> Result<ProductDeclarationV1, TheoryRouterError> {
    let participants = [
        ("docs.observation", "docs", ParticipantKind::BoundedActor),
        (
            "workspace.source",
            "workspace",
            ParticipantKind::PassiveSource,
        ),
        (
            "world_model.graph_replay",
            "world-model",
            ParticipantKind::BoundedActor,
        ),
        (
            "world_model.belief_assessment",
            "world-model",
            ParticipantKind::BoundedActor,
        ),
        (
            "world_model.evidence_ingestion",
            "world-model",
            ParticipantKind::BoundedActor,
        ),
        (
            "world_model.standing_curation",
            "world-model",
            ParticipantKind::BoundedActor,
        ),
        (
            "world_model.agent_reconciliation",
            "world-model",
            ParticipantKind::BoundedActor,
        ),
        (
            "execution.task_admission",
            "execution",
            ParticipantKind::DurableOperationAdapter,
        ),
        (
            "execution.task_dispatch",
            "execution",
            ParticipantKind::BoundedActor,
        ),
        (
            "execution.publication",
            "execution",
            ParticipantKind::BoundedActor,
        ),
    ]
    .into_iter()
    .filter(|(participant_id, _, _)| match *participant_id {
        "workspace.source" => source_owners.contains("workspace_fs"),
        "docs.observation" => source_owners.contains("docs"),
        _ => true,
    })
    .map(
        |(participant_id, owner_domain, kind)| ActivationParticipantSpec {
            participant_id: participant_id.to_string(),
            owner_domain: owner_domain.to_string(),
            kind,
            required: true,
            depends_on: BTreeSet::new(),
            readiness_contract_ref: format!("{owner_domain}.readiness.v1"),
            wake_contract_ref: format!("{owner_domain}.wake.v1"),
            safe_point_contract_ref: format!("{owner_domain}.safe-point.v1"),
            stop_contract_ref: format!("{owner_domain}.stop.v1"),
        },
    )
    .collect();
    let participant_plan = ActivationParticipantPlanV1::new(participants)?;
    ProductDeclarationV1::new(
        product_id.to_string(),
        principal_id.to_string(),
        vec![ProductPackageSelectionV1 {
            package_id: receipt.package_id.clone(),
            package_version: receipt.package_version.clone(),
            package_content_hash: receipt.package_content_hash.clone(),
        }],
        vec![ProductAgentPositionV1 {
            position_id: "steward".to_string(),
            directive: directive.to_string(),
            required_owner_routes: receipt
                .components
                .iter()
                .map(|component| component.route.clone())
                .collect(),
            observation_scope_component_id: observation_scope_component_id.to_string(),
            required_subscriptions: vec![ProductAgentSubscriptionV1 {
                source_owner: "belief".to_string(),
                source_contract_component_id: observation_scope_component_id.to_string(),
                initial_cursor_policy: "from_genesis".to_string(),
            }],
            participant_ref: "world_model.agent_reconciliation".to_string(),
        }],
        participant_plan,
        requested_authority_ref.to_string(),
        format!("principal-grant::{principal_id}"),
        PRODUCT_COMPILATION_POLICY.to_string(),
    )
}
