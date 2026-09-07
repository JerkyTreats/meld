//! Curation return evidence retains publication and Graph consumption as separate claims.

use super::{
    CurationPublicationKind, CurationQuery, CurationTerminalDisposition, CURATION_OWNER_ID,
};
use crate::error::StorageError;
use crate::world_state::graph::contracts::{
    OwnerPublicationExpectation, TraversalCut, TraversalCutRequest, TraversalCutStatus,
};
use crate::world_state::graph::query::TraversalQuery;

/// Constructed by Curation only after its durable return is visible in native Graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurationVisibilityProof {
    result_id: String,
    cut_id: String,
    graph_revision_id: String,
}

impl CurationVisibilityProof {
    #[cfg(test)]
    pub(crate) fn fixture(result: &super::CurationResult, cut: &TraversalCut) -> Self {
        assert_eq!(result.disposition, CurationTerminalDisposition::Unchanged);
        Self {
            result_id: result.result_id.clone(),
            cut_id: cut.cut_id.clone(),
            graph_revision_id: "fixture-curation-revision".into(),
        }
    }

    pub fn result_id(&self) -> &str {
        &self.result_id
    }
    pub fn cut_id(&self) -> &str {
        &self.cut_id
    }
    pub fn graph_revision_id(&self) -> &str {
        &self.graph_revision_id
    }
}

impl CurationQuery<'_> {
    /// Closed authority can absorb a result through its exact publication position.
    /// This reconstructs owner evidence without replacing the Agent's authorization cut.
    pub fn historical_prerequisite_visibility(
        &self,
        operation_id: &str,
        graph: &TraversalQuery<'_>,
    ) -> Result<Option<CurationVisibilityProof>, StorageError> {
        let Some(operation) = self.store.operation(operation_id)? else {
            return Ok(None);
        };
        let Some(result) = self.store.result_for_operation(operation_id)? else {
            return Ok(None);
        };
        let Some(terminal) = self
            .store
            .publication_receipt(&result.result_id, CurationPublicationKind::Terminal)?
        else {
            return Ok(None);
        };
        let cut = graph.cut(&TraversalCutRequest {
            owners: operation.source_cut.owners,
            scope: operation.source_cut.scope,
            currentness: operation.source_cut.currentness,
            event_position: terminal.event_position,
        })?;
        self.prerequisite_visibility(operation_id, &cut, graph)
    }

    /// A terminal result in the producer store does not establish this return.
    pub fn prerequisite_visibility(
        &self,
        operation_id: &str,
        cut: &TraversalCut,
        graph: &TraversalQuery<'_>,
    ) -> Result<Option<CurationVisibilityProof>, StorageError> {
        cut.validate_identity()?;
        if cut.status != TraversalCutStatus::Complete {
            return Ok(None);
        }
        let Some(result) = self.store.result_for_operation(operation_id)? else {
            return Ok(None);
        };
        let Some(operation) = self.store.operation(operation_id)? else {
            return Ok(None);
        };
        result.validate(&operation)?;
        let Some(terminal) = self
            .store
            .publication_receipt(&result.result_id, CurationPublicationKind::Terminal)?
        else {
            return Ok(None);
        };
        if terminal.event_position.ledger_id != cut.event_position.ledger_id
            || terminal.event_position.after_seq > cut.event_position.after_seq
            || cut.graph_position.ledger_id != cut.event_position.ledger_id
            || terminal.event_position.after_seq > cut.graph_position.after_seq
        {
            return Ok(None);
        }
        let graph_revision_id = match result.disposition {
            CurationTerminalDisposition::Applied => {
                let publication = result
                    .semantic_publication
                    .as_ref()
                    .expect("validated Applied result");
                let Some(semantic) = self
                    .store
                    .publication_receipt(&result.result_id, CurationPublicationKind::Semantic)?
                else {
                    return Ok(None);
                };
                let expected = OwnerPublicationExpectation {
                    owner_id: CURATION_OWNER_ID.into(),
                    revision_id: publication.batch.revision_id.clone(),
                    scope: publication.batch.scope.clone(),
                    event_record_id: semantic.event_record_id,
                };
                let Some(visible) = graph.publication_visibility(cut, &expected)? else {
                    return Ok(None);
                };
                if visible.receipt().source_event.is_none_or(|source| {
                    source.ledger_id != semantic.event_position.ledger_id
                        || source.seq != semantic.event_position.after_seq
                }) {
                    return Ok(None);
                }
                visible.receipt().revision_id.clone()
            }
            CurationTerminalDisposition::Unchanged => {
                // Unchanged work cites existing state instead of publishing a second revision.
                let Some(selected) = operation.source_cut.receipts.iter().find(|receipt| {
                    receipt.owner_id == CURATION_OWNER_ID
                        && receipt.scope == operation.source_cut.scope
                }) else {
                    return Ok(None);
                };
                let Some(prior) = self.store.result(&selected.revision_id)? else {
                    return Ok(None);
                };
                let Some(publication) = prior.semantic_publication else {
                    return Ok(None);
                };
                let expected = OwnerPublicationExpectation {
                    owner_id: CURATION_OWNER_ID.into(),
                    revision_id: selected.revision_id.clone(),
                    scope: selected.scope.clone(),
                    event_record_id: publication.event_record_id(),
                };
                let Some(visible) = graph.publication_visibility(cut, &expected)? else {
                    return Ok(None);
                };
                if !result.cited_publication_ids.iter().all(|id| {
                    publication
                        .batch
                        .objects
                        .iter()
                        .any(|object| &object.publication_id == id)
                        || publication
                            .batch
                            .relations
                            .iter()
                            .any(|relation| &relation.occurrence_id == id)
                }) {
                    return Ok(None);
                }
                visible.receipt().revision_id.clone()
            }
            CurationTerminalDisposition::Abstained
            | CurationTerminalDisposition::Incomplete
            | CurationTerminalDisposition::Conflicted
            | CurationTerminalDisposition::Failed => return Ok(None),
        };
        Ok(Some(CurationVisibilityProof {
            result_id: result.result_id,
            cut_id: cut.cut_id.clone(),
            graph_revision_id,
        }))
    }
}
