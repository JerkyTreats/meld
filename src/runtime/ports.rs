//! Thin direct handoff ports built by product runtime assembly.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use meld_events::error::EventAuthorityError;
use meld_events::{
    AppendMode, AppendReceipt, EventAppendCapability, EventAuthority,
    EventConsumerRegistryCapability, EventEnvelope, EventObservabilityCapability, EventPage,
    EventRecord, EventReplayCapability, EventWatermark, EventWatermarkCapability, LedgerCursor,
    LedgerIdentity, ReplayRequest,
};
use meld_execution::task::{TaskArtifactRepoFactory, TaskInitializationPayload};
use meld_execution::task_admission::{
    ExecutionTask, TaskAdmissionApi, TaskAdmissionDecision, TaskAdmissionLineage,
    TaskAdmissionRecord, TaskAdmissionRequest,
};
use meld_execution::task_network::dispatch::Claim;
use meld_execution::task_network::dispatch_actor::{
    AdmissionGenerationObserver, ClaimedInvocationOutcome, ClaimedTaskInvoker, DispatchPortError,
};
use meld_execution::task_network::sharing::admission_discharge_account;
use meld_execution::task_network::state::TaskNode;
use meld_execution::task_network::store::TaskNetworkStoreFactory;
use meld_execution::task_network::{
    EventAppendSink, JournalRecord, PublicationState, SledTaskNetworkStore,
};
use meld_world_model::belief::EvidenceEventReplaySource;
use meld_world_model::planner::{
    PlannerAssemblyOutcome, PlannerCurrentAssemblyRequest, PlannerQuery,
};
use meld_world_model::world_state::graph::contracts::{
    BoundedTraversalRequest, TraversalCut, TraversalCutRequest, TraversalResult,
};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::world_state::graph::{
    GraphConsumerCursorReporter, GraphDerivedEventSink, GraphEventReplaySource,
};
use meld_world_model::TraversalQuery;
use meld_world_model::{
    AgentAuthorityPort, AgentAuthorizationFence, AgentAuthorizedProduct, AgentCurationPort,
    AgentExecutionAdmissionDecision, AgentExecutionPort, AgentExecutionPosition, AgentPlannerPort,
    AgentProductAuthorization, AgentStatus, AgentStore, BeliefQuery, BeliefStore,
    CurationAcceptanceRecord, CurationEventPort, CurationOperation, CurationResult, CurationStore,
    CurationTraversalPort,
};

use crate::context::frame::FrameStorage;
use crate::control::projection::ExecutionProjectionReplaySource;
use crate::prompt_context::PromptContextArtifactStorage;
use crate::runtime::error::{RuntimeAssemblyError, RuntimePortError};
use crate::runtime::storage::{OpenProductStores, ScopedResource};
use crate::store::SledNodeRecordStore;

/// Maximum events returned by one root-assembled replay port call.
pub const MAX_EVENT_REPLAY_LIMIT: usize = 1024;

/// All direct handoff and root adapter ports produced by product assembly.
///
/// Store-backed ports are scoped: a composition built from an explicit
/// registration set constructs only the ports whose stores that set opened.
/// Event-authority ports are always present because the authority is
/// resolved and supplied by product binding.
pub struct ProductRuntimePorts {
    event_append: ProductEventAppendPort,
    event_replay: ProductEventReplayPort,
    graph_cursor: ProductGraphCursorPort,
    adapters: RuntimeAdapterPorts,
}

/// Passive root adapter ports shared with runtime factories.
pub struct RuntimeAdapterPorts {
    context: ScopedResource<ContextRuntimePort>,
    provider: ProviderRuntimePort,
    prompt: ScopedResource<PromptRuntimePort>,
    workspace: ScopedResource<WorkspaceRuntimePort>,
    task_artifacts: ScopedResource<TaskArtifactFactoryPort>,
    task_networks: ScopedResource<TaskNetworkFactoryPort>,
}

/// Provider availability settings checked during assembly.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderPortConfig {
    /// Whether enabled runtime factories require provider access.
    pub provider_required: bool,
    /// Whether provider config and credentials are available.
    pub provider_available: bool,
    /// Environment variables that may satisfy provider availability.
    pub required_env_vars: Vec<String>,
}

/// Execution callable event append port backed by an authority capability.
///
/// The port appends envelopes idempotently through the single-writer ingress
/// so producer appends share group commits, and returns the event sequence.
/// It does not inspect payload meaning, choose publication readiness, or mark
/// execution publication state.
#[derive(Clone)]
pub struct ProductEventAppendPort {
    append: EventAppendCapability,
    watermark: EventWatermarkCapability,
    observability: EventObservabilityCapability,
}

/// Exact Traversal query adapter supplied to standing Curation.
#[derive(Clone)]
pub struct ProductCurationTraversalPort {
    store: Arc<TraversalStore>,
}

/// Bounded event replay source backed by an authority capability.
///
/// The port reads caller-supplied windows without storing replay cursors or
/// deciding which world model reducer should run.
#[derive(Clone)]
pub struct ProductEventReplayPort {
    replay: EventReplayCapability,
}

/// Graph consumer cursor reporter backed by the authority registry.
#[derive(Clone)]
pub struct ProductGraphCursorPort {
    registry: EventConsumerRegistryCapability,
}

/// Exact current Planner assembly boundary used by Agent reconciliation.
#[derive(Clone)]
pub struct ProductAgentPlannerPort {
    curation_store: Arc<CurationStore>,
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
    event_append: ProductEventAppendPort,
    request: PlannerCurrentAssemblyRequest,
}

/// Structural Planner inputs known before native Agent selects an epoch observation.
#[derive(Clone)]
pub struct ProductEpochPlannerBinding {
    pub belief_family: meld_world_model::belief::TheoryRevisionRef,
    pub outcome_mappings: Vec<meld_world_model::belief::TheoryRevisionRef>,
    pub context: meld_world_model::planner::PlannerDecisionContext,
    pub policy: meld_world_model::planner::PlannerAssemblyPolicy,
    pub belief_key: meld_world_model::belief::BeliefKey,
    pub unanchored_belief: bool,
    pub source_positions: Vec<meld_world_model::planner::PlannerSourcePosition>,
}

pub struct ProductEpochAgentPlannerPort {
    curation_store: Arc<CurationStore>,
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
    event_append: ProductEventAppendPort,
    binding: ProductEpochPlannerBinding,
}

impl ProductEpochAgentPlannerPort {
    pub fn new(
        belief_store: Arc<BeliefStore>,
        traversal_store: Arc<TraversalStore>,
        curation_store: Arc<CurationStore>,
        event_append: ProductEventAppendPort,
        binding: ProductEpochPlannerBinding,
    ) -> Self {
        Self {
            curation_store,
            belief_store,
            traversal_store,
            event_append,
            binding,
        }
    }
}

impl AgentPlannerPort for ProductEpochAgentPlannerPort {
    fn publication_visibility(
        &self,
        cut: &meld_world_model::world_state::graph::contracts::TraversalCut,
        expected: &meld_world_model::world_state::graph::contracts::OwnerPublicationExpectation,
    ) -> Result<
        Option<meld_world_model::world_state::graph::visibility::OwnerPublicationVisibilityProof>,
        meld_world_model::error::StorageError,
    > {
        meld_world_model::TraversalQuery::new(self.traversal_store.as_ref())
            .publication_visibility(cut, expected)
    }

    fn assemble(&self) -> PlannerAssemblyOutcome {
        PlannerAssemblyOutcome::Refused(meld_world_model::PlannerRefusal {
            request_context_id: self.binding.context.context_id.clone(),
            grounds: vec![meld_world_model::PlannerRefusalGround::InvalidInput {
                detail: "Planner requires native epoch products for this observation".into(),
            }],
        })
    }

    fn assemble_epoch(
        &self,
        products: &meld_world_model::agent::AgentEpochProducts,
        fence: &AgentAuthorizationFence,
    ) -> PlannerAssemblyOutcome {
        let current = match self.event_append.watermark() {
            Ok(current) => current,
            Err(error) => {
                return PlannerAssemblyOutcome::Refused(meld_world_model::PlannerRefusal {
                    request_context_id: self.binding.context.context_id.clone(),
                    grounds: vec![meld_world_model::PlannerRefusalGround::InvalidInput {
                        detail: error.to_string(),
                    }],
                })
            }
        };
        let request = PlannerCurrentAssemblyRequest {
            required_derived_evidence: products.curation_rule.rule.publishes_source_judgments().then(|| meld_world_model::planner::PlannerDerivedEvidenceRequirement {
                curation_rule: products.curation_rule.revision_ref(),
                belief_family: self.binding.belief_family.clone(),
                outcome_mappings: self.binding.outcome_mappings.clone(),
            }),
            required_graph_evidence: products.curation_rule.rule.source_readiness_requirements(),
            context: self.binding.context.clone(), policy: self.binding.policy.clone(),
            belief_key: self.binding.belief_key.clone(), unanchored_belief: self.binding.unanchored_belief,
            source_positions: self.binding.source_positions.clone(),
            traversal_request: products.curation_rule.rule.traversal_request(),
            traversal_cut_request: TraversalCutRequest {
                owners: vec![], scope: products.curation_rule.rule.scope.clone(),
                currentness: meld_world_model::world_state::graph::contracts::OwnerCurrentnessPolicy::LatestComplete,
                event_position: LedgerCursor { ledger_id: current.ledger_id, after_seq: current.committed_seq },
            },
        };
        ProductAgentPlannerPort::new(
            self.belief_store.clone(),
            self.traversal_store.clone(),
            self.curation_store.clone(),
            self.event_append.clone(),
            request,
        )
        .assemble_epoch(products, fence)
    }
}

/// Read-only live activation and authority-policy observer for Agent.
#[derive(Clone)]
pub(crate) struct ProductAgentAuthorityPort {
    admission: ProductAdmissionGenerationObserver,
    agent_id: String,
    authority_policy_content_hash: String,
}

/// Read-only adapter from the prepared assignment lifecycle to consumer admission fencing.
#[derive(Clone)]
pub struct ProductAdmissionGenerationObserver {
    agent_store: Arc<AgentStore>,
    lifecycle: crate::runtime::lifecycle::ActivationLifecycleStore,
    assignment_id: String,
    activation_id: String,
    prepared_id: String,
}

#[derive(Clone)]
pub(crate) struct ProductCurationAuthorityPort {
    pub admission: ProductAdmissionGenerationObserver,
    pub authority: meld_world_model::CurationAuthority,
}

impl meld_world_model::curation::CurationAuthorityPort for ProductCurationAuthorityPort {
    fn observe(
        &self,
    ) -> Result<Option<meld_world_model::CurationAuthority>, meld_world_model::error::StorageError>
    {
        Ok(self
            .admission
            .observe_epoch(&self.authority.agent_id)
            .map_err(meld_world_model::error::StorageError::InvalidPath)?
            .map(|epoch| {
                let mut authority = self.authority.clone();
                authority.activation_generation = epoch.generation_id;
                authority.admission_epoch = Some(epoch.epoch_id);
                authority
            }))
    }
}

/// Planned Curation intake over the canonical Curation store and actor authority.
#[derive(Clone)]
pub struct ProductPlannedCurationPort {
    store: Arc<CurationStore>,
}

/// Root adapter from exact Agent Task authority to Execution positions.
#[derive(Clone)]
pub struct ProductAgentExecutionPort {
    network: Arc<Mutex<SledTaskNetworkStore>>,
    catalog: meld_execution::capability::CapabilityCatalog,
    authority: Arc<dyn AgentAuthorityPort>,
    authority_policy_content_hash: String,
}

/// Context frame adapter port.
#[derive(Clone)]
pub struct ContextRuntimePort {
    storage: Arc<FrameStorage>,
}

/// Provider adapter availability port.
#[derive(Clone)]
pub struct ProviderRuntimePort {
    config: ProviderPortConfig,
}

/// Prompt artifact adapter port.
#[derive(Clone)]
pub struct PromptRuntimePort {
    storage: Arc<PromptContextArtifactStorage>,
}

/// Workspace node record adapter port.
#[derive(Clone)]
pub struct WorkspaceRuntimePort {
    store: Arc<SledNodeRecordStore>,
}

/// Task artifact repository factory port.
#[derive(Clone)]
pub struct TaskArtifactFactoryPort {
    factory: TaskArtifactRepoFactory,
}

/// Task network store factory port.
#[derive(Clone)]
pub struct TaskNetworkFactoryPort {
    factory: TaskNetworkStoreFactory,
}

impl ProductRuntimePorts {
    /// Build root adapters from one already-resolved event authority.
    ///
    /// Ports are built only for store groups the composition opened; a
    /// closed store leaves its port closed rather than failing assembly.
    pub fn from_authority(
        stores: &OpenProductStores,
        authority: &EventAuthority,
        provider: ProviderPortConfig,
    ) -> Result<Self, RuntimeAssemblyError> {
        let provider_port = ProviderRuntimePort::new(provider)?;
        let event_append = ProductEventAppendPort::new(authority);
        let event_replay = ProductEventReplayPort::new(authority.replay_capability());

        Ok(Self {
            event_append,
            event_replay: event_replay.clone(),
            graph_cursor: ProductGraphCursorPort::new(authority.consumer_registry_capability()),
            adapters: RuntimeAdapterPorts {
                context: match stores.frame_storage.opened() {
                    Some(storage) => ScopedResource::open(
                        "context_port",
                        ContextRuntimePort::new(Arc::clone(storage)),
                    ),
                    None => ScopedResource::closed("context_port"),
                },
                provider: provider_port,
                prompt: match stores.prompt_artifacts.opened() {
                    Some(storage) => ScopedResource::open(
                        "prompt_port",
                        PromptRuntimePort::new(Arc::clone(storage)),
                    ),
                    None => ScopedResource::closed("prompt_port"),
                },
                workspace: match stores.node_store.opened() {
                    Some(store) => ScopedResource::open(
                        "workspace_port",
                        WorkspaceRuntimePort::new(Arc::clone(store)),
                    ),
                    None => ScopedResource::closed("workspace_port"),
                },
                task_artifacts: match stores.task_artifacts.opened() {
                    Some(factory) => ScopedResource::open(
                        "task_artifacts_port",
                        TaskArtifactFactoryPort::new(factory.clone()),
                    ),
                    None => ScopedResource::closed("task_artifacts_port"),
                },
                task_networks: match stores.task_networks.opened() {
                    Some(factory) => ScopedResource::open(
                        "task_networks_port",
                        TaskNetworkFactoryPort::new(factory.clone()),
                    ),
                    None => ScopedResource::closed("task_networks_port"),
                },
            },
        })
    }

    /// Return the execution callable event append port.
    pub fn event_append(&self) -> &ProductEventAppendPort {
        &self.event_append
    }

    /// Return the bounded event replay port.
    pub fn event_replay(&self) -> &ProductEventReplayPort {
        &self.event_replay
    }

    /// Return the graph consumer cursor reporter.
    pub fn graph_cursor(&self) -> &ProductGraphCursorPort {
        &self.graph_cursor
    }

    /// Return passive adapter ports.
    pub fn adapters(&self) -> &RuntimeAdapterPorts {
        &self.adapters
    }
}

impl RuntimeAdapterPorts {
    /// Return the context frame adapter port.
    pub fn context(&self) -> &ContextRuntimePort {
        &self.context
    }

    /// Return the provider adapter port.
    pub fn provider(&self) -> &ProviderRuntimePort {
        &self.provider
    }

    /// Return the prompt artifact adapter port.
    pub fn prompt(&self) -> &PromptRuntimePort {
        &self.prompt
    }

    /// Return the workspace node adapter port.
    pub fn workspace(&self) -> &WorkspaceRuntimePort {
        &self.workspace
    }

    /// Return the task artifact factory adapter port.
    pub fn task_artifacts(&self) -> &TaskArtifactFactoryPort {
        &self.task_artifacts
    }

    /// Return the task artifact factory port when its store is in scope.
    pub fn try_task_artifacts(&self) -> Option<&TaskArtifactFactoryPort> {
        self.task_artifacts.opened()
    }

    /// Return the task network factory adapter port.
    pub fn task_networks(&self) -> &TaskNetworkFactoryPort {
        &self.task_networks
    }

    /// Return the task network factory port when its store is in scope.
    pub fn try_task_networks(&self) -> Option<&TaskNetworkFactoryPort> {
        self.task_networks.opened()
    }
}

impl ProductEventAppendPort {
    pub(crate) fn append_capability(&self) -> meld_events::EventAppendCapability {
        self.append.clone()
    }

    pub fn watermark_capability(&self) -> EventWatermarkCapability {
        self.watermark.clone()
    }
    /// Bind the port to capabilities derived from one event authority.
    pub fn new(authority: &EventAuthority) -> Self {
        Self {
            append: authority.append_capability(),
            watermark: authority.watermark_capability(),
            observability: authority.observability_capability(),
        }
    }

    /// Returns an identity-bearing recovered watermark snapshot.
    pub fn watermark(&self) -> Result<EventWatermark, RuntimePortError> {
        self.watermark
            .snapshot()
            .map_err(|error| RuntimePortError::EventAppend(error.to_string()))
    }

    /// Waits for a commit beyond the supplied identity-bearing cursor.
    pub fn wait_past(
        &self,
        cursor: LedgerCursor,
        timeout: std::time::Duration,
    ) -> Result<EventWatermark, RuntimePortError> {
        self.watermark
            .wait_past(cursor, timeout)
            .map_err(|error| RuntimePortError::EventAppend(error.to_string()))
    }

    /// Computes authority-bound event health for provisional runtime sensing.
    pub fn health(&self) -> Result<meld_events::EventHealthReport, RuntimePortError> {
        self.observability
            .health(self.append.ledger_identity())
            .map_err(|error| RuntimePortError::EventAppend(error.to_string()))
    }
}

impl CurationEventPort for ProductEventAppendPort {
    fn watermark(&self) -> Result<EventWatermark, String> {
        ProductEventAppendPort::watermark(self).map_err(|error| error.to_string())
    }

    fn append_idempotent(&self, envelope: EventEnvelope) -> Result<AppendReceipt, String> {
        self.append
            .append_durable(envelope, AppendMode::Idempotent)
            .map_err(|error| error.to_string())
    }
}

impl ProductCurationTraversalPort {
    /// Bind standing Curation to the canonical Traversal store.
    pub fn new(store: Arc<TraversalStore>) -> Self {
        Self { store }
    }
}

impl CurationTraversalPort for ProductCurationTraversalPort {
    fn cut(
        &self,
        request: &TraversalCutRequest,
    ) -> Result<TraversalCut, meld_world_model::error::StorageError> {
        TraversalQuery::new(self.store.as_ref()).cut(request)
    }

    fn traverse(
        &self,
        cut: &TraversalCut,
        request: &BoundedTraversalRequest,
    ) -> Result<TraversalResult, meld_world_model::error::StorageError> {
        TraversalQuery::new(self.store.as_ref()).traverse(cut, request)
    }
}

impl EventAppendSink for ProductEventAppendPort {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.append.ledger_identity()
    }

    fn append_envelope_idempotent(&self, envelope: EventEnvelope) -> Result<AppendReceipt, String> {
        self.append
            .append_durable(envelope, AppendMode::Idempotent)
            .map_err(|error| error.to_string())
    }
}

impl ProductEventReplayPort {
    /// Bind the port to one authority replay capability.
    pub fn new(replay: EventReplayCapability) -> Self {
        Self { replay }
    }

    /// Returns the ledger replayed by this port.
    pub fn ledger_identity(&self) -> LedgerIdentity {
        self.replay.ledger_identity()
    }

    /// Read ordered event records after the caller supplied sequence.
    pub fn read_after_limit(
        &self,
        after_seq: u64,
        limit: usize,
    ) -> Result<Vec<EventRecord>, RuntimePortError> {
        if !(1..=MAX_EVENT_REPLAY_LIMIT).contains(&limit) {
            return Err(RuntimePortError::InvalidRequest(format!(
                "event replay limit must be in 1..={MAX_EVENT_REPLAY_LIMIT}, got {limit}"
            )));
        }
        let page = self
            .replay
            .replay(ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id: self.replay.ledger_identity(),
                    after_seq,
                },
                limit,
            })
            .map_err(|error| RuntimePortError::EventReplay(error.to_string()))?;
        Ok(page.records)
    }
}

impl ProductGraphCursorPort {
    /// Bind the reporter to one authority consumer registry.
    pub fn new(registry: EventConsumerRegistryCapability) -> Self {
        Self { registry }
    }

    /// Returns the durable graph consumer position for diagnostics and tests.
    pub fn current(&self) -> Result<Option<meld_events::ConsumerCursorPosition>, RuntimePortError> {
        self.registry
            .get("world_state.graph.reducer")
            .map_err(|error| RuntimePortError::EventReplay(error.to_string()))
    }
}

impl GraphEventReplaySource for ProductEventReplayPort {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.replay.ledger_identity()
    }

    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        self.replay.replay(request)
    }
}

impl ExecutionProjectionReplaySource for ProductEventReplayPort {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.replay.ledger_identity()
    }

    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        self.replay.replay(request)
    }
}

impl GraphDerivedEventSink for ProductEventAppendPort {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.append.ledger_identity()
    }

    fn append_derived(
        &self,
        envelope: EventEnvelope,
    ) -> Result<AppendReceipt, EventAuthorityError> {
        self.append.append_durable(envelope, AppendMode::Idempotent)
    }
}

impl GraphConsumerCursorReporter for ProductGraphCursorPort {
    fn report_owner_source_cursor(
        &self,
        source: &meld_world_model::world_state::graph::admission::OwnerEventSourceRef,
        cursor: LedgerCursor,
    ) -> Result<(), EventAuthorityError> {
        self.registry
            .report(&source.consumer_id(), cursor)
            .map(|_| ())
    }

    fn ledger_identity(&self) -> LedgerIdentity {
        self.registry.ledger_identity()
    }

    fn report_graph_cursor(&self, cursor: LedgerCursor) -> Result<(), EventAuthorityError> {
        self.registry
            .report("world_state.graph.reducer", cursor)
            .map(|_| ())
    }
}

impl ProductAgentPlannerPort {
    pub fn new(
        belief_store: Arc<BeliefStore>,
        traversal_store: Arc<TraversalStore>,
        curation_store: Arc<CurationStore>,
        event_append: ProductEventAppendPort,
        request: PlannerCurrentAssemblyRequest,
    ) -> Self {
        Self {
            curation_store,
            belief_store,
            traversal_store,
            event_append,
            request,
        }
    }
}

impl AgentPlannerPort for ProductAgentPlannerPort {
    fn publication_visibility(
        &self,
        cut: &meld_world_model::world_state::graph::contracts::TraversalCut,
        expected: &meld_world_model::world_state::graph::contracts::OwnerPublicationExpectation,
    ) -> Result<
        Option<meld_world_model::world_state::graph::visibility::OwnerPublicationVisibilityProof>,
        meld_world_model::error::StorageError,
    > {
        meld_world_model::TraversalQuery::new(self.traversal_store.as_ref())
            .publication_visibility(cut, expected)
    }

    fn assemble(&self) -> PlannerAssemblyOutcome {
        self.assemble_request(self.request.clone())
    }

    fn assemble_for(
        &self,
        goal_id: &str,
        fence: &AgentAuthorizationFence,
    ) -> PlannerAssemblyOutcome {
        let mut request = self.request.clone();
        request.context.goal_id = goal_id.to_string();
        request.context.context_id = format!("agent-context::{goal_id}");
        request.context.activation_generation = fence.activation_generation.clone();
        request.context.admission_epoch = fence.admission_epoch.clone();
        self.assemble_request(request)
    }

    fn assemble_epoch(
        &self,
        products: &meld_world_model::agent::AgentEpochProducts,
        fence: &AgentAuthorizationFence,
    ) -> PlannerAssemblyOutcome {
        let validation = products.validate().and_then(|()| {
            let authority = &products.specification.authority;
            let context = &self.request.context;
            if (!products.specification.is_prepared_request()
                && products.specification.fence != *fence)
                || products.specification.fence.authority_policy_content_hash
                    != fence.authority_policy_content_hash
                || authority.agent_id != context.agent_id
                || authority.subject != context.subject
                || authority.perspective != self.request.belief_key.perspective
                || authority.branch_scope != self.request.belief_key.branch_scope
            {
                Err(meld_world_model::error::StorageError::InvalidPath(
                    "epoch specification belongs to another bound Planner participant".into(),
                ))
            } else {
                Ok(())
            }
        });
        if let Err(error) = validation {
            return PlannerAssemblyOutcome::Refused(meld_world_model::PlannerRefusal {
                request_context_id: self.request.context.context_id.clone(),
                grounds: vec![meld_world_model::PlannerRefusalGround::InvalidInput {
                    detail: error.to_string(),
                }],
            });
        }
        let specification = &products.specification;
        let rule = &products.curation_rule;
        let mut request = self.request.clone();
        request.context.goal_id = specification.goal_id.clone();
        request.context.context_id = format!("agent-context::{}", specification.goal_id);
        request.context.activation_generation = fence.activation_generation.clone();
        request.context.admission_epoch = fence.admission_epoch.clone();
        request.context.scope_id = rule.rule.scope.scope_id.clone();
        request.context.observation_subject = Some(products.observation_subject.clone());
        request.belief_key.subject = products.observation_subject.clone();
        request.traversal_request = rule.rule.traversal_request();
        request.traversal_cut_request.scope = rule.rule.scope.clone();
        request.traversal_cut_request.owners = vec![
            meld_world_model::world_state::graph::contracts::TraversalOwnerRequirement {
                owner_id: rule.rule.source_owner_id.clone(),
                scope: rule.rule.scope.clone(),
                required: true,
                event_source: rule.rule.source_event_route.clone(),
            },
            meld_world_model::world_state::graph::contracts::TraversalOwnerRequirement {
                owner_id: meld_world_model::CURATION_OWNER_ID.into(),
                scope: rule.rule.scope.clone(),
                required: false,
                event_source: None,
            },
        ];
        request.traversal_cut_request.owners.sort();
        for source in &mut request.source_positions {
            source.scope_id = request.context.scope_id.clone();
            match source.kind {
                meld_world_model::PlannerSourceKind::Directive => {
                    source.source_id = specification.specification_id.clone();
                    source.revision_id = specification.specification_id.clone();
                    source.content_hash = specification.specification_id.clone();
                }
                meld_world_model::PlannerSourceKind::CurationCatalog => {
                    source.source_id = rule.rule_id.clone();
                    source.revision_id = rule.content_hash.clone();
                    source.content_hash = rule.content_hash.clone();
                }
                _ => {}
            }
        }
        self.assemble_request(request)
    }
}

impl ProductAgentPlannerPort {
    fn assemble_request(
        &self,
        mut request: PlannerCurrentAssemblyRequest,
    ) -> PlannerAssemblyOutcome {
        match self.event_append.watermark() {
            Ok(watermark) => {
                request.traversal_cut_request.event_position = LedgerCursor {
                    ledger_id: watermark.ledger_id,
                    after_seq: watermark.committed_seq,
                }
            }
            Err(error) => {
                return PlannerAssemblyOutcome::Refused(meld_world_model::PlannerRefusal {
                    request_context_id: request.context.context_id,
                    grounds: vec![meld_world_model::PlannerRefusalGround::InvalidInput {
                        detail: error.to_string(),
                    }],
                })
            }
        }
        let query = PlannerQuery::new(
            BeliefQuery::new(self.belief_store.as_ref()),
            TraversalQuery::new(self.traversal_store.as_ref()),
        );
        query
            .with_curation(meld_world_model::CurationQuery::new(
                self.curation_store.as_ref(),
            ))
            .assemble_current(request)
    }
}

impl ProductPlannedCurationPort {
    pub fn new(store: Arc<CurationStore>) -> Self {
        Self { store }
    }
}

impl ProductAgentExecutionPort {
    /// Bind one exact live Agent fence to the shared Execution network.
    pub fn new(
        network: Arc<Mutex<SledTaskNetworkStore>>,
        catalog: meld_execution::capability::CapabilityCatalog,
        authority: Arc<dyn AgentAuthorityPort>,
        authority_policy_content_hash: String,
    ) -> Self {
        Self {
            network,
            catalog,
            authority,
            authority_policy_content_hash,
        }
    }

    fn position(
        &self,
        authorization: &AgentProductAuthorization,
        allow_intake: bool,
    ) -> Result<Option<AgentExecutionPosition>, meld_world_model::error::StorageError> {
        let AgentAuthorizedProduct::Task(task) = &authorization.product else {
            return Err(meld_world_model::error::StorageError::InvalidPath(
                "Execution port accepts only Agent-authorized Tasks".to_string(),
            ));
        };
        let request = TaskAdmissionRequest {
            lineage: TaskAdmissionLineage {
                request_ref: authorization.request_ref.clone(),
                agent_id: authorization.agent_id.clone(),
                goal_id: authorization.goal_id.clone(),
                plan_revision_id: authorization.plan_revision_id.clone(),
                product_id: authorization.product_id.clone(),
                authorization_id: authorization.authorization_id.clone(),
                context_id: authorization.context_id.clone(),
                authority_scope_id: authorization.authority_scope_id.clone(),
                authority_policy_content_hash: authorization.authority_policy_content_hash.clone(),
                authority_decision: authorization.authority_decision.clone(),
                activation_generation: authorization.activation_generation.clone(),
                admission_epoch: authorization.admission_epoch.clone(),
            },
            task: ExecutionTask {
                execution_subject: task.execution_subject.clone(),
                initial_inputs: task.initial_inputs.clone(),
                task_id: task.task_id.clone(),
                composition: task.composition.clone(),
                bindings: task.bindings.clone(),
                capability_contract_ids: task.capability_contract_ids.clone(),
                expected_outcome_contract_id: task.expected_outcome_contract_id.clone(),
                authority_requirements: task.authority_requirements.clone(),
                idempotency_key: task.idempotency_key.clone(),
            },
            idempotency_key: authorization.idempotency_key.clone(),
        };
        let admission_id = TaskAdmissionRecord::admission_id_for(&request);
        let snapshot = {
            let mut network = self.network.lock().map_err(|_| {
                meld_world_model::error::StorageError::InvalidPath(
                    "shared Task Network lock is poisoned".to_string(),
                )
            })?;
            let admission = match network.state().admissions.get(&admission_id) {
                Some(existing) if existing.request == request => existing.clone(),
                Some(_) => {
                    return Err(meld_world_model::error::StorageError::InvalidPath(
                        "durable admission identity contains a different Task offer".to_string(),
                    ))
                }
                None if !allow_intake => return Ok(None),
                None => {
                    let fence = self.authority.observe()?;
                    let generation = fence
                        .as_ref()
                        .map(|fence| fence.activation_generation.as_str())
                        .unwrap_or("");
                    let epoch = fence
                        .as_ref()
                        .and_then(|fence| fence.admission_epoch.as_deref());
                    TaskAdmissionApi::new(
                        &mut *network,
                        &self.catalog,
                        generation,
                        &self.authority_policy_content_hash,
                    )
                    .with_admission_epoch(epoch)
                    .admit(request)
                    .map_err(meld_world_model::error::StorageError::InvalidPath)?
                }
            };
            let network_commit_revision = network.journal().iter().find_map(|record| {
                let JournalRecord::Commit(commit) = record else {
                    return None;
                };
                commit
                    .mutation_set
                    .mutations
                    .iter()
                    .any(|mutation| match mutation {
                        meld_execution::task_network::Mutation::Inject(inject) => inject
                            .task_node
                            .lineage
                            .admission
                            .as_ref()
                            .is_some_and(|lineage| lineage.admission_id == admission.admission_id),
                    })
                    .then_some(commit.revision)
            });
            let account = admission_discharge_account(network.state(), &admission.admission_id);
            let outcome_id = account.as_ref().map(|account| account.outcome_id.clone());
            let execution_publication_position_id = outcome_id.as_ref().and_then(|outcome_id| {
                network
                    .state()
                    .publications
                    .values()
                    .find_map(|publication| {
                        let carries_return = account.as_ref().is_some_and(|account| {
                            if account.shared_action_decision_ids.is_empty() {
                                &publication.outcome.outcome_id == outcome_id
                            } else {
                                publication.shared_discharge_accounts.contains(account)
                            }
                        });
                        if !carries_return {
                            return None;
                        }
                        match &publication.state {
                            PublicationState::Published {
                                receipt: Some(receipt),
                                ..
                            } => Some(format!("{}::{}", receipt.ledger_id, receipt.seq)),
                            _ => None,
                        }
                    })
            });
            (
                admission,
                network_commit_revision,
                outcome_id,
                execution_publication_position_id,
            )
        };
        let (admission, network_commit_revision, outcome_id, execution_publication_position_id) =
            snapshot;
        let decision = match admission.decision {
            TaskAdmissionDecision::Admitted => AgentExecutionAdmissionDecision::Admitted,
            TaskAdmissionDecision::Rejected { grounds } => {
                AgentExecutionAdmissionDecision::Rejected { grounds }
            }
            TaskAdmissionDecision::StaleFence { .. } => AgentExecutionAdmissionDecision::StaleFence,
        };
        Ok(Some(AgentExecutionPosition {
            authorization_id: authorization.authorization_id.clone(),
            admission_id: admission.admission_id,
            admission_decision: decision,
            admission_revision: admission.recorded_revision,
            network_commit_revision,
            outcome_id,
            execution_publication_position_id,
        }))
    }
}

impl ProductAdmissionGenerationObserver {
    /// Observe only the exact prepared assignment through its canonical lifecycle authority.
    pub fn new(
        agent_store: Arc<AgentStore>,
        lifecycle: crate::runtime::lifecycle::ActivationLifecycleStore,
        prepared: &crate::theory::PreparedActivationClosureV1,
    ) -> Self {
        Self {
            agent_store,
            lifecycle,
            assignment_id: prepared.assignment.assignment_id.clone(),
            activation_id: prepared.activation.activation_id.clone(),
            prepared_id: prepared.prepared_id.clone(),
        }
    }

    fn observe_epoch(
        &self,
        agent_id: &str,
    ) -> Result<Option<crate::runtime::lifecycle::AdmissionEpochV1>, String> {
        let Some(agent) = self
            .agent_store
            .get_agent(agent_id)
            .map_err(|error| error.to_string())?
        else {
            return Ok(None);
        };
        if agent.status != AgentStatus::Operational {
            return Ok(None);
        }
        let receipts = self
            .agent_store
            .genesis_receipts_for_assignment(&self.assignment_id)
            .map_err(|error| error.to_string())?;
        if !receipts.iter().any(|receipt| receipt.agent_id == agent_id) {
            return Ok(None);
        }
        let Some(generation) = self
            .lifecycle
            .current_generation(&self.assignment_id)
            .map_err(|error| error.to_string())?
        else {
            return Ok(None);
        };
        if !generation.admission_open()
            || generation.activation_id != self.activation_id
            || generation.prepared_id != self.prepared_id
        {
            return Ok(None);
        }
        Ok(generation.current_admission_epoch().cloned())
    }
}

impl AdmissionGenerationObserver for ProductAdmissionGenerationObserver {
    fn active_generation(&self, agent_id: &str) -> Result<Option<String>, String> {
        Ok(self
            .observe_epoch(agent_id)?
            .map(|epoch| epoch.generation_id))
    }

    fn validates_admission(
        &self,
        attribution: &meld_execution::task_network::TaskAdmissionAttribution,
    ) -> Result<bool, String> {
        Ok(self
            .observe_epoch(&attribution.agent_id)?
            .is_some_and(|epoch| {
                epoch.generation_id == attribution.activation_generation
                    && Some(&epoch.epoch_id) == attribution.admission_epoch.as_ref()
            }))
    }
}

impl AgentExecutionPort for ProductAgentExecutionPort {
    fn submit(
        &self,
        authorization: &AgentProductAuthorization,
    ) -> Result<AgentExecutionPosition, meld_world_model::error::StorageError> {
        self.position(authorization, true)?.ok_or_else(|| {
            meld_world_model::error::StorageError::InvalidPath(
                "Task intake returned no decision".into(),
            )
        })
    }

    fn advance(
        &self,
        authorization: &AgentProductAuthorization,
    ) -> Result<AgentExecutionPosition, meld_world_model::error::StorageError> {
        self.position(authorization, true)?.ok_or_else(|| {
            meld_world_model::error::StorageError::InvalidPath(
                "Task intake returned no decision".into(),
            )
        })
    }
    fn observe(
        &self,
        authorization: &AgentProductAuthorization,
    ) -> Result<Option<AgentExecutionPosition>, meld_world_model::error::StorageError> {
        self.position(authorization, false)
    }
}

impl ProductAgentAuthorityPort {
    pub(crate) fn new(
        admission: ProductAdmissionGenerationObserver,
        agent_id: String,
        authority_policy_content_hash: String,
    ) -> Self {
        Self {
            admission,
            agent_id,
            authority_policy_content_hash,
        }
    }
}

impl AgentAuthorityPort for ProductAgentAuthorityPort {
    fn observe(
        &self,
    ) -> Result<Option<AgentAuthorizationFence>, meld_world_model::error::StorageError> {
        Ok(self
            .admission
            .observe_epoch(&self.agent_id)
            .map_err(meld_world_model::error::StorageError::InvalidPath)?
            .map(|epoch| AgentAuthorizationFence {
                activation_generation: epoch.generation_id,
                admission_epoch: Some(epoch.epoch_id),
                authority_policy_content_hash: self.authority_policy_content_hash.clone(),
            }))
    }
    fn same_preparation(
        &self,
        prior_generation: &str,
        current: &AgentAuthorizationFence,
    ) -> Result<bool, meld_world_model::error::StorageError> {
        if self.observe()?.as_ref() != Some(current) {
            return Ok(false);
        }
        let prior = self
            .admission
            .lifecycle
            .generation(&self.admission.assignment_id, prior_generation)
            .map_err(|error| {
                meld_world_model::error::StorageError::InvalidPath(error.to_string())
            })?;
        Ok(prior.is_some_and(|generation| {
            generation.assignment_id == self.admission.assignment_id
                && generation.activation_id == self.admission.activation_id
                && generation.prepared_id == self.admission.prepared_id
        }))
    }
}

impl AgentCurationPort for ProductPlannedCurationPort {
    fn resolve_operation(
        &self,
        candidate: CurationOperation,
    ) -> Result<CurationOperation, meld_world_model::error::StorageError> {
        self.store.resolve_operation(candidate)
    }

    fn submit(
        &self,
        operation: CurationOperation,
    ) -> Result<(), meld_world_model::error::StorageError> {
        self.store.submit_planned(&operation)
    }

    fn acceptance(
        &self,
        operation_id: &str,
    ) -> Result<Option<CurationAcceptanceRecord>, meld_world_model::error::StorageError> {
        self.store.acceptance_for_planned_operation(operation_id)
    }

    fn result(
        &self,
        operation_id: &str,
    ) -> Result<Option<CurationResult>, meld_world_model::error::StorageError> {
        self.store.result_for_operation(operation_id)
    }
}

impl ContextRuntimePort {
    /// Bind the port to context frame storage.
    pub fn new(storage: Arc<FrameStorage>) -> Self {
        Self { storage }
    }

    /// Return context frame storage for runtime factories.
    pub fn storage(&self) -> &Arc<FrameStorage> {
        &self.storage
    }
}

impl ProviderRuntimePort {
    /// Validate provider availability before supervisor handoff.
    pub fn new(mut config: ProviderPortConfig) -> Result<Self, RuntimeAssemblyError> {
        let env_available = !config.required_env_vars.is_empty()
            && config
                .required_env_vars
                .iter()
                .all(|name| !name.trim().is_empty() && std::env::var_os(name).is_some());
        if config.provider_required && !config.provider_available && !env_available {
            return Err(RuntimeAssemblyError::ProviderConstruction(
                "enabled runtimes require provider access but no provider is available".to_string(),
            ));
        }
        if env_available {
            config.provider_available = true;
        }
        Ok(Self { config })
    }

    /// Return whether provider access is available.
    pub fn is_available(&self) -> bool {
        self.config.provider_available
    }

    /// Return whether enabled runtime factories require provider access.
    pub fn is_required(&self) -> bool {
        self.config.provider_required
    }
}

impl PromptRuntimePort {
    /// Bind the port to prompt artifact storage.
    pub fn new(storage: Arc<PromptContextArtifactStorage>) -> Self {
        Self { storage }
    }

    /// Return prompt artifact storage for runtime factories.
    pub fn storage(&self) -> &Arc<PromptContextArtifactStorage> {
        &self.storage
    }
}

impl WorkspaceRuntimePort {
    /// Bind the port to workspace node storage.
    pub fn new(store: Arc<SledNodeRecordStore>) -> Self {
        Self { store }
    }

    /// Return workspace node storage for runtime factories.
    pub fn store(&self) -> &Arc<SledNodeRecordStore> {
        &self.store
    }
}

impl TaskArtifactFactoryPort {
    /// Bind the port to the execution-owned task artifact factory.
    pub fn new(factory: TaskArtifactRepoFactory) -> Self {
        Self { factory }
    }

    /// Return the execution-owned task artifact factory.
    pub fn factory(&self) -> &TaskArtifactRepoFactory {
        &self.factory
    }
}

impl TaskNetworkFactoryPort {
    /// Bind the port to the execution-owned task network store factory.
    pub fn new(factory: TaskNetworkStoreFactory) -> Self {
        Self { factory }
    }

    /// Return the execution-owned task network store factory.
    pub fn factory(&self) -> &TaskNetworkStoreFactory {
        &self.factory
    }
}

impl EvidenceEventReplaySource for ProductEventReplayPort {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.replay.ledger_identity()
    }

    fn replay(&self, request: ReplayRequest) -> Result<EventPage, EventAuthorityError> {
        self.replay.replay(request)
    }
}

pub(crate) struct ProductionDocsClaimJudge {
    pub api: Arc<crate::api::ContextApi>,
    pub config: crate::docs::capability::DocsCapabilityConfig,
}

#[async_trait]
impl crate::docs::claim_validation::DocsClaimJudge for ProductionDocsClaimJudge {
    async fn correspond(
        &self,
        request: &crate::docs::correspondence::DocsCorrespondenceRequest<'_>,
    ) -> Result<crate::docs::correspondence::ProposedCorrespondence, crate::error::ApiError> {
        crate::docs::claim_validation::ProviderDocsClaimJudge {
            api: self.api.as_ref(),
            config: &self.config,
            event_context: None,
        }
        .correspond(request)
        .await
    }

    async fn extract_source(
        &self,
        request: &crate::docs::source_claims::DocsSourceClaimRequest<'_>,
    ) -> Result<crate::docs::source_claims::ProposedSourceClaims, crate::error::ApiError> {
        crate::docs::claim_validation::ProviderDocsClaimJudge {
            api: self.api.as_ref(),
            config: &self.config,
            event_context: None,
        }
        .extract_source(request)
        .await
    }

    async fn assess(
        &self,
        request: &crate::docs::claim_validation::DocsClaimJudgmentRequest<'_>,
    ) -> Result<Vec<crate::docs::claim_validation::ProviderClaimAssessment>, crate::error::ApiError>
    {
        crate::docs::claim_validation::ProviderDocsClaimJudge {
            api: self.api.as_ref(),
            config: &self.config,
            event_context: None,
        }
        .assess(request)
        .await
    }
}

/// Shared composition core for production Task Network claim dispatch.
///
/// Owner: root runtime composition. The port executes admitted compiled Tasks
/// through the production Capability catalog and invoker registry.
pub struct ProductionDispatchRouteContext {
    /// Root api facade the workflow task path executes through.
    pub api: Arc<crate::api::ContextApi>,
    /// Ledger session partition recorded on prepared runs.
    pub session_id: Option<String>,
    /// Production Capability catalog used by admitted Tasks.
    pub catalog: crate::capability::CapabilityCatalog,
    /// Production Capability executor registry matching the catalog.
    pub registry: crate::capability::CapabilityExecutorRegistry,
}

impl ProductionDispatchRouteContext {
    /// Build the shared production claim invoker over one core.
    pub fn into_claim_port(self) -> SharedClaimedTaskInvoker {
        let core = Arc::new(self);
        SharedClaimedTaskInvoker(Arc::new(CompiledTaskClaimInvoker { core }))
    }
}

/// Production claimed-task invoker over the real task executor.
///
/// One claimed task node executes its own compiled capability graph to
/// completion through the workflow task-path capability set. The invoker
/// returns the emitted artifact records without persisting them: the
/// dispatch actor owns artifact persistence order and outcome recording.
struct CompiledTaskClaimInvoker {
    core: Arc<ProductionDispatchRouteContext>,
}

#[async_trait]
impl ClaimedTaskInvoker for CompiledTaskClaimInvoker {
    async fn recover_claimed_task(
        &self,
        node: &TaskNode,
        claim: &Claim,
        init_payload: &TaskInitializationPayload,
    ) -> Result<Option<ClaimedInvocationOutcome>, DispatchPortError> {
        let mut executor = crate::task::TaskExecutor::new(
            node.compiled_task.clone(),
            init_payload.clone(),
            format!("dispatch_claim::{}", claim.claim_id),
        )
        .map_err(|error| DispatchPortError::fatal(error.to_string()))?;
        let context = self.event_context(node);
        crate::task::runtime::recover_task_to_completion(
            self.core.api.as_ref(),
            &mut executor,
            &self.core.catalog,
            &self.core.registry,
            context.as_ref(),
        )
        .await
        .map_err(|error| DispatchPortError::retryable(error.to_string()))?;
        Ok(Some(ClaimedInvocationOutcome::Completed(
            meld_execution::task_network::dispatch::emitted_artifact_records(&executor),
        )))
    }

    async fn invoke_claimed_task(
        &self,
        node: &TaskNode,
        claim: &Claim,
        init_payload: &TaskInitializationPayload,
    ) -> Result<ClaimedInvocationOutcome, DispatchPortError> {
        let mut executor = crate::task::TaskExecutor::new(
            node.compiled_task.clone(),
            init_payload.clone(),
            format!("dispatch_claim::{}", claim.claim_id),
        )
        .map_err(|error| DispatchPortError::fatal(error.to_string()))?;
        // Per-task lifecycle events reach the ledger only through an event
        // context; the route core carries the session partition the
        // composed actors share, so the claimed route publishes under it.
        let event_context = self.event_context(node);
        match crate::task::execute_task_to_completion(
            self.core.api.as_ref(),
            &mut executor,
            &self.core.catalog,
            &self.core.registry,
            event_context.as_ref(),
            None,
        )
        .await
        {
            Ok(_summary) => Ok(ClaimedInvocationOutcome::Completed(
                meld_execution::task_network::dispatch::emitted_artifact_records(&executor),
            )),
            Err(error) => {
                let message = error.to_string();
                // A gate violation that survived its declared retry budget
                // is deterministic over the recorded artifacts, and an
                // all-fresh expansion has no work by construction: both are
                // terminal, recorded through the command boundary so belief
                // learns them, never refenced into unbounded retries.
                if is_terminal_claimed_failure(&message) {
                    Ok(ClaimedInvocationOutcome::Failed { error: message })
                } else {
                    // Any other unresolved invocation keeps the claim
                    // fenced: a later tick resumes it instead of recording
                    // a premature terminal outcome.
                    Err(DispatchPortError::retryable(message))
                }
            }
        }
    }
}

impl CompiledTaskClaimInvoker {
    fn event_context(&self, node: &TaskNode) -> Option<crate::execution::ExecutionEventContext> {
        self.core
            .session_id
            .as_ref()
            .map(|session_id| crate::execution::ExecutionEventContext {
                effect_authority: node
                    .lineage
                    .admission
                    .as_ref()
                    .zip(node.lineage.authority_decision.as_ref())
                    .and_then(|(admission, decision)| {
                        admission.admission_epoch.as_ref().map(|epoch| {
                            meld_execution::ExecutionEffectAuthority {
                                request_ref: admission.request_ref.clone(),
                                issuer_ref: admission.agent_id.clone(),
                                principal_id: decision.principal_id.clone(),
                                subject: decision.subject.clone(),
                                fence_ref: epoch.clone(),
                            }
                        })
                    }),
                session_id: session_id.clone(),
            })
    }
}

fn is_terminal_claimed_failure(message: &str) -> bool {
    message.contains(meld_execution::error::TERMINAL_CAPABILITY_FAILURE_MARKER)
        || message.contains(crate::context::capability::GATE_FAILURE_MARKER)
        || message.contains(crate::merkle_traversal::expansion::NOTHING_TO_REGENERATE_MARKER)
        || message.contains(crate::workspace::capability::MISSING_HEAD_MARKER)
}

/// Shared handle adapter for an injected claimed-task invoker.
#[derive(Clone)]
pub struct SharedClaimedTaskInvoker(pub Arc<dyn ClaimedTaskInvoker>);

#[async_trait]
impl ClaimedTaskInvoker for SharedClaimedTaskInvoker {
    async fn recover_claimed_task(
        &self,
        node: &TaskNode,
        claim: &Claim,
        init_payload: &TaskInitializationPayload,
    ) -> Result<Option<ClaimedInvocationOutcome>, DispatchPortError> {
        self.0.recover_claimed_task(node, claim, init_payload).await
    }

    async fn invoke_claimed_task(
        &self,
        node: &TaskNode,
        claim: &Claim,
        init_payload: &TaskInitializationPayload,
    ) -> Result<ClaimedInvocationOutcome, DispatchPortError> {
        self.0.invoke_claimed_task(node, claim, init_payload).await
    }
}
