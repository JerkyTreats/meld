//! Graph-owned proof that an exact producer publication belongs to an admitted cut.

use super::contracts::{
    OwnerGraphRevisionReceipt, OwnerPublicationExpectation, TraversalCut, TraversalCutRequest,
    TraversalCutStatus,
};
use super::query::TraversalQuery;
use crate::error::StorageError;

/// Only a native Graph query can establish this proof. Event append alone cannot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerPublicationVisibilityProof {
    cut_id: String,
    expectation: OwnerPublicationExpectation,
    receipt: OwnerGraphRevisionReceipt,
}

impl OwnerPublicationVisibilityProof {
    pub fn cut_id(&self) -> &str {
        &self.cut_id
    }
    pub fn expectation(&self) -> &OwnerPublicationExpectation {
        &self.expectation
    }
    pub fn receipt(&self) -> &OwnerGraphRevisionReceipt {
        &self.receipt
    }
}

impl TraversalQuery<'_> {
    /// Re-resolve the frozen owner selection against durable Graph state.
    /// Later replay may advance Graph's cursor without changing the frozen evidence.
    pub fn publication_visibility(
        &self,
        cut: &TraversalCut,
        expected: &OwnerPublicationExpectation,
    ) -> Result<Option<OwnerPublicationVisibilityProof>, StorageError> {
        expected.validate()?;
        cut.validate_identity()?;
        if cut.status != TraversalCutStatus::Complete
            || cut.event_position.ledger_id != cut.graph_position.ledger_id
            || cut.event_position.after_seq > cut.graph_position.after_seq
        {
            return Ok(None);
        }
        let observed = self.cut(&TraversalCutRequest {
            owners: cut.owners.clone(),
            scope: cut.scope.clone(),
            currentness: cut.currentness,
            event_position: cut.event_position,
        })?;
        if observed.status != TraversalCutStatus::Complete
            || observed.graph_position.after_seq < cut.graph_position.after_seq
            || observed.receipts != cut.receipts
        {
            return Ok(None);
        }
        let publication = self
            .store
            .owner_publications_through_seq(cut.event_position.after_seq)?
            .into_iter()
            .find(|publication| {
                publication.operation.event_record_id() == expected.event_record_id
                    && publication.operation.batch.owner_id == expected.owner_id
                    && publication.operation.batch.revision_id == expected.revision_id
                    && publication.operation.batch.scope == expected.scope
            });
        let Some(publication) = publication else {
            return Ok(None);
        };
        let receipt = observed.receipts.into_iter().find(|receipt| {
            receipt.owner_id == expected.owner_id
                && receipt.revision_id == expected.revision_id
                && receipt.scope == expected.scope
                && receipt.source_event.as_ref().is_some_and(|event| {
                    *event == publication.source_event
                        && event.ledger_id == cut.event_position.ledger_id
                        && event.seq <= cut.event_position.after_seq
                })
        });
        Ok(receipt.map(|receipt| OwnerPublicationVisibilityProof {
            cut_id: cut.cut_id.clone(),
            expectation: expected.clone(),
            receipt,
        }))
    }
}
