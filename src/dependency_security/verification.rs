use super::assessment::assess;
use super::contracts::*;
use super::policy::DependencySecurityPolicyV1;

pub fn verify(
    assessment: &DependencySecurityAssessmentV1,
    inventory: &DependencyInventorySnapshotV1,
    advisory: &AdvisoryKnowledgeSnapshotV1,
    policy: &DependencySecurityPolicyV1,
) -> Result<DependencySecurityVerificationV1, String> {
    let recomputed = assess(
        &assessment.subject,
        Some(inventory),
        Some(advisory),
        policy,
        assessment.reference_time,
    )?;
    let finding_refs = recomputed
        .findings
        .iter()
        .map(|item| item.finding_id.clone())
        .collect::<Vec<_>>();
    let expected_refs = assessment
        .findings
        .iter()
        .map(|item| item.finding_id.clone())
        .collect::<Vec<_>>();
    let exact_inputs = assessment.inventory_snapshot_ref == inventory.snapshot_id
        && assessment.advisory_snapshot_ref == advisory.snapshot_id
        && assessment.policy_revision == policy.revision_ref()?;
    let verified = exact_inputs
        && recomputed.posture == assessment.posture
        && finding_refs == expected_refs
        && recomputed.coverage == assessment.coverage
        && recomputed.reasons == assessment.reasons;
    let checks = vec![
        if exact_inputs {
            "exact_inputs_match"
        } else {
            "exact_inputs_changed"
        }
        .into(),
        if verified {
            "calculation_matches"
        } else {
            "calculation_differs"
        }
        .into(),
    ];
    let identity = (
        &assessment.assessment_id,
        &inventory.snapshot_id,
        &advisory.snapshot_id,
        &assessment.policy_revision,
        &recomputed.posture,
        &finding_refs,
        &checks,
        verified,
    );
    Ok(DependencySecurityVerificationV1 {
        verification_id: content_hash(&identity)?,
        assessment_ref: assessment.assessment_id.clone(),
        inventory_snapshot_ref: inventory.snapshot_id.clone(),
        advisory_snapshot_ref: advisory.snapshot_id.clone(),
        policy_revision: assessment.policy_revision.clone(),
        independently_computed_posture: recomputed.posture,
        independently_computed_finding_refs: finding_refs,
        checks,
        verified,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency_security::policy::{
        CoverageRequirementV1, CurrencyRequirementV1, VerificationRequirementV1,
    };
    use meld_events::DomainObjectRef;

    #[test]
    fn verification_recomputes_exact_inputs() {
        let subject = DependencySecuritySubjectV1 {
            subject: DomainObjectRef::new("workspace_fs", "node", "repo").unwrap(),
            ecosystem: PackageEcosystem::Cargo,
            inventory_scope: InventoryScopeV1 {
                manifest_ref: DomainObjectRef::new("workspace_fs", "file", "Cargo.toml").unwrap(),
                lockfile_ref: DomainObjectRef::new("workspace_fs", "file", "Cargo.lock").unwrap(),
                include_transitive: true,
            },
        };
        let inventory = DependencyInventorySnapshotV1::canonical(
            subject.clone(),
            "revision".into(),
            "manifest".into(),
            "lock".into(),
            vec![],
            InventoryCompleteness::CompleteTransitive,
            10,
        )
        .unwrap();
        let advisory = AdvisoryKnowledgeSnapshotV1::canonical(
            "fixture".into(),
            "r1".into(),
            PackageEcosystem::Cargo,
            vec![],
            vec![],
            vec![],
            AdvisoryCompleteness::CompleteForDeclaredCoverage,
            10,
        )
        .unwrap();
        let policy = DependencySecurityPolicyV1 {
            policy_id: "policy".into(),
            subject_kind: "workspace".into(),
            ecosystem: PackageEcosystem::Cargo,
            required_advisory_source_id: "fixture".into(),
            required_coverage: CoverageRequirementV1 {
                require_complete_inventory: true,
                require_all_components_covered: true,
                require_transitive_dependencies: true,
            },
            currency: CurrencyRequirementV1 {
                maximum_source_age_seconds: 10,
                maximum_inventory_age_seconds: 10,
            },
            severity_threshold: SeverityV1::High,
            verification: VerificationRequirementV1 {
                require_independent_calculation: true,
            },
        };
        let assessment = assess(&subject, Some(&inventory), Some(&advisory), &policy, 10).unwrap();
        assert!(
            verify(&assessment, &inventory, &advisory, &policy)
                .unwrap()
                .verified
        );
    }
}
