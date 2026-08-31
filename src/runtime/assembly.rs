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
use meld_lang::{AuthorityPolicyBinding, Method};
use meld_lang::{Goal, GoalLifecycle, GoalSource};
use meld_world_model::agent::{
    AgentActivationStatus, AgentAuthorizationFence, AgentReconciliationActor,
    AgentReconciliationReport, AgentStatus, AgentStore, AGENT_RECONCILIATION_RUNTIME_ID,
};
use meld_world_model::belief::{
    BeliefAssessmentActor, BeliefAssessmentReport, BeliefAssessmentRequest, BeliefFamilyRegistry,
    BeliefFamilyRegistryStore, BeliefStore, BeliefSubjectBinding, BranchScope,
    ConfiguredOutcomeMappingSet, EvidenceEventReplaySource, EvidenceIngestionActor,
    EvidenceIngestionReport, EvidenceIngestionRequest, OutcomeEvidenceMapping,
    OutcomeMappingSetConfig,
};
use meld_world_model::world_state::graph::contracts::{
    OwnerCurrentnessPolicy, TraversalCutRequest, TraversalOwnerRequirement,
};
use meld_world_model::world_state::graph::runtime::{GraphCatchUpBudget, GraphRuntime};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::PerspectiveKey;
use meld_world_model::{
    CurationAuthority, CurationEventPort, CurationStepReport, CurationStore, CurationTraversalPort,
    PlannerAssemblyPolicy, PlannerCurrentAssemblyRequest, PlannerDecisionContext,
    PlannerSourceKind, PlannerSourcePosition, StandingCurationActor, StandingCurationRuleRevision,
};
use serde::{Deserialize, Serialize};

use crate::capability::{
    ExactCapabilityActivationRequest, OwnerBindingView, ProductCapabilityInventory,
};
use crate::config::MerkleConfig;
use crate::config::PhysicalBinding;
use crate::runtime::contracts::{
    WorkBudget, WorkerCheckpoint, WorkerScope, WorkerTickIssue, WorkerTickReport,
};
use crate::runtime::error::{RuntimeAssemblyError, RuntimeRegistryError};
use crate::runtime::lifecycle::{
    ActivationLifecycleService, ActivationLifecycleStore, LifecycleStepOutcome,
    STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID,
};
use crate::runtime::ports::{
    ExactKeyPlanningProjectionPort, ProductAgentAuthorityPort, ProductAgentPlannerPort,
    ProductCurationTraversalPort, ProductEventAppendPort, ProductEventReplayPort,
    ProductPlannedCurationPort, ProductRuntimePorts, ProviderPortConfig, SharedClaimedTaskInvoker,
    SharedPackageRunPreparer, SharedPackageStepInvoker,
};
use crate::runtime::registration::{RegistrationKind, RegistrationSet, RuntimeRegistration};
use crate::runtime::storage::{
    OpenProductStores, ProductStorageLayout, ProductStorageRoot, StoreScope,
};
use crate::runtime::supervisor::SupervisorStore;
use crate::runtime::theory::ResolvedStewardshipTheory;

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
    capability_runtime: Option<ProductCapabilityRuntime>,
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
    /// PDS package, assignment, activation, and lifecycle storage.
    Theory,
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
        RuntimeResource::Theory => StoreScope {
            theory: true,
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
    /// Package-declared yield source as artifact type and array field,
    /// derived from the selected package document.
    pub semantic_yield_source: Option<(String, String)>,
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
            folder_unit_capability_types: Vec::new(),
            semantic_yield_source: None,
            expression,
        })
    }
}

/// Injected theory and route bindings for one stewardship composition.
///
/// Production assembly hydrates every semantic body from one complete exact
/// receipt snapshot. The loose fields below remain only for compatibility
/// fixtures while they migrate to installed receipts.
// TODO compat-shim: remove loose fields after every harness fixture builds a
// complete receipt-resolved snapshot and compatibility parity remains green.
#[derive(Default)]
pub struct StewardshipTheoryBindings {
    /// Complete exact semantic image injected by receipt-aware harness callers.
    ///
    /// Production assembly resolves this value from the active receipt.
    pub resolved: Option<Arc<ResolvedStewardshipTheory>>,
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
    /// Activated world-model Strategy problem template for Goal curation.
    ///
    /// `None` deliberately retains compatibility curation until root assembly
    /// can supply an authored, content-identified theory snapshot.
    pub strategy: Option<meld_world_model::AgentStrategyRuntimeConfig>,
    /// Live atomic contracts and matching invokers shared across execution.
    pub capability_runtime: Option<ProductCapabilityRuntime>,
    /// Exact effective-authority policy shared by judgment and execution.
    pub authority_policy: Option<AuthorityPolicyBinding>,
    /// Planning theory: methods, catalog, afforded actions, realizations.
    pub planning: Option<PlanningTheoryBinding>,
    /// Real execution route bindings for the dispatch actor.
    pub dispatch: Option<DispatchRouteBindings>,
}

/// Product-neutral capability runtime shared by Strategy, planning, and
/// dispatch for one stewardship composition.
#[derive(Clone)]
pub struct ProductCapabilityRuntime {
    /// Exact contracts visible to planning and lowering.
    pub catalog: CapabilityCatalog,
    /// Matching executable invokers visible to dispatch.
    pub registry: crate::capability::CapabilityExecutorRegistry,
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
    /// Belief family selected by the stewardship declaration.
    pub belief_family_id: String,
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

fn hydrate_stewardship_theory(
    stores: &OpenProductStores,
    binding: &PhysicalBinding,
    theory: &mut StewardshipTheoryBindings,
    diagnostics: &mut Vec<AssemblyDiagnostic>,
) {
    let subject = match DomainObjectRef::new("workspace_fs", "node", binding.subject.clone()) {
        Ok(subject) => subject,
        Err(error) => {
            diagnostics.push(AssemblyDiagnostic {
                code: "theory_image_inconsistent".to_string(),
                message: error.to_string(),
            });
            return;
        }
    };
    let resolved = match theory.resolved.clone() {
        Some(resolved) => resolved,
        None => match ResolvedStewardshipTheory::resolve(stores, &binding.package, &subject) {
            Ok(resolved) => Arc::new(resolved),
            Err(error) => {
                diagnostics.push(AssemblyDiagnostic {
                    code: error
                        .to_string()
                        .split(':')
                        .next()
                        .unwrap_or("theory_image_unresolved")
                        .to_string(),
                    message: error.to_string(),
                });
                return;
            }
        },
    };
    if let Err(error) = resolved.validate_activation(&binding.package, &subject) {
        diagnostics.push(AssemblyDiagnostic {
            code: "theory_image_inconsistent".to_string(),
            message: error.to_string(),
        });
        return;
    }
    let contracts = resolved
        .executable_contracts
        .iter()
        .map(|revision| revision.contract.clone())
        .collect::<Vec<_>>();
    let capability_runtime = match activate_exact_capabilities(binding, &resolved, &contracts) {
        Ok(runtime) => runtime,
        Err(error) => {
            diagnostics.push(AssemblyDiagnostic {
                code: "theory_image_inconsistent".to_string(),
                message: error.to_string(),
            });
            return;
        }
    };
    let mut strategy = match meld_world_model::AgentStrategyRuntimeConfig::activate_installed(
        resolved.strategy_theory.package.clone(),
        subject,
        binding.agent_id.clone(),
    ) {
        Ok(strategy) => strategy,
        Err(error) => {
            diagnostics.push(AssemblyDiagnostic {
                code: "theory_image_inconsistent".to_string(),
                message: error.to_string(),
            });
            return;
        }
    };
    let authority_policy = match resolved.authority_policy.binding() {
        Ok(policy) => policy,
        Err(error) => {
            diagnostics.push(AssemblyDiagnostic {
                code: "theory_image_inconsistent".to_string(),
                message: error.to_string(),
            });
            return;
        }
    };
    strategy.theory_revision = Some(resolved.strategy_theory.revision_ref());
    strategy = strategy.with_authority_policy(authority_policy.clone());
    theory.outcome_mapping = Some(resolved.outcome_mapping.config.clone());
    theory.strategy = Some(strategy);
    theory.planning = Some(PlanningTheoryBinding {
        methods: Vec::new(),
        capability_catalog: capability_runtime.catalog.clone(),
        available_actions: AvailableActionSet {
            actions: Vec::new(),
        },
        method_realizations: Vec::new(),
        requested_dimensions: resolved
            .strategy_theory
            .package
            .requested_dimensions
            .clone(),
    });
    theory.capability_runtime = Some(capability_runtime);
    theory.authority_policy = Some(authority_policy);
    theory.resolved = Some(resolved);
}

/// Bind built-in product capability implementations by exact contract.
///
/// This adapter composes implementation publishers. It never selects by
/// stewardship expression and it rejects any installed contract for which
/// the process has no exact invoker.
fn activate_exact_capabilities(
    binding: &PhysicalBinding,
    resolved: &ResolvedStewardshipTheory,
    contracts: &[crate::capability::CapabilityTypeContract],
) -> Result<ProductCapabilityRuntime, crate::error::ApiError> {
    let inventory: ProductCapabilityInventory =
        crate::capability::product_capability_inventory()
            .map_err(|error| crate::error::ApiError::ConfigError(error.to_string()))?;
    let selected_contracts = resolved
        .executable_contracts
        .iter()
        .map(|revision| revision.revision_ref())
        .collect::<Vec<_>>();
    let selected_implementations = selected_contracts
        .iter()
        .map(|contract_ref| {
            inventory
                .unique_implementation_ref(contract_ref)
                .map(|implementation_ref| (contract_ref.clone(), implementation_ref))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()
        .map_err(|error| crate::error::ApiError::ConfigError(error.to_string()))?;
    let prepared = inventory
        .prepare(
            ExactCapabilityActivationRequest {
                assignment_id: format!("legacy::{}::{}", binding.agent_id, binding.subject),
                activation_id: format!(
                    "legacy::{}::{}",
                    binding.workspace_root.display(),
                    binding.provider_id
                ),
                selected_contracts,
                selected_implementations,
            },
            &OwnerBindingView::new(BTreeMap::from([
                (
                    "workspace".to_string(),
                    binding.workspace_root.display().to_string(),
                ),
                ("subject".to_string(), binding.subject.clone()),
                ("agent".to_string(), binding.agent_id.clone()),
                ("provider".to_string(), binding.provider_id.clone()),
            ])),
        )
        .map_err(|error| crate::error::ApiError::ConfigError(error.to_string()))?;
    for contract in contracts {
        if prepared
            .invokers
            .get(&contract.capability_type_id, contract.capability_version)
            .is_none()
        {
            return Err(crate::error::ApiError::ConfigError(format!(
                "installed capability '{}' version '{}' has no product invoker",
                contract.capability_type_id, contract.capability_version
            )));
        }
    }
    Ok(ProductCapabilityRuntime {
        catalog: prepared.contracts,
        registry: prepared.invokers,
    })
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
        belief_family_id: binding.package.belief_family_id.clone(),
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
    family_revision: Option<meld_world_model::belief::BeliefFamilyRevision>,
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
    mapping_revision: Option<meld_world_model::belief::TheoryRevisionRef>,
    perspective: PerspectiveKey,
    branch_scope: BranchScope,
    family_revision: Option<meld_world_model::belief::BeliefFamilyRevision>,
}

#[derive(Clone)]
struct StandingCurationFactory {
    runtime_id: String,
    session_id: String,
    authority: CurationAuthority,
    rule: StandingCurationRuleRevision,
    store: Arc<CurationStore>,
    traversal: ProductCurationTraversalPort,
    events: ProductEventAppendPort,
}

#[derive(Clone)]
struct AgentActorFactory {
    runtime_id: String,
    goal: Goal,
    store: Arc<AgentStore>,
    planner: Arc<dyn meld_world_model::AgentPlannerPort>,
    authority_port: Arc<dyn meld_world_model::AgentAuthorityPort>,
    frozen_authority: AgentAuthorizationFence,
    curation: Arc<dyn meld_world_model::AgentCurationPort>,
    strategy: meld_world_model::AgentStrategyRuntimeConfig,
    authority: CurationAuthority,
    rule: StandingCurationRuleRevision,
    #[cfg(test)]
    planner_source_positions: Vec<PlannerSourcePosition>,
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
    authority_policy: Option<AuthorityPolicyBinding>,
}

#[derive(Clone)]
struct DispatchFactory {
    /// Late-binding route slot: an unbound slot builds a body-less handle.
    routes: DispatchRouteSlot,
    execution_db: sled::Db,
    network: Arc<Mutex<SledTaskNetworkStore>>,
    handoffs: Arc<PackageRouteHandoffs>,
    worker_id: String,
    authority_policy: Option<AuthorityPolicyBinding>,
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
    ActivationLifecycle { service: ActivationLifecycleService },
    GraphReplay { graph_runtime: Arc<GraphRuntime> },
    EventAppend { port: ProductEventAppendPort },
    BeliefAssessment(Box<BeliefAssessmentFactory>),
    EvidenceIngestion(Box<EvidenceIngestionFactory>),
    StandingCuration(Box<StandingCurationFactory>),
    AgentActor(Box<AgentActorFactory>),
    Planning(Box<PlanningFactory>),
    Dispatch(Box<DispatchFactory>),
    AggregatePublication(Box<PublicationFactory>),
}

enum RuntimeSemanticHandle {
    None,
    ActivationLifecycle(ActivationLifecycleHandle),
    GraphReplay(GraphReplayRuntimeHandle),
    EventAppend(EventAppendRuntimeHandle),
    BeliefAssessment(Box<BeliefAssessmentHandle>),
    EvidenceIngestion(Box<EvidenceIngestionHandle>),
    StandingCuration(Box<StandingCurationHandle>),
    AgentActor(Box<AgentActorHandle>),
    Planning(Box<PlanningHandle>),
    Dispatch(Box<DispatchHandle>),
    AggregatePublication(Box<PublicationHandle>),
}

#[derive(Clone)]
struct GraphReplayRuntimeHandle {
    graph_runtime: Arc<GraphRuntime>,
}

struct ActivationLifecycleHandle {
    service: ActivationLifecycleService,
    transition_sequence: u64,
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

struct StandingCurationHandle {
    actor: StandingCurationActor,
}

struct AgentActorHandle {
    runtime_id: String,
    agent_id: String,
    actor: AgentReconciliationActor,
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
            // A bounded actor may legitimately wait on provider I/O. Keep
            // its lease aligned with the existing fifteen minute generation
            // wait bound so another supervisor cannot steal live work.
            lease_duration_ms: 15 * 60 * 1_000,
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
        let mut scope = registration_set
            .as_ref()
            .map(|set| scope_for_registration_set(set, &registry))
            .unwrap_or_else(StoreScope::all);
        if stewardship.is_some() {
            scope.theory = true;
        }

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
            Some(mut composition) => {
                hydrate_stewardship_theory(
                    stores.as_ref(),
                    &composition.binding,
                    &mut composition.theory,
                    &mut diagnostics,
                );
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
        let capability_runtime = composed_stewardship
            .as_ref()
            .and_then(|composed| composed.theory.capability_runtime.clone());

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
            capability_runtime,
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

    /// Return the exact capability runtime shared by planning and dispatch.
    pub fn capability_runtime(&self) -> Option<&ProductCapabilityRuntime> {
        self.capability_runtime.as_ref()
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
            RuntimeFactoryDescriptor::new(
                "world_model.standing_curation",
                vec![EventAppend, WorldModel],
            )?,
            RuntimeFactoryDescriptor::new("world_model.belief_assessment", vec![WorldModel])?,
            RuntimeFactoryDescriptor::new(
                AGENT_RECONCILIATION_RUNTIME_ID,
                vec![PlannerProjection, WorldModel],
            )?,
            RuntimeFactoryDescriptor::new(
                "world_model.evidence_ingestion",
                vec![EventReplay, EventConsumerRegistry, WorldModel],
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
            RuntimeFactoryDescriptor::new(STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID, vec![Theory])?,
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
            STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID => {
                let Some(theory_db) = stores.theory_db.opened() else {
                    return unresolved(
                        diagnostics,
                        "pds_activation_lifecycle_store_unresolved",
                        "PDS activation lifecycle requires the theory store scope".to_string(),
                    );
                };
                let store = ActivationLifecycleStore::new(theory_db.clone()).map_err(|error| {
                    RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                })?;
                Ok(Self::ActivationLifecycle {
                    service: ActivationLifecycleService::new(store),
                })
            }
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
            "world_model.standing_curation" => {
                let Some(composed) = stewardship else {
                    return Ok(Self::None);
                };
                let (Some(curation_store), Some(traversal_store), Some(agent_store)) = (
                    stores.curation_store.opened(),
                    stores.traversal_store.opened(),
                    stores.agent_store.opened(),
                ) else {
                    return Ok(Self::None);
                };
                let agent_id = composed.bindings.agent_id.clone();
                let agent = match agent_store.get_agent(&agent_id) {
                    Ok(Some(agent)) => agent,
                    Ok(None) => {
                        return unresolved(
                            diagnostics,
                            "standing_curation_agent_unresolved",
                            format!(
                                "Agent '{agent_id}' has no durable genesis record; standing Curation stays unresolved"
                            ),
                        )
                    }
                    Err(error) => {
                        return unresolved(
                            diagnostics,
                            "standing_curation_agent_probe_failed",
                            format!("standing Curation Agent probe failed: {error}"),
                        )
                    }
                };
                if agent.status != AgentStatus::Operational {
                    return unresolved(
                        diagnostics,
                        "standing_curation_agent_inactive",
                        format!(
                            "Agent '{agent_id}' is not operational; standing Curation stays unresolved"
                        ),
                    );
                }
                let rule = match curation_store.active_rule(&agent_id) {
                    Ok(Some(rule)) => rule,
                    Ok(None) => {
                        return unresolved(
                            diagnostics,
                            "standing_curation_rule_unresolved",
                            format!("Agent '{agent_id}' has no installed standing Curation rule"),
                        )
                    }
                    Err(error) => {
                        return unresolved(
                            diagnostics,
                            "standing_curation_rule_probe_failed",
                            format!("standing Curation rule probe failed: {error}"),
                        )
                    }
                };
                let activation = match agent_store.activations_for_agent(&agent_id) {
                    Ok(activations) => activations.into_iter().next_back(),
                    Err(error) => {
                        return unresolved(
                            diagnostics,
                            "standing_curation_activation_probe_failed",
                            format!("standing Curation activation probe failed: {error}"),
                        )
                    }
                };
                let Some(activation) = activation else {
                    return unresolved(
                        diagnostics,
                        "standing_curation_activation_unresolved",
                        format!(
                            "Agent '{agent_id}' has no activated generation for standing Curation"
                        ),
                    );
                };
                if activation.status != AgentActivationStatus::Activated {
                    return unresolved(
                        diagnostics,
                        "standing_curation_activation_inactive",
                        format!("Agent '{agent_id}' latest activation generation is not activated"),
                    );
                }
                let authority = CurationAuthority {
                    agent_id,
                    perspective: agent.perspective_key,
                    branch_scope: agent.branch_scope,
                    activation_generation: activation.activation_id,
                    subject: agent.subject,
                };
                authority.validate().map_err(|error| {
                    RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                })?;
                rule.rule.validate().map_err(|error| {
                    RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                })?;
                Ok(Self::StandingCuration(Box::new(StandingCurationFactory {
                    runtime_id: runtime_id.to_string(),
                    session_id: composed.bindings.session_id.clone(),
                    authority,
                    rule,
                    store: Arc::clone(curation_store),
                    traversal: ProductCurationTraversalPort::new(Arc::clone(traversal_store)),
                    events: ports.event_append().clone(),
                })))
            }
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
                if composed.theory.resolved.is_none()
                    && !family_installed(registry, &family_id, runtime_id, diagnostics)
                {
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
                    family_revision: composed
                        .theory
                        .resolved
                        .as_ref()
                        .map(|resolved| resolved.belief_family.clone()),
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
                if composed.theory.resolved.is_none()
                    && !family_installed(registry, &family_id, runtime_id, diagnostics)
                {
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
                        mapping_revision: composed
                            .theory
                            .resolved
                            .as_ref()
                            .map(|resolved| resolved.outcome_mapping.revision_ref()),
                        perspective: composed.bindings.perspective.clone(),
                        branch_scope: composed.bindings.branch_scope.clone(),
                        family_revision: composed
                            .theory
                            .resolved
                            .as_ref()
                            .map(|resolved| resolved.belief_family.clone()),
                    },
                )))
            }
            AGENT_RECONCILIATION_RUNTIME_ID => {
                let Some(composed) = stewardship else {
                    return Ok(Self::None);
                };
                let (
                    Some(belief),
                    Some(traversal),
                    Some(agent_store),
                    Some(curation_store),
                    Some(registry),
                    Some(theory_receipts),
                    Some(pds_packages),
                ) = (
                    stores.belief_store.opened(),
                    stores.traversal_store.opened(),
                    stores.agent_store.opened(),
                    stores.curation_store.opened(),
                    stores.belief_family_registry.opened(),
                    stores.theory_receipts.opened(),
                    stores.pds_packages.opened(),
                )
                else {
                    return Ok(Self::None);
                };
                let agent_id = composed.bindings.agent_id.clone();
                let agent = match agent_store.get_agent(&agent_id) {
                    Ok(Some(agent)) => agent,
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
                };
                let Some(resolved) = composed.theory.resolved.as_ref() else {
                    return unresolved(
                        diagnostics,
                        "agent_reasoning_theory_unresolved",
                        "Agent reconciliation requires one complete installed theory receipt"
                            .to_string(),
                    );
                };
                let Some(strategy) = composed.theory.strategy.clone() else {
                    return unresolved(
                        diagnostics,
                        "agent_strategy_unresolved",
                        "Agent reconciliation requires installed Strategy theory".to_string(),
                    );
                };
                let activation = agent_store
                    .activations_for_agent(&agent_id)
                    .map_err(|error| {
                        RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                    })?
                    .into_iter()
                    .next_back()
                    .filter(|activation| activation.status == AgentActivationStatus::Activated);
                let Some(activation) = activation else {
                    return unresolved(
                        diagnostics,
                        "agent_reconciliation_activation_unresolved",
                        format!("Agent '{agent_id}' has no active generation"),
                    );
                };
                let rule = curation_store
                    .active_rule(&agent_id)
                    .map_err(|error| {
                        RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                    })?
                    .ok_or_else(|| {
                        RuntimeAssemblyError::RuntimeHandleConstruction(
                            "Agent reconciliation Curation rule is not installed".to_string(),
                        )
                    })?;
                let condition = &resolved.maintained_condition.condition;
                let goal_id = format!(
                    "agent-goal::{}::{}::{}",
                    agent_id, condition.condition_id, activation.activation_id
                );
                let goal = Goal {
                    goal_id: goal_id.clone(),
                    agent_id: agent_id.clone(),
                    target: condition
                        .target_for(agent.subject.clone())
                        .map_err(|error| {
                            RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                        })?,
                    priority: condition.goal_priority.clone(),
                    source: GoalSource::MaintainedConditionBreach {
                        maintained_condition_id: condition.condition_id.clone(),
                        dimension: condition.dimension_id.clone(),
                        observed: "requires_reconciliation".to_string(),
                        desired: condition.desired_summary.clone(),
                    },
                    lifecycle: GoalLifecycle::Proposed,
                };
                let authority_scope_id = strategy
                    .authority_policy
                    .as_ref()
                    .map(|binding| binding.policy.policy_id.clone())
                    .unwrap_or_else(|| "world_model.agent_reconciliation".to_string());
                let context = PlannerDecisionContext {
                    context_id: format!("agent-context::{goal_id}"),
                    agent_id: agent_id.clone(),
                    goal_id,
                    subject: agent.subject.clone(),
                    scope_id: rule.rule.scope.scope_id.clone(),
                    branch_id: agent.branch_scope.branch_id.clone(),
                    perspective_id: agent.perspective_key.perspective_id.clone(),
                    authority_scope_id: authority_scope_id.clone(),
                    activation_generation: activation.activation_id.clone(),
                };
                let source =
                    |kind,
                     owner_id: &str,
                     source_id: &str,
                     revision_id: &str,
                     content_hash: &str| PlannerSourcePosition {
                        kind,
                        owner_id: owner_id.to_string(),
                        source_id: source_id.to_string(),
                        revision_id: revision_id.to_string(),
                        content_hash: content_hash.to_string(),
                        scope_id: context.scope_id.clone(),
                        branch_id: context.branch_id.clone(),
                        perspective_id: context.perspective_id.clone(),
                        authority_scope_id: context.authority_scope_id.clone(),
                        invalidated_by_revision_id: None,
                    };
                let capability_catalog_revision_id = {
                    let bytes = serde_json::to_vec(&resolved.receipt.executable_contracts)
                        .map_err(|error| {
                            RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                        })?;
                    blake3::hash(&bytes).to_hex().to_string()
                };
                let family_revision = registry
                    .current(&composed.bindings.belief_family_id)
                    .map_err(|error| {
                        RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                    })?
                    .ok_or_else(|| {
                        RuntimeAssemblyError::RuntimeHandleConstruction(
                            "Agent reconciliation belief family is not installed".to_string(),
                        )
                    })?;
                let belief_key = meld_world_model::configured_belief_key(
                    &family_revision,
                    &agent.subject,
                    &agent.perspective_key,
                    &agent.branch_scope,
                );
                let current = ports.event_append().watermark().map_err(|error| {
                    RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                })?;
                let planner_source_positions = vec![
                    source(
                        PlannerSourceKind::Directive,
                        "agent",
                        &agent.agent_id,
                        &resolved.receipt.receipt_id,
                        &resolved.receipt.receipt_id,
                    ),
                    source(
                        PlannerSourceKind::MaintainedCondition,
                        "agent",
                        &condition.condition_id,
                        &resolved.maintained_condition.content_hash,
                        &resolved.maintained_condition.content_hash,
                    ),
                    source(
                        PlannerSourceKind::CapabilityCatalog,
                        "execution.capability",
                        "active-executable-contract-set",
                        &capability_catalog_revision_id,
                        &capability_catalog_revision_id,
                    ),
                    source(
                        PlannerSourceKind::CurationCatalog,
                        "curation",
                        &rule.rule_id,
                        &rule.content_hash,
                        &rule.content_hash,
                    ),
                    source(
                        PlannerSourceKind::StrategyPolicy,
                        "strategy",
                        &strategy.package.evaluation_policy.policy_id,
                        &resolved.strategy_theory.content_hash,
                        &resolved.strategy_theory.content_hash,
                    ),
                ];
                let planner_request = PlannerCurrentAssemblyRequest {
                    context: context.clone(),
                    policy: PlannerAssemblyPolicy {
                        policy_revision_id: format!(
                            "reasoning-policy::{}",
                            resolved.receipt.receipt_id
                        ),
                        required_sources: vec![
                            PlannerSourceKind::Graph,
                            PlannerSourceKind::Belief,
                            PlannerSourceKind::Directive,
                            PlannerSourceKind::MaintainedCondition,
                            PlannerSourceKind::CapabilityCatalog,
                            PlannerSourceKind::CurationCatalog,
                            PlannerSourceKind::StrategyPolicy,
                        ],
                        explicitly_not_required: vec![
                            PlannerSourceKind::Causation,
                            PlannerSourceKind::Regime,
                        ],
                    },
                    traversal_cut_request: TraversalCutRequest {
                        owners: {
                            let mut owners = vec![
                                TraversalOwnerRequirement {
                                    owner_id: rule.rule.source_owner_id.clone(),
                                    scope: rule.rule.scope.clone(),
                                    required: true,
                                },
                                TraversalOwnerRequirement {
                                    owner_id: meld_world_model::CURATION_OWNER_ID.to_string(),
                                    scope: rule.rule.scope.clone(),
                                    required: false,
                                },
                            ];
                            owners.sort();
                            owners
                        },
                        scope: rule.rule.scope.clone(),
                        currentness: OwnerCurrentnessPolicy::LatestComplete,
                        event_position: meld_events::LedgerCursor {
                            ledger_id: current.ledger_id,
                            after_seq: current.committed_seq,
                        },
                    },
                    traversal_request: rule.rule.traversal_request(),
                    belief_key,
                    unanchored_belief: family_revision.config.anchor_requirement
                        == meld_world_model::belief::AnchorRequirement::Unanchored,
                    source_positions: planner_source_positions.clone(),
                };
                let authority = CurationAuthority {
                    agent_id: agent_id.clone(),
                    perspective: agent.perspective_key,
                    branch_scope: agent.branch_scope,
                    activation_generation: activation.activation_id,
                    subject: agent.subject,
                };
                let frozen_authority = AgentAuthorizationFence {
                    activation_generation: authority.activation_generation.clone(),
                    authority_policy_content_hash: resolved
                        .receipt
                        .authority_policy
                        .content_hash
                        .clone(),
                };
                Ok(Self::AgentActor(Box::new(AgentActorFactory {
                    runtime_id: runtime_id.to_string(),
                    goal,
                    store: Arc::clone(agent_store),
                    planner: Arc::new(ProductAgentPlannerPort::new(
                        Arc::clone(belief),
                        Arc::clone(traversal),
                        ports.event_append().clone(),
                        planner_request,
                    )),
                    authority_port: Arc::new(ProductAgentAuthorityPort::new(
                        Arc::clone(agent_store),
                        Arc::clone(theory_receipts),
                        Arc::clone(pds_packages),
                        resolved.receipt.selection.clone(),
                        agent_id,
                    )),
                    frozen_authority,
                    curation: Arc::new(ProductPlannedCurationPort::new(Arc::clone(curation_store))),
                    strategy,
                    authority,
                    rule,
                    #[cfg(test)]
                    planner_source_positions,
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
                    authority_policy: composed.theory.authority_policy.clone(),
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
                    authority_policy: composed.theory.authority_policy.clone(),
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
            Self::ActivationLifecycle { service } => {
                RuntimeSemanticHandle::ActivationLifecycle(ActivationLifecycleHandle {
                    service: service.clone(),
                    transition_sequence: 0,
                })
            }
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
                    actor: {
                        let actor = BeliefAssessmentActor::new(
                            "world_model.belief_assessment",
                            Arc::clone(&factory.belief_store),
                            Arc::clone(&factory.traversal_store),
                            Arc::clone(&factory.registry)
                                as Arc<dyn BeliefFamilyRegistry + Send + Sync>,
                            vec![factory.family_id.clone()],
                            vec![factory.subject_binding.clone()],
                            factory.perspective.clone(),
                            factory.branch_scope.clone(),
                        );
                        match factory.family_revision.clone() {
                            Some(revision) => actor.with_pinned_families(vec![revision]),
                            None => actor,
                        }
                    },
                    subject_key: factory.subject_binding.subject.index_key(),
                    sequence: DurableStepSequence::new(
                        Arc::clone(&factory.belief_store),
                        "world_model.belief_assessment",
                    ),
                }))
            }
            Self::EvidenceIngestion(factory) => {
                RuntimeSemanticHandle::EvidenceIngestion(Box::new(EvidenceIngestionHandle {
                    actor: {
                        let actor = EvidenceIngestionActor::new(
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
                        );
                        let actor = match factory.mapping_revision.clone() {
                            Some(revision) => actor.with_mapping_revision(revision),
                            None => actor,
                        };
                        match factory.family_revision.clone() {
                            Some(revision) => actor.with_family_revision(revision),
                            None => actor,
                        }
                    },
                }))
            }
            Self::StandingCuration(factory) => {
                RuntimeSemanticHandle::StandingCuration(Box::new(StandingCurationHandle {
                    actor: StandingCurationActor::new(
                        factory.runtime_id.clone(),
                        factory.session_id.clone(),
                        factory.authority.clone(),
                        factory.rule.clone(),
                        Arc::clone(&factory.store),
                        Arc::new(factory.traversal.clone()) as Arc<dyn CurationTraversalPort>,
                        Arc::new(factory.events.clone()) as Arc<dyn CurationEventPort>,
                    )
                    .expect("standing Curation factory holds validated authority and rule"),
                }))
            }
            Self::AgentActor(factory) => {
                let actor = AgentReconciliationActor::new(
                    factory.runtime_id.clone(),
                    factory.goal.clone(),
                    Arc::clone(&factory.store),
                    Arc::clone(&factory.planner),
                    Arc::clone(&factory.authority_port),
                    factory.frozen_authority.clone(),
                    Arc::clone(&factory.curation),
                    factory.strategy.clone(),
                    factory.authority.clone(),
                    factory.rule.clone(),
                )
                .expect("Agent reconciliation factory holds validated exact bindings");
                RuntimeSemanticHandle::AgentActor(Box::new(AgentActorHandle {
                    runtime_id: factory.runtime_id.clone(),
                    agent_id: factory.goal.agent_id.clone(),
                    actor,
                }))
            }
            Self::Planning(factory) => RuntimeSemanticHandle::Planning(Box::new(PlanningHandle {
                actor: PlanningRuntimeActor::new(
                    {
                        let runtime = PlanningRuntime::new(
                            MethodLibrary::from_methods(
                                factory.theory.methods.clone(),
                                &factory.theory.capability_catalog,
                            ),
                            factory.theory.capability_catalog.clone(),
                        );
                        match &factory.authority_policy {
                            Some(policy) => runtime.with_authority_policy(policy.clone()),
                            None => runtime,
                        }
                    },
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
                let actor_result = DispatchRuntimeActor::new(
                    factory.worker_id.clone(),
                    factory.execution_db.clone(),
                    routes.preparer,
                    routes.package_invoker,
                    routes.claim_invoker,
                )
                .map(|actor| match &factory.authority_policy {
                    Some(policy) => actor.with_authority_policy(policy.clone()),
                    None => actor,
                });
                let (actor, construction_error) = match actor_result {
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
            Self::ActivationLifecycle(handle) => Some(handle.tick()),
            Self::GraphReplay(handle) => Some(handle.tick(budget)),
            Self::EventAppend(handle) => Some(handle.tick()),
            Self::BeliefAssessment(handle) => Some(handle.tick(budget)),
            Self::EvidenceIngestion(handle) => Some(handle.tick(budget)),
            Self::StandingCuration(handle) => Some(handle.tick(budget)),
            Self::AgentActor(handle) => Some(handle.tick(budget)),
            Self::Planning(handle) => Some(handle.tick(budget)),
            Self::Dispatch(handle) => Some(handle.tick(budget)),
            Self::AggregatePublication(handle) => Some(handle.tick(budget)),
        }
    }

    fn request_stop(&mut self) {}
}

impl ActivationLifecycleHandle {
    fn tick(&mut self) -> WorkerTickReport {
        let input = self.transition_sequence;
        match self.service.bounded_step() {
            Ok(LifecycleStepOutcome::Advanced { .. }) => {
                self.transition_sequence = self.transition_sequence.saturating_add(1);
                WorkerTickReport {
                    actor_id: STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID.to_string(),
                    scope: worker_scope("runtime", Some("pds_activation_lifecycle"), None, None),
                    input_checkpoint: WorkerCheckpoint {
                        name: "pds_lifecycle_transition_sequence".to_string(),
                        value: input,
                    },
                    output_checkpoint: WorkerCheckpoint {
                        name: "pds_lifecycle_transition_sequence".to_string(),
                        value: self.transition_sequence,
                    },
                    items_attempted: 1,
                    items_committed: 1,
                    retryable_errors: Vec::new(),
                    fatal_errors: Vec::new(),
                    budget_exhausted: false,
                    waiting_on: Vec::new(),
                }
            }
            Ok(LifecycleStepOutcome::NoWork) => WorkerTickReport {
                actor_id: STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID.to_string(),
                scope: worker_scope("runtime", Some("pds_activation_lifecycle"), None, None),
                input_checkpoint: WorkerCheckpoint {
                    name: "pds_lifecycle_transition_sequence".to_string(),
                    value: input,
                },
                output_checkpoint: WorkerCheckpoint {
                    name: "pds_lifecycle_transition_sequence".to_string(),
                    value: input,
                },
                items_attempted: 0,
                items_committed: 0,
                retryable_errors: Vec::new(),
                fatal_errors: Vec::new(),
                budget_exhausted: false,
                waiting_on: Vec::new(),
            },
            Err(error) => WorkerTickReport::fatal(
                STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID,
                "runtime",
                Some("pds_activation_lifecycle"),
                "pds_lifecycle_transition_sequence",
                "pds_lifecycle_step_failed",
                error.to_string(),
            ),
        }
    }
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

impl StandingCurationHandle {
    fn tick(&mut self, budget: WorkBudget) -> WorkerTickReport {
        standing_curation_worker_report(self.actor.bounded_step(budget.max_items))
    }
}

impl AgentActorHandle {
    fn tick(&mut self, budget: WorkBudget) -> WorkerTickReport {
        agent_step_worker_report(
            self.actor.bounded_step(budget.max_items),
            &self.runtime_id,
            &self.agent_id,
        )
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
                semantic_yield_source: self.bindings.semantic_yield_source.clone(),
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

/// Translate standing Curation persistence and publication into supervisor diagnostics.
fn standing_curation_worker_report(report: CurationStepReport) -> WorkerTickReport {
    WorkerTickReport {
        actor_id: report.actor_id,
        scope: worker_scope("world_model", Some("standing_curation"), None, None),
        input_checkpoint: WorkerCheckpoint {
            name: "standing_curation_event_cut".to_string(),
            value: report.input_after_seq,
        },
        output_checkpoint: WorkerCheckpoint {
            name: "standing_curation_event_cut".to_string(),
            value: report.output_after_seq,
        },
        items_attempted: report.operations_attempted,
        items_committed: report.acceptances_persisted
            + report.results_persisted
            + report.publications_appended,
        retryable_errors: report
            .retryable_errors
            .into_iter()
            .map(|message| WorkerTickIssue {
                item_id: None,
                code: "standing_curation_retryable".to_string(),
                message,
            })
            .collect(),
        fatal_errors: report
            .fatal_errors
            .into_iter()
            .map(|message| WorkerTickIssue {
                item_id: None,
                code: "standing_curation_fatal".to_string(),
                message,
            })
            .collect(),
        budget_exhausted: false,
        waiting_on: world_model_waiting(report.waiting_on),
    }
}

/// Translate one bounded agent step report into the supervisor shape.
///
/// Checkpoints are the durable append-only Agent reconciliation position.
fn agent_step_worker_report(
    report: AgentReconciliationReport,
    runtime_id: &str,
    agent_id: &str,
) -> WorkerTickReport {
    WorkerTickReport {
        actor_id: runtime_id.to_string(),
        scope: worker_scope("world_model", Some("agent_curation"), Some(agent_id), None),
        input_checkpoint: WorkerCheckpoint {
            name: "agent_reconciliation_records".to_string(),
            value: report.input_position,
        },
        output_checkpoint: WorkerCheckpoint {
            name: "agent_reconciliation_records".to_string(),
            value: report.output_position,
        },
        items_attempted: report.items_attempted,
        items_committed: report.records_persisted,
        retryable_errors: report
            .retryable_errors
            .into_iter()
            .map(|message| WorkerTickIssue {
                item_id: None,
                code: "agent_reconciliation_retryable".to_string(),
                message,
            })
            .collect(),
        fatal_errors: report
            .fatal_errors
            .into_iter()
            .map(|message| WorkerTickIssue {
                item_id: None,
                code: "agent_reconciliation_fatal".to_string(),
                message,
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
    use meld_execution::task_network::EventAppendSink;
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
        assert_eq!(assembly.registry().len(), 13);
        assert!(assembly
            .registry()
            .contains(AGENT_RECONCILIATION_RUNTIME_ID));
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
        assert_eq!(description.desired_runtime_state.len(), 13);
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
        assert_eq!(second.desired_runtime_state().len(), 13);
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
            11
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
    fn root_registry_exposes_one_agent_reconciliation_participant_without_execution_writers() {
        let registry = RuntimeFactoryRegistry::first_proof_registry().unwrap();
        let agent_ids = registry
            .descriptors()
            .filter(|descriptor| descriptor.runtime_id.starts_with("world_model.agent"))
            .map(|descriptor| descriptor.runtime_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(agent_ids, vec![AGENT_RECONCILIATION_RUNTIME_ID]);
        let descriptor = registry.get(AGENT_RECONCILIATION_RUNTIME_ID).unwrap();
        assert_eq!(
            descriptor.required_resources,
            vec![
                RuntimeResource::PlannerProjection,
                RuntimeResource::WorldModel
            ]
        );
        assert!(!descriptor
            .required_resources
            .contains(&RuntimeResource::GoalCommand));
        assert!(!descriptor
            .required_resources
            .contains(&RuntimeResource::GoalMutation));
    }

    #[test]
    fn root_handle_drives_mixed_plan_through_curation_to_unpublished_task_eligibility() {
        use meld_world_model::agent::AgentActivationRecord;

        let harness = StewardshipHarness::new();
        let subject = stewardship_subject_ref(&harness.binding).unwrap();
        let rule = standing_curation_rule(subject.clone());
        {
            let assembly = harness.assembly(StewardshipTheoryBindings::default());
            harness.run_world_genesis(&assembly);
            crate::docs::theory::install_package(
                assembly.stores(),
                &Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/docs_freshness"),
                5,
            )
            .unwrap();
            assembly
                .stores()
                .agent_store
                .put_activation(&AgentActivationRecord {
                    activation_id: "activation-root-v1".to_string(),
                    agent_id: STEWARD_AGENT_ID.to_string(),
                    started_at_seq: 6,
                    status: AgentActivationStatus::Activated,
                    last_error: None,
                    lease_id: Some("agent-activation-root-v1".to_string()),
                })
                .unwrap();
            assembly
                .stores()
                .curation_store
                .install_rule(rule, 7)
                .unwrap();
            assembly
                .ports()
                .event_append()
                .append_envelope_idempotent(workspace_owner_publication(
                    subject.clone(),
                    "workspace-v1",
                ))
                .unwrap();
            assembly
                .graph_runtime()
                .catch_up_bounded(GraphCatchUpBudget { max_items: 16 })
                .unwrap();
            assembly.flush_product_boundary().unwrap();
        }

        let assembly = harness.assembly(StewardshipTheoryBindings::default());
        let resolved = ResolvedStewardshipTheory::resolve(
            assembly.stores(),
            &harness.binding.package,
            &subject,
        )
        .unwrap();
        let catalog_bytes = serde_json::to_vec(&resolved.receipt.executable_contracts).unwrap();
        let catalog_revision_id = blake3::hash(&catalog_bytes).to_hex().to_string();
        let agent_factory = assembly
            .handle_factories()
            .get(AGENT_RECONCILIATION_RUNTIME_ID)
            .unwrap();
        assert!(agent_factory.has_semantic_body());
        let RuntimeSemanticHandleFactory::AgentActor(agent_semantic) = &agent_factory.semantic
        else {
            panic!("production Agent descriptor did not resolve its Agent participant");
        };
        let capability_source = agent_semantic
            .planner_source_positions
            .iter()
            .find(|position| position.kind == PlannerSourceKind::CapabilityCatalog)
            .unwrap();
        assert_eq!(capability_source.owner_id, "execution.capability");
        assert_eq!(capability_source.revision_id, catalog_revision_id);
        assert_eq!(capability_source.content_hash, catalog_revision_id);
        let strategy_source = agent_semantic
            .planner_source_positions
            .iter()
            .find(|position| position.kind == PlannerSourceKind::StrategyPolicy)
            .unwrap();
        assert_eq!(
            strategy_source.source_id,
            resolved.strategy_theory.package.evaluation_policy.policy_id
        );
        assert_eq!(
            strategy_source.revision_id,
            resolved.strategy_theory.content_hash
        );
        assert_eq!(
            strategy_source.content_hash,
            resolved.strategy_theory.content_hash
        );
        assert_ne!(
            strategy_source.content_hash,
            resolved.strategy_theory.theory_id
        );

        let mut belief = assembly
            .handle_factories()
            .get("world_model.belief_assessment")
            .unwrap()
            .build_handle();
        belief
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "world_model.belief_assessment".to_string(),
                lease_id: "belief-root-proof".to_string(),
            })
            .unwrap();
        let belief_report = belief.tick(WorkBudget { max_items: 8 }).unwrap();
        assert!(belief_report.fatal_errors.is_empty());
        assert!(belief_report.items_committed > 0);

        let mut handle = agent_factory.build_handle();
        handle
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: AGENT_RECONCILIATION_RUNTIME_ID.to_string(),
                lease_id: "agent-root-proof".to_string(),
            })
            .unwrap();
        let first = handle.tick(WorkBudget { max_items: 8 }).unwrap();
        assert_eq!(first.actor_id, AGENT_RECONCILIATION_RUNTIME_ID);
        assert!(first.fatal_errors.is_empty(), "{first:#?}");
        assert!(first.items_committed > 0);
        let mut curation = assembly
            .handle_factories()
            .get("world_model.standing_curation")
            .unwrap()
            .build_handle();
        curation
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "world_model.standing_curation".to_string(),
                lease_id: "curation-root-proof".to_string(),
            })
            .unwrap();
        let curation_report = curation.tick(WorkBudget { max_items: 1 }).unwrap();
        assert!(curation_report.fatal_errors.is_empty());
        assert!(curation_report.items_committed > 0);
        assembly
            .graph_runtime()
            .catch_up_bounded(GraphCatchUpBudget { max_items: 16 })
            .unwrap();
        assembly.flush_product_boundary().unwrap();
        drop(handle);
        drop(curation);
        drop(belief);
        drop(assembly);

        let assembly = harness.assembly(StewardshipTheoryBindings::default());
        let mut handle = assembly
            .handle_factories()
            .get(AGENT_RECONCILIATION_RUNTIME_ID)
            .unwrap()
            .build_handle();
        handle
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: AGENT_RECONCILIATION_RUNTIME_ID.to_string(),
                lease_id: "agent-root-proof-reopen".to_string(),
            })
            .unwrap();
        let second = handle.tick(WorkBudget { max_items: 8 }).unwrap();
        assert_eq!(second.input_checkpoint.value, first.output_checkpoint.value);
        assert!(second.output_checkpoint.value > second.input_checkpoint.value);
        assert!(
            second
                .waiting_on
                .iter()
                .any(|waiting| waiting.condition == "future_execution_admission"),
            "{second:#?}"
        );
        let replay = handle.tick(WorkBudget { max_items: 8 }).unwrap();
        assert_eq!(
            replay.input_checkpoint.value,
            second.output_checkpoint.value
        );
        assert_eq!(
            replay.output_checkpoint.value,
            replay.input_checkpoint.value
        );
        assert_eq!(replay.items_committed, 0);
        assert!(replay
            .waiting_on
            .iter()
            .all(|wait| wait.subject_key.is_some()));
        assembly
            .stores()
            .agent_store
            .put_activation(&AgentActivationRecord {
                activation_id: "activation-root-v2".to_string(),
                agent_id: STEWARD_AGENT_ID.to_string(),
                started_at_seq: 8,
                status: AgentActivationStatus::Activated,
                last_error: None,
                lease_id: Some("agent-activation-root-v2".to_string()),
            })
            .unwrap();
        let stale = handle.tick(WorkBudget { max_items: 8 }).unwrap();
        assert!(stale
            .waiting_on
            .iter()
            .any(|wait| wait.condition == "agent_authority_changed"));
        assert!(assembly
            .stores()
            .goal_store
            .goal_records()
            .unwrap()
            .is_empty());
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
        assert_eq!(package.handle_factories.len(), 13);
        assert_eq!(package.default_work_budget.max_items, 64);
        assert_eq!(package.lifecycle_config.heartbeat_interval_ms, 1_000);
        assert_eq!(package.lifecycle_config.lease_duration_ms, 15 * 60 * 1_000);
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

    #[test]
    fn stable_activation_lifecycle_role_runs_beneath_supervisor() {
        use crate::runtime::lifecycle::{ActivationLifecycleIntentV1, LifecycleAction};

        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let store =
            ActivationLifecycleStore::new(assembly.stores().theory_db.opened().unwrap().clone())
                .unwrap();
        store
            .submit_intent(
                ActivationLifecycleIntentV1::new(
                    "supervised-request".to_string(),
                    "assignment-a".to_string(),
                    "activation-a".to_string(),
                    None,
                    LifecycleAction::Activate,
                )
                .unwrap(),
            )
            .unwrap();

        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("lifecycle-supervisor", 100),
        )
        .unwrap();
        let tick = supervisor.tick(1_100).unwrap();
        let action = tick
            .actions
            .iter()
            .find(|action| action.runtime_id == STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID)
            .unwrap();

        assert_eq!(action.metrics.attempted, 1);
        assert_eq!(action.metrics.committed, 1);
        assert_eq!(
            lifecycle_of(
                &supervisor.status_snapshot(1_100).unwrap(),
                STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID,
            ),
            Some(RegistrationLifecycle::ActiveWorking)
        );
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
            declarations: Default::default(),
            docs_freshness: Some(DocsFreshnessSelection {
                expression: "docs_freshness".to_string(),
                target_root: workspace.to_path_buf(),
                subject: "docs".to_string(),
                agent_id: STEWARD_AGENT_ID.to_string(),
                principal_id: "workspace-owner".to_string(),
                provider_id: "main-provider".to_string(),
                theory: TheorySelection {
                    belief_family_id: FAMILY_ID.to_string(),
                    evidence_mapping_id: MAPPING_ID.to_string(),
                    curation_rule_id: "docs_freshness".to_string(),
                    maintained_condition_id: "docs_freshness".to_string(),
                    strategy_theory_id: "docs_freshness".to_string(),
                    authority_policy_id: "docs_workspace_local".to_string(),
                    claim_policy_id: "docs-claims-strict-v1".to_string(),
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

    fn standing_curation_outcome_mapping() -> OutcomeMappingSetConfig {
        OutcomeMappingSetConfig {
            mapping_id: MAPPING_ID.to_string(),
            rules: vec![OutcomeMappingConfig {
                mapping_id: "standing-curation-applied".to_string(),
                source_kind: "content_written".to_string(),
                match_domain_id: meld_world_model::CURATION_OWNER_ID.to_string(),
                match_event_type: meld_world_model::CURATION_RESULT_EVENT_TYPE.to_string(),
                match_content: vec![OutcomeContentRule::FieldEquals {
                    pointer: "/disposition".to_string(),
                    equals: "applied".to_string(),
                }],
                subject: OutcomeSubjectBinding {
                    from: Default::default(),
                    object_kind: "assessment".to_string(),
                    domain_id: Some(meld_world_model::CURATION_OWNER_ID.to_string()),
                },
                evidence_fields: vec![OutcomeFieldRule {
                    field: "stale_probability".to_string(),
                    source: OutcomeValueSource::Constant { value: 0.0 },
                }],
            }],
        }
    }

    fn standing_curation_scope(
    ) -> meld_world_model::world_state::graph::contracts::OwnerPublicationScope {
        meld_world_model::world_state::graph::contracts::OwnerPublicationScope {
            scope_id: "docs".to_string(),
            branch_id: Some("main".to_string()),
            perspective_id: Some("default".to_string()),
            valid_at: None,
        }
    }

    fn standing_curation_rule(subject: DomainObjectRef) -> meld_world_model::StandingCurationRule {
        use meld_world_model::world_state::graph::contracts::{
            TraversalBounds, TraversalDirection,
        };

        meld_world_model::StandingCurationRule {
            rule_id: "docs-standing-curation".to_string(),
            agent_id: STEWARD_AGENT_ID.to_string(),
            source_owner_id: "workspace_fs".to_string(),
            scope: standing_curation_scope(),
            roots: vec![subject],
            traversal_direction: TraversalDirection::Incoming,
            bounds: TraversalBounds {
                max_depth: 4,
                max_objects: 32,
                max_occurrences: 32,
                max_paths: 32,
            },
            expected_object_kind: "assessment".to_string(),
            expected_object_id: "docs::standing-assessment".to_string(),
            relation_type: "curation_assesses".to_string(),
            output_policy_revision: "docs-standing-output-v1".to_string(),
        }
    }

    fn workspace_owner_publication(subject: DomainObjectRef, revision: &str) -> EventEnvelope {
        use meld_world_model::world_state::graph::contracts::{
            HydrationReference, OwnerCompletenessReceipt, OwnerCompletenessStatus,
            OwnerObjectPublication, OwnerPublicationBatch, OwnerPublicationOperation,
            OwnerPublicationState,
        };
        use meld_world_model::world_state::graph::events::owner_publication_envelope;

        let publication_id = format!("workspace-publication::{revision}");
        let operation = OwnerPublicationOperation::reconstruct(
            "workspace-enumeration-v1",
            OwnerPublicationBatch {
                owner_id: "workspace_fs".to_string(),
                revision_id: revision.to_string(),
                scope: standing_curation_scope(),
                objects: vec![OwnerObjectPublication {
                    publication_id: publication_id.clone(),
                    object_ref: subject,
                    state: OwnerPublicationState::Observed,
                    source_product_ref: revision.to_string(),
                    hydration: HydrationReference {
                        owner_id: "workspace_fs".to_string(),
                        product_kind: "workspace_snapshot".to_string(),
                        product_id: revision.to_string(),
                        revision_id: revision.to_string(),
                        role: "workspace_root".to_string(),
                    },
                    provenance_refs: Vec::new(),
                    qualifications: BTreeMap::new(),
                }],
                relations: Vec::new(),
                completeness: OwnerCompletenessReceipt {
                    receipt_id: format!("workspace-completeness::{revision}"),
                    scope: standing_curation_scope(),
                    included_ids: vec![publication_id],
                    exclusions: Vec::new(),
                    failures: Vec::new(),
                    status: OwnerCompletenessStatus::Complete,
                },
            },
        )
        .unwrap();
        owner_publication_envelope("standing-curation-proof", &operation).unwrap()
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
                                maintained_condition_id: None,
                                dimension_id: "docs_freshness".to_string(),
                                threshold: 0.7,
                                priority_urgency: 8,
                                desired_summary: "fresh docs".to_string(),
                                source_kind: "docs_freshness".to_string(),
                            },
                        )
                        .unwrap(),
                    ),
                    curation_rule_revision: None,
                    maintained_condition: None,
                    maintained_condition_revision: None,
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

        assert_eq!(set.registrations.len(), 13);
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
            "world_model.standing_curation",
            AGENT_RECONCILIATION_RUNTIME_ID,
            "execution.planning",
            "execution.task_dispatch",
            "execution.publication",
            STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID,
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
            "world_model.standing_curation",
            AGENT_RECONCILIATION_RUNTIME_ID,
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
        for idle in [
            "world_model.graph_replay",
            "execution.publication",
            STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID,
        ] {
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
            vec![
                "execution.publication",
                STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID,
                "world_model.graph_replay",
            ]
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
            "execution.publication",
            STABLE_ACTIVATION_LIFECYCLE_RUNTIME_ID,
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
        assert_eq!(
            lifecycle_of(&status, AGENT_RECONCILIATION_RUNTIME_ID),
            Some(RegistrationLifecycle::UnresolvedRequiredBinding)
        );
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

    #[test]
    fn standing_curation_settles_through_root_graph_events_and_belief() {
        use meld_world_model::agent::AgentActivationRecord;
        use meld_world_model::world_state::graph::contracts::{
            OwnerCurrentnessPolicy, TraversalCutRequest, TraversalCutStatus,
            TraversalOwnerRequirement,
        };
        use meld_world_model::TraversalQuery;

        let harness = StewardshipHarness::new();
        let subject = stewardship_subject_ref(&harness.binding).unwrap();
        let rule = standing_curation_rule(subject.clone());
        {
            let assembly = harness.assembly(StewardshipTheoryBindings::default());
            harness.run_world_genesis(&assembly);
            assembly
                .stores()
                .agent_store
                .put_activation(&AgentActivationRecord {
                    activation_id: "standing-generation-a".to_string(),
                    agent_id: STEWARD_AGENT_ID.to_string(),
                    started_at_seq: 5,
                    status: AgentActivationStatus::Activated,
                    last_error: None,
                    lease_id: Some("standing-lease-a".to_string()),
                })
                .unwrap();
            assembly
                .stores()
                .curation_store
                .install_rule(rule.clone(), 6)
                .unwrap();
            assembly
                .ports()
                .event_append()
                .append_envelope_idempotent(workspace_owner_publication(
                    subject.clone(),
                    "workspace-v1",
                ))
                .unwrap();
            let mut graph = assembly
                .handle_factories()
                .get("world_model.graph_replay")
                .unwrap()
                .build_handle();
            graph
                .start_after_lease(RuntimeLeaseContext {
                    runtime_id: "world_model.graph_replay".to_string(),
                    lease_id: "graph-lease-a".to_string(),
                })
                .unwrap();
            assert_eq!(
                graph
                    .tick(WorkBudget { max_items: 32 })
                    .unwrap()
                    .items_committed,
                1
            );
            assembly.flush_product_boundary().unwrap();
        }

        let assembly = harness.assembly(StewardshipTheoryBindings {
            outcome_mapping: Some(standing_curation_outcome_mapping()),
            ..StewardshipTheoryBindings::default()
        });
        let mut curation = assembly
            .handle_factories()
            .get("world_model.standing_curation")
            .unwrap()
            .build_handle();
        curation
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "world_model.standing_curation".to_string(),
                lease_id: "curation-lease-a".to_string(),
            })
            .unwrap();

        let applied_tick = curation.tick(WorkBudget { max_items: 1 }).unwrap();

        assert_eq!(applied_tick.items_attempted, 1);
        assert_eq!(applied_tick.items_committed, 4);
        assert!(applied_tick.retryable_errors.is_empty());
        assert!(applied_tick.fatal_errors.is_empty());
        let records = assembly
            .ports()
            .event_replay()
            .read_after_limit(0, 32)
            .unwrap();
        let applied_result = records
            .iter()
            .find(|record| {
                record.envelope.event_type == meld_world_model::CURATION_RESULT_EVENT_TYPE
            })
            .map(|record| {
                serde_json::from_value::<meld_world_model::CurationResult>(
                    record.envelope.data.clone(),
                )
                .unwrap()
            })
            .unwrap();
        assert_eq!(
            applied_result.disposition,
            meld_world_model::CurationTerminalDisposition::Applied
        );
        let operation_id = applied_result.operation_id.clone();
        let result_id = applied_result.result_id.clone();

        let before_projection = assembly.ports().event_append().watermark().unwrap();
        let unprojected_cut = TraversalQuery::new(assembly.stores().traversal_store.as_ref())
            .cut(&TraversalCutRequest {
                owners: vec![TraversalOwnerRequirement {
                    owner_id: meld_world_model::CURATION_OWNER_ID.to_string(),
                    scope: standing_curation_scope(),
                    required: true,
                }],
                scope: standing_curation_scope(),
                currentness: OwnerCurrentnessPolicy::LatestComplete,
                event_position: meld_events::LedgerCursor {
                    ledger_id: before_projection.ledger_id,
                    after_seq: before_projection.committed_seq,
                },
            })
            .unwrap();
        assert_eq!(unprojected_cut.status, TraversalCutStatus::Incomplete);

        let mut graph = assembly
            .handle_factories()
            .get("world_model.graph_replay")
            .unwrap()
            .build_handle();
        graph
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "world_model.graph_replay".to_string(),
                lease_id: "graph-lease-b".to_string(),
            })
            .unwrap();
        graph.tick(WorkBudget { max_items: 32 }).unwrap();
        let watermark = assembly.ports().event_append().watermark().unwrap();
        let cut = TraversalQuery::new(assembly.stores().traversal_store.as_ref())
            .cut(&TraversalCutRequest {
                owners: vec![
                    TraversalOwnerRequirement {
                        owner_id: "workspace_fs".to_string(),
                        scope: standing_curation_scope(),
                        required: true,
                    },
                    TraversalOwnerRequirement {
                        owner_id: meld_world_model::CURATION_OWNER_ID.to_string(),
                        scope: standing_curation_scope(),
                        required: true,
                    },
                ],
                scope: standing_curation_scope(),
                currentness: OwnerCurrentnessPolicy::LatestComplete,
                event_position: meld_events::LedgerCursor {
                    ledger_id: watermark.ledger_id,
                    after_seq: watermark.committed_seq,
                },
            })
            .unwrap();
        assert_eq!(cut.status, TraversalCutStatus::Complete);
        assert_eq!(cut.receipts.len(), 2);
        let traversal = TraversalQuery::new(assembly.stores().traversal_store.as_ref())
            .traverse(&cut, &rule.traversal_request())
            .unwrap();
        let expected_object = rule.expected_object().unwrap();
        assert!(traversal
            .objects
            .iter()
            .any(|object| object.object_ref == expected_object));
        assert!(traversal
            .occurrences
            .iter()
            .any(|occurrence| occurrence.relation_type == rule.relation_type));

        let mut ingestion = assembly
            .handle_factories()
            .get("world_model.evidence_ingestion")
            .unwrap()
            .build_handle();
        ingestion
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "world_model.evidence_ingestion".to_string(),
                lease_id: "ingestion-lease-a".to_string(),
            })
            .unwrap();
        let ingestion_tick = ingestion.tick(WorkBudget { max_items: 32 }).unwrap();
        assert!(ingestion_tick.items_committed >= 2);
        let family = assembly
            .stores()
            .belief_family_registry
            .current(FAMILY_ID)
            .unwrap()
            .unwrap();
        let belief_key = configured_belief_key(
            &family,
            &expected_object,
            &PerspectiveKey::new("default", "default").unwrap(),
            &BranchScope::main(),
        );
        assert_eq!(
            assembly
                .stores()
                .belief_store
                .revision_history(&belief_key)
                .unwrap()
                .len(),
            1
        );

        assembly
            .ports()
            .event_append()
            .append_envelope_idempotent(workspace_owner_publication(subject, "workspace-v2"))
            .unwrap();
        graph.tick(WorkBudget { max_items: 32 }).unwrap();
        let unchanged_tick = curation.tick(WorkBudget { max_items: 1 }).unwrap();
        assert_eq!(unchanged_tick.items_committed, 3);
        let unchanged = assembly
            .ports()
            .event_replay()
            .read_after_limit(0, 32)
            .unwrap()
            .into_iter()
            .filter(|record| {
                record.envelope.event_type == meld_world_model::CURATION_RESULT_EVENT_TYPE
            })
            .map(|record| {
                serde_json::from_value::<meld_world_model::CurationResult>(record.envelope.data)
                    .unwrap()
            })
            .find(|result| {
                result.disposition == meld_world_model::CurationTerminalDisposition::Unchanged
            })
            .unwrap();
        assert_ne!(unchanged.operation_id, operation_id);
        ingestion.tick(WorkBudget { max_items: 32 }).unwrap();
        assert_eq!(
            assembly
                .stores()
                .belief_store
                .revision_history(&belief_key)
                .unwrap()
                .len(),
            1
        );
        assembly.flush_product_boundary().unwrap();
        drop(ingestion);
        drop(graph);
        drop(curation);
        drop(assembly);

        let reopened = harness.assembly(StewardshipTheoryBindings {
            outcome_mapping: Some(standing_curation_outcome_mapping()),
            ..StewardshipTheoryBindings::default()
        });
        let persisted =
            meld_world_model::CurationQuery::new(reopened.stores().curation_store.as_ref())
                .result_for_operation(&operation_id)
                .unwrap()
                .unwrap();
        assert_eq!(persisted.result_id, result_id);
        assert_eq!(
            persisted.disposition,
            meld_world_model::CurationTerminalDisposition::Applied
        );
        let mut reopened_graph = reopened
            .handle_factories()
            .get("world_model.graph_replay")
            .unwrap()
            .build_handle();
        reopened_graph
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "world_model.graph_replay".to_string(),
                lease_id: "graph-lease-reopen".to_string(),
            })
            .unwrap();
        reopened_graph.tick(WorkBudget { max_items: 32 }).unwrap();
        let mut reopened_curation = reopened
            .handle_factories()
            .get("world_model.standing_curation")
            .unwrap()
            .build_handle();
        reopened_curation
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "world_model.standing_curation".to_string(),
                lease_id: "curation-lease-reopen".to_string(),
            })
            .unwrap();
        let replay_tick = reopened_curation.tick(WorkBudget { max_items: 1 }).unwrap();
        assert_eq!(replay_tick.items_attempted, 1);
        assert_eq!(replay_tick.items_committed, 0);
        assert_eq!(replay_tick.waiting_on.len(), 1);
        assert_eq!(
            reopened
                .ports()
                .event_replay()
                .read_after_limit(0, 32)
                .unwrap()
                .len(),
            5
        );
        let reopened_family = reopened
            .stores()
            .belief_family_registry
            .current(FAMILY_ID)
            .unwrap()
            .unwrap();
        let reopened_key = configured_belief_key(
            &reopened_family,
            &rule.expected_object().unwrap(),
            &PerspectiveKey::new("default", "default").unwrap(),
            &BranchScope::main(),
        );
        assert_eq!(
            reopened
                .stores()
                .belief_store
                .revision_history(&reopened_key)
                .unwrap()
                .len(),
            1
        );
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
        assert_eq!(seed.belief_family_id, FAMILY_ID);
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
                AGENT_RECONCILIATION_RUNTIME_ID,
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
            AGENT_RECONCILIATION_RUNTIME_ID,
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
