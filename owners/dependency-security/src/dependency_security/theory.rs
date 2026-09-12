//! Dependency-security owned exact policy registry.

mod route;
pub use route::{route_contract, route_handler};

use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

use super::policy::DependencySecurityPolicyV2;
use crate::theory::TheoryRevisionRef;

pub const PACKAGE_ID: &str = "meld.dependency-security-fixture";

pub fn install_package(
    stores: &crate::runtime::storage::OpenProductStores,
    package_root: &std::path::Path,
    installed_at_seq: u64,
) -> Result<crate::theory::PdsPackageInstallationReceiptV1, crate::theory::TheoryRouterError> {
    crate::init::world::product::install_package(
        stores,
        package_root,
        Some(PACKAGE_ID),
        installed_at_seq,
    )
}

const LEGACY_TREE: &str = "dependency_security_policy_revisions_v1";
const TREE: &str = "dependency_security_policy_revisions_v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencySecurityPolicyRevision {
    pub reference: TheoryRevisionRef,
    pub policy: DependencySecurityPolicyV2,
    pub installed_at_seq: u64,
}

#[derive(Clone)]
pub struct DependencySecurityPolicyRegistry {
    db: Db,
    revisions: Tree,
}

impl DependencySecurityPolicyRegistry {
    pub fn new(db: Db) -> Result<Self, String> {
        if db
            .tree_names()
            .iter()
            .any(|name| name.as_ref() == LEGACY_TREE.as_bytes())
            && !db
                .open_tree(LEGACY_TREE)
                .map_err(|error| error.to_string())?
                .is_empty()
        {
            return Err("retained dependency-security policy v1 revisions are incompatible with the fixed v2 contract; preserve the history and migrate it explicitly".into());
        }
        let revisions = db.open_tree(TREE).map_err(|e| e.to_string())?;
        Ok(Self { db, revisions })
    }
    pub fn install(
        &self,
        policy: DependencySecurityPolicyV2,
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
    use super::super::policy::REGISTRY;
    use crate::runtime::storage::{OpenProductStores, ProductStorageLayout};
    use crate::theory::{
        ActivationParticipantPlanV1, ActivationParticipantSpec, ParticipantKind,
        ProductAgentPositionV1, ProductAgentSubscriptionV1, ProductCompilationReceiptV1,
        ProductDeclarationV1, ProductPackageSelectionV1,
    };
    #[test]
    fn shipped_source_hashes_are_visible_for_manifest_review() {
        for contract in super::super::capability::published_contracts() {
            println!(
                "CONTRACT {} {}",
                contract.capability_type_id,
                contract.content_identity()
            );
        }
        for name in [
            "policy.cargo_fixture.json",
            "belief_family.security_coverage.json",
            "belief_family.security_posture.json",
            "outcome_mapping.security.json",
            "curation_rule.security.json",
            "maintained_condition.security.json",
            "authority_policy.security_read_only.json",
            "strategy_theory.security.json",
            "graph_owner_event_route.security.json",
            "epistemic_rule.security.json",
            "graph_owner_event_route.condition.json",
            "product_topology.json",
        ] {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
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
        let mut stores =
            OpenProductStores::open(&ProductStorageLayout::from_root(root.path())).unwrap();
        crate::test_support::configure(
            &mut stores,
            root.path(),
            &[("dependency-security", "meld-dependency-security-owner")],
        );
        let package_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("theory/dependency_security");
        let receipt = super::install_package(&stores, &package_root, 1).unwrap();
        assert_eq!(receipt.package_id, super::PACKAGE_ID);
        assert_eq!(receipt.package_version, "2.0.0");
        assert_eq!(receipt.components.len(), 16);
        let policy = receipt
            .components
            .iter()
            .find(|component| component.component_id == "security-policy")
            .unwrap();
        assert_eq!(policy.route.route_version, 2);
        assert_eq!(policy.component_schema_version, 2);
        assert_eq!(policy.owner_revision.registry, REGISTRY);
        let owners: std::collections::BTreeSet<_> = receipt
            .components
            .iter()
            .map(|component| component.route.owner_domain.as_str())
            .collect();
        assert_eq!(
            owners,
            std::collections::BTreeSet::from([
                "dependency-security",
                "execution",
                "runtime",
                "world-model"
            ])
        );
        let repeated = super::install_package(&stores, &package_root, 99).unwrap();
        assert_eq!(receipt.receipt_id, repeated.receipt_id);
    }

    #[test]
    fn legacy_policy_revision_identity_is_not_resolved_as_v2() {
        let registry = super::DependencySecurityPolicyRegistry::new(
            sled::Config::new().temporary(true).open().unwrap(),
        )
        .unwrap();
        let policy = serde_json::from_str(include_str!(
            "../../../../theory/dependency_security/policy.cargo_fixture.json"
        ))
        .unwrap();
        let current = registry.install(policy, 1).unwrap();
        let legacy = crate::theory::TheoryRevisionRef {
            registry: "dependency_security_policy".into(),
            id: current.id.clone(),
            content_hash: current.content_hash.clone(),
        };
        assert!(registry.resolve(&legacy).unwrap().is_none());
        assert!(registry.resolve(&current).unwrap().is_some());
    }

    #[test]
    fn nonempty_legacy_policy_tree_requires_explicit_migration() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        db.open_tree(super::LEGACY_TREE)
            .unwrap()
            .insert(b"legacy-ref", b"retained-v1-policy")
            .unwrap();
        let error = super::DependencySecurityPolicyRegistry::new(db)
            .err()
            .unwrap();
        assert!(error.contains("policy v1 revisions are incompatible"));
        assert!(error.contains("migrate it explicitly"));
    }

    #[test]
    fn security_product_compiles_with_its_own_topology_and_source_contract() {
        let root = tempfile::tempdir().unwrap();
        let mut stores =
            OpenProductStores::open(&ProductStorageLayout::from_root(root.path())).unwrap();
        crate::test_support::configure(
            &mut stores,
            root.path(),
            &[("dependency-security", "meld-dependency-security-owner")],
        );
        let package_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("theory/dependency_security");
        let receipt = super::install_package(&stores, &package_root, 1).unwrap();
        let participant_plan = ActivationParticipantPlanV1::new(vec![ActivationParticipantSpec {
            participant_id: "world_model.security_reconciliation".to_string(),
            owner_domain: "world-model".to_string(),
            kind: ParticipantKind::BoundedActor,
            required: true,
            depends_on: std::collections::BTreeSet::new(),
            readiness_contract_ref: "world-model.readiness.v1".to_string(),
            wake_contract_ref: "world-model.wake.v1".to_string(),
            safe_point_contract_ref: "world-model.safe-point.v1".to_string(),
            stop_contract_ref: "world-model.stop.v1".to_string(),
        }])
        .unwrap();
        let declaration = ProductDeclarationV1::new(
            "dependency_security".to_string(),
            "workspace-owner".to_string(),
            vec![ProductPackageSelectionV1 {
                package_id: receipt.package_id.clone(),
                package_version: receipt.package_version.clone(),
                package_content_hash: receipt.package_content_hash.clone(),
            }],
            vec![ProductAgentPositionV1 {
                observation_source_position_id: None,
                package_id: None,
                position_id: "security-steward".to_string(),
                directive: "maintain dependency security".to_string(),
                required_owner_routes: receipt
                    .components
                    .iter()
                    .map(|component| component.route.clone())
                    .collect(),
                observation_scope_component_id: "security-coverage-belief".to_string(),
                required_subscriptions: vec![ProductAgentSubscriptionV1 {
                    source_owner: "belief".to_string(),
                    source_contract_component_id: "security-coverage-belief".to_string(),
                    initial_cursor_policy: "from_genesis".to_string(),
                }],
                participant_ref: "world_model.security_reconciliation".to_string(),
            }],
            participant_plan,
            "security-read-only".to_string(),
            "principal-grant::workspace-owner".to_string(),
            "pds-product-compilation.v1".to_string(),
            Default::default(),
        )
        .unwrap();
        let compilation = ProductCompilationReceiptV1::compile(
            &declaration,
            vec![receipt],
            &stores.pds_packages,
            2,
        )
        .unwrap();
        assert_eq!(
            compilation.product_revision_id,
            declaration.product_revision_id
        );
        assert!(compilation
            .installed_owner_revisions
            .iter()
            .any(|component| component.component_id == "security-policy"));
    }
}
