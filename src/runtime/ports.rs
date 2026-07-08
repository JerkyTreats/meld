//! Thin direct handoff ports built by product runtime assembly.

use std::sync::Arc;

use meld_events::events::store::EventStore;
use meld_events::{CommitWatermark, DomainObjectRef, EventEnvelope, EventRecord, SpineWriter};
use meld_execution::goals::{
    GoalAcceptanceLifecycle, GoalAcceptanceRequest, GoalCommandMetadata, GoalCommandOutcome,
    GoalSetApi, PersistentGoalSetStore,
};
use meld_execution::task::TaskArtifactRepoFactory;
use meld_execution::task_network::store::TaskNetworkStoreFactory;
use meld_execution::task_network::EventAppendSink;
use meld_world_model::belief::{
    ingest_promoted_evidence, ConfigSnapshot, PromotedEvidenceIngestionRequest,
};
use meld_world_model::planner::{PlannerProjectionError, PlannerProjectionOutput, PlannerQuery};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::world_state::graph::PerspectiveKey;
use meld_world_model::{
    AgentGoalCommand, AgentGoalMutationCommand, BeliefQuery, BeliefRuntime, BeliefStore,
    PromotedEvidenceIngestionResult,
};
use meld_world_model::{BranchScope, TraversalQuery};

use crate::context::frame::FrameStorage;
use crate::execution::goal_mutation::{satisfy_request_from_agent_mutation, GoalMutationRequest};
use crate::execution::{build_docs_task_success_evidence, DocsTaskSuccessEvidenceRequest};
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
    docs_task_evidence: DocsTaskEvidenceReplayPort,
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

/// Request to replay docs task success events into belief evidence.
#[derive(Debug, Clone)]
pub struct DocsTaskEvidenceReplayRequest {
    /// Last event sequence already processed by the caller.
    pub after_seq: u64,
    /// Maximum event records to inspect.
    pub limit: usize,
    /// Workspace subject whose belief should receive support evidence.
    pub subject: DomainObjectRef,
    /// Runtime family configuration used for evidence normalization.
    pub config: ConfigSnapshot,
    /// Perspective for candidate belief keys.
    pub perspective: PerspectiveKey,
    /// Branch scope for candidate belief keys.
    pub branch_scope: BranchScope,
    /// Worker identity recorded on assessment leases.
    pub owner_id: String,
    /// Artifact type that marks a success event as applicable docs content.
    pub required_artifact_type_id: Option<String>,
}

/// Report from one bounded docs task evidence replay pass.
#[derive(Debug, Clone, PartialEq)]
pub struct DocsTaskEvidenceReplayReport {
    /// Replay cursor supplied by the caller.
    pub input_event_seq: u64,
    /// Highest event sequence inspected by this pass.
    pub output_event_seq: u64,
    /// Event records inspected.
    pub events_attempted: usize,
    /// Promoted evidence records produced by the mapping.
    pub promoted_evidence_count: usize,
    /// Promoted evidence records rejected by belief config.
    pub rejected_evidence_count: usize,
    /// Evidence items produced by normalization.
    pub normalized_evidence_count: usize,
    /// New belief assignment edges inserted.
    pub new_assignment_count: usize,
    /// Per ingestion results returned by the belief runtime.
    pub ingestions: Vec<PromotedEvidenceIngestionResult>,
}

/// Execution callable event append port backed by the spine writer.
///
/// The port appends envelopes idempotently through the single-writer ingress
/// so producer appends share group commits, and returns the event sequence.
/// It does not inspect payload meaning, choose publication readiness, or mark
/// execution publication state.
#[derive(Clone)]
pub struct ProductEventAppendPort {
    writer: Arc<SpineWriter>,
}

/// Bounded event replay source backed by the event store.
///
/// The port reads caller-supplied windows without storing replay cursors or
/// deciding which world model reducer should run.
#[derive(Clone)]
pub struct ProductEventReplayPort {
    store: Arc<EventStore>,
}

/// Bounded docs task event to belief evidence replay port.
#[derive(Clone)]
pub struct DocsTaskEvidenceReplayPort {
    event_replay: ProductEventReplayPort,
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
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
        let event_replay = ProductEventReplayPort::new(Arc::clone(&stores.event_store));
        Ok(Self {
            event_append: ProductEventAppendPort::new(Arc::clone(&stores.event_store)),
            event_replay: event_replay.clone(),
            docs_task_evidence: DocsTaskEvidenceReplayPort::new(
                event_replay,
                Arc::clone(&stores.belief_store),
                Arc::clone(&stores.traversal_store),
            ),
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

    /// Return the docs task event to belief evidence replay port.
    pub fn docs_task_evidence(&self) -> &DocsTaskEvidenceReplayPort {
        &self.docs_task_evidence
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
    /// Bind the port to a spine writer over the opened event store.
    pub fn new(store: Arc<EventStore>) -> Self {
        Self {
            writer: Arc::new(SpineWriter::spawn(store)),
        }
    }

    /// Return the writer's commit watermark for wake-on-commit consumers.
    pub fn watermark(&self) -> Arc<CommitWatermark> {
        self.writer.watermark()
    }

    /// Return how many best-effort events backpressure has dropped.
    pub fn dropped_events(&self) -> u64 {
        self.writer.dropped_events()
    }
}

impl EventAppendSink for ProductEventAppendPort {
    fn append_envelope_idempotent(&self, envelope: EventEnvelope) -> Result<u64, String> {
        self.writer
            .append_durable(envelope, true)
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

impl DocsTaskEvidenceReplayPort {
    /// Bind the port to event replay and world model stores.
    pub fn new(
        event_replay: ProductEventReplayPort,
        belief_store: Arc<BeliefStore>,
        traversal_store: Arc<TraversalStore>,
    ) -> Self {
        Self {
            event_replay,
            belief_store,
            traversal_store,
        }
    }

    /// Replay bounded execution success events into belief evidence.
    pub fn ingest_after_limit(
        &self,
        request: DocsTaskEvidenceReplayRequest,
    ) -> Result<DocsTaskEvidenceReplayReport, RuntimePortError> {
        if request.owner_id.trim().is_empty() {
            return Err(RuntimePortError::InvalidRequest(
                "owner id must be non-empty".to_string(),
            ));
        }
        request
            .subject
            .validate()
            .map_err(|error| RuntimePortError::InvalidRequest(error.to_string()))?;
        let events = self
            .event_replay
            .read_after_limit(request.after_seq, request.limit)?;
        let runtime = BeliefRuntime::new(
            Arc::clone(&self.belief_store),
            Arc::clone(&self.traversal_store),
            request.config.clone(),
            request.perspective.clone(),
            request.branch_scope.clone(),
        );
        let mut report = DocsTaskEvidenceReplayReport {
            input_event_seq: request.after_seq,
            output_event_seq: request.after_seq,
            events_attempted: events.len(),
            promoted_evidence_count: 0,
            rejected_evidence_count: 0,
            normalized_evidence_count: 0,
            new_assignment_count: 0,
            ingestions: Vec::new(),
        };

        for event in events {
            report.output_event_seq = report.output_event_seq.max(event.seq);
            let promoted = build_docs_task_success_evidence(DocsTaskSuccessEvidenceRequest {
                event,
                subject: request.subject.clone(),
                stale_probability: 0.0,
                review_probability: 0.2,
                source_kind: "content_written".to_string(),
                required_artifact_type_id: request.required_artifact_type_id.clone(),
            })
            .map_err(|error| RuntimePortError::InvalidRequest(error.to_string()))?;
            let Some(record) = promoted else {
                continue;
            };
            report.promoted_evidence_count += 1;
            let ingestion = ingest_promoted_evidence(
                self.belief_store.as_ref(),
                &runtime,
                PromotedEvidenceIngestionRequest {
                    record,
                    config: request.config.clone(),
                    perspective: request.perspective.clone(),
                    branch_scope: request.branch_scope.clone(),
                    owner_id: &request.owner_id,
                },
            )
            .map_err(|error| RuntimePortError::Storage(error.to_string()))?;
            if ingestion.rejected {
                report.rejected_evidence_count += 1;
            }
            report.normalized_evidence_count += ingestion.normalized_evidence_count;
            report.new_assignment_count += ingestion.new_assignment_count;
            report.ingestions.push(ingestion);
        }

        Ok(report)
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
