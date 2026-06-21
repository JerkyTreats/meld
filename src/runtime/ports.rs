//! Thin direct handoff ports built by product runtime assembly.

use std::sync::Arc;

use meld_events::events::store::EventStore;
use meld_events::{DomainObjectRef, EventEnvelope, EventRecord};
use meld_execution::goals::{
    GoalAcceptanceLifecycle, GoalAcceptanceRequest, GoalCommandMetadata, GoalCommandOutcome,
    GoalSetApi, PersistentGoalSetStore,
};
use meld_execution::task::TaskArtifactRepoFactory;
use meld_execution::task_network::store::TaskNetworkStoreFactory;
use meld_execution::task_network::EventAppendSink;
use meld_world_model::planner::{PlannerProjectionError, PlannerProjectionOutput, PlannerQuery};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::world_state::graph::PerspectiveKey;
use meld_world_model::{AgentGoalCommand, AgentGoalMutationCommand, BeliefQuery, BeliefStore};
use meld_world_model::{BranchScope, TraversalQuery};

use crate::context::frame::FrameStorage;
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
    task_networks: TaskNetworkFactoryPort,
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

/// Execution callable event append port backed by the event store.
///
/// The port appends envelopes idempotently and returns the event sequence. It
/// does not inspect payload meaning, choose publication readiness, or mark
/// execution publication state.
#[derive(Clone)]
pub struct ProductEventAppendPort {
    store: Arc<EventStore>,
}

/// Bounded event replay source backed by the event store.
///
/// The port reads caller-supplied windows without storing replay cursors or
/// deciding which world model reducer should run.
#[derive(Clone)]
pub struct ProductEventReplayPort {
    store: Arc<EventStore>,
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
    /// Build all product runtime ports from opened stores and passive config.
    pub fn from_stores(
        stores: &OpenProductStores,
        provider: ProviderPortConfig,
    ) -> Result<Self, RuntimeAssemblyError> {
        let provider_port = ProviderRuntimePort::new(provider)?;
        Ok(Self {
            event_append: ProductEventAppendPort::new(Arc::clone(&stores.event_store)),
            event_replay: ProductEventReplayPort::new(Arc::clone(&stores.event_store)),
            goal_command: ExecutionGoalCommandPort::new(Arc::clone(&stores.goal_store)),
            goal_mutation: ExecutionGoalMutationPort::new(Arc::clone(&stores.goal_store)),
            planner_projection: PlannerProjectionPort::new(
                Arc::clone(&stores.belief_store),
                Arc::clone(&stores.traversal_store),
            ),
            adapters: RuntimeAdapterPorts {
                context: ContextRuntimePort::new(Arc::clone(&stores.frame_storage)),
                provider: provider_port,
                prompt: PromptRuntimePort::new(Arc::clone(&stores.prompt_artifacts)),
                workspace: WorkspaceRuntimePort::new(Arc::clone(&stores.node_store)),
                task_artifacts: TaskArtifactFactoryPort::new(stores.task_artifacts.clone()),
                task_networks: TaskNetworkFactoryPort::new(stores.task_networks.clone()),
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

    /// Return the task network factory adapter port.
    pub fn task_networks(&self) -> &TaskNetworkFactoryPort {
        &self.task_networks
    }
}

impl ProductEventAppendPort {
    /// Bind the port to an opened event store.
    pub fn new(store: Arc<EventStore>) -> Self {
        Self { store }
    }
}

impl EventAppendSink for ProductEventAppendPort {
    fn append_envelope_idempotent(&self, envelope: EventEnvelope) -> Result<u64, String> {
        self.store
            .append_envelope_idempotent(envelope)
            .map_err(|error| error.to_string())
    }
}

impl ProductEventReplayPort {
    /// Bind the port to an opened event store.
    pub fn new(store: Arc<EventStore>) -> Self {
        Self { store }
    }

    /// Read ordered event records after the caller supplied sequence.
    pub fn read_after_limit(
        &self,
        after_seq: u64,
        limit: usize,
    ) -> Result<Vec<EventRecord>, RuntimePortError> {
        if limit > MAX_EVENT_REPLAY_LIMIT {
            return Err(RuntimePortError::InvalidRequest(format!(
                "event replay limit {limit} exceeds maximum {MAX_EVENT_REPLAY_LIMIT}"
            )));
        }
        self.store
            .read_all_events_after_limit(after_seq, limit)
            .map_err(|error| RuntimePortError::EventReplay(error.to_string()))
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

fn map_projection_error(error: PlannerProjectionError) -> RuntimePortError {
    RuntimePortError::PlannerProjection(error.to_string())
}
