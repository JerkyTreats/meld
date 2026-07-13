//! Process-hydration and operational-readiness contracts.

use meld_lang::Goal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::agent::AgentId;
use crate::belief::BeliefReadinessAttestation;
use crate::planner::{
    PlannerProjectionFrame, PlannerProjectionRequestRecord, PlannerProjectionRequestStatus,
};

const READINESS_SIGNAL_HASH_DOMAIN: &[u8] = b"meld.agent-readiness-signal.v2";
const READINESS_PROOF_HASH_DOMAIN: &[u8] = b"meld.agent-readiness-proof.v2";
const MAX_HYDRATION_ERROR_BYTES: usize = 1024;

/// One belief-owned attestation consumed as an operational readiness signal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentReadinessSignal {
    /// Deterministic signal identity derived from the attestation.
    pub signal_id: String,
    /// Belief-owned proof of the exact persisted current view.
    pub attestation: BeliefReadinessAttestation,
}

impl AgentReadinessSignal {
    /// Construct a readiness signal from one durable belief attestation.
    pub fn identified(
        attestation: BeliefReadinessAttestation,
    ) -> Result<Self, AgentHydrationContractError> {
        let mut signal = Self {
            signal_id: String::new(),
            attestation,
        };
        signal.signal_id = signal.derive_id()?;
        signal.validate()?;
        Ok(signal)
    }

    /// Validate the belief-owned product and deterministic signal identity.
    pub fn validate(&self) -> Result<(), AgentHydrationContractError> {
        self.attestation
            .validate()
            .map_err(|error| AgentHydrationContractError::Invalid(error.to_string()))?;
        if self.signal_id != self.derive_id()? {
            return Err(AgentHydrationContractError::IdentityMismatch(
                "readiness signal id".to_string(),
            ));
        }
        Ok(())
    }

    fn derive_id(&self) -> Result<String, AgentHydrationContractError> {
        semantic_hash(READINESS_SIGNAL_HASH_DOMAIN, &self.attestation)
    }
}

/// Complete proof required before one registered agent becomes operational.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentReadinessProof {
    /// Deterministic proof identity derived from every other field.
    pub proof_id: String,
    /// Agent update sequence that hydration read before evaluation.
    pub expected_agent_updated_at_seq: u64,
    /// Processed belief-owned readiness signal.
    pub signal: AgentReadinessSignal,
    /// Exact completed world-model projection request.
    pub planner_request: PlannerProjectionRequestRecord,
    /// Exact durable world-model projection frame.
    pub planner_frame: PlannerProjectionFrame,
    /// Ground goal constructed without submitting it to execution.
    pub ground_goal: Goal,
}

impl AgentReadinessProof {
    /// Identify one complete proof after belief and planner products exist.
    pub fn identified(
        expected_agent_updated_at_seq: u64,
        signal: AgentReadinessSignal,
        planner_request: PlannerProjectionRequestRecord,
        planner_frame: PlannerProjectionFrame,
        ground_goal: Goal,
    ) -> Result<Self, AgentHydrationContractError> {
        let mut proof = Self {
            proof_id: String::new(),
            expected_agent_updated_at_seq,
            signal,
            planner_request,
            planner_frame,
            ground_goal,
        };
        proof.proof_id = proof.derive_id()?;
        proof.validate()?;
        Ok(proof)
    }

    /// Validate exact belief, planner, agent, and goal relationships.
    pub fn validate(&self) -> Result<(), AgentHydrationContractError> {
        self.signal.validate()?;
        self.planner_request
            .validate()
            .map_err(|error| AgentHydrationContractError::Invalid(error.to_string()))?;
        self.planner_frame
            .validate()
            .map_err(|error| AgentHydrationContractError::Invalid(error.to_string()))?;
        if self.planner_request.status != PlannerProjectionRequestStatus::Completed
            || self.planner_request.frame_id.as_deref()
                != Some(self.planner_frame.identity.frame_id.as_str())
            || self.planner_request.request.request_id != self.planner_frame.identity.request_id
            || self.planner_request.request.source_request_hash
                != self.planner_frame.identity.source_request_hash
        {
            return Err(AgentHydrationContractError::PlannerProductMismatch);
        }
        let attestation = &self.signal.attestation;
        let request = &self.planner_request.request;
        if request.agent_id != attestation.agent_id
            || request.subject != attestation.belief_key.subject
            || request.perspective != attestation.belief_key.perspective
            || request.branch_scope != attestation.belief_key.branch_scope
        {
            return Err(AgentHydrationContractError::PlannerScopeMismatch);
        }
        require_non_empty("ground goal id", &self.ground_goal.goal_id)?;
        if self.ground_goal.agent_id != attestation.agent_id {
            return Err(AgentHydrationContractError::AgentMismatch);
        }
        if !self.ground_goal.target.is_ground() {
            return Err(AgentHydrationContractError::GoalNotGround);
        }
        if self.proof_id != self.derive_id()? {
            return Err(AgentHydrationContractError::IdentityMismatch(
                "readiness proof id".to_string(),
            ));
        }
        Ok(())
    }

    fn derive_id(&self) -> Result<String, AgentHydrationContractError> {
        #[derive(Serialize)]
        struct Identity<'a> {
            expected_agent_updated_at_seq: u64,
            signal: &'a AgentReadinessSignal,
            planner_request: &'a PlannerProjectionRequestRecord,
            planner_frame: &'a PlannerProjectionFrame,
            ground_goal: &'a Goal,
        }
        semantic_hash(
            READINESS_PROOF_HASH_DOMAIN,
            &Identity {
                expected_agent_updated_at_seq: self.expected_agent_updated_at_seq,
                signal: &self.signal,
                planner_request: &self.planner_request,
                planner_frame: &self.planner_frame,
                ground_goal: &self.ground_goal,
            },
        )
    }
}

/// Durable state of one process-hydration attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentProcessHydrationStatus {
    /// Hydration has begun but readiness has not been proven.
    Started,
    /// Every readiness gate has a durable proof.
    Ready,
    /// Hydration stopped on a bounded failure.
    Failed,
}

/// Durable diagnostic record for one process-hydration attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentProcessHydrationRecord {
    /// Stable hydration-attempt identity.
    pub hydration_id: String,
    /// Agent whose existing durable state is being hydrated.
    pub agent_id: AgentId,
    /// Domain-owned epoch that fences stale hydration workers.
    pub attempt_epoch: u64,
    /// Current hydration state.
    pub status: AgentProcessHydrationStatus,
    /// Readiness proof accepted by the completed attempt.
    pub readiness_proof_id: Option<String>,
    /// Runtime lease hosting this hydration attempt.
    pub lease_id: String,
    /// Bounded failure detail for a failed attempt.
    pub last_error: Option<String>,
    /// Sequence at which this attempt began.
    pub started_at_seq: u64,
    /// Last durable update sequence for this attempt.
    pub updated_at_seq: u64,
}

impl AgentProcessHydrationRecord {
    /// Validate identity, epoch, lease, status products, and sequences.
    pub fn validate(&self) -> Result<(), AgentHydrationContractError> {
        require_non_empty("hydration id", &self.hydration_id)?;
        require_non_empty("hydration agent id", &self.agent_id)?;
        require_non_empty("hydration lease id", &self.lease_id)?;
        if self.attempt_epoch == 0
            || self.started_at_seq == 0
            || self.updated_at_seq < self.started_at_seq
        {
            return Err(AgentHydrationContractError::SequenceRegression);
        }
        match self.status {
            AgentProcessHydrationStatus::Started => {
                if self.readiness_proof_id.is_some() || self.last_error.is_some() {
                    return Err(AgentHydrationContractError::InvalidStatusProducts);
                }
            }
            AgentProcessHydrationStatus::Ready => {
                if self.readiness_proof_id.as_deref().is_none_or(str::is_empty)
                    || self.last_error.is_some()
                {
                    return Err(AgentHydrationContractError::InvalidStatusProducts);
                }
            }
            AgentProcessHydrationStatus::Failed => {
                if self.last_error.as_deref().is_none_or(str::is_empty)
                    || self.readiness_proof_id.is_some()
                    || self
                        .last_error
                        .as_ref()
                        .is_some_and(|error| error.len() > MAX_HYDRATION_ERROR_BYTES)
                {
                    return Err(AgentHydrationContractError::InvalidStatusProducts);
                }
            }
        }
        Ok(())
    }
}

/// Fenced command that begins one new hydration attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartAgentHydrationCommand {
    /// Stable identity for the new hydration attempt.
    pub hydration_id: String,
    /// Existing registered agent to hydrate.
    pub agent_id: AgentId,
    /// Current epoch expected before this attempt starts.
    pub expected_prior_attempt_epoch: u64,
    /// New domain-owned attempt epoch.
    pub attempt_epoch: u64,
    /// Supervisor lease hosting the process-local attempt.
    pub lease_id: String,
    /// Sequence assigned to the start transition.
    pub started_at_seq: u64,
}

impl StartAgentHydrationCommand {
    /// Validate identity and the exact next-epoch transition.
    pub fn validate(&self) -> Result<(), AgentHydrationContractError> {
        require_non_empty("hydration id", &self.hydration_id)?;
        require_non_empty("hydration agent id", &self.agent_id)?;
        require_non_empty("hydration lease id", &self.lease_id)?;
        if self.attempt_epoch != self.expected_prior_attempt_epoch.saturating_add(1)
            || self.attempt_epoch == 0
            || self.started_at_seq == 0
        {
            return Err(AgentHydrationContractError::SequenceRegression);
        }
        Ok(())
    }
}

/// Fenced command that records one hydration attempt failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailAgentHydrationCommand {
    /// Existing hydration attempt to fail.
    pub hydration_id: String,
    /// Exact domain-owned attempt epoch.
    pub attempt_epoch: u64,
    /// Exact runtime lease that owns the attempt.
    pub lease_id: String,
    /// Current hydration update sequence expected by the writer.
    pub expected_updated_at_seq: u64,
    /// New update sequence assigned to the failure.
    pub failed_at_seq: u64,
    /// Bounded diagnostic explaining the failure.
    pub error: String,
}

impl FailAgentHydrationCommand {
    /// Validate identity, fence, sequence, and bounded diagnostic content.
    pub fn validate(&self) -> Result<(), AgentHydrationContractError> {
        require_non_empty("hydration id", &self.hydration_id)?;
        require_non_empty("hydration lease id", &self.lease_id)?;
        require_non_empty("hydration failure", &self.error)?;
        if self.attempt_epoch == 0
            || self.failed_at_seq <= self.expected_updated_at_seq
            || self.error.len() > MAX_HYDRATION_ERROR_BYTES
        {
            return Err(AgentHydrationContractError::SequenceRegression);
        }
        Ok(())
    }
}

/// Fenced command that transitions one exact registered agent to operational.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkAgentOperationalCommand {
    /// Hydration attempt that produced the readiness proof.
    pub hydration_id: String,
    /// Exact domain-owned hydration attempt epoch.
    pub attempt_epoch: u64,
    /// Exact runtime lease that owns the attempt.
    pub lease_id: String,
    /// Hydration update sequence read before readiness evaluation.
    pub expected_hydration_updated_at_seq: u64,
    /// Complete readiness proof for the target agent.
    pub readiness: AgentReadinessProof,
    /// New monotonic agent and hydration sequence written by the transition.
    pub updated_at_seq: u64,
}

impl MarkAgentOperationalCommand {
    /// Validate proof identity and hydration and agent sequence fences.
    pub fn validate(&self) -> Result<(), AgentHydrationContractError> {
        require_non_empty("hydration id", &self.hydration_id)?;
        require_non_empty("hydration lease id", &self.lease_id)?;
        self.readiness.validate()?;
        if self.attempt_epoch == 0
            || self.updated_at_seq <= self.readiness.expected_agent_updated_at_seq
            || self.updated_at_seq <= self.expected_hydration_updated_at_seq
            || self.updated_at_seq < self.readiness.signal.attestation.attested_at_seq
        {
            return Err(AgentHydrationContractError::SequenceRegression);
        }
        Ok(())
    }
}

/// Invalid process-hydration contract.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AgentHydrationContractError {
    /// One required field is absent or malformed.
    #[error("invalid agent hydration contract: {0}")]
    Invalid(String),
    /// A deterministic signal or proof identity diverged from its fields.
    #[error("agent hydration identity mismatch for {0}")]
    IdentityMismatch(String),
    /// The ground goal belongs to another agent.
    #[error("readiness goal agent does not match the readiness signal agent")]
    AgentMismatch,
    /// Readiness goal still contains variables.
    #[error("readiness goal target must be ground")]
    GoalNotGround,
    /// Planner request and durable frame do not describe one completion.
    #[error("readiness planner request and frame products disagree")]
    PlannerProductMismatch,
    /// Planner request scope does not match the attested belief stream.
    #[error("readiness planner scope does not match the belief attestation")]
    PlannerScopeMismatch,
    /// A durable transition would move an epoch or sequence backwards.
    #[error("agent hydration epoch or sequence would regress")]
    SequenceRegression,
    /// Hydration status does not match its readiness or failure products.
    #[error("agent hydration status products are inconsistent")]
    InvalidStatusProducts,
    /// A deterministic identity projection could not be encoded.
    #[error("agent hydration identity encoding failed: {0}")]
    Encoding(String),
}

fn require_non_empty(field: &str, value: &str) -> Result<(), AgentHydrationContractError> {
    if value.trim().is_empty() {
        return Err(AgentHydrationContractError::Invalid(format!(
            "{field} must be non-empty"
        )));
    }
    Ok(())
}

fn semantic_hash(
    domain: &[u8],
    value: &impl Serialize,
) -> Result<String, AgentHydrationContractError> {
    let encoded = serde_json::to_vec(value)
        .map_err(|error| AgentHydrationContractError::Encoding(error.to_string()))?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(&encoded);
    Ok(hasher.finalize().to_hex().to_string())
}
