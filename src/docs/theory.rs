//! Routed installation of the canonical docs stewardship package.

use std::collections::BTreeSet;
use std::path::Path;

use crate::capability::product_capability_inventory;
use crate::init::world::routes::current_product_route_catalog;
use crate::runtime::storage::OpenProductStores;
use crate::theory::{
    ActivationParticipantPlanV1, ActivationParticipantSpec, ParticipantKind,
    PdsPackageInstallationReceiptV1, PdsPackageManifestV1, PdsPackageStore, ProductAgentPositionV1,
    ProductAgentSubscriptionV1, ProductDeclarationV1, ProductPackageSelectionV1, TheoryRouter,
    TheoryRouterDiagnostic, TheoryRouterError,
};

pub const DOCS_PACKAGE_ID: &str = "meld.docs-freshness";
pub const DOCS_PRODUCT_ID: &str = "docs_freshness";
pub const PRODUCT_COMPILATION_POLICY: &str = "pds-product-compilation.v1";

pub fn install_package(
    stores: &OpenProductStores,
    package_root: &Path,
    installed_at_seq: u64,
) -> Result<PdsPackageInstallationReceiptV1, TheoryRouterError> {
    let manifest_path = package_root.join("pds-package.json");
    let manifest: PdsPackageManifestV1 = serde_json::from_slice(
        &std::fs::read(&manifest_path).map_err(|failure| source_error(failure.to_string()))?,
    )
    .map_err(|failure| source_error(failure.to_string()))?;
    if manifest.package_id != DOCS_PACKAGE_ID {
        return Err(source_error(
            "docs package id differs from canonical identity",
        ));
    }
    let inventory =
        product_capability_inventory().map_err(|failure| source_error(failure.to_string()))?;
    let package = manifest.materialize_with_published(package_root, Some(&inventory))?;
    let catalog = current_product_route_catalog(stores)?;
    let theory_db = stores
        .theory_db
        .opened()
        .expect("docs package installation requires the theory store")
        .clone();
    let package_store = PdsPackageStore::new(theory_db)?;
    let prior = package_store.head(DOCS_PACKAGE_ID)?;
    TheoryRouter::new(catalog, package_store).install(
        &package,
        installed_at_seq,
        true,
        prior.as_ref(),
    )
}

/// Build the exact reusable docs product declaration over one package receipt.
pub fn product_declaration(
    product_id: &str,
    principal_id: &str,
    receipt: &PdsPackageInstallationReceiptV1,
    observation_scope_component_id: &str,
    directive: &str,
) -> Result<ProductDeclarationV1, TheoryRouterError> {
    let participants = [
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
                source_contract_component_id: "docs-belief-family".to_string(),
                initial_cursor_policy: "from_genesis".to_string(),
            }],
            participant_ref: "world_model.agent_reconciliation".to_string(),
        }],
        participant_plan,
        "docs_workspace_local".to_string(),
        format!("principal-grant::{principal_id}"),
        PRODUCT_COMPILATION_POLICY.to_string(),
    )
}

fn source_error(message: impl Into<String>) -> TheoryRouterError {
    TheoryRouterDiagnostic::new("package_source_invalid", message).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SelectedStewardshipPackage;
    use crate::runtime::storage::ProductStorageLayout;
    use crate::runtime::theory::ResolvedStewardshipTheory;
    use crate::theory::{PdsPackageResolver, ProductCompilationReceiptV1, TheoryRouteId};
    use meld_events::DomainObjectRef;
    use std::path::PathBuf;

    fn package_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/docs_freshness")
    }

    #[test]
    fn routed_docs_package_round_trips_all_exact_owner_revisions() {
        let root = tempfile::tempdir().unwrap();
        let stores =
            OpenProductStores::open(&ProductStorageLayout::from_root(root.path())).unwrap();
        let receipt = install_package(&stores, &package_root(), 1).unwrap();
        assert_eq!(receipt.package_id, DOCS_PACKAGE_ID);
        assert_eq!(receipt.components.len(), 12);

        let catalog = current_product_route_catalog(&stores).unwrap();
        let package_store =
            PdsPackageStore::new(stores.theory_db.opened().unwrap().clone()).unwrap();
        let resolved = PdsPackageResolver::new(catalog, package_store)
            .resolve(&receipt.receipt_id)
            .unwrap();
        assert_eq!(resolved.receipt, receipt);
        assert_eq!(resolved.components_by_route.len(), 8);
        assert_eq!(
            resolved
                .components_by_route
                .get(&TheoryRouteId::new("execution", "capability-contract", 1))
                .unwrap()
                .len(),
            5
        );
    }

    #[test]
    fn exact_reinstall_is_idempotent_and_does_not_advance_the_head() {
        let root = tempfile::tempdir().unwrap();
        let stores =
            OpenProductStores::open(&ProductStorageLayout::from_root(root.path())).unwrap();
        let first = install_package(&stores, &package_root(), 1).unwrap();
        let second = install_package(&stores, &package_root(), 99).unwrap();
        assert_eq!(first.receipt_id, second.receipt_id);
        let store = PdsPackageStore::new(stores.theory_db.opened().unwrap().clone()).unwrap();
        let head = store.head(DOCS_PACKAGE_ID).unwrap().unwrap();
        assert_eq!(head.receipt_id, first.receipt_id);
        assert_eq!(head.head_revision, 1);
    }

    #[test]
    fn routed_receipt_resolves_by_explicit_identity_for_historical_reading() {
        let root = tempfile::tempdir().unwrap();
        let stores =
            OpenProductStores::open(&ProductStorageLayout::from_root(root.path())).unwrap();
        let receipt = install_package(&stores, &package_root(), 1).unwrap();
        let selection = SelectedStewardshipPackage {
            expression: "docs_freshness".to_string(),
            principal_id: "workspace-owner".to_string(),
            belief_family_id: "docs_freshness".to_string(),
            evidence_mapping_id: "docs_freshness_outcome_interpretation_v1".to_string(),
            curation_rule_id: "docs_freshness".to_string(),
            maintained_condition_id: "docs_freshness".to_string(),
            strategy_theory_id: "docs_freshness".to_string(),
            authority_policy_id: "docs_workspace_local".to_string(),
            claim_policy_id: "docs-claims-strict-v1".to_string(),
        };
        let subject = DomainObjectRef::new("workspace_fs", "node", "docs").unwrap();
        let resolved = ResolvedStewardshipTheory::resolve_pds_receipt(
            &stores,
            &selection,
            &subject,
            &receipt.receipt_id,
        )
        .unwrap();
        assert_eq!(resolved.package_receipt_ids, vec![receipt.receipt_id]);
        assert!(resolved.product_compilation_receipt_id.is_none());
        assert_eq!(resolved.executable_contracts.len(), 5);
        assert_eq!(resolved.receipt.selection, selection);
    }

    #[test]
    fn product_compilation_requires_the_complete_package_set_and_reopens_exactly() {
        let root = tempfile::tempdir().unwrap();
        let layout = ProductStorageLayout::from_root(root.path());
        let stores = OpenProductStores::open(&layout).unwrap();
        let receipt = install_package(&stores, &package_root(), 1).unwrap();
        let declaration = product_declaration(
            DOCS_PRODUCT_ID,
            "workspace-owner",
            &receipt,
            "docs-belief-family",
            "steward documentation freshness",
        )
        .unwrap();
        assert!(ProductCompilationReceiptV1::compile(&declaration, Vec::new(), 1).is_err());
        let compilation =
            ProductCompilationReceiptV1::compile(&declaration, vec![receipt], 1).unwrap();
        let head = stores
            .pds_products
            .install(&declaration, &compilation, None)
            .unwrap();
        assert_eq!(head.head_revision, 1);
        drop(stores);

        let reopened = OpenProductStores::open(&layout).unwrap();
        assert_eq!(
            reopened.pds_products.head(DOCS_PRODUCT_ID).unwrap(),
            Some(head)
        );
        assert_eq!(
            reopened
                .pds_products
                .compilation(&compilation.compilation_receipt_id)
                .unwrap(),
            Some(compilation)
        );
    }
}
