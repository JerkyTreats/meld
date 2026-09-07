use crate::curation::{
    CurationAcceptanceRecord, CurationOperation, CurationPublicationKind,
    CurationPublicationReceipt, CurationResult, CurationStore, StandingCurationRuleRevision,
};
use crate::error::StorageError;

/// Read-only facade over durable standing Curation state.
pub struct CurationQuery<'a> {
    pub(super) store: &'a CurationStore,
}

impl<'a> CurationQuery<'a> {
    /// Bind a query facade to the Curation-owned store family.
    pub fn new(store: &'a CurationStore) -> Self {
        Self { store }
    }

    /// Read the active standing rule for one Agent.
    pub fn active_rule(
        &self,
        agent_id: &str,
    ) -> Result<Option<StandingCurationRuleRevision>, StorageError> {
        self.store.active_rule(agent_id)
    }

    /// Read the exact operation behind a returned result without consulting active selection.
    pub fn operation(&self, operation_id: &str) -> Result<Option<CurationOperation>, StorageError> {
        self.store.operation(operation_id)
    }

    /// Read the durable operation selected for one declared input.
    pub fn operation_for_selection(
        &self,
        selection_id: &str,
    ) -> Result<Option<CurationOperation>, StorageError> {
        self.store.operation_for_selection(selection_id)
    }

    /// Read one durable admission decision.
    pub fn acceptance(
        &self,
        acceptance_id: &str,
    ) -> Result<Option<CurationAcceptanceRecord>, StorageError> {
        self.store.acceptance(acceptance_id)
    }

    /// Read the terminal result for one admitted operation.
    pub fn result_for_operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<CurationResult>, StorageError> {
        self.store.result_for_operation(operation_id)
    }

    /// Read the Event append receipt for one terminal publication.
    pub fn publication_receipt(
        &self,
        result_id: &str,
        kind: CurationPublicationKind,
    ) -> Result<Option<CurationPublicationReceipt>, StorageError> {
        self.store.publication_receipt(result_id, kind)
    }
}
