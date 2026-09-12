//! Curation-owned proof that derived evidence still names the selected source.

use super::{
    CurationAcceptanceRecord, CurationAdmissionDecision, CurationPublicationKind, CurationQuery,
    CURATION_OWNER_ID,
};
use crate::belief::TheoryRevisionRef;
use crate::error::StorageError;
use crate::world_state::graph::contracts::{TraversalCut, TraversalCutStatus};

/// Only Curation can tie its terminal publication to a current Graph source basis.
pub struct CurationEvidenceBasisProof {
    pub(crate) publication_record_id: String,
}

impl CurationQuery<'_> {
    /// Require the selected Curation publication to retain the same non-Curation
    /// source receipts and installed rule. Older evidence remains historical.
    pub fn current_evidence_basis(
        &self,
        rule_ref: &TheoryRevisionRef,
        cut: &TraversalCut,
    ) -> Result<Option<CurationEvidenceBasisProof>, StorageError> {
        if cut.status != TraversalCutStatus::Complete {
            return Ok(None);
        }
        let rule = self.store.resolve_rule(rule_ref)?;
        if rule.revision_ref() != *rule_ref || rule.rule.scope != cut.scope {
            return Ok(None);
        }
        let Some(selected) = cut.receipts.iter().find(|receipt| {
            receipt.owner_id == CURATION_OWNER_ID && &receipt.scope == rule.rule.publication_scope()
        }) else {
            return Ok(None);
        };
        let Some(result) = self.store.result(&selected.revision_id)? else {
            return Ok(None);
        };
        let Some(operation) = self.store.operation(&result.operation_id)? else {
            return Ok(None);
        };
        if CurationAcceptanceRecord::for_operation(&operation, &rule)?.decision
            != CurationAdmissionDecision::Admitted
        {
            return Ok(None);
        }
        let current: Vec<_> = cut
            .receipts
            .iter()
            .filter(|receipt| receipt.owner_id != CURATION_OWNER_ID)
            .map(|receipt| receipt.semantic_basis())
            .collect();
        let source: Vec<_> = operation
            .source_cut
            .receipts
            .iter()
            .filter(|receipt| receipt.owner_id != CURATION_OWNER_ID)
            .map(|receipt| receipt.semantic_basis())
            .collect();
        if source != current || source.is_empty() {
            return Ok(None);
        }
        let Some(publication) = &result.semantic_publication else {
            return Ok(None);
        };
        let Some(semantic) = self
            .store
            .publication_receipt(&result.result_id, CurationPublicationKind::Semantic)?
        else {
            return Ok(None);
        };
        let Some(terminal) = self
            .store
            .publication_receipt(&result.result_id, CurationPublicationKind::Terminal)?
        else {
            return Ok(None);
        };
        if publication.batch.revision_id != selected.revision_id
            || selected.source_event.is_none_or(|source| {
                source.ledger_id != semantic.event_position.ledger_id
                    || source.seq != semantic.event_position.after_seq
            })
        {
            return Ok(None);
        }
        Ok(Some(CurationEvidenceBasisProof {
            publication_record_id: terminal.event_record_id,
        }))
    }
}
