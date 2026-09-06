//! Owner-qualified requirements and realization under a complete bounded source.

use super::{stable_identity, CurationOperation, StandingCurationRuleRevision, CURATION_OWNER_ID};
use crate::error::StorageError;
use crate::world_state::graph::contracts::*;
use meld_events::DomainObjectRef;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Vocabulary and predicates supplied by an installed semantic owner. Curation
/// authors expectations and requirements; it does not interpret foreign payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurationCoverageRule {
    pub source_required_qualifications: BTreeMap<String, String>,
    pub target_kind: String,
    pub target_required_qualifications: BTreeMap<String, String>,
    pub item_kind: String,
    pub item_target_relation: String,
    pub item_source_relation: String,
    pub source_kind: String,
    pub item_required_qualifications: BTreeMap<String, String>,
    pub expectation_kind: String,
    pub requirement_kind: String,
    pub expected_target_relation: String,
    pub expectation_requirement_relation: String,
    pub requirement_source_relation: String,
}

impl CurationCoverageRule {
    pub fn validate(&self) -> Result<(), StorageError> {
        if [
            &self.target_kind,
            &self.item_kind,
            &self.item_target_relation,
            &self.item_source_relation,
            &self.source_kind,
            &self.expectation_kind,
            &self.requirement_kind,
            &self.expected_target_relation,
            &self.expectation_requirement_relation,
            &self.requirement_source_relation,
        ]
        .iter()
        .any(|value| value.trim().is_empty())
            || [
                &self.source_required_qualifications,
                &self.target_required_qualifications,
                &self.item_required_qualifications,
            ]
            .iter()
            .any(|predicates| {
                predicates.is_empty()
                    || predicates
                        .iter()
                        .any(|(key, value)| key.trim().is_empty() || value.trim().is_empty())
            })
        {
            return Err(invalid(
                "Curation coverage requires explicit vocabulary and nonempty owner predicates",
            ));
        }
        Ok(())
    }
}

fn matches(object: &OwnerObjectPublication, predicates: &BTreeMap<String, String>) -> bool {
    object.state == OwnerPublicationState::Observed
        && predicates
            .iter()
            .all(|(key, value)| object.qualifications.get(key) == Some(value))
}

/// No absence or vacuous coverage may be inferred before the declared owner has
/// published its complete comparison source and readiness predicates.
pub(crate) fn coverage_ready(
    rule: &StandingCurationRuleRevision,
    traversal: &TraversalResult,
) -> bool {
    let Some(coverage) = &rule.rule.coverage else {
        return true;
    };
    traversal.receipts.iter().any(|receipt| {
        receipt.owner_id == rule.rule.source_owner_id
            && receipt.scope == rule.rule.scope
            && receipt.completeness.status == OwnerCompletenessStatus::Complete
            && receipt.completeness.failures.is_empty()
    }) && rule.rule.roots.iter().all(|root| {
        traversal.objects.iter().any(|object| {
            object.object_ref == *root && matches(object, &coverage.source_required_qualifications)
        })
    })
}

pub(crate) fn author_coverage(
    rule: &StandingCurationRuleRevision,
    operation: &CurationOperation,
    traversal: &TraversalResult,
    batch: &mut OwnerPublicationBatch,
) -> Result<(), StorageError> {
    let Some(coverage) = &rule.rule.coverage else {
        return Ok(());
    };
    let source_objects = traversal
        .objects
        .iter()
        .filter(|object| object.object_ref.domain_id == rule.rule.source_owner_id)
        .map(|object| (object.object_ref.clone(), object))
        .collect::<BTreeMap<_, _>>();
    let targets = source_objects
        .values()
        .filter(|object| object.object_ref.object_kind == coverage.target_kind)
        .map(|object| (object.object_ref.clone(), *object))
        .collect::<BTreeMap<_, _>>();
    let mut items =
        BTreeMap::<DomainObjectRef, Vec<(&OwnerObjectPublication, DomainObjectRef)>>::new();
    let mut pairs = BTreeSet::new();
    for item in source_objects
        .values()
        .filter(|object| object.object_ref.object_kind == coverage.item_kind)
    {
        let endpoints = |relation_type: &str| -> Result<DomainObjectRef, StorageError> {
            let selected = traversal
                .occurrences
                .iter()
                .filter(|relation| {
                    relation.src == item.object_ref && relation.relation_type == relation_type
                })
                .map(|relation| relation.dst.clone())
                .collect::<Vec<_>>();
            let [endpoint] = selected.as_slice() else {
                return Err(invalid(
                    "Curation requirement item must have one exact target and source relation",
                ));
            };
            Ok(endpoint.clone())
        };
        let target = endpoints(&coverage.item_target_relation)?;
        let source = endpoints(&coverage.item_source_relation)?;
        if !targets.contains_key(&target)
            || source_objects.get(&source).is_none_or(|object| {
                object.object_ref.object_kind != coverage.source_kind
                    || object.state != OwnerPublicationState::Observed
            })
            || !pairs.insert((target.clone(), source.clone()))
        {
            return Err(invalid("Curation requirement item names a foreign, missing, or repeated target/source pair"));
        }
        items.entry(target).or_default().push((item, source));
    }
    let basis = traversal
        .receipts
        .iter()
        .filter(|receipt| receipt.owner_id == rule.rule.source_owner_id)
        .map(|receipt| receipt.semantic_basis())
        .collect::<Vec<_>>();
    let seed = stable_identity(
        "curation-coverage-source-v1",
        &(&rule.content_hash, &operation.authority, &basis),
    )?;
    let hydration = batch.objects[0].hydration.clone();
    let provenance = batch.objects[0].provenance_refs.clone();
    let base = batch.objects[0].qualifications.clone();
    let mut all_satisfied = true;
    let mut requirement_count = 0usize;
    for (target, observed) in &targets {
        let expected = DomainObjectRef::new(
            CURATION_OWNER_ID,
            &coverage.expectation_kind,
            stable_identity(
                "curation-expected-target-v1",
                &(&rule.rule.expected_object_id, target),
            )?,
        )?;
        let mut satisfied = matches(observed, &coverage.target_required_qualifications);
        let requirements = items.remove(target).unwrap_or_default();
        for (item, source) in &requirements {
            let realized = matches(item, &coverage.item_required_qualifications);
            satisfied &= realized;
            let requirement = DomainObjectRef::new(
                CURATION_OWNER_ID,
                &coverage.requirement_kind,
                stable_identity("curation-required-source-v1", &(&expected, source))?,
            )?;
            let mut qualifications = base.clone();
            qualifications.insert(
                "realization".into(),
                if realized { "realized" } else { "not_realized" }.into(),
            );
            qualifications.insert("source_item".into(), item.object_ref.index_key());
            batch.objects.push(OwnerObjectPublication {
                publication_id: stable_identity(
                    "curation-required-publication-v1",
                    &(&seed, &requirement),
                )?,
                object_ref: requirement.clone(),
                state: OwnerPublicationState::Observed,
                source_product_ref: operation.operation_id.clone(),
                hydration: hydration.clone(),
                provenance_refs: provenance.clone(),
                qualifications,
            });
            for (relation_type, src, dst) in [
                (
                    &coverage.expectation_requirement_relation,
                    expected.clone(),
                    requirement.clone(),
                ),
                (
                    &coverage.requirement_source_relation,
                    requirement,
                    source.clone(),
                ),
            ] {
                batch.relations.push(relation(
                    &seed,
                    relation_type,
                    src,
                    dst,
                    operation,
                    &hydration,
                    &provenance,
                )?);
            }
        }
        requirement_count += requirements.len();
        all_satisfied &= satisfied;
        let mut qualifications = base.clone();
        qualifications.insert(
            "coverage".into(),
            if satisfied {
                "satisfied"
            } else {
                "unsatisfied"
            }
            .into(),
        );
        qualifications.insert("requirement_count".into(), requirements.len().to_string());
        batch.objects.push(OwnerObjectPublication {
            publication_id: stable_identity(
                "curation-expected-publication-v1",
                &(&seed, &expected),
            )?,
            object_ref: expected.clone(),
            state: OwnerPublicationState::Observed,
            source_product_ref: operation.operation_id.clone(),
            hydration: hydration.clone(),
            provenance_refs: provenance.clone(),
            qualifications,
        });
        batch.relations.push(relation(
            &seed,
            &coverage.expected_target_relation,
            expected,
            target.clone(),
            operation,
            &hydration,
            &provenance,
        )?);
    }
    // The aggregate is the existing Curation result subject. Its revision changes
    // with the source basis even when the set of expected identities is unchanged.
    batch.objects[0].publication_id = stable_identity(
        "curation-coverage-publication-v1",
        &(&seed, &batch.objects[0].object_ref),
    )?;
    batch.objects[0].qualifications.extend(BTreeMap::from([
        (
            "coverage".into(),
            if all_satisfied {
                "satisfied"
            } else {
                "unsatisfied"
            }
            .into(),
        ),
        (
            "judgment_subject_id".into(),
            operation.authority.subject.object_id.clone(),
        ),
        (
            "judgment_subject_domain".into(),
            operation.authority.subject.domain_id.clone(),
        ),
        (
            "judgment_subject_kind".into(),
            operation.authority.subject.object_kind.clone(),
        ),
        ("expected_count".into(), targets.len().to_string()),
        ("requirement_count".into(), requirement_count.to_string()),
    ]));
    batch.completeness.included_ids = batch
        .objects
        .iter()
        .map(|object| object.publication_id.clone())
        .chain(
            batch
                .relations
                .iter()
                .map(|relation| relation.occurrence_id.clone()),
        )
        .collect();
    Ok(())
}

fn relation(
    seed: &str,
    relation_type: &str,
    src: DomainObjectRef,
    dst: DomainObjectRef,
    operation: &CurationOperation,
    hydration: &HydrationReference,
    provenance: &[String],
) -> Result<OwnerRelationOccurrence, StorageError> {
    Ok(OwnerRelationOccurrence {
        occurrence_id: stable_identity(
            "curation-coverage-relation-v1",
            &(seed, relation_type, &src, &dst),
        )?,
        relation_type: relation_type.into(),
        src,
        dst,
        source_product_ref: operation.operation_id.clone(),
        hydration: hydration.clone(),
        qualifications: BTreeMap::new(),
        provenance_refs: provenance.to_vec(),
    })
}
fn invalid(message: &str) -> StorageError {
    StorageError::InvalidPath(message.into())
}
