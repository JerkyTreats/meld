//! Bounded recovery-first process hydration for registered seed agents.

use std::sync::Arc;

use meld_lang::{Goal, GoalLifecycle, GoalPriority, GoalSource};
use serde::Serialize;

use super::{
    AgentProcessHydrationRecord, AgentProcessHydrationStatus, AgentReadinessProof,
    AgentReadinessSignal, FailAgentHydrationCommand, MarkAgentOperationalCommand,
    StartAgentHydrationCommand,
};
use crate::activation::{
    validate_world_model_activation, AgentBootstrapReceipt, AgentCurationRuleRecord,
    SeedAgentActivation, WorldModelActivationInput,
};
use crate::agent::contracts::{
    deterministic_id, threshold_target, AgentHydrationCheckpoint, AgentHydrationCheckpointStage,
    AgentRecord, AgentStatus, AgentSubscriptionRecord, AgentSubscriptionStatus,
};
use crate::agent::store::{hydration_owner_fence_hash, AgentHydrationOwnerFence};
use crate::agent::{AgentRegistration, AgentStore};
use crate::belief::{
    BeliefConfigLoader, BeliefConfigSnapshotFence, BeliefQuery, BeliefReadinessAttestation,
    BeliefReadinessAttestationRequest, BeliefRevision, BeliefStore,
};
use crate::error::StorageError;
use crate::planner::{
    PlannerAttestedBeliefSnapshot, PlannerProjectionFrame, PlannerProjectionRequest,
    PlannerProjectionRequestRecord, PlannerProjectionRequestStatus, PlannerProjectionStore,
    PreparedPlannerProjectionRequest,
};

/// Canonical recurring process hydration runtime identity.
pub const AGENT_HYDRATION_ACTOR_ID: &str = "world_model.agent_hydration";

/// Hard maximum registered agents one hydration tick may select.
pub const MAX_AGENT_HYDRATION_ITEMS: usize = 1024;

const MAX_ISSUES: usize = 16;
const MAX_ISSUE_BYTES: usize = 1024;
const READINESS_PROJECTION_SOURCE_DOMAIN: &[u8] = b"meld.agent-readiness-projection.v1";

/// Input for one bounded hydration tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentHydrationTickRequest {
    /// Supervisor lease supplied only as a stale-writer fence.
    pub lease_id: String,
    /// Maximum registered agents selected in durable order.
    pub max_items: usize,
}

/// One bounded diagnostic from process hydration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentHydrationIssue {
    /// Agent identity when the issue belongs to one selection.
    pub agent_id: Option<String>,
    /// Stable machine-readable category.
    pub code: String,
    /// Bounded human-readable detail.
    pub message: String,
}

/// Native report for one bounded hydration tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentHydrationTickReport {
    /// Canonical actor identity.
    pub actor_id: String,
    /// Highest durable input sequence observed.
    pub input_sequence: u64,
    /// Highest durable output sequence committed or replayed.
    pub output_sequence: u64,
    /// Registered agents selected in deterministic order.
    pub selected_count: usize,
    /// New hydration epochs started with a selected checkpoint.
    pub started_count: usize,
    /// Belief readiness attestations persisted and checkpointed.
    pub attested_count: usize,
    /// Durable planner requests submitted and checkpointed.
    pub projection_requested_count: usize,
    /// Hydrations waiting on a pending planner request.
    pub projection_pending_count: usize,
    /// Agents atomically marked operational with a readiness proof.
    pub operational_count: usize,
    /// Hydration attempts durably failed by this tick.
    pub failed_count: usize,
    /// Retryable issues that retain durable recovery state.
    pub retryable_errors: Vec<AgentHydrationIssue>,
    /// Fatal malformed or divergent durable products.
    pub fatal_errors: Vec<AgentHydrationIssue>,
    /// True when another registered agent remains beyond the budget.
    pub budget_exhausted: bool,
}

impl AgentHydrationTickReport {
    fn empty() -> Self {
        Self {
            actor_id: AGENT_HYDRATION_ACTOR_ID.to_string(),
            input_sequence: 0,
            output_sequence: 0,
            selected_count: 0,
            started_count: 0,
            attested_count: 0,
            projection_requested_count: 0,
            projection_pending_count: 0,
            operational_count: 0,
            failed_count: 0,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
        }
    }

    /// Return whether this tick advanced durable hydration state.
    pub fn work_performed(&self) -> bool {
        self.started_count
            + self.attested_count
            + self.projection_requested_count
            + self.operational_count
            + self.failed_count
            > 0
    }
}

/// World-model-owned actor that proves registered seed readiness.
pub struct AgentHydrationActor {
    agent_store: Arc<AgentStore>,
    belief_store: Arc<BeliefStore>,
    planner_store: Arc<PlannerProjectionStore>,
}

impl AgentHydrationActor {
    /// Bind the agent authority and public belief and planner authorities.
    pub fn new(
        agent_store: Arc<AgentStore>,
        belief_store: Arc<BeliefStore>,
        planner_store: Arc<PlannerProjectionStore>,
    ) -> Self {
        Self {
            agent_store,
            belief_store,
            planner_store,
        }
    }

    /// Advance at most one durable orchestration stage per selected agent.
    pub fn tick(&self, request: AgentHydrationTickRequest) -> AgentHydrationTickReport {
        let mut report = AgentHydrationTickReport::empty();
        if request.lease_id.trim().is_empty()
            || request.max_items == 0
            || request.max_items > MAX_AGENT_HYDRATION_ITEMS
        {
            push_fatal(
                &mut report,
                None,
                "invalid_request",
                "hydration lease must be non-empty and budget must be within the actor bound",
            );
            return report;
        }

        let selection = match self
            .agent_store
            .registered_agents_bounded(request.max_items)
        {
            Ok(selection) => selection,
            Err(error) => {
                push_storage(&mut report, None, error);
                return report;
            }
        };
        report.budget_exhausted = selection.budget_exhausted;
        report.selected_count = selection.agents.len();

        for agent in &selection.agents {
            report.input_sequence = report.input_sequence.max(agent.updated_at_seq);
            if let Err(error) = self.advance_agent(agent, &request.lease_id, &mut report) {
                push_actor_error(&mut report, Some(agent.agent_id.clone()), error);
            }
        }
        if let Some(advanced) = selection.advanced_continuation.as_ref() {
            if let Err(error) = retry_indeterminate(|| {
                self.agent_store.advance_hydration_continuation(
                    selection.expected_continuation.as_ref(),
                    advanced,
                )
            }) {
                push_storage(&mut report, None, error);
            }
        }
        report
    }

    fn advance_agent(
        &self,
        selected_agent: &AgentRecord,
        lease_id: &str,
        report: &mut AgentHydrationTickReport,
    ) -> Result<(), HydrationActorError> {
        let context = self.load_static_context(selected_agent)?;
        report.input_sequence = report
            .input_sequence
            .max(context.receipt.completed_at_seq)
            .max(context.subscription.updated_at_seq);

        let current = self
            .agent_store
            .current_process_hydration(&selected_agent.agent_id)
            .map_err(HydrationActorError::storage)?;
        let same_lease = self
            .agent_store
            .process_hydration_for_lease(&selected_agent.agent_id, lease_id)
            .map_err(HydrationActorError::storage)?;
        if let Some(current) = current.as_ref() {
            report.input_sequence = report.input_sequence.max(current.updated_at_seq);
        }
        if let Some(owned) = same_lease.as_ref() {
            report.input_sequence = report.input_sequence.max(owned.updated_at_seq);
        }

        let hydration = if let Some(owned) = same_lease.as_ref() {
            if current.as_ref().map(|record| record.hydration_id.as_str())
                != Some(owned.hydration_id.as_str())
            {
                return Err(HydrationActorError::fatal(
                    "stale_lease",
                    "hydration lease was superseded by a newer domain epoch",
                ));
            }
            match owned.status {
                AgentProcessHydrationStatus::Started => owned.clone(),
                AgentProcessHydrationStatus::Ready => {
                    return Err(HydrationActorError::fatal(
                        "registered_ready_agent",
                        "registered agent has a completed hydration without operational status",
                    ));
                }
                AgentProcessHydrationStatus::Failed => {
                    return Err(HydrationActorError::fatal(
                        "failed_lease",
                        owned
                            .last_error
                            .clone()
                            .unwrap_or_else(|| "hydration lease previously failed".to_string()),
                    ));
                }
            }
        } else {
            return self.start_epoch(selected_agent, &context, current.as_ref(), lease_id, report);
        };
        report.input_sequence = report.input_sequence.max(hydration.updated_at_seq);

        let checkpoint = self
            .agent_store
            .hydration_checkpoint(&hydration.hydration_id)
            .map_err(HydrationActorError::storage)?;
        let Some(checkpoint) = checkpoint else {
            return self.create_selected_checkpoint(selected_agent, &context, &hydration, report);
        };
        self.validate_checkpoint(&hydration, &context, &checkpoint)?;
        report.input_sequence = report.input_sequence.max(checkpoint.updated_at_seq);

        match checkpoint.stage {
            AgentHydrationCheckpointStage::Selected => {
                self.attest_selection(selected_agent, &context, &hydration, checkpoint, report)
            }
            AgentHydrationCheckpointStage::Attested => {
                self.request_projection(selected_agent, &context, &hydration, checkpoint, report)
            }
            AgentHydrationCheckpointStage::ProjectionRequested => {
                self.finish_or_wait(selected_agent, &context, &hydration, checkpoint, report)
            }
        }
    }

    fn start_epoch(
        &self,
        agent: &AgentRecord,
        context: &SeedContext,
        prior: Option<&AgentProcessHydrationRecord>,
        lease_id: &str,
        report: &mut AgentHydrationTickReport,
    ) -> Result<(), HydrationActorError> {
        let revision = self.select_current_revision(context)?;
        report.input_sequence = report.input_sequence.max(revision.source_cursor_end);
        let prior_epoch = prior.map_or(0, |record| record.attempt_epoch);
        let attempt_epoch = exact_successor(prior_epoch, "hydration epoch")?;
        if prior.is_some_and(|record| record.status == AgentProcessHydrationStatus::Ready) {
            return Err(HydrationActorError::fatal(
                "registered_ready_agent",
                "registered agent has a completed hydration without operational status",
            ));
        }
        let prior_causal_floor = prior
            .map(|record| self.hydration_causal_floor(agent, context, record))
            .transpose()?
            .unwrap_or(0);
        let started_at_seq = exact_successor(
            [
                agent.updated_at_seq,
                context.subscription.updated_at_seq,
                context.receipt.completed_at_seq,
                revision.source_cursor_end,
                prior.map_or(0, |record| record.updated_at_seq),
                prior_causal_floor,
            ]
            .into_iter()
            .max()
            .unwrap_or(0),
            "hydration start sequence",
        )?;
        let hydration_id = deterministic_id(
            "agent-hydration",
            &format!("{}::{attempt_epoch}", agent.agent_id),
        );
        let command = StartAgentHydrationCommand {
            hydration_id,
            agent_id: agent.agent_id.clone(),
            expected_prior_attempt_epoch: prior_epoch,
            attempt_epoch,
            lease_id: lease_id.to_string(),
            started_at_seq,
        };
        let registration = AgentRegistration::new(self.agent_store.as_ref());
        let owner = context.owner_fence(agent);
        let hydration =
            retry_indeterminate(|| registration.start_hydration_fenced(&command, &owner))
                .map_err(HydrationActorError::storage)?;
        let checkpoint = selected_checkpoint(&hydration, &context.subscription, &revision)?;
        retry_indeterminate(|| {
            self.agent_store
                .put_hydration_checkpoint_fenced(&checkpoint, &owner)
        })
        .map_err(HydrationActorError::storage)?;
        report.started_count += 1;
        report.output_sequence = report
            .output_sequence
            .max(hydration.updated_at_seq)
            .max(checkpoint.updated_at_seq);
        Ok(())
    }

    fn create_selected_checkpoint(
        &self,
        agent: &AgentRecord,
        context: &SeedContext,
        hydration: &AgentProcessHydrationRecord,
        report: &mut AgentHydrationTickReport,
    ) -> Result<(), HydrationActorError> {
        let durable_agent = self.require_exact_agent(agent)?;
        if durable_agent.status != AgentStatus::Registered {
            return Err(HydrationActorError::retryable(
                "agent_changed",
                "agent status changed before hydration selection",
            ));
        }
        let revision = self.select_current_revision(context)?;
        report.input_sequence = report.input_sequence.max(revision.source_cursor_end);
        let checkpoint = selected_checkpoint(hydration, &context.subscription, &revision)?;
        let owner = context.owner_fence(agent);
        retry_indeterminate(|| {
            self.agent_store
                .put_hydration_checkpoint_fenced(&checkpoint, &owner)
        })
        .map_err(HydrationActorError::storage)?;
        report.started_count += 1;
        report.output_sequence = report.output_sequence.max(checkpoint.updated_at_seq);
        Ok(())
    }

    fn attest_selection(
        &self,
        agent: &AgentRecord,
        context: &SeedContext,
        hydration: &AgentProcessHydrationRecord,
        checkpoint: AgentHydrationCheckpoint,
        report: &mut AgentHydrationTickReport,
    ) -> Result<(), HydrationActorError> {
        self.require_exact_agent(agent)?;
        self.require_exact_subscription(&context.subscription)?;
        self.require_exact_current_hydration(hydration)?;
        let revision = self.select_current_revision(context)?;
        report.input_sequence = report.input_sequence.max(revision.source_cursor_end);
        if revision.revision_id != checkpoint.selected_revision_id
            || revision.source_cursor_end != checkpoint.selected_revision_seq
        {
            return self.fail_changed_view(
                agent,
                context,
                hydration,
                &checkpoint,
                checkpoint.updated_at_seq.max(revision.source_cursor_end),
                report,
            );
        }
        let attested_at_seq = expected_attestation_sequence(hydration, &checkpoint)?;
        let request = BeliefReadinessAttestationRequest {
            agent_id: agent.agent_id.clone(),
            subscription_id: context.subscription.subscription_id.clone(),
            belief_key: context.subscription.belief_key.clone(),
            expected_revision_id: checkpoint.selected_revision_id.clone(),
            attested_at_seq,
        };
        let owner = context.owner_fence(agent);
        let owner_fence_hash =
            hydration_owner_fence_hash(&owner).map_err(HydrationActorError::storage)?;
        let attestation = match retry_indeterminate(|| {
            self.belief_store
                .prepare_readiness_attestation_fenced(&request, &owner_fence_hash)
        }) {
            Ok(attestation) => attestation,
            Err(StorageError::Backpressure(_)) => {
                return self.fail_changed_view(
                    agent,
                    context,
                    hydration,
                    &checkpoint,
                    checkpoint.updated_at_seq.max(revision.source_cursor_end),
                    report,
                );
            }
            Err(error) => return Err(HydrationActorError::storage(error)),
        };
        validate_attestation(agent, context, hydration, &checkpoint, &attestation)?;
        let mut advanced = checkpoint.clone();
        advanced.stage = AgentHydrationCheckpointStage::Attested;
        advanced.attestation_id = Some(attestation.attestation_id.clone());
        advanced.updated_at_seq = expected_attested_checkpoint_sequence(hydration, &checkpoint)?;
        retry_indeterminate(|| {
            self.agent_store
                .advance_hydration_checkpoint_cas_fenced(&checkpoint, &advanced, &owner)
        })
        .map_err(HydrationActorError::storage)?;
        let activation_fence = self
            .agent_store
            .hydration_activation_fence(&owner, hydration, &advanced)
            .map_err(HydrationActorError::storage)?;
        retry_indeterminate(|| {
            self.belief_store
                .activate_prepared_readiness_attestation(&attestation, &activation_fence)
        })
        .map_err(HydrationActorError::storage)?;
        report.attested_count += 1;
        report.output_sequence = report
            .output_sequence
            .max(attestation.attested_at_seq)
            .max(advanced.updated_at_seq);
        Ok(())
    }

    fn request_projection(
        &self,
        agent: &AgentRecord,
        context: &SeedContext,
        hydration: &AgentProcessHydrationRecord,
        checkpoint: AgentHydrationCheckpoint,
        report: &mut AgentHydrationTickReport,
    ) -> Result<(), HydrationActorError> {
        self.require_exact_agent(agent)?;
        self.require_exact_subscription(&context.subscription)?;
        self.require_exact_current_hydration(hydration)?;
        let attestation = self.load_attestation(agent, context, hydration, &checkpoint)?;
        let request = readiness_projection_request(agent, context, hydration, &attestation)?;
        let created_at_seq = expected_planner_request_sequence(hydration, &checkpoint)?;
        let owner = context.owner_fence(agent);
        let owner_fence_hash =
            hydration_owner_fence_hash(&owner).map_err(HydrationActorError::storage)?;
        let visible = self
            .planner_store
            .get_request(&request.request_id)
            .map_err(HydrationActorError::storage)?;
        let prepared = if let Some(record) = visible.as_ref() {
            require_exact_planner_request(record, &request, created_at_seq)?;
            if record.status != PlannerProjectionRequestStatus::Pending {
                return Err(HydrationActorError::fatal(
                    "divergent_planner_request",
                    "planner request became terminal before its hydration checkpoint",
                ));
            }
            None
        } else if let Some(prepared) = self
            .planner_store
            .get_prepared_request(&request.request_id)
            .map_err(HydrationActorError::storage)?
        {
            require_exact_prepared_planner_request(&prepared, &request, created_at_seq)?;
            Some(prepared)
        } else {
            let snapshot = self
                .belief_store
                .readiness_snapshot(&attestation.attestation_id)
                .map_err(HydrationActorError::storage)?
                .ok_or_else(|| {
                    HydrationActorError::fatal(
                        "missing_readiness_snapshot",
                        "planner request requires the exact attested belief snapshot",
                    )
                })?;
            let result = self.planner_store.put_prepared_fenced(
                request.clone(),
                &snapshot,
                created_at_seq,
                &owner_fence_hash,
            );
            let prepared = match result {
                Ok(record) => record,
                Err(StorageError::DurabilityIndeterminate(_)) => self
                    .planner_store
                    .get_prepared_request(&request.request_id)
                    .map_err(HydrationActorError::storage)?
                    .ok_or_else(|| {
                        HydrationActorError::retryable(
                            "planner_request_indeterminate",
                            "planner request flush was indeterminate and no exact intent is durable",
                        )
                    })?,
                Err(error) => return Err(HydrationActorError::storage(error)),
            };
            require_exact_prepared_planner_request(&prepared, &request, created_at_seq)?;
            Some(prepared)
        };
        let mut advanced = checkpoint.clone();
        advanced.stage = AgentHydrationCheckpointStage::ProjectionRequested;
        advanced.planner_request_id = Some(request.request_id.clone());
        advanced.updated_at_seq = expected_projection_checkpoint_sequence(hydration, &checkpoint)?;
        retry_indeterminate(|| {
            self.agent_store
                .advance_hydration_checkpoint_cas_fenced(&checkpoint, &advanced, &owner)
        })
        .map_err(HydrationActorError::storage)?;
        let record = if let Some(prepared) = prepared.as_ref() {
            let activation_fence = self
                .agent_store
                .hydration_activation_fence(&owner, hydration, &advanced)
                .map_err(HydrationActorError::storage)?;
            let activated_at_seq = expected_planner_activation_sequence(hydration, &checkpoint)?;
            retry_indeterminate(|| {
                self.planner_store
                    .activate_prepared(prepared, activated_at_seq, &activation_fence)
            })
            .map_err(HydrationActorError::storage)?
        } else {
            visible.expect("visible request was validated")
        };
        report.projection_requested_count += 1;
        report.output_sequence = report
            .output_sequence
            .max(record.updated_at_seq)
            .max(advanced.updated_at_seq);
        Ok(())
    }

    fn finish_or_wait(
        &self,
        agent: &AgentRecord,
        context: &SeedContext,
        hydration: &AgentProcessHydrationRecord,
        checkpoint: AgentHydrationCheckpoint,
        report: &mut AgentHydrationTickReport,
    ) -> Result<(), HydrationActorError> {
        let attestation = self.load_attestation(agent, context, hydration, &checkpoint)?;
        let expected_request =
            readiness_projection_request(agent, context, hydration, &attestation)?;
        let created_at_seq = expected_planner_request_sequence(hydration, &checkpoint)?;
        let request_id = checkpoint.planner_request_id.as_deref().ok_or_else(|| {
            HydrationActorError::fatal(
                "missing_planner_request_id",
                "projection checkpoint has no planner request identity",
            )
        })?;
        if request_id != expected_request.request_id {
            return Err(HydrationActorError::fatal(
                "divergent_planner_request",
                "checkpoint planner request diverges from canonical hydration input",
            ));
        }
        let visible = self
            .planner_store
            .get_request(request_id)
            .map_err(HydrationActorError::storage)?;
        let record = if let Some(record) = visible {
            require_exact_planner_request(&record, &expected_request, created_at_seq)?;
            record
        } else {
            let prepared = self
                .planner_store
                .get_prepared_request(request_id)
                .map_err(HydrationActorError::storage)?
                .ok_or_else(|| {
                    HydrationActorError::fatal(
                        "missing_planner_request",
                        "checkpoint planner request is missing",
                    )
                })?;
            require_exact_prepared_planner_request(&prepared, &expected_request, created_at_seq)?;
            let owner = context.owner_fence(agent);
            let activation_fence = self
                .agent_store
                .hydration_activation_fence(&owner, hydration, &checkpoint)
                .map_err(HydrationActorError::storage)?;
            let activated_at_seq = expected_planner_activation_sequence(hydration, &checkpoint)?;
            retry_indeterminate(|| {
                self.planner_store
                    .activate_prepared(&prepared, activated_at_seq, &activation_fence)
            })
            .map_err(HydrationActorError::storage)?
        };
        report.input_sequence = report.input_sequence.max(record.updated_at_seq);

        match record.status {
            PlannerProjectionRequestStatus::Pending => {
                report.projection_pending_count += 1;
                push_retryable(
                    report,
                    Some(agent.agent_id.clone()),
                    "planner_projection_pending",
                    "durable planner request is awaiting projection work",
                );
                Ok(())
            }
            PlannerProjectionRequestStatus::Failed => {
                let message = record
                    .last_error
                    .clone()
                    .unwrap_or_else(|| "planner projection failed".to_string());
                self.fail_hydration(
                    agent,
                    context,
                    hydration,
                    &checkpoint,
                    HydrationFailure {
                        causal_floor: checkpoint.updated_at_seq.max(record.updated_at_seq),
                        code: "planner_projection_failed",
                        message: &message,
                    },
                    report,
                )
            }
            PlannerProjectionRequestStatus::Completed => {
                let frame_id = record.frame_id.as_deref().ok_or_else(|| {
                    HydrationActorError::fatal(
                        "missing_planner_frame_id",
                        "completed planner request has no frame identity",
                    )
                })?;
                let frame = self
                    .planner_store
                    .get_frame(frame_id)
                    .map_err(HydrationActorError::storage)?
                    .ok_or_else(|| {
                        HydrationActorError::fatal(
                            "missing_planner_frame",
                            "completed planner request frame is missing",
                        )
                    })?;
                self.planner_store
                    .verify_completed_projection(&record, &frame)
                    .map_err(|error| {
                        HydrationActorError::fatal("malformed_planner_products", error.to_string())
                    })?;
                self.commit_readiness(
                    agent,
                    context,
                    hydration,
                    checkpoint,
                    attestation,
                    record,
                    frame,
                    report,
                )
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn commit_readiness(
        &self,
        agent: &AgentRecord,
        context: &SeedContext,
        hydration: &AgentProcessHydrationRecord,
        checkpoint: AgentHydrationCheckpoint,
        attestation: BeliefReadinessAttestation,
        planner_request: PlannerProjectionRequestRecord,
        planner_frame: PlannerProjectionFrame,
        report: &mut AgentHydrationTickReport,
    ) -> Result<(), HydrationActorError> {
        let goal = readiness_goal(agent, &context.rule);
        let proof = AgentReadinessProof::identified(
            agent.updated_at_seq,
            AgentReadinessSignal::identified(attestation.clone()).map_err(|error| {
                HydrationActorError::fatal("invalid_readiness_signal", error.to_string())
            })?,
            planner_request.clone(),
            planner_frame.clone(),
            goal,
        )
        .map_err(|error| {
            HydrationActorError::fatal("invalid_readiness_proof", error.to_string())
        })?;

        self.revalidate_final(
            agent,
            context,
            hydration,
            &checkpoint,
            &attestation,
            &planner_request,
            &planner_frame,
        )?;
        let updated_at_seq = exact_successor(
            [
                agent.updated_at_seq,
                context.subscription.updated_at_seq,
                hydration.updated_at_seq,
                checkpoint.updated_at_seq,
                attestation.attested_at_seq,
                planner_request.updated_at_seq,
            ]
            .into_iter()
            .max()
            .unwrap_or(0),
            "agent operational sequence",
        )?;
        let command = MarkAgentOperationalCommand {
            hydration_id: hydration.hydration_id.clone(),
            attempt_epoch: hydration.attempt_epoch,
            lease_id: hydration.lease_id.clone(),
            expected_hydration_updated_at_seq: hydration.updated_at_seq,
            readiness: proof,
            updated_at_seq,
        };
        let registration = AgentRegistration::new(self.agent_store.as_ref());
        let owner = context.owner_fence(agent);
        let operational = retry_indeterminate(|| {
            registration.mark_operational_fenced(&command, &owner, &checkpoint)
        })
        .map_err(HydrationActorError::storage)?;
        report.operational_count += 1;
        report.output_sequence = report.output_sequence.max(operational.updated_at_seq);
        Ok(())
    }

    fn hydration_causal_floor(
        &self,
        agent: &AgentRecord,
        context: &SeedContext,
        hydration: &AgentProcessHydrationRecord,
    ) -> Result<u64, HydrationActorError> {
        let mut floor = hydration.started_at_seq.max(hydration.updated_at_seq);
        let Some(checkpoint) = self
            .agent_store
            .hydration_checkpoint(&hydration.hydration_id)
            .map_err(HydrationActorError::storage)?
        else {
            return Ok(floor);
        };
        self.validate_checkpoint(hydration, context, &checkpoint)?;
        floor = floor.max(checkpoint.updated_at_seq);
        if checkpoint.stage == AgentHydrationCheckpointStage::Selected {
            return Ok(floor);
        }

        let attestation = self.load_attestation(agent, context, hydration, &checkpoint)?;
        floor = floor.max(attestation.attested_at_seq);
        if checkpoint.stage == AgentHydrationCheckpointStage::Attested {
            return Ok(floor);
        }

        let expected_request =
            readiness_projection_request(agent, context, hydration, &attestation)?;
        let request_id = checkpoint.planner_request_id.as_deref().ok_or_else(|| {
            HydrationActorError::fatal(
                "missing_planner_request_id",
                "projection checkpoint has no planner request identity",
            )
        })?;
        if request_id != expected_request.request_id {
            return Err(HydrationActorError::fatal(
                "divergent_planner_request",
                "checkpoint planner request diverges from canonical hydration input",
            ));
        }
        let planner_request = self
            .planner_store
            .get_request(request_id)
            .map_err(HydrationActorError::storage)?
            .ok_or_else(|| {
                HydrationActorError::fatal(
                    "missing_planner_request",
                    "checkpoint planner request is missing",
                )
            })?;
        require_exact_planner_request(
            &planner_request,
            &expected_request,
            expected_planner_request_sequence(hydration, &checkpoint)?,
        )?;
        floor = floor.max(planner_request.updated_at_seq);
        if planner_request.status == PlannerProjectionRequestStatus::Completed {
            let frame_id = planner_request.frame_id.as_deref().ok_or_else(|| {
                HydrationActorError::fatal(
                    "missing_planner_frame_id",
                    "completed planner request has no frame identity",
                )
            })?;
            let frame = self
                .planner_store
                .get_frame(frame_id)
                .map_err(HydrationActorError::storage)?
                .ok_or_else(|| {
                    HydrationActorError::fatal(
                        "missing_planner_frame",
                        "completed planner request frame is missing",
                    )
                })?;
            self.planner_store
                .verify_completed_projection(&planner_request, &frame)
                .map_err(|error| {
                    HydrationActorError::fatal("malformed_planner_products", error.to_string())
                })?;
            floor = floor.max(frame.completed_at_seq);
        }
        Ok(floor)
    }

    #[allow(clippy::too_many_arguments)]
    fn revalidate_final(
        &self,
        agent: &AgentRecord,
        context: &SeedContext,
        hydration: &AgentProcessHydrationRecord,
        checkpoint: &AgentHydrationCheckpoint,
        attestation: &BeliefReadinessAttestation,
        planner_request: &PlannerProjectionRequestRecord,
        planner_frame: &PlannerProjectionFrame,
    ) -> Result<(), HydrationActorError> {
        self.require_exact_agent(agent)?;
        let reloaded = self.load_static_context(agent)?;
        if &reloaded != context {
            return Err(HydrationActorError::retryable(
                "seed_context_changed",
                "bootstrap receipt, rule, or subscription changed before readiness commit",
            ));
        }
        self.require_exact_current_hydration(hydration)?;
        let durable_checkpoint = self
            .agent_store
            .hydration_checkpoint(&hydration.hydration_id)
            .map_err(HydrationActorError::storage)?;
        if durable_checkpoint.as_ref() != Some(checkpoint) {
            return Err(HydrationActorError::retryable(
                "hydration_checkpoint_changed",
                "hydration checkpoint changed before readiness commit",
            ));
        }
        let durable_attestation = self
            .belief_store
            .get_readiness_attestation(&attestation.attestation_id)
            .map_err(HydrationActorError::storage)?;
        if durable_attestation.as_ref() != Some(attestation) {
            return Err(HydrationActorError::fatal(
                "readiness_attestation_changed",
                "belief readiness attestation changed or disappeared",
            ));
        }
        self.belief_store
            .verify_readiness_attestation(attestation)
            .map_err(|error| {
                HydrationActorError::fatal("readiness_attestation_invalid", error.to_string())
            })?;
        let durable_request = self
            .planner_store
            .get_request(&planner_request.request.request_id)
            .map_err(HydrationActorError::storage)?;
        let durable_frame = self
            .planner_store
            .get_frame(&planner_frame.identity.frame_id)
            .map_err(HydrationActorError::storage)?;
        if durable_request.as_ref() != Some(planner_request)
            || durable_frame.as_ref() != Some(planner_frame)
        {
            return Err(HydrationActorError::fatal(
                "planner_products_changed",
                "planner request or frame changed before readiness commit",
            ));
        }
        self.planner_store
            .verify_completed_projection(planner_request, planner_frame)
            .map_err(|error| {
                HydrationActorError::fatal("planner_products_invalid", error.to_string())
            })
    }

    fn load_static_context(
        &self,
        selected_agent: &AgentRecord,
    ) -> Result<SeedContext, HydrationActorError> {
        let agent = self.require_exact_agent(selected_agent)?;
        if agent.status != AgentStatus::Registered {
            return Err(HydrationActorError::retryable(
                "agent_not_registered",
                "selected agent is no longer registered",
            ));
        }
        let receipt = self
            .agent_store
            .bootstrap_receipt_for_agent(&agent.agent_id)
            .map_err(HydrationActorError::storage)?
            .ok_or_else(|| {
                HydrationActorError::fatal(
                    "missing_bootstrap_receipt",
                    "registered seed requires one exact completed bootstrap receipt",
                )
            })?;
        if receipt.agent_id != agent.agent_id
            || receipt.directive_id != agent.directive_id
            || receipt.completed_at_seq == 0
        {
            return Err(HydrationActorError::fatal(
                "bootstrap_receipt_mismatch",
                "bootstrap receipt does not match the registered seed",
            ));
        }
        let rule = self
            .agent_store
            .get_curation_rule(&receipt.rule_id)
            .map_err(HydrationActorError::storage)?
            .ok_or_else(|| {
                HydrationActorError::fatal(
                    "missing_curation_rule",
                    "bootstrap curation rule is missing",
                )
            })?;
        rule.config.validate().map_err(|error| {
            HydrationActorError::fatal("invalid_curation_rule", error.to_string())
        })?;
        if rule.rule_id != receipt.rule_id || rule.agent_id != agent.agent_id {
            return Err(HydrationActorError::fatal(
                "curation_rule_mismatch",
                "bootstrap curation rule belongs to another agent",
            ));
        }
        let subscription = self
            .agent_store
            .get_subscription(&receipt.subscription_id)
            .map_err(HydrationActorError::storage)?
            .ok_or_else(|| {
                HydrationActorError::fatal(
                    "missing_subscription",
                    "bootstrap subscription is missing",
                )
            })?;
        subscription.validate().map_err(|error| {
            HydrationActorError::fatal("invalid_subscription", error.to_string())
        })?;
        if subscription.subscription_id != receipt.subscription_id
            || subscription.agent_id != agent.agent_id
            || subscription.status != AgentSubscriptionStatus::Active
            || subscription.belief_key.subject != agent.subject
            || subscription.belief_key.perspective != agent.perspective_key
            || subscription.belief_key.branch_scope != agent.branch_scope
            || subscription.belief_key.dimension_id != rule.config.dimension_id
        {
            return Err(HydrationActorError::fatal(
                "subscription_mismatch",
                "bootstrap subscription, agent scope, and curation rule disagree",
            ));
        }
        let directive = self
            .agent_store
            .get_directive(&receipt.directive_id)
            .map_err(HydrationActorError::storage)?
            .ok_or_else(|| {
                HydrationActorError::fatal("missing_directive", "bootstrap directive is missing")
            })?;
        if directive.directive_id != receipt.directive_id
            || directive.directive_id != agent.directive_id
        {
            return Err(HydrationActorError::fatal(
                "directive_mismatch",
                "bootstrap directive identity disagrees with the registered seed",
            ));
        }
        let belief_config = self
            .belief_store
            .config_snapshot_fence(&receipt.belief.config_snapshot_hash)
            .map_err(HydrationActorError::storage)?
            .ok_or_else(|| {
                HydrationActorError::fatal(
                    "missing_belief_config",
                    "bootstrap belief configuration snapshot is missing",
                )
            })?;
        let belief = BeliefConfigLoader::load_json(&belief_config.json).map_err(|error| {
            HydrationActorError::fatal("invalid_belief_config", error.to_string())
        })?;
        if belief.hash != receipt.belief.config_snapshot_hash
            || belief.config.family_id != receipt.belief.family_id
        {
            return Err(HydrationActorError::fatal(
                "belief_config_mismatch",
                "bootstrap belief configuration identity disagrees with its receipt",
            ));
        }
        let durable_input = WorldModelActivationInput {
            activation_hash: receipt.activation_hash.clone(),
            activation_id: receipt.activation_id.clone(),
            bootstrap_id: receipt.bootstrap_id.clone(),
            belief_family: belief.config,
            directive: directive.clone(),
            seed_agent: SeedAgentActivation {
                agent_id: agent.agent_id.clone(),
                perspective_key: agent.perspective_key.clone(),
                subject: agent.subject.clone(),
                branch_scope: agent.branch_scope.clone(),
                observation_scope: agent.observation_scope.clone(),
                directive_id: agent.directive_id.clone(),
                seed_provenance: agent.seed_provenance.clone(),
            },
            curation_rule: rule.clone(),
            belief_key: subscription.belief_key.clone(),
        };
        let durable_identity =
            validate_world_model_activation(&durable_input).map_err(|error| {
                HydrationActorError::fatal("invalid_bootstrap_authority", error.to_string())
            })?;
        let expected_subscription_id = deterministic_id(
            "subscription",
            &AgentSubscriptionRecord::natural_key(
                &durable_input.seed_agent.agent_id,
                &durable_input.belief_key,
            ),
        );
        if durable_identity.input_hash != receipt.input_hash
            || durable_identity.belief_config_hash != receipt.belief.config_snapshot_hash
            || expected_subscription_id != receipt.subscription_id
        {
            return Err(HydrationActorError::fatal(
                "bootstrap_input_hash_mismatch",
                "durable bootstrap owner content disagrees with the receipt input hash",
            ));
        }
        Ok(SeedContext {
            receipt,
            directive,
            rule,
            subscription,
            belief_config,
        })
    }

    fn select_current_revision(
        &self,
        context: &SeedContext,
    ) -> Result<BeliefRevision, HydrationActorError> {
        let query = BeliefQuery::new(self.belief_store.as_ref());
        let view = query
            .current_view(&context.subscription.belief_key)
            .map_err(HydrationActorError::storage)?
            .ok_or_else(|| {
                HydrationActorError::retryable(
                    "belief_not_ready",
                    "bootstrap subscription has no current belief view",
                )
            })?;
        let revision_id = view.current_revision_id.as_deref().ok_or_else(|| {
            HydrationActorError::retryable(
                "belief_not_ready",
                "current belief view has no revision identity",
            )
        })?;
        let revision = query
            .current_revision(&context.subscription.belief_key)
            .map_err(HydrationActorError::storage)?
            .ok_or_else(|| {
                HydrationActorError::retryable(
                    "belief_not_ready",
                    "bootstrap subscription has no current belief revision",
                )
            })?;
        if view.key != context.subscription.belief_key || revision.revision_id != revision_id {
            return Err(HydrationActorError::retryable(
                "belief_view_changed",
                "belief revision head and current view disagree",
            ));
        }
        if revision.source_cursor_end == 0 || revision.belief_key != context.subscription.belief_key
        {
            return Err(HydrationActorError::fatal(
                "invalid_belief_revision",
                "current belief revision has invalid sequence or scope",
            ));
        }
        Ok(revision)
    }

    fn load_attestation(
        &self,
        agent: &AgentRecord,
        context: &SeedContext,
        hydration: &AgentProcessHydrationRecord,
        checkpoint: &AgentHydrationCheckpoint,
    ) -> Result<BeliefReadinessAttestation, HydrationActorError> {
        let attestation_id = checkpoint.attestation_id.as_deref().ok_or_else(|| {
            HydrationActorError::fatal(
                "missing_attestation_id",
                "hydration checkpoint has no readiness attestation identity",
            )
        })?;
        let visible = self
            .belief_store
            .get_readiness_attestation(attestation_id)
            .map_err(|error| {
                HydrationActorError::fatal("invalid_attestation", error.to_string())
            })?;
        let was_visible = visible.is_some();
        let attestation = match visible {
            Some(attestation) => attestation,
            None => self
                .belief_store
                .get_prepared_readiness_attestation(attestation_id)
                .map_err(|error| {
                    HydrationActorError::fatal("invalid_attestation", error.to_string())
                })?
                .ok_or_else(|| {
                    HydrationActorError::fatal(
                        "missing_attestation",
                        "hydration readiness attestation intent is missing",
                    )
                })?,
        };
        validate_attestation(agent, context, hydration, checkpoint, &attestation)?;
        let owner = context.owner_fence(agent);
        let activation_fence = self
            .agent_store
            .hydration_activation_fence(&owner, hydration, checkpoint)
            .map_err(HydrationActorError::storage)?;
        retry_indeterminate(|| {
            self.belief_store
                .claim_readiness_attestation_fenced(&attestation, &activation_fence)
        })
        .map_err(HydrationActorError::storage)?;
        if !was_visible {
            retry_indeterminate(|| {
                self.belief_store
                    .activate_prepared_readiness_attestation(&attestation, &activation_fence)
            })
            .map_err(HydrationActorError::storage)?;
        }
        self.belief_store
            .verify_readiness_attestation(&attestation)
            .map_err(|error| {
                HydrationActorError::fatal("invalid_attestation", error.to_string())
            })?;
        Ok(attestation)
    }

    fn require_exact_agent(
        &self,
        expected: &AgentRecord,
    ) -> Result<AgentRecord, HydrationActorError> {
        let durable = self
            .agent_store
            .get_agent(&expected.agent_id)
            .map_err(HydrationActorError::storage)?
            .ok_or_else(|| {
                HydrationActorError::retryable("agent_missing", "selected agent disappeared")
            })?;
        durable
            .validate()
            .map_err(|error| HydrationActorError::fatal("invalid_agent", error.to_string()))?;
        if durable.agent_id != expected.agent_id {
            return Err(HydrationActorError::fatal(
                "agent_identity_mismatch",
                "agent payload identity conflicts with its lookup identity",
            ));
        }
        if &durable != expected {
            return Err(HydrationActorError::retryable(
                "agent_changed",
                "selected agent changed before hydration transition",
            ));
        }
        Ok(durable)
    }

    fn require_exact_subscription(
        &self,
        expected: &AgentSubscriptionRecord,
    ) -> Result<(), HydrationActorError> {
        let durable = self
            .agent_store
            .get_subscription(&expected.subscription_id)
            .map_err(HydrationActorError::storage)?;
        if durable.as_ref() != Some(expected) {
            return Err(HydrationActorError::retryable(
                "subscription_changed",
                "hydration subscription changed before transition",
            ));
        }
        Ok(())
    }

    fn require_exact_current_hydration(
        &self,
        expected: &AgentProcessHydrationRecord,
    ) -> Result<(), HydrationActorError> {
        let durable = self
            .agent_store
            .current_process_hydration(&expected.agent_id)
            .map_err(HydrationActorError::storage)?;
        if durable.as_ref() != Some(expected) {
            return Err(HydrationActorError::retryable(
                "hydration_fence_changed",
                "hydration epoch or lease changed before transition",
            ));
        }
        Ok(())
    }

    fn validate_checkpoint(
        &self,
        hydration: &AgentProcessHydrationRecord,
        context: &SeedContext,
        checkpoint: &AgentHydrationCheckpoint,
    ) -> Result<(), HydrationActorError> {
        checkpoint.validate().map_err(|error| {
            HydrationActorError::fatal("invalid_hydration_checkpoint", error.to_string())
        })?;
        if checkpoint.hydration_id != hydration.hydration_id
            || checkpoint.agent_id != hydration.agent_id
            || checkpoint.attempt_epoch != hydration.attempt_epoch
            || checkpoint.lease_id != hydration.lease_id
            || checkpoint.subscription_id != context.subscription.subscription_id
        {
            return Err(HydrationActorError::fatal(
                "hydration_checkpoint_mismatch",
                "hydration checkpoint conflicts with its epoch, lease, or subscription",
            ));
        }
        let expected_sequence = match checkpoint.stage {
            AgentHydrationCheckpointStage::Selected => {
                expected_selected_checkpoint_sequence(hydration, checkpoint)?
            }
            AgentHydrationCheckpointStage::Attested => {
                expected_attested_checkpoint_sequence(hydration, checkpoint)?
            }
            AgentHydrationCheckpointStage::ProjectionRequested => {
                expected_projection_checkpoint_sequence(hydration, checkpoint)?
            }
        };
        if checkpoint.updated_at_seq != expected_sequence {
            return Err(HydrationActorError::fatal(
                "hydration_checkpoint_sequence_mismatch",
                "hydration checkpoint does not preserve the exact causal sequence chain",
            ));
        }
        Ok(())
    }

    fn fail_changed_view(
        &self,
        agent: &AgentRecord,
        context: &SeedContext,
        hydration: &AgentProcessHydrationRecord,
        checkpoint: &AgentHydrationCheckpoint,
        causal_floor: u64,
        report: &mut AgentHydrationTickReport,
    ) -> Result<(), HydrationActorError> {
        self.fail_hydration(
            agent,
            context,
            hydration,
            checkpoint,
            HydrationFailure {
                causal_floor,
                code: "belief_view_changed",
                message: "selected belief view changed before readiness attestation",
            },
            report,
        )
    }

    fn fail_hydration(
        &self,
        agent: &AgentRecord,
        context: &SeedContext,
        hydration: &AgentProcessHydrationRecord,
        checkpoint: &AgentHydrationCheckpoint,
        failure: HydrationFailure<'_>,
        report: &mut AgentHydrationTickReport,
    ) -> Result<(), HydrationActorError> {
        self.require_exact_current_hydration(hydration)?;
        let failed_at_seq = exact_successor(
            hydration.updated_at_seq.max(failure.causal_floor),
            "hydration failure sequence",
        )?;
        let command = FailAgentHydrationCommand {
            hydration_id: hydration.hydration_id.clone(),
            attempt_epoch: hydration.attempt_epoch,
            lease_id: hydration.lease_id.clone(),
            expected_updated_at_seq: hydration.updated_at_seq,
            failed_at_seq,
            error: bounded(failure.message),
        };
        let registration = AgentRegistration::new(self.agent_store.as_ref());
        let owner = context.owner_fence(agent);
        let failed = retry_indeterminate(|| {
            registration.fail_hydration_fenced(&command, &owner, checkpoint)
        })
        .map_err(HydrationActorError::storage)?;
        report.failed_count += 1;
        report.output_sequence = report.output_sequence.max(failed.updated_at_seq);
        push_fatal(
            report,
            Some(hydration.agent_id.clone()),
            failure.code,
            failure.message,
        );
        Ok(())
    }
}

struct HydrationFailure<'a> {
    causal_floor: u64,
    code: &'static str,
    message: &'a str,
}

#[derive(Debug, Clone)]
struct SeedContext {
    receipt: AgentBootstrapReceipt,
    directive: crate::activation::DirectiveRecord,
    rule: AgentCurationRuleRecord,
    subscription: AgentSubscriptionRecord,
    belief_config: BeliefConfigSnapshotFence,
}

impl PartialEq for SeedContext {
    fn eq(&self, other: &Self) -> bool {
        self.receipt == other.receipt
            && self.directive == other.directive
            && self.rule == other.rule
            && self.subscription == other.subscription
            && self.belief_config.hash == other.belief_config.hash
            && self.belief_config.json == other.belief_config.json
    }
}

impl SeedContext {
    fn owner_fence(&self, agent: &AgentRecord) -> AgentHydrationOwnerFence {
        AgentHydrationOwnerFence {
            agent: agent.clone(),
            receipt: self.receipt.clone(),
            directive: self.directive.clone(),
            rule: self.rule.clone(),
            subscription: self.subscription.clone(),
            belief_config: self.belief_config.clone(),
        }
    }
}

#[derive(Debug)]
struct HydrationActorError {
    code: &'static str,
    message: String,
    retryable: bool,
}

impl HydrationActorError {
    fn retryable(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retryable: true,
        }
    }

    fn fatal(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            retryable: false,
        }
    }

    fn storage(error: StorageError) -> Self {
        let retryable = matches!(
            &error,
            StorageError::Backpressure(_)
                | StorageError::Unavailable(_)
                | StorageError::DurabilityIndeterminate(_)
        ) || matches!(
            &error,
            StorageError::IoError(source) if source.kind() != std::io::ErrorKind::InvalidData
        );
        Self {
            code: if matches!(
                &error,
                StorageError::IoError(source) if source.kind() == std::io::ErrorKind::InvalidData
            ) {
                "durable_data_corrupt"
            } else if retryable {
                "storage_retryable"
            } else {
                "storage_fatal"
            },
            message: error.to_string(),
            retryable,
        }
    }
}

fn selected_checkpoint(
    hydration: &AgentProcessHydrationRecord,
    subscription: &AgentSubscriptionRecord,
    revision: &BeliefRevision,
) -> Result<AgentHydrationCheckpoint, HydrationActorError> {
    let checkpoint = AgentHydrationCheckpoint {
        hydration_id: hydration.hydration_id.clone(),
        agent_id: hydration.agent_id.clone(),
        attempt_epoch: hydration.attempt_epoch,
        lease_id: hydration.lease_id.clone(),
        subscription_id: subscription.subscription_id.clone(),
        selected_revision_id: revision.revision_id.clone(),
        selected_revision_seq: revision.source_cursor_end,
        stage: AgentHydrationCheckpointStage::Selected,
        attestation_id: None,
        planner_request_id: None,
        updated_at_seq: exact_successor(
            hydration.started_at_seq.max(revision.source_cursor_end),
            "hydration selection checkpoint sequence",
        )?,
    };
    checkpoint.validate().map_err(|error| {
        HydrationActorError::fatal("invalid_selected_checkpoint", error.to_string())
    })?;
    Ok(checkpoint)
}

fn validate_attestation(
    agent: &AgentRecord,
    context: &SeedContext,
    hydration: &AgentProcessHydrationRecord,
    checkpoint: &AgentHydrationCheckpoint,
    attestation: &BeliefReadinessAttestation,
) -> Result<(), HydrationActorError> {
    attestation
        .validate()
        .map_err(|error| HydrationActorError::fatal("invalid_attestation", error.to_string()))?;
    let expected_seq = expected_attestation_sequence(hydration, checkpoint)?;
    if attestation.agent_id != agent.agent_id
        || attestation.subscription_id != context.subscription.subscription_id
        || attestation.belief_key != context.subscription.belief_key
        || attestation.belief_revision_id != checkpoint.selected_revision_id
        || attestation.attested_at_seq != expected_seq
        || checkpoint
            .attestation_id
            .as_deref()
            .is_some_and(|id| id != attestation.attestation_id)
    {
        return Err(HydrationActorError::fatal(
            "attestation_mismatch",
            "belief readiness attestation conflicts with the selected hydration input",
        ));
    }
    Ok(())
}

fn expected_selected_checkpoint_sequence(
    hydration: &AgentProcessHydrationRecord,
    checkpoint: &AgentHydrationCheckpoint,
) -> Result<u64, HydrationActorError> {
    exact_successor(
        hydration
            .started_at_seq
            .max(checkpoint.selected_revision_seq),
        "hydration selection checkpoint sequence",
    )
}

fn expected_attestation_sequence(
    hydration: &AgentProcessHydrationRecord,
    checkpoint: &AgentHydrationCheckpoint,
) -> Result<u64, HydrationActorError> {
    exact_successor(
        expected_selected_checkpoint_sequence(hydration, checkpoint)?,
        "belief readiness attestation sequence",
    )
}

fn expected_attested_checkpoint_sequence(
    hydration: &AgentProcessHydrationRecord,
    checkpoint: &AgentHydrationCheckpoint,
) -> Result<u64, HydrationActorError> {
    exact_successor(
        expected_attestation_sequence(hydration, checkpoint)?,
        "hydration attested checkpoint sequence",
    )
}

fn expected_planner_request_sequence(
    hydration: &AgentProcessHydrationRecord,
    checkpoint: &AgentHydrationCheckpoint,
) -> Result<u64, HydrationActorError> {
    exact_successor(
        expected_attested_checkpoint_sequence(hydration, checkpoint)?,
        "planner readiness request sequence",
    )
}

fn expected_projection_checkpoint_sequence(
    hydration: &AgentProcessHydrationRecord,
    checkpoint: &AgentHydrationCheckpoint,
) -> Result<u64, HydrationActorError> {
    exact_successor(
        expected_planner_request_sequence(hydration, checkpoint)?,
        "hydration projection checkpoint sequence",
    )
}

fn expected_planner_activation_sequence(
    hydration: &AgentProcessHydrationRecord,
    checkpoint: &AgentHydrationCheckpoint,
) -> Result<u64, HydrationActorError> {
    exact_successor(
        expected_projection_checkpoint_sequence(hydration, checkpoint)?,
        "planner request activation sequence",
    )
}

fn readiness_projection_request(
    agent: &AgentRecord,
    context: &SeedContext,
    hydration: &AgentProcessHydrationRecord,
    attestation: &BeliefReadinessAttestation,
) -> Result<PlannerProjectionRequest, HydrationActorError> {
    #[derive(Serialize)]
    struct Source<'a> {
        hydration_id: &'a str,
        agent_id: &'a str,
        attempt_epoch: u64,
        attestation_id: &'a str,
        rule_id: &'a str,
    }
    let source = Source {
        hydration_id: &hydration.hydration_id,
        agent_id: &agent.agent_id,
        attempt_epoch: hydration.attempt_epoch,
        attestation_id: &attestation.attestation_id,
        rule_id: &context.rule.rule_id,
    };
    let encoded = serde_json::to_vec(&source).map_err(|error| {
        HydrationActorError::fatal("projection_source_encoding", error.to_string())
    })?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(READINESS_PROJECTION_SOURCE_DOMAIN);
    hasher.update(&encoded);
    PlannerProjectionRequest::identified_attested(
        hasher.finalize().to_hex().to_string(),
        agent.agent_id.clone(),
        agent.subject.clone(),
        agent.perspective_key.clone(),
        agent.branch_scope.clone(),
        vec![context.rule.config.dimension_id.clone()],
        Vec::new(),
        PlannerAttestedBeliefSnapshot::from_attestation(attestation),
    )
    .map_err(|error| HydrationActorError::fatal("invalid_planner_request", error.to_string()))
}

fn require_exact_planner_request(
    record: &PlannerProjectionRequestRecord,
    request: &PlannerProjectionRequest,
    created_at_seq: u64,
) -> Result<(), HydrationActorError> {
    record.validate().map_err(|error| {
        HydrationActorError::fatal("invalid_planner_request_record", error.to_string())
    })?;
    if &record.request != request || record.created_at_seq != created_at_seq {
        return Err(HydrationActorError::fatal(
            "divergent_planner_request",
            "planner request identity or creation sequence conflicts with hydration",
        ));
    }
    Ok(())
}

fn require_exact_prepared_planner_request(
    prepared: &PreparedPlannerProjectionRequest,
    request: &PlannerProjectionRequest,
    created_at_seq: u64,
) -> Result<(), HydrationActorError> {
    prepared.request.validate().map_err(|error| {
        HydrationActorError::fatal("invalid_prepared_planner_request", error.to_string())
    })?;
    if &prepared.request != request || prepared.created_at_seq != created_at_seq {
        return Err(HydrationActorError::fatal(
            "divergent_planner_request",
            "prepared planner request identity or creation sequence conflicts with hydration",
        ));
    }
    Ok(())
}

fn readiness_goal(agent: &AgentRecord, rule: &AgentCurationRuleRecord) -> Goal {
    let goal_key = format!(
        "{}::{}::{}::{}::{}",
        agent.agent_id,
        rule.rule_id,
        agent.subject.index_key(),
        agent.branch_scope.branch_id,
        rule.config.dimension_id
    );
    Goal {
        goal_id: deterministic_id("agent-readiness-goal", &goal_key),
        agent_id: agent.agent_id.clone(),
        target: threshold_target(
            agent.subject.clone(),
            rule.config.dimension_id.clone(),
            rule.config.target_condition(),
        ),
        priority: GoalPriority {
            urgency: rule.config.priority_urgency,
            cost_ceiling: None,
        },
        source: GoalSource::Maintenance {
            invariant_description: rule.config.desired_summary.clone(),
        },
        lifecycle: GoalLifecycle::Proposed,
    }
}

fn exact_successor(value: u64, field: &'static str) -> Result<u64, HydrationActorError> {
    value.checked_add(1).ok_or_else(|| {
        HydrationActorError::fatal(
            "sequence_exhausted",
            format!("{field} cannot advance beyond its durable predecessor"),
        )
    })
}

fn retry_indeterminate<T>(
    mut operation: impl FnMut() -> Result<T, StorageError>,
) -> Result<T, StorageError> {
    match operation() {
        Err(StorageError::DurabilityIndeterminate(_)) => operation(),
        result => result,
    }
}

fn push_actor_error(
    report: &mut AgentHydrationTickReport,
    agent_id: Option<String>,
    error: HydrationActorError,
) {
    if error.retryable {
        push_retryable(report, agent_id, error.code, error.message);
    } else {
        push_fatal(report, agent_id, error.code, error.message);
    }
}

fn push_storage(
    report: &mut AgentHydrationTickReport,
    agent_id: Option<String>,
    error: StorageError,
) {
    push_actor_error(report, agent_id, HydrationActorError::storage(error));
}

fn push_retryable(
    report: &mut AgentHydrationTickReport,
    agent_id: Option<String>,
    code: impl Into<String>,
    message: impl Into<String>,
) {
    if report.retryable_errors.len() < MAX_ISSUES {
        report.retryable_errors.push(AgentHydrationIssue {
            agent_id,
            code: code.into(),
            message: bounded(message),
        });
    }
}

fn push_fatal(
    report: &mut AgentHydrationTickReport,
    agent_id: Option<String>,
    code: impl Into<String>,
    message: impl Into<String>,
) {
    if report.fatal_errors.len() < MAX_ISSUES {
        report.fatal_errors.push(AgentHydrationIssue {
            agent_id,
            code: code.into(),
            message: bounded(message),
        });
    }
}

fn bounded(message: impl Into<String>) -> String {
    let message = message.into();
    if message.len() <= MAX_ISSUE_BYTES {
        return message;
    }
    let mut end = MAX_ISSUE_BYTES;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_classification_preserves_indeterminate_retry() {
        let retryable = HydrationActorError::storage(StorageError::DurabilityIndeterminate(
            "flush unknown".to_string(),
        ));
        let fatal =
            HydrationActorError::storage(StorageError::InvalidPath("malformed".to_string()));
        let corrupt = HydrationActorError::storage(StorageError::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "malformed durable bytes",
        )));

        assert!(retryable.retryable);
        assert!(!fatal.retryable);
        assert!(!corrupt.retryable);
        assert_eq!(corrupt.code, "durable_data_corrupt");
    }

    #[test]
    fn report_diagnostics_are_bounded() {
        let mut report = AgentHydrationTickReport::empty();
        for index in 0..MAX_ISSUES + 4 {
            push_retryable(
                &mut report,
                None,
                format!("issue-{index}"),
                "x".repeat(MAX_ISSUE_BYTES + 20),
            );
        }

        assert_eq!(report.retryable_errors.len(), MAX_ISSUES);
        assert!(report
            .retryable_errors
            .iter()
            .all(|issue| issue.message.len() <= MAX_ISSUE_BYTES));
    }
}
