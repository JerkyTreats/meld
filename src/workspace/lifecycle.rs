//! Native lifecycle owner for the passive workspace command source.

use crate::runtime::error::RuntimeAssemblyError;
use crate::runtime::lifecycle::{
    NativeOwnerReadinessEvidenceV1, OwnerReadinessReceiptV1, OwnerReleaseReceiptV1,
    OwnerStopReceiptV1, OwnerWaitReceiptV1, ParticipantLifecycleContextV1, PassiveFenceReceiptV1,
    StructuralWakeRef,
};
use crate::runtime::ports::ProductEventAppendPort;

/// Passive workspace source bound to one product event authority.
pub(crate) struct WorkspaceSourceLifecycle {
    event_append: ProductEventAppendPort,
    workspace_binding_ref: String,
    fence_receipt_id: Option<String>,
    fence_checkpoint_ref: Option<String>,
    stop_receipt_id: Option<String>,
    stop_checkpoint_ref: Option<String>,
}

impl WorkspaceSourceLifecycle {
    pub(crate) fn new(event_append: ProductEventAppendPort, workspace_binding_ref: String) -> Self {
        Self {
            event_append,
            workspace_binding_ref,
            fence_receipt_id: None,
            fence_checkpoint_ref: None,
            stop_receipt_id: None,
            stop_checkpoint_ref: None,
        }
    }

    pub(crate) fn binding_ref(&self) -> &str {
        &self.workspace_binding_ref
    }

    pub(crate) fn readiness(
        &self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
        let watermark = self.watermark()?;
        let checkpoint =
            self.checkpoint_ref(watermark.ledger_id.to_string(), watermark.committed_seq);
        let evidence = NativeOwnerReadinessEvidenceV1::new(
            context.participant_id.clone(),
            context.owner_domain.clone(),
            checkpoint.clone(),
            vec!["workspace-command-source-schema::v1".to_string()],
            vec![self.workspace_binding_ref.clone()],
            vec![format!("event-ledger::{}", watermark.ledger_id)],
            checkpoint,
        )
        .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))?;
        OwnerReadinessReceiptV1::from_native(context, evidence)
            .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
    }

    pub(crate) fn wait(
        &self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
        let watermark = self.watermark()?;
        OwnerWaitReceiptV1::new(
            context.generation_id.clone(),
            context.incarnation_id.clone(),
            self.checkpoint_ref(watermark.ledger_id.to_string(), watermark.committed_seq),
            "workspace-source-awaiting-command".to_string(),
            vec![StructuralWakeRef::PassiveSubscription(format!(
                "workspace-command-delivery::{}::{}",
                watermark.ledger_id, watermark.committed_seq
            ))],
        )
        .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
    }

    pub(crate) fn resolves_wake(
        &self,
        wake_ref: &StructuralWakeRef,
    ) -> Result<bool, RuntimeAssemblyError> {
        if self.fence_receipt_id.is_some() || self.stop_receipt_id.is_some() {
            return Ok(false);
        }
        let watermark = self.watermark()?;
        Ok(match wake_ref {
            StructuralWakeRef::PassiveSubscription(value) => value.starts_with(&format!(
                "workspace-command-delivery::{}::",
                watermark.ledger_id
            )),
            StructuralWakeRef::EventPosition(value) => {
                value.starts_with(&format!("event-ledger::{}::after::", watermark.ledger_id))
            }
            _ => false,
        })
    }

    pub(crate) fn fence(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<PassiveFenceReceiptV1, RuntimeAssemblyError> {
        let watermark = self.watermark()?;
        let checkpoint =
            self.checkpoint_ref(watermark.ledger_id.to_string(), watermark.committed_seq);
        let receipt = PassiveFenceReceiptV1::new(
            context,
            checkpoint.clone(),
            format!("workspace-source-fence::{checkpoint}"),
        )
        .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))?;
        self.fence_receipt_id = Some(receipt.receipt_id.clone());
        self.fence_checkpoint_ref = Some(checkpoint);
        Ok(receipt)
    }

    pub(crate) fn stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
        let fence_receipt_id = self.fence_receipt_id.as_ref().ok_or_else(|| {
            RuntimeAssemblyError::SupervisorHandoff(
                "workspace source stop precedes its native fence".to_string(),
            )
        })?;
        let watermark = self.watermark()?;
        let checkpoint =
            self.checkpoint_ref(watermark.ledger_id.to_string(), watermark.committed_seq);
        if self.fence_checkpoint_ref.as_deref() != Some(checkpoint.as_str()) {
            return Err(RuntimeAssemblyError::SupervisorHandoff(
                "workspace source advanced beyond its installed fence".to_string(),
            ));
        }
        let receipt = OwnerStopReceiptV1::new(
            context,
            checkpoint.clone(),
            format!("workspace-source-stop::{fence_receipt_id}"),
        )
        .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))?;
        self.stop_receipt_id = Some(receipt.receipt_id.clone());
        self.stop_checkpoint_ref = Some(checkpoint);
        Ok(receipt)
    }

    pub(crate) fn release(
        &self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
        let stop_receipt_id = self.stop_receipt_id.as_ref().ok_or_else(|| {
            RuntimeAssemblyError::SupervisorHandoff(
                "workspace source release precedes its native stop".to_string(),
            )
        })?;
        let watermark = self.watermark()?;
        let checkpoint =
            self.checkpoint_ref(watermark.ledger_id.to_string(), watermark.committed_seq);
        if self.stop_checkpoint_ref.as_deref() != Some(checkpoint.as_str()) {
            return Err(RuntimeAssemblyError::SupervisorHandoff(
                "workspace source advanced after native stop".to_string(),
            ));
        }
        OwnerReleaseReceiptV1::new(
            context,
            format!("workspace-source-release::{stop_receipt_id}"),
        )
        .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
    }

    fn watermark(&self) -> Result<meld_events::EventWatermark, RuntimeAssemblyError> {
        self.event_append
            .watermark()
            .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
    }

    fn checkpoint_ref(&self, ledger_id: String, committed_seq: u64) -> String {
        format!("workspace-source::{ledger_id}::{committed_seq}")
    }
}
