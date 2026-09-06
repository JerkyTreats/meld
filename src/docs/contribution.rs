//! Docs-owned publication of exact contracts and physical implementations.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use meld_execution::capability::{CapabilityContractRevision, CapabilityInvoker};

use crate::capability::{
    CapabilityContributionDiagnostic, CapabilityFactoryRequest, CapabilityImplementationOffer,
    CapabilityInvokerFactory, OwnerBindingView, PreparedCapabilityInvoker,
    ProductCapabilityContributor,
};
use crate::docs::capability::{
    AssessPublishedScopeCapability, DocsCapabilityConfig, DraftPatchSetCapability,
    InspectScopeCapability, PublishPatchSetCapability, ValidatePatchSetCapability,
    ASSESS_PUBLISHED_SCOPE, DRAFT_PATCH_SET, INSPECT_SCOPE, PUBLISH_PATCH_SET, VALIDATE_PATCH_SET,
};
use crate::docs::claim_validation::{DocsClaimPolicy, DocsClaimPolicyRevision};
use crate::error::ApiError;
use crate::execution::ExecutionRuntimeContext;
use crate::provider::{ProviderExecutionBinding, ProviderRuntimeOverrides};

const WORKSPACE_BINDING: &str = "workspace";
const PROVIDER_BINDING: &str = "provider";
const SUBJECT_BINDING: &str = "subject";
const AGENT_BINDING: &str = "agent";
const CLAIM_POLICY_BINDING: &str = "docs.claim-policy";

pub struct DocsCapabilityContributor;

/// Bind the exact owner-resolved revision selected by product preparation.
pub fn bind_claim_policy(
    bindings: OwnerBindingView,
    revision: &DocsClaimPolicyRevision,
) -> Result<OwnerBindingView, ApiError> {
    revision.policy.validate()?;
    if revision.content_identity != revision.policy.content_identity() {
        return Err(ApiError::ConfigError(
            "docs claim policy binding is corrupt".into(),
        ));
    }
    let body = serde_json::to_string(revision)
        .map_err(|error| ApiError::ConfigError(error.to_string()))?;
    Ok(bindings.with_value(CLAIM_POLICY_BINDING, body))
}

impl ProductCapabilityContributor for DocsCapabilityContributor {
    fn owner_domain(&self) -> &str {
        "docs"
    }

    fn published_contracts(&self) -> Vec<CapabilityContractRevision> {
        crate::docs::capability::published_contracts()
            .into_iter()
            .map(|contract| CapabilityContractRevision {
                content_identity: contract.content_identity(),
                contract,
                installed_at_seq: 0,
            })
            .collect()
    }

    fn implementation_offers(&self) -> Vec<CapabilityImplementationOffer> {
        self.published_contracts()
            .into_iter()
            .map(|revision| {
                let type_id = revision.contract.capability_type_id.clone();
                let mut required_binding_ids = BTreeSet::from([
                    WORKSPACE_BINDING.to_string(),
                    SUBJECT_BINDING.to_string(),
                    AGENT_BINDING.to_string(),
                ]);
                if matches!(type_id.as_str(), DRAFT_PATCH_SET | VALIDATE_PATCH_SET) {
                    required_binding_ids.insert(PROVIDER_BINDING.to_string());
                }
                if matches!(type_id.as_str(), VALIDATE_PATCH_SET | PUBLISH_PATCH_SET) {
                    required_binding_ids.insert(CLAIM_POLICY_BINDING.to_string());
                }
                CapabilityImplementationOffer {
                    contract_ref: revision.revision_ref(),
                    implementation_ref: format!("docs.in-process.v1::{type_id}"),
                    required_binding_ids,
                    execution_class: revision.contract.execution_contract.execution_class,
                    factory: Arc::new(DocsInvokerFactory {
                        capability_type_id: type_id,
                    }),
                }
            })
            .collect()
    }
}

struct DocsInvokerFactory {
    capability_type_id: String,
}

fn selected_claim_policy(
    bindings: &OwnerBindingView,
) -> Result<DocsClaimPolicy, CapabilityContributionDiagnostic> {
    let body = bindings.get(CLAIM_POLICY_BINDING).ok_or_else(|| {
        diagnostic(
            "selected_binding_missing",
            "docs implementation requires an installed claim policy",
        )
    })?;
    let revision: DocsClaimPolicyRevision = serde_json::from_str(body)
        .map_err(|error| diagnostic("selected_binding_invalid", error.to_string()))?;
    revision
        .policy
        .validate()
        .map_err(|error| diagnostic("selected_binding_invalid", error.to_string()))?;
    if revision.content_identity != revision.policy.content_identity() {
        return Err(diagnostic(
            "selected_binding_invalid",
            "docs claim policy binding is corrupt",
        ));
    }
    Ok(revision.policy)
}

impl CapabilityInvokerFactory for DocsInvokerFactory {
    fn prepare(
        &self,
        _request: &CapabilityFactoryRequest<'_>,
        bindings: &OwnerBindingView,
    ) -> Result<PreparedCapabilityInvoker, CapabilityContributionDiagnostic> {
        let workspace = bindings.get(WORKSPACE_BINDING).ok_or_else(|| {
            diagnostic(
                "selected_binding_missing",
                "docs implementation requires a workspace binding",
            )
        })?;
        let provider_name = bindings
            .get(PROVIDER_BINDING)
            .unwrap_or("provider-not-selected");
        let subject_id = bindings.get(SUBJECT_BINDING).ok_or_else(|| {
            diagnostic(
                "selected_binding_missing",
                "docs implementation requires a subject binding",
            )
        })?;
        let agent_id = bindings.get(AGENT_BINDING).ok_or_else(|| {
            diagnostic(
                "selected_binding_missing",
                "docs implementation requires an agent binding",
            )
        })?;
        let provider =
            ProviderExecutionBinding::new(provider_name, ProviderRuntimeOverrides::default())
                .map_err(|failure| diagnostic("selected_binding_missing", failure.to_string()))?;
        let config = DocsCapabilityConfig {
            target_root: PathBuf::from(workspace),
            subject_id: subject_id.to_string(),
            agent_id: agent_id.to_string(),
            provider,
        };
        let invoker: Arc<
            dyn CapabilityInvoker<Error = ApiError, ExecutionApi = dyn ExecutionRuntimeContext>,
        > = match self.capability_type_id.as_str() {
            INSPECT_SCOPE => Arc::new(InspectScopeCapability::new(config)),
            DRAFT_PATCH_SET => Arc::new(DraftPatchSetCapability::new(config)),
            VALIDATE_PATCH_SET => Arc::new(ValidatePatchSetCapability::new(
                config,
                selected_claim_policy(bindings)?,
            )),
            PUBLISH_PATCH_SET => Arc::new(PublishPatchSetCapability::new(
                config,
                selected_claim_policy(bindings)?,
            )),
            ASSESS_PUBLISHED_SCOPE => Arc::new(AssessPublishedScopeCapability::new(config)),
            other => {
                return Err(diagnostic(
                    "selected_implementation_missing",
                    format!("docs implementation does not publish '{other}'"),
                ));
            }
        };
        Ok(invoker)
    }
}

fn diagnostic(
    code: impl Into<String>,
    message: impl Into<String>,
) -> CapabilityContributionDiagnostic {
    let mut diagnostic = CapabilityContributionDiagnostic::new(code, message);
    diagnostic.owner_domain = Some("docs".to_string());
    diagnostic
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::capability::ExecutionClass;
    use crate::capability::{ExactCapabilityActivationRequest, ProductCapabilityInventory};

    #[test]
    fn deterministic_selection_needs_no_provider_and_excludes_unselected_invokers() {
        let contributor = Arc::new(DocsCapabilityContributor);
        let inventory = ProductCapabilityInventory::assemble(vec![contributor]).unwrap();
        let contract = inventory
            .contracts()
            .find(|revision| revision.contract.capability_type_id == INSPECT_SCOPE)
            .unwrap()
            .revision_ref();
        let implementation = format!("docs.in-process.v1::{INSPECT_SCOPE}");
        let closure = inventory
            .prepare(
                ExactCapabilityActivationRequest {
                    assignment_id: "assignment-a".to_string(),
                    activation_id: "activation-a".to_string(),
                    selected_contracts: vec![contract.clone()],
                    selected_implementations: BTreeMap::from([(contract, implementation)]),
                    compatibility_policy_revision: "capability-compatibility.v1".into(),
                },
                &OwnerBindingView::new(BTreeMap::from([
                    (WORKSPACE_BINDING.to_string(), "/tmp".to_string()),
                    (SUBJECT_BINDING.to_string(), "docs".to_string()),
                    (AGENT_BINDING.to_string(), "agent".to_string()),
                ])),
            )
            .unwrap();

        assert!(closure.contracts.contains(INSPECT_SCOPE, 1));
        assert!(closure.invokers.get(INSPECT_SCOPE, 1).is_some());
        assert!(!closure.contracts.contains(DRAFT_PATCH_SET, 1));
        assert!(closure.invokers.get(DRAFT_PATCH_SET, 1).is_none());
    }

    #[test]
    fn provider_backed_selection_requires_only_its_declared_provider_binding() {
        let contributor = Arc::new(DocsCapabilityContributor);
        let inventory = ProductCapabilityInventory::assemble(vec![contributor]).unwrap();
        let contract = inventory
            .contracts()
            .find(|revision| revision.contract.capability_type_id == DRAFT_PATCH_SET)
            .unwrap()
            .revision_ref();
        let error = inventory
            .prepare(
                ExactCapabilityActivationRequest {
                    assignment_id: "assignment-a".to_string(),
                    activation_id: "activation-a".to_string(),
                    selected_contracts: vec![contract.clone()],
                    selected_implementations: BTreeMap::from([(
                        contract,
                        format!("docs.in-process.v1::{DRAFT_PATCH_SET}"),
                    )]),
                    compatibility_policy_revision: "capability-compatibility.v1".into(),
                },
                &OwnerBindingView::new(BTreeMap::from([
                    (WORKSPACE_BINDING.to_string(), "/tmp".to_string()),
                    (SUBJECT_BINDING.to_string(), "docs".to_string()),
                    (AGENT_BINDING.to_string(), "agent".to_string()),
                ])),
            )
            .err()
            .unwrap();
        assert_eq!(error.code, "selected_binding_missing");
    }

    #[test]
    fn product_inventory_is_deterministic() {
        let first = ProductCapabilityInventory::assemble(vec![Arc::new(DocsCapabilityContributor)])
            .unwrap();
        let second =
            ProductCapabilityInventory::assemble(vec![Arc::new(DocsCapabilityContributor)])
                .unwrap();
        let first_refs: Vec<_> = first.contracts().map(|item| item.revision_ref()).collect();
        let second_refs: Vec<_> = second.contracts().map(|item| item.revision_ref()).collect();
        assert_eq!(first_refs, second_refs);
    }

    #[test]
    fn shipped_execution_classes_are_preserved() {
        let contributor = DocsCapabilityContributor;
        assert!(contributor
            .implementation_offers()
            .iter()
            .any(|offer| offer.execution_class == ExecutionClass::Queued));
    }

    #[test]
    fn policy_dependent_invokers_require_exact_bound_revision() {
        let inventory =
            ProductCapabilityInventory::assemble(vec![Arc::new(DocsCapabilityContributor)])
                .unwrap();
        let db = sled::Config::new().temporary(true).open().unwrap();
        let store = crate::docs::claim_validation::DocsClaimPolicyRegistryStore::new(db).unwrap();
        let policy: DocsClaimPolicy = serde_json::from_str(include_str!(
            "../../theory/docs_freshness/claim_policy.docs-claims-strict-v1.json"
        ))
        .unwrap();
        let (_, first) = store.install(policy.clone(), 1).unwrap();
        let mut alternate = policy;
        alternate.minimum_claim_confidence = 0.99;
        let (_, second) = store.install(alternate, 2).unwrap();
        let physical = OwnerBindingView::new(BTreeMap::from([
            (WORKSPACE_BINDING.into(), "/tmp".into()),
            (SUBJECT_BINDING.into(), "docs".into()),
            (AGENT_BINDING.into(), "agent".into()),
            (PROVIDER_BINDING.into(), "provider".into()),
        ]));
        for capability in [VALIDATE_PATCH_SET, PUBLISH_PATCH_SET] {
            let contract = inventory
                .contracts()
                .find(|revision| revision.contract.capability_type_id == capability)
                .unwrap()
                .revision_ref();
            let request = ExactCapabilityActivationRequest {
                assignment_id: "assignment".into(),
                activation_id: "activation".into(),
                selected_contracts: vec![contract.clone()],
                selected_implementations: BTreeMap::from([(
                    contract,
                    format!("docs.in-process.v1::{capability}"),
                )]),
                compatibility_policy_revision: "capability-compatibility.v1".into(),
            };
            assert_eq!(
                inventory
                    .prepare(request.clone(), &physical)
                    .err()
                    .unwrap()
                    .code,
                "selected_binding_missing"
            );
            let first_binding = bind_claim_policy(physical.clone(), &first).unwrap();
            let second_binding = bind_claim_policy(physical.clone(), &second).unwrap();
            assert_eq!(selected_claim_policy(&first_binding).unwrap(), first.policy);
            assert_eq!(
                selected_claim_policy(&second_binding).unwrap(),
                second.policy
            );
            let first_prepared = inventory.prepare(request.clone(), &first_binding).unwrap();
            let second_prepared = inventory.prepare(request.clone(), &second_binding).unwrap();
            assert_ne!(
                first_prepared.preparation_receipt,
                second_prepared.preparation_receipt
            );
            assert!(first_prepared.invokers.get(capability, 1).is_some());
            let mut corrupt = first.clone();
            corrupt.policy.minimum_claim_confidence = 0.7;
            assert!(bind_claim_policy(physical.clone(), &corrupt).is_err());
            let corrupt_binding = physical.clone().with_value(
                CLAIM_POLICY_BINDING,
                serde_json::to_string(&corrupt).unwrap(),
            );
            assert_eq!(
                inventory
                    .prepare(request, &corrupt_binding)
                    .err()
                    .unwrap()
                    .code,
                "selected_binding_invalid"
            );
        }
    }

    #[test]
    fn shipped_theory_source_hashes_are_visible_for_manifest_review() {
        for path in [
            "theory/docs_freshness/belief_family.docs_freshness.json",
            "theory/docs_freshness/outcome_interpretation.docs_freshness.json",
            "theory/docs_freshness/curation_rule.docs_freshness.json",
            "theory/docs_freshness/maintained_condition.docs_freshness.json",
            "theory/docs_freshness/strategy_theory.docs_freshness.json",
            "theory/docs_freshness/authority_policy.docs_workspace_local.json",
            "theory/docs_freshness/claim_policy.docs-claims-strict-v1.json",
        ] {
            let bytes =
                std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap();
            println!("{}  {path}", blake3::hash(&bytes).to_hex());
        }
    }
}
