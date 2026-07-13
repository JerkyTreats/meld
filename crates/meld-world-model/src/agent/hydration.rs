//! Process-hydration and operational-readiness contracts.

mod runtime;

pub use runtime::{
    AgentHydrationActor, AgentHydrationIssue, AgentHydrationTickReport, AgentHydrationTickRequest,
    AGENT_HYDRATION_ACTOR_ID, MAX_AGENT_HYDRATION_ITEMS,
};

#[cfg(feature = "test-support")]
#[doc(hidden)]
pub fn fuzz_hydration_terminal_fence(data: &[u8]) {
    super::store::fuzz_terminal_hydration_fence(data);
}

use meld_lang::Goal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::activation::{AgentBootstrapReceipt, AgentCurationRuleRecord, DirectiveRecord};
use crate::agent::contracts::{AgentRecord, AgentSubscriptionRecord};
use crate::agent::AgentId;
use crate::belief::BeliefReadinessAttestation;
use crate::error::StorageError;
use crate::planner::{
    PlannerProjectionFrame, PlannerProjectionRequestRecord, PlannerProjectionRequestStatus,
    PlannerSourceRef,
};

const READINESS_SIGNAL_HASH_DOMAIN: &[u8] = b"meld.agent-readiness-signal.v2";
const READINESS_PROOF_HASH_DOMAIN: &[u8] = b"meld.agent-readiness-proof.v2";
const READINESS_SIGNAL_V2_HASH_DOMAIN: &[u8] = b"meld.agent-readiness-signal.v3";
const READINESS_PROOF_V2_HASH_DOMAIN: &[u8] = b"meld.agent-readiness-proof.v3";
const READINESS_SCHEMA_V1: u16 = 1;
const READINESS_SCHEMA_V2: u16 = 2;
const READINESS_IDENTITY_V1: u16 = 1;
const READINESS_IDENTITY_V2: u16 = 2;
const MAX_HYDRATION_ERROR_BYTES: usize = 1024;
const OWNER_FENCE_HASH_DOMAIN: &[u8] = b"meld.agent-hydration-owner-fence.v1";

/// Canonical owner content bound into prepared hydration products.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentHydrationOwnerSnapshot {
    /// Exact durable agent record.
    pub agent: AgentRecord,
    /// Exact completed bootstrap receipt.
    pub receipt: AgentBootstrapReceipt,
    /// Exact durable directive.
    pub directive: DirectiveRecord,
    /// Exact durable curation rule.
    pub rule: AgentCurationRuleRecord,
    /// Exact durable belief subscription.
    pub subscription: AgentSubscriptionRecord,
    /// Immutable belief configuration snapshot key.
    pub belief_config_hash: String,
    /// Canonical serialized belief configuration content.
    pub belief_config_json: String,
}

impl AgentHydrationOwnerSnapshot {
    /// Validate owner relationships and return the canonical fence digest.
    pub fn fence_hash(&self) -> Result<String, StorageError> {
        self.agent.validate()?;
        self.rule.config.validate()?;
        self.subscription.validate()?;
        if self.receipt.agent_id != self.agent.agent_id
            || self.receipt.directive_id != self.agent.directive_id
            || self.receipt.directive_id != self.directive.directive_id
            || self.receipt.rule_id != self.rule.rule_id
            || self.receipt.subscription_id != self.subscription.subscription_id
            || self.rule.agent_id != self.agent.agent_id
            || self.subscription.agent_id != self.agent.agent_id
            || self.receipt.belief.config_snapshot_hash != self.belief_config_hash
            || self.belief_config_json.trim().is_empty()
        {
            return Err(StorageError::InvalidPath(
                "hydration owner snapshot has divergent identities".to_string(),
            ));
        }
        let encoded = serde_json::to_vec(self).map_err(|error| {
            StorageError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })?;
        let mut hasher = blake3::Hasher::new();
        hasher.update(OWNER_FENCE_HASH_DOMAIN);
        hasher.update(&encoded);
        Ok(hasher.finalize().to_hex().to_string())
    }
}

/// One belief-owned attestation consumed as an operational readiness signal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentReadinessSignal {
    /// Durable wire schema version.
    // TODO compat-shim: remove the v1 serde default after every supported store
    // is schema v2 and accepted W3A signal fixture plus hash parity tests stay green.
    #[serde(default = "legacy_readiness_version")]
    pub schema_version: u16,
    /// Hash preimage version that owns the stable signal id.
    #[serde(default = "legacy_readiness_version")]
    pub identity_version: u16,
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
            schema_version: READINESS_SCHEMA_V2,
            identity_version: READINESS_IDENTITY_V2,
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
        // TODO compat-shim: remove v1 signal acceptance after accepted W3A
        // fixtures and signal migration plus hash parity tests are retired.
        match (self.schema_version, self.identity_version) {
            (READINESS_SCHEMA_V1, READINESS_IDENTITY_V1)
                if self.attestation.requires_legacy_upgrade() => {}
            (READINESS_SCHEMA_V2, READINESS_IDENTITY_V1)
                if self.attestation.has_legacy_identity()
                    && !self.attestation.requires_legacy_upgrade() => {}
            (READINESS_SCHEMA_V2, READINESS_IDENTITY_V2)
                if !self.attestation.requires_legacy_upgrade() => {}
            _ => {
                return Err(AgentHydrationContractError::Invalid(
                    "unsupported readiness signal schema or identity version".to_string(),
                ));
            }
        }
        if self.signal_id != self.derive_id()? {
            return Err(AgentHydrationContractError::IdentityMismatch(
                "readiness signal id".to_string(),
            ));
        }
        Ok(())
    }

    fn derive_id(&self) -> Result<String, AgentHydrationContractError> {
        // TODO compat-shim: remove the v1 signal preimage after accepted W3A
        // stable-id fixtures no longer require exact identity preservation.
        if self.identity_version == READINESS_IDENTITY_V1 {
            return semantic_hash(
                READINESS_SIGNAL_HASH_DOMAIN,
                &self.attestation.legacy_identity_wire(),
            );
        }
        semantic_hash(READINESS_SIGNAL_V2_HASH_DOMAIN, &self.attestation)
    }

    pub(crate) fn upgrade_legacy(
        mut self,
        attestation: BeliefReadinessAttestation,
    ) -> Result<Self, AgentHydrationContractError> {
        // TODO compat-shim: remove this signal rewrite after all supported W3A
        // records are migrated and signal identity plus reopen tests stay green.
        self.validate()?;
        if self.schema_version != READINESS_SCHEMA_V1
            || self.identity_version != READINESS_IDENTITY_V1
            || !attestation.has_legacy_identity()
            || attestation.requires_legacy_upgrade()
            || attestation.attestation_id != self.attestation.attestation_id
        {
            return Err(AgentHydrationContractError::Invalid(
                "readiness signal legacy upgrade has divergent identity".to_string(),
            ));
        }
        self.schema_version = READINESS_SCHEMA_V2;
        self.attestation = attestation;
        self.validate()?;
        Ok(self)
    }
}

/// Complete proof required before one registered agent becomes operational.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentReadinessProof {
    /// Durable wire schema version.
    // TODO compat-shim: remove the v1 serde default after every supported store
    // is schema v2 and accepted W3A proof fixture plus hash parity tests stay green.
    #[serde(default = "legacy_readiness_version")]
    pub schema_version: u16,
    /// Hash preimage version that owns the stable proof id.
    #[serde(default = "legacy_readiness_version")]
    pub identity_version: u16,
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
            schema_version: READINESS_SCHEMA_V2,
            identity_version: READINESS_IDENTITY_V2,
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
        // TODO compat-shim: remove v1 proof acceptance after accepted W3A
        // embedded-proof reopen and hash parity tests are retired.
        match (self.schema_version, self.identity_version) {
            (READINESS_SCHEMA_V1, READINESS_IDENTITY_V1)
                if self.signal.schema_version == READINESS_SCHEMA_V1
                    && self.signal.identity_version == READINESS_IDENTITY_V1 => {}
            (READINESS_SCHEMA_V2, READINESS_IDENTITY_V1)
                if self.signal.schema_version == READINESS_SCHEMA_V2
                    && self.signal.identity_version == READINESS_IDENTITY_V1 => {}
            (READINESS_SCHEMA_V2, READINESS_IDENTITY_V2)
                if self.signal.schema_version == READINESS_SCHEMA_V2
                    && self.signal.identity_version == READINESS_IDENTITY_V2 => {}
            _ => {
                return Err(AgentHydrationContractError::Invalid(
                    "unsupported readiness proof schema or identity version".to_string(),
                ));
            }
        }
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
        if self.identity_version == READINESS_IDENTITY_V2 {
            let snapshot = request
                .attested_belief
                .as_ref()
                .ok_or(AgentHydrationContractError::PlannerSnapshotMismatch)?;
            if snapshot.attestation_id != attestation.attestation_id
                || snapshot.revision_id != attestation.belief_revision_id
                || snapshot.revision_hash != attestation.belief_revision_hash
                || snapshot.view_id != attestation.belief_view_id
                || snapshot.view_hash != attestation.belief_view_hash
                || snapshot.source_cursor_start != attestation.source_cursor_start
                || snapshot.source_cursor_end != attestation.source_cursor_end
                || self.planner_request.created_at_seq
                    != attestation.attested_at_seq.checked_add(2).unwrap_or(0)
                || self.planner_request.updated_at_seq
                    != self
                        .planner_request
                        .created_at_seq
                        .checked_add(3)
                        .unwrap_or(0)
                || self.planner_frame.completed_at_seq != self.planner_request.updated_at_seq
                || self.planner_frame.output.hydration_refs.revision_ids
                    != vec![attestation.belief_revision_id.clone()]
                || !self.planner_frame.output.source_refs.contains(
                    &PlannerSourceRef::BeliefRevision {
                        revision_id: attestation.belief_revision_id.clone(),
                    },
                )
            {
                return Err(AgentHydrationContractError::PlannerSnapshotMismatch);
            }
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
        // TODO compat-shim: remove the v1 proof preimage after accepted W3A
        // embedded-proof stable-id fixtures no longer require it.
        if self.identity_version == READINESS_IDENTITY_V1 {
            #[derive(Serialize)]
            struct LegacySignal<'a> {
                signal_id: &'a str,
                attestation: crate::belief::readiness::LegacyBeliefReadinessAttestationV1<'a>,
            }
            #[derive(Serialize)]
            struct LegacyIdentity<'a> {
                expected_agent_updated_at_seq: u64,
                signal: LegacySignal<'a>,
                planner_request: &'a PlannerProjectionRequestRecord,
                planner_frame: &'a PlannerProjectionFrame,
                ground_goal: &'a Goal,
            }
            return semantic_hash(
                READINESS_PROOF_HASH_DOMAIN,
                &LegacyIdentity {
                    expected_agent_updated_at_seq: self.expected_agent_updated_at_seq,
                    signal: LegacySignal {
                        signal_id: &self.signal.signal_id,
                        attestation: self.signal.attestation.legacy_identity_wire(),
                    },
                    planner_request: &self.planner_request,
                    planner_frame: &self.planner_frame,
                    ground_goal: &self.ground_goal,
                },
            );
        }
        #[derive(Serialize)]
        struct Identity<'a> {
            schema_version: u16,
            identity_version: u16,
            expected_agent_updated_at_seq: u64,
            signal: &'a AgentReadinessSignal,
            planner_request: &'a PlannerProjectionRequestRecord,
            planner_frame: &'a PlannerProjectionFrame,
            ground_goal: &'a Goal,
        }
        semantic_hash(
            READINESS_PROOF_V2_HASH_DOMAIN,
            &Identity {
                schema_version: self.schema_version,
                identity_version: self.identity_version,
                expected_agent_updated_at_seq: self.expected_agent_updated_at_seq,
                signal: &self.signal,
                planner_request: &self.planner_request,
                planner_frame: &self.planner_frame,
                ground_goal: &self.ground_goal,
            },
        )
    }

    pub(crate) fn requires_legacy_upgrade(&self) -> bool {
        self.schema_version == READINESS_SCHEMA_V1
    }

    pub(crate) fn upgrade_legacy(
        mut self,
        attestation: BeliefReadinessAttestation,
    ) -> Result<Self, AgentHydrationContractError> {
        // TODO compat-shim: remove this proof rewrite after all supported W3A
        // records are migrated and proof identity plus reopen tests stay green.
        self.validate()?;
        if !self.requires_legacy_upgrade() || self.identity_version != READINESS_IDENTITY_V1 {
            return Err(AgentHydrationContractError::Invalid(
                "readiness proof is not an accepted W3A legacy record".to_string(),
            ));
        }
        self.signal = self.signal.upgrade_legacy(attestation)?;
        self.schema_version = READINESS_SCHEMA_V2;
        self.validate()?;
        Ok(self)
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
    /// Planner products do not preserve the exact attested belief snapshot.
    #[error("readiness planner products do not preserve the attested belief snapshot")]
    PlannerSnapshotMismatch,
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

fn legacy_readiness_version() -> u16 {
    READINESS_SCHEMA_V1
}
