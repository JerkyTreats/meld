use serde::{Deserialize, Serialize};

use super::contracts::{content_hash, PackageEcosystem, SeverityV1};
use crate::theory::TheoryRevisionRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageRequirementV1 {
    pub require_complete_inventory: bool,
    pub require_all_components_covered: bool,
    pub require_transitive_dependencies: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrencyRequirementV1 {
    pub maximum_source_age_seconds: u64,
    pub maximum_inventory_age_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationRequirementV1 {
    pub require_independent_calculation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencySecurityPolicyV1 {
    pub policy_id: String,
    pub subject_kind: String,
    pub ecosystem: PackageEcosystem,
    pub required_advisory_source_id: String,
    pub required_coverage: CoverageRequirementV1,
    pub currency: CurrencyRequirementV1,
    pub severity_threshold: SeverityV1,
    pub verification: VerificationRequirementV1,
}

impl DependencySecurityPolicyV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.policy_id.trim().is_empty()
            || self.subject_kind.trim().is_empty()
            || self.required_advisory_source_id.trim().is_empty()
        {
            return Err("dependency-security policy identity is incomplete".into());
        }
        if !self.required_coverage.require_complete_inventory
            || !self.required_coverage.require_all_components_covered
            || !self.required_coverage.require_transitive_dependencies
        {
            return Err("authorized policy requires complete transitive coverage".into());
        }
        Ok(())
    }

    pub fn revision_ref(&self) -> Result<TheoryRevisionRef, String> {
        self.validate()?;
        Ok(TheoryRevisionRef {
            registry: "dependency_security_policy".into(),
            id: self.policy_id.clone(),
            content_hash: content_hash(self)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn policy_is_state_free_and_exact() {
        let policy = DependencySecurityPolicyV1 {
            policy_id: "cargo-fixture".into(),
            subject_kind: "workspace".into(),
            ecosystem: PackageEcosystem::Cargo,
            required_advisory_source_id: "fixture".into(),
            required_coverage: CoverageRequirementV1 {
                require_complete_inventory: true,
                require_all_components_covered: true,
                require_transitive_dependencies: true,
            },
            currency: CurrencyRequirementV1 {
                maximum_source_age_seconds: 100,
                maximum_inventory_age_seconds: 100,
            },
            severity_threshold: SeverityV1::High,
            verification: VerificationRequirementV1 {
                require_independent_calculation: true,
            },
        };
        assert_eq!(
            policy.revision_ref().unwrap(),
            policy.revision_ref().unwrap()
        );
    }
}
