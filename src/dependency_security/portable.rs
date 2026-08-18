//! Portable local and serialized execution over one owner admission contract.

use meld_execution::capability::CapabilityContractRevisionRef;

use super::adapter::{DependencySecurityAdapterResultV1, DependencySecurityTruthAdapter};
use super::admission::{
    AdmittedOperationContext, CanonicalProductRef, DependencySecurityAdmission,
};
use crate::runtime::delivery::LateResultDisposition;
use crate::runtime::lifecycle::{
    ActivationLifecycleStore, DurableOperationStatus, DurableOperationV1,
};

#[allow(clippy::too_many_arguments)]
pub fn execute_fixture_truth_operation(
    lifecycle: &ActivationLifecycleStore,
    admission: &DependencySecurityAdmission,
    adapter: &dyn DependencySecurityTruthAdapter,
    contract_ref: CapabilityContractRevisionRef,
    incarnation_id: String,
    mut proposed: DependencySecurityAdapterResultV1,
) -> Result<CanonicalProductRef, String> {
    lifecycle
        .persist_operation(DurableOperationV1 {
            operation_key: proposed.operation_key.clone(),
            assignment_id: proposed.assignment_id.clone(),
            generation_id: proposed.activation_generation_id.clone(),
            capability_contract_ref: contract_ref,
            semantic_request_hash: blake3::hash(proposed.response_identity.as_bytes())
                .to_hex()
                .to_string(),
            status: DurableOperationStatus::Pending,
            admitted_product_refs: vec![],
        })
        .map_err(|e| e.to_string())?;
    let attempt = lifecycle
        .begin_attempt(
            &proposed.operation_key,
            incarnation_id,
            "fixture-dispatch-claim".into(),
            1,
        )
        .map_err(|e| e.to_string())?;
    proposed.attempt_id = attempt.attempt_id;
    proposed.response_identity = super::contracts::content_hash(&(
        &proposed.adapter_implementation_ref,
        &proposed.source_revisions,
        &proposed.value,
    ))?;
    let result = adapter.transport(&proposed)?;
    let product = admission.admit(
        &AdmittedOperationContext {
            assignment_id: result.assignment_id.clone(),
            package_receipt_id: result.package_receipt_id.clone(),
            activation_id: result.activation_id.clone(),
            generation_id: result.activation_generation_id.clone(),
            operation_key: result.operation_key.clone(),
            attempt_id: result.attempt_id.clone(),
            adapter_implementation_ref: result.adapter_implementation_ref.clone(),
            active: true,
        },
        &result,
    )?;
    lifecycle
        .complete_operation(
            &result.operation_key,
            product.product_id.clone(),
            &result.activation_generation_id,
            LateResultDisposition::Reject,
        )
        .map_err(|e| e.to_string())?;
    Ok(product)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency_security::adapter::*;
    use crate::dependency_security::contracts::*;
    use meld_lang::CapabilityRef;
    fn proposed(implementation: &str) -> DependencySecurityAdapterResultV1 {
        DependencySecurityAdapterResultV1 {
            assignment_id: "assignment".into(),
            package_receipt_id: "package".into(),
            activation_id: "activation".into(),
            activation_generation_id: "generation".into(),
            operation_key: format!("operation-{implementation}"),
            attempt_id: String::new(),
            adapter_implementation_ref: implementation.into(),
            response_identity: String::new(),
            observed_scope: vec![],
            source_revisions: vec!["r1".into()],
            completeness: AdapterCompleteness::ExplicitComplete,
            value: DependencySecurityAdapterValueV1::Advisories(AdvisoryKnowledgeSnapshotV1 {
                snapshot_id: "canonical-advisories".into(),
                source_id: "fixture".into(),
                source_revision: "r1".into(),
                covered_ecosystem: PackageEcosystem::Cargo,
                covered_components: vec![],
                advisories: vec![],
                conflicts: vec![],
                completeness: AdvisoryCompleteness::CompleteForDeclaredCoverage,
                acquired_at: 1,
            }),
        }
    }
    fn contract() -> CapabilityContractRevisionRef {
        CapabilityContractRevisionRef {
            selector: CapabilityRef {
                capability_type_id: "dependency_security.acquire_advisories".into(),
                capability_version: 1,
            },
            content_identity: "contract".into(),
        }
    }
    #[test]
    fn local_and_serialized_products_are_equivalent() {
        let run = |adapter: &dyn DependencySecurityTruthAdapter, implementation: &str| {
            let lifecycle =
                ActivationLifecycleStore::new(sled::Config::new().temporary(true).open().unwrap())
                    .unwrap();
            lifecycle
                .publish_head("assignment", None, "generation")
                .unwrap();
            execute_fixture_truth_operation(
                &lifecycle,
                &DependencySecurityAdmission::default(),
                adapter,
                contract(),
                "incarnation".into(),
                proposed(implementation),
            )
            .unwrap()
        };
        let local = run(&LocalFixtureAdapter, "local");
        let serialized = run(&FakeSerializedAdapter, "serialized");
        assert_eq!(local, serialized);
    }
}
