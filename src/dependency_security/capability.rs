//! Four read-and-emit dependency-security proof capabilities.

use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use meld_execution::capability::{CapabilityContractRevision, CapabilityInvoker};

use crate::capability::*;
use crate::error::ApiError;
use crate::execution::{ExecutionEventContext, ExecutionRuntimeContext};

pub const OBSERVE_INVENTORY: &str = "dependency_security.observe_inventory";
pub const ACQUIRE_ADVISORIES: &str = "dependency_security.acquire_advisories";
pub const ASSESS: &str = "dependency_security.assess";
pub const VERIFY: &str = "dependency_security.verify";
const VERSION: u32 = 1;

pub fn published_contracts() -> Vec<CapabilityTypeContract> {
    [OBSERVE_INVENTORY, ACQUIRE_ADVISORIES, ASSESS, VERIFY]
        .into_iter()
        .map(contract)
        .collect()
}

fn contract(id: &str) -> CapabilityTypeContract {
    CapabilityTypeContract {
        capability_type_id: id.into(),
        capability_version: VERSION,
        owning_domain: "dependency-security".into(),
        scope_contract: ScopeContract {
            scope_kind: "cargo_dependency_graph".into(),
            scope_ref_kind: "subject_id".into(),
            allow_fan_out: false,
        },
        binding_contract: vec![],
        input_contract: vec![],
        output_contract: vec![OutputSlotSpec {
            slot_id: "canonical_product".into(),
            artifact_type_id: "dependency_security_product".into(),
            schema_version: 1,
            guaranteed: true,
        }],
        effect_contract: vec![EffectSpec {
            effect_id: format!("{id}.emit"),
            kind: EffectKind::Emit,
            target: "dependency_security_admission".into(),
            exclusive: false,
        }],
        execution_contract: ExecutionContract {
            execution_class: ExecutionClass::Inline,
            completion_semantics: "owner_admitted_product_or_failure".into(),
            retry_class: "deterministic_fixture".into(),
            cancellation_supported: false,
        },
    }
}

pub struct DependencySecurityCapabilityContributor;
impl ProductCapabilityContributor for DependencySecurityCapabilityContributor {
    fn owner_domain(&self) -> &str {
        "dependency-security"
    }
    fn published_contracts(&self) -> Vec<CapabilityContractRevision> {
        published_contracts()
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
                let contract = revision.contract.clone();
                CapabilityImplementationOffer {
                    contract_ref: revision.revision_ref(),
                    implementation_ref: format!(
                        "dependency-security.fixture-local.v1::{}",
                        contract.capability_type_id
                    ),
                    required_binding_ids: BTreeSet::from(["fixture".into()]),
                    execution_class: ExecutionClass::Inline,
                    factory: Arc::new(FixtureFactory { contract }),
                }
            })
            .collect()
    }
}

struct FixtureFactory {
    contract: CapabilityTypeContract,
}
impl CapabilityInvokerFactory for FixtureFactory {
    fn prepare(
        &self,
        _request: &CapabilityFactoryRequest<'_>,
        bindings: &OwnerBindingView,
    ) -> Result<PreparedCapabilityInvoker, CapabilityContributionDiagnostic> {
        if !bindings.contains("fixture") {
            return Err(CapabilityContributionDiagnostic::new(
                "selected_binding_missing",
                "dependency-security implementation requires a fixture binding",
            ));
        }
        Ok(Arc::new(FixtureInvoker {
            contract: self.contract.clone(),
        }))
    }
}
struct FixtureInvoker {
    contract: CapabilityTypeContract,
}
#[async_trait]
impl CapabilityInvoker for FixtureInvoker {
    type Error = ApiError;
    type ExecutionApi = dyn ExecutionRuntimeContext;
    fn contract(&self) -> CapabilityTypeContract {
        self.contract.clone()
    }
    async fn invoke(
        &self,
        _api: &dyn ExecutionRuntimeContext,
        runtime_init: &CapabilityRuntimeInit,
        payload: &CapabilityInvocationPayload,
        _event_context: Option<&ExecutionEventContext>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        payload.validate_against(runtime_init)?;
        Ok(CapabilityInvocationResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capabilities_are_read_and_emit_only() {
        for contract in published_contracts() {
            println!(
                "{} {}",
                contract.capability_type_id,
                contract.content_identity()
            );
            assert_eq!(contract.owning_domain, "dependency-security");
            assert!(contract
                .effect_contract
                .iter()
                .all(|effect| matches!(effect.kind, EffectKind::Emit)));
        }
    }
}
