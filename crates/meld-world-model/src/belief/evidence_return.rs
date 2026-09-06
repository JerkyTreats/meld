//! Exact owner-publication lineage in an immutable Belief revision.

use crate::agent::AgentSubscriptionRequestV1;
use crate::error::StorageError;

use super::{BeliefKey, BeliefStore, TheoryRevisionRef};

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
        if revision.belief_key != request.subscription.belief_key
            || revision.theory_revision.as_ref()
                != Some(&request.subscription.source_contract_revision)
        {
            continue;
        }
        for evidence in store.evidence_by_revision(revision_id)? {
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
                    belief_key: revision.belief_key,
                    revision_id: revision.revision_id,
                }));
            }
        }
    }
    Ok(None)
}
