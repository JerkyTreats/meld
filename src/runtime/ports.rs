//! Thin direct handoff ports built by product runtime assembly.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use meld_events::error::EventAuthorityError;
use meld_events::{
    AppendMode, AppendReceipt, DomainObjectRef, EventAppendCapability, EventAuthority,
    EventConsumerRegistryCapability, EventEnvelope, EventObservabilityCapability, EventPage,
    EventRecord, EventReplayCapability, EventWatermark, EventWatermarkCapability, LedgerCursor,
    LedgerIdentity, ReplayRequest,
};
use meld_execution::capability::{
    BoundCapabilityInstance, CapabilityInvocationPayload, CapabilityInvocationResult,
};
use meld_execution::goals::{
    GoalAcceptanceLifecycle, GoalAcceptanceRequest, GoalCommandMetadata, GoalCommandOutcome,
    GoalSetApi, PersistentGoalSetStore,
};
use meld_execution::planning::realization::TaskPackageRoutePlan;
use meld_execution::planning::{
    PlanningProjectionError as ExecutionPlanningProjectionError, PlanningProjectionPort,
    PlanningWorldStateFrameRef, PlanningWorldStateProjection, PlanningWorldStateRequest,
};
use meld_execution::task::expansion::{CompiledTaskDelta, TaskExpansionRequest};
use meld_execution::task::{
    CompiledTaskRecord, PackageStepInvoker, TaskArtifactRepoFactory, TaskInitializationPayload,
};
use meld_execution::task_network::dispatch::Claim;
use meld_execution::task_network::dispatch_actor::{
    ClaimedInvocationOutcome, ClaimedTaskInvoker, DispatchPortError, PackageRunPreparer,
    PreparedPackageRun,
};
use meld_execution::task_network::state::TaskNode;
use meld_execution::task_network::store::TaskNetworkStoreFactory;
use meld_execution::task_network::EventAppendSink;
use meld_world_model::agent::{
    ActiveGoalSummary, AgentActiveGoalQuery, AgentActiveGoalQueryError, AgentGoalCommandSink,
    AgentGoalMutationSink, AgentSinkError, AgentSinkSubmission,
};
use meld_world_model::belief::{
    configured_belief_key, BeliefFamilyRegistry, BeliefFamilyRegistryStore,
    EvidenceEventReplaySource,
};
use meld_world_model::planner::{
    PlannerProjectionError, PlannerProjectionOutput, PlannerQuery, PlannerSourceRef,
};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::world_state::graph::{
    GraphConsumerCursorReporter, GraphDerivedEventSink, GraphEventReplaySource, PerspectiveKey,
};
use meld_world_model::{AgentGoalCommand, AgentGoalMutationCommand, BeliefQuery, BeliefStore};
use meld_world_model::{BranchScope, TraversalQuery};

use crate::context::frame::FrameStorage;
use crate::control::projection::ExecutionProjectionReplaySource;
use crate::execution::goal_mutation::{
    execution_mutation_from_agent_command, ExecutionGoalMutation, GoalMutationRequest,
};
use crate::prompt_context::PromptContextArtifactStorage;
use crate::provider::ProviderExecutionBinding;
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
    goal_command: ScopedResource<ExecutionGoalCommandPort>,
    goal_mutation: ScopedResource<ExecutionGoalMutationPort>,
    planner_projection: ScopedResource<PlannerProjectionPort>,
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

/// Execution goal command sink backed by the execution goal API.
///
/// The port validates and maps producer commands into execution-owned goal
/// acceptance. It does not decide whether a goal is worthwhile or cache goal
/// ids as root progress.
#[derive(Clone)]
pub struct ExecutionGoalCommandPort {
    store: Arc<PersistentGoalSetStore>,
}

/// Execution goal mutation sink backed by the execution goal API.
///
/// The port maps already-persisted world model satisfaction decisions into
/// execution lifecycle mutations. It does not run satisfaction curation.
#[derive(Clone)]
pub struct ExecutionGoalMutationPort {
    store: Arc<PersistentGoalSetStore>,
}

/// Planner projection query port backed by world model stores.
#[derive(Clone)]
pub struct PlannerProjectionPort {
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
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

        let world_model = stores
            .belief_store
            .opened()
            .zip(stores.traversal_store.opened());
        let planner_projection = match world_model {
            Some((belief, traversal)) => ScopedResource::open(
                "planner_projection_port",
                PlannerProjectionPort::new(Arc::clone(belief), Arc::clone(traversal)),
            ),
            None => ScopedResource::closed("planner_projection_port"),
        };
        let (goal_command, goal_mutation) = match stores.goal_store.opened() {
            Some(goal_store) => (
                ScopedResource::open(
                    "goal_command_port",
                    ExecutionGoalCommandPort::new(Arc::clone(goal_store)),
                ),
                ScopedResource::open(
                    "goal_mutation_port",
                    ExecutionGoalMutationPort::new(Arc::clone(goal_store)),
                ),
            ),
            None => (
                ScopedResource::closed("goal_command_port"),
                ScopedResource::closed("goal_mutation_port"),
            ),
        };

        Ok(Self {
            event_append,
            event_replay: event_replay.clone(),
            graph_cursor: ProductGraphCursorPort::new(authority.consumer_registry_capability()),
            goal_command,
            goal_mutation,
            planner_projection,
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

    /// Return the goal command sink port.
    pub fn goal_command(&self) -> &ExecutionGoalCommandPort {
        &self.goal_command
    }

    /// Return the goal command sink port when its store is in scope.
    pub fn try_goal_command(&self) -> Option<&ExecutionGoalCommandPort> {
        self.goal_command.opened()
    }

    /// Return the goal mutation sink port.
    pub fn goal_mutation(&self) -> &ExecutionGoalMutationPort {
        &self.goal_mutation
    }

    /// Return the goal mutation sink port when its store is in scope.
    pub fn try_goal_mutation(&self) -> Option<&ExecutionGoalMutationPort> {
        self.goal_mutation.opened()
    }

    /// Return the planner projection query port.
    pub fn planner_projection(&self) -> &PlannerProjectionPort {
        &self.planner_projection
    }

    /// Return the planner projection port when its stores are in scope.
    pub fn try_planner_projection(&self) -> Option<&PlannerProjectionPort> {
        self.planner_projection.opened()
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
    fn ledger_identity(&self) -> LedgerIdentity {
        self.registry.ledger_identity()
    }

    fn report_graph_cursor(&self, cursor: LedgerCursor) -> Result<(), EventAuthorityError> {
        self.registry
            .report("world_state.graph.reducer", cursor)
            .map(|_| ())
    }
}

impl ExecutionGoalCommandPort {
    /// Bind the port to an opened execution goal store.
    pub fn new(store: Arc<PersistentGoalSetStore>) -> Self {
        Self { store }
    }

    /// Accept a producer neutral goal request through execution-owned APIs.
    pub fn accept_goal(
        &self,
        request: GoalAcceptanceRequest,
    ) -> Result<GoalCommandOutcome, RuntimePortError> {
        let mut store = self.store.as_ref().clone();
        GoalSetApi::new(&mut store)
            .accept_goal(request)
            .map_err(|error| RuntimePortError::Storage(error.to_string()))
    }

    /// Validate and accept one world-model authored goal command.
    pub fn accept_agent_goal_command(
        &self,
        command: AgentGoalCommand,
        seq: u64,
    ) -> Result<GoalCommandOutcome, RuntimePortError> {
        command
            .validate()
            .map_err(|error| RuntimePortError::InvalidRequest(error.to_string()))?;
        let request = GoalAcceptanceRequest {
            metadata: GoalCommandMetadata {
                command_id: command.command_id,
                source_identity: Some(command.dedupe_key.index_key()),
                seq,
            },
            goal: command.goal,
            lifecycle_policy: GoalAcceptanceLifecycle::RequireProposedThenActivate,
            strategy_authorization: command.strategy_authorization.map(|authorization| {
                let method_id = match authorization.candidate.origin {
                    meld_world_model::StrategyCandidateOrigin::Direct => None,
                    meld_world_model::StrategyCandidateOrigin::Method { method_id } => {
                        Some(method_id)
                    }
                };
                meld_execution::goals::ExecutionStrategyAuthorization {
                    authorization_id: authorization.authorization_id,
                    agent_decision_id: authorization.agent_decision_id,
                    candidate_id: authorization.candidate.candidate_id,
                    goal_id: authorization.candidate.goal_id,
                    planner_snapshot_id: authorization.candidate.planner_snapshot_id,
                    composition: authorization.candidate.composition,
                    bindings: authorization.candidate.bindings,
                    capability_contract_ids: authorization.candidate.capability_contract_ids,
                    method_id,
                }
            }),
        };
        self.accept_goal(request)
    }
}

impl ExecutionGoalMutationPort {
    /// Bind the port to an opened execution goal store.
    pub fn new(store: Arc<PersistentGoalSetStore>) -> Self {
        Self { store }
    }

    /// Apply a world-model goal mutation through execution-owned APIs.
    ///
    /// Routes both satisfaction and reopen mutations; the historical name is
    /// kept for existing callers.
    pub fn satisfy_agent_goal_mutation(
        &self,
        command: AgentGoalMutationCommand,
    ) -> Result<GoalCommandOutcome, RuntimePortError> {
        let mutation = execution_mutation_from_agent_command(GoalMutationRequest { command })
            .map_err(|error| RuntimePortError::InvalidRequest(error.to_string()))?;
        let mut store = self.store.as_ref().clone();
        let mut api = GoalSetApi::new(&mut store);
        match mutation {
            ExecutionGoalMutation::Satisfy(command) => api
                .satisfy_goal(command)
                .map_err(|error| RuntimePortError::Storage(error.to_string())),
            ExecutionGoalMutation::Reopen(command) => api
                .reopen_goal(command)
                .map_err(|error| RuntimePortError::Storage(error.to_string())),
        }
    }
}

impl PlannerProjectionPort {
    /// Bind the port to opened world model graph and belief stores.
    pub fn new(belief_store: Arc<BeliefStore>, traversal_store: Arc<TraversalStore>) -> Self {
        Self {
            belief_store,
            traversal_store,
        }
    }

    /// Project current planner state for one subject and dimension.
    pub fn project_current_world_state(
        &self,
        subject: &DomainObjectRef,
        dimension_id: &str,
        perspective: Option<PerspectiveKey>,
        branch_scope: Option<BranchScope>,
    ) -> Result<PlannerProjectionOutput, RuntimePortError> {
        let query = PlannerQuery::new(
            BeliefQuery::new(self.belief_store.as_ref()),
            TraversalQuery::new(self.traversal_store.as_ref()),
        );
        query
            .project_current_world_state(subject, dimension_id, perspective, branch_scope)
            .map_err(map_projection_error)
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

/// Execution-owned active-goal view adapted for world-model curation.
///
/// Owner: root translation only. The adapter copies every goal record the
/// execution store holds for one agent — including proposed and satisfied
/// records so dedupe and the reopen path see them — together with the
/// authoritative lifecycle epoch by goal id, so satisfaction and reopen
/// commands bind the epoch they observed.
#[derive(Clone)]
pub struct ExecutionAgentGoalQueryPort {
    store: Arc<PersistentGoalSetStore>,
}

impl ExecutionAgentGoalQueryPort {
    /// Bind the port to an opened execution goal store.
    pub fn new(store: Arc<PersistentGoalSetStore>) -> Self {
        Self { store }
    }
}

impl AgentActiveGoalQuery for ExecutionAgentGoalQueryPort {
    fn active_goals_for_agent(
        &mut self,
        agent_id: &str,
    ) -> Result<ActiveGoalSummary, AgentActiveGoalQueryError> {
        let records = self
            .store
            .goal_records()
            .map_err(|error| AgentActiveGoalQueryError::retryable(error.to_string()))?;
        let mut summary = ActiveGoalSummary::default();
        for record in records
            .into_iter()
            .filter(|record| record.goal.agent_id == agent_id)
        {
            summary
                .lifecycle_epochs
                .insert(record.goal.goal_id.clone(), record.lifecycle_epoch);
            summary.goals.push(record.goal);
        }
        Ok(summary)
    }
}

/// The named curation-to-goal-set port bound to execution goal storage.
///
/// This is the composition-time binding of
/// [`meld_world_model::agent::CurationGoalSetPort`] (satisfied through the
/// blanket impl over both sinks): curated goal commands route through the
/// execution goal command port and both mutation kinds — satisfy and
/// reopen — route through the execution goal mutation port. The sequence is
/// the injected step sequence of the invoking bounded actor tick and is
/// recorded as the accepted command's ordering sequence.
pub struct CurationGoalExecutionPort {
    command: ExecutionGoalCommandPort,
    mutation: ExecutionGoalMutationPort,
    sequence: u64,
}

impl CurationGoalExecutionPort {
    /// Bind the port for one bounded step at the injected sequence.
    pub fn new(
        command: ExecutionGoalCommandPort,
        mutation: ExecutionGoalMutationPort,
        sequence: u64,
    ) -> Self {
        Self {
            command,
            mutation,
            sequence,
        }
    }
}

impl AgentGoalCommandSink for CurationGoalExecutionPort {
    fn submit_goal_command(
        &mut self,
        command: &AgentGoalCommand,
    ) -> Result<AgentSinkSubmission, AgentSinkError> {
        let command_id = command.command_id.clone();
        let outcome = self
            .command
            .accept_agent_goal_command(command.clone(), self.sequence)
            .map_err(|error| AgentSinkError::retryable(error.to_string()))?;
        goal_outcome_submission(command_id, outcome)
    }
}

impl AgentGoalMutationSink for CurationGoalExecutionPort {
    fn submit_goal_mutation(
        &mut self,
        command: &AgentGoalMutationCommand,
    ) -> Result<AgentSinkSubmission, AgentSinkError> {
        let command_id = command.command_id.clone();
        let outcome = self
            .mutation
            .satisfy_agent_goal_mutation(command.clone())
            .map_err(|error| AgentSinkError::retryable(error.to_string()))?;
        goal_outcome_submission(command_id, outcome)
    }
}

/// Translate one goal command outcome into the sink submission vocabulary.
///
/// A stale no-op is absorbed as `duplicate`: the record was left
/// byte-identical by design (epoch fence), so the curation receipt must not
/// read as a fresh application.
fn goal_outcome_submission(
    command_id: String,
    outcome: GoalCommandOutcome,
) -> Result<AgentSinkSubmission, AgentSinkError> {
    match outcome {
        GoalCommandOutcome::Applied(record) => Ok(AgentSinkSubmission::new(
            command_id,
            record.goal.goal_id,
            "applied",
        )),
        GoalCommandOutcome::Duplicate { existing_goal_id } => Ok(AgentSinkSubmission::new(
            command_id,
            existing_goal_id,
            "duplicate",
        )),
        GoalCommandOutcome::StaleNoOp { goal_id, .. } => {
            Ok(AgentSinkSubmission::new(command_id, goal_id, "duplicate"))
        }
        GoalCommandOutcome::NotFound { goal_id } => Err(AgentSinkError::fatal(format!(
            "goal '{goal_id}' does not exist in the execution goal set"
        ))),
    }
}

/// Exact-key planner projection port for the execution planning actor.
///
/// Owner: root translation. The port resolves the configured belief
/// family's current theory revision per projection, derives the exact
/// configured belief key, and reads through the world model's exact-key
/// path, so the projected revision and theory lineage are the identities
/// the belief store holds for that key — never a same-subject neighbor.
pub struct ExactKeyPlanningProjectionPort {
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
    registry: Arc<BeliefFamilyRegistryStore>,
    family_id: String,
    subject: DomainObjectRef,
    perspective: PerspectiveKey,
    branch_scope: BranchScope,
}

impl ExactKeyPlanningProjectionPort {
    /// Bind the port to world-model stores and the configured scope.
    pub fn new(
        belief_store: Arc<BeliefStore>,
        traversal_store: Arc<TraversalStore>,
        registry: Arc<BeliefFamilyRegistryStore>,
        family_id: impl Into<String>,
        subject: DomainObjectRef,
        perspective: PerspectiveKey,
        branch_scope: BranchScope,
    ) -> Self {
        Self {
            belief_store,
            traversal_store,
            registry,
            family_id: family_id.into(),
            subject,
            perspective,
            branch_scope,
        }
    }
}

impl PlanningProjectionPort for ExactKeyPlanningProjectionPort {
    fn project(
        &mut self,
        request: PlanningWorldStateRequest,
    ) -> Result<PlanningWorldStateProjection, ExecutionPlanningProjectionError> {
        // Theory resolves per projection, mirroring the belief actors, so
        // planning always consumes the currently installed revision and a
        // not-yet-installed family stays retryable rather than fatal.
        let revision = self
            .registry
            .current(&self.family_id)
            .map_err(|error| ExecutionPlanningProjectionError::fatal(error.to_string()))?
            .ok_or_else(|| {
                ExecutionPlanningProjectionError::retryable(format!(
                    "belief family '{}' has no installed registry revision",
                    self.family_id
                ))
            })?;
        let key = configured_belief_key(
            &revision,
            &self.subject,
            &self.perspective,
            &self.branch_scope,
        );
        let query = PlannerQuery::new(
            BeliefQuery::new(self.belief_store.as_ref()),
            TraversalQuery::new(self.traversal_store.as_ref()),
        );
        // The family's anchor declaration decides scope semantics on the
        // planner side exactly as it does on the assessment side: an
        // unanchored family's maintained scope is declared accessible.
        let output = if revision.config.anchor_requirement
            == meld_world_model::belief::AnchorRequirement::Unanchored
        {
            query.project_world_state_for_unanchored_key(&key)
        } else {
            query.project_world_state_for_key(&key)
        }
        .map_err(|error| ExecutionPlanningProjectionError::retryable(error.to_string()))?;

        let source_refs: Vec<String> = output.source_refs.iter().map(render_source_ref).collect();
        // Frame identity is deterministic from the consumed belief revision
        // so identical projections carry identical causality and a revised
        // belief produces a causally distinct frame.
        let frame_id = output
            .source_refs
            .iter()
            .find_map(|source| match source {
                PlannerSourceRef::BeliefRevision { revision_id } => {
                    Some(format!("belief-revision::{revision_id}"))
                }
                _ => None,
            })
            .unwrap_or_else(|| format!("unassessed::{}", key.index_key()));
        Ok(PlanningWorldStateProjection {
            world_state: output.world_state,
            frame: PlanningWorldStateFrameRef {
                frame_id,
                projection_version: output.projection_version,
                perspective_id: request.perspective_id,
                branch_id: request.branch_id,
                source_refs,
                warnings: output
                    .warnings
                    .iter()
                    .map(|warning| format!("{warning:?}"))
                    .collect(),
            },
        })
    }
}

fn render_source_ref(source: &PlannerSourceRef) -> String {
    match source {
        PlannerSourceRef::BeliefRevision { revision_id } => {
            format!("belief_revision::{revision_id}")
        }
        PlannerSourceRef::Evidence { evidence_id } => format!("evidence::{evidence_id}"),
        PlannerSourceRef::SourceFact { source_fact_id } => {
            format!("source_fact::{source_fact_id}")
        }
        PlannerSourceRef::GraphAnchor { anchor_id } => format!("graph_anchor::{anchor_id:?}"),
        PlannerSourceRef::ProjectionRule { rule_id } => format!("projection_rule::{rule_id}"),
    }
}

/// Shared composition core for the production dispatch route ports.
///
/// Owner: root runtime composition. The three production ports execute plan
/// handoffs over the same machinery the registered workflow route uses: the
/// root api facade, the registered workflow package surface, the workflow
/// task-path capability set, and the provider registry the api carries. The
/// ports never invent execution semantics — they delegate to the public
/// preparation, capability, and task execution surfaces.
pub struct ProductionDispatchRouteContext {
    /// Root api facade the workflow task path executes through.
    pub api: Arc<crate::api::ContextApi>,
    /// Registered workflow profiles resolving each plan's workflow id.
    pub workflow_registry: Arc<parking_lot::RwLock<crate::workflow::WorkflowRegistry>>,
    /// Canonical workspace root of the stewarded subject.
    pub workspace_root: PathBuf,
    /// Workspace-relative subject path the package route triggers on.
    pub subject_path: PathBuf,
    /// Durable agent identity driving the workflow.
    pub agent_id: String,
    /// Validated provider binding from the stewardship selection.
    pub provider: ProviderExecutionBinding,
    /// Frame type the docs route publishes under.
    pub frame_type: String,
    /// Ledger session partition recorded on prepared runs.
    pub session_id: Option<String>,
    /// Production capability catalog: the workflow task-path set.
    pub catalog: crate::capability::CapabilityCatalog,
    /// Production capability executor registry matching the catalog.
    pub registry: crate::capability::CapabilityExecutorRegistry,
}

impl ProductionDispatchRouteContext {
    /// Build the three shared production route ports over one core.
    pub fn into_route_ports(
        self,
    ) -> (
        SharedPackageRunPreparer,
        SharedPackageStepInvoker,
        SharedClaimedTaskInvoker,
    ) {
        let core = Arc::new(self);
        (
            SharedPackageRunPreparer(Arc::new(WorkflowPackageRunPreparer {
                core: Arc::clone(&core),
            })),
            SharedPackageStepInvoker(Arc::new(RootCapabilityStepInvoker {
                core: Arc::clone(&core),
            })),
            SharedClaimedTaskInvoker(Arc::new(CompiledTaskClaimInvoker { core })),
        )
    }
}

/// Production package-run preparer over the registered workflow surface.
///
/// One plan handoff resolves through `prepare_registered_workflow_task_run`
/// — the same preparation the registered workflow route runs — so the
/// compiled task, seeds, and prompts are the production artifacts, not a
/// parallel formula. The prepared run context carries the actor-derived
/// task run id so durable dedupe and progress stay keyed by plan identity.
struct WorkflowPackageRunPreparer {
    core: Arc<ProductionDispatchRouteContext>,
}

impl PackageRunPreparer for WorkflowPackageRunPreparer {
    fn prepare_package_run(
        &self,
        plan: &TaskPackageRoutePlan,
        task_run_id: &str,
    ) -> Result<PreparedPackageRun, DispatchPortError> {
        // A plan naming an unregistered workflow cannot succeed without
        // operator action: fatal, recorded through the command boundary.
        let registered_profile = self
            .core
            .workflow_registry
            .read()
            .get(&plan.workflow_id)
            .cloned()
            .ok_or_else(|| {
                DispatchPortError::fatal(format!(
                    "plan '{}' names unregistered workflow '{}'",
                    plan.plan_id, plan.workflow_id
                ))
            })?;
        let request = crate::task::WorkflowPackageTriggerRequest {
            package_id: plan.package_id.clone(),
            workflow_id: plan.workflow_id.clone(),
            node_id: None,
            path: Some(self.core.subject_path.clone()),
            agent_id: self.core.agent_id.clone(),
            provider: self.core.provider.clone(),
            frame_type: self.core.frame_type.clone(),
            // Incremental by identity: node ids are content-addressed, so a
            // node holding a current frame is fresh by construction and only
            // changed subtrees expand into work. Forcing here would
            // regenerate the whole scope on every flywheel turn.
            force: false,
            session_id: self.core.session_id.clone(),
        };
        let prepared = crate::task::prepare_registered_workflow_task_run(
            self.core.api.as_ref(),
            &self.core.workspace_root,
            &registered_profile,
            &request,
            &self.core.catalog,
        )
        // Preparation reads mutable workspace state (scanned nodes, belief
        // context); a later tick may succeed once that state exists.
        .map_err(|error| DispatchPortError::retryable(error.to_string()))?;
        let mut init_payload = prepared.init_payload;
        // The actor rejects a drifted run id before opening durable
        // progress, so the prepared context binds the actor-derived id.
        init_payload.task_run_context.task_run_id = task_run_id.to_string();
        Ok(PreparedPackageRun {
            compiled_task: prepared.compiled_task,
            init_payload,
        })
    }
}

/// Production package-step invoker over the root capability registry.
///
/// Released invocations execute through the registered capability invokers
/// (the workflow task-path set) against the root api facade; expansion
/// requests compile through the public expansion compiler registry.
struct RootCapabilityStepInvoker {
    core: Arc<ProductionDispatchRouteContext>,
}

#[async_trait]
impl PackageStepInvoker for RootCapabilityStepInvoker {
    async fn invoke_capability(
        &self,
        instance: &BoundCapabilityInstance,
        payload: &CapabilityInvocationPayload,
    ) -> Result<CapabilityInvocationResult, meld_execution::error::ApiError> {
        let runtime_init = self.core.registry.runtime_init_for(instance)?;
        let invoker = self
            .core
            .registry
            .get(&instance.capability_type_id, instance.capability_version)
            .cloned()
            .ok_or_else(|| {
                meld_execution::error::ExecutionInvariantError::ConfigError(format!(
                    "dispatch route is missing invoker for '{}' version '{}'",
                    instance.capability_type_id, instance.capability_version
                ))
            })?;
        // Same event-context rule as the claimed route: capability
        // invocations on the package route publish their task lifecycle
        // events under the composed session partition.
        let event_context = self.core.session_id.as_ref().map(|session_id| {
            crate::execution::ExecutionEventContext {
                session_id: session_id.clone(),
            }
        });
        invoker
            .invoke(
                self.core.api.as_ref(),
                &runtime_init,
                payload,
                event_context.as_ref(),
            )
            .await
            .map_err(|error| {
                meld_execution::error::ExecutionInvariantError::GenerationFailed(error.to_string())
            })
    }

    fn compile_expansion(
        &self,
        compiled_task: &CompiledTaskRecord,
        request: &TaskExpansionRequest,
    ) -> Result<CompiledTaskDelta, meld_execution::error::ApiError> {
        crate::task::expansion::compile_task_expansion_request(
            self.core.api.as_ref(),
            compiled_task,
            request,
            &self.core.catalog,
        )
        .map_err(|error| {
            meld_execution::error::ExecutionInvariantError::ConfigError(error.to_string())
        })
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
        let event_context = self.core.session_id.as_ref().map(|session_id| {
            crate::execution::ExecutionEventContext {
                session_id: session_id.clone(),
            }
        });
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
                executor.artifact_repo().record().artifacts.clone(),
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

fn is_terminal_claimed_failure(message: &str) -> bool {
    message.contains(meld_execution::error::TERMINAL_CAPABILITY_FAILURE_MARKER)
        || message.contains(crate::context::capability::GATE_FAILURE_MARKER)
        || message.contains(crate::merkle_traversal::expansion::NOTHING_TO_REGENERATE_MARKER)
        || message.contains(crate::workspace::capability::MISSING_HEAD_MARKER)
}

/// Shared handle adapter for an injected package-run preparation port.
#[derive(Clone)]
pub struct SharedPackageRunPreparer(pub Arc<dyn PackageRunPreparer + Send + Sync>);

impl PackageRunPreparer for SharedPackageRunPreparer {
    fn prepare_package_run(
        &self,
        plan: &TaskPackageRoutePlan,
        task_run_id: &str,
    ) -> Result<PreparedPackageRun, DispatchPortError> {
        self.0.prepare_package_run(plan, task_run_id)
    }
}

/// Shared handle adapter for an injected package capability invoker.
#[derive(Clone)]
pub struct SharedPackageStepInvoker(pub Arc<dyn PackageStepInvoker>);

#[async_trait]
impl PackageStepInvoker for SharedPackageStepInvoker {
    async fn invoke_capability(
        &self,
        instance: &BoundCapabilityInstance,
        payload: &CapabilityInvocationPayload,
    ) -> Result<CapabilityInvocationResult, meld_execution::error::ApiError> {
        self.0.invoke_capability(instance, payload).await
    }

    fn compile_expansion(
        &self,
        compiled_task: &CompiledTaskRecord,
        request: &TaskExpansionRequest,
    ) -> Result<CompiledTaskDelta, meld_execution::error::ApiError> {
        self.0.compile_expansion(compiled_task, request)
    }
}

/// Shared handle adapter for an injected claimed-task invoker.
#[derive(Clone)]
pub struct SharedClaimedTaskInvoker(pub Arc<dyn ClaimedTaskInvoker>);

#[async_trait]
impl ClaimedTaskInvoker for SharedClaimedTaskInvoker {
    async fn invoke_claimed_task(
        &self,
        node: &TaskNode,
        claim: &Claim,
        init_payload: &TaskInitializationPayload,
    ) -> Result<ClaimedInvocationOutcome, DispatchPortError> {
        self.0.invoke_claimed_task(node, claim, init_payload).await
    }
}

fn map_projection_error(error: PlannerProjectionError) -> RuntimePortError {
    RuntimePortError::PlannerProjection(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use meld_execution::goals::{GoalAcceptanceLifecycle, GoalAcceptanceRequest};
    use meld_lang::{Goal, GoalLifecycle, GoalPriority, GoalSource, Proposition, Term};
    use meld_world_model::agent::AgentGoalMutationKind;
    use meld_world_model::AgentCurationDedupeKey;

    #[test]
    fn generic_terminal_capability_marker_prevents_unbounded_claim_replay() {
        assert!(is_terminal_claimed_failure(&format!(
            "{}: exhausted bounded validation",
            meld_execution::error::TERMINAL_CAPABILITY_FAILURE_MARKER
        )));
        assert!(!is_terminal_claimed_failure(
            "Provider request failed: connection reset"
        ));
    }

    fn subject() -> DomainObjectRef {
        DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap()
    }

    fn rule() -> meld_world_model::AgentCurationRuleConfig {
        meld_world_model::AgentCurationRuleConfig {
            dimension_id: "docs_freshness".to_string(),
            threshold: 0.7,
            priority_urgency: 8,
            desired_summary: "fresh docs".to_string(),
            source_kind: "docs_freshness".to_string(),
        }
    }

    fn goal(goal_id: &str) -> Goal {
        Goal {
            goal_id: goal_id.to_string(),
            agent_id: "agent-a".to_string(),
            target: Proposition::Holds {
                subject: Term::Object(subject()),
                dimension: Term::Dimension("docs_freshness".to_string()),
                condition: rule().target_condition(),
            },
            priority: GoalPriority {
                urgency: 8,
                cost_ceiling: None,
            },
            source: GoalSource::BeliefDivergence {
                dimension: "docs_freshness".to_string(),
                observed: "confidence=0.2".to_string(),
                desired: "fresh docs".to_string(),
            },
            lifecycle: GoalLifecycle::Proposed,
        }
    }

    fn open_goal_store() -> (tempfile::TempDir, Arc<PersistentGoalSetStore>) {
        let temp = tempfile::tempdir().unwrap();
        let db = sled::open(temp.path().join("goals.sled")).unwrap();
        (temp, Arc::new(PersistentGoalSetStore::new(db).unwrap()))
    }

    fn accept_goal(store: &Arc<PersistentGoalSetStore>, goal_id: &str, seq: u64) {
        let command = GoalAcceptanceRequest {
            metadata: GoalCommandMetadata {
                command_id: format!("command-{goal_id}"),
                source_identity: Some(format!("identity-{goal_id}")),
                seq,
            },
            goal: goal(goal_id),
            lifecycle_policy: GoalAcceptanceLifecycle::RequireProposedThenActivate,
            strategy_authorization: None,
        };
        ExecutionGoalCommandPort::new(Arc::clone(store))
            .accept_goal(command)
            .unwrap();
    }

    #[test]
    fn agent_goal_query_port_reports_lifecycle_epochs_from_goal_records() {
        let (_temp, store) = open_goal_store();
        accept_goal(&store, "goal-a", 5);
        let mutation_port = ExecutionGoalMutationPort::new(Arc::clone(&store));
        let dedupe_key = AgentCurationDedupeKey::threshold_rule(
            "agent-a",
            &subject(),
            &BranchScope::main(),
            &rule(),
        );
        // Satisfy at epoch zero, then reopen: the durable record advances to
        // lifecycle epoch one.
        mutation_port
            .satisfy_agent_goal_mutation(AgentGoalMutationCommand {
                command_id: "mutation-satisfy".to_string(),
                agent_id: "agent-a".to_string(),
                goal_id: "goal-a".to_string(),
                kind: AgentGoalMutationKind::Satisfy {
                    at_seq: 6,
                    lifecycle_epoch: 0,
                },
                dedupe_key: dedupe_key.clone(),
                review_seq: 6,
                projection_version: "world_model.planner.v1".to_string(),
                planner_source_refs: Vec::new(),
                planner_warnings: Vec::new(),
            })
            .unwrap();
        mutation_port
            .satisfy_agent_goal_mutation(AgentGoalMutationCommand {
                command_id: "mutation-reopen".to_string(),
                agent_id: "agent-a".to_string(),
                goal_id: "goal-a".to_string(),
                kind: AgentGoalMutationKind::Reopen {
                    triggering_belief_revision_id: "belief-revision-b".to_string(),
                    observed_lifecycle_epoch: 0,
                },
                dedupe_key,
                review_seq: 7,
                projection_version: "world_model.planner.v1".to_string(),
                planner_source_refs: Vec::new(),
                planner_warnings: Vec::new(),
            })
            .unwrap();

        let summary = ExecutionAgentGoalQueryPort::new(Arc::clone(&store))
            .active_goals_for_agent("agent-a")
            .unwrap();

        // The observed epoch flows from the execution goal record into the
        // snapshot the satisfy adapter binds its mutations to.
        assert_eq!(summary.lifecycle_epoch("goal-a"), 1);
        assert_eq!(summary.goals.len(), 1);
        assert!(matches!(summary.goals[0].lifecycle, GoalLifecycle::Active));
    }

    #[test]
    fn agent_goal_query_port_filters_by_agent_identity() {
        let (_temp, store) = open_goal_store();
        accept_goal(&store, "goal-a", 5);

        let summary = ExecutionAgentGoalQueryPort::new(Arc::clone(&store))
            .active_goals_for_agent("agent-other")
            .unwrap();

        assert!(summary.goals.is_empty());
        assert!(summary.lifecycle_epochs.is_empty());
    }

    #[test]
    fn curation_port_routes_commands_and_absorbs_stale_mutations() {
        let (_temp, store) = open_goal_store();
        accept_goal(&store, "goal-a", 5);
        let mut port = CurationGoalExecutionPort::new(
            ExecutionGoalCommandPort::new(Arc::clone(&store)),
            ExecutionGoalMutationPort::new(Arc::clone(&store)),
            9,
        );
        let dedupe_key = AgentCurationDedupeKey::threshold_rule(
            "agent-a",
            &subject(),
            &BranchScope::main(),
            &rule(),
        );

        // A satisfy that observed a stale epoch is absorbed as duplicate:
        // the goal record must stay byte-identical under the epoch fence.
        let stale = port
            .submit_goal_mutation(&AgentGoalMutationCommand {
                command_id: "mutation-stale".to_string(),
                agent_id: "agent-a".to_string(),
                goal_id: "goal-a".to_string(),
                kind: AgentGoalMutationKind::Satisfy {
                    at_seq: 9,
                    lifecycle_epoch: 3,
                },
                dedupe_key,
                review_seq: 9,
                projection_version: "world_model.planner.v1".to_string(),
                planner_source_refs: Vec::new(),
                planner_warnings: Vec::new(),
            })
            .unwrap();

        assert_eq!(stale.outcome, "duplicate");
        let record = store.get_goal("goal-a").unwrap().unwrap();
        assert_eq!(record.lifecycle_epoch, 0);
        assert!(matches!(record.goal.lifecycle, GoalLifecycle::Active));
    }
}
