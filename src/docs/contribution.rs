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
use crate::docs::claim_validation::DocsClaimPolicy;
use crate::error::ApiError;
use crate::execution::ExecutionRuntimeContext;
use crate::provider::{ProviderExecutionBinding, ProviderRuntimeOverrides};

const WORKSPACE_BINDING: &str = "workspace";
const PROVIDER_BINDING: &str = "provider";
const SUBJECT_BINDING: &str = "subject";
const AGENT_BINDING: &str = "agent";

pub struct DocsCapabilityContributor {
    policy: DocsClaimPolicy,
}

impl DocsCapabilityContributor {
    pub fn shipped() -> Self {
        Self {
            policy: serde_json::from_str(include_str!(
                "../../theory/docs_freshness/claim_policy.docs-claims-strict-v1.json"
            ))
            .expect("shipped docs claim policy is valid JSON"),
        }
    }
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
                CapabilityImplementationOffer {
                    contract_ref: revision.revision_ref(),
                    implementation_ref: format!("docs.in-process.v1::{type_id}"),
                    required_binding_ids,
                    execution_class: revision.contract.execution_contract.execution_class,
                    factory: Arc::new(DocsInvokerFactory {
                        capability_type_id: type_id,
                        policy: self.policy.clone(),
                    }),
                }
            })
            .collect()
    }
}

struct DocsInvokerFactory {
    capability_type_id: String,
    policy: DocsClaimPolicy,
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
            VALIDATE_PATCH_SET => {
                Arc::new(ValidatePatchSetCapability::new(config, self.policy.clone()))
            }
            PUBLISH_PATCH_SET => {
                Arc::new(PublishPatchSetCapability::new(config, self.policy.clone()))
            }
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
        let contributor = Arc::new(DocsCapabilityContributor::shipped());
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
        let contributor = Arc::new(DocsCapabilityContributor::shipped());
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
        let first = ProductCapabilityInventory::assemble(vec![Arc::new(
            DocsCapabilityContributor::shipped(),
        )])
        .unwrap();
        let second = ProductCapabilityInventory::assemble(vec![Arc::new(
            DocsCapabilityContributor::shipped(),
        )])
        .unwrap();
        let first_refs: Vec<_> = first.contracts().map(|item| item.revision_ref()).collect();
        let second_refs: Vec<_> = second.contracts().map(|item| item.revision_ref()).collect();
        assert_eq!(first_refs, second_refs);
    }

    #[test]
    fn shipped_execution_classes_are_preserved() {
        let contributor = DocsCapabilityContributor::shipped();
        assert!(contributor
            .implementation_offers()
            .iter()
            .any(|offer| offer.execution_class == ExecutionClass::Queued));
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
