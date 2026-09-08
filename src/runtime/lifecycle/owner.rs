//! Native owner lifecycle contract shared by in-process and serialized participants.

use super::*;
use crate::runtime::contracts::WorkerTickReport;
use crate::runtime::error::RuntimeAssemblyError;

#[derive(Debug, Clone)]
pub struct NativeOwnerLifecycleSnapshot {
    checkpoint_ref: String,
    installed_revision_refs: Vec<String>,
    binding_refs: Vec<String>,
    subscription_refs: Vec<String>,
    proof_position_ref: String,
    unresolved_operation_summary_ref: String,
}

impl From<meld_world_model::lifecycle::NativeLifecycleEvidence> for NativeOwnerLifecycleSnapshot {
    fn from(evidence: meld_world_model::lifecycle::NativeLifecycleEvidence) -> Self {
        Self {
            checkpoint_ref: evidence.checkpoint_ref,
            installed_revision_refs: evidence.installed_revision_refs,
            binding_refs: evidence.binding_refs,
            subscription_refs: evidence.subscription_refs,
            proof_position_ref: evidence.proof_position_ref,
            unresolved_operation_summary_ref: evidence.unresolved_operation_summary_ref,
        }
    }
}
impl From<meld_execution::lifecycle::NativeLifecycleEvidence> for NativeOwnerLifecycleSnapshot {
    fn from(evidence: meld_execution::lifecycle::NativeLifecycleEvidence) -> Self {
        Self {
            checkpoint_ref: evidence.checkpoint_ref,
            installed_revision_refs: evidence.installed_revision_refs,
            binding_refs: evidence.binding_refs,
            subscription_refs: evidence.subscription_refs,
            proof_position_ref: evidence.proof_position_ref,
            unresolved_operation_summary_ref: evidence.unresolved_operation_summary_ref,
        }
    }
}

pub trait NativeTransitionProof {
    fn generation_id(&self) -> &str;
    fn incarnation_id(&self) -> &str;
    fn proof_ref(&self) -> &str;
}

impl NativeTransitionProof for meld_world_model::lifecycle::NativeLifecycleTransition {
    fn generation_id(&self) -> &str {
        &self.generation_id
    }

    fn incarnation_id(&self) -> &str {
        &self.incarnation_id
    }

    fn proof_ref(&self) -> &str {
        &self.proof_ref
    }
}

impl NativeTransitionProof for meld_execution::lifecycle::NativeLifecycleTransition {
    fn generation_id(&self) -> &str {
        &self.generation_id
    }

    fn incarnation_id(&self) -> &str {
        &self.incarnation_id
    }

    fn proof_ref(&self) -> &str {
        &self.proof_ref
    }
}

pub fn verified_native_transition<T: NativeTransitionProof>(
    context: &ParticipantLifecycleContextV1,
    transition: T,
) -> Result<String, RuntimeAssemblyError> {
    if transition.generation_id() != context.generation_id
        || transition.incarnation_id() != context.incarnation_id
        || transition.proof_ref().trim().is_empty()
    {
        return Err(RuntimeAssemblyError::SupervisorHandoff(
            "native lifecycle transition belongs to another activation position".to_string(),
        ));
    }
    Ok(transition.proof_ref().to_string())
}

pub trait NativeOwnerLifecycle {
    fn native_snapshot(&self) -> Result<NativeOwnerLifecycleSnapshot, RuntimeAssemblyError>;
    fn native_readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError>;
    fn native_wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError>;
    fn native_safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError>;
    fn native_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError>;
    fn native_release(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError>;
    fn native_resolves_wake(
        &self,
        wake_ref: &StructuralWakeRef,
    ) -> Result<bool, RuntimeAssemblyError>;
}

pub fn owner_readiness_receipt(
    context: &ParticipantLifecycleContextV1,
    snapshot: NativeOwnerLifecycleSnapshot,
    transition_proof_ref: String,
) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
    let evidence = NativeOwnerReadinessEvidenceV1::new(
        context.participant_id.clone(),
        context.owner_domain.clone(),
        snapshot.checkpoint_ref,
        snapshot.installed_revision_refs,
        snapshot.binding_refs,
        snapshot.subscription_refs,
        transition_proof_ref,
    )
    .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))?;
    OwnerReadinessReceiptV1::from_native(context, evidence)
        .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
}

pub fn owner_wait_receipt(
    context: &ParticipantLifecycleContextV1,
    snapshot: NativeOwnerLifecycleSnapshot,
    report: &WorkerTickReport,
) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
    if report.made_progress()
        || !report.retryable_errors.is_empty()
        || !report.fatal_errors.is_empty()
        || report.budget_exhausted
    {
        return Err(RuntimeAssemblyError::SupervisorHandoff(
            "native owner cannot author a wait for an active or failed step".to_string(),
        ));
    }
    if report.waiting_on.is_empty()
        || report
            .waiting_on
            .iter()
            .any(|declaration| declaration.wake_refs.is_empty())
    {
        return Err(RuntimeAssemblyError::SupervisorHandoff(
            "native owner supplied incomplete wait evidence".to_string(),
        ));
    }
    let mut reasons = report
        .waiting_on
        .iter()
        .map(|wait| wait.condition.as_str())
        .collect::<Vec<_>>();
    reasons.sort_unstable();
    reasons.dedup();
    let reason = reasons.join("+");
    let wake_refs = report
        .waiting_on
        .iter()
        .flat_map(|wait| wait.wake_refs.iter().cloned())
        .collect();
    OwnerWaitReceiptV1::new(
        context.generation_id.clone(),
        context.incarnation_id.clone(),
        snapshot.checkpoint_ref,
        reason,
        wake_refs,
    )
    .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
}

pub fn owner_safe_point_receipt(
    context: &ParticipantLifecycleContextV1,
    snapshot: NativeOwnerLifecycleSnapshot,
    transition_proof_ref: String,
) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError> {
    OwnerSafePointReceiptV1::new(
        context,
        snapshot.checkpoint_ref,
        snapshot.unresolved_operation_summary_ref,
        vec![snapshot.proof_position_ref, transition_proof_ref],
    )
    .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
}

pub fn owner_stop_receipt(
    context: &ParticipantLifecycleContextV1,
    snapshot: NativeOwnerLifecycleSnapshot,
    transition_proof_ref: String,
) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
    OwnerStopReceiptV1::new(context, snapshot.checkpoint_ref, transition_proof_ref)
        .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
}

pub fn owner_release_receipt(
    context: &ParticipantLifecycleContextV1,
    _snapshot: NativeOwnerLifecycleSnapshot,
    transition_proof_ref: String,
) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
    OwnerReleaseReceiptV1::new(context, transition_proof_ref)
        .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
}
