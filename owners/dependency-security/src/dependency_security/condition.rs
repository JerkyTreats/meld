//! Current Security meaning over exact, durably retained owner products.

use super::{
    capability::*,
    contracts::*,
    publication::{Receipt, OWNER, RECEIPT_EVENT},
};
use crate::{
    capability::{CapabilityInvocationPayload, CapabilityRuntimeInit},
    error::ApiError,
};
use meld_events::{DomainObjectRef, EventReplayCapability, LedgerCursor, ReplayRequest};
use meld_world_model::world_state::graph::{
    admission::GraphOwnerEventRoute, contracts::*, events::owner_publication_envelope,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const EVENT: &str = "dependency_security.current_condition.v1";
const SCHEMA: &str = "dependency_security.current_condition.v1";

pub fn graph_route() -> GraphOwnerEventRoute {
    GraphOwnerEventRoute {
        complete_event_source: true,
        route_id: "dependency-security-current-condition".into(),
        owner_id: OWNER.into(),
        event_type: EVENT.into(),
        enumeration_rule_revision: SCHEMA.into(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductPosition {
    pub receipt_id: String,
    pub receipt_seq: u64,
    pub product_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentSecurityCondition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_input_basis_id: Option<String>,
    pub subject: DomainObjectRef,
    pub policy_revision: crate::theory::TheoryRevisionRef,
    pub reference_time: u64,
    pub products: BTreeMap<String, ProductPosition>,
    pub coverage_current: bool,
    pub current_posture: DependencySecurityPosture,
    pub verified_clean: bool,
}

pub fn curation_source(
    template: &meld_world_model::curation::CurationRuleTemplate,
    binding: &meld_world_model::curation::CurationRuleBinding,
) -> Result<meld_world_model::curation::CurationSourceBinding, String> {
    let policy = template
        .realization
        .as_ref()
        .and_then(|rule| rule.required_qualifications.get("policy_revision"))
        .filter(|hash| !hash.is_empty())
        .ok_or("Security Curation requires an exact policy revision qualification")?;
    let scope = scope(&binding.subject, policy)?;
    Ok(meld_world_model::curation::CurationSourceBinding {
        event_source: Some(graph_route().source_ref().map_err(|e| e.to_string())?),
        roots: vec![
            DomainObjectRef::new(OWNER, "current_condition", &scope.scope_id)
                .map_err(|e| e.to_string())?,
        ],
        scope,
    })
}

pub(crate) fn scope(
    subject: &DomainObjectRef,
    policy_hash: &str,
) -> Result<OwnerPublicationScope, String> {
    Ok(OwnerPublicationScope {
        scope_id: format!(
            "security-condition::{}",
            content_hash(&(subject, policy_hash))?
        ),
        branch_id: None,
        perspective_id: None,
        valid_at: None,
    })
}

pub(crate) fn current(
    capability: &SecurityCapability,
    events: &EventReplayCapability,
) -> Result<Option<CurrentSecurityCondition>, ApiError> {
    current_state(capability, events).map(|(current, _)| current)
}

type SourceBodies = BTreeMap<String, serde_json::Value>;

pub(crate) fn current_state(
    capability: &SecurityCapability,
    events: &EventReplayCapability,
) -> Result<(Option<CurrentSecurityCondition>, SourceBodies), ApiError> {
    let mut cursor = LedgerCursor {
        ledger_id: events.ledger_identity(),
        after_seq: 0,
    };
    let mut tip = None;
    let mut products = BTreeMap::new();
    let mut bodies = BTreeMap::new();
    let mut reference_time = 0;
    let expected_subject = serde_json::to_value(&capability.subject).map_err(invalid)?;
    let expected_policy = serde_json::to_value(&capability.policy).map_err(invalid)?;
    loop {
        let page = events
            .replay(ReplayRequest {
                cursor,
                limit: 1024,
            })
            .map_err(invalid)?;
        if page.coverage.retained_from > 1 {
            return Err(invalid(
                "Security current evidence requires retained owner history",
            ));
        }
        let frozen = *tip.get_or_insert(page.coverage.tip_seq);
        for record in page.records.iter().filter(|record| record.seq <= frozen) {
            if record.domain_id == OWNER && record.event_type == super::currency::EVENT {
                let observed = super::currency::CurrencyObservation::from_record(
                    record,
                    events.ledger_identity(),
                )
                .map_err(invalid)?;
                if observed.subject != capability.subject || observed.policy != capability.policy {
                    continue;
                }
                observed
                    .validate_basis(&products, &bodies, reference_time)
                    .map_err(invalid)?;
                reference_time = observed.reference_time;
                let receipt_id = observed
                    .record_id(events.ledger_identity())
                    .map_err(invalid)?;
                products.insert(
                    super::currency::PRODUCT.into(),
                    ProductPosition {
                        product_id: receipt_id.clone(),
                        receipt_id,
                        receipt_seq: record.seq,
                    },
                );
                continue;
            }
            if record.domain_id == OWNER
                && matches!(
                    record.event_type.as_str(),
                    super::observation::EVENT | super::observation::INVENTORY_EVENT
                )
            {
                let observed = super::observation::SourceObservation::from_record(
                    record,
                    events.ledger_identity(),
                )
                .map_err(invalid)?;
                if observed.subject != capability.subject || observed.policy != capability.policy {
                    continue;
                }
                super::returns::validate_retained(&observed, events).map_err(invalid)?;
                reference_time = reference_time.max(observed.observed_at);
                let kind = observed.kind().to_owned();
                let unavailable = observed.unavailable_kind().to_owned();
                products.remove(&kind);
                products.remove(&unavailable);
                bodies.remove(&kind);
                let receipt_id = observed
                    .record_id(events.ledger_identity())
                    .map_err(invalid)?;
                if let Some((product_id, body)) = observed.product().map_err(invalid)? {
                    products.insert(
                        kind.clone(),
                        ProductPosition {
                            receipt_id,
                            receipt_seq: record.seq,
                            product_id,
                        },
                    );
                    bodies.insert(kind, body);
                } else {
                    products.insert(
                        unavailable,
                        ProductPosition {
                            receipt_id,
                            receipt_seq: record.seq,
                            product_id: content_hash(&observed.failure.unwrap())
                                .map_err(invalid)?,
                        },
                    );
                }
                continue;
            }
            if record.domain_id != OWNER || record.event_type != RECEIPT_EVENT {
                continue;
            }
            let receipt: Receipt = serde_json::from_value(record.data.clone()).map_err(invalid)?;
            if receipt.binding["subject"] != expected_subject
                || receipt.binding["policy"] != expected_policy
            {
                continue;
            }
            let runtime: CapabilityRuntimeInit =
                serde_json::from_value(receipt.binding["runtime"].clone()).map_err(invalid)?;
            let payload: CapabilityInvocationPayload =
                serde_json::from_value(receipt.binding["payload"].clone()).map_err(invalid)?;
            let mut producer = capability.clone();
            producer.id = runtime.capability_type_id;
            let expected_id = format!(
                "security-return::{}",
                content_hash(&(events.ledger_identity(), &receipt.binding)).map_err(invalid)?
            );
            if record.record_id.as_deref() != Some(expected_id.as_str()) {
                return Err(invalid(
                    "Security receipt identity differs from its binding",
                ));
            }
            producer.validate_result(
                &serde_json::from_value(receipt.binding["runtime"].clone()).map_err(invalid)?,
                &payload,
                &receipt.result(),
            )?;
            let [artifact] = receipt.artifacts.as_slice() else {
                return Err(invalid("Security product absent"));
            };
            let product_id = ["snapshot_id", "assessment_id", "verification_id"]
                .iter()
                .find_map(|field| artifact.content[*field].as_str())
                .ok_or_else(|| invalid("Security product identity absent"))?;
            let at = chrono::DateTime::parse_from_rfc3339(&record.ts)
                .map_err(invalid)?
                .timestamp();
            reference_time = reference_time.max(u64::try_from(at).map_err(invalid)?);
            match artifact.artifact_type_id.as_str() {
                ADVISORIES => {
                    products.remove(super::observation::UNAVAILABLE);
                }
                INVENTORY => {
                    products.remove(super::observation::INVENTORY_UNAVAILABLE);
                }
                _ => {}
            }
            products.insert(
                artifact.artifact_type_id.clone(),
                ProductPosition {
                    receipt_id: expected_id,
                    receipt_seq: record.seq,
                    product_id: product_id.into(),
                },
            );
            bodies.insert(artifact.artifact_type_id.clone(), artifact.content.clone());
        }
        if page.next_cursor.after_seq >= frozen {
            break;
        }
        if page.next_cursor.after_seq <= cursor.after_seq {
            return Err(invalid(
                "Security history replay did not cover its frozen source position",
            ));
        }
        cursor = page.next_cursor;
    }
    if products.is_empty() {
        return Ok((None, bodies));
    }
    let inventory: Option<DependencyInventorySnapshotV1> = bodies
        .get(INVENTORY)
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(invalid)?;
    let advisory: Option<AdvisoryKnowledgeSnapshotV1> = bodies
        .get(ADVISORIES)
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(invalid)?;
    let assessment: Option<DependencySecurityAssessmentV1> = bodies
        .get(ASSESSMENT)
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(invalid)?;
    let verification: Option<DependencySecurityVerificationV1> = bodies
        .get(VERIFICATION)
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(invalid)?;
    let current = super::assessment::assess(
        &capability.subject,
        inventory.as_ref(),
        advisory.as_ref(),
        &capability.policy,
        reference_time,
    )
    .map_err(invalid)?;
    let verified_clean = match (&inventory, &advisory, &assessment, &verification) {
        (Some(inventory), Some(advisory), Some(assessment), Some(verification)) => {
            let recomputed =
                super::verification::verify(assessment, inventory, advisory, &capability.policy)
                    .map_err(invalid)?;
            recomputed == *verification
                && verification.verified
                && current.posture == DependencySecurityPosture::CleanWithinCoverage
        }
        _ => false,
    };
    Ok((
        Some(CurrentSecurityCondition {
            work_input_basis_id: work_input_basis(
                &capability.subject,
                &capability.policy,
                inventory.as_ref(),
                advisory.as_ref(),
            )
            .map_err(invalid)?,
            subject: capability.subject.subject.clone(),
            policy_revision: capability.policy.revision_ref().map_err(invalid)?,
            reference_time,
            products,
            coverage_current: matches!(
                current.posture,
                DependencySecurityPosture::CleanWithinCoverage
                    | DependencySecurityPosture::Violated
            ),
            current_posture: current.posture,
            verified_clean,
        }),
        bodies,
    ))
}

/// Acquisition times and our own assessment returns are not changed domain inputs.
fn work_input_basis(
    subject: &DependencySecuritySubjectV1,
    policy: &super::policy::DependencySecurityPolicyV1,
    inventory: Option<&DependencyInventorySnapshotV1>,
    advisory: Option<&AdvisoryKnowledgeSnapshotV1>,
) -> Result<Option<String>, String> {
    let (Some(inventory), Some(advisory)) = (inventory, advisory) else {
        return Ok(None);
    };
    let material_inventory = (
        &inventory.subject,
        &inventory.workspace_revision,
        &inventory.manifest_content_hash,
        &inventory.lockfile_content_hash,
        &inventory.components,
        &inventory.completeness,
    );
    let material_advisory = (
        &advisory.source_id,
        &advisory.source_revision,
        &advisory.covered_ecosystem,
        &advisory.covered_components,
        &advisory.advisories,
        &advisory.conflicts,
        &advisory.completeness,
    );
    content_hash(&(subject, policy, material_inventory, material_advisory))
        .map(|hash| Some(format!("security-work-input-v1::{hash}")))
}

impl CurrentSecurityCondition {
    fn envelope(
        &self,
        session: &str,
        ledger: meld_events::LedgerIdentity,
    ) -> Result<meld_events::EventEnvelope, ApiError> {
        let scope = scope(&self.subject, &self.policy_revision.content_hash).map_err(invalid)?;
        let revision = format!(
            "security-condition-revision::{}",
            content_hash(self).map_err(invalid)?
        );
        let root =
            DomainObjectRef::new(OWNER, "current_condition", &scope.scope_id).map_err(invalid)?;
        let hydration = HydrationReference {
            owner_id: OWNER.into(),
            product_kind: "current_condition".into(),
            product_id: scope.scope_id.clone(),
            revision_id: revision.clone(),
            role: "current_security_evidence".into(),
        };
        let publication_id = format!("{revision}::condition");
        let relation_id = format!("{revision}::subject");
        let operation = OwnerPublicationOperation::reconstruct(
            SCHEMA,
            OwnerPublicationBatch {
                work_input_basis_id: self.work_input_basis_id.clone(),
                owner_id: OWNER.into(),
                revision_id: revision.clone(),
                scope: scope.clone(),
                objects: vec![OwnerObjectPublication {
                    publication_id: publication_id.clone(),
                    object_ref: root.clone(),
                    state: OwnerPublicationState::Observed,
                    source_product_ref: revision.clone(),
                    hydration: hydration.clone(),
                    provenance_refs: self
                        .products
                        .values()
                        .map(|p| p.receipt_id.clone())
                        .collect(),
                    qualifications: BTreeMap::from([
                        (
                            "condition".into(),
                            serde_json::to_string(self).map_err(invalid)?,
                        ),
                        (
                            "policy_revision".into(),
                            self.policy_revision.content_hash.clone(),
                        ),
                        ("verified_clean".into(), self.verified_clean.to_string()),
                        ("coverage_current".into(), self.coverage_current.to_string()),
                    ]),
                }],
                relations: vec![OwnerRelationOccurrence {
                    occurrence_id: relation_id.clone(),
                    relation_type: "security_condition_subject".into(),
                    src: root,
                    dst: self.subject.clone(),
                    source_product_ref: revision.clone(),
                    hydration,
                    qualifications: BTreeMap::new(),
                    provenance_refs: self
                        .products
                        .values()
                        .map(|p| p.receipt_id.clone())
                        .collect(),
                }],
                completeness: OwnerCompletenessReceipt {
                    receipt_id: format!("{revision}::enumeration"),
                    scope,
                    included_ids: vec![publication_id, relation_id],
                    exclusions: vec![],
                    failures: vec![],
                    status: OwnerCompletenessStatus::Complete,
                },
            },
        )
        .map_err(invalid)?;
        let mut envelope = owner_publication_envelope(session, &operation).map_err(invalid)?;
        envelope.event_type = EVENT.into();
        let mut sources: Vec<_> = self
            .products
            .values()
            .map(|p| meld_events::EventRecordRef {
                ledger_id: ledger,
                seq: p.receipt_seq,
            })
            .collect();
        sources.sort_by_key(|source| source.seq);
        sources.dedup();
        Ok(envelope.with_source_records(sources))
    }
}

pub(crate) fn publication(
    capability: &SecurityCapability,
    events: &EventReplayCapability,
) -> Result<meld_events::EventEnvelope, ApiError> {
    current(capability, events)?
        .ok_or_else(|| invalid("Security condition has no retained products"))?
        .envelope("dependency-security-condition", events.ledger_identity())
}

pub(crate) fn is_published(
    capability: &SecurityCapability,
    events: &EventReplayCapability,
) -> Result<bool, ApiError> {
    Ok(events
        .prove_existing(&publication(capability, events)?)
        .map_err(invalid)?
        .is_some())
}

fn invalid(error: impl ToString) -> ApiError {
    ApiError::ConfigError(error.to_string())
}

#[cfg(test)]
mod input_basis_tests {
    use super::*;

    #[test]
    fn security_inputs_ignore_acquisition_churn_and_track_material_changes() {
        let policy: super::super::policy::DependencySecurityPolicyV1 = serde_json::from_str(
            include_str!("../../../../theory/dependency_security/policy.cargo_fixture.json"),
        )
        .unwrap();
        let subject = DependencySecuritySubjectV1 {
            subject: DomainObjectRef::new("workspace_fs", "node", "repo").unwrap(),
            ecosystem: PackageEcosystem::Cargo,
            inventory_scope: InventoryScopeV1 {
                manifest_ref: DomainObjectRef::new("workspace_fs", "file", "Cargo.toml").unwrap(),
                lockfile_ref: DomainObjectRef::new("workspace_fs", "file", "Cargo.lock").unwrap(),
                include_transitive: true,
            },
        };
        let inventory = |time, hash: &str| {
            DependencyInventorySnapshotV1::canonical(
                subject.clone(),
                "workspace-revision".into(),
                "manifest".into(),
                hash.into(),
                vec![],
                InventoryCompleteness::CompleteTransitive,
                time,
            )
            .unwrap()
        };
        let advisory = |time, revision: &str| {
            AdvisoryKnowledgeSnapshotV1::canonical(
                policy.required_advisory_source_id.clone(),
                revision.into(),
                PackageEcosystem::Cargo,
                vec![],
                vec![],
                vec![],
                AdvisoryCompleteness::CompleteForDeclaredCoverage,
                time,
            )
            .unwrap()
        };
        let first_inventory = inventory(1, "lockfile");
        let first_advisory = advisory(1, "source-revision");
        let later_inventory = inventory(2, "lockfile");
        let later_advisory = advisory(2, "source-revision");
        assert_ne!(first_inventory.snapshot_id, later_inventory.snapshot_id);
        assert_ne!(first_advisory.snapshot_id, later_advisory.snapshot_id);
        let basis = work_input_basis(
            &subject,
            &policy,
            Some(&first_inventory),
            Some(&first_advisory),
        )
        .unwrap();
        assert!(basis.is_some());
        assert_eq!(
            basis,
            work_input_basis(
                &subject,
                &policy,
                Some(&later_inventory),
                Some(&later_advisory)
            )
            .unwrap()
        );
        assert_ne!(
            basis,
            work_input_basis(
                &subject,
                &policy,
                Some(&inventory(2, "changed-lockfile")),
                Some(&later_advisory)
            )
            .unwrap()
        );
        assert_ne!(
            basis,
            work_input_basis(
                &subject,
                &policy,
                Some(&later_inventory),
                Some(&advisory(2, "changed-source"))
            )
            .unwrap()
        );
        assert!(
            work_input_basis(&subject, &policy, Some(&first_inventory), None)
                .unwrap()
                .is_none()
        );
    }
}
