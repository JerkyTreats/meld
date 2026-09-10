use serde::{Deserialize, Serialize};

use super::contracts::{content_hash, PackageEcosystem, SeverityV1};
use crate::theory::TheoryRevisionRef;

pub const REGISTRY: &str = "dependency_security_policy_v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageContractV2 {
    CompleteTransitiveAllComponents,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrencyRequirementV1 {
    pub maximum_source_age_seconds: u64,
    pub maximum_inventory_age_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationContractV2 {
    IndependentRecalculation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencySecurityPolicyV2 {
    pub policy_id: String,
    pub subject_kind: String,
    pub ecosystem: PackageEcosystem,
    pub required_advisory_source_id: String,
    pub coverage_contract: CoverageContractV2,
    pub currency: CurrencyRequirementV1,
    pub severity_threshold: SeverityV1,
    pub verification_contract: VerificationContractV2,
}

impl DependencySecurityPolicyV2 {
    pub fn validate(&self) -> Result<(), String> {
        if self.policy_id.trim().is_empty()
            || self.subject_kind.trim().is_empty()
            || self.required_advisory_source_id.trim().is_empty()
        {
            return Err("dependency-security policy identity is incomplete".into());
        }
        Ok(())
    }

    pub fn revision_ref(&self) -> Result<TheoryRevisionRef, String> {
        self.validate()?;
        Ok(TheoryRevisionRef {
            registry: REGISTRY.into(),
            id: self.policy_id.clone(),
            content_hash: content_hash(self)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> DependencySecurityPolicyV2 {
        DependencySecurityPolicyV2 {
            policy_id: "cargo-fixture".into(),
            subject_kind: "workspace".into(),
            ecosystem: PackageEcosystem::Cargo,
            required_advisory_source_id: "fixture".into(),
            coverage_contract: CoverageContractV2::CompleteTransitiveAllComponents,
            currency: CurrencyRequirementV1 {
                maximum_source_age_seconds: 100,
                maximum_inventory_age_seconds: 100,
            },
            severity_threshold: SeverityV1::High,
            verification_contract: VerificationContractV2::IndependentRecalculation,
        }
    }

    #[test]
    fn policy_is_state_free_and_exact() {
        let policy = policy();
        assert_eq!(
            policy.revision_ref().unwrap(),
            policy.revision_ref().unwrap()
        );
        assert_eq!(policy.revision_ref().unwrap().registry, REGISTRY);
    }

    #[test]
    fn legacy_boolean_policy_is_incompatible_with_the_v2_contract() {
        let legacy = serde_json::json!({
            "policy_id": "cargo-fixture",
            "subject_kind": "workspace",
            "ecosystem": "cargo",
            "required_advisory_source_id": "fixture",
            "required_coverage": {
                "require_complete_inventory": true,
                "require_all_components_covered": true,
                "require_transitive_dependencies": true
            },
            "currency": {
                "maximum_source_age_seconds": 100,
                "maximum_inventory_age_seconds": 100
            },
            "severity_threshold": "high",
            "verification": { "require_independent_calculation": true }
        });
        assert!(serde_json::from_value::<DependencySecurityPolicyV2>(legacy).is_err());
    }

    #[test]
    fn fixed_contract_rejects_unsupported_modes() {
        let mut value = serde_json::to_value(policy()).unwrap();
        value["coverage_contract"] = "direct_dependencies_only".into();
        assert!(serde_json::from_value::<DependencySecurityPolicyV2>(value).is_err());

        let mut value = serde_json::to_value(policy()).unwrap();
        value["verification_contract"] = "trust_assessment".into();
        assert!(serde_json::from_value::<DependencySecurityPolicyV2>(value).is_err());
    }
}
