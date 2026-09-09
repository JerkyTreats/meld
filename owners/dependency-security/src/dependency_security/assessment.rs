use std::collections::BTreeSet;

use serde::Serialize;

use super::contracts::*;
use super::policy::DependencySecurityPolicyV1;

#[derive(Serialize)]
struct Identity<'a> {
    subject: &'a DependencySecuritySubjectV1,
    inventory_snapshot_ref: &'a str,
    advisory_snapshot_ref: &'a str,
    policy_revision: &'a crate::theory::TheoryRevisionRef,
    reference_time: u64,
    posture: &'a DependencySecurityPosture,
    findings: &'a [SecurityFindingV1],
    coverage: &'a AssessmentCoverageV1,
    reasons: &'a [String],
}

pub fn assess(
    subject: &DependencySecuritySubjectV1,
    inventory: Option<&DependencyInventorySnapshotV1>,
    advisory: Option<&AdvisoryKnowledgeSnapshotV1>,
    policy: &DependencySecurityPolicyV1,
    reference_time: u64,
) -> Result<DependencySecurityAssessmentV1, String> {
    policy.validate()?;
    if let Some(inventory) = inventory {
        inventory.validate()?;
    }
    if let Some(advisory) = advisory {
        advisory.validate()?;
    }
    let policy_revision = policy.revision_ref()?;
    let mut reasons = Vec::new();
    let mut findings = Vec::new();
    let mut coverage = AssessmentCoverageV1 {
        component_count: 0,
        covered_component_count: 0,
    };
    let mut inventory_ref = String::new();
    let mut advisory_ref = String::new();
    let posture = match (inventory, advisory) {
        (Some(inventory), Some(advisory)) => {
            inventory_ref = inventory.snapshot_id.clone();
            advisory_ref = advisory.snapshot_id.clone();
            coverage.component_count = inventory.components.len();
            let covered: BTreeSet<_> = advisory
                .covered_components
                .iter()
                .map(|item| (item.package_name.as_str(), item.source_identity.as_str()))
                .collect();
            coverage.covered_component_count = inventory
                .components
                .iter()
                .filter(|item| {
                    covered.contains(&(item.package_name.as_str(), item.source_identity.as_str()))
                })
                .count();
            for component in &inventory.components {
                for item in advisory.advisories.iter().filter(|item| {
                    item.package_name == component.package_name
                        && item.affected_versions.contains(&component.resolved_version)
                        && item.severity.rank() >= policy.severity_threshold.rank()
                }) {
                    findings.push(SecurityFindingV1 {
                        finding_id: content_hash(&(
                            item.source_advisory_id.as_str(),
                            component.component_id.as_str(),
                        ))?,
                        advisory_id: item.source_advisory_id.clone(),
                        component_id: component.component_id.clone(),
                        severity: item.severity.clone(),
                    });
                }
            }
            findings.sort_by(|a, b| a.finding_id.cmp(&b.finding_id));
            if inventory.subject != *subject
                || advisory.source_id != policy.required_advisory_source_id
            {
                reasons.push("required exact input does not match subject or source".into());
                DependencySecurityPosture::Unknown
            } else if !advisory.conflicts.is_empty() {
                reasons.extend(advisory.conflicts.clone());
                DependencySecurityPosture::Conflicted
            } else if inventory.observed_at > reference_time
                || advisory.acquired_at > reference_time
            {
                reasons
                    .push("required evidence is dated after the assessment reference time".into());
                DependencySecurityPosture::Unknown
            } else if reference_time.saturating_sub(inventory.observed_at)
                > policy.currency.maximum_inventory_age_seconds
                || reference_time.saturating_sub(advisory.acquired_at)
                    > policy.currency.maximum_source_age_seconds
            {
                reasons.push("required evidence exceeds policy currency".into());
                DependencySecurityPosture::Stale
            } else if !matches!(
                inventory.completeness,
                InventoryCompleteness::CompleteTransitive
            ) || !matches!(
                advisory.completeness,
                AdvisoryCompleteness::CompleteForDeclaredCoverage
            ) || coverage.covered_component_count != coverage.component_count
            {
                reasons.push("required complete transitive coverage is absent".into());
                DependencySecurityPosture::Insufficient
            } else if !findings.is_empty() {
                reasons.push("one or more threshold findings apply".into());
                DependencySecurityPosture::Violated
            } else {
                reasons.push("complete current coverage contains no threshold finding".into());
                DependencySecurityPosture::CleanWithinCoverage
            }
        }
        _ => {
            reasons.push("required exact inventory or advisory input is absent".into());
            DependencySecurityPosture::Unknown
        }
    };
    let identity = Identity {
        subject,
        inventory_snapshot_ref: &inventory_ref,
        advisory_snapshot_ref: &advisory_ref,
        policy_revision: &policy_revision,
        reference_time,
        posture: &posture,
        findings: &findings,
        coverage: &coverage,
        reasons: &reasons,
    };
    let assessment_id = content_hash(&identity)?;
    Ok(DependencySecurityAssessmentV1 {
        assessment_id,
        subject: subject.clone(),
        inventory_snapshot_ref: inventory_ref,
        advisory_snapshot_ref: advisory_ref,
        policy_revision,
        reference_time,
        posture,
        findings,
        coverage,
        reasons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency_security::policy::*;
    use meld_events::DomainObjectRef;

    fn policy() -> DependencySecurityPolicyV1 {
        DependencySecurityPolicyV1 {
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
        }
    }
    fn subject() -> DependencySecuritySubjectV1 {
        DependencySecuritySubjectV1 {
            assignment_scope_id: "fixture-assignment".into(),
            subject: DomainObjectRef::new("workspace_fs", "node", "repo").unwrap(),
            ecosystem: PackageEcosystem::Cargo,
            inventory_scope: InventoryScopeV1 {
                manifest_ref: DomainObjectRef::new("workspace_fs", "file", "Cargo.toml").unwrap(),
                lockfile_ref: DomainObjectRef::new("workspace_fs", "file", "Cargo.lock").unwrap(),
                include_transitive: true,
            },
        }
    }
    fn inventory(complete: bool, at: u64) -> DependencyInventorySnapshotV1 {
        DependencyInventorySnapshotV1::canonical(
            subject(),
            "rev".into(),
            "m".into(),
            "l".into(),
            vec![ResolvedDependencyComponentV1 {
                component_id: "serde-1".into(),
                ecosystem: PackageEcosystem::Cargo,
                package_name: "serde".into(),
                resolved_version: "1.0.0".into(),
                source_identity: "registry".into(),
                dependency_paths: vec![vec!["root".into(), "serde".into()]],
            }],
            if complete {
                InventoryCompleteness::CompleteTransitive
            } else {
                InventoryCompleteness::Incomplete {
                    reasons: vec!["truncated".into()],
                }
            },
            at,
        )
        .unwrap()
    }
    fn advisory(
        covered: bool,
        conflict: bool,
        at: u64,
        affected: bool,
    ) -> AdvisoryKnowledgeSnapshotV1 {
        AdvisoryKnowledgeSnapshotV1::canonical(
            "fixture".into(),
            "r1".into(),
            PackageEcosystem::Cargo,
            if covered {
                vec![ComponentCoverageV1 {
                    package_name: "serde".into(),
                    source_identity: "registry".into(),
                }]
            } else {
                vec![]
            },
            if affected {
                vec![NormalizedAdvisoryV1 {
                    source_advisory_id: "ADV-1".into(),
                    aliases: vec![],
                    package_name: "serde".into(),
                    affected_versions: vec!["1.0.0".into()],
                    severity: SeverityV1::High,
                }]
            } else {
                vec![]
            },
            if conflict {
                vec!["fixture records disagree".into()]
            } else {
                vec![]
            },
            AdvisoryCompleteness::CompleteForDeclaredCoverage,
            at,
        )
        .unwrap()
    }
    #[test]
    fn posture_precedence_is_deterministic() {
        assert_eq!(
            assess(
                &subject(),
                Some(&inventory(true, 0)),
                Some(&advisory(true, true, 0, true)),
                &policy(),
                100
            )
            .unwrap()
            .posture,
            DependencySecurityPosture::Conflicted
        );
        assert_eq!(
            assess(
                &subject(),
                Some(&inventory(true, 0)),
                Some(&advisory(true, false, 0, true)),
                &policy(),
                100
            )
            .unwrap()
            .posture,
            DependencySecurityPosture::Stale
        );
    }
    #[test]
    fn empty_findings_do_not_imply_clean() {
        assert_eq!(
            assess(
                &subject(),
                Some(&inventory(false, 10)),
                Some(&advisory(false, false, 10, false)),
                &policy(),
                10
            )
            .unwrap()
            .posture,
            DependencySecurityPosture::Insufficient
        );
    }
    #[test]
    fn clean_requires_complete_current_coverage() {
        assert_eq!(
            assess(
                &subject(),
                Some(&inventory(true, 10)),
                Some(&advisory(true, false, 10, false)),
                &policy(),
                10
            )
            .unwrap()
            .posture,
            DependencySecurityPosture::CleanWithinCoverage
        );
    }

    #[test]
    fn tampered_or_future_inputs_cannot_establish_current_clean_posture() {
        let inventory = inventory(true, 10);
        let advisory = advisory(true, false, 10, false);
        let mut altered_inventory = inventory.clone();
        altered_inventory.components.clear();
        assert!(assess(
            &subject(),
            Some(&altered_inventory),
            Some(&advisory),
            &policy(),
            10
        )
        .is_err());
        let mut altered_advisory = advisory.clone();
        altered_advisory.source_revision = "foreign-revision".into();
        assert!(assess(
            &subject(),
            Some(&inventory),
            Some(&altered_advisory),
            &policy(),
            10
        )
        .is_err());
        assert_eq!(
            assess(&subject(), Some(&inventory), Some(&advisory), &policy(), 9)
                .unwrap()
                .posture,
            DependencySecurityPosture::Unknown
        );
        assert_eq!(
            assess(&subject(), Some(&inventory), Some(&advisory), &policy(), 21)
                .unwrap()
                .posture,
            DependencySecurityPosture::Stale
        );
    }
}
