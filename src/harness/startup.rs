//! Read-only correlation of the native Startup nonce's success path and parallel work.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use meld_events::{EventReplayCapability, EventWatermarkCapability};
use meld_execution::task_admission::TaskAdmissionDecision;
use meld_execution::task_network::store::TaskNetworkReader;
use meld_world_model::agent::{
    AgentEpochSpecification, AgentObservationScope, AgentReconciliationIntent, AgentStore,
};
use meld_world_model::belief::{BeliefStatus, BeliefStore};
use meld_world_model::curation::{
    CurationAdmissionDecision, CurationPublicationKind, CurationQuery, CurationStore,
    CurationTerminalDisposition,
};
use meld_world_model::strategy::{PlanMilestoneRequirement, StrategyProduct};
use meld_world_model::world_state::graph::store::TraversalStore;
use serde::{Deserialize, Serialize};

use crate::harness::boot::HarnessError;
use crate::nonce::NonceRequest;
use crate::runtime::assembly::ProductRuntimeAssembly;
use crate::runtime::lifecycle::{ActivationGenerationV1, ActivationLifecycleStore};
use crate::theory::PreparedActivationClosureV1;

/// Omitted generation and epoch select the native current head. An explicit fence
/// requires an exact repeat of a prior inspection, never a reconstruction of past truth.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupAccountRequest {
    pub agent_id: String,
    pub generation_id: Option<String>,
    pub admission_epoch: Option<String>,
    pub nonce_id: Option<String>,
    pub inspection_fence: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    Available,
    Incomplete,
    Stale,
    Conflicted,
    Unavailable,
    Unproved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupPosition {
    ProductCompiled,
    AgentGenesis,
    GenerationCurrent,
    AdmissionOpen,
    NonceInstantiated,
    InitialAssessment,
    GoalIncepted,
    PlannerCut,
    PlanAdmitted,
    TaskAuthorized,
    ExecutionAdmission,
    NetworkAttribution,
    AttemptRecorded,
    NonceEvent,
    GraphVisibility,
    Confirmation,
    BeliefSettled,
    AgentAcceptance,
    GoalSatisfied,
}

impl StartupPosition {
    const ORDERED: [Self; 19] = [
        Self::ProductCompiled,
        Self::AgentGenesis,
        Self::GenerationCurrent,
        Self::AdmissionOpen,
        Self::NonceInstantiated,
        Self::InitialAssessment,
        Self::GoalIncepted,
        Self::PlannerCut,
        Self::PlanAdmitted,
        Self::TaskAuthorized,
        Self::ExecutionAdmission,
        Self::NetworkAttribution,
        Self::AttemptRecorded,
        Self::NonceEvent,
        Self::GraphVisibility,
        Self::Confirmation,
        Self::BeliefSettled,
        Self::AgentAcceptance,
        Self::GoalSatisfied,
    ];

    fn owner(self) -> &'static str {
        match self {
            Self::ProductCompiled => "pds",
            Self::GenerationCurrent | Self::AdmissionOpen => "lifecycle",
            Self::InitialAssessment | Self::Confirmation => "curation",
            Self::PlannerCut => "planner",
            Self::ExecutionAdmission | Self::NetworkAttribution | Self::AttemptRecorded => {
                "execution"
            }
            Self::NonceEvent => "nonce",
            Self::GraphVisibility => "graph",
            Self::BeliefSettled => "belief",
            _ => "agent",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionEvidence {
    pub position: StartupPosition,
    pub owner: String,
    pub state: EvidenceState,
    pub references: Vec<String>,
}

/// An operational return can remain absent after the semantic Goal was satisfied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParallelObligation {
    pub owner: String,
    pub condition: String,
    pub references: Vec<String>,
}

/// Native identities and positions, not an independent health or satisfaction receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartupNonceAccount {
    pub agent_id: String,
    pub assignment_id: Option<String>,
    pub generation_id: Option<String>,
    pub admission_epoch: Option<String>,
    pub nonce_id: Option<String>,
    pub goal_id: Option<String>,
    pub perspective: Option<meld_world_model::world_state::graph::contracts::PerspectiveKey>,
    pub branch_scope: Option<meld_world_model::belief::BranchScope>,
    pub inspection_fence: String,
    pub read_state: EvidenceState,
    pub owner_positions: BTreeMap<String, String>,
    pub positions: Vec<PositionEvidence>,
    pub first_missing: Option<StartupPosition>,
    pub parallel_obligations: Vec<ParallelObligation>,
    /// Native operational returns remain distinct from the Agent's satisfaction receipt.
    pub execution_outcomes: Vec<String>,
    pub lifecycle: Option<ActivationGenerationV1>,
}

/// Cloned native read handles serve both the live process and offline inspection.
#[derive(Clone)]
pub struct StartupAccountReader {
    prepared: Option<PreparedActivationClosureV1>,
    agent: Option<Arc<AgentStore>>,
    curation: Option<Arc<CurationStore>>,
    belief: Option<Arc<BeliefStore>>,
    graph: Option<Arc<TraversalStore>>,
    lifecycle: Option<ActivationLifecycleStore>,
    network: Option<TaskNetworkReader>,
    events: EventReplayCapability,
    watermark: EventWatermarkCapability,
}

impl StartupAccountReader {
    pub fn over_assembly(assembly: &ProductRuntimeAssembly) -> Self {
        let stores = assembly.stores();
        Self {
            prepared: assembly.prepared_activation().cloned(),
            agent: stores.agent_store.opened().cloned(),
            curation: stores.curation_store.opened().cloned(),
            belief: stores.belief_store.opened().cloned(),
            graph: stores.traversal_store.opened().cloned(),
            lifecycle: assembly.lifecycle_store().cloned(),
            network: assembly.task_network_reader(),
            events: assembly.event_authority().replay_capability(),
            watermark: assembly.event_authority().watermark_capability(),
        }
    }

    pub fn inspect(
        &self,
        request: &StartupAccountRequest,
    ) -> Result<StartupNonceAccount, HarnessError> {
        if request.agent_id.trim().is_empty() {
            return Err(invalid("Startup inspection requires an Agent identity"));
        }
        let mut account = self.capture(request)?;
        let confirmation = self.capture(request)?;
        if account.inspection_fence != confirmation.inspection_fence
            || request
                .inspection_fence
                .as_ref()
                .is_some_and(|fence| fence != &account.inspection_fence)
        {
            account.read_state = EvidenceState::Stale;
            account.first_missing = None;
        }
        Ok(account)
    }

    fn capture(
        &self,
        request: &StartupAccountRequest,
    ) -> Result<StartupNonceAccount, HarnessError> {
        let mut account = StartupNonceAccount {
            agent_id: request.agent_id.clone(),
            assignment_id: None,
            generation_id: None,
            admission_epoch: None,
            nonce_id: None,
            goal_id: None,
            perspective: None,
            branch_scope: None,
            inspection_fence: String::new(),
            read_state: EvidenceState::Available,
            owner_positions: BTreeMap::new(),
            positions: Vec::new(),
            first_missing: None,
            parallel_obligations: Vec::new(),
            execution_outcomes: Vec::new(),
            lifecycle: None,
        };
        let watermark = self.watermark.snapshot().map_err(storage)?;
        account.owner_positions.insert(
            "events".into(),
            serde_json::to_string(&watermark).map_err(storage)?,
        );
        if let Some(store) = &self.agent {
            account.owner_positions.insert(
                format!("agent::{}", store.resource_id()),
                store.reconciliation_position().to_string(),
            );
        }
        if let Some(store) = &self.curation {
            account.owner_positions.insert(
                format!("curation::{}", store.resource_id()),
                CurationQuery::new(store).position().to_string(),
            );
        }
        let genesis = self
            .agent
            .as_ref()
            .map(|store| store.genesis_intent_for_agent(&request.agent_id))
            .transpose()
            .map_err(storage)?
            .flatten();
        let prepared = self.prepared.as_ref().filter(|prepared| {
            genesis.as_ref().is_some_and(|genesis| {
                genesis.assignment_id == prepared.assignment.assignment_id
                    && genesis.product_compilation_receipt_id
                        == prepared.product_compilation_receipt_id
            })
        });
        account.add(
            StartupPosition::ProductCompiled,
            self.prepared.is_some(),
            prepared
                .map(|p| {
                    vec![
                        p.product_revision_id.clone(),
                        p.product_compilation_receipt_id.clone(),
                        p.prepared_id.clone(),
                    ]
                })
                .unwrap_or_default(),
        );
        let genesis_receipts = match (&self.agent, &genesis) {
            (Some(store), Some(genesis)) => store
                .genesis_receipts_for_assignment(&genesis.assignment_id)
                .map_err(storage)?
                .into_iter()
                .filter(|receipt| receipt.intent_id == genesis.intent_id)
                .map(|receipt| receipt.genesis_receipt_id)
                .collect(),
            _ => Vec::new(),
        };
        account.add(
            StartupPosition::AgentGenesis,
            self.agent.is_some(),
            genesis_receipts,
        );
        let Some(genesis) = genesis else {
            return account.finish();
        };
        account.assignment_id = Some(genesis.assignment_id.clone());
        account.perspective = Some(genesis.registration.perspective_key.clone());
        account.branch_scope = Some(genesis.registration.branch_scope.clone());
        let condition = genesis
            .registration
            .maintained_condition
            .as_ref()
            .ok_or_else(|| invalid("Agent has no installed maintained intent"))?;
        if condition.condition.observation_scope != AgentObservationScope::AdmissionEpoch {
            return Err(invalid(
                "Agent does not declare admission-epoch observation",
            ));
        }
        let assignment = self
            .lifecycle
            .as_ref()
            .map(|store| store.assignment(&genesis.assignment_id))
            .transpose()
            .map_err(storage)?
            .flatten();
        if let Some(assignment) = &assignment {
            account.owner_positions.insert(
                "lifecycle".into(),
                assignment.lifecycle_revision.to_string(),
            );
        }
        let generation_id = request.generation_id.as_ref().or_else(|| {
            assignment.as_ref().and_then(|a| {
                a.current_generation_id.as_ref().or_else(|| {
                    a.generations
                        .values()
                        .max_by_key(|generation| generation.generation_number)
                        .map(|generation| &generation.generation_id)
                })
            })
        });
        let generation = generation_id.and_then(|id| assignment.as_ref()?.generations.get(id));
        account.generation_id = generation_id.cloned();
        account.add(
            StartupPosition::GenerationCurrent,
            self.lifecycle.is_some(),
            generation
                .map(|g| vec![g.generation_id.clone(), g.last_transition_ref.clone()])
                .unwrap_or_default(),
        );
        if generation.is_some()
            && generation_id
                != assignment
                    .as_ref()
                    .and_then(|a| a.current_generation_id.as_ref())
        {
            account.mark(StartupPosition::GenerationCurrent, EvidenceState::Stale);
        }
        let epoch_id = request.admission_epoch.as_ref().or_else(|| {
            generation
                .and_then(|g| g.current_admission_epoch())
                .map(|epoch| &epoch.epoch_id)
        });
        let epoch = epoch_id.and_then(|id| {
            generation?
                .admission_epochs
                .iter()
                .find(|epoch| &epoch.epoch_id == id)
        });
        account.admission_epoch = epoch_id.cloned();
        account.add(
            StartupPosition::AdmissionOpen,
            self.lifecycle.is_some(),
            epoch
                .map(|e| vec![e.epoch_id.clone(), e.transition_ref.clone()])
                .unwrap_or_default(),
        );
        if epoch.is_some()
            && !generation.is_some_and(|generation| {
                generation.admission_open() && generation.current_admission_epoch() == epoch
            })
        {
            account.mark(StartupPosition::AdmissionOpen, EvidenceState::Stale);
        }
        account.lifecycle = generation.cloned();
        let (Some(generation_id), Some(epoch_id)) = (generation_id, epoch_id) else {
            return account.finish();
        };
        let intent = AgentReconciliationIntent::MaintainedCondition(condition.clone());
        let goal_id = AgentEpochSpecification::goal_identity(
            &intent,
            &genesis,
            generation_id,
            Some(epoch_id),
        );
        account.goal_id = Some(goal_id.clone());
        let store = self.agent.as_ref().unwrap();
        let products = store.epoch_products(&goal_id).map_err(storage)?;
        let nonce = products
            .as_ref()
            .map(|products| {
                products.validate().map_err(storage)?;
                let input = products
                    .task_inputs
                    .iter()
                    .find(|input| input.artifact_type_id == crate::nonce::capability::REQUEST)
                    .ok_or_else(|| invalid("scoped observation has no native nonce input"))?;
                let nonce: NonceRequest =
                    serde_json::from_value(input.content.clone()).map_err(storage)?;
                nonce.validate().map_err(storage)?;
                Ok::<_, HarnessError>(nonce)
            })
            .transpose()?;
        account.nonce_id = nonce.as_ref().map(|nonce| nonce.nonce_id.clone());
        account.add(
            StartupPosition::NonceInstantiated,
            true,
            products
                .as_ref()
                .map(|p| {
                    vec![
                        p.specification.specification_id.clone(),
                        nonce.as_ref().unwrap().nonce_id.clone(),
                    ]
                })
                .unwrap_or_default(),
        );
        if nonce.is_some()
            && request
                .nonce_id
                .as_ref()
                .is_some_and(|expected| account.nonce_id.as_ref() != Some(expected))
        {
            account.mark(
                StartupPosition::NonceInstantiated,
                EvidenceState::Conflicted,
            );
        }
        let Some(products) = products else {
            return account.finish();
        };
        if products.specification.genesis != genesis
            || products.specification.fence.activation_generation != *generation_id
            || products.specification.fence.admission_epoch.as_ref() != Some(epoch_id)
        {
            return Err(invalid("nonce observation names another native lineage"));
        }
        let nonce = nonce.unwrap();
        let event = self
            .events
            .committed_record(&nonce.expected_event_id().map_err(storage)?)
            .map_err(storage)?;
        if let Some(event) = &event {
            if crate::nonce::hydrate(event).map_err(storage)? != nonce {
                return Err(invalid("committed Event names another nonce"));
            }
        }
        let operations = self
            .curation
            .as_ref()
            .map(|store| {
                CurationQuery::new(store).operation_accounts(
                    &products.specification.authority,
                    &products.curation_rule.revision_ref(),
                )
            })
            .transpose()
            .map_err(storage)?
            .unwrap_or_default();
        // Initial evidence may be standing or Agent-planned. Keep it separate
        // from confirmation by the native source cut relative to the nonce.
        let initial: Vec<_> = operations
            .iter()
            .filter(|entry| {
                event.as_ref().is_none_or(|event| {
                    entry.operation.source_cut.event_position.after_seq < event.seq
                })
            })
            .collect();
        for entry in &operations {
            if let Some(result) = &entry.result {
                for kind in [
                    CurationPublicationKind::Semantic,
                    CurationPublicationKind::Terminal,
                ] {
                    if kind == CurationPublicationKind::Semantic
                        && result.semantic_publication.is_none()
                    {
                        continue;
                    }
                    if !entry
                        .publications
                        .iter()
                        .any(|receipt| receipt.kind == kind)
                    {
                        account.parallel_obligations.push(ParallelObligation {
                            owner: "curation".into(),
                            condition: format!("{kind:?}_publication_pending"),
                            references: vec![
                                entry.operation.operation_id.clone(),
                                result.result_id.clone(),
                            ],
                        });
                    }
                }
            }
        }
        let initial_results: Vec<_> = initial
            .iter()
            .filter_map(|entry| entry.result.as_ref())
            .collect();
        account.add(
            StartupPosition::InitialAssessment,
            self.curation.is_some(),
            initial_results
                .iter()
                .map(|r| r.result_id.clone())
                .collect(),
        );
        if !initial_results.is_empty()
            && !initial_results
                .iter()
                .any(|r| successful_curation(r.disposition))
        {
            account.mark(
                StartupPosition::InitialAssessment,
                EvidenceState::Conflicted,
            );
        }
        let goal = store.reconciliation_goal(&goal_id).map_err(storage)?;
        account.add(
            StartupPosition::GoalIncepted,
            true,
            goal.as_ref()
                .map(|g| vec![g.goal.goal_id.clone(), g.context_id.clone()])
                .unwrap_or_default(),
        );
        let plan = store
            .current_reconciliation_plan(&goal_id)
            .map_err(storage)?;
        let mut cuts = BTreeSet::new();
        if let Some(plan) = &plan {
            if let Some(cut) = store
                .reconciliation_cut(&plan.planner_cut_id)
                .map_err(storage)?
            {
                cuts.insert(cut.cut_id);
            }
        }
        for judgment in store.condition_judgments().map_err(storage)? {
            if judgment.agent_id == request.agent_id
                && judgment.activation_generation == *generation_id
            {
                if let Some(cut) = store
                    .reconciliation_cut(&judgment.planner_cut_id)
                    .map_err(storage)?
                {
                    if cut.context.goal_id == goal_id {
                        cuts.insert(cut.cut_id);
                    }
                }
            }
        }
        account.add(
            StartupPosition::PlannerCut,
            true,
            cuts.into_iter().collect(),
        );
        account.add(
            StartupPosition::PlanAdmitted,
            true,
            plan.as_ref()
                .map(|p| vec![p.plan_revision_id.clone()])
                .unwrap_or_default(),
        );
        let authorizations = store
            .product_authorizations_for_goal(&goal_id)
            .map_err(storage)?;
        let tasks: BTreeSet<_> = authorizations
            .iter()
            .filter(|a| matches!(a.product, StrategyProduct::Task(_)))
            .map(|a| a.authorization_id.clone())
            .collect();
        account.add(
            StartupPosition::TaskAuthorized,
            true,
            tasks.iter().cloned().collect(),
        );
        let network = self
            .network
            .as_ref()
            .map(|reader| reader.snapshot())
            .transpose()
            .map_err(storage)?;
        if let Some(network) = &network {
            account.owner_positions.insert(
                format!("execution::{}", network.network_id),
                format!("{}::{}", network.revision, network.state_hash),
            );
        }
        let admissions: Vec<_> = network
            .iter()
            .flat_map(|n| n.admissions.values())
            .filter(|a| tasks.contains(&a.request.lineage.authorization_id))
            .collect();
        account.add(
            StartupPosition::ExecutionAdmission,
            self.network.is_some(),
            admissions
                .iter()
                .filter(|a| a.decision == TaskAdmissionDecision::Admitted)
                .map(|a| a.admission_id.clone())
                .collect(),
        );
        if !admissions.is_empty()
            && admissions
                .iter()
                .all(|a| a.decision != TaskAdmissionDecision::Admitted)
        {
            account.mark(
                StartupPosition::ExecutionAdmission,
                EvidenceState::Conflicted,
            );
        }
        let attributed = |a: &meld_execution::task_network::state::TaskAdmissionAttribution| {
            tasks.contains(&a.authorization_id)
                && a.goal_id == goal_id
                && a.activation_generation == *generation_id
                && a.admission_epoch.as_ref() == Some(epoch_id)
        };
        let nodes: Vec<_> = network
            .iter()
            .flat_map(|n| n.tasks.values())
            .filter(|n| n.lineage.admission.as_ref().is_some_and(&attributed))
            .collect();
        account.add(
            StartupPosition::NetworkAttribution,
            self.network.is_some(),
            nodes.iter().map(|n| n.task_instance_id.clone()).collect(),
        );
        let claims: Vec<_> = network
            .iter()
            .flat_map(|n| n.claims.values())
            .filter(|c| c.admission.as_ref().is_some_and(&attributed))
            .collect();
        account.add(
            StartupPosition::AttemptRecorded,
            self.network.is_some(),
            claims.iter().map(|c| c.claim_id.clone()).collect(),
        );
        if let Some(network) = &network {
            account.execution_outcomes = network
                .outcomes
                .values()
                .filter(|outcome| outcome.admission.as_ref().is_some_and(&attributed))
                .map(|outcome| outcome.outcome_id.clone())
                .collect();
            for publication in network.publications.values().filter(|publication| {
                publication
                    .outcome
                    .admission
                    .as_ref()
                    .is_some_and(&attributed)
                    && !matches!(
                        publication.state,
                        meld_execution::task_network::PublicationState::Published { .. }
                    )
            }) {
                account.parallel_obligations.push(ParallelObligation {
                    owner: "execution".into(),
                    condition: "outcome_publication_pending".into(),
                    references: vec![
                        publication.publication_id.clone(),
                        publication.outcome.outcome_id.clone(),
                    ],
                });
            }
            for claim in claims {
                if !network
                    .outcomes
                    .values()
                    .any(|o| o.claim_id == claim.claim_id)
                {
                    account.parallel_obligations.push(ParallelObligation {
                        owner: "execution".into(),
                        condition: "claimed_operation_outcome_unresolved".into(),
                        references: vec![claim.claim_id.clone(), claim.task_instance_id.clone()],
                    });
                }
            }
        }
        account.add(
            StartupPosition::NonceEvent,
            true,
            event
                .as_ref()
                .map(|event| {
                    vec![
                        event.record_id.clone().unwrap(),
                        format!("{}::{}", watermark.ledger_id, event.seq),
                    ]
                })
                .unwrap_or_default(),
        );
        let mut visible = Vec::new();
        if let Some(graph) = &self.graph {
            let cursor = graph
                .authority_cursor(watermark.ledger_id)
                .map_err(storage)?;
            account.owner_positions.insert(
                format!("graph::{}", graph.resource_id()),
                cursor.after_seq.to_string(),
            );
            if let Some(event) = &event {
                let expected = nonce.publication().map_err(storage)?;
                for publication in graph
                    .owner_publications_through_seq(cursor.after_seq)
                    .map_err(storage)?
                {
                    if publication.operation == expected
                        && publication.source_event.ledger_id == watermark.ledger_id
                        && publication.source_event.seq == event.seq
                    {
                        visible.push(publication.operation.operation_id);
                    }
                }
            }
        }
        account.add(
            StartupPosition::GraphVisibility,
            self.graph.is_some(),
            visible,
        );
        let confirmation_ids: BTreeSet<_> = authorizations
            .iter()
            .filter_map(|a| match &a.product {
                StrategyProduct::Epistemic(op) => Some(op.operation.operation_id.clone()),
                _ => None,
            })
            .collect();
        let confirmations: Vec<_> = operations
            .iter()
            .filter(|entry| confirmation_ids.contains(&entry.operation.operation_id))
            .collect();
        let confirmed: Vec<_> = confirmations
            .iter()
            .filter(|entry| {
                entry
                    .acceptance
                    .as_ref()
                    .is_some_and(|a| a.decision == CurationAdmissionDecision::Admitted)
            })
            .filter_map(|entry| entry.result.as_ref())
            .collect();
        account.add(
            StartupPosition::Confirmation,
            self.curation.is_some(),
            confirmed
                .iter()
                .filter(|r| successful_curation(r.disposition))
                .map(|r| r.result_id.clone())
                .collect(),
        );
        if !confirmed.is_empty()
            && confirmed
                .iter()
                .all(|r| !successful_curation(r.disposition))
        {
            account.mark(StartupPosition::Confirmation, EvidenceState::Conflicted);
        }
        let mut settled = Vec::new();
        if let Some(belief) = &self.belief {
            for subscription in products.subscription_requests().map_err(storage)? {
                if let Some(revision) = belief
                    .current_revision(&subscription.belief_key)
                    .map_err(storage)?
                {
                    account.owner_positions.insert(
                        format!("belief::{}", subscription.request_id),
                        revision.revision_id.clone(),
                    );
                    if revision.status == BeliefStatus::Settled && !revision.freshness.stale {
                        settled.push(revision.revision_id);
                    }
                }
            }
        }
        account.add(
            StartupPosition::BeliefSettled,
            self.belief.is_some(),
            settled.clone(),
        );
        let milestones = store.milestones_for_goal(&goal_id).map_err(storage)?;
        account.add(
            StartupPosition::AgentAcceptance,
            true,
            milestones
                .iter()
                .filter(|m| {
                    matches!(
                        m.requirement,
                        PlanMilestoneRequirement::BeliefRevision { .. }
                    ) && settled.contains(&m.owner_position_id)
                })
                .map(|m| m.milestone_id.clone())
                .collect(),
        );
        let disposition = plan
            .as_ref()
            .map(|p| store.goal_disposition_for_plan(&p.plan_revision_id))
            .transpose()
            .map_err(storage)?
            .flatten();
        account.add(
            StartupPosition::GoalSatisfied,
            true,
            disposition
                .filter(|d| matches!(d.lifecycle, meld_lang::GoalLifecycle::Satisfied { .. }))
                .map(|d| vec![d.disposition_id])
                .unwrap_or_default(),
        );
        account.finish()
    }
}

impl StartupNonceAccount {
    fn add(&mut self, position: StartupPosition, readable: bool, references: Vec<String>) {
        let state = if !readable {
            EvidenceState::Unavailable
        } else if references.is_empty() {
            EvidenceState::Incomplete
        } else {
            EvidenceState::Available
        };
        self.positions.push(PositionEvidence {
            position,
            owner: position.owner().into(),
            state,
            references,
        });
    }
    fn mark(&mut self, position: StartupPosition, state: EvidenceState) {
        if let Some(entry) = self
            .positions
            .iter_mut()
            .find(|entry| entry.position == position)
        {
            entry.state = state;
        }
    }
    fn finish(mut self) -> Result<Self, HarnessError> {
        for position in StartupPosition::ORDERED {
            if !self
                .positions
                .iter()
                .any(|entry| entry.position == position)
            {
                self.positions.push(PositionEvidence {
                    position,
                    owner: position.owner().into(),
                    state: EvidenceState::Unproved,
                    references: Vec::new(),
                });
            }
        }
        self.positions.sort_by_key(|entry| {
            StartupPosition::ORDERED
                .iter()
                .position(|position| *position == entry.position)
        });
        self.first_missing = self
            .positions
            .iter()
            .find(|entry| entry.state != EvidenceState::Available)
            .map(|entry| entry.position);
        self.inspection_fence = format!(
            "startup-inspection-v1::{}",
            blake3::hash(&serde_json::to_vec(&self).map_err(storage)?).to_hex()
        );
        Ok(self)
    }
}

fn successful_curation(disposition: CurationTerminalDisposition) -> bool {
    matches!(
        disposition,
        CurationTerminalDisposition::Applied | CurationTerminalDisposition::Unchanged
    )
}
fn storage(error: impl std::fmt::Display) -> HarnessError {
    HarnessError::Storage(error.to_string())
}
fn invalid(message: &str) -> HarnessError {
    HarnessError::Storage(message.into())
}
