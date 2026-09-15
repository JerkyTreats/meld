//! Exact owner-publication lineage in an immutable Belief revision.

use crate::agent::AgentSubscriptionRequestV1;
use crate::error::StorageError;

use super::{BeliefKey, BeliefRevision, BeliefStore, TheoryRevisionRef};

/// Agent's required return, scoped to its accepted source relationship.
#[derive(Clone)]
pub struct BeliefEvidenceReturnRequest {
    pub subscription: AgentSubscriptionRequestV1,
    pub revision_ids: Vec<String>,
    pub publication_record_id: String,
    pub evidence_schema_id: String,
    pub mapping_revisions: Vec<TheoryRevisionRef>,
}

/// Only Belief can establish that a revision consumed the requested publication.
pub struct BeliefEvidenceReturnProof {
    belief_key: BeliefKey,
    revision_id: String,
}

impl BeliefEvidenceReturnProof {
    pub(crate) fn belief_key(&self) -> &BeliefKey {
        &self.belief_key
    }

    pub(crate) fn revision_id(&self) -> &str {
        &self.revision_id
    }
}

pub(super) fn resolve(
    store: &BeliefStore,
    request: &BeliefEvidenceReturnRequest,
) -> Result<Option<BeliefEvidenceReturnProof>, StorageError> {
    request.subscription.validate()?;
    if request.publication_record_id.trim().is_empty()
        || request.evidence_schema_id.trim().is_empty()
        || request.mapping_revisions.is_empty()
    {
        return Err(StorageError::InvalidPath(
            "Belief return requires an exact publication and installed mapping".into(),
        ));
    }
    for revision_id in &request.revision_ids {
        let Some(revision) = store.get_revision(revision_id)? else {
            continue;
        };
        if let Some(proof) = proof_for_revision(store, request, &revision)? {
            return Ok(Some(proof));
        }
    }

    // A current planner cut can supersede an earlier revision before another
    // goal reconciles the exact publication that revision consumed. Belief
    // owns a derived point lookup over that append-only provenance. The index
    // is not authority: every referenced revision and evidence predicate is
    // revalidated here before a historical proof can be returned.
    let Some(revision) = store.indexed_evidence_return_revision(
        &request.subscription.belief_key,
        &request.subscription.source_contract_revision,
        &request.publication_record_id,
        &request.evidence_schema_id,
        &request.mapping_revisions,
    )?
    else {
        return Ok(None);
    };
    if let Some(proof) = proof_for_revision(store, request, &revision)? {
        return Ok(Some(proof));
    }
    Err(StorageError::InvalidPath(
        "Belief evidence-return index differs from retained lineage".into(),
    ))
}

fn proof_for_revision(
    store: &BeliefStore,
    request: &BeliefEvidenceReturnRequest,
    revision: &BeliefRevision,
) -> Result<Option<BeliefEvidenceReturnProof>, StorageError> {
    if revision.belief_key != request.subscription.belief_key
        || revision.theory_revision.as_ref() != Some(&request.subscription.source_contract_revision)
    {
        return Ok(None);
    }
    for evidence in store.evidence_by_revision(&revision.revision_id)? {
        if evidence.publication_record_id.as_ref() == Some(&request.publication_record_id)
            && evidence.candidate_key == revision.belief_key
            && evidence.evidence_schema_id == request.evidence_schema_id
            && evidence
                .outcome_mapping_revision
                .as_ref()
                .is_some_and(|mapping| request.mapping_revisions.contains(mapping))
            && revision
                .supporting_evidence_ids
                .contains(&evidence.evidence_id)
        {
            return Ok(Some(BeliefEvidenceReturnProof {
                belief_key: revision.belief_key.clone(),
                revision_id: revision.revision_id.clone(),
            }));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use meld_events::DomainObjectRef;

    use super::*;
    use crate::agent::{AgentEpochSubscriptionPort, AgentSubscriptionRequestV1};
    use crate::belief::{
        configured_belief_key, BeliefFamilyConfig, BeliefFamilyRegistry, BeliefFamilyRegistryStore,
        BeliefProvenanceSummary, BeliefRuntime, BeliefSubscriptionAuthority,
        BeliefSubscriptionSource, BranchScope, EvidenceAssignment, EvidenceItem, EvidenceValue,
    };
    use crate::world_state::graph::PerspectiveKey;

    #[test]
    fn exact_superseded_publication_is_indexed_revalidated_and_rebuilt() {
        let root = tempfile::tempdir().unwrap();
        let db = sled::open(root.path()).unwrap();
        let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        let mut registry = BeliefFamilyRegistryStore::new(db.clone()).unwrap();
        let config: BeliefFamilyConfig = serde_json::from_str(include_str!(
            "../../../../theory/startup/belief_family.startup_realization.json"
        ))
        .unwrap();
        let family = registry.install(config, 1).unwrap().1;
        let key = configured_belief_key(
            &family,
            &DomainObjectRef::new("curation", "expected_startup", "indexed").unwrap(),
            &PerspectiveKey::new("agent", "observer").unwrap(),
            &BranchScope::main(),
        );
        let subscription = AgentSubscriptionRequestV1::new(
            "observer".into(),
            "belief".into(),
            family.revision_ref(),
            key.clone(),
            "from_genesis".into(),
        )
        .unwrap();
        BeliefSubscriptionAuthority::new(&store)
            .accept(&subscription, &family)
            .unwrap();
        let mapping = TheoryRevisionRef {
            registry: "outcome_mapping".into(),
            id: "curation_realization_v1".into(),
            content_hash: "mapping-revision".into(),
        };
        let runtime = BeliefRuntime::from_family_revision(
            store.clone(),
            &family,
            key.perspective.clone(),
            key.branch_scope.clone(),
        );
        let commit = |cursor: u64, publication: &str| {
            let evidence = EvidenceItem {
                publication_record_id: Some(publication.into()),
                evidence_id: format!("evidence-{cursor}"),
                candidate_key: key.clone(),
                source_fact_ids: Vec::new(),
                graph_anchor_ids: Vec::new(),
                source_cursor_start: cursor,
                source_cursor_end: cursor,
                role: crate::belief::EvidenceRole::Support,
                evidence_schema_id: "curation_realization_v1".into(),
                typed_value: EvidenceValue::Scalar(1.0),
                reliability: 1.0,
                precision: 1.0,
                reference_time: None,
                transaction_seq: cursor,
                content_hash: None,
                outcome_mapping_revision: Some(mapping.clone()),
                provenance: BeliefProvenanceSummary::empty(),
            };
            store.put_evidence(&evidence).unwrap();
            store
                .put_assignment(&EvidenceAssignment {
                    assignment_id: format!("assignment-{cursor}"),
                    evidence_id: evidence.evidence_id,
                    belief_key: key.clone(),
                    role: evidence.role,
                    source_cursor_start: cursor,
                    source_cursor_end: cursor,
                })
                .unwrap();
            runtime
                .assess_dirty_key(&key, "belief-test")
                .unwrap()
                .unwrap()
                .revision_id
        };
        let historical_revision_id = commit(10, "publication-historical");
        let current_revision_id = commit(20, "publication-current");
        assert_ne!(historical_revision_id, current_revision_id);
        assert_eq!(
            store.current_revision(&key).unwrap().unwrap().revision_id,
            current_revision_id
        );
        let request = BeliefEvidenceReturnRequest {
            subscription: subscription.clone(),
            revision_ids: vec![current_revision_id],
            publication_record_id: "publication-historical".into(),
            evidence_schema_id: "curation_realization_v1".into(),
            mapping_revisions: vec![mapping.clone()],
        };
        let source = BeliefSubscriptionSource::new(store.clone(), Arc::new(registry.clone()));
        let proof = source.returned_evidence(&request).unwrap().unwrap();
        assert_eq!(proof.revision_id(), historical_revision_id);
        assert!(!request.revision_ids.contains(&historical_revision_id));

        let mut foreign = request.clone();
        foreign.publication_record_id = "foreign-publication".into();
        assert!(source.returned_evidence(&foreign).unwrap().is_none());
        foreign = request.clone();
        foreign.evidence_schema_id = "foreign-schema".into();
        assert!(source.returned_evidence(&foreign).unwrap().is_none());
        foreign = request.clone();
        foreign.mapping_revisions[0].content_hash = "foreign-mapping".into();
        assert!(source.returned_evidence(&foreign).unwrap().is_none());
        foreign = request.clone();
        let mut family_ref = foreign.subscription.source_contract_revision.clone();
        family_ref.content_hash = "foreign-family".into();
        foreign.subscription = AgentSubscriptionRequestV1::new(
            foreign.subscription.agent_id.clone(),
            foreign.subscription.source_owner.clone(),
            family_ref,
            key.clone(),
            foreign.subscription.initial_cursor_policy.clone(),
        )
        .unwrap();
        assert!(source.returned_evidence(&foreign).unwrap().is_none());
        foreign = request.clone();
        let mut foreign_key = key.clone();
        foreign_key.subject.object_id = "foreign-subject".into();
        foreign.subscription = AgentSubscriptionRequestV1::new(
            foreign.subscription.agent_id.clone(),
            foreign.subscription.source_owner.clone(),
            foreign.subscription.source_contract_revision.clone(),
            foreign_key,
            foreign.subscription.initial_cursor_policy.clone(),
        )
        .unwrap();
        assert!(source.returned_evidence(&foreign).unwrap().is_none());

        let index_key = serde_json::to_vec(&(
            &key,
            family.revision_ref(),
            "publication-historical",
            "curation_realization_v1",
            &mapping,
        ))
        .map(|bytes| {
            format!(
                "evidence-return-lineage-v1::{}",
                blake3::hash(&bytes).to_hex()
            )
        })
        .unwrap();
        let index = db.open_tree("belief_evidence_return_lineage_v1").unwrap();
        index
            .insert(
                index_key.as_bytes(),
                br#"{"revision_id":"absent","source_cursor_end":10}"#.as_slice(),
            )
            .unwrap();
        assert!(source.returned_evidence(&request).is_err());

        drop(source);
        drop(runtime);
        drop(store);
        index.insert(b"partial", b"interrupted").unwrap();
        db.open_tree("belief_runtime_meta")
            .unwrap()
            .remove(b"evidence-return-lineage-index-v1")
            .unwrap();
        drop(index);
        drop(registry);
        drop(db);
        let db = sled::open(root.path()).unwrap();
        let store = Arc::new(BeliefStore::new(db.clone()).unwrap());
        assert!(db
            .open_tree("belief_evidence_return_lineage_v1")
            .unwrap()
            .get(b"partial")
            .unwrap()
            .is_none());
        let source = BeliefSubscriptionSource::new(
            store,
            Arc::new(BeliefFamilyRegistryStore::new(db).unwrap()),
        );
        assert_eq!(
            source
                .returned_evidence(&request)
                .unwrap()
                .unwrap()
                .revision_id(),
            historical_revision_id
        );
    }
}
