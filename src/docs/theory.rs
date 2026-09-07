//! Docs-owned package selection over the shared product installer.

use crate::runtime::storage::OpenProductStores;
use crate::theory::{PdsPackageInstallationReceiptV1, TheoryRouterError};
use std::path::Path;

pub const DOCS_PACKAGE_ID: &str = "meld.docs-freshness";
pub const DOCS_PRODUCT_ID: &str = "docs_freshness";

pub fn install_package(
    stores: &OpenProductStores,
    package_root: &Path,
    installed_at_seq: u64,
) -> Result<PdsPackageInstallationReceiptV1, TheoryRouterError> {
    crate::init::world::product::install_package(
        stores,
        package_root,
        Some(DOCS_PACKAGE_ID),
        installed_at_seq,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SelectedStewardshipPackage;
    use crate::init::world::routes::current_product_route_catalog;
    use crate::runtime::storage::ProductStorageLayout;
    use crate::runtime::theory::ResolvedStewardshipTheory;
    use crate::theory::PdsPackageStore;
    use crate::theory::{PdsPackageResolver, ProductCompilationReceiptV1, TheoryRouteId};
    use meld_events::DomainObjectRef;
    use std::path::PathBuf;

    fn package_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/docs_freshness")
    }

    #[test]
    fn native_coverage_mapping_rejects_task_success_and_unobserved_scope_shortcuts() {
        use meld_world_model::belief::{
            ConfiguredOutcomeMappingSet, OutcomeEvidenceMapping, OutcomeMappingDisposition,
            OutcomeMappingInput,
        };
        let mapping = ConfiguredOutcomeMappingSet::new(
            serde_json::from_str(include_str!(
                "../../theory/docs_freshness/outcome_interpretation.docs_freshness.json"
            ))
            .unwrap(),
        )
        .unwrap();
        let native = serde_json::json!({"disposition":"applied", "semantic_publication":{"batch":{"objects":[{"qualifications":{
            "coverage":"satisfied", "output_policy_revision":"docs-required-coverage-v1",
            "judgment_subject_domain":"workspace_fs", "judgment_subject_kind":"node", "judgment_subject_id":"scope"
        }}]}}});
        let mut foreign_policy = native.clone();
        foreign_policy["semantic_publication"]["batch"]["objects"][0]["qualifications"]
            ["output_policy_revision"] = "foreign-policy".into();
        for (domain, event_type, payload, applicable) in [
            ("curation", "world_model.curation.result.v1", native, true),
            (
                "curation",
                "world_model.curation.result.v1",
                foreign_policy,
                false,
            ),
            (
                "execution",
                "execution.task.succeeded",
                serde_json::json!({"artifact_records":[{"artifact_type_id":"docs_freshness_assessment","content":{"subject_id":"scope","stale_probability":0.0}}]}),
                false,
            ),
            (
                "world_model",
                "world_model.unobserved_scope",
                serde_json::json!({}),
                false,
            ),
        ] {
            let input = OutcomeMappingInput {
                record: meld_events::EventRecord {
                    seq: 1,
                    envelope: meld_events::EventEnvelope::with_now_domain(
                        "docs-mapping-test",
                        domain,
                        "scope",
                        event_type,
                        None,
                        payload,
                    )
                    .with_record_id("mapping-test"),
                },
                mapping_id: mapping.mapping_id().into(),
                mapping_revision: None,
            };
            let result = mapping.map_outcome(&input);
            if applicable {
                assert!(
                    matches!(result, OutcomeMappingDisposition::Applicable { .. }),
                    "{result:?}"
                );
            } else {
                assert!(
                    matches!(result, OutcomeMappingDisposition::NotApplicable { .. }),
                    "{result:?}"
                );
            }
        }
    }

    #[test]
    fn routed_docs_package_round_trips_all_exact_owner_revisions() {
        let root = tempfile::tempdir().unwrap();
        let stores =
            OpenProductStores::open(&ProductStorageLayout::from_root(root.path())).unwrap();
        let receipt = install_package(&stores, &package_root(), 1).unwrap();
        assert_eq!(receipt.package_id, DOCS_PACKAGE_ID);
        assert_eq!(receipt.components.len(), 13);

        let catalog = current_product_route_catalog(&stores).unwrap();
        let package_store =
            PdsPackageStore::new(stores.theory_db.opened().unwrap().clone()).unwrap();
        let resolved = PdsPackageResolver::new(catalog, package_store)
            .resolve(&receipt.receipt_id)
            .unwrap();
        assert_eq!(resolved.receipt, receipt);
        assert_eq!(resolved.components_by_route.len(), 10);
        assert_eq!(
            resolved
                .components_by_route
                .get(&TheoryRouteId::new("execution", "capability-contract", 1))
                .unwrap()
                .len(),
            4
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
        assert_eq!(resolved.executable_contracts.len(), 4);
        assert_eq!(resolved.receipt.selection, selection);
    }

    #[test]
    fn product_compilation_requires_the_complete_package_set_and_reopens_exactly() {
        let root = tempfile::tempdir().unwrap();
        let layout = ProductStorageLayout::from_root(root.path());
        let stores = OpenProductStores::open(&layout).unwrap();
        let receipt = install_package(&stores, &package_root(), 1).unwrap();
        let declaration = crate::init::world::product::product_declaration(
            DOCS_PRODUCT_ID,
            "workspace-owner",
            &receipt,
            "docs-belief-family",
            "steward documentation freshness",
            "docs_workspace_local",
            &std::collections::BTreeSet::from(["workspace_fs".into(), "docs".into()]),
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
