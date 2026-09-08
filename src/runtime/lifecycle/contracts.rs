use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::theory::{ActivationParticipantSpec, ParticipantKind, PreparedActivationClosureV1};

/// Requested assignment lifecycle transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleAction {
    /// Create the first generation for an assignment.
    Activate,
    /// Prepare a successor to the current generation.
    Replace,
    /// Reconstruct a non-terminal generation after process loss.
    Recover,
}

/// Result classification for one idempotent lifecycle acceptance request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleAcceptanceOutcome {
    /// A new durable decision was accepted.
    Accepted,
    /// The exact prior accepted decision was returned.
    Duplicate,
    /// The request key already names different inputs.
    Conflicted,
    /// The transition is not valid from current durable state.
    Rejected,
}

/// Exact lifecycle request over one inert prepared closure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationLifecycleRequestV1 {
    /// Idempotency identity chosen by the initiating adapter.
    pub request_key: String,
    /// Requested lifecycle operation.
    pub action: LifecycleAction,
    /// Complete inert product closure accepted for realization.
    pub prepared: PreparedActivationClosureV1,
    /// Assignment head the caller expects before current publication.
    pub expected_prior_generation: Option<String>,
}

impl ActivationLifecycleRequestV1 {
    /// Build and validate one exact lifecycle request.
    pub fn new(
        request_key: String,
        action: LifecycleAction,
        prepared: PreparedActivationClosureV1,
        expected_prior_generation: Option<String>,
    ) -> Result<Self, LifecycleError> {
        if request_key.trim().is_empty() {
            return Err(LifecycleError::Invalid(
                "lifecycle request key is empty".to_string(),
            ));
        }
        prepared
            .verify_identity()
            .map_err(|error| LifecycleError::Invalid(error.to_string()))?;
        Ok(Self {
            request_key,
            action,
            prepared,
            expected_prior_generation,
        })
    }
}

/// Durable lifecycle decision for one request key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationLifecycleDecisionV1 {
    /// Content identity of the accepted request.
    pub decision_id: String,
    /// Idempotency request key.
    pub request_key: String,
    /// Requested action.
    pub action: LifecycleAction,
    /// Exact prepared closure identity.
    pub prepared_id: String,
    /// Expected assignment head supplied by the caller.
    pub expected_prior_generation: Option<String>,
    /// Generation created or resumed by an accepted decision.
    pub generation_id: Option<String>,
    /// Durable original outcome.
    pub outcome: LifecycleAcceptanceOutcome,
}

/// Response from lifecycle acceptance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleAcceptance {
    /// Accepted decision or conflict account.
    pub decision: ActivationLifecycleDecisionV1,
    /// Whether the exact decision already existed.
    pub outcome: LifecycleAcceptanceOutcome,
}

/// Durable generation lifecycle position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationGenerationStatus {
    /// Generation exists under closed admission while realization begins.
    Preparing,
    /// Every realized participant has exact structural readiness.
    Ready,
    /// Assignment head and admission name this generation coherently.
    Current,
    /// A required incarnation was lost and admission is closed.
    Interrupted,
    /// Admission is closed while accepted obligations drain.
    Draining,
    /// Every owner and passive path reached a fenced safe point.
    FencedQuiescent,
    /// Participants stopped, leases released, and terminal evidence exists.
    Retired,
    /// Preparation failed before the generation became current.
    FailedBeforeCurrent,
}

/// Exact physical realization of one declared participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantRealizationV1 {
    /// Content identity of this realization.
    pub realization_id: String,
    /// Activation generation being realized.
    pub generation_id: String,
    /// Exact declared participant identity.
    pub participant_id: String,
    /// Native semantic owner.
    pub owner_domain: String,
    /// Declared participant kind.
    pub kind: ParticipantKind,
    /// Required position from the accepted plan.
    pub required: bool,
    /// Runtime implementation selected by composition.
    pub implementation_ref: String,
    /// Exact registration projection identity.
    pub registration_id: String,
}

impl ParticipantRealizationV1 {
    /// Build a realization bound to one exact participant specification.
    pub fn new(
        generation_id: String,
        specification: &ActivationParticipantSpec,
        implementation_ref: String,
        registration_id: String,
    ) -> Result<Self, LifecycleError> {
        if [implementation_ref.as_str(), registration_id.as_str()]
            .iter()
            .any(|value| value.trim().is_empty())
        {
            return Err(LifecycleError::Invalid(
                "participant realization identity is incomplete".to_string(),
            ));
        }
        let realization_id = content_hash(&(
            &generation_id,
            &specification.participant_id,
            &implementation_ref,
            &registration_id,
        ))?;
        Ok(Self {
            realization_id,
            generation_id,
            participant_id: specification.participant_id.clone(),
            owner_domain: specification.owner_domain.clone(),
            kind: specification.kind.clone(),
            required: specification.required,
            implementation_ref,
            registration_id,
        })
    }
}

/// One process or passive-source incarnation of a realized participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantIncarnationV1 {
    /// Content identity of this incarnation.
    pub incarnation_id: String,
    /// Owning generation.
    pub generation_id: String,
    /// Realized participant identity.
    pub participant_id: String,
    /// Exact realization identity.
    pub realization_id: String,
    /// Operational lease or passive binding reference.
    pub lease_ref: String,
    /// Monotonic ordinal within this participant and generation.
    pub incarnation_number: u64,
}

impl ParticipantIncarnationV1 {
    /// Build one immutable incarnation identity.
    pub fn new(
        generation_id: String,
        participant_id: String,
        realization_id: String,
        lease_ref: String,
        incarnation_number: u64,
    ) -> Result<Self, LifecycleError> {
        if [
            participant_id.as_str(),
            realization_id.as_str(),
            lease_ref.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
            || incarnation_number == 0
        {
            return Err(LifecycleError::Invalid(
                "participant incarnation identity is incomplete".to_string(),
            ));
        }
        let incarnation_id = content_hash(&(
            &generation_id,
            &participant_id,
            &realization_id,
            &lease_ref,
            incarnation_number,
        ))?;
        Ok(Self {
            incarnation_id,
            generation_id,
            participant_id,
            realization_id,
            lease_ref,
            incarnation_number,
        })
    }
}

/// Exact structural context supplied to one native lifecycle owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantLifecycleContextV1 {
    /// Owning generation.
    pub generation_id: String,
    /// Exact current incarnation.
    pub incarnation_id: String,
    /// Exact physical realization.
    pub realization_id: String,
    /// Declared participant identity.
    pub participant_id: String,
    /// Native semantic owner.
    pub owner_domain: String,
    /// Declared participant kind.
    pub kind: ParticipantKind,
    /// Exact readiness contract.
    pub readiness_contract_ref: String,
    /// Exact wake contract.
    pub wake_contract_ref: String,
    /// Exact safe-point contract.
    pub safe_point_contract_ref: String,
    /// Exact stop contract.
    pub stop_contract_ref: String,
    /// Operational lease or passive binding owned by this incarnation.
    pub lease_ref: String,
}

impl ParticipantLifecycleContextV1 {
    /// Bind one declared participant to its realized incarnation.
    pub fn new(
        specification: &ActivationParticipantSpec,
        realization: &ParticipantRealizationV1,
        incarnation: &ParticipantIncarnationV1,
    ) -> Result<Self, LifecycleError> {
        if realization.participant_id != specification.participant_id
            || realization.owner_domain != specification.owner_domain
            || realization.kind != specification.kind
            || incarnation.participant_id != specification.participant_id
            || incarnation.realization_id != realization.realization_id
            || incarnation.generation_id != realization.generation_id
        {
            return Err(LifecycleError::Conflict(
                "participant lifecycle context is structurally inconsistent".to_string(),
            ));
        }
        Ok(Self {
            generation_id: incarnation.generation_id.clone(),
            incarnation_id: incarnation.incarnation_id.clone(),
            realization_id: realization.realization_id.clone(),
            participant_id: specification.participant_id.clone(),
            owner_domain: specification.owner_domain.clone(),
            kind: specification.kind.clone(),
            readiness_contract_ref: specification.readiness_contract_ref.clone(),
            wake_contract_ref: specification.wake_contract_ref.clone(),
            safe_point_contract_ref: specification.safe_point_contract_ref.clone(),
            stop_contract_ref: specification.stop_contract_ref.clone(),
            lease_ref: incarnation.lease_ref.clone(),
        })
    }
}

/// Evidence produced by a native owner after reconstructing its durable state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeOwnerReadinessEvidenceV1 {
    /// Native participant producing this evidence.
    pub participant_id: String,
    /// Native semantic owner producing this evidence.
    pub owner_domain: String,
    /// Native durable checkpoint or source position.
    pub owner_checkpoint_ref: String,
    /// Exact installed owner revisions inspected during reconstruction.
    pub installed_revision_refs: Vec<String>,
    /// Exact physical or authority bindings inspected by the owner.
    pub binding_refs: Vec<String>,
    /// Exact subscriptions or durable input positions required by the owner.
    pub subscription_refs: Vec<String>,
    /// Owner-specific position proving the reconstruction result.
    pub proof_position_ref: String,
}

impl NativeOwnerReadinessEvidenceV1 {
    /// Build complete owner-produced readiness evidence.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        participant_id: String,
        owner_domain: String,
        owner_checkpoint_ref: String,
        mut installed_revision_refs: Vec<String>,
        mut binding_refs: Vec<String>,
        mut subscription_refs: Vec<String>,
        proof_position_ref: String,
    ) -> Result<Self, LifecycleError> {
        for refs in [
            &mut installed_revision_refs,
            &mut binding_refs,
            &mut subscription_refs,
        ] {
            refs.sort();
            refs.dedup();
        }
        if [
            participant_id.as_str(),
            owner_domain.as_str(),
            owner_checkpoint_ref.as_str(),
            proof_position_ref.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
            || [
                installed_revision_refs.as_slice(),
                binding_refs.as_slice(),
                subscription_refs.as_slice(),
            ]
            .iter()
            .any(|refs| refs.is_empty() || refs.iter().any(|value| value.trim().is_empty()))
        {
            return Err(LifecycleError::Invalid(
                "native owner readiness evidence is incomplete".to_string(),
            ));
        }
        Ok(Self {
            participant_id,
            owner_domain,
            owner_checkpoint_ref,
            installed_revision_refs,
            binding_refs,
            subscription_refs,
            proof_position_ref,
        })
    }
}

/// Native-owner structural readiness evidence bound to one incarnation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerReadinessReceiptV1 {
    /// Content identity of this readiness receipt.
    pub receipt_id: String,
    /// Owning generation.
    pub generation_id: String,
    /// Exact participant incarnation.
    pub incarnation_id: String,
    /// Exact realization.
    pub realization_id: String,
    /// Exact participant producing the evidence.
    pub participant_id: String,
    /// Native owner named by the participant plan.
    pub owner_domain: String,
    /// Readiness contract named by the participant plan.
    pub readiness_contract_ref: String,
    /// Complete evidence returned by the native owner seam.
    pub native_evidence: NativeOwnerReadinessEvidenceV1,
}

impl OwnerReadinessReceiptV1 {
    /// Bind native evidence to the exact lifecycle context.
    pub fn from_native(
        context: &ParticipantLifecycleContextV1,
        native_evidence: NativeOwnerReadinessEvidenceV1,
    ) -> Result<Self, LifecycleError> {
        if native_evidence.participant_id != context.participant_id
            || native_evidence.owner_domain != context.owner_domain
        {
            return Err(LifecycleError::Conflict(
                "native readiness evidence belongs to another owner".to_string(),
            ));
        }
        let receipt_id = content_hash(&(
            &context.generation_id,
            &context.incarnation_id,
            &context.realization_id,
            &context.participant_id,
            &context.owner_domain,
            &context.readiness_contract_ref,
            &native_evidence,
        ))?;
        Ok(Self {
            receipt_id,
            generation_id: context.generation_id.clone(),
            incarnation_id: context.incarnation_id.clone(),
            realization_id: context.realization_id.clone(),
            participant_id: context.participant_id.clone(),
            owner_domain: context.owner_domain.clone(),
            readiness_contract_ref: context.readiness_contract_ref.clone(),
            native_evidence,
        })
    }
}

/// Exact participant-plan to registration projection receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistrationParityReceiptV1 {
    /// Content identity of the parity proof.
    pub receipt_id: String,
    /// Owning generation.
    pub generation_id: String,
    /// Accepted participant plan identity.
    pub participant_plan_id: String,
    /// Realization identities in canonical participant order.
    pub realization_ids: Vec<String>,
    /// Registration identities in canonical participant order.
    pub registration_ids: Vec<String>,
}

/// Admission epoch status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionEpochStatus {
    /// New owner work may enter under this epoch.
    Open,
    /// New work is fenced while prior accepted work remains attributable.
    Closed,
}

/// Durable admission epoch bound to one assignment head.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionEpochV1 {
    /// Content identity of this epoch.
    pub epoch_id: String,
    /// Owning generation.
    pub generation_id: String,
    /// Monotonic generation-local epoch number.
    pub epoch_number: u64,
    /// Current fence state.
    pub status: AdmissionEpochStatus,
    /// Transition that opened or closed the epoch.
    pub transition_ref: String,
}

/// Structural wake address published by a native owner.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralWakeRef {
    /// Durable Event position or watermark.
    EventPosition(String),
    /// Durable native-owner revision.
    OwnerRevision(String),
    /// Durable operation completion position.
    DurableOperation(String),
    /// Durable deadline.
    DurableDeadline(String),
    /// Passive source subscription position.
    PassiveSubscription(String),
    /// Recovery of a physical or authority binding.
    BindingRecovery(String),
    /// Explicit operator action channel.
    OperatorAction(String),
}

impl StructuralWakeRef {
    fn address(&self) -> &str {
        match self {
            Self::EventPosition(value)
            | Self::OwnerRevision(value)
            | Self::DurableOperation(value)
            | Self::DurableDeadline(value)
            | Self::PassiveSubscription(value)
            | Self::BindingRecovery(value)
            | Self::OperatorAction(value) => value,
        }
    }
}

/// Native-owner no-work account at one exact checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerWaitReceiptV1 {
    /// Content identity of the wait.
    pub wait_ref: String,
    /// Owning generation.
    pub generation_id: String,
    /// Exact participant incarnation.
    pub incarnation_id: String,
    /// Native durable checkpoint.
    pub owner_checkpoint_ref: String,
    /// Native-owner reason code, never interpreted by lifecycle.
    pub reason_code: String,
    /// Complete structural addresses that can change eligibility.
    pub wake_refs: Vec<StructuralWakeRef>,
}

impl OwnerWaitReceiptV1 {
    /// Build one complete native-owner wait account.
    pub fn new(
        generation_id: String,
        incarnation_id: String,
        owner_checkpoint_ref: String,
        reason_code: String,
        mut wake_refs: Vec<StructuralWakeRef>,
    ) -> Result<Self, LifecycleError> {
        wake_refs.sort();
        wake_refs.dedup();
        if [
            incarnation_id.as_str(),
            owner_checkpoint_ref.as_str(),
            reason_code.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
            || wake_refs.is_empty()
            || wake_refs
                .iter()
                .any(|wake| wake.address().trim().is_empty())
        {
            return Err(LifecycleError::Invalid(
                "owner wait evidence is incomplete".to_string(),
            ));
        }
        let wait_ref = content_hash(&(
            &generation_id,
            &incarnation_id,
            &owner_checkpoint_ref,
            &reason_code,
            &wake_refs,
        ))?;
        Ok(Self {
            wait_ref,
            generation_id,
            incarnation_id,
            owner_checkpoint_ref,
            reason_code,
            wake_refs,
        })
    }
}

/// Native-owner safe point under closed admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerSafePointReceiptV1 {
    /// Content identity of the owner safe point.
    pub receipt_id: String,
    /// Owning generation.
    pub generation_id: String,
    /// Exact participant incarnation.
    pub incarnation_id: String,
    /// Exact participant producing the safe point.
    pub participant_id: String,
    /// Native semantic owner.
    pub owner_domain: String,
    /// Safe-point contract from the accepted plan.
    pub safe_point_contract_ref: String,
    /// Native durable checkpoint after drain.
    pub owner_checkpoint_ref: String,
    /// Native summary of any durably attributable uncertain effects.
    pub unresolved_operation_summary_ref: String,
    /// Owner-specific durable proof positions supporting the safe point.
    pub proof_refs: Vec<String>,
}

impl OwnerSafePointReceiptV1 {
    /// Build one exact owner safe-point receipt.
    pub fn new(
        context: &ParticipantLifecycleContextV1,
        owner_checkpoint_ref: String,
        unresolved_operation_summary_ref: String,
        mut proof_refs: Vec<String>,
    ) -> Result<Self, LifecycleError> {
        proof_refs.sort();
        proof_refs.dedup();
        if [
            owner_checkpoint_ref.as_str(),
            unresolved_operation_summary_ref.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
            || proof_refs.is_empty()
            || proof_refs.iter().any(|value| value.trim().is_empty())
        {
            return Err(LifecycleError::Invalid(
                "owner safe-point evidence is incomplete".to_string(),
            ));
        }
        let receipt_id = content_hash(&(
            &context.generation_id,
            &context.incarnation_id,
            &context.participant_id,
            &context.owner_domain,
            &context.safe_point_contract_ref,
            &owner_checkpoint_ref,
            &unresolved_operation_summary_ref,
            &proof_refs,
        ))?;
        Ok(Self {
            receipt_id,
            generation_id: context.generation_id.clone(),
            incarnation_id: context.incarnation_id.clone(),
            participant_id: context.participant_id.clone(),
            owner_domain: context.owner_domain.clone(),
            safe_point_contract_ref: context.safe_point_contract_ref.clone(),
            owner_checkpoint_ref,
            unresolved_operation_summary_ref,
            proof_refs,
        })
    }
}

/// Native acknowledgement that one participant accepted its exact stop contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerStopReceiptV1 {
    /// Content identity of the stop receipt.
    pub receipt_id: String,
    /// Owning generation.
    pub generation_id: String,
    /// Exact participant incarnation.
    pub incarnation_id: String,
    /// Exact participant.
    pub participant_id: String,
    /// Native semantic owner.
    pub owner_domain: String,
    /// Stop contract from the accepted plan.
    pub stop_contract_ref: String,
    /// Native checkpoint where stop was accepted.
    pub owner_checkpoint_ref: String,
    /// Native proof that the stop hook completed.
    pub owner_proof_ref: String,
}

impl OwnerStopReceiptV1 {
    /// Build a native stop receipt for one exact lifecycle context.
    pub fn new(
        context: &ParticipantLifecycleContextV1,
        owner_checkpoint_ref: String,
        owner_proof_ref: String,
    ) -> Result<Self, LifecycleError> {
        if owner_checkpoint_ref.trim().is_empty() || owner_proof_ref.trim().is_empty() {
            return Err(LifecycleError::Invalid(
                "native owner stop evidence is incomplete".to_string(),
            ));
        }
        let receipt_id = content_hash(&(
            &context.generation_id,
            &context.incarnation_id,
            &context.participant_id,
            &context.owner_domain,
            &context.stop_contract_ref,
            &owner_checkpoint_ref,
            &owner_proof_ref,
        ))?;
        Ok(Self {
            receipt_id,
            generation_id: context.generation_id.clone(),
            incarnation_id: context.incarnation_id.clone(),
            participant_id: context.participant_id.clone(),
            owner_domain: context.owner_domain.clone(),
            stop_contract_ref: context.stop_contract_ref.clone(),
            owner_checkpoint_ref,
            owner_proof_ref,
        })
    }
}

/// Native passive-owner fence over one durable delivery position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassiveFenceReceiptV1 {
    /// Content identity of the passive fence.
    pub receipt_id: String,
    /// Owning generation.
    pub generation_id: String,
    /// Exact passive incarnation.
    pub incarnation_id: String,
    /// Exact passive participant.
    pub participant_id: String,
    /// Native source owner.
    pub owner_domain: String,
    /// Wake contract whose delivery position is fenced.
    pub wake_contract_ref: String,
    /// Native durable source position at the fence.
    pub source_checkpoint_ref: String,
    /// Owner-produced proof of the installed fence.
    pub owner_proof_ref: String,
}

impl PassiveFenceReceiptV1 {
    /// Build one exact passive-owner fence receipt.
    pub fn new(
        context: &ParticipantLifecycleContextV1,
        source_checkpoint_ref: String,
        owner_proof_ref: String,
    ) -> Result<Self, LifecycleError> {
        if context.kind != ParticipantKind::PassiveSource
            || source_checkpoint_ref.trim().is_empty()
            || owner_proof_ref.trim().is_empty()
        {
            return Err(LifecycleError::Invalid(
                "native passive fence evidence is incomplete".to_string(),
            ));
        }
        let receipt_id = content_hash(&(
            &context.generation_id,
            &context.incarnation_id,
            &context.participant_id,
            &context.owner_domain,
            &context.wake_contract_ref,
            &source_checkpoint_ref,
            &owner_proof_ref,
        ))?;
        Ok(Self {
            receipt_id,
            generation_id: context.generation_id.clone(),
            incarnation_id: context.incarnation_id.clone(),
            participant_id: context.participant_id.clone(),
            owner_domain: context.owner_domain.clone(),
            wake_contract_ref: context.wake_contract_ref.clone(),
            source_checkpoint_ref,
            owner_proof_ref,
        })
    }
}

/// Native acknowledgement that owner resources for one incarnation were released.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerReleaseReceiptV1 {
    /// Content identity of the release receipt.
    pub receipt_id: String,
    /// Owning generation.
    pub generation_id: String,
    /// Exact participant incarnation.
    pub incarnation_id: String,
    /// Exact participant.
    pub participant_id: String,
    /// Native semantic owner.
    pub owner_domain: String,
    /// Lease or passive binding released by the owner.
    pub released_binding_ref: String,
    /// Native proof that release completed.
    pub owner_proof_ref: String,
}

impl OwnerReleaseReceiptV1 {
    /// Build one exact native release receipt.
    pub fn new(
        context: &ParticipantLifecycleContextV1,
        owner_proof_ref: String,
    ) -> Result<Self, LifecycleError> {
        if owner_proof_ref.trim().is_empty() {
            return Err(LifecycleError::Invalid(
                "native owner release evidence is incomplete".to_string(),
            ));
        }
        let receipt_id = content_hash(&(
            &context.generation_id,
            &context.incarnation_id,
            &context.participant_id,
            &context.owner_domain,
            &context.lease_ref,
            &owner_proof_ref,
        ))?;
        Ok(Self {
            receipt_id,
            generation_id: context.generation_id.clone(),
            incarnation_id: context.incarnation_id.clone(),
            participant_id: context.participant_id.clone(),
            owner_domain: context.owner_domain.clone(),
            released_binding_ref: context.lease_ref.clone(),
            owner_proof_ref,
        })
    }
}

/// Aggregate structural closure under closed admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FencedQuiescenceReceiptV1 {
    /// Content identity of the aggregate receipt.
    pub receipt_id: String,
    /// Owning generation.
    pub generation_id: String,
    /// Closed admission epoch.
    pub admission_epoch_id: String,
    /// One native safe point for every realized active participant.
    pub owner_safe_point_receipt_ids: Vec<String>,
    /// One fence for every realized passive participant.
    pub passive_fence_receipt_ids: Vec<String>,
}

/// Immutable terminal generation evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetirementReceiptV1 {
    /// Content identity of terminal evidence.
    pub receipt_id: String,
    /// Retired generation.
    pub generation_id: String,
    /// Aggregate fenced-quiescence evidence.
    pub fenced_quiescence_receipt_id: String,
    /// Stop receipts in reverse structural dependency order.
    pub stop_receipts: Vec<OwnerStopReceiptV1>,
    /// Released supervisor lease or passive binding references.
    pub release_receipts: Vec<OwnerReleaseReceiptV1>,
}

/// Complete durable state of one activation generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationGenerationV1 {
    /// Content identity of this generation.
    pub generation_id: String,
    /// Monotonic assignment-local ordinal.
    pub generation_number: u64,
    /// Assignment identity.
    pub assignment_id: String,
    /// Exact activation input identity.
    pub activation_id: String,
    /// Exact prepared closure identity.
    pub prepared_id: String,
    /// Exact accepted participant plan.
    pub participant_plan_id: String,
    /// Exact participant specifications keyed by participant identity.
    pub participant_specs: BTreeMap<String, ActivationParticipantSpec>,
    /// Expected predecessor at publication.
    pub expected_prior_generation: Option<String>,
    /// Current lifecycle position.
    pub status: ActivationGenerationStatus,
    /// Exact physical realization by participant id.
    pub realizations: BTreeMap<String, ParticipantRealizationV1>,
    /// Current incarnation by participant id.
    pub incarnations: BTreeMap<String, ParticipantIncarnationV1>,
    /// Current native-owner readiness by participant id.
    pub readiness: BTreeMap<String, OwnerReadinessReceiptV1>,
    /// Exact registration parity once complete.
    pub registration_parity: Option<RegistrationParityReceiptV1>,
    /// Admission epoch history.
    pub admission_epochs: Vec<AdmissionEpochV1>,
    /// Latest native-owner wait by participant id.
    pub waits: BTreeMap<String, OwnerWaitReceiptV1>,
    /// Native-owner safe points by participant id.
    pub safe_points: BTreeMap<String, OwnerSafePointReceiptV1>,
    /// Native passive fences by participant id.
    pub passive_fences: BTreeMap<String, PassiveFenceReceiptV1>,
    /// Native stop receipts by participant id.
    pub stop_receipts: BTreeMap<String, OwnerStopReceiptV1>,
    /// Participants in the exact order their native stop hooks completed.
    pub stop_order: Vec<String>,
    /// Native release receipts by participant id.
    pub release_receipts: BTreeMap<String, OwnerReleaseReceiptV1>,
    /// Aggregate fenced closure.
    pub fenced_quiescence: Option<FencedQuiescenceReceiptV1>,
    /// Immutable terminal evidence.
    pub retirement: Option<RetirementReceiptV1>,
    /// Last durable transition identity.
    pub last_transition_ref: String,
}

impl ActivationGenerationV1 {
    /// Return the current admission epoch when one exists.
    pub fn current_admission_epoch(&self) -> Option<&AdmissionEpochV1> {
        self.admission_epochs.last()
    }

    /// Return whether new work may enter this exact generation and epoch.
    pub fn admission_open(&self) -> bool {
        self.status == ActivationGenerationStatus::Current
            && self
                .current_admission_epoch()
                .is_some_and(|epoch| epoch.status == AdmissionEpochStatus::Open)
    }
}

/// One assignment's complete lifecycle authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignmentLifecycleV1 {
    /// Assignment identity.
    pub assignment_id: String,
    /// Monotonic optimistic-concurrency revision of this aggregate.
    #[serde(default)]
    pub lifecycle_revision: u64,
    /// Sole current generation head.
    pub current_generation_id: Option<String>,
    /// All generation accounts by immutable identity.
    pub generations: BTreeMap<String, ActivationGenerationV1>,
}

/// Truthful assignment-wide liveness projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectedLifecycleState {
    /// At least one owner reports eligible or accepted work.
    ActiveWork,
    /// Every observed owner pass was clean but wait closure is incomplete.
    ActiveIdle,
    /// Every realized owner has a complete wait and every wake resolves.
    Quiescent,
    /// Required participant, wait, or wake closure is missing.
    Stalled,
    /// Admission is fenced for recovery.
    Interrupted,
    /// Admission is closed for drain.
    Draining,
    /// Generation is terminal.
    Retired,
}

/// Assignment liveness without semantic interpretation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignmentLifecycleProjectionV1 {
    /// Assignment identity.
    pub assignment_id: String,
    /// Current generation when present.
    pub generation_id: Option<String>,
    /// Structurally derived lifecycle state.
    pub state: ProjectedLifecycleState,
    /// Required participants without current readiness or waits.
    pub incomplete_participants: Vec<String>,
    /// Wake references without a present structural resolver.
    pub broken_wake_refs: Vec<StructuralWakeRef>,
    /// Last durable lifecycle transition.
    pub last_transition_ref: String,
}

/// Lifecycle contract or persistence failure.
#[derive(Debug, thiserror::Error)]
pub enum LifecycleError {
    /// Supplied structural identity is incomplete or inconsistent.
    #[error("lifecycle_invalid: {0}")]
    Invalid(String),
    /// Durable state conflicts with the requested transition.
    #[error("lifecycle_conflict: {0}")]
    Conflict(String),
    /// Storage or encoding failed.
    #[error("lifecycle_storage: {0}")]
    Storage(String),
}

pub(crate) fn content_hash(value: &impl Serialize) -> Result<String, LifecycleError> {
    bincode::serialize(value)
        .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
        .map_err(|error| LifecycleError::Storage(error.to_string()))
}
