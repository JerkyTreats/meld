//! Thin direct handoff ports built by product runtime assembly.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use meld_events::error::EventAuthorityError;
use meld_events::{
    AppendMode, AppendReceipt, EventAppendCapability, EventAuthority,
    EventConsumerRegistryCapability, EventEnvelope, EventObservabilityCapability, EventPage,
    EventRecord, EventReplayCapability, EventWatermark, EventWatermarkCapability, LedgerCursor,
    LedgerIdentity, ReplayRequest,
};
use meld_execution::goals::{
    GoalAcceptanceLifecycle, GoalAcceptanceRequest, GoalCommandMetadata, GoalCommandOutcome,
    GoalSetApi, PersistentGoalSetStore,
};
use meld_execution::planning::{
    PlanningProjectionError as ExecutionPlanningProjectionError,
    PlanningProjectionPort as ExecutionPlanningProjectionPort, PlanningWorldStateFrameRef,
    PlanningWorldStateProjection, PlanningWorldStateRequest,
};
use meld_execution::task::TaskArtifactRepoFactory;
use meld_execution::task_network::authority::{
    TaskNetworkAuthority, TaskNetworkAuthorityLifecycleSnapshot,
    TaskNetworkAuthorityShutdownReceipt, TaskNetworkCommandPort, TaskNetworkQueryPort,
};
use meld_execution::task_network::store::TaskNetworkStoreFactory;
use meld_execution::task_network::{EventAppendFailure, EventAppendSink};
use meld_world_model::belief::EvidenceEventReplaySource;
use meld_world_model::error::StorageError as WorldModelStorageError;
use meld_world_model::planner::{
    PlannerProjectionFrame, PlannerProjectionRequest, PlannerProjectionRequestRecord,
    PlannerProjectionRequestStatus, PlannerProjectionStore,
};
use meld_world_model::world_state::graph::{
    GraphConsumerCursorReporter, GraphDerivedEventSink, GraphEventReplaySource, PerspectiveKey,
};
use meld_world_model::{AgentGoalCommand, AgentGoalMutationCommand, BranchScope};

use crate::context::frame::FrameStorage;
use crate::control::projection::ExecutionProjectionReplaySource;
use crate::execution::goal_mutation::{satisfy_request_from_agent_mutation, GoalMutationRequest};
use crate::prompt_context::PromptContextArtifactStorage;
use crate::runtime::error::{RuntimeAssemblyError, RuntimePortError};
use crate::runtime::storage::OpenProductStores;
use crate::store::SledNodeRecordStore;

/// Maximum events returned by one root-assembled replay port call.
pub const MAX_EVENT_REPLAY_LIMIT: usize = 1024;

/// All direct handoff and root adapter ports produced by product assembly.
#[derive(Clone)]
pub struct ProductRuntimePorts {
    event_append: ProductEventAppendPort,
    event_replay: ProductEventReplayPort,
    graph_cursor: ProductGraphCursorPort,
    goal_command: ExecutionGoalCommandPort,
    goal_mutation: ExecutionGoalMutationPort,
    planner_projection: PlannerProjectionPort,
    adapters: RuntimeAdapterPorts,
}

/// Passive root adapter ports shared with runtime factories.
#[derive(Clone)]
pub struct RuntimeAdapterPorts {
    context: ContextRuntimePort,
    provider: ProviderRuntimePort,
    prompt: PromptRuntimePort,
    workspace: WorkspaceRuntimePort,
    task_artifacts: TaskArtifactFactoryPort,
    task_networks: TaskNetworkAuthorityHostPort,
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

/// Durable planner projection port backed by the world-model authority.
#[derive(Clone)]
pub struct PlannerProjectionPort {
    store: Arc<PlannerProjectionStore>,
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

/// Lazy process host for execution-owned task-network authorities.
///
/// The adapter owns no task semantics. It serializes process-local authority
/// admission so one configured network cannot be opened twice, then exposes
/// only cloneable execution-owned command and query capabilities.
#[derive(Clone)]
pub struct TaskNetworkAuthorityHostPort {
    factory: TaskNetworkStoreFactory,
    authorities: Arc<Mutex<BTreeMap<String, TaskNetworkAuthority>>>,
}

impl ProductRuntimePorts {
    /// Build root adapters from one already-resolved event authority.
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
            goal_command: ExecutionGoalCommandPort::new(Arc::clone(&stores.goal_store)),
            goal_mutation: ExecutionGoalMutationPort::new(Arc::clone(&stores.goal_store)),
            planner_projection: PlannerProjectionPort::new(Arc::clone(
                &stores.planner_projection_store,
            )),
            adapters: RuntimeAdapterPorts {
                context: ContextRuntimePort::new(Arc::clone(&stores.frame_storage)),
                provider: provider_port,
                prompt: PromptRuntimePort::new(Arc::clone(&stores.prompt_artifacts)),
                workspace: WorkspaceRuntimePort::new(Arc::clone(&stores.node_store)),
                task_artifacts: TaskArtifactFactoryPort::new(stores.task_artifacts.clone()),
                task_networks: TaskNetworkAuthorityHostPort::new(stores.task_networks.clone()),
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

    /// Return the goal mutation sink port.
    pub fn goal_mutation(&self) -> &ExecutionGoalMutationPort {
        &self.goal_mutation
    }

    /// Return the planner projection query port.
    pub fn planner_projection(&self) -> &PlannerProjectionPort {
        &self.planner_projection
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

    /// Return the lazy task-network authority host adapter.
    pub fn task_networks(&self) -> &TaskNetworkAuthorityHostPort {
        &self.task_networks
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

    /// Fence shared append ingress, drain accepted work, and return the final durable barrier.
    pub(crate) fn close_and_drain(
        &self,
    ) -> Result<meld_events::EventFinalBarrier, RuntimePortError> {
        self.append
            .close_and_drain()
            .map_err(|error| RuntimePortError::EventAppend(error.to_string()))
    }

    /// Return the current shared append-ingress fence for root lifecycle routing.
    pub(crate) fn ingress_fence(&self) -> meld_events::EventIngressFenceSnapshot {
        self.append.ingress_fence()
    }
}

impl EventAppendSink for ProductEventAppendPort {
    fn ledger_identity(&self) -> LedgerIdentity {
        self.append.ledger_identity()
    }

    fn append_envelope_idempotent(
        &self,
        envelope: EventEnvelope,
    ) -> Result<AppendReceipt, EventAppendFailure> {
        self.append
            .append_durable(envelope, AppendMode::Idempotent)
            .map_err(EventAppendFailure::from)
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

impl EvidenceEventReplaySource for ProductEventReplayPort {
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
        };
        self.accept_goal(request)
    }
}

impl ExecutionGoalMutationPort {
    /// Bind the port to an opened execution goal store.
    pub fn new(store: Arc<PersistentGoalSetStore>) -> Self {
        Self { store }
    }

    /// Apply a world-model satisfaction mutation through execution-owned APIs.
    pub fn satisfy_agent_goal_mutation(
        &self,
        command: AgentGoalMutationCommand,
    ) -> Result<GoalCommandOutcome, RuntimePortError> {
        let command = satisfy_request_from_agent_mutation(GoalMutationRequest { command })
            .map_err(|error| RuntimePortError::InvalidRequest(error.to_string()))?;
        let mut store = self.store.as_ref().clone();
        GoalSetApi::new(&mut store)
            .satisfy_goal(command)
            .map_err(|error| RuntimePortError::Storage(error.to_string()))
    }
}

impl PlannerProjectionPort {
    /// Bind the port to the world-model planner request authority.
    pub fn new(store: Arc<PlannerProjectionStore>) -> Self {
        Self { store }
    }
}

impl ExecutionPlanningProjectionPort for PlannerProjectionPort {
    fn project(
        &mut self,
        request: PlanningWorldStateRequest,
    ) -> Result<PlanningWorldStateProjection, ExecutionPlanningProjectionError> {
        let durable_request = execution_projection_request(&request)?;
        let record = match self
            .store
            .get_request(&durable_request.request_id)
            .map_err(map_planner_storage_error)?
        {
            Some(record) => revalidate_existing_planner_request(
                self.store.as_ref(),
                &durable_request,
                request.source_seq,
                record,
            )?,
            None => match self
                .store
                .put_pending(durable_request.clone(), request.source_seq)
            {
                Ok(record) => record,
                Err(error) if planner_submit_outcome_ambiguous(&error) => recover_planner_submit(
                    self.store.as_ref(),
                    &durable_request,
                    request.source_seq,
                    error,
                )?,
                Err(error) => return Err(map_planner_storage_error(error)),
            },
        };
        resolve_planner_projection(self.store.as_ref(), &request, &durable_request, record)
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

impl TaskNetworkAuthorityHostPort {
    /// Bind a lazy authority host to the execution-owned store factory.
    pub fn new(factory: TaskNetworkStoreFactory) -> Self {
        Self {
            factory,
            authorities: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Return a cloneable command capability for one hosted network.
    pub fn command_port(
        &self,
        network_id: &str,
    ) -> Result<TaskNetworkCommandPort, RuntimePortError> {
        self.with_authority(network_id, TaskNetworkAuthority::command_port)
    }

    /// Return a cloneable query capability for one hosted network.
    pub fn query_port(&self, network_id: &str) -> Result<TaskNetworkQueryPort, RuntimePortError> {
        self.with_authority(network_id, TaskNetworkAuthority::query_port)
    }

    /// Return the execution-owned lifecycle for one hosted network.
    pub fn lifecycle(
        &self,
        network_id: &str,
    ) -> Result<TaskNetworkAuthorityLifecycleSnapshot, RuntimePortError> {
        self.with_authority(network_id, TaskNetworkAuthority::lifecycle)
    }

    pub(crate) fn start_authority(
        &self,
        network_id: &str,
        mailbox_capacity: usize,
    ) -> Result<TaskNetworkAuthorityLifecycleSnapshot, RuntimePortError> {
        let mut authorities = self.lock_authorities()?;
        if authorities.contains_key(network_id) {
            return Err(RuntimePortError::TaskNetworkAuthority(format!(
                "network '{network_id}' already has a process authority"
            )));
        }
        let authority =
            TaskNetworkAuthority::open(&self.factory, network_id.to_string(), mailbox_capacity)
                .map_err(map_task_network_authority_error)?;
        let lifecycle = authority.lifecycle();
        authorities.insert(network_id.to_string(), authority);
        Ok(lifecycle)
    }

    pub(crate) fn shutdown_authority(
        &self,
        network_id: &str,
    ) -> Result<TaskNetworkAuthorityShutdownReceipt, RuntimePortError> {
        let mut authorities = self.lock_authorities()?;
        let shutdown = authorities
            .get_mut(network_id)
            .ok_or_else(|| {
                RuntimePortError::TaskNetworkAuthority(format!(
                    "network '{network_id}' has no hosted process authority"
                ))
            })?
            .shutdown()
            .map_err(map_task_network_authority_error);
        authorities.remove(network_id);
        shutdown
    }

    /// Reopen and flush one authority store after an indeterminate stop.
    pub(crate) fn reconcile_stopped_authority(
        &self,
        network_id: &str,
    ) -> Result<(), RuntimePortError> {
        {
            let mut authorities = self.lock_authorities()?;
            if let Some(authority) = authorities.get_mut(network_id) {
                let _ = authority.shutdown();
                authorities.remove(network_id);
            }
        }
        self.factory
            .open_network(network_id.to_string())
            .and_then(|store| store.flush())
            .map_err(|error| {
                RuntimePortError::TaskNetworkAuthority(format!(
                    "network '{network_id}' stopped authority could not be reflushed: {error}"
                ))
            })
    }

    fn with_authority<T>(
        &self,
        network_id: &str,
        map: impl FnOnce(&TaskNetworkAuthority) -> T,
    ) -> Result<T, RuntimePortError> {
        let authorities = self.lock_authorities()?;
        authorities.get(network_id).map(map).ok_or_else(|| {
            RuntimePortError::TaskNetworkAuthority(format!(
                "network '{network_id}' has no hosted process authority"
            ))
        })
    }

    fn lock_authorities(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, BTreeMap<String, TaskNetworkAuthority>>, RuntimePortError>
    {
        self.authorities.lock().map_err(|_| {
            RuntimePortError::TaskNetworkAuthority(
                "process authority host lock is poisoned".to_string(),
            )
        })
    }
}

fn map_task_network_authority_error(
    error: meld_execution::task_network::authority::TaskNetworkAuthorityError,
) -> RuntimePortError {
    RuntimePortError::TaskNetworkAuthority(error.to_string())
}

fn execution_projection_request(
    request: &PlanningWorldStateRequest,
) -> Result<PlannerProjectionRequest, ExecutionPlanningProjectionError> {
    request.validate().map_err(|error| {
        ExecutionPlanningProjectionError::fatal(format!(
            "execution projection request is invalid: {error}"
        ))
    })?;
    let source_request_hash = request.canonical_hash().map_err(|error| {
        ExecutionPlanningProjectionError::fatal(format!(
            "execution projection request identity failed: {error}"
        ))
    })?;
    let perspective = PerspectiveKey::new(
        request.perspective.perspective_kind.clone(),
        request.perspective.perspective_id.clone(),
    )
    .map_err(|error| {
        ExecutionPlanningProjectionError::fatal(format!(
            "execution projection perspective is invalid: {error}"
        ))
    })?;
    let branch_scope = BranchScope::new(request.branch_id.clone()).map_err(|error| {
        ExecutionPlanningProjectionError::fatal(format!(
            "execution projection branch is invalid: {error}"
        ))
    })?;
    PlannerProjectionRequest::identified(
        source_request_hash,
        request.agent_id.clone(),
        request.subject.clone(),
        perspective,
        branch_scope,
        request.requested_dimensions.clone(),
        request.required_preconditions.clone(),
    )
    .map_err(|error| {
        ExecutionPlanningProjectionError::fatal(format!(
            "world-model projection request identity failed: {error}"
        ))
    })
}

fn resolve_planner_projection(
    store: &PlannerProjectionStore,
    source_request: &PlanningWorldStateRequest,
    expected_request: &PlannerProjectionRequest,
    record: PlannerProjectionRequestRecord,
) -> Result<PlanningWorldStateProjection, ExecutionPlanningProjectionError> {
    record.validate().map_err(|error| {
        ExecutionPlanningProjectionError::fatal(format!(
            "durable planner projection request is invalid: {error}"
        ))
    })?;
    if record.request != *expected_request || record.created_at_seq != source_request.source_seq {
        return Err(ExecutionPlanningProjectionError::fatal(format!(
            "durable planner projection identity conflicts with execution request '{}'",
            expected_request.request_id
        )));
    }

    match record.status {
        PlannerProjectionRequestStatus::Pending => {
            Err(ExecutionPlanningProjectionError::retryable(format!(
                "planner projection request '{}' is pending",
                expected_request.request_id
            )))
        }
        PlannerProjectionRequestStatus::Failed => {
            Err(ExecutionPlanningProjectionError::fatal(format!(
                "planner projection request '{}' failed: {}",
                expected_request.request_id,
                record
                    .last_error
                    .as_deref()
                    .unwrap_or("missing failure detail")
            )))
        }
        PlannerProjectionRequestStatus::Completed => {
            let frame_id = record.frame_id.as_deref().ok_or_else(|| {
                ExecutionPlanningProjectionError::fatal(format!(
                    "completed planner projection request '{}' has no frame identity",
                    expected_request.request_id
                ))
            })?;
            let frame = store
                .get_frame(frame_id)
                .map_err(map_planner_storage_error)?
                .ok_or_else(|| {
                    ExecutionPlanningProjectionError::fatal(format!(
                        "completed planner projection request '{}' is missing frame '{frame_id}'",
                        expected_request.request_id
                    ))
                })?;
            store
                .verify_completed_projection(&record, &frame)
                .map_err(map_planner_storage_error)?;
            adapt_planner_frame(source_request, frame)
        }
    }
}

fn adapt_planner_frame(
    request: &PlanningWorldStateRequest,
    frame: PlannerProjectionFrame,
) -> Result<PlanningWorldStateProjection, ExecutionPlanningProjectionError> {
    let mut source_refs = frame
        .output
        .source_refs
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            ExecutionPlanningProjectionError::fatal(format!(
                "planner projection source provenance is invalid: {error}"
            ))
        })?;
    source_refs.sort();
    source_refs.dedup();
    let mut warnings = frame
        .output
        .warnings
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            ExecutionPlanningProjectionError::fatal(format!(
                "planner projection warning provenance is invalid: {error}"
            ))
        })?;
    warnings.sort();
    warnings.dedup();
    let world_state = frame.output.world_state;
    let frame_ref = PlanningWorldStateFrameRef::from_authority(
        frame.identity.frame_id,
        frame.identity.request_id,
        frame.identity.source_request_hash,
        frame.identity.projection_version,
        frame.identity.projection_hash,
        frame.identity.world_state_hash,
        request,
        &world_state,
        source_refs,
        warnings,
    )
    .map_err(|error| {
        ExecutionPlanningProjectionError::fatal(format!(
            "planner projection frame does not satisfy execution identity: {error}"
        ))
    })?;
    Ok(PlanningWorldStateProjection {
        world_state,
        frame: frame_ref,
    })
}

fn map_planner_storage_error(error: WorldModelStorageError) -> ExecutionPlanningProjectionError {
    let message = format!("planner projection authority failed: {error}");
    if planner_storage_retryable(&error) {
        ExecutionPlanningProjectionError::retryable(message)
    } else {
        ExecutionPlanningProjectionError::fatal(message)
    }
}

fn planner_storage_retryable(error: &WorldModelStorageError) -> bool {
    matches!(
        error,
        WorldModelStorageError::Unavailable(_)
            | WorldModelStorageError::DurabilityIndeterminate(_)
            | WorldModelStorageError::IoError(_)
    )
}

fn planner_submit_outcome_ambiguous(error: &WorldModelStorageError) -> bool {
    matches!(error, WorldModelStorageError::Backpressure(_)) || planner_storage_retryable(error)
}

fn recover_planner_submit(
    store: &PlannerProjectionStore,
    request: &PlannerProjectionRequest,
    created_at_seq: u64,
    original_error: WorldModelStorageError,
) -> Result<PlannerProjectionRequestRecord, ExecutionPlanningProjectionError> {
    let recovered = match store
        .get_request(&request.request_id)
        .map_err(map_planner_storage_error)?
    {
        Some(record) => record,
        None => return Err(map_planner_storage_error(original_error)),
    };
    if recovered.request != *request || recovered.created_at_seq != created_at_seq {
        return Ok(recovered);
    }
    if recovered.status != PlannerProjectionRequestStatus::Pending {
        return Ok(recovered);
    }

    match store.put_pending(request.clone(), created_at_seq) {
        Ok(record) => Ok(record),
        Err(WorldModelStorageError::Backpressure(message)) => {
            let latest = store
                .get_request(&request.request_id)
                .map_err(map_planner_storage_error)?
                .ok_or_else(|| {
                    ExecutionPlanningProjectionError::fatal(format!(
                        "planner projection request '{}' disappeared during submit recovery",
                        request.request_id
                    ))
                })?;
            if latest.request == *request
                && latest.created_at_seq == created_at_seq
                && latest.status != PlannerProjectionRequestStatus::Pending
            {
                Ok(latest)
            } else {
                Err(ExecutionPlanningProjectionError::fatal(format!(
                    "planner projection request '{}' conflicts with durable submit state: {message}",
                    request.request_id
                )))
            }
        }
        Err(error) => Err(map_planner_storage_error(error)),
    }
}

fn revalidate_existing_planner_request(
    store: &PlannerProjectionStore,
    request: &PlannerProjectionRequest,
    created_at_seq: u64,
    record: PlannerProjectionRequestRecord,
) -> Result<PlannerProjectionRequestRecord, ExecutionPlanningProjectionError> {
    if record.request != *request
        || record.created_at_seq != created_at_seq
        || record.status != PlannerProjectionRequestStatus::Pending
    {
        return Ok(record);
    }
    match store.put_pending(request.clone(), created_at_seq) {
        Ok(replayed) => Ok(replayed),
        Err(WorldModelStorageError::Backpressure(message)) => {
            let latest = store
                .get_request(&request.request_id)
                .map_err(map_planner_storage_error)?
                .ok_or_else(|| {
                    ExecutionPlanningProjectionError::fatal(format!(
                        "planner projection request '{}' disappeared during replay validation",
                        request.request_id
                    ))
                })?;
            if latest.request == *request
                && latest.created_at_seq == created_at_seq
                && latest.status != PlannerProjectionRequestStatus::Pending
            {
                Ok(latest)
            } else {
                Err(ExecutionPlanningProjectionError::fatal(format!(
                    "planner projection request '{}' conflicts with its durable status index: {message}",
                    request.request_id
                )))
            }
        }
        Err(error) => Err(map_planner_storage_error(error)),
    }
}
