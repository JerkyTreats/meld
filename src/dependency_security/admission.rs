use std::collections::BTreeMap;
use std::sync::Mutex;

use super::adapter::{
    AdapterCompleteness, DependencySecurityAdapterResultV1, DependencySecurityAdapterValueV1,
};
use super::contracts::content_hash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedOperationContext {
    pub assignment_id: String,
    pub package_receipt_id: String,
    pub activation_id: String,
    pub generation_id: String,
    pub operation_key: String,
    pub attempt_id: String,
    pub adapter_implementation_ref: String,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalProductRef {
    pub product_kind: String,
    pub product_id: String,
}

#[derive(Default)]
pub struct DependencySecurityAdmission {
    admitted: Mutex<BTreeMap<(String, String), CanonicalProductRef>>,
}

impl DependencySecurityAdmission {
    pub fn admit(
        &self,
        context: &AdmittedOperationContext,
        result: &DependencySecurityAdapterResultV1,
    ) -> Result<CanonicalProductRef, String> {
        if !context.active
            || context.assignment_id != result.assignment_id
            || context.package_receipt_id != result.package_receipt_id
            || context.activation_id != result.activation_id
            || context.generation_id != result.activation_generation_id
            || context.operation_key != result.operation_key
            || context.attempt_id != result.attempt_id
            || context.adapter_implementation_ref != result.adapter_implementation_ref
        {
            return Err("adapter result lineage is not admitted".into());
        }
        if !matches!(
            result.completeness,
            AdapterCompleteness::ExplicitComplete | AdapterCompleteness::ExplicitIncomplete
        ) {
            return Err("adapter result completeness is absent".into());
        }
        let (kind, id) = match &result.value {
            DependencySecurityAdapterValueV1::Inventory(value) => {
                ("inventory", value.snapshot_id.clone())
            }
            DependencySecurityAdapterValueV1::Advisories(value) => {
                ("advisories", value.snapshot_id.clone())
            }
        };
        let expected_response = content_hash(&(
            &result.adapter_implementation_ref,
            &result.source_revisions,
            &result.value,
        ))?;
        if expected_response != result.response_identity {
            return Err("adapter response identity mismatch".into());
        }
        let product = CanonicalProductRef {
            product_kind: kind.into(),
            product_id: id,
        };
        let key = (
            result.operation_key.clone(),
            result.response_identity.clone(),
        );
        let mut admitted = self
            .admitted
            .lock()
            .map_err(|_| "admission lock poisoned".to_string())?;
        if let Some(existing) = admitted.get(&key) {
            if existing != &product {
                return Err("conflicting duplicate adapter response".into());
            }
            return Ok(existing.clone());
        }
        admitted.insert(key, product.clone());
        Ok(product)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency_security::adapter::*;
    use crate::dependency_security::contracts::*;
    fn result() -> DependencySecurityAdapterResultV1 {
        let value = DependencySecurityAdapterValueV1::Advisories(AdvisoryKnowledgeSnapshotV1 {
            snapshot_id: "product".into(),
            source_id: "fixture".into(),
            source_revision: "s".into(),
            covered_ecosystem: PackageEcosystem::Cargo,
            covered_components: vec![],
            advisories: vec![],
            conflicts: vec![],
            completeness: AdvisoryCompleteness::CompleteForDeclaredCoverage,
            acquired_at: 1,
        });
        DependencySecurityAdapterResultV1 {
            assignment_id: "a".into(),
            package_receipt_id: "p".into(),
            activation_id: "x".into(),
            activation_generation_id: "g".into(),
            operation_key: "o".into(),
            attempt_id: "t".into(),
            adapter_implementation_ref: "local".into(),
            response_identity: content_hash(&("local", &vec!["s".to_string()], &value)).unwrap(),
            observed_scope: vec![],
            source_revisions: vec!["s".into()],
            completeness: AdapterCompleteness::ExplicitComplete,
            value,
        }
    }
    fn context() -> AdmittedOperationContext {
        AdmittedOperationContext {
            assignment_id: "a".into(),
            package_receipt_id: "p".into(),
            activation_id: "x".into(),
            generation_id: "g".into(),
            operation_key: "o".into(),
            attempt_id: "t".into(),
            adapter_implementation_ref: "local".into(),
            active: true,
        }
    }
    #[test]
    fn transport_success_is_not_canonical() {
        let admission = DependencySecurityAdmission::default();
        let mut bad = result();
        bad.attempt_id = "other".into();
        assert!(admission.admit(&context(), &bad).is_err());
    }
    #[test]
    fn tampered_or_duplicate_conflicting_response_is_rejected() {
        let admission = DependencySecurityAdmission::default();
        let exact = result();
        assert_eq!(
            admission.admit(&context(), &exact).unwrap(),
            admission.admit(&context(), &exact).unwrap()
        );
        let mut bad = exact;
        bad.response_identity = "tampered".into();
        assert!(admission.admit(&context(), &bad).is_err());
    }
}
