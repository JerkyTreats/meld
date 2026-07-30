//! Product runtime assembly for durable flywheel infrastructure.
//!
//! Owner: root runtime composition. Assembly is machine initialization
//! (Runtime Initialization stages 0, 1, and 5): it opens stores for the
//! composed registration scope, builds ports, derives registrations from
//! the selected stewardship expression, and binds concrete domain actor
//! factories. It never creates semantic state — no genesis, no theory
//! install, no seeding. A world with incomplete genesis hydrates with the
//! genesis-dependent actors truthfully unresolved (no semantic body, so
//! the supervisor projects `UnresolvedRequiredBinding`), never with
//! manufactured state; a later process start after explicit initialization
//! resolves them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[cfg(test)]
use meld_events::EventAuthorityOpenOptions;
use meld_events::EventConsumerRegistryCapability;
use meld_events::{DomainObjectRef, DurableConsumerCursor, EventAuthority};
use meld_execution::capability::CapabilityCatalog;
use meld_execution::goals::PersistentGoalSetStore;
use meld_execution::planning::realization::TaskPackageRoutePlan;
use meld_execution::planning::{
    AvailableActionSet, ExecutionCompositionLowerer, MethodLibrary, MethodRealizationBinding,
    PlanningRuntime, PlanningRuntimeActor, PlanningRuntimeActorGoalResult,
    PlanningRuntimeActorRequest,
};
use meld_execution::task::package::{load_builtin_task_package_spec, PackageExpansionSpec};
use meld_execution::task::{TaskCompiler, TaskProgressStore};
use meld_execution::task_network::aggregate_publication::{
    publish_aggregate_for_run, AggregatePublicationError, AggregatePublicationStore,
    AggregatePublishResult, AggregateRunBinding, AggregateSkipReason, PublishAggregateRequest,
};
use meld_execution::task_network::dispatch_actor::{
    package_route_run_id, DispatchRuntimeActor, DispatchTickReport, DispatchTickRequest,
};
use meld_execution::task_network::terminal_recording::package_run_terminal_outcome;
use meld_execution::task_network::{
    PublicationRuntime, PublishPendingPublicationsRequest, SledTaskNetworkStore,
};
use meld_lang::Method;
use meld_world_model::agent::{
    AgentGoalCurationActor, AgentSatisfactionCurationActor, AgentStepReport, AgentStepRequest,
    AgentStore,
};
use meld_world_model::belief::{
    BeliefAssessmentActor, BeliefAssessmentReport, BeliefAssessmentRequest, BeliefFamilyRegistry,
    BeliefFamilyRegistryStore, BeliefStore, BeliefSubjectBinding, BranchScope,
    ConfiguredOutcomeMappingSet, EvidenceEventReplaySource, EvidenceIngestionActor,
    EvidenceIngestionReport, EvidenceIngestionRequest, OutcomeEvidenceMapping,
    OutcomeMappingSetConfig,
};
use meld_world_model::world_state::graph::runtime::{GraphCatchUpBudget, GraphRuntime};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::PerspectiveKey;
use serde::{Deserialize, Serialize};

use crate::config::MerkleConfig;
use crate::config::PhysicalBinding;
use crate::runtime::contracts::{
    WorkBudget, WorkerCheckpoint, WorkerScope, WorkerTickIssue, WorkerTickReport,
};
use crate::runtime::error::{RuntimeAssemblyError, RuntimeRegistryError};
use crate::runtime::ports::{
    CurationGoalExecutionPort, ExactKeyPlanningProjectionPort, ExecutionAgentGoalQueryPort,
    ExecutionGoalCommandPort, ExecutionGoalMutationPort, ProductEventAppendPort,
    ProductEventReplayPort, ProductRuntimePorts, ProviderPortConfig, SharedClaimedTaskInvoker,
    SharedPackageRunPreparer, SharedPackageStepInvoker,
};
use crate::runtime::registration::{RegistrationKind, RegistrationSet, RuntimeRegistration};
use crate::runtime::storage::{
    OpenProductStores, ProductStorageLayout, ProductStorageRoot, StoreScope,
};
use crate::runtime::supervisor::SupervisorStore;

/// Root product runtime assembly.
///
/// This type opens product stores for the composed registration scope,
/// opens supervisor lifecycle storage, builds direct handoff ports, and
/// binds concrete actor factories. It does not run semantic work or own
/// domain progress.
pub struct ProductRuntimeAssembly {
    product_root: ProductStorageRoot,
    layout: ProductStorageLayout,
    stores: Arc<OpenProductStores>,
    event_authority: Arc<EventAuthority>,
    graph_runtime: Option<Arc<GraphRuntime>>,
    supervisor_store: SupervisorStore,
    ports: ProductRuntimePorts,
    registry: RuntimeFactoryRegistry,
    handle_factories: RuntimeHandleFactoryRegistry,
    registration_set: Option<RegistrationSet>,
    desired_runtime_state: Vec<DesiredRuntimeState>,
    lifecycle_config: RuntimeLifecycleConfig,
    default_work_budget: WorkBudget,
    process_services: RuntimeProcessServices,
    diagnostics: Vec<AssemblyDiagnostic>,
    dispatch_route_slot: Option<DispatchRouteSlot>,
    dispatch_route_seed: Option<DispatchRouteSeed>,
}

/// Read-only product runtime description for operator CLI commands.
///
/// This description resolves product paths and desired runtime state without
/// opening product stores, creating directories, constructing ports, or
/// building process-local handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductRuntimeDescription {
    /// Resolved product storage root.
    pub product_root: PathBuf,
    /// Derived product storage layout.
    pub layout: ProductStorageLayout,
    /// Derived supervisor store path.
    pub supervisor_store_path: PathBuf,
    /// Desired runtime state calculated from runtime configuration.
    pub desired_runtime_state: Vec<DesiredRuntimeState>,
    /// Supervisor lifecycle timing defaults.
    pub lifecycle_config: RuntimeLifecycleConfig,
    /// Default bounded work budget.
    pub default_work_budget: WorkBudget,
    /// Passive process service identities.
    pub process_services: RuntimeProcessServices,
}

/// Inputs needed to build product runtime infrastructure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductRuntimeConfig {
    /// Product storage root used to derive every product store path.
    pub product_root: PathBuf,
    /// Optional override for the root supervisor store path.
    pub supervisor_store_path: Option<PathBuf>,
    /// Runtime ids desired by configuration. Empty means all first proof ids.
    pub enabled_runtime_ids: Vec<String>,
    /// Runtime ids kept disabled but visible in desired state.
    pub disabled_runtime_ids: Vec<String>,
    /// Explicit registration set composing this assembly.
    ///
    /// Registration-set composition is a public surface: harness and proof
    /// callers may compose any actor subset here, and store opening is
    /// scoped to the composed set. When absent, a stewardship composition
    /// derives the set; a plain composition opens everything and keeps the
    /// conservative supervisor classification.
    pub registration_set: Option<RegistrationSet>,
    /// Passive provider availability check.
    pub provider: ProviderPortConfig,
    /// Supervisor lifecycle timing defaults.
    pub lifecycle_config: RuntimeLifecycleConfig,
    /// Work budget defaults passed to inert runtime factories.
    pub default_work_budget: WorkBudget,
    /// Passive process service identities for supervisor handoff.
    pub process_services: RuntimeProcessServices,
}

/// Diagnostic captured while assembling product infrastructure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblyDiagnostic {
    /// Stable diagnostic code.
    pub code: String,
    /// Human-readable diagnostic detail with no secret values.
    pub message: String,
}

/// Desired runtime lifecycle mode loaded from product config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredRuntimeState {
    /// Stable runtime id.
    pub runtime_id: String,
    /// Whether the supervisor should start this runtime.
    pub enabled: bool,
    /// Whether product assembly has a factory for this runtime id.
    pub factory_available: bool,
}

/// Passive runtime factory metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFactoryDescriptor {
    /// Stable runtime id.
    pub runtime_id: String,
    /// Passive resources required by the future runtime handle.
    pub required_resources: Vec<RuntimeResource>,
}

/// Resource requirement advertised by a runtime factory descriptor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RuntimeResource {
    /// Event append port.
    EventAppend,
    /// Event replay port.
    EventReplay,
    /// Event authority durable consumer cursor registry.
    EventConsumerRegistry,
    /// Execution goal command port.
    GoalCommand,
    /// Execution goal mutation port.
    GoalMutation,
    /// Planner projection port.
    PlannerProjection,
    /// Task artifact repository factory.
    TaskArtifactFactory,
    /// Task network store factory.
    TaskNetworkFactory,
    /// Context frame storage.
    Context,
    /// Provider access.
    Provider,
    /// Prompt artifact storage.
    Prompt,
    /// Workspace node storage.
    Workspace,
    /// World model belief, agent, registry, and traversal stores.
    WorldModel,
}

/// Store scope one resource requirement pulls into a composition.
fn resource_store_scope(resource: &RuntimeResource) -> StoreScope {
    match resource {
        RuntimeResource::Workspace => StoreScope {
            workspace: true,
            ..StoreScope::none()
        },
        RuntimeResource::WorldModel | RuntimeResource::PlannerProjection => StoreScope {
            world_model: true,
            ..StoreScope::none()
        },
        RuntimeResource::GoalCommand | RuntimeResource::GoalMutation => StoreScope {
            execution_goals: true,
            ..StoreScope::none()
        },
        RuntimeResource::TaskArtifactFactory | RuntimeResource::TaskNetworkFactory => StoreScope {
            task_execution: true,
            ..StoreScope::none()
        },
        RuntimeResource::Context => StoreScope {
            context_frames: true,
            ..StoreScope::none()
        },
        RuntimeResource::Prompt => StoreScope {
            prompt_artifacts: true,
            ..StoreScope::none()
        },
        // Event capabilities come from the supplied authority and the
        // provider from process configuration; neither opens product stores.
        RuntimeResource::EventAppend
        | RuntimeResource::EventReplay
        | RuntimeResource::EventConsumerRegistry
        | RuntimeResource::Provider => StoreScope::none(),
    }
}

/// Derive the store scope one composed registration set requires.
///
/// The union covers both the resources declared on each registration and
/// the catalog descriptor's resources for its runtime id, so an explicit
/// set with empty resource lists still opens what its actors need.
pub fn scope_for_registration_set(
    set: &RegistrationSet,
    registry: &RuntimeFactoryRegistry,
) -> StoreScope {
    let mut scope = StoreScope::none();
    for registration in &set.registrations {
        for resource in &registration.required_resources {
            scope = scope.union(resource_store_scope(resource));
        }
        if let Some(descriptor) = registry.get(&registration.runtime_id) {
            for resource in &descriptor.required_resources {
                scope = scope.union(resource_store_scope(resource));
            }
        }
    }
    scope
}

/// Runtime ids classified as passive services in the stewardship-derived set.
///
/// The event append and replay capabilities and the goal-set and
/// task-network command services are called by actors; they are never
/// leased, never ticked, and receive no actor health. This includes the
/// former `event.append` diagnostics observer: its ledger-health facts now
/// live on the passive append capability's health surface
/// (`ProductEventAppendPort::health`) and the existing self-observation
/// watcher, so the actor-shaped diagnostics handle survives only as a
/// compatibility body for explicit legacy compositions.
const STEWARDSHIP_PASSIVE_SERVICE_IDS: [&str; 4] = [
    "event.append",
    "event.replay",
    "execution.goal_set",
    "execution.task_network_command",
];

/// Derive the runtime registration set from one validated stewardship binding.
///
/// Per the ground map: selecting docs freshness causes composition to derive
/// exactly the actor and passive-service registrations the convergence loop
/// requires. There are no product slots or slot counts — the derivation
/// walks the internal descriptor catalog and classifies each required role
/// honestly. Registration production stays a public composition surface:
/// this derivation is one producer, and harness callers may supply an
/// explicit [`RegistrationSet`] instead.
pub fn derive_stewardship_registrations(
    binding: &PhysicalBinding,
) -> Result<RegistrationSet, RuntimeAssemblyError> {
    let registry = RuntimeFactoryRegistry::first_proof_registry()?;
    let expression = &binding.package.expression;
    let registrations = registry
        .descriptors()
        .map(|descriptor| {
            let kind = if STEWARDSHIP_PASSIVE_SERVICE_IDS.contains(&descriptor.runtime_id.as_str())
            {
                RegistrationKind::PassiveService
            } else {
                RegistrationKind::ActiveActor
            };
            RuntimeRegistration {
                registration_id: format!("stewardship::{expression}::{}", descriptor.runtime_id),
                runtime_id: descriptor.runtime_id.clone(),
                kind,
                required_resources: descriptor.required_resources.clone(),
            }
        })
        .collect();
    Ok(RegistrationSet { registrations })
}

/// Canonical subject reference for one stewardship binding.
///
/// The docs freshness subject is a workspace filesystem node. This
/// derivation is shared identity: the initialization command surface must
/// derive the same reference when seeding the stage 4 genesis fact, or the
/// composed actors observe a different subject than genesis declared.
pub fn stewardship_subject_ref(
    binding: &PhysicalBinding,
) -> Result<DomainObjectRef, RuntimeAssemblyError> {
    DomainObjectRef::new("workspace_fs", "node", &binding.subject)
        .map_err(|error| RuntimeAssemblyError::Config(error.to_string()))
}

/// Pure actor-facing values derived from one stewardship binding.
///
/// Everything here is identity or physical scope — never theory bodies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StewardshipActorBindings {
    /// Stewardship expression name.
    pub expression: String,
    /// Canonical subject the expression stewards.
    pub subject: DomainObjectRef,
    /// Durable agent identity that stewards the subject.
    pub agent_id: String,
    /// Selected belief family identity, resolved by the world model per tick.
    pub belief_family_id: String,
    /// Selected outcome-to-evidence mapping identity.
    pub evidence_mapping_id: String,
    /// World model perspective the composed actors read through.
    pub perspective: PerspectiveKey,
    /// World model branch scope the composed actors read through.
    pub branch_scope: BranchScope,
    /// Graph anchor perspective kind for initial belief assessment.
    pub anchor_perspective_kind: String,
    /// Graph anchor perspective id for initial belief assessment.
    pub anchor_perspective_id: String,
    /// Task network the composed execution actors share.
    pub network_id: String,
    /// Event ledger session partition for aggregate publication.
    pub session_id: String,
    /// Durable capability type ids whose package work units carry
    /// per-folder work, derived from the selected package document.
    pub folder_unit_capability_types: Vec<String>,
}

impl StewardshipActorBindings {
    /// Derive actor-facing bindings from one validated physical binding.
    pub fn derive(binding: &PhysicalBinding) -> Result<Self, RuntimeAssemblyError> {
        let expression = binding.package.expression.clone();
        Ok(Self {
            subject: stewardship_subject_ref(binding)?,
            agent_id: binding.agent_id.clone(),
            belief_family_id: binding.package.belief_family_id.clone(),
            evidence_mapping_id: binding.package.evidence_mapping_id.clone(),
            perspective: PerspectiveKey::new("default", "default")
                .map_err(|error| RuntimeAssemblyError::Config(error.to_string()))?,
            branch_scope: BranchScope::main(),
            // Workspace graph anchors are published by context head
            // selections whose frame type derives from the agent identity —
            // the same convention the production dispatch route publishes
            // under, so anchor reads and frame publications share one
            // lineage vocabulary. This is identity derivation, not belief
            // theory.
            anchor_perspective_kind: "frame_type".to_string(),
            anchor_perspective_id: format!("context-{}", binding.agent_id),
            network_id: format!("stewardship.{expression}"),
            session_id: format!("stewardship::{expression}"),
            folder_unit_capability_types: folder_unit_capability_types(&expression)?,
            expression,
        })
    }
}

/// Derive per-folder work unit capability types from the selected package.
///
/// The stewardship expression selects the built-in package; the package
/// document's repeated per-node stage chain declares which durable
/// capability types execute folder work. Root only reads the declaration —
/// the classification is package-authored data, not root vocabulary.
fn folder_unit_capability_types(expression: &str) -> Result<Vec<String>, RuntimeAssemblyError> {
    let package_id = match expression {
        "docs_freshness" => "docs_writer",
        other => {
            return Err(RuntimeAssemblyError::Config(format!(
                "stewardship expression '{other}' selects no known task package"
            )))
        }
    };
    let spec = load_builtin_task_package_spec(package_id)
        .map_err(|error| RuntimeAssemblyError::Config(error.to_string()))?;
    let mut types = BTreeSet::new();
    for expansion in &spec.expansions {
        let PackageExpansionSpec::TraversalPrerequisite(traversal) = expansion;
        for stage in &traversal.repeated_region.stage_chain.stages {
            types.insert(stage.capability_type_id.clone());
        }
    }
    if types.is_empty() {
        return Err(RuntimeAssemblyError::Config(format!(
            "task package '{package_id}' declares no per-folder stage capability types"
        )));
    }
    Ok(types.into_iter().collect())
}

/// Injected theory and route bindings for one stewardship composition.
///
/// Assembly never bakes theory bodies into factories. Theory with a durable
/// registry (the belief family, the curation rule on the agent record)
/// resolves at tick time from that registry. The remaining kinds have no
/// durable registry yet, so composition callers inject them here; a missing
/// injection leaves the dependent actor a truthful unresolved required
/// binding instead of manufacturing behavior.
#[derive(Default)]
pub struct StewardshipTheoryBindings {
    /// Installed outcome-to-evidence mapping set configuration.
    ///
    /// The installed unit is the mapping set: one selected identity whose
    /// rules interpret every canonical outcome shape the expression's theory
    /// recognizes — per-task publications plus package aggregates — so the
    /// assembled ingestion actor never binds a narrower vocabulary than the
    /// installed theory declares.
    ///
    /// Seam: when the world model gains its durable mapping registry
    /// (Runtime Initialization stage 2), assembly hydrates from it and this
    /// injection becomes harness-only.
    pub outcome_mapping: Option<OutcomeMappingSetConfig>,
    /// Planning theory: methods, catalog, afforded actions, realizations.
    pub planning: Option<PlanningTheoryBinding>,
    /// Real execution route bindings for the dispatch actor.
    pub dispatch: Option<DispatchRouteBindings>,
}

/// Planning theory injected until a durable method registry exists.
#[derive(Clone)]
pub struct PlanningTheoryBinding {
    /// Planning methods for the stewarded domain.
    pub methods: Vec<Method>,
    /// Capability catalog the methods and lowering resolve against.
    pub capability_catalog: CapabilityCatalog,
    /// Actions the stewarded domain affords, in the frozen affordance shape.
    pub available_actions: AvailableActionSet,
    /// Data-driven method-to-action associations.
    pub method_realizations: Vec<MethodRealizationBinding>,
    /// Dimensions the projection port should prioritize for each goal.
    pub requested_dimensions: Vec<String>,
}

/// Execution route ports injected for the dispatch actor.
#[derive(Clone)]
pub struct DispatchRouteBindings {
    /// Resolves plan handoffs into compiled package runs.
    pub preparer: SharedPackageRunPreparer,
    /// Executes package capability invocations over the real route.
    pub package_invoker: SharedPackageStepInvoker,
    /// Executes claimed task invocations over the real route.
    pub claim_invoker: SharedClaimedTaskInvoker,
}

impl DispatchRouteBindings {
    /// Compose the production execution routes over the real machinery.
    ///
    /// The preparer resolves plans through the registered workflow package
    /// surface, capability invocations execute through the workflow
    /// task-path capability set, and providers resolve through the provider
    /// registry the api facade carries — mirroring the registered workflow
    /// route, sourced from product config and stewardship bindings.
    pub fn production(context: crate::runtime::ports::ProductionDispatchRouteContext) -> Self {
        let (preparer, package_invoker, claim_invoker) = context.into_route_ports();
        Self {
            preparer,
            package_invoker,
            claim_invoker,
        }
    }
}

/// Shared late-binding slot for the dispatch execution route ports.
///
/// Owner: product runtime assembly. The production dispatch routes execute
/// through the root api facade, which the CLI composes after product stores
/// open — after this assembly has already built its factories. The slot lets
/// the composition caller bind the routes exactly once before supervisor
/// start without rebuilding factories.
///
/// Invariants:
///
/// - The first binding wins; an injected `StewardshipTheoryBindings::dispatch`
///   preloads the slot and later bind calls are no-ops.
/// - An unbound slot leaves the dispatch handle body-less, so a boot that
///   never binds routes still projects a truthful
///   `UnresolvedRequiredBinding` — the slot never manufactures behavior.
#[derive(Clone, Default)]
pub struct DispatchRouteSlot {
    routes: Arc<Mutex<Option<DispatchRouteBindings>>>,
}

impl DispatchRouteSlot {
    fn preloaded(routes: Option<DispatchRouteBindings>) -> Self {
        Self {
            routes: Arc::new(Mutex::new(routes)),
        }
    }

    /// Bind execution routes once; returns whether this call installed them.
    pub fn bind(&self, routes: DispatchRouteBindings) -> bool {
        let mut slot = self.routes.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_some() {
            return false;
        }
        *slot = Some(routes);
        true
    }

    /// Return whether execution routes are currently bound.
    pub fn is_bound(&self) -> bool {
        self.routes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    fn current(&self) -> Option<DispatchRouteBindings> {
        self.routes
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

/// Physical identities a route composer needs to build the production
/// dispatch routes for one stewardship composition.
///
/// Everything here is identity or physical scope copied from the validated
/// physical binding — never theory bodies and never open resources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchRouteSeed {
    /// Canonical workspace root the stewarded subject lives in.
    pub workspace_root: PathBuf,
    /// Workspace-relative subject path the package route triggers on.
    pub subject_path: PathBuf,
    /// Durable agent identity that stewards the subject.
    pub agent_id: String,
    /// Provider key guaranteed present in the root provider map.
    pub provider_id: String,
    /// Event ledger session partition shared with the composed actors.
    pub session_id: String,
}

/// One stewardship composition input for product assembly.
pub struct StewardshipComposition {
    /// Validated physical binding resolved from configuration (stage 0).
    pub binding: PhysicalBinding,
    /// Injected theory and route bindings.
    pub theory: StewardshipTheoryBindings,
}

/// In-process package-route plan handoffs between planning and dispatch.
///
/// The handoff carries no durable record by design: durable dedupe lives in
/// the package progress store keyed by the run id derived from the
/// deterministic plan identity. This registry only carries plans across
/// bounded ticks inside one process; a restart reconstructs it from the
/// next planning tick, and dispatch converges on the same durable run via
/// [`package_route_run_id`].
#[derive(Default)]
pub struct PackageRouteHandoffs {
    plans: Mutex<BTreeMap<String, TaskPackageRoutePlan>>,
}

impl PackageRouteHandoffs {
    /// Record one plan handoff, deduped on its deterministic plan identity.
    pub fn record(&self, plan: TaskPackageRoutePlan) {
        let mut plans = self.plans.lock().unwrap_or_else(|e| e.into_inner());
        plans.entry(plan.plan_id.clone()).or_insert(plan);
    }

    /// Current plan handoffs in deterministic plan id order.
    pub fn plans(&self) -> Vec<TaskPackageRoutePlan> {
        let plans = self.plans.lock().unwrap_or_else(|e| e.into_inner());
        plans.values().cloned().collect()
    }

    /// Durable run bindings for the recorded plans.
    ///
    /// Every consumer derives the run id through the exported
    /// [`package_route_run_id`] so plan identity and durable package
    /// progress can never fork.
    pub fn run_bindings(&self) -> Vec<(String, String)> {
        let plans = self.plans.lock().unwrap_or_else(|e| e.into_inner());
        plans
            .keys()
            .map(|plan_id| (plan_id.clone(), package_route_run_id(plan_id)))
            .collect()
    }
}

/// Stewardship values shared by the composed actor factories.
struct ComposedStewardship {
    bindings: StewardshipActorBindings,
    theory: StewardshipTheoryBindings,
    handoffs: Arc<PackageRouteHandoffs>,
    network: Option<Arc<Mutex<SledTaskNetworkStore>>>,
    cursor_registry: EventConsumerRegistryCapability,
    worker_id: String,
    dispatch_slot: DispatchRouteSlot,
    dispatch_route_seed: DispatchRouteSeed,
}

/// Derive the production route seed from one validated physical binding.
fn dispatch_route_seed(
    binding: &PhysicalBinding,
    bindings: &StewardshipActorBindings,
) -> DispatchRouteSeed {
    DispatchRouteSeed {
        workspace_root: binding.workspace_root.clone(),
        subject_path: PathBuf::from(&binding.subject),
        agent_id: binding.agent_id.clone(),
        provider_id: binding.provider_id.clone(),
        session_id: bindings.session_id.clone(),
    }
}

/// Process-local registry of inert runtime factories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFactoryRegistry {
    descriptors: BTreeMap<String, RuntimeFactoryDescriptor>,
}

/// Registry of runtime handle factories.
pub struct RuntimeHandleFactoryRegistry {
    factories: BTreeMap<String, RuntimeHandleFactory>,
}

/// Factory for one supervised runtime handle.
pub struct RuntimeHandleFactory {
    descriptor: RuntimeFactoryDescriptor,
    semantic: RuntimeSemanticHandleFactory,
}

/// Supervised runtime handle.
///
/// The handle exposes lifecycle hooks and may own a bounded domain tick hook.
/// Construction remains passive; semantic work only runs after supervisor
/// lease acquisition and an explicit tick.
pub struct InertRuntimeHandle {
    runtime_id: String,
    required_resources: Vec<RuntimeResource>,
    started: bool,
    semantic: RuntimeSemanticHandle,
}

/// Durably monotonic per-actor step sequence.
///
/// Satisfaction reviews and assessment checkpoints require an injected
/// sequence that never regresses across supervisor restarts. The ratchet
/// persists `last + 1` through the belief store's runtime-meta surface
/// before the actor sees the value, so a crash or restart can never replay
/// a smaller sequence — wall clock resets are irrelevant.
#[derive(Clone)]
struct DurableStepSequence {
    store: Arc<BeliefStore>,
    key: String,
}

impl DurableStepSequence {
    fn new(store: Arc<BeliefStore>, runtime_id: &str) -> Self {
        Self {
            store,
            key: format!("root_step_sequence::{runtime_id}"),
        }
    }

    fn next(&self) -> Result<u64, String> {
        let last = self
            .store
            .get_runtime_meta(&self.key)
            .map_err(|error| error.to_string())?
            .and_then(|raw| raw.parse::<u64>().ok())
            .unwrap_or(0);
        let next = last.saturating_add(1);
        self.store
            .put_runtime_meta(&self.key, &next.to_string())
            .map_err(|error| error.to_string())?;
        self.store.flush().map_err(|error| error.to_string())?;
        Ok(next)
    }
}

#[derive(Clone)]
struct BeliefAssessmentFactory {
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
    registry: Arc<BeliefFamilyRegistryStore>,
    family_id: String,
    subject_binding: BeliefSubjectBinding,
    perspective: PerspectiveKey,
    branch_scope: BranchScope,
}

#[derive(Clone)]
struct EvidenceIngestionFactory {
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
    registry: Arc<BeliefFamilyRegistryStore>,
    family_id: String,
    replay: ProductEventReplayPort,
    cursor: EventConsumerRegistryCapability,
    mapping: Arc<ConfiguredOutcomeMappingSet>,
    perspective: PerspectiveKey,
    branch_scope: BranchScope,
}

/// Which bounded agent actor an [`AgentActorFactory`] builds.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AgentActorKind {
    GoalCuration,
    SatisfactionCuration,
}

#[derive(Clone)]
struct AgentActorFactory {
    kind: AgentActorKind,
    runtime_id: String,
    agent_id: String,
    agent_store: Arc<AgentStore>,
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
    goal_store: Arc<PersistentGoalSetStore>,
    goal_command: ExecutionGoalCommandPort,
    goal_mutation: ExecutionGoalMutationPort,
}

#[derive(Clone)]
struct PlanningFactory {
    theory: PlanningTheoryBinding,
    goal_store: Arc<PersistentGoalSetStore>,
    network: Arc<Mutex<SledTaskNetworkStore>>,
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
    registry: Arc<BeliefFamilyRegistryStore>,
    bindings: StewardshipActorBindings,
    handoffs: Arc<PackageRouteHandoffs>,
}

#[derive(Clone)]
struct DispatchFactory {
    /// Late-binding route slot: an unbound slot builds a body-less handle.
    routes: DispatchRouteSlot,
    execution_db: sled::Db,
    network: Arc<Mutex<SledTaskNetworkStore>>,
    handoffs: Arc<PackageRouteHandoffs>,
    worker_id: String,
}

#[derive(Clone)]
struct PublicationFactory {
    execution_db: sled::Db,
    event_append: ProductEventAppendPort,
    handoffs: Arc<PackageRouteHandoffs>,
    bindings: StewardshipActorBindings,
    worker_id: String,
    network: Option<Arc<Mutex<SledTaskNetworkStore>>>,
}

#[derive(Clone)]
enum RuntimeSemanticHandleFactory {
    None,
    GraphReplay { graph_runtime: Arc<GraphRuntime> },
    EventAppend { port: ProductEventAppendPort },
    BeliefAssessment(Box<BeliefAssessmentFactory>),
    EvidenceIngestion(Box<EvidenceIngestionFactory>),
    AgentActor(Box<AgentActorFactory>),
    Planning(Box<PlanningFactory>),
    Dispatch(Box<DispatchFactory>),
    AggregatePublication(Box<PublicationFactory>),
}

enum RuntimeSemanticHandle {
    None,
    GraphReplay(GraphReplayRuntimeHandle),
    EventAppend(EventAppendRuntimeHandle),
    BeliefAssessment(Box<BeliefAssessmentHandle>),
    EvidenceIngestion(Box<EvidenceIngestionHandle>),
    AgentActor(Box<AgentActorHandle>),
    Planning(Box<PlanningHandle>),
    Dispatch(Box<DispatchHandle>),
    AggregatePublication(Box<PublicationHandle>),
}

#[derive(Clone)]
struct GraphReplayRuntimeHandle {
    graph_runtime: Arc<GraphRuntime>,
}

/// Diagnostics-only handle publishing ledger ingress health through the
/// standard tick report path: the watermark is its checkpoint and drop
/// bursts surface as retryable issues, so heartbeats and health snapshots
/// carry ledger state without new publisher plumbing.
///
/// Compatibility: the stewardship-derived registration set declares
/// `event.append` a passive service, so this body is never leased or
/// ticked there; ledger health lives on the passive capability and the
/// self-observation watcher. The body remains only for explicit legacy
/// compositions without a registration set.
struct EventAppendRuntimeHandle {
    port: ProductEventAppendPort,
    // Baseline sampled on the first tick so a restart or an existing ledger
    // never misreports history as fresh work or fresh drops.
    last: Option<(u64, u64)>,
}

struct BeliefAssessmentHandle {
    actor: BeliefAssessmentActor,
    subject_key: String,
    sequence: DurableStepSequence,
}

struct EvidenceIngestionHandle {
    actor: EvidenceIngestionActor,
}

struct AgentActorHandle {
    kind: AgentActorKind,
    runtime_id: String,
    agent_id: String,
    curation: Option<AgentGoalCurationActor>,
    satisfaction: Option<AgentSatisfactionCurationActor>,
    goal_query: ExecutionAgentGoalQueryPort,
    goal_command: ExecutionGoalCommandPort,
    goal_mutation: ExecutionGoalMutationPort,
    sequence: DurableStepSequence,
}

struct PlanningHandle {
    actor: PlanningRuntimeActor<TaskCompiler>,
    goal_store: PersistentGoalSetStore,
    network: Arc<Mutex<SledTaskNetworkStore>>,
    projection: ExactKeyPlanningProjectionPort,
    request_seed: PlanningRequestSeed,
    handoffs: Arc<PackageRouteHandoffs>,
}

#[derive(Clone)]
struct PlanningRequestSeed {
    network_id: String,
    perspective_id: String,
    branch_id: String,
    requested_dimensions: Vec<String>,
    available_actions: AvailableActionSet,
    method_realizations: Vec<MethodRealizationBinding>,
}

struct DispatchHandle {
    actor: Option<
        DispatchRuntimeActor<
            SharedPackageRunPreparer,
            SharedPackageStepInvoker,
            SharedClaimedTaskInvoker,
        >,
    >,
    construction_error: Option<String>,
    tokio_runtime: Option<tokio::runtime::Runtime>,
    network: Arc<Mutex<SledTaskNetworkStore>>,
    handoffs: Arc<PackageRouteHandoffs>,
    sequence: u64,
}

struct PublicationHandle {
    stores: Result<(TaskProgressStore, AggregatePublicationStore), String>,
    event_append: ProductEventAppendPort,
    handoffs: Arc<PackageRouteHandoffs>,
    bindings: StewardshipActorBindings,
    worker_id: String,
    network: Option<Arc<Mutex<SledTaskNetworkStore>>>,
}

/// Lease context supplied by the supervisor before a handle starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLeaseContext {
    /// Runtime id whose lease was acquired.
    pub runtime_id: String,
    /// Non-empty supervisor lease id.
    pub lease_id: String,
}

/// Report returned when an inert handle accepts a supervisor start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandleStartReport {
    /// Runtime id that accepted the start.
    pub runtime_id: String,
}

/// Report returned when an inert handle accepts a stop request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandleStopReport {
    /// Runtime id that accepted the stop.
    pub runtime_id: String,
    /// Whether the handle was running before the stop request.
    pub was_started: bool,
}

/// Report returned when an inert handle reaches a safe point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandleSafePointReport {
    /// Runtime id that reached the safe point.
    pub runtime_id: String,
    /// Whether the handle is safe for product flush.
    pub safe_for_flush: bool,
}

/// Diagnostic snapshot from one inert runtime handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandleDiagnostic {
    /// Runtime id that produced the diagnostic.
    pub runtime_id: String,
    /// Whether the supervisor has started the handle.
    pub started: bool,
    /// Passive resources referenced by the handle.
    pub required_resources: Vec<RuntimeResource>,
}

/// Report returned by an inert handle flush hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandleFlushReport {
    /// Runtime id whose flush hook was called.
    pub runtime_id: String,
    /// Whether the hook flushed a per-resource store.
    pub flushed_resource: bool,
}

/// Supervisor lifecycle timing defaults assembled by root `meld`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLifecycleConfig {
    /// Milliseconds between expected heartbeats.
    pub heartbeat_interval_ms: u64,
    /// Milliseconds before a lease can be considered expired.
    pub lease_duration_ms: u64,
    /// Milliseconds allowed for graceful shutdown.
    pub shutdown_grace_ms: u64,
}

/// Passive process service identities handed to the supervisor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProcessServices {
    /// Clock source identifier.
    pub clock_source: String,
    /// Cancellation source identifier.
    pub cancellation_source: String,
    /// Process signal source identifier.
    pub signal_source: String,
}

/// Startup resources handed from product assembly to the supervisor.
pub struct SupervisorStartupPackage<'a> {
    /// Product root identity.
    pub product_root: &'a Path,
    /// Opened product stores.
    pub stores: &'a OpenProductStores,
    /// Opened supervisor lifecycle store.
    pub supervisor_store: &'a SupervisorStore,
    /// Direct handoff and adapter ports.
    pub ports: &'a ProductRuntimePorts,
    /// Inert runtime handle factories.
    pub handle_factories: &'a RuntimeHandleFactoryRegistry,
    /// Desired runtime state from config.
    pub desired_runtime_state: &'a [DesiredRuntimeState],
    /// Supervisor lifecycle timing defaults.
    pub lifecycle_config: &'a RuntimeLifecycleConfig,
    /// Default bounded work budget.
    pub default_work_budget: WorkBudget,
    /// Passive process service identities.
    pub process_services: &'a RuntimeProcessServices,
    /// Assembly diagnostics collected before handoff.
    pub diagnostics: &'a [AssemblyDiagnostic],
}

impl ProductRuntimeConfig {
    /// Build config for a concrete product root with default safe runtimes.
    ///
    /// Provider-dependent task dispatch is present in the registry but disabled
    /// by default until provider access is explicitly available.
    pub fn for_product_root(product_root: impl Into<PathBuf>) -> Self {
        Self {
            product_root: product_root.into(),
            supervisor_store_path: None,
            enabled_runtime_ids: Vec::new(),
            disabled_runtime_ids: vec!["execution.task_dispatch".to_string()],
            registration_set: None,
            provider: ProviderPortConfig::default(),
            lifecycle_config: RuntimeLifecycleConfig::default(),
            default_work_budget: WorkBudget { max_items: 64 },
            process_services: RuntimeProcessServices::default(),
        }
    }
}

impl Default for RuntimeLifecycleConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: 1_000,
            lease_duration_ms: 30_000,
            shutdown_grace_ms: 10_000,
        }
    }
}

impl Default for RuntimeProcessServices {
    fn default() -> Self {
        Self {
            clock_source: "system".to_string(),
            cancellation_source: "process".to_string(),
            signal_source: "process".to_string(),
        }
    }
}

impl ProductRuntimeAssembly {
    /// Describe runtime infrastructure from a workspace without opening stores.
    pub fn describe_for_workspace(
        workspace_root: &Path,
        config: &MerkleConfig,
    ) -> Result<ProductRuntimeDescription, RuntimeAssemblyError> {
        let product_root = config
            .system
            .storage
            .resolve_product_root(workspace_root)
            .map_err(|error| RuntimeAssemblyError::Config(error.to_string()))?;
        Self::describe(ProductRuntimeConfig::for_product_root(product_root))
    }

    /// Describe runtime infrastructure from explicit config without side effects.
    pub fn describe(
        config: ProductRuntimeConfig,
    ) -> Result<ProductRuntimeDescription, RuntimeAssemblyError> {
        if config.product_root.as_os_str().is_empty() {
            return Err(RuntimeAssemblyError::Config(
                "product root must not be empty".to_string(),
            ));
        }

        let product_root = ProductStorageRoot::new(config.product_root);
        let layout = product_root.layout();
        let supervisor_store_path = config
            .supervisor_store_path
            .unwrap_or_else(|| layout.root.join("supervisor.sled"));
        let registry = RuntimeFactoryRegistry::first_proof_registry()?;
        validate_runtime_selection(&config.enabled_runtime_ids)?;
        validate_runtime_selection(&config.disabled_runtime_ids)?;
        let desired_runtime_state = desired_runtime_state(
            &registry,
            config.enabled_runtime_ids,
            config.disabled_runtime_ids,
        )?;

        Ok(ProductRuntimeDescription {
            product_root: product_root.root,
            layout,
            supervisor_store_path,
            desired_runtime_state,
            lifecycle_config: config.lifecycle_config,
            default_work_budget: config.default_work_budget,
            process_services: config.process_services,
        })
    }

    /// Build assembly from a workspace and an already-resolved event authority.
    pub fn load_for_workspace_with_authority(
        workspace_root: &Path,
        config: &MerkleConfig,
        event_authority: Arc<EventAuthority>,
    ) -> Result<Self, RuntimeAssemblyError> {
        let product_root = config
            .system
            .storage
            .resolve_product_root(workspace_root)
            .map_err(|error| RuntimeAssemblyError::Config(error.to_string()))?;
        Self::load_with_authority(
            ProductRuntimeConfig::for_product_root(product_root),
            event_authority,
        )
    }

    #[cfg(test)]
    pub(crate) fn load_for_product_root(
        product_root: impl Into<PathBuf>,
    ) -> Result<Self, RuntimeAssemblyError> {
        Self::load(ProductRuntimeConfig::for_product_root(product_root))
    }

    #[cfg(test)]
    pub(crate) fn load(config: ProductRuntimeConfig) -> Result<Self, RuntimeAssemblyError> {
        if config.product_root.as_os_str().is_empty() {
            return Err(RuntimeAssemblyError::Config(
                "product root must not be empty".to_string(),
            ));
        }
        let layout = ProductStorageLayout::from_root(config.product_root.clone());
        layout.create_dirs()?;
        let db = sled::open(&layout.ledger_db)
            .map_err(|error| RuntimeAssemblyError::PortConstruction(error.to_string()))?;
        let authority = Arc::new(
            EventAuthority::open(db, EventAuthorityOpenOptions::default())
                .map_err(|error| RuntimeAssemblyError::PortConstruction(error.to_string()))?,
        );
        Self::load_with_authority(config, authority)
    }

    /// Open product stores and compose runtime around one supplied authority.
    pub fn load_with_authority(
        config: ProductRuntimeConfig,
        event_authority: Arc<EventAuthority>,
    ) -> Result<Self, RuntimeAssemblyError> {
        Self::load_composed(config, event_authority, None)
    }

    /// Compose the runtime with an optional stewardship expression binding.
    ///
    /// The composed registration set comes from, in precedence order: the
    /// explicit set on `config` (harness and proof callers), then the
    /// stewardship-derived set, else none (legacy conservative
    /// classification). Store and port opening is scoped to the composed
    /// set; without one, everything opens.
    pub fn load_composed(
        config: ProductRuntimeConfig,
        event_authority: Arc<EventAuthority>,
        stewardship: Option<StewardshipComposition>,
    ) -> Result<Self, RuntimeAssemblyError> {
        if config.product_root.as_os_str().is_empty() {
            return Err(RuntimeAssemblyError::Config(
                "product root must not be empty".to_string(),
            ));
        }

        let product_root = ProductStorageRoot::new(config.product_root);
        let layout = product_root.layout();
        let registry = RuntimeFactoryRegistry::first_proof_registry()?;
        validate_runtime_selection(&config.enabled_runtime_ids)?;
        validate_runtime_selection(&config.disabled_runtime_ids)?;

        let registration_set = match (&config.registration_set, &stewardship) {
            (Some(explicit), _) => Some(explicit.clone()),
            (None, Some(composition)) => {
                Some(derive_stewardship_registrations(&composition.binding)?)
            }
            (None, None) => None,
        };
        let scope = registration_set
            .as_ref()
            .map(|set| scope_for_registration_set(set, &registry))
            .unwrap_or_else(StoreScope::all);

        let stores = Arc::new(OpenProductStores::open_scoped(&layout, &scope)?);
        let supervisor_store_path = config
            .supervisor_store_path
            .unwrap_or_else(|| layout.root.join("supervisor.sled"));
        let desired_runtime_state = desired_runtime_state(
            &registry,
            config.enabled_runtime_ids,
            config.disabled_runtime_ids,
        )?;
        let mut provider = config.provider;
        provider.provider_required = provider_required(&registry, &desired_runtime_state);
        let supervisor_store = SupervisorStore::open(supervisor_store_path)?;
        let ports = ProductRuntimePorts::from_authority(
            stores.as_ref(),
            event_authority.as_ref(),
            provider,
        )?;
        let graph_runtime = match stores.traversal_store.opened() {
            Some(traversal_store) => Some(Arc::new(
                GraphRuntime::from_ports(
                    Arc::new(ports.event_replay().clone()),
                    Arc::new(ports.event_append().clone()),
                    Arc::new(ports.graph_cursor().clone()),
                    Arc::clone(traversal_store),
                )
                .map_err(|error| {
                    RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                })?,
            )),
            None => None,
        };

        let mut diagnostics = Vec::new();
        let composed_stewardship = match stewardship {
            Some(composition) => {
                let bindings = StewardshipActorBindings::derive(&composition.binding)?;
                let network = match stores.task_networks.opened() {
                    Some(factory) => Some(Arc::new(Mutex::new(
                        factory
                            .open_network(&bindings.network_id)
                            .map_err(|error| {
                                RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                            })?,
                    ))),
                    None => None,
                };
                Some(ComposedStewardship {
                    dispatch_slot: DispatchRouteSlot::preloaded(
                        composition.theory.dispatch.clone(),
                    ),
                    handoffs: Arc::new(PackageRouteHandoffs::default()),
                    network,
                    cursor_registry: event_authority.consumer_registry_capability(),
                    // Worker identity derives from the durable ledger
                    // identity, never process-random state, so interrupted
                    // claims are resumable across supervisor restarts.
                    worker_id: format!("runtime-worker::{}", event_authority.ledger_identity()),
                    theory: composition.theory,
                    dispatch_route_seed: dispatch_route_seed(&composition.binding, &bindings),
                    bindings,
                })
            }
            None => None,
        };

        let handle_factories = RuntimeHandleFactoryRegistry::from_registry(
            &registry,
            &ports,
            &stores,
            graph_runtime.as_ref(),
            composed_stewardship.as_ref(),
            &mut diagnostics,
        )?;
        let dispatch_route_slot = composed_stewardship
            .as_ref()
            .map(|composed| composed.dispatch_slot.clone());
        let dispatch_route_seed = composed_stewardship
            .as_ref()
            .map(|composed| composed.dispatch_route_seed.clone());

        Ok(Self {
            product_root,
            layout,
            stores,
            event_authority,
            graph_runtime,
            supervisor_store,
            ports,
            registry,
            handle_factories,
            registration_set,
            desired_runtime_state,
            lifecycle_config: config.lifecycle_config,
            default_work_budget: config.default_work_budget,
            process_services: config.process_services,
            diagnostics,
            dispatch_route_slot,
            dispatch_route_seed,
        })
    }

    /// Return the resolved product root.
    pub fn product_root(&self) -> &Path {
        &self.product_root.root
    }

    /// Return the concrete product storage layout.
    pub fn layout(&self) -> &ProductStorageLayout {
        &self.layout
    }

    /// Return opened product stores.
    pub fn stores(&self) -> &OpenProductStores {
        self.stores.as_ref()
    }

    /// Return the single event authority supplied by product binding resolution.
    pub fn event_authority(&self) -> Arc<EventAuthority> {
        Arc::clone(&self.event_authority)
    }

    /// Return the graph runtime shared by direct catch-up and supervisor handles.
    ///
    /// Panics when the composed registration scope excludes the world model
    /// store group; scoped callers use [`Self::try_graph_runtime`].
    pub fn graph_runtime(&self) -> Arc<GraphRuntime> {
        self.try_graph_runtime()
            .expect("graph runtime requires the world model store scope")
    }

    /// Return the graph runtime when the world model scope is open.
    pub fn try_graph_runtime(&self) -> Option<Arc<GraphRuntime>> {
        self.graph_runtime.as_ref().map(Arc::clone)
    }

    /// Return supervisor lifecycle storage.
    pub fn supervisor_store(&self) -> &SupervisorStore {
        &self.supervisor_store
    }

    /// Return direct handoff and adapter ports.
    pub fn ports(&self) -> &ProductRuntimePorts {
        &self.ports
    }

    /// Return the inert runtime factory registry.
    pub fn registry(&self) -> &RuntimeFactoryRegistry {
        &self.registry
    }

    /// Return inert runtime handle factories.
    pub fn handle_factories(&self) -> &RuntimeHandleFactoryRegistry {
        &self.handle_factories
    }

    /// Return the registration set composing this assembly, when one exists.
    ///
    /// The caller passes this set to the supervisor through
    /// `SupervisorStartCommand::registration_set` so classification follows
    /// the composed declaration rather than the conservative seam.
    pub fn registration_set(&self) -> Option<&RegistrationSet> {
        self.registration_set.as_ref()
    }

    /// Return desired runtime states prepared for supervisor handoff.
    pub fn desired_runtime_state(&self) -> &[DesiredRuntimeState] {
        &self.desired_runtime_state
    }

    /// Return the production route seed for this stewardship composition.
    ///
    /// `None` means no stewardship expression composed this assembly, so
    /// there is no dispatch route to build.
    pub fn dispatch_route_seed(&self) -> Option<&DispatchRouteSeed> {
        self.dispatch_route_seed.as_ref()
    }

    /// Return whether execution routes are bound for the dispatch actor.
    ///
    /// `false` covers both an unbound slot and a composition with no
    /// stewardship expression at all.
    pub fn dispatch_routes_bound(&self) -> bool {
        self.dispatch_route_slot
            .as_ref()
            .is_some_and(DispatchRouteSlot::is_bound)
    }

    /// Bind production execution routes for the dispatch actor.
    ///
    /// Returns whether this call installed the routes: `false` means either
    /// no stewardship composition carries a dispatch slot or routes were
    /// already bound (injected theory bindings win). Binding after
    /// supervisor start has no effect on already-built handles; callers bind
    /// before starting the supervisor.
    pub fn bind_dispatch_routes(&self, routes: DispatchRouteBindings) -> bool {
        self.dispatch_route_slot
            .as_ref()
            .is_some_and(|slot| slot.bind(routes))
    }

    /// Return the currently bound dispatch execution routes, when any.
    pub fn dispatch_routes(&self) -> Option<DispatchRouteBindings> {
        self.dispatch_route_slot
            .as_ref()
            .and_then(DispatchRouteSlot::current)
    }

    /// Return assembly diagnostics captured before supervisor handoff.
    pub fn diagnostics(&self) -> &[AssemblyDiagnostic] {
        &self.diagnostics
    }

    /// Build the passive startup package handed to the supervisor.
    pub fn supervisor_startup_package(&self) -> SupervisorStartupPackage<'_> {
        SupervisorStartupPackage {
            product_root: self.product_root(),
            stores: self.stores(),
            supervisor_store: self.supervisor_store(),
            ports: self.ports(),
            handle_factories: self.handle_factories(),
            desired_runtime_state: self.desired_runtime_state(),
            lifecycle_config: &self.lifecycle_config,
            default_work_budget: self.default_work_budget.clone(),
            process_services: &self.process_services,
            diagnostics: self.diagnostics(),
        }
    }

    /// Return the default bounded work budget.
    pub fn default_work_budget(&self) -> &WorkBudget {
        &self.default_work_budget
    }

    /// Flush product stores through the assembly checkpoint boundary.
    ///
    /// This reports storage durability only. It does not mark semantic
    /// convergence, repair domain records, or perform supervisor lease work.
    pub fn flush_product_boundary(&self) -> Result<(), RuntimeAssemblyError> {
        self.stores.flush_boundary()?;
        Ok(())
    }

    /// Flush supervisor lifecycle storage separately from product stores.
    pub fn flush_supervisor_store(&self) -> Result<(), RuntimeAssemblyError> {
        self.supervisor_store.flush()?;
        Ok(())
    }
}

impl RuntimeFactoryDescriptor {
    /// Construct one inert runtime factory descriptor.
    pub fn new(
        runtime_id: impl Into<String>,
        required_resources: Vec<RuntimeResource>,
    ) -> Result<Self, RuntimeRegistryError> {
        let runtime_id = runtime_id.into();
        validate_runtime_id(&runtime_id)?;
        Ok(Self {
            runtime_id,
            required_resources,
        })
    }
}

impl RuntimeFactoryRegistry {
    /// Build a registry from descriptors, rejecting duplicates and invalid ids.
    pub fn from_descriptors(
        descriptors: impl IntoIterator<Item = RuntimeFactoryDescriptor>,
    ) -> Result<Self, RuntimeRegistryError> {
        let mut registry = BTreeMap::new();
        for descriptor in descriptors {
            validate_runtime_id(&descriptor.runtime_id)?;
            if registry.contains_key(&descriptor.runtime_id) {
                return Err(RuntimeRegistryError::DuplicateRuntimeId(
                    descriptor.runtime_id,
                ));
            }
            registry.insert(descriptor.runtime_id.clone(), descriptor);
        }
        Ok(Self {
            descriptors: registry,
        })
    }

    /// Build the first durable flywheel proof runtime registry.
    pub fn first_proof_registry() -> Result<Self, RuntimeRegistryError> {
        use RuntimeResource::*;
        Self::from_descriptors([
            RuntimeFactoryDescriptor::new("event.append", vec![EventAppend])?,
            RuntimeFactoryDescriptor::new("event.replay", vec![EventReplay])?,
            RuntimeFactoryDescriptor::new(
                "world_model.graph_replay",
                vec![EventAppend, EventReplay, EventConsumerRegistry, WorldModel],
            )?,
            RuntimeFactoryDescriptor::new("world_model.belief_assessment", vec![WorldModel])?,
            RuntimeFactoryDescriptor::new(
                "world_model.agent_goal_curation",
                vec![GoalCommand, GoalMutation, PlannerProjection, WorldModel],
            )?,
            RuntimeFactoryDescriptor::new(
                "world_model.evidence_ingestion",
                vec![EventReplay, EventConsumerRegistry, WorldModel],
            )?,
            RuntimeFactoryDescriptor::new(
                "world_model.satisfaction_curation",
                vec![GoalCommand, GoalMutation, PlannerProjection, WorldModel],
            )?,
            RuntimeFactoryDescriptor::new("execution.goal_set", vec![GoalCommand])?,
            RuntimeFactoryDescriptor::new(
                "execution.planning",
                vec![
                    GoalCommand,
                    PlannerProjection,
                    TaskNetworkFactory,
                    WorldModel,
                ],
            )?,
            RuntimeFactoryDescriptor::new(
                "execution.task_network_command",
                vec![TaskNetworkFactory],
            )?,
            RuntimeFactoryDescriptor::new(
                "execution.task_dispatch",
                vec![
                    TaskNetworkFactory,
                    TaskArtifactFactory,
                    Context,
                    Provider,
                    Prompt,
                    Workspace,
                ],
            )?,
            RuntimeFactoryDescriptor::new(
                "execution.publication",
                vec![TaskNetworkFactory, TaskArtifactFactory, EventAppend],
            )?,
        ])
    }

    /// Return one descriptor by runtime id.
    pub fn get(&self, runtime_id: &str) -> Option<&RuntimeFactoryDescriptor> {
        self.descriptors.get(runtime_id)
    }

    /// Return descriptors in stable runtime id order.
    pub fn descriptors(&self) -> impl Iterator<Item = &RuntimeFactoryDescriptor> {
        self.descriptors.values()
    }

    /// Return whether the registry has a runtime id.
    pub fn contains(&self, runtime_id: &str) -> bool {
        self.descriptors.contains_key(runtime_id)
    }

    /// Return the number of registered runtime factories.
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    /// Return whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }
}

impl RuntimeFactoryDescriptor {
    fn requires_resource(&self, resource: RuntimeResource) -> bool {
        self.required_resources.contains(&resource)
    }
}

impl RuntimeHandleFactoryRegistry {
    /// Build inert handle factories from the runtime factory registry.
    ///
    /// Stage 5 hydration: each stewardship actor factory probes the durable
    /// state it requires (installed theory revisions, genesis identities,
    /// injected route bindings). A probe miss produces a body-less factory
    /// — a truthful unresolved required binding — plus a diagnostic; it
    /// never manufactures the missing state.
    fn from_registry(
        registry: &RuntimeFactoryRegistry,
        ports: &ProductRuntimePorts,
        stores: &OpenProductStores,
        graph_runtime: Option<&Arc<GraphRuntime>>,
        stewardship: Option<&ComposedStewardship>,
        diagnostics: &mut Vec<AssemblyDiagnostic>,
    ) -> Result<Self, RuntimeAssemblyError> {
        let factories = registry
            .descriptors()
            .map(|descriptor| {
                Ok((
                    descriptor.runtime_id.clone(),
                    RuntimeHandleFactory {
                        descriptor: descriptor.clone(),
                        semantic: RuntimeSemanticHandleFactory::for_descriptor(
                            descriptor,
                            ports,
                            stores,
                            graph_runtime,
                            stewardship,
                            diagnostics,
                        )?,
                    },
                ))
            })
            .collect::<Result<_, RuntimeAssemblyError>>()?;
        Ok(Self { factories })
    }

    /// Return one inert handle factory by runtime id.
    pub fn get(&self, runtime_id: &str) -> Option<&RuntimeHandleFactory> {
        self.factories.get(runtime_id)
    }

    /// Return the number of inert handle factories.
    pub fn len(&self) -> usize {
        self.factories.len()
    }

    /// Return whether the factory registry is empty.
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }
}

impl RuntimeHandleFactory {
    /// Return this factory's stable runtime id.
    pub fn runtime_id(&self) -> &str {
        &self.descriptor.runtime_id
    }

    /// Return whether built handles carry a concrete semantic body.
    ///
    /// Catalog-only descriptors build body-less handles; the supervisor uses
    /// this distinction to classify active actors truthfully instead of
    /// leasing and health-reporting inert placeholders. Dispatch is dynamic:
    /// its body exists only while its execution route slot is bound, so the
    /// classification observed at supervisor start stays truthful for boots
    /// that never compose routes.
    pub fn has_semantic_body(&self) -> bool {
        self.semantic.resolved()
    }

    /// Build an inert runtime handle.
    pub fn build_handle(&self) -> InertRuntimeHandle {
        InertRuntimeHandle {
            runtime_id: self.descriptor.runtime_id.clone(),
            required_resources: self.descriptor.required_resources.clone(),
            started: false,
            semantic: self.semantic.build_handle(),
        }
    }
}

impl InertRuntimeHandle {
    /// Return this handle's runtime id.
    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    /// Return passive resources required by this runtime handle.
    pub fn required_resources(&self) -> &[RuntimeResource] {
        &self.required_resources
    }

    /// Return whether the supervisor has started this handle.
    pub fn is_started(&self) -> bool {
        self.started
    }

    /// Return whether this handle carries a concrete semantic body.
    ///
    /// A body-less handle can never produce a tick report and must never be
    /// projected as a healthy active actor.
    pub fn has_semantic_body(&self) -> bool {
        !matches!(self.semantic, RuntimeSemanticHandle::None)
    }

    /// Run one bounded semantic tick when this handle owns concrete work.
    pub fn tick(&mut self, budget: WorkBudget) -> Option<WorkerTickReport> {
        if !self.started {
            return None;
        }
        self.semantic.tick(budget)
    }

    /// Return a supervisor-facing diagnostic snapshot.
    pub fn diagnostic_report(&self) -> RuntimeHandleDiagnostic {
        RuntimeHandleDiagnostic {
            runtime_id: self.runtime_id.clone(),
            started: self.started,
            required_resources: self.required_resources.clone(),
        }
    }

    /// Flush per-handle resources when a concrete handle owns any.
    pub fn flush_resources(&self) -> Result<RuntimeHandleFlushReport, RuntimeAssemblyError> {
        Ok(RuntimeHandleFlushReport {
            runtime_id: self.runtime_id.clone(),
            flushed_resource: false,
        })
    }

    /// Request that an inert handle stop at its next safe point.
    pub fn request_stop(&mut self) -> RuntimeHandleStopReport {
        let was_started = self.started;
        self.started = false;
        self.semantic.request_stop();
        RuntimeHandleStopReport {
            runtime_id: self.runtime_id.clone(),
            was_started,
        }
    }

    /// Wait for an inert handle safe point.
    pub fn wait_for_safe_point(&self) -> RuntimeHandleSafePointReport {
        RuntimeHandleSafePointReport {
            runtime_id: self.runtime_id.clone(),
            safe_for_flush: !self.started,
        }
    }

    /// Start only after the supervisor supplies a matching non-empty lease.
    pub fn start_after_lease(
        &mut self,
        lease: RuntimeLeaseContext,
    ) -> Result<RuntimeHandleStartReport, RuntimeAssemblyError> {
        if lease.runtime_id != self.runtime_id {
            return Err(RuntimeAssemblyError::SupervisorHandoff(format!(
                "lease runtime id '{}' does not match handle '{}'",
                lease.runtime_id, self.runtime_id
            )));
        }
        if lease.lease_id.trim().is_empty() {
            return Err(RuntimeAssemblyError::SupervisorHandoff(
                "lease id must be non-empty".to_string(),
            ));
        }
        self.started = true;
        Ok(RuntimeHandleStartReport {
            runtime_id: self.runtime_id.clone(),
        })
    }
}

impl RuntimeSemanticHandleFactory {
    /// Bind the semantic factory for one catalog descriptor.
    ///
    /// The `None` outcomes are truthful unresolved bindings; each one is
    /// paired with a diagnostic naming the missing durable state.
    fn for_descriptor(
        descriptor: &RuntimeFactoryDescriptor,
        ports: &ProductRuntimePorts,
        stores: &OpenProductStores,
        graph_runtime: Option<&Arc<GraphRuntime>>,
        stewardship: Option<&ComposedStewardship>,
        diagnostics: &mut Vec<AssemblyDiagnostic>,
    ) -> Result<Self, RuntimeAssemblyError> {
        let runtime_id = descriptor.runtime_id.as_str();
        fn unresolved(
            diagnostics: &mut Vec<AssemblyDiagnostic>,
            code: &str,
            message: String,
        ) -> Result<RuntimeSemanticHandleFactory, RuntimeAssemblyError> {
            diagnostics.push(AssemblyDiagnostic {
                code: code.to_string(),
                message,
            });
            Ok(RuntimeSemanticHandleFactory::None)
        }
        match runtime_id {
            "world_model.graph_replay" => match graph_runtime {
                Some(graph_runtime) => Ok(Self::GraphReplay {
                    graph_runtime: Arc::clone(graph_runtime),
                }),
                None => unresolved(
                    diagnostics,
                    "graph_replay_unresolved",
                    "graph replay requires the world model store scope".to_string(),
                ),
            },
            "event.append" => Ok(Self::EventAppend {
                port: ports.event_append().clone(),
            }),
            "world_model.belief_assessment" => {
                let Some(composed) = stewardship else {
                    return Ok(Self::None);
                };
                let (Some(belief), Some(traversal), Some(registry)) = (
                    stores.belief_store.opened(),
                    stores.traversal_store.opened(),
                    stores.belief_family_registry.opened(),
                ) else {
                    return Ok(Self::None);
                };
                let family_id = composed.bindings.belief_family_id.clone();
                if !family_installed(registry, &family_id, runtime_id, diagnostics) {
                    return Ok(Self::None);
                }
                Ok(Self::BeliefAssessment(Box::new(BeliefAssessmentFactory {
                    belief_store: Arc::clone(belief),
                    traversal_store: Arc::clone(traversal),
                    registry: Arc::clone(registry),
                    family_id,
                    subject_binding: BeliefSubjectBinding {
                        subject: composed.bindings.subject.clone(),
                        anchor_perspective_kind: composed.bindings.anchor_perspective_kind.clone(),
                        anchor_perspective_id: composed.bindings.anchor_perspective_id.clone(),
                    },
                    perspective: composed.bindings.perspective.clone(),
                    branch_scope: composed.bindings.branch_scope.clone(),
                })))
            }
            "world_model.evidence_ingestion" => {
                let Some(composed) = stewardship else {
                    return Ok(Self::None);
                };
                let (Some(belief), Some(traversal), Some(registry)) = (
                    stores.belief_store.opened(),
                    stores.traversal_store.opened(),
                    stores.belief_family_registry.opened(),
                ) else {
                    return Ok(Self::None);
                };
                let family_id = composed.bindings.belief_family_id.clone();
                if !family_installed(registry, &family_id, runtime_id, diagnostics) {
                    return Ok(Self::None);
                }
                let Some(mapping_config) = composed.theory.outcome_mapping.clone() else {
                    return unresolved(
                        diagnostics,
                        "evidence_mapping_unresolved",
                        format!(
                            "evidence mapping '{}' has no installed configuration; \
                             evidence ingestion stays unresolved",
                            composed.bindings.evidence_mapping_id
                        ),
                    );
                };
                let mapping = match ConfiguredOutcomeMappingSet::new(mapping_config) {
                    Ok(mapping) => Arc::new(mapping),
                    Err(error) => {
                        return unresolved(
                            diagnostics,
                            "evidence_mapping_invalid",
                            format!("installed outcome mapping is invalid: {error}"),
                        )
                    }
                };
                if mapping.mapping_id() != composed.bindings.evidence_mapping_id {
                    // The actor still binds the installed identity — the
                    // mapping id it ingests under always comes from the
                    // mapping itself — but the drift is surfaced.
                    diagnostics.push(AssemblyDiagnostic {
                        code: "evidence_mapping_selection_drift".to_string(),
                        message: format!(
                            "selection names mapping '{}' but the installed mapping is '{}'",
                            composed.bindings.evidence_mapping_id,
                            mapping.mapping_id()
                        ),
                    });
                }
                Ok(Self::EvidenceIngestion(Box::new(
                    EvidenceIngestionFactory {
                        belief_store: Arc::clone(belief),
                        traversal_store: Arc::clone(traversal),
                        registry: Arc::clone(registry),
                        family_id,
                        replay: ports.event_replay().clone(),
                        cursor: composed.cursor_registry.clone(),
                        mapping,
                        perspective: composed.bindings.perspective.clone(),
                        branch_scope: composed.bindings.branch_scope.clone(),
                    },
                )))
            }
            "world_model.agent_goal_curation" | "world_model.satisfaction_curation" => {
                let Some(composed) = stewardship else {
                    return Ok(Self::None);
                };
                let (Some(belief), Some(traversal), Some(agent_store), Some(goal_store)) = (
                    stores.belief_store.opened(),
                    stores.traversal_store.opened(),
                    stores.agent_store.opened(),
                    stores.goal_store.opened(),
                ) else {
                    return Ok(Self::None);
                };
                let (Some(goal_command), Some(goal_mutation)) =
                    (ports.try_goal_command(), ports.try_goal_mutation())
                else {
                    return Ok(Self::None);
                };
                // Stage 5 hydration probe: activation loads durable agent
                // identities and never creates them. A missing genesis
                // identity is a truthful unresolved required binding.
                let agent_id = composed.bindings.agent_id.clone();
                match agent_store.get_agent(&agent_id) {
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        return unresolved(
                            diagnostics,
                            "agent_identity_unresolved",
                            format!(
                                "agent '{agent_id}' has no durable genesis record; \
                                 '{runtime_id}' stays unresolved"
                            ),
                        )
                    }
                    Err(error) => {
                        return unresolved(
                            diagnostics,
                            "agent_probe_failed",
                            format!("agent record probe failed: {error}"),
                        )
                    }
                }
                let kind = if runtime_id == "world_model.agent_goal_curation" {
                    AgentActorKind::GoalCuration
                } else {
                    AgentActorKind::SatisfactionCuration
                };
                Ok(Self::AgentActor(Box::new(AgentActorFactory {
                    kind,
                    runtime_id: runtime_id.to_string(),
                    agent_id,
                    agent_store: Arc::clone(agent_store),
                    belief_store: Arc::clone(belief),
                    traversal_store: Arc::clone(traversal),
                    goal_store: Arc::clone(goal_store),
                    goal_command: goal_command.clone(),
                    goal_mutation: goal_mutation.clone(),
                })))
            }
            "execution.planning" => {
                let Some(composed) = stewardship else {
                    return Ok(Self::None);
                };
                let Some(theory) = composed.theory.planning.clone() else {
                    return unresolved(
                        diagnostics,
                        "planning_theory_unresolved",
                        "no planning theory is installed; execution planning stays unresolved"
                            .to_string(),
                    );
                };
                let (Some(belief), Some(traversal), Some(registry), Some(goal_store)) = (
                    stores.belief_store.opened(),
                    stores.traversal_store.opened(),
                    stores.belief_family_registry.opened(),
                    stores.goal_store.opened(),
                ) else {
                    return Ok(Self::None);
                };
                let Some(network) = composed.network.as_ref() else {
                    return Ok(Self::None);
                };
                Ok(Self::Planning(Box::new(PlanningFactory {
                    theory,
                    goal_store: Arc::clone(goal_store),
                    network: Arc::clone(network),
                    belief_store: Arc::clone(belief),
                    traversal_store: Arc::clone(traversal),
                    registry: Arc::clone(registry),
                    bindings: composed.bindings.clone(),
                    handoffs: Arc::clone(&composed.handoffs),
                })))
            }
            "execution.task_dispatch" => {
                let Some(composed) = stewardship else {
                    return Ok(Self::None);
                };
                let (Some(execution_db), Some(network)) =
                    (stores.execution_db.opened(), composed.network.as_ref())
                else {
                    return Ok(Self::None);
                };
                // The factory carries the shared route slot: an injected
                // theory binding preloads it and the CLI composition may
                // bind the production routes before supervisor start. An
                // unbound slot stays a truthful unresolved required binding.
                if !composed.dispatch_slot.is_bound() {
                    diagnostics.push(AssemblyDiagnostic {
                        code: "dispatch_route_unresolved".to_string(),
                        message: "no execution route bindings are composed; task dispatch \
                                  stays unresolved until routes are bound"
                            .to_string(),
                    });
                }
                Ok(Self::Dispatch(Box::new(DispatchFactory {
                    routes: composed.dispatch_slot.clone(),
                    execution_db: execution_db.clone(),
                    network: Arc::clone(network),
                    handoffs: Arc::clone(&composed.handoffs),
                    worker_id: composed.worker_id.clone(),
                })))
            }
            "execution.publication" => {
                let Some(composed) = stewardship else {
                    return Ok(Self::None);
                };
                let Some(execution_db) = stores.execution_db.opened() else {
                    return Ok(Self::None);
                };
                Ok(Self::AggregatePublication(Box::new(PublicationFactory {
                    execution_db: execution_db.clone(),
                    event_append: ports.event_append().clone(),
                    handoffs: Arc::clone(&composed.handoffs),
                    bindings: composed.bindings.clone(),
                    worker_id: composed.worker_id.clone(),
                    network: composed.network.clone(),
                })))
            }
            _ => Ok(Self::None),
        }
    }

    /// Return whether this factory currently resolves a semantic body.
    fn resolved(&self) -> bool {
        match self {
            Self::None => false,
            // The dispatch body exists only while execution routes are
            // bound; an unbound slot is an unresolved required binding.
            Self::Dispatch(factory) => factory.routes.is_bound(),
            _ => true,
        }
    }

    fn build_handle(&self) -> RuntimeSemanticHandle {
        match self {
            Self::None => RuntimeSemanticHandle::None,
            Self::GraphReplay { graph_runtime } => {
                RuntimeSemanticHandle::GraphReplay(GraphReplayRuntimeHandle {
                    graph_runtime: Arc::clone(graph_runtime),
                })
            }
            Self::EventAppend { port } => {
                RuntimeSemanticHandle::EventAppend(EventAppendRuntimeHandle {
                    port: port.clone(),
                    last: None,
                })
            }
            Self::BeliefAssessment(factory) => {
                RuntimeSemanticHandle::BeliefAssessment(Box::new(BeliefAssessmentHandle {
                    actor: BeliefAssessmentActor::new(
                        "world_model.belief_assessment",
                        Arc::clone(&factory.belief_store),
                        Arc::clone(&factory.traversal_store),
                        Arc::clone(&factory.registry)
                            as Arc<dyn BeliefFamilyRegistry + Send + Sync>,
                        vec![factory.family_id.clone()],
                        vec![factory.subject_binding.clone()],
                        factory.perspective.clone(),
                        factory.branch_scope.clone(),
                    ),
                    subject_key: factory.subject_binding.subject.index_key(),
                    sequence: DurableStepSequence::new(
                        Arc::clone(&factory.belief_store),
                        "world_model.belief_assessment",
                    ),
                }))
            }
            Self::EvidenceIngestion(factory) => {
                RuntimeSemanticHandle::EvidenceIngestion(Box::new(EvidenceIngestionHandle {
                    actor: EvidenceIngestionActor::new(
                        "world_model.evidence_ingestion",
                        Arc::clone(&factory.belief_store),
                        Arc::clone(&factory.traversal_store),
                        Arc::clone(&factory.registry)
                            as Arc<dyn BeliefFamilyRegistry + Send + Sync>,
                        factory.family_id.clone(),
                        Arc::new(factory.replay.clone()) as Arc<dyn EvidenceEventReplaySource>,
                        Arc::new(factory.cursor.clone())
                            as Arc<dyn DurableConsumerCursor + Send + Sync>,
                        Arc::clone(&factory.mapping)
                            as Arc<dyn OutcomeEvidenceMapping + Send + Sync>,
                        // The ingestion mapping id always derives from the
                        // installed mapping itself, so a selected-versus-
                        // installed mismatch is unconstructible here.
                        factory.mapping.mapping_id(),
                        factory.perspective.clone(),
                        factory.branch_scope.clone(),
                    ),
                }))
            }
            Self::AgentActor(factory) => {
                let (curation, satisfaction) = match factory.kind {
                    AgentActorKind::GoalCuration => (
                        Some(AgentGoalCurationActor::new(
                            factory.runtime_id.clone(),
                            factory.agent_id.clone(),
                            Arc::clone(&factory.agent_store),
                            Arc::clone(&factory.belief_store),
                            Arc::clone(&factory.traversal_store),
                        )),
                        None,
                    ),
                    AgentActorKind::SatisfactionCuration => (
                        None,
                        Some(AgentSatisfactionCurationActor::new(
                            factory.runtime_id.clone(),
                            factory.agent_id.clone(),
                            Arc::clone(&factory.agent_store),
                            Arc::clone(&factory.belief_store),
                            Arc::clone(&factory.traversal_store),
                        )),
                    ),
                };
                RuntimeSemanticHandle::AgentActor(Box::new(AgentActorHandle {
                    kind: factory.kind,
                    runtime_id: factory.runtime_id.clone(),
                    agent_id: factory.agent_id.clone(),
                    curation,
                    satisfaction,
                    goal_query: ExecutionAgentGoalQueryPort::new(Arc::clone(&factory.goal_store)),
                    goal_command: factory.goal_command.clone(),
                    goal_mutation: factory.goal_mutation.clone(),
                    sequence: DurableStepSequence::new(
                        Arc::clone(&factory.belief_store),
                        &factory.runtime_id,
                    ),
                }))
            }
            Self::Planning(factory) => RuntimeSemanticHandle::Planning(Box::new(PlanningHandle {
                actor: PlanningRuntimeActor::new(
                    PlanningRuntime::new(
                        MethodLibrary::from_methods(
                            factory.theory.methods.clone(),
                            &factory.theory.capability_catalog,
                        ),
                        factory.theory.capability_catalog.clone(),
                    ),
                    ExecutionCompositionLowerer::new(
                        TaskCompiler::new(),
                        factory.theory.capability_catalog.clone(),
                    ),
                ),
                goal_store: factory.goal_store.as_ref().clone(),
                network: Arc::clone(&factory.network),
                projection: ExactKeyPlanningProjectionPort::new(
                    Arc::clone(&factory.belief_store),
                    Arc::clone(&factory.traversal_store),
                    Arc::clone(&factory.registry),
                    factory.bindings.belief_family_id.clone(),
                    factory.bindings.subject.clone(),
                    factory.bindings.perspective.clone(),
                    factory.bindings.branch_scope.clone(),
                ),
                request_seed: PlanningRequestSeed {
                    network_id: factory.bindings.network_id.clone(),
                    perspective_id: factory.bindings.perspective.perspective_id.clone(),
                    branch_id: factory.bindings.branch_scope.branch_id.clone(),
                    requested_dimensions: factory.theory.requested_dimensions.clone(),
                    available_actions: factory.theory.available_actions.clone(),
                    method_realizations: factory.theory.method_realizations.clone(),
                },
                handoffs: Arc::clone(&factory.handoffs),
            })),
            Self::Dispatch(factory) => {
                // Body-less when routes are unbound: the supervisor projects
                // an unresolved required binding, never a healthy placeholder.
                let Some(routes) = factory.routes.current() else {
                    return RuntimeSemanticHandle::None;
                };
                let (actor, construction_error) = match DispatchRuntimeActor::new(
                    factory.worker_id.clone(),
                    factory.execution_db.clone(),
                    routes.preparer,
                    routes.package_invoker,
                    routes.claim_invoker,
                ) {
                    Ok(actor) => (Some(actor), None),
                    Err(error) => (None, Some(error.to_string())),
                };
                let tokio_runtime = tokio::runtime::Runtime::new().ok();
                RuntimeSemanticHandle::Dispatch(Box::new(DispatchHandle {
                    actor,
                    construction_error,
                    tokio_runtime,
                    network: Arc::clone(&factory.network),
                    handoffs: Arc::clone(&factory.handoffs),
                    sequence: 0,
                }))
            }
            Self::AggregatePublication(factory) => {
                let stores = TaskProgressStore::open(factory.execution_db.clone())
                    .map_err(|error| error.to_string())
                    .and_then(|progress| {
                        AggregatePublicationStore::open(factory.execution_db.clone())
                            .map(|outbox| (progress, outbox))
                            .map_err(|error| error.to_string())
                    });
                RuntimeSemanticHandle::AggregatePublication(Box::new(PublicationHandle {
                    stores,
                    event_append: factory.event_append.clone(),
                    handoffs: Arc::clone(&factory.handoffs),
                    bindings: factory.bindings.clone(),
                    worker_id: factory.worker_id.clone(),
                    network: factory.network.clone(),
                }))
            }
        }
    }
}

/// Probe the durable registry for one installed family revision.
fn family_installed(
    registry: &Arc<BeliefFamilyRegistryStore>,
    family_id: &str,
    runtime_id: &str,
    diagnostics: &mut Vec<AssemblyDiagnostic>,
) -> bool {
    match registry.current(family_id) {
        Ok(Some(_)) => true,
        Ok(None) => {
            diagnostics.push(AssemblyDiagnostic {
                code: "belief_family_unresolved".to_string(),
                message: format!(
                    "belief family '{family_id}' has no installed registry revision; \
                     '{runtime_id}' stays unresolved"
                ),
            });
            false
        }
        Err(error) => {
            diagnostics.push(AssemblyDiagnostic {
                code: "belief_family_probe_failed".to_string(),
                message: format!("belief family registry probe failed: {error}"),
            });
            false
        }
    }
}

impl RuntimeSemanticHandle {
    fn tick(&mut self, budget: WorkBudget) -> Option<WorkerTickReport> {
        match self {
            Self::None => None,
            Self::GraphReplay(handle) => Some(handle.tick(budget)),
            Self::EventAppend(handle) => Some(handle.tick()),
            Self::BeliefAssessment(handle) => Some(handle.tick(budget)),
            Self::EvidenceIngestion(handle) => Some(handle.tick(budget)),
            Self::AgentActor(handle) => Some(handle.tick(budget)),
            Self::Planning(handle) => Some(handle.tick(budget)),
            Self::Dispatch(handle) => Some(handle.tick(budget)),
            Self::AggregatePublication(handle) => Some(handle.tick(budget)),
        }
    }

    fn request_stop(&mut self) {}
}

impl EventAppendRuntimeHandle {
    fn tick(&mut self) -> WorkerTickReport {
        let (watermark, dropped, health_error) = match self.port.health() {
            Ok(health) => (health.committed_watermark, health.dropped_events, None),
            Err(error) => {
                let (watermark, dropped) = self.last.unwrap_or((0, 0));
                (watermark, dropped, Some(error.to_string()))
            }
        };
        // The first tick only establishes the baseline: an existing ledger
        // is not fresh work and all-time drops are not a fresh burst.
        let (input_watermark, mut retryable_errors) = match self.last {
            None => (watermark, Vec::new()),
            Some((last_watermark, last_dropped)) => {
                let mut issues = Vec::new();
                if dropped > last_dropped {
                    issues.push(WorkerTickIssue {
                        item_id: None,
                        code: "ingest_drops_observed".to_string(),
                        message: format!(
                            "{} best-effort events dropped since the last tick, {dropped} total",
                            dropped - last_dropped
                        ),
                    });
                }
                (last_watermark, issues)
            }
        };
        if let Some(error) = health_error {
            retryable_errors.push(WorkerTickIssue {
                item_id: None,
                code: "event_health_unavailable".to_string(),
                message: error,
            });
        }
        retryable_errors.shrink_to_fit();
        // Items stay zero: the observer commits nothing itself, and the
        // checkpoint movement alone reports ledger progress.
        let report = WorkerTickReport {
            actor_id: "event.append".to_string(),
            scope: worker_scope("events", None, None, None),
            input_checkpoint: WorkerCheckpoint {
                name: "event_commit_watermark".to_string(),
                value: input_watermark,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "event_commit_watermark".to_string(),
                value: watermark,
            },
            items_attempted: 0,
            items_committed: 0,
            retryable_errors,
            fatal_errors: Vec::new(),
            budget_exhausted: false,
            waiting_on: Vec::new(),
        };
        self.last = Some((watermark, dropped));
        report
    }
}

impl GraphReplayRuntimeHandle {
    fn tick(&self, budget: WorkBudget) -> WorkerTickReport {
        self.graph_runtime
            .catch_up_bounded(GraphCatchUpBudget {
                max_items: budget.max_items,
            })
            .map(WorkerTickReport::from)
            .unwrap_or_else(|error| {
                WorkerTickReport::fatal(
                    "world_state.graph.reducer",
                    "world_state",
                    Some("graph"),
                    "event_spine_seq",
                    "graph_replay_failed",
                    error.to_string(),
                )
            })
    }
}

impl BeliefAssessmentHandle {
    fn tick(&mut self, budget: WorkBudget) -> WorkerTickReport {
        let sequence = match self.sequence.next() {
            Ok(sequence) => sequence,
            Err(error) => {
                return WorkerTickReport::fatal(
                    "world_model.belief_assessment",
                    "world_model",
                    Some("belief_assessment"),
                    "belief_assessment_sequence",
                    "step_sequence_failed",
                    error,
                )
            }
        };
        let report = self.actor.bounded_step(&BeliefAssessmentRequest {
            sequence,
            max_items: budget.max_items,
        });
        belief_assessment_worker_report(report, &self.subject_key)
    }
}

impl EvidenceIngestionHandle {
    fn tick(&mut self, budget: WorkBudget) -> WorkerTickReport {
        let report = self.actor.bounded_step(&EvidenceIngestionRequest {
            max_events: budget.max_items,
        });
        evidence_ingestion_worker_report(report)
    }
}

impl AgentActorHandle {
    fn tick(&mut self, budget: WorkBudget) -> WorkerTickReport {
        let sequence = match self.sequence.next() {
            Ok(sequence) => sequence,
            Err(error) => {
                return WorkerTickReport::fatal(
                    self.runtime_id.clone(),
                    "world_model",
                    Some("agent_curation"),
                    "agent_step_sequence",
                    "step_sequence_failed",
                    error,
                )
            }
        };
        let request = AgentStepRequest {
            sequence,
            max_items: budget.max_items,
        };
        // Curation crosses into execution only through the named
        // curation-to-goal-set port, bound here at composition.
        let mut port = CurationGoalExecutionPort::new(
            self.goal_command.clone(),
            self.goal_mutation.clone(),
            sequence,
        );
        let report = match self.kind {
            AgentActorKind::GoalCuration => self
                .curation
                .as_ref()
                .expect("goal curation handle holds its actor")
                .bounded_step(&request, &mut self.goal_query, &mut port),
            AgentActorKind::SatisfactionCuration => self
                .satisfaction
                .as_ref()
                .expect("satisfaction handle holds its actor")
                .bounded_step(&request, &mut self.goal_query, &mut port),
        };
        agent_step_worker_report(report, &self.runtime_id, &self.agent_id)
    }
}

impl PlanningHandle {
    fn tick(&mut self, budget: WorkBudget) -> WorkerTickReport {
        let request = PlanningRuntimeActorRequest {
            network_id: self.request_seed.network_id.clone(),
            perspective_id: self.request_seed.perspective_id.clone(),
            branch_id: self.request_seed.branch_id.clone(),
            requested_dimensions: self.request_seed.requested_dimensions.clone(),
            required_preconditions: Vec::new(),
            available_actions: self.request_seed.available_actions.clone(),
            method_realizations: self.request_seed.method_realizations.clone(),
            limit: Some(budget.max_items),
        };
        let mut network = match self.network.lock() {
            Ok(network) => network,
            Err(_) => {
                return WorkerTickReport::fatal(
                    "execution.planning",
                    "execution",
                    Some("planning"),
                    "task_network_revision",
                    "network_lock_poisoned",
                    "shared task network mutex is poisoned".to_string(),
                )
            }
        };
        match self.actor.run_once(
            &mut self.goal_store,
            &mut network,
            &mut self.projection,
            request,
        ) {
            Ok(report) => {
                // Package-route handoffs are in-process by design; durable
                // dedupe happens in dispatch keyed by the run id derived
                // from the deterministic plan identity.
                for result in &report.results {
                    if let PlanningRuntimeActorGoalResult::PackageRouteSelected { plan, .. } =
                        result
                    {
                        self.handoffs.record(plan.clone());
                    }
                }
                WorkerTickReport::from(report)
            }
            Err(error) => WorkerTickReport::fatal(
                "execution.planning",
                "execution",
                Some("planning"),
                "task_network_revision",
                "planning_actor_failed",
                error.to_string(),
            ),
        }
    }
}

impl DispatchHandle {
    fn tick(&mut self, budget: WorkBudget) -> WorkerTickReport {
        let fatal = |code: &str, message: String| {
            WorkerTickReport::fatal(
                "execution.task_dispatch",
                "execution",
                Some("dispatch"),
                "task_network_revision",
                code,
                message,
            )
        };
        if let Some(error) = &self.construction_error {
            return fatal("dispatch_construction_failed", error.clone());
        }
        let Some(actor) = self.actor.as_ref() else {
            return fatal(
                "dispatch_construction_failed",
                "dispatch actor is absent".to_string(),
            );
        };
        let Some(tokio_runtime) = self.tokio_runtime.as_ref() else {
            return fatal(
                "dispatch_runtime_unavailable",
                "tokio runtime construction failed".to_string(),
            );
        };
        self.sequence = self.sequence.saturating_add(1);
        let request = DispatchTickRequest {
            sequence: self.sequence,
            max_items: budget.max_items,
            package_plans: self.handoffs.plans(),
        };
        let mut network = match self.network.lock() {
            Ok(network) => network,
            Err(_) => {
                return fatal(
                    "network_lock_poisoned",
                    "shared task network mutex is poisoned".to_string(),
                )
            }
        };
        match tokio_runtime.block_on(actor.tick(&mut *network, request)) {
            Ok(report) => dispatch_worker_report(report),
            Err(error) => fatal("dispatch_tick_failed", error.to_string()),
        }
    }
}

impl PublicationHandle {
    fn tick(&mut self, budget: WorkBudget) -> WorkerTickReport {
        let actor_id = "execution.publication";
        let (progress, outbox) = match &self.stores {
            Ok(stores) => stores,
            Err(error) => {
                return WorkerTickReport::fatal(
                    actor_id,
                    "execution",
                    Some("aggregate_publication"),
                    "aggregate_publications",
                    "publication_store_unavailable",
                    error.clone(),
                )
            }
        };
        let mut report = WorkerTickReport {
            actor_id: actor_id.to_string(),
            scope: worker_scope(
                "execution",
                Some("aggregate_publication"),
                None,
                Some(self.bindings.subject.index_key()),
            ),
            input_checkpoint: WorkerCheckpoint {
                name: "aggregate_publications".to_string(),
                value: 0,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "aggregate_publications".to_string(),
                value: 0,
            },
            items_attempted: 0,
            items_committed: 0,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
            waiting_on: Vec::new(),
        };
        // Per-task publications drain first: the durable outbox the task
        // network reducer fills on every recorded outcome, appended to the
        // ledger through the NAG-2 bridge under deterministic record ids so
        // retry is idempotent. Aggregate publication reads reduced state
        // only, so ordering within the tick carries no semantic coupling.
        let mut drained_pending = None;
        if let Some(network) = &self.network {
            let mut store = network.lock().unwrap_or_else(|e| e.into_inner());
            match PublicationRuntime::new().publish_pending(
                &mut store,
                &self.event_append,
                PublishPendingPublicationsRequest {
                    session_id: self.bindings.session_id.clone(),
                    worker_id: self.worker_id.clone(),
                    limit: Some(budget.max_items),
                },
            ) {
                Ok(bridge) => {
                    report.items_attempted += bridge.attempted;
                    report.items_committed += bridge.committed;
                    report.budget_exhausted |= bridge.budget_exhausted;
                    drained_pending = Some(bridge.attempted);
                    for issue in bridge.retryable_errors {
                        report.retryable_errors.push(WorkerTickIssue {
                            item_id: issue.publication_id.clone(),
                            code: "task_publication_append_failed".to_string(),
                            message: issue.message,
                        });
                    }
                    for issue in bridge.fatal_errors {
                        report.fatal_errors.push(WorkerTickIssue {
                            item_id: issue.publication_id.clone(),
                            code: "task_publication_invalid".to_string(),
                            message: issue.message,
                        });
                    }
                }
                Err(error) => {
                    report.retryable_errors.push(WorkerTickIssue {
                        item_id: None,
                        code: "task_publication_bridge_failed".to_string(),
                        message: error.to_string(),
                    });
                }
            }
        }
        let request = PublishAggregateRequest {
            session_id: self.bindings.session_id.clone(),
            worker_id: self.worker_id.clone(),
        };
        for (plan_id, run_id) in self.handoffs.run_bindings() {
            report.items_attempted += 1;
            let binding = AggregateRunBinding {
                package_run_id: run_id,
                network_id: self.bindings.network_id.clone(),
                selected_scope: self.bindings.subject.clone(),
                // Folder classification comes from the stewardship-derived
                // configuration (the selected package's stage chain), never
                // from runtime inventory.
                folder_unit_capability_types: self.bindings.folder_unit_capability_types.clone(),
            };
            // The dispatch actor records the run's terminal outcome through
            // the task-network command boundary; publication reads it back
            // from reduced state. A run without a recorded terminal outcome
            // stays truthfully skipped rather than publishing from a
            // synthesized one.
            let terminal = self.network.as_ref().and_then(|network| {
                let store = network.lock().unwrap_or_else(|e| e.into_inner());
                package_run_terminal_outcome(store.state(), &binding.package_run_id)
            });
            match publish_aggregate_for_run(
                progress,
                terminal.as_ref().map(|record| &record.outcome),
                &binding,
                outbox,
                &self.event_append,
                &request,
            ) {
                Ok(AggregatePublishResult::Published { .. }) => {
                    report.items_committed += 1;
                    report.output_checkpoint.value += 1;
                }
                Ok(AggregatePublishResult::AlreadyPublished { .. }) => {}
                Ok(AggregatePublishResult::Skipped(reason)) => {
                    // The awaited facts travel as a waiting-on declaration
                    // instead of being dropped: the skip reason carries
                    // exactly what would have to exist for the aggregate to
                    // publish.
                    let detail = match reason {
                        AggregateSkipReason::RunProgressNotFound => {
                            "no durable progress exists for the package run".to_string()
                        }
                        AggregateSkipReason::RunNotTerminal {
                            pending_instance_ids,
                        } => format!(
                            "run has no terminal outcome; pending work units: {}",
                            pending_instance_ids.join(", ")
                        ),
                    };
                    report.waiting_on.extend(execution_waiting(vec![
                        meld_execution::WaitingOnDeclaration::about(
                            meld_execution::waiting::conditions::AGGREGATE_RUN_NOT_TERMINAL,
                            binding.package_run_id.clone(),
                            detail,
                        ),
                    ]));
                }
                Ok(AggregatePublishResult::AppendFailed { error, .. }) => {
                    report.retryable_errors.push(WorkerTickIssue {
                        item_id: Some(plan_id.clone()),
                        code: "aggregate_append_failed".to_string(),
                        message: error,
                    });
                }
                Err(error) => {
                    let issue = aggregate_publish_issue(&plan_id, error);
                    if issue.0 {
                        report.fatal_errors.push(issue.1);
                    } else {
                        report.retryable_errors.push(issue.1);
                    }
                }
            }
        }
        // The hardened DBG-016 rule: a tick that absorbed nothing states
        // what would change that. An empty outbox and an empty work list
        // wait on the next recorded task outcome.
        // An errored tick declares nothing quiet: the outbox emptiness was
        // never confirmed, and the recorded issues already narrate the tick.
        if report.items_attempted == 0
            && report.waiting_on.is_empty()
            && report.retryable_errors.is_empty()
            && report.fatal_errors.is_empty()
        {
            let detail = match drained_pending {
                Some(_) => "no pending task publications and no package-route runs to aggregate",
                None => "no task network is composed; nothing records outcomes to publish",
            };
            report.waiting_on.extend(execution_waiting(vec![
                meld_execution::WaitingOnDeclaration::broad(
                    meld_execution::waiting::conditions::NO_PENDING_PUBLICATIONS,
                    detail,
                ),
            ]));
        }
        report
    }
}

/// Classify one aggregate publication error for the tick report.
///
/// A completed run rejected with the no-folder-work signal is a
/// classification mismatch between the stewardship-derived folder types and
/// the compiled package graph — fatal, never retryable.
fn aggregate_publish_issue(
    plan_id: &str,
    error: AggregatePublicationError,
) -> (bool, WorkerTickIssue) {
    let fatal = matches!(
        error,
        AggregatePublicationError::NoFolderWorkUnits { .. }
            | AggregatePublicationError::InvalidRequest(_)
            | AggregatePublicationError::IdentityDrift { .. }
            | AggregatePublicationError::LedgerIdentityMismatch { .. }
    );
    let code = if matches!(error, AggregatePublicationError::NoFolderWorkUnits { .. }) {
        "aggregate_classification_mismatch"
    } else if fatal {
        "aggregate_publication_invalid"
    } else {
        "aggregate_publication_failed"
    };
    (
        fatal,
        WorkerTickIssue {
            item_id: Some(plan_id.to_string()),
            code: code.to_string(),
            message: error.to_string(),
        },
    )
}

fn worker_scope(
    domain_id: &str,
    work_key: Option<&str>,
    agent_id: Option<&str>,
    subject_key: Option<String>,
) -> WorkerScope {
    WorkerScope {
        domain_id: domain_id.to_string(),
        stream_id: None,
        work_key: work_key.map(str::to_string),
        agent_id: agent_id.map(str::to_string),
        perspective_key: None,
        branch_id: None,
        subject_key,
    }
}

/// Translate one belief assessment report into the supervisor shape.
///
/// The step-sequence checkpoint is bookkeeping, not a domain progress
/// cursor, so both worker checkpoints carry the persisted value and
/// progress is signaled through committed items alone: a zero-work step

/// Translate world-model waiting-on declarations into the carrier shape.
fn world_model_waiting(
    declarations: Vec<meld_world_model::WaitingOnDeclaration>,
) -> Vec<crate::runtime::contracts::WaitingOnDeclaration> {
    declarations
        .into_iter()
        .map(
            |declaration| crate::runtime::contracts::WaitingOnDeclaration {
                condition: declaration.condition,
                subject_key: declaration.subject_key,
                detail: declaration.detail,
            },
        )
        .collect()
}

/// Translate execution waiting-on declarations into the carrier shape.
fn execution_waiting(
    declarations: Vec<meld_execution::WaitingOnDeclaration>,
) -> Vec<crate::runtime::contracts::WaitingOnDeclaration> {
    declarations
        .into_iter()
        .map(
            |declaration| crate::runtime::contracts::WaitingOnDeclaration {
                condition: declaration.condition,
                subject_key: declaration.subject_key,
                detail: declaration.detail,
            },
        )
        .collect()
}

/// projects truthfully to active idle.
fn belief_assessment_worker_report(
    report: BeliefAssessmentReport,
    subject_key: &str,
) -> WorkerTickReport {
    WorkerTickReport {
        actor_id: report.actor_id,
        scope: worker_scope(
            "world_model",
            Some("belief_assessment"),
            None,
            Some(subject_key.to_string()),
        ),
        input_checkpoint: WorkerCheckpoint {
            name: "belief_assessment_checkpoint".to_string(),
            value: report.output_checkpoint,
        },
        output_checkpoint: WorkerCheckpoint {
            name: "belief_assessment_checkpoint".to_string(),
            value: report.output_checkpoint,
        },
        items_attempted: report.items_attempted,
        items_committed: report.items_committed,
        retryable_errors: report
            .retryable_errors
            .into_iter()
            .map(|issue| WorkerTickIssue {
                item_id: issue.item_id,
                code: issue.code,
                message: issue.message,
            })
            .collect(),
        fatal_errors: report
            .fatal_errors
            .into_iter()
            .map(|issue| WorkerTickIssue {
                item_id: issue.item_id,
                code: issue.code,
                message: issue.message,
            })
            .collect(),
        budget_exhausted: report.budget_exhausted,
        waiting_on: world_model_waiting(report.waiting_on),
    }
}

/// Translate one evidence ingestion report into the supervisor shape.
///
/// The durable consumer cursor is a real domain cursor, so it maps onto the
/// worker checkpoints directly.
fn evidence_ingestion_worker_report(report: EvidenceIngestionReport) -> WorkerTickReport {
    WorkerTickReport {
        actor_id: report.actor_id,
        scope: worker_scope("world_model", Some("evidence_ingestion"), None, None),
        input_checkpoint: WorkerCheckpoint {
            name: "event_spine_seq".to_string(),
            value: report.input_after_seq,
        },
        output_checkpoint: WorkerCheckpoint {
            name: "event_spine_seq".to_string(),
            value: report.output_after_seq,
        },
        items_attempted: report.events_replayed,
        items_committed: report.new_assignment_count + report.revisions_committed,
        retryable_errors: report
            .retryable_errors
            .into_iter()
            .map(|issue| WorkerTickIssue {
                item_id: issue.item_id,
                code: issue.code,
                message: issue.message,
            })
            .collect(),
        fatal_errors: report
            .fatal_errors
            .into_iter()
            .map(|issue| WorkerTickIssue {
                item_id: issue.item_id,
                code: issue.code,
                message: issue.message,
            })
            .collect(),
        budget_exhausted: report.more_available,
        waiting_on: world_model_waiting(report.waiting_on),
    }
}

/// Translate one bounded agent step report into the supervisor shape.
///
/// Like assessment, the injected step sequence is bookkeeping: both
/// checkpoints carry it and progress derives from persisted decisions and
/// accepted sink submissions.
fn agent_step_worker_report(
    report: AgentStepReport,
    runtime_id: &str,
    agent_id: &str,
) -> WorkerTickReport {
    WorkerTickReport {
        actor_id: runtime_id.to_string(),
        scope: worker_scope("world_model", Some("agent_curation"), Some(agent_id), None),
        input_checkpoint: WorkerCheckpoint {
            name: "agent_step_sequence".to_string(),
            value: report.input_sequence,
        },
        output_checkpoint: WorkerCheckpoint {
            name: "agent_step_sequence".to_string(),
            value: report.input_sequence,
        },
        items_attempted: report.items_attempted,
        items_committed: report.decisions_persisted + report.sink_submissions,
        retryable_errors: report
            .retryable_errors
            .into_iter()
            .map(|issue| WorkerTickIssue {
                item_id: issue.item_id,
                code: issue.code,
                message: issue.message,
            })
            .collect(),
        fatal_errors: report
            .fatal_errors
            .into_iter()
            .map(|issue| WorkerTickIssue {
                item_id: issue.item_id,
                code: issue.code,
                message: issue.message,
            })
            .collect(),
        budget_exhausted: report.budget_exhausted,
        waiting_on: world_model_waiting(report.waiting_on),
    }
}

/// Translate one dispatch tick report into the supervisor shape.
fn dispatch_worker_report(report: DispatchTickReport) -> WorkerTickReport {
    WorkerTickReport {
        actor_id: "execution.task_dispatch".to_string(),
        scope: worker_scope("execution", Some("dispatch"), None, None),
        input_checkpoint: WorkerCheckpoint {
            name: "task_network_revision".to_string(),
            value: report.input_revision,
        },
        output_checkpoint: WorkerCheckpoint {
            name: "task_network_revision".to_string(),
            value: report.output_revision,
        },
        items_attempted: report.items_attempted,
        items_committed: report.items_committed,
        retryable_errors: report
            .retryable_errors
            .into_iter()
            .map(|issue| WorkerTickIssue {
                item_id: issue.item_id,
                code: issue.code,
                message: issue.message,
            })
            .collect(),
        fatal_errors: report
            .fatal_errors
            .into_iter()
            .map(|issue| WorkerTickIssue {
                item_id: issue.item_id,
                code: issue.code,
                message: issue.message,
            })
            .collect(),
        budget_exhausted: report.budget_exhausted,
        waiting_on: execution_waiting(report.waiting_on),
    }
}

fn validate_runtime_selection(runtime_ids: &[String]) -> Result<(), RuntimeAssemblyError> {
    let mut seen = BTreeSet::new();
    for runtime_id in runtime_ids {
        validate_runtime_id(runtime_id)?;
        if !seen.insert(runtime_id) {
            return Err(RuntimeRegistryError::DuplicateRuntimeId(runtime_id.clone()).into());
        }
    }
    Ok(())
}

fn desired_runtime_state(
    registry: &RuntimeFactoryRegistry,
    enabled_runtime_ids: Vec<String>,
    disabled_runtime_ids: Vec<String>,
) -> Result<Vec<DesiredRuntimeState>, RuntimeAssemblyError> {
    let disabled_ids = disabled_runtime_ids.into_iter().collect::<BTreeSet<_>>();
    let enabled_ids = if enabled_runtime_ids.is_empty() {
        registry
            .descriptors()
            .map(|descriptor| descriptor.runtime_id.clone())
            .filter(|runtime_id| !disabled_ids.contains(runtime_id))
            .collect::<BTreeSet<_>>()
    } else {
        enabled_runtime_ids.into_iter().collect::<BTreeSet<_>>()
    };
    if let Some(overlap) = enabled_ids.intersection(&disabled_ids).next() {
        return Err(RuntimeAssemblyError::Config(format!(
            "runtime id '{overlap}' cannot be both enabled and disabled"
        )));
    }

    let mut states = enabled_ids
        .into_iter()
        .map(|runtime_id| DesiredRuntimeState {
            factory_available: registry.contains(&runtime_id),
            runtime_id,
            enabled: true,
        })
        .chain(
            disabled_ids
                .into_iter()
                .map(|runtime_id| DesiredRuntimeState {
                    factory_available: registry.contains(&runtime_id),
                    runtime_id,
                    enabled: false,
                }),
        )
        .collect::<Vec<_>>();
    states.sort_by(|left, right| left.runtime_id.cmp(&right.runtime_id));
    Ok(states)
}

fn provider_required(
    registry: &RuntimeFactoryRegistry,
    desired_runtime_state: &[DesiredRuntimeState],
) -> bool {
    desired_runtime_state.iter().any(|state| {
        state.enabled
            && state.factory_available
            && registry
                .get(&state.runtime_id)
                .is_some_and(|descriptor| descriptor.requires_resource(RuntimeResource::Provider))
    })
}

fn validate_runtime_id(runtime_id: &str) -> Result<(), RuntimeRegistryError> {
    if runtime_id.is_empty()
        || runtime_id.starts_with('.')
        || runtime_id.ends_with('.')
        || runtime_id.contains("..")
        || !runtime_id.contains('.')
        || !runtime_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_')
    {
        return Err(RuntimeRegistryError::InvalidRuntimeId(
            runtime_id.to_string(),
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use meld_events::EventEnvelope;
    use meld_execution::goals::GoalCommandOutcome;
    use meld_execution::task_network::EventAppendSink;
    use meld_lang::{Goal, GoalLifecycle, GoalPriority, GoalSource, Proposition, Term};
    use meld_world_model::PerspectiveKey;
    use proptest::prelude::*;

    use super::*;
    use crate::runtime::error::RuntimePortError;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn assembly_opens_product_and_supervisor_stores() {
        let temp = tempfile::tempdir().unwrap();

        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();

        assert_eq!(assembly.product_root(), temp.path());
        assert_eq!(assembly.layout().ledger_db, temp.path().join("ledger.sled"));
        assert_eq!(
            assembly.supervisor_store().path(),
            temp.path().join("supervisor.sled")
        );
        assert_eq!(assembly.registry().len(), 12);
        assert!(assembly
            .registry()
            .contains("world_model.agent_goal_curation"));
        let task_dispatch = assembly
            .desired_runtime_state()
            .iter()
            .find(|state| state.runtime_id == "execution.task_dispatch")
            .unwrap();
        assert!(!task_dispatch.enabled);
        assert!(task_dispatch.factory_available);
        assembly.flush_product_boundary().unwrap();
        assembly.flush_supervisor_store().unwrap();
    }

    #[test]
    fn describe_for_workspace_does_not_create_or_open_runtime_stores() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        std::fs::create_dir_all(&workspace).unwrap();
        let mut config = MerkleConfig::default();
        config.system.storage.product_root = Some(PathBuf::from(".meld-runtime"));

        let description =
            ProductRuntimeAssembly::describe_for_workspace(&workspace, &config).unwrap();

        let expected_root = crate::config::xdg::workspace_data_dir(&workspace)
            .unwrap()
            .join(".meld-runtime");
        assert_eq!(description.product_root, expected_root);
        assert_eq!(
            description.supervisor_store_path,
            expected_root.join("supervisor.sled")
        );
        assert_eq!(description.desired_runtime_state.len(), 12);
        assert!(!description.product_root.exists());
        assert!(!description.supervisor_store_path.exists());
    }

    #[test]
    fn assembly_can_be_reopened_from_same_product_root() {
        let temp = tempfile::tempdir().unwrap();
        let first = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        first.flush_product_boundary().unwrap();
        first.flush_supervisor_store().unwrap();
        drop(first);

        let second = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();

        assert_eq!(second.product_root(), temp.path());
        assert!(second.registry().contains("execution.publication"));
        assert_eq!(second.desired_runtime_state().len(), 12);
    }

    #[test]
    fn disabled_runtime_ids_are_excluded_from_default_enabled_set() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config
            .disabled_runtime_ids
            .push("execution.publication".to_string());

        let assembly = ProductRuntimeAssembly::load(config).unwrap();

        let disabled = assembly
            .desired_runtime_state()
            .iter()
            .find(|state| state.runtime_id == "execution.publication")
            .unwrap();
        assert!(!disabled.enabled);
        assert_eq!(
            assembly
                .desired_runtime_state()
                .iter()
                .filter(|state| state.enabled)
                .count(),
            10
        );
    }

    #[test]
    fn explicit_enabled_and_disabled_overlap_fails() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["execution.publication".to_string()];
        config.disabled_runtime_ids = vec!["execution.publication".to_string()];

        let error = match ProductRuntimeAssembly::load(config) {
            Ok(_) => panic!("overlapping enabled and disabled runtime ids should fail"),
            Err(error) => error,
        };

        assert!(matches!(error, RuntimeAssemblyError::Config(_)));
    }

    #[test]
    fn unsupported_desired_runtime_is_preserved_for_supervisor_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["future.runtime".to_string()];

        let assembly = ProductRuntimeAssembly::load(config).unwrap();

        let future = assembly
            .desired_runtime_state()
            .iter()
            .find(|state| state.runtime_id == "future.runtime")
            .unwrap();
        assert!(future.enabled);
        assert!(!future.factory_available);
    }

    #[test]
    fn provider_required_runtime_without_provider_fails_before_start() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["execution.task_dispatch".to_string()];
        config.disabled_runtime_ids = Vec::new();

        let error = match ProductRuntimeAssembly::load(config) {
            Ok(_) => panic!("provider-dependent runtime should require provider availability"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            RuntimeAssemblyError::ProviderConstruction(_)
        ));
    }

    #[test]
    fn provider_required_runtime_opens_when_provider_is_available() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["execution.task_dispatch".to_string()];
        config.disabled_runtime_ids = Vec::new();
        config.provider.provider_available = true;

        let assembly = ProductRuntimeAssembly::load(config).unwrap();

        assert!(assembly.ports().adapters().provider().is_required());
        assert!(assembly.ports().adapters().provider().is_available());
    }

    #[test]
    fn provider_required_runtime_accepts_present_environment_credentials() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let env_name = "MELD_RUNTIME_ASSEMBLY_TEST_PROVIDER_KEY";
        std::env::set_var(env_name, "present");
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["execution.task_dispatch".to_string()];
        config.disabled_runtime_ids = Vec::new();
        config.provider.required_env_vars = vec![env_name.to_string()];

        let assembly = ProductRuntimeAssembly::load(config).unwrap();

        std::env::remove_var(env_name);
        assert!(assembly.ports().adapters().provider().is_required());
        assert!(assembly.ports().adapters().provider().is_available());
    }

    #[test]
    fn event_append_and_replay_ports_are_wired_to_one_authority() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let envelope = EventEnvelope::with_now_domain(
            "session-a",
            "execution",
            "network-a",
            "execution.test",
            None,
            serde_json::json!({"ok": true}),
        )
        .with_record_id("record-a");

        let seq = assembly
            .ports()
            .event_append()
            .append_envelope_idempotent(envelope)
            .unwrap();
        let records = assembly
            .ports()
            .event_replay()
            .read_after_limit(0, 10)
            .unwrap();

        assert_eq!(seq.seq, 1);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].seq, 1);
        assert_eq!(records[0].record_id.as_deref(), Some("record-a"));
        assert!(matches!(
            assembly
            .ports()
            .event_replay()
            .read_after_limit(0, 0),
            Err(RuntimePortError::InvalidRequest(message))
                if message == "event replay limit must be in 1..=1024, got 0"
        ));
        assert!(assembly
            .ports()
            .event_replay()
            .read_after_limit(0, crate::runtime::ports::MAX_EVENT_REPLAY_LIMIT)
            .is_ok());
        assert!(matches!(
            assembly
            .ports()
            .event_replay()
            .read_after_limit(0, crate::runtime::ports::MAX_EVENT_REPLAY_LIMIT + 1),
            Err(RuntimePortError::InvalidRequest(message))
                if message == "event replay limit must be in 1..=1024, got 1025"
        ));
        assert!(matches!(
            assembly
            .ports()
            .event_replay()
            .read_after_limit(0, usize::MAX),
            Err(RuntimePortError::InvalidRequest(message))
                if message == format!("event replay limit must be in 1..=1024, got {}", usize::MAX)
        ));
    }

    #[test]
    fn supplied_authority_and_graph_runtime_are_shared_across_assembly() {
        let temp = tempfile::tempdir().unwrap();
        let authority_db = sled::open(temp.path().join("bound-ledger.sled")).unwrap();
        let authority = Arc::new(
            EventAuthority::open(authority_db, EventAuthorityOpenOptions::default()).unwrap(),
        );
        let expected_identity = authority.ledger_identity();

        let assembly = ProductRuntimeAssembly::load_with_authority(
            ProductRuntimeConfig::for_product_root(temp.path().join("product")),
            Arc::clone(&authority),
        )
        .unwrap();

        let assembled_authority = assembly.event_authority();
        assert!(Arc::ptr_eq(&authority, &assembled_authority));
        assert_eq!(
            assembly.ports().event_replay().ledger_identity(),
            expected_identity
        );
        let shared_graph = assembly.graph_runtime();
        let factory = assembly
            .handle_factories()
            .get("world_model.graph_replay")
            .unwrap();
        match &factory.semantic {
            RuntimeSemanticHandleFactory::GraphReplay { graph_runtime } => {
                assert!(Arc::ptr_eq(&shared_graph, graph_runtime));
            }
            _ => panic!("graph replay factory must retain the shared graph runtime"),
        }
    }

    #[test]
    fn graph_replay_descriptor_declares_complete_authority_dependencies() {
        let registry = RuntimeFactoryRegistry::first_proof_registry().unwrap();
        let descriptor = registry.get("world_model.graph_replay").unwrap();
        assert_eq!(
            descriptor.required_resources,
            vec![
                RuntimeResource::EventAppend,
                RuntimeResource::EventReplay,
                RuntimeResource::EventConsumerRegistry,
                RuntimeResource::WorldModel,
            ]
        );
    }

    #[test]
    fn goal_command_port_accepts_agent_goal_command() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let command = agent_goal_command();

        let outcome = assembly
            .ports()
            .goal_command()
            .accept_agent_goal_command(command, 7)
            .unwrap();

        match outcome {
            GoalCommandOutcome::Applied(record) => {
                assert_eq!(record.goal.goal_id, "goal-a");
                assert!(matches!(record.goal.lifecycle, GoalLifecycle::Active));
                assert_eq!(record.created_at_seq, 7);
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test]
    fn goal_mutation_port_satisfies_agent_goal_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let command = agent_goal_command();
        let goal_id = command.goal.goal_id.clone();
        let dedupe_key = command.dedupe_key.clone();
        assembly
            .ports()
            .goal_command()
            .accept_agent_goal_command(command, 7)
            .unwrap();

        let mutation = agent_goal_mutation_command(dedupe_key, goal_id, 9);
        let outcome = assembly
            .ports()
            .goal_mutation()
            .satisfy_agent_goal_mutation(mutation.clone())
            .unwrap();
        let replay = assembly
            .ports()
            .goal_mutation()
            .satisfy_agent_goal_mutation(mutation)
            .unwrap();
        assert_eq!(replay, outcome);

        match outcome {
            GoalCommandOutcome::Applied(record) => {
                assert!(matches!(
                    record.goal.lifecycle,
                    GoalLifecycle::Satisfied { at_seq: 9 }
                ));
            }
            other => panic!("unexpected outcome: {other:?}"),
        }
    }

    #[test]
    fn planner_projection_port_is_wired_to_world_model_stores() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let subject = subject();

        let projection = assembly
            .ports()
            .planner_projection()
            .project_current_world_state(&subject, "docs_freshness", None, None)
            .unwrap();

        assert_eq!(projection.projection_version, "world_model.planner.v1");
    }

    #[test]
    fn adapter_ports_expose_passive_factories_and_stores() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let adapters = assembly.ports().adapters();

        assert!(!adapters.provider().is_required());
        assert!(!adapters.provider().is_available());
        assert!(adapters
            .task_networks()
            .factory()
            .root()
            .ends_with("task_networks"));
        adapters.task_artifacts().factory().flush().unwrap();
        assert!(adapters
            .context()
            .storage()
            .as_ref()
            .root()
            .ends_with("frames"));
        assert!(adapters
            .prompt()
            .storage()
            .as_ref()
            .root()
            .ends_with("prompt_artifacts"));
        adapters.workspace().store().flush().unwrap();
    }

    #[test]
    fn startup_package_exposes_runtime_handle_factories() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();

        let package = assembly.supervisor_startup_package();

        assert_eq!(package.product_root, temp.path());
        assert_eq!(package.handle_factories.len(), 12);
        assert_eq!(package.default_work_budget.max_items, 64);
        assert_eq!(package.lifecycle_config.heartbeat_interval_ms, 1_000);
        assert_eq!(package.process_services.clock_source, "system");
        let factory = package.handle_factories.get("event.append").unwrap();
        let mut handle = factory.build_handle();
        assert_eq!(handle.runtime_id(), "event.append");
        assert!(!handle.is_started());
        let started = handle
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "event.append".to_string(),
                lease_id: "lease-a".to_string(),
            })
            .unwrap();
        assert_eq!(started.runtime_id, "event.append");
        assert!(handle.is_started());
        let diagnostic = handle.diagnostic_report();
        assert_eq!(diagnostic.runtime_id, "event.append");
        assert!(diagnostic.started);
        let flush = handle.flush_resources().unwrap();
        assert_eq!(flush.runtime_id, "event.append");
        assert!(!flush.flushed_resource);
        let stop = handle.request_stop();
        assert_eq!(stop.runtime_id, "event.append");
        assert!(stop.was_started);
        assert!(!handle.is_started());
        let safe_point = handle.wait_for_safe_point();
        assert_eq!(safe_point.runtime_id, "event.append");
        assert!(safe_point.safe_for_flush);
    }

    #[test]
    fn duplicate_runtime_ids_fail_registry_construction() {
        let descriptor =
            RuntimeFactoryDescriptor::new("execution.planning", vec![RuntimeResource::Provider])
                .unwrap();

        let error =
            RuntimeFactoryRegistry::from_descriptors([descriptor.clone(), descriptor]).unwrap_err();

        assert!(matches!(error, RuntimeRegistryError::DuplicateRuntimeId(_)));
    }

    #[test]
    fn invalid_runtime_ids_fail_registry_construction() {
        let error = RuntimeFactoryDescriptor::new("execution planning", Vec::new()).unwrap_err();

        assert!(matches!(error, RuntimeRegistryError::InvalidRuntimeId(_)));
    }

    #[test]
    fn runtime_id_validation_accepts_only_domain_scoped_ascii_ids() {
        let mut runner = proptest::test_runner::TestRunner::default();
        let strategy = "[A-Za-z0-9_. -]{0,32}";

        runner
            .run(&strategy, |candidate| {
                let result = RuntimeFactoryDescriptor::new(candidate.clone(), Vec::new());
                let expected = !candidate.is_empty()
                    && !candidate.starts_with('.')
                    && !candidate.ends_with('.')
                    && !candidate.contains("..")
                    && candidate.contains('.')
                    && candidate.chars().all(|ch| {
                        ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_'
                    });
                prop_assert_eq!(result.is_ok(), expected);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn duplicate_enabled_and_disabled_runtime_ids_fail_selection() {
        let temp = tempfile::tempdir().unwrap();
        let mut enabled = ProductRuntimeConfig::for_product_root(temp.path());
        enabled.enabled_runtime_ids = vec![
            "execution.publication".to_string(),
            "execution.publication".to_string(),
        ];

        let enabled_error = match ProductRuntimeAssembly::load(enabled) {
            Ok(_) => panic!("duplicate enabled runtime ids should fail"),
            Err(error) => error,
        };
        assert!(matches!(
            enabled_error,
            RuntimeAssemblyError::RuntimeRegistry(RuntimeRegistryError::DuplicateRuntimeId(_))
        ));

        let mut disabled = ProductRuntimeConfig::for_product_root(temp.path());
        disabled.disabled_runtime_ids = vec![
            "execution.publication".to_string(),
            "execution.publication".to_string(),
        ];

        let disabled_error = match ProductRuntimeAssembly::load(disabled) {
            Ok(_) => panic!("duplicate disabled runtime ids should fail"),
            Err(error) => error,
        };
        assert!(matches!(
            disabled_error,
            RuntimeAssemblyError::RuntimeRegistry(RuntimeRegistryError::DuplicateRuntimeId(_))
        ));
    }

    #[test]
    fn construction_does_not_append_events_or_query_semantic_work() {
        let temp = tempfile::tempdir().unwrap();
        seed_invalid_goal_record(temp.path());
        seed_invalid_belief_view(temp.path());
        seed_invalid_task_network_journal(temp.path());

        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();

        assert!(assembly
            .ports()
            .event_replay()
            .read_after_limit(0, 1)
            .unwrap()
            .is_empty());
        assert!(assembly.stores().goal_store.active_goals().is_err());
        assert!(assembly
            .ports()
            .planner_projection()
            .project_current_world_state(&subject(), "docs_freshness", None, None)
            .is_err());
        assert!(assembly
            .ports()
            .adapters()
            .task_networks()
            .factory()
            .open_network("network-a")
            .is_err());
        let outcome = assembly
            .ports()
            .goal_command()
            .accept_agent_goal_command(agent_goal_command(), 7)
            .unwrap();
        assert!(matches!(outcome, GoalCommandOutcome::Applied(_)));
    }

    #[test]
    fn assembly_reopens_ports_over_persisted_state() {
        let temp = tempfile::tempdir().unwrap();
        let first = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let envelope = EventEnvelope::with_now_domain(
            "session-a",
            "execution",
            "network-a",
            "execution.test",
            None,
            serde_json::json!({"ok": true}),
        )
        .with_record_id("record-a");
        let command = agent_goal_command();
        let command_replay = command.clone();

        first
            .ports()
            .event_append()
            .append_envelope_idempotent(envelope)
            .unwrap();
        first
            .ports()
            .goal_command()
            .accept_agent_goal_command(command, 7)
            .unwrap();
        let network = first
            .ports()
            .adapters()
            .task_networks()
            .factory()
            .open_network("network-a")
            .unwrap();
        network.flush().unwrap();
        first.flush_product_boundary().unwrap();
        first.flush_supervisor_store().unwrap();
        drop(first);

        let second = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let records = second
            .ports()
            .event_replay()
            .read_after_limit(0, 10)
            .unwrap();
        let outcome = second
            .ports()
            .goal_command()
            .accept_agent_goal_command(command_replay, 7)
            .unwrap();

        assert_eq!(records.len(), 1);
        assert!(matches!(outcome, GoalCommandOutcome::Applied(_)));
        second.flush_product_boundary().unwrap();
        second.flush_supervisor_store().unwrap();
    }

    fn agent_goal_mutation_command(
        dedupe_key: meld_world_model::AgentCurationDedupeKey,
        goal_id: String,
        review_seq: u64,
    ) -> meld_world_model::AgentGoalMutationCommand {
        meld_world_model::AgentGoalMutationCommand {
            command_id: "mutation-a".to_string(),
            agent_id: "agent-a".to_string(),
            goal_id,
            kind: meld_world_model::AgentGoalMutationKind::Satisfy {
                at_seq: review_seq,
                lifecycle_epoch: 0,
            },
            dedupe_key,
            review_seq,
            projection_version: "world_model.planner.v1".to_string(),
            planner_source_refs: Vec::new(),
            planner_warnings: Vec::new(),
        }
    }

    fn seed_invalid_goal_record(root: &Path) {
        let layout = ProductStorageLayout::from_root(root);
        let db = sled::open(layout.execution_goals_db).unwrap();
        db.open_tree("execution_goal_records")
            .unwrap()
            .insert("corrupt-goal", b"not json".as_slice())
            .unwrap();
        db.flush().unwrap();
        drop(db);
    }

    fn seed_invalid_belief_view(root: &Path) {
        let layout = ProductStorageLayout::from_root(root);
        let subject = subject();
        let perspective = PerspectiveKey::new("default", "default").unwrap();
        let belief_key = meld_world_model::BeliefKey {
            subject: subject.clone(),
            dimension_id: "docs_freshness".to_string(),
            predicate_id: "confidence".to_string(),
            perspective: perspective.clone(),
            branch_scope: meld_world_model::BranchScope::main(),
            evidence_policy_id: "default_policy".to_string(),
        };
        let key = belief_key.index_key();
        let db = sled::open(layout.world_model_db).unwrap();
        db.open_tree("belief_views")
            .unwrap()
            .insert(key.as_bytes(), b"not json".as_slice())
            .unwrap();
        db.open_tree("belief_view_by_subject")
            .unwrap()
            .insert(
                format!(
                    "{}::{}::{}",
                    subject.index_key(),
                    perspective.index_key(),
                    key
                )
                .as_bytes(),
                key.as_bytes(),
            )
            .unwrap();
        db.flush().unwrap();
        drop(db);
    }

    fn seed_invalid_task_network_journal(root: &Path) {
        let layout = ProductStorageLayout::from_root(root);
        std::fs::create_dir_all(&layout.task_networks_root).unwrap();
        let db = sled::open(layout.task_networks_root.join("network-a.sled")).unwrap();
        db.open_tree("task_network_journal_by_revision")
            .unwrap()
            .insert(1_u64.to_be_bytes().as_slice(), b"not json".as_slice())
            .unwrap();
        db.flush().unwrap();
        drop(db);
    }

    fn agent_goal_command() -> meld_world_model::AgentGoalCommand {
        let subject = subject();
        let rule = meld_world_model::AgentCurationRuleConfig {
            dimension_id: "docs_freshness".to_string(),
            threshold: 0.7,
            priority_urgency: 8,
            desired_summary: "fresh docs".to_string(),
            source_kind: "docs_freshness".to_string(),
        };
        let dedupe_key = meld_world_model::AgentCurationDedupeKey::threshold_rule(
            "agent-a",
            &subject,
            &meld_world_model::BranchScope::main(),
            &rule,
        );
        meld_world_model::AgentGoalCommand {
            command_id: "command-a".to_string(),
            goal: Goal {
                goal_id: "goal-a".to_string(),
                agent_id: "agent-a".to_string(),
                target: Proposition::Holds {
                    subject: Term::Object(subject),
                    dimension: Term::Dimension("docs_freshness".to_string()),
                    condition: rule.target_condition(),
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
            },
            dedupe_key,
        }
    }

    fn subject() -> meld_events::DomainObjectRef {
        meld_events::DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap()
    }

    #[test]
    fn event_append_handle_reports_watermark_and_drop_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let factory = assembly.handle_factories().get("event.append").unwrap();
        let mut handle = factory.build_handle();
        handle
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "event.append".to_string(),
                lease_id: "lease-a".to_string(),
            })
            .unwrap();

        let first = handle.tick(WorkBudget { max_items: 8 }).unwrap();
        assert_eq!(first.actor_id, "event.append");
        assert_eq!(first.scope.domain_id, "events");
        assert_eq!(first.input_checkpoint.name, "event_commit_watermark");
        assert!(first.fatal_errors.is_empty());
        assert!(first.retryable_errors.is_empty());
        // The first tick is a baseline: an existing ledger is never
        // reported as fresh progress or fresh work.
        assert_eq!(first.input_checkpoint.value, first.output_checkpoint.value);
        assert_eq!(first.items_committed, 0);
        assert!(!first.made_progress());

        assembly
            .ports()
            .event_append()
            .append_envelope_idempotent(meld_events::EventEnvelope::with_now(
                "session-a",
                "session.tick",
                serde_json::json!({}),
            ))
            .unwrap();
        let second = handle.tick(WorkBudget { max_items: 8 }).unwrap();
        assert!(second.output_checkpoint.value > second.input_checkpoint.value);
        assert!(second.made_progress());
        assert_eq!(second.items_committed, 0);
    }

    // ---- Stewardship composition fixtures ----

    use crate::config::{DocsFreshnessSelection, StewardshipConfig, TheorySelection};
    use crate::runtime::registration::RegistrationLifecycle;
    use crate::runtime::supervisor::{RuntimeSupervisor, SupervisorStartCommand};
    use meld_world_model::agent::{
        AgentCurationRuleBinding, AgentRegistration, AgentSubscription, SeedAgentRegistration,
        SubscribeAgentCommand,
    };
    use meld_world_model::belief::{
        configured_belief_key, OutcomeContentRule, OutcomeFieldRule, OutcomeMappingConfig,
        OutcomeMappingSetConfig, OutcomeSubjectBinding, OutcomeValueSource,
    };

    const STEWARD_AGENT_ID: &str = "seed.docs_freshness";
    const FAMILY_ID: &str = "docs_freshness";
    const MAPPING_ID: &str = "docs_freshness_outcome_interpretation_v1";

    fn stewardship_merkle_config(workspace: &Path, storage_root: &Path) -> MerkleConfig {
        let mut config = MerkleConfig::default();
        config.providers.insert(
            "main-provider".to_string(),
            crate::config::ProviderConfig {
                provider_name: Some("main-provider".to_string()),
                provider_type: crate::provider::ProviderType::Ollama,
                model: "test-model".to_string(),
                api_key: None,
                endpoint: None,
                default_options: crate::provider::CompletionOptions::default(),
            },
        );
        config.system.storage.product_root = Some(storage_root.to_path_buf());
        config.stewardship = StewardshipConfig {
            docs_freshness: Some(DocsFreshnessSelection {
                expression: "docs_freshness".to_string(),
                target_root: workspace.to_path_buf(),
                subject: "docs".to_string(),
                agent_id: STEWARD_AGENT_ID.to_string(),
                provider_id: "main-provider".to_string(),
                theory: TheorySelection {
                    belief_family_id: FAMILY_ID.to_string(),
                    evidence_mapping_id: MAPPING_ID.to_string(),
                    curation_rule_id: "docs_freshness".to_string(),
                },
            }),
        };
        config
    }

    fn resolved_physical_binding(workspace: &Path, storage_root: &Path) -> PhysicalBinding {
        PhysicalBinding::resolve(&stewardship_merkle_config(workspace, storage_root)).unwrap()
    }

    fn open_external_authority(dir: &Path) -> Arc<EventAuthority> {
        let db = sled::open(dir.join("ledger.sled")).unwrap();
        Arc::new(EventAuthority::open(db, EventAuthorityOpenOptions::default()).unwrap())
    }

    fn family_config_json() -> &'static str {
        r#"{
            "family_id": "docs_freshness",
            "dimension_id": "docs_freshness",
            "predicate_id": "confidence",
            "evidence_policy_id": "default_policy",
            "evidence_schemas": [
                {
                    "schema_id": "content_written_signal",
                    "required": false,
                    "role": "Support",
                    "reliability": 1.0,
                    "precision": 1.0
                }
            ],
            "source_mappings": [
                {
                    "mapping_id": "content_written_to_signal",
                    "source_kind": "content_written",
                    "evidence_schema_id": "content_written_signal",
                    "subject_from": "record.subject",
                    "value_field": "stale_probability",
                    "factor_id": "content_written_signal"
                }
            ],
            "comparator": {
                "engine_id": "weighted_bayesian",
                "engine_version": "1",
                "factors": [
                    {
                        "factor_id": "content_written_signal",
                        "evidence_schema_id": "content_written_signal",
                        "weight": 1.0,
                        "polarity": "Supports"
                    }
                ],
                "missing_evidence_uncertainty": 0.9
            },
            "default_prior": 0.8,
            "planner_projection": {
                "confidence_field": "confidence",
                "threshold": 0.7,
                "posterior_meaning": "stale_probability"
            },
            "config_version": "1"
        }"#
    }

    fn installed_outcome_mapping() -> OutcomeMappingSetConfig {
        OutcomeMappingSetConfig {
            mapping_id: MAPPING_ID.to_string(),
            rules: vec![installed_task_success_rule()],
        }
    }

    fn installed_task_success_rule() -> OutcomeMappingConfig {
        OutcomeMappingConfig {
            mapping_id: "task-success-rule".to_string(),
            source_kind: "content_written".to_string(),
            match_domain_id: "execution".to_string(),
            match_event_type: "execution.task.succeeded".to_string(),
            match_content: vec![OutcomeContentRule::ArrayAnyFieldEquals {
                array_pointer: "/artifact_records".to_string(),
                field: "artifact_type_id".to_string(),
                equals: "docs_patch".to_string(),
            }],
            subject: OutcomeSubjectBinding {
                from: Default::default(),
                object_kind: "node".to_string(),
                domain_id: Some("workspace_fs".to_string()),
            },
            evidence_fields: vec![OutcomeFieldRule {
                field: "stale_probability".to_string(),
                source: OutcomeValueSource::Constant { value: 0.0 },
            }],
        }
    }

    struct StewardshipHarness {
        _workspace: tempfile::TempDir,
        _external: tempfile::TempDir,
        binding: PhysicalBinding,
        authority: Arc<EventAuthority>,
    }

    impl StewardshipHarness {
        fn new() -> Self {
            let workspace = tempfile::tempdir().unwrap();
            let external = tempfile::tempdir().unwrap();
            let binding =
                resolved_physical_binding(workspace.path(), &external.path().join("runtime"));
            let ledger_dir = external.path().join("ledger");
            std::fs::create_dir_all(&ledger_dir).unwrap();
            let authority = open_external_authority(&ledger_dir);
            Self {
                _workspace: workspace,
                _external: external,
                binding,
                authority,
            }
        }

        fn assembly(&self, theory: StewardshipTheoryBindings) -> ProductRuntimeAssembly {
            ProductRuntimeAssembly::load_composed(
                ProductRuntimeConfig::for_product_root(self.binding.storage_root.clone()),
                Arc::clone(&self.authority),
                Some(StewardshipComposition {
                    binding: self.binding.clone(),
                    theory,
                }),
            )
            .unwrap()
        }

        /// Stage 2 and 3 world initialization through public domain commands.
        fn run_world_genesis(&self, assembly: &ProductRuntimeAssembly) {
            let stores = assembly.stores();
            let mut registry = stores.belief_family_registry.as_ref().clone();
            let family_config = serde_json::from_str(family_config_json()).unwrap();
            let (_, revision) = registry.install(family_config, 1).unwrap();
            let agent_store = stores.agent_store.opened().unwrap();
            AgentRegistration::new(agent_store)
                .register_seed_agent(SeedAgentRegistration {
                    agent_id: STEWARD_AGENT_ID.to_string(),
                    perspective_key: PerspectiveKey::new("default", "default").unwrap(),
                    subject: stewardship_subject_ref(&self.binding).unwrap(),
                    branch_scope: BranchScope::main(),
                    observation_scope: FAMILY_ID.to_string(),
                    directive: "steward docs freshness".to_string(),
                    seed_provenance: "trusted init".to_string(),
                    curation_rule: Some(
                        AgentCurationRuleBinding::for_rule(
                            meld_world_model::AgentCurationRuleConfig {
                                dimension_id: "docs_freshness".to_string(),
                                threshold: 0.7,
                                priority_urgency: 8,
                                desired_summary: "fresh docs".to_string(),
                                source_kind: "docs_freshness".to_string(),
                            },
                        )
                        .unwrap(),
                    ),
                    created_at_seq: 2,
                })
                .unwrap();
            let belief_key = configured_belief_key(
                &revision,
                &stewardship_subject_ref(&self.binding).unwrap(),
                &PerspectiveKey::new("default", "default").unwrap(),
                &BranchScope::main(),
            );
            AgentSubscription::new(agent_store)
                .subscribe(SubscribeAgentCommand {
                    agent_id: STEWARD_AGENT_ID.to_string(),
                    belief_key,
                    created_at_seq: 3,
                })
                .unwrap();
            AgentRegistration::new(agent_store)
                .mark_operational(STEWARD_AGENT_ID, 4)
                .unwrap();
            assembly.flush_product_boundary().unwrap();
        }
    }

    fn lifecycle_of(
        status: &crate::runtime::supervisor::SupervisorStatusSnapshot,
        runtime_id: &str,
    ) -> Option<RegistrationLifecycle> {
        status
            .runtimes
            .iter()
            .find(|row| row.runtime_id == runtime_id)
            .unwrap_or_else(|| panic!("status row for '{runtime_id}'"))
            .lifecycle
    }

    fn kind_of(
        status: &crate::runtime::supervisor::SupervisorStatusSnapshot,
        runtime_id: &str,
    ) -> RegistrationKind {
        status
            .runtimes
            .iter()
            .find(|row| row.runtime_id == runtime_id)
            .unwrap_or_else(|| panic!("status row for '{runtime_id}'"))
            .registration_kind
    }

    #[test]
    fn stewardship_registration_derivation_classifies_required_roles() {
        let harness = StewardshipHarness::new();

        let set = derive_stewardship_registrations(&harness.binding).unwrap();

        assert_eq!(set.registrations.len(), 12);
        for passive in STEWARDSHIP_PASSIVE_SERVICE_IDS {
            assert_eq!(
                set.kind_of(passive),
                Some(RegistrationKind::PassiveService),
                "'{passive}' must be a passive service"
            );
        }
        for active in [
            "world_model.graph_replay",
            "world_model.belief_assessment",
            "world_model.evidence_ingestion",
            "world_model.agent_goal_curation",
            "world_model.satisfaction_curation",
            "execution.planning",
            "execution.task_dispatch",
            "execution.publication",
        ] {
            assert_eq!(
                set.kind_of(active),
                Some(RegistrationKind::ActiveActor),
                "'{active}' must be an active actor"
            );
        }
        assert!(set.registrations.iter().all(|registration| registration
            .registration_id
            .starts_with("stewardship::docs_freshness::")));
    }

    #[test]
    fn ungenesised_stewardship_boot_is_truthful_and_writes_no_semantic_state() {
        let harness = StewardshipHarness::new();
        let assembly = harness.assembly(StewardshipTheoryBindings::default());
        let registration_set = assembly.registration_set().cloned().unwrap();

        let mut command = SupervisorStartCommand::new("instance-a", 100);
        command.registration_set = Some(registration_set);
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        let tick = supervisor.tick(1_100).unwrap();
        let status = supervisor.status_snapshot(1_100).unwrap();

        // Genesis-dependent actors hydrate truthfully unresolved: activation
        // never creates the theory or identities they require.
        for unresolved in [
            "world_model.belief_assessment",
            "world_model.evidence_ingestion",
            "world_model.agent_goal_curation",
            "world_model.satisfaction_curation",
            "execution.planning",
        ] {
            assert_eq!(
                lifecycle_of(&status, unresolved),
                Some(RegistrationLifecycle::UnresolvedRequiredBinding),
                "'{unresolved}' must be an unresolved required binding"
            );
        }
        // Dispatch is disabled by default until provider access and route
        // bindings are composed.
        assert_eq!(
            lifecycle_of(&status, "execution.task_dispatch"),
            Some(RegistrationLifecycle::Stopped)
        );
        // Passive services carry no actor lifecycle and are never leased.
        for passive in STEWARDSHIP_PASSIVE_SERVICE_IDS {
            assert_eq!(kind_of(&status, passive), RegistrationKind::PassiveService);
            assert_eq!(lifecycle_of(&status, passive), None);
        }
        // The quiescent bound actors reach truthful active idle.
        for idle in ["world_model.graph_replay", "execution.publication"] {
            assert_eq!(
                lifecycle_of(&status, idle),
                Some(RegistrationLifecycle::ActiveIdle),
                "'{idle}' must be active idle over an empty world"
            );
        }
        // Exactly one bounded invocation per bound active actor per pass.
        let mut ticked: Vec<&str> = tick
            .actions
            .iter()
            .map(|action| action.runtime_id.as_str())
            .collect();
        ticked.sort_unstable();
        assert_eq!(
            ticked,
            vec!["execution.publication", "world_model.graph_replay"]
        );

        // Boot and tick created no semantic state anywhere.
        assert!(assembly
            .ports()
            .event_replay()
            .read_after_limit(0, 10)
            .unwrap()
            .is_empty());
        assert!(assembly
            .stores()
            .goal_store
            .goal_records()
            .unwrap()
            .is_empty());
        assert!(assembly
            .stores()
            .agent_store
            .get_agent(STEWARD_AGENT_ID)
            .unwrap()
            .is_none());
        supervisor.request_shutdown(2_000).unwrap();
    }

    #[test]
    fn genesised_world_binds_epistemic_actors_and_ticks_each_exactly_once() {
        let harness = StewardshipHarness::new();
        {
            let assembly = harness.assembly(StewardshipTheoryBindings::default());
            harness.run_world_genesis(&assembly);
        }

        let assembly = harness.assembly(StewardshipTheoryBindings {
            outcome_mapping: Some(installed_outcome_mapping()),
            ..StewardshipTheoryBindings::default()
        });
        let registration_set = assembly.registration_set().cloned().unwrap();
        let mut command = SupervisorStartCommand::new("instance-b", 100);
        command.registration_set = Some(registration_set);
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        let tick = supervisor.tick(1_100).unwrap();
        let status = supervisor.status_snapshot(1_100).unwrap();

        let bound = [
            "world_model.graph_replay",
            "world_model.belief_assessment",
            "world_model.evidence_ingestion",
            "world_model.agent_goal_curation",
            "world_model.satisfaction_curation",
            "execution.publication",
        ];
        for runtime_id in bound {
            let lifecycle = lifecycle_of(&status, runtime_id);
            assert!(
                matches!(
                    lifecycle,
                    Some(RegistrationLifecycle::ActiveIdle)
                        | Some(RegistrationLifecycle::ActiveWorking)
                ),
                "'{runtime_id}' must be truthfully active after genesis, got {lifecycle:?}"
            );
        }
        // Planning theory has no durable registry yet, so it stays a
        // truthful unresolved binding until a composition injects it.
        assert_eq!(
            lifecycle_of(&status, "execution.planning"),
            Some(RegistrationLifecycle::UnresolvedRequiredBinding)
        );
        // One bounded invocation per bound actor per maintenance pass.
        let mut ticked: Vec<&str> = tick
            .actions
            .iter()
            .map(|action| action.runtime_id.as_str())
            .collect();
        ticked.sort_unstable();
        let mut expected = bound.to_vec();
        expected.sort_unstable();
        assert_eq!(ticked, expected);
        supervisor.request_shutdown(2_000).unwrap();
    }

    /// Route stubs shaped like production bindings; never invoked because
    /// these tests exercise resolution, not execution.
    fn stub_routes() -> DispatchRouteBindings {
        use meld_execution::capability::{
            BoundCapabilityInstance, CapabilityInvocationPayload, CapabilityInvocationResult,
        };
        use meld_execution::task::expansion::{CompiledTaskDelta, TaskExpansionRequest};
        use meld_execution::task::{
            CompiledTaskRecord, PackageStepInvoker, TaskInitializationPayload,
        };
        use meld_execution::task_network::dispatch::Claim;
        use meld_execution::task_network::dispatch_actor::{
            ClaimedInvocationOutcome, ClaimedTaskInvoker, DispatchPortError, PackageRunPreparer,
            PreparedPackageRun,
        };
        use meld_execution::task_network::state::TaskNode;

        struct StubPreparer;
        impl PackageRunPreparer for StubPreparer {
            fn prepare_package_run(
                &self,
                _plan: &TaskPackageRoutePlan,
                _task_run_id: &str,
            ) -> Result<PreparedPackageRun, DispatchPortError> {
                Err(DispatchPortError::retryable("stub preparer"))
            }
        }

        struct StubStepInvoker;
        #[async_trait::async_trait]
        impl PackageStepInvoker for StubStepInvoker {
            async fn invoke_capability(
                &self,
                _instance: &BoundCapabilityInstance,
                _payload: &CapabilityInvocationPayload,
            ) -> Result<CapabilityInvocationResult, meld_execution::error::ApiError> {
                Err(meld_execution::error::ExecutionInvariantError::ConfigError(
                    "stub step invoker".to_string(),
                ))
            }

            fn compile_expansion(
                &self,
                _compiled_task: &CompiledTaskRecord,
                _request: &TaskExpansionRequest,
            ) -> Result<CompiledTaskDelta, meld_execution::error::ApiError> {
                Err(meld_execution::error::ExecutionInvariantError::ConfigError(
                    "stub step invoker".to_string(),
                ))
            }
        }

        struct StubClaimInvoker;
        #[async_trait::async_trait]
        impl ClaimedTaskInvoker for StubClaimInvoker {
            async fn invoke_claimed_task(
                &self,
                _node: &TaskNode,
                _claim: &Claim,
                _init_payload: &TaskInitializationPayload,
            ) -> Result<ClaimedInvocationOutcome, DispatchPortError> {
                Err(DispatchPortError::retryable("stub claim invoker"))
            }
        }

        DispatchRouteBindings {
            preparer: SharedPackageRunPreparer(Arc::new(StubPreparer)),
            package_invoker: SharedPackageStepInvoker(Arc::new(StubStepInvoker)),
            claim_invoker: SharedClaimedTaskInvoker(Arc::new(StubClaimInvoker)),
        }
    }

    #[test]
    fn late_bound_dispatch_routes_resolve_the_dispatch_actor() {
        let harness = StewardshipHarness::new();
        let assembly = harness.assembly(StewardshipTheoryBindings::default());

        // Without composed routes the dispatch factory truthfully carries
        // no semantic body and the route seed names the composition inputs.
        let factory = assembly
            .handle_factories()
            .get("execution.task_dispatch")
            .unwrap();
        assert!(!factory.has_semantic_body());
        assert!(!assembly.dispatch_routes_bound());
        let seed = assembly.dispatch_route_seed().unwrap();
        assert_eq!(seed.subject_path, PathBuf::from("docs"));
        assert_eq!(seed.agent_id, STEWARD_AGENT_ID);
        assert_eq!(seed.provider_id, "main-provider");
        assert_eq!(seed.session_id, "stewardship::docs_freshness");
        assert_eq!(seed.workspace_root, harness.binding.workspace_root);

        // Binding production-shaped routes resolves the actor; the first
        // binding wins and later binds are no-ops.
        assert!(assembly.bind_dispatch_routes(stub_routes()));
        assert!(!assembly.bind_dispatch_routes(stub_routes()));
        assert!(assembly.dispatch_routes_bound());
        let factory = assembly
            .handle_factories()
            .get("execution.task_dispatch")
            .unwrap();
        assert!(factory.has_semantic_body());
        assert!(factory.build_handle().has_semantic_body());
    }

    #[test]
    fn plain_composition_has_no_dispatch_route_slot() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();

        assert!(assembly.dispatch_route_seed().is_none());
        assert!(!assembly.dispatch_routes_bound());
        assert!(!assembly.bind_dispatch_routes(stub_routes()));
    }

    #[test]
    fn satisfaction_sequences_are_durably_monotonic_across_assemblies() {
        let harness = StewardshipHarness::new();
        {
            let assembly = harness.assembly(StewardshipTheoryBindings::default());
            harness.run_world_genesis(&assembly);
        }

        let first_sequence = {
            let assembly = harness.assembly(StewardshipTheoryBindings::default());
            let sequence = DurableStepSequence::new(
                Arc::clone(assembly.stores().belief_store.opened().unwrap()),
                "world_model.satisfaction_curation",
            );
            let first = sequence.next().unwrap();
            let second = sequence.next().unwrap();
            assert!(second > first);
            second
        };

        // A fresh assembly over the same durable root continues the ratchet
        // instead of restarting it: injected sequences never regress.
        let assembly = harness.assembly(StewardshipTheoryBindings::default());
        let sequence = DurableStepSequence::new(
            Arc::clone(assembly.stores().belief_store.opened().unwrap()),
            "world_model.satisfaction_curation",
        );
        assert!(sequence.next().unwrap() > first_sequence);
    }

    #[test]
    fn explicit_single_actor_registration_set_opens_only_its_stores() {
        let temp = tempfile::tempdir().unwrap();
        let ledger_dir = temp.path().join("ledger");
        std::fs::create_dir_all(&ledger_dir).unwrap();
        let authority = open_external_authority(&ledger_dir);
        let product_root = temp.path().join("product");
        let mut config = ProductRuntimeConfig::for_product_root(&product_root);
        config.registration_set = Some(RegistrationSet {
            registrations: vec![RuntimeRegistration {
                registration_id: "isolate::belief_assessment".to_string(),
                runtime_id: "world_model.belief_assessment".to_string(),
                kind: RegistrationKind::ActiveActor,
                required_resources: Vec::new(),
            }],
        });

        let assembly = ProductRuntimeAssembly::load_composed(config, authority, None).unwrap();

        let layout = assembly.layout();
        assert!(layout.world_model_db.exists());
        assert!(!layout.workspace_db.exists());
        assert!(!layout.execution_goals_db.exists());
        assert!(!layout.task_artifacts_db.exists());
        assert!(!layout.task_networks_root.exists());
        assert!(!layout.frame_blob_root.exists());
        assert!(!layout.prompt_artifact_root.exists());
        assert!(assembly.stores().belief_store.is_open());
        assert!(!assembly.stores().goal_store.is_open());
        assert!(!assembly.stores().node_store.is_open());
        assert!(assembly.try_graph_runtime().is_some());
    }

    #[test]
    fn package_route_handoffs_derive_run_ids_through_the_exported_derivation() {
        let handoffs = PackageRouteHandoffs::default();
        handoffs.record(route_plan("plan-a"));
        // Duplicate plan ids dedupe to one recorded handoff.
        handoffs.record(route_plan("plan-a"));

        let bindings = handoffs.run_bindings();

        assert_eq!(
            bindings,
            vec![("plan-a".to_string(), package_route_run_id("plan-a"))]
        );
    }

    fn route_plan(plan_id: &str) -> TaskPackageRoutePlan {
        TaskPackageRoutePlan {
            plan_id: plan_id.to_string(),
            network_id: "stewardship.docs_freshness".to_string(),
            composition_id: "composition-a".to_string(),
            goal_id: "goal-a".to_string(),
            method_id: "method-a".to_string(),
            action_id: "action-a".to_string(),
            package_id: "docs_writer".to_string(),
            workflow_id: "docs_writer_thread_v1".to_string(),
            outcome_contract_id: "execution.package.aggregate.v1".to_string(),
            artifact: meld_execution::planning::ActionArtifactMeaning {
                artifact_type_id: "docs_patch".to_string(),
                schema_version: 1,
            },
            world_state_frame: meld_execution::planning::PlanningWorldStateFrameRef {
                frame_id: "frame-1".to_string(),
                projection_version: "world_model.planner.v1".to_string(),
                perspective_id: "default".to_string(),
                branch_id: "main".to_string(),
                source_refs: Vec::new(),
                warnings: Vec::new(),
            },
        }
    }

    #[test]
    fn no_folder_work_units_is_a_fatal_classification_mismatch() {
        let (fatal, issue) = aggregate_publish_issue(
            "plan-a",
            AggregatePublicationError::NoFolderWorkUnits {
                package_run_id: "package-route::plan-a".to_string(),
            },
        );
        assert!(fatal);
        assert_eq!(issue.code, "aggregate_classification_mismatch");

        let (retryable_fatal, retryable_issue) = aggregate_publish_issue(
            "plan-a",
            AggregatePublicationError::Storage("io".to_string()),
        );
        assert!(!retryable_fatal);
        assert_eq!(retryable_issue.code, "aggregate_publication_failed");
    }

    #[test]
    fn folder_unit_capability_types_derive_from_the_selected_package() {
        let types = folder_unit_capability_types("docs_freshness").unwrap();

        assert!(types.contains(&"provider_execute_chat".to_string()));
        assert!(types.contains(&"context_generate_prepare".to_string()));
        assert!(types.contains(&"context_generate_finalize".to_string()));
        assert!(folder_unit_capability_types("code_health").is_err());
    }

    #[test]
    fn stewardship_subject_ref_is_a_workspace_node() {
        let harness = StewardshipHarness::new();

        let subject = stewardship_subject_ref(&harness.binding).unwrap();

        assert_eq!(subject.domain_id, "workspace_fs");
        assert_eq!(subject.object_kind, "node");
        assert_eq!(subject.object_id, "docs");
    }

    /// The publication tick declares quiet only when the outbox emptiness
    /// was actually confirmed: a composed empty network and an absent
    /// network both narrate, each with its own detail.
    #[test]
    fn publication_tick_declares_quiet_only_when_confirmed() {
        let harness = StewardshipHarness::new();
        let bindings = StewardshipActorBindings::derive(&harness.binding).unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let execution_db = sled::open(store_dir.path().join("execution")).unwrap();
        let progress = meld_execution::task::TaskProgressStore::open(execution_db.clone()).unwrap();
        let outbox =
            meld_execution::task_network::aggregate_publication::AggregatePublicationStore::open(
                execution_db.clone(),
            )
            .unwrap();
        let network =
            SledTaskNetworkStore::open(execution_db, bindings.network_id.clone()).unwrap();

        let mut handle = PublicationHandle {
            stores: Ok((progress, outbox)),
            event_append: crate::runtime::ports::ProductEventAppendPort::new(&harness.authority),
            handoffs: Arc::new(PackageRouteHandoffs::default()),
            bindings,
            worker_id: "worker-test".to_string(),
            network: Some(Arc::new(Mutex::new(network))),
        };

        let report = handle.tick(WorkBudget { max_items: 8 });
        assert!(report.retryable_errors.is_empty(), "{report:?}");
        assert_eq!(report.waiting_on.len(), 1, "{report:?}");
        assert_eq!(report.waiting_on[0].condition, "no_pending_publications");
        assert!(report.waiting_on[0]
            .detail
            .contains("no pending task publications"));

        // Without a composed network the declaration carries the
        // no-network detail instead of claiming a confirmed empty outbox.
        handle.network = None;
        let report = handle.tick(WorkBudget { max_items: 8 });
        assert_eq!(report.waiting_on.len(), 1, "{report:?}");
        assert_eq!(report.waiting_on[0].condition, "no_pending_publications");
        assert!(report.waiting_on[0].detail.contains("no task network"));
    }
}
