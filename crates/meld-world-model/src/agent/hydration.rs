//! Process-hydration and operational-readiness contracts.

use meld_lang::Goal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::agent::{AgentId, AgentSubscriptionId};
use crate::belief::BeliefKey;

const READINESS_SIGNAL_HASH_DOMAIN: &[u8] = b"meld.agent-readiness-signal.v1";
const READINESS_PROOF_HASH_DOMAIN: &[u8] = b"meld.agent-readiness-proof.v1";

/// One subscription delivery processed as an operational readiness signal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentReadinessSignal {
    /// Deterministic signal identity derived from every other field.
    pub signal_id: String,
    /// Agent whose process hydration consumed the signal.
    pub agent_id: AgentId,
    /// Active subscription that delivered the belief revision.
    pub subscription_id: AgentSubscriptionId,
    /// Exact belief stream that was readable for the delivery.
    pub belief_key: BeliefKey,
    /// Readable belief revision processed by hydration.
    pub belief_revision_id: String,
    /// Durable sequence observed for the processed signal.
    pub processed_at_seq: u64,
}

impl AgentReadinessSignal {
    /// Construct and identify one processed readiness signal.
    pub fn identified(
        agent_id: impl Into<String>,
        subscription_id: impl Into<String>,
        belief_key: BeliefKey,
        belief_revision_id: impl Into<String>,
        processed_at_seq: u64,
    ) -> Result<Self, AgentHydrationContractError> {
        let mut signal = Self {
            signal_id: String::new(),
            agent_id: agent_id.into(),
            subscription_id: subscription_id.into(),
            belief_key,
            belief_revision_id: belief_revision_id.into(),
            processed_at_seq,
        };
        signal.signal_id = signal.derive_id()?;
        signal.validate()?;
        Ok(signal)
    }

    /// Validate the signal identity and required durable references.
    pub fn validate(&self) -> Result<(), AgentHydrationContractError> {
        require_non_empty("signal agent id", &self.agent_id)?;
        require_non_empty("signal subscription id", &self.subscription_id)?;
        require_non_empty("signal belief revision id", &self.belief_revision_id)?;
        self.belief_key
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
        #[derive(Serialize)]
        struct Identity<'a> {
            agent_id: &'a str,
            subscription_id: &'a str,
            belief_key: &'a BeliefKey,
            belief_revision_id: &'a str,
            processed_at_seq: u64,
        }
        semantic_hash(
            READINESS_SIGNAL_HASH_DOMAIN,
            &Identity {
                agent_id: &self.agent_id,
                subscription_id: &self.subscription_id,
                belief_key: &self.belief_key,
                belief_revision_id: &self.belief_revision_id,
                processed_at_seq: self.processed_at_seq,
            },
        )
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
    /// Processed subscription delivery proving a readable belief view.
    pub signal: AgentReadinessSignal,
    /// Durable world-model planner frame produced for this perspective.
    pub planner_frame_id: String,
    /// Planner projection version carried by the durable frame.
    pub planner_projection_version: String,
    /// Ground goal constructed without submitting it to execution.
    pub ground_goal: Goal,
}

impl AgentReadinessProof {
    /// Identify one complete readiness proof after all gates succeed.
    pub fn identified(
        expected_agent_updated_at_seq: u64,
        signal: AgentReadinessSignal,
        planner_frame_id: impl Into<String>,
        planner_projection_version: impl Into<String>,
        ground_goal: Goal,
    ) -> Result<Self, AgentHydrationContractError> {
        let mut proof = Self {
            proof_id: String::new(),
            expected_agent_updated_at_seq,
            signal,
            planner_frame_id: planner_frame_id.into(),
            planner_projection_version: planner_projection_version.into(),
            ground_goal,
        };
        proof.proof_id = proof.derive_id()?;
        proof.validate()?;
        Ok(proof)
    }

    /// Validate every operational-readiness gate and deterministic identity.
    pub fn validate(&self) -> Result<(), AgentHydrationContractError> {
        self.signal.validate()?;
        require_non_empty("planner frame id", &self.planner_frame_id)?;
        require_non_empty(
            "planner projection version",
            &self.planner_projection_version,
        )?;
        require_non_empty("ground goal id", &self.ground_goal.goal_id)?;
        if self.ground_goal.agent_id != self.signal.agent_id {
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
            planner_frame_id: &'a str,
            planner_projection_version: &'a str,
            ground_goal: &'a Goal,
        }
        semantic_hash(
            READINESS_PROOF_HASH_DOMAIN,
            &Identity {
                expected_agent_updated_at_seq: self.expected_agent_updated_at_seq,
                signal: &self.signal,
                planner_frame_id: &self.planner_frame_id,
                planner_projection_version: &self.planner_projection_version,
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
    /// Current hydration state.
    pub status: AgentProcessHydrationStatus,
    /// Readiness proof accepted by the completed attempt.
    pub readiness_proof_id: Option<String>,
    /// Runtime lease hosting this hydration attempt.
    pub lease_id: Option<String>,
    /// Bounded failure detail for a failed attempt.
    pub last_error: Option<String>,
    /// Sequence at which this attempt began.
    pub started_at_seq: u64,
    /// Last durable update sequence for this attempt.
    pub updated_at_seq: u64,
}

impl AgentProcessHydrationRecord {
    /// Validate status products and monotonic sequence fields.
    pub fn validate(&self) -> Result<(), AgentHydrationContractError> {
        require_non_empty("hydration id", &self.hydration_id)?;
        require_non_empty("hydration agent id", &self.agent_id)?;
        if self.updated_at_seq < self.started_at_seq {
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
                {
                    return Err(AgentHydrationContractError::InvalidStatusProducts);
                }
            }
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
    /// Complete readiness proof for the target agent.
    pub readiness: AgentReadinessProof,
    /// New monotonic agent sequence written by the transition.
    pub updated_at_seq: u64,
}

impl MarkAgentOperationalCommand {
    /// Validate proof identity and the optimistic agent sequence fence.
    pub fn validate(&self) -> Result<(), AgentHydrationContractError> {
        require_non_empty("hydration id", &self.hydration_id)?;
        self.readiness.validate()?;
        if self.updated_at_seq <= self.readiness.expected_agent_updated_at_seq
            || self.updated_at_seq < self.readiness.signal.processed_at_seq
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
    /// A durable transition would move a sequence backwards or fail to advance.
    #[error("agent hydration sequence would regress or fail to advance")]
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

#[cfg(test)]
mod tests {
    use meld_lang::{Condition, GoalLifecycle, GoalPriority, GoalSource, Proposition, Term};

    use super::*;
    use crate::belief::BranchScope;
    use crate::events::DomainObjectRef;
    use crate::world_state::graph::PerspectiveKey;

    fn belief_key() -> BeliefKey {
        BeliefKey {
            subject: DomainObjectRef::new("workspace_fs", "node", "readme").unwrap(),
            dimension_id: "docs_freshness".to_string(),
            predicate_id: "fresh".to_string(),
            perspective: PerspectiveKey::new("agent", "docs").unwrap(),
            branch_scope: BranchScope::main(),
            evidence_policy_id: "docs".to_string(),
        }
    }

    fn goal(agent_id: &str) -> Goal {
        Goal {
            goal_id: "goal-readiness".to_string(),
            agent_id: agent_id.to_string(),
            target: Proposition::Holds {
                subject: Term::Object(
                    DomainObjectRef::new("workspace_fs", "node", "readme").unwrap(),
                ),
                dimension: Term::Dimension("docs_freshness".to_string()),
                condition: Condition::Present,
            },
            priority: GoalPriority {
                urgency: 1,
                cost_ceiling: None,
            },
            source: GoalSource::Maintenance {
                invariant_description: "docs remain current".to_string(),
            },
            lifecycle: GoalLifecycle::Proposed,
        }
    }

    #[test]
    fn readiness_proof_binds_signal_frame_ground_goal_and_agent_fence() {
        let signal = AgentReadinessSignal::identified(
            "agent-docs",
            "subscription-docs",
            belief_key(),
            "revision-1",
            8,
        )
        .unwrap();
        let proof = AgentReadinessProof::identified(
            7,
            signal,
            "planner-frame-1",
            "world_model.planner.v1",
            goal("agent-docs"),
        )
        .unwrap();
        let command = MarkAgentOperationalCommand {
            hydration_id: "hydration-docs-1".to_string(),
            readiness: proof.clone(),
            updated_at_seq: 9,
        };

        assert!(proof.validate().is_ok());
        assert!(command.validate().is_ok());
        let encoded = serde_json::to_vec(&command).unwrap();
        let decoded: MarkAgentOperationalCommand = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, command);
    }

    #[test]
    fn readiness_proof_rejects_cross_agent_and_nonground_goal() {
        let signal = AgentReadinessSignal::identified(
            "agent-docs",
            "subscription-docs",
            belief_key(),
            "revision-1",
            8,
        )
        .unwrap();
        assert!(matches!(
            AgentReadinessProof::identified(
                7,
                signal.clone(),
                "planner-frame-1",
                "world_model.planner.v1",
                goal("agent-other"),
            ),
            Err(AgentHydrationContractError::AgentMismatch)
        ));

        let mut nonground = goal("agent-docs");
        nonground.target = Proposition::Holds {
            subject: Term::Variable("subject".to_string()),
            dimension: Term::Dimension("docs_freshness".to_string()),
            condition: Condition::Present,
        };
        assert!(matches!(
            AgentReadinessProof::identified(
                7,
                signal,
                "planner-frame-1",
                "world_model.planner.v1",
                nonground,
            ),
            Err(AgentHydrationContractError::GoalNotGround)
        ));
    }

    #[test]
    fn hydration_status_requires_exact_terminal_products() {
        let ready = AgentProcessHydrationRecord {
            hydration_id: "hydration-docs-1".to_string(),
            agent_id: "agent-docs".to_string(),
            status: AgentProcessHydrationStatus::Ready,
            readiness_proof_id: Some("proof-1".to_string()),
            lease_id: Some("lease-1".to_string()),
            last_error: None,
            started_at_seq: 1,
            updated_at_seq: 9,
        };
        assert!(ready.validate().is_ok());

        let mut invalid = ready;
        invalid.last_error = Some("failed".to_string());
        assert!(matches!(
            invalid.validate(),
            Err(AgentHydrationContractError::InvalidStatusProducts)
        ));
    }
}
