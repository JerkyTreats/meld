//! Product runtime assembly for durable flywheel infrastructure.
//!
//! Owner: root runtime composition. Assembly is machine initialization
//! (Runtime Initialization stages 0, 1, and 5): it opens stores for the
//! composed registration scope, builds ports, projects registrations from
//! the accepted participant plan, and binds concrete domain actor
//! factories. It never creates semantic state — no genesis, no theory
//! install, no seeding. A world with incomplete genesis hydrates with the
//! genesis-dependent actors truthfully unresolved (no semantic body, so
//! the supervisor projects `UnresolvedRequiredBinding`), never with
//! manufactured state; a later process start after explicit initialization
//! resolves them.

#[cfg(test)]
mod docs_fixture;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[cfg(test)]
use meld_events::EventAuthorityOpenOptions;
use meld_events::EventConsumerRegistryCapability;
use meld_events::{DomainObjectRef, DurableConsumerCursor, EventAuthority};
use meld_execution::capability::CapabilityCatalog;
use meld_execution::task::TaskCompiler;
use meld_execution::task_admission::{
    TaskAdmissionLowerer, TaskAdmissionRuntimeActor, TaskAdmissionRuntimeRequest,
};
use meld_execution::task_network::dispatch_actor::{
    DispatchRuntimeActor, DispatchTickReport, DispatchTickRequest,
};
use meld_execution::task_network::{
    PublicationRuntime, PublishPendingPublicationsRequest, SledTaskNetworkStore,
};
use meld_lang::AuthorityPolicyBinding;
use meld_world_model::agent::AgentReconciliationIntent;
use meld_world_model::agent::{
    AgentAuthorizationFence, AgentReconciliationActor, AgentReconciliationReport, AgentStatus,
    AgentStore, AGENT_RECONCILIATION_RUNTIME_ID,
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
    PlannerSourceKind, PlannerSourcePosition, StandingCurationActor,
};
use serde::{Deserialize, Serialize};

use crate::capability::{
    ExactCapabilityActivationRequest, OwnerBindingView, ProductCapabilityInventory,
};
use crate::config::MerkleConfig;
use crate::config::PhysicalBinding;
use crate::runtime::contracts::{
    structural_wake_to_execution, structural_wake_to_world_model, WorkBudget, WorkerCheckpoint,
    WorkerScope, WorkerTickIssue, WorkerTickReport,
};
use crate::runtime::error::{RuntimeAssemblyError, RuntimeRegistryError};
use crate::runtime::lifecycle::{
    ActivationLifecycleStore, NativeOwnerReadinessEvidenceV1, OwnerReadinessReceiptV1,
    OwnerReleaseReceiptV1, OwnerSafePointReceiptV1, OwnerStopReceiptV1, OwnerWaitReceiptV1,
    ParticipantLifecycleContextV1, StructuralWakeRef,
};
use crate::runtime::ports::{
    ProductAdmissionGenerationObserver, ProductAgentAuthorityPort, ProductAgentExecutionPort,
    ProductAgentPlannerPort, ProductCurationAuthorityPort, ProductCurationTraversalPort,
    ProductEventAppendPort, ProductEventReplayPort, ProductPlannedCurationPort,
    ProductRuntimePorts, ProviderPortConfig, SharedClaimedTaskInvoker,
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
    lifecycle_store: Option<ActivationLifecycleStore>,
    prepared_activation: Option<crate::theory::PreparedActivationClosureV1>,
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
        RuntimeResource::WorldModel => StoreScope {
            world_model: true,
            ..StoreScope::none()
        },
        RuntimeResource::Theory => StoreScope {
            theory: true,
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

/// Project one exact prepared participant plan into runtime registrations.
///
/// The descriptor catalog supplies factories and resource requirements only.
/// It cannot add a participant absent from the accepted plan.
pub fn project_prepared_registrations(
    prepared: &crate::theory::PreparedActivationClosureV1,
    registry: &RuntimeFactoryRegistry,
) -> Result<RegistrationSet, RuntimeAssemblyError> {
    let registrations = prepared
        .participant_plan
        .participants
        .iter()
        .filter_map(|participant| {
            let descriptor = match registry.get(&participant.participant_id) {
                Some(descriptor) => descriptor,
                None if !participant.required => return None,
                None => {
                    return Some(Err(RuntimeAssemblyError::Config(format!(
                        "required prepared participant '{}' has no runtime factory",
                        participant.participant_id
                    ))));
                }
            };
            let kind = match participant.kind {
                crate::theory::ParticipantKind::PassiveSource => RegistrationKind::PassiveService,
                crate::theory::ParticipantKind::BoundedActor
                | crate::theory::ParticipantKind::DurableOperationAdapter => {
                    RegistrationKind::ActiveActor
                }
            };
            Some(Ok(RuntimeRegistration {
                registration_id: format!(
                    "prepared::{}::{}",
                    prepared.prepared_id, participant.participant_id
                ),
                runtime_id: participant.participant_id.clone(),
                kind,
                required_resources: descriptor.required_resources.clone(),
            }))
        })
        .collect::<Result<Vec<_>, RuntimeAssemblyError>>()?;
    Ok(RegistrationSet { registrations })
}

/// Canonical subject reference for one stewardship binding.
///
/// Initialization and live composition retain the complete declared identity.
pub fn stewardship_subject_ref(
    binding: &PhysicalBinding,
) -> Result<DomainObjectRef, RuntimeAssemblyError> {
    binding
        .subject
        .validate()
        .map_err(|error| RuntimeAssemblyError::Config(error.to_string()))?;
    Ok(binding.subject.clone())
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
            expression,
        })
    }
}

/// Exact theory bindings hydrated internally from one prepared product.
#[derive(Default)]
struct HydratedStewardshipTheory {
    resolved: Option<Arc<ResolvedStewardshipTheory>>,
    outcome_mapping: Option<OutcomeMappingSetConfig>,
    strategy: Option<meld_world_model::AgentStrategyRuntimeConfig>,
    capability_runtime: Option<ProductCapabilityRuntime>,
    authority_policy: Option<AuthorityPolicyBinding>,
}

/// Product-neutral capability runtime shared by Strategy, Task admission, and
/// dispatch for one stewardship composition.
#[derive(Clone)]
pub struct ProductCapabilityRuntime {
    /// Exact contracts visible to Task admission and lowering.
    pub catalog: CapabilityCatalog,
    /// Matching executable invokers visible to dispatch.
    pub registry: crate::capability::CapabilityExecutorRegistry,
}

/// Execution route ports injected for the dispatch actor.
#[derive(Clone)]
pub struct DispatchRouteBindings {
    /// Executes claimed task invocations over the real route.
    pub claim_invoker: SharedClaimedTaskInvoker,
}

impl DispatchRouteBindings {
    /// Compose the production execution routes over the real machinery.
    ///
    /// Admitted compiled Tasks execute through the product Capability set and
    /// provider registry carried by the root api facade.
    pub fn production(context: crate::runtime::ports::ProductionDispatchRouteContext) -> Self {
        Self {
            claim_invoker: context.into_claim_port(),
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
/// - The first production binding wins and later bind calls are no-ops.
/// - An unbound slot leaves the dispatch handle body-less, so a boot that
///   never binds routes still projects a truthful
///   `UnresolvedRequiredBinding` — the slot never manufactures behavior.
#[derive(Clone, Default)]
pub struct DispatchRouteSlot {
    routes: Arc<Mutex<Option<DispatchRouteBindings>>>,
}

impl DispatchRouteSlot {
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

/// Identity a route composer needs to bind production claim dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchRouteSeed {
    /// Event ledger session partition shared with the composed actors.
    pub session_id: String,
}

/// One stewardship composition input for product assembly.
pub struct StewardshipComposition {
    /// Validated physical binding resolved from configuration (stage 0).
    pub binding: PhysicalBinding,
}

fn hydrate_stewardship_theory(
    stores: &OpenProductStores,
    binding: &PhysicalBinding,
    diagnostics: &mut Vec<AssemblyDiagnostic>,
) -> HydratedStewardshipTheory {
    let mut theory = HydratedStewardshipTheory::default();
    let subject = match stewardship_subject_ref(binding) {
        Ok(subject) => subject,
        Err(error) => {
            diagnostics.push(AssemblyDiagnostic {
                code: "theory_image_inconsistent".to_string(),
                message: error.to_string(),
            });
            return theory;
        }
    };
    let resolved = match ResolvedStewardshipTheory::resolve_prepared_product(
        stores,
        &binding.package,
        &subject,
    ) {
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
            return theory;
        }
    };
    if let Err(error) = resolved.validate_activation(&binding.package, &subject) {
        diagnostics.push(AssemblyDiagnostic {
            code: "theory_image_inconsistent".to_string(),
            message: error.to_string(),
        });
        return theory;
    }
    let contracts = resolved
        .executable_contracts
        .iter()
        .map(|revision| revision.contract.clone())
        .collect::<Vec<_>>();
    let prepared_closure = resolved
        .prepared_closure
        .as_ref()
        .expect("prepared-product resolution must retain its closure");
    let capability_runtime = match activate_exact_capabilities(
        stores,
        binding,
        &contracts,
        prepared_closure,
        resolved.claim_policy.as_ref(),
    ) {
        Ok(runtime) => runtime,
        Err(error) => {
            diagnostics.push(AssemblyDiagnostic {
                code: "theory_image_inconsistent".to_string(),
                message: error.to_string(),
            });
            return theory;
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
            return theory;
        }
    };
    let authority_policy = match resolved.authority_policy.binding() {
        Ok(policy) => policy,
        Err(error) => {
            diagnostics.push(AssemblyDiagnostic {
                code: "theory_image_inconsistent".to_string(),
                message: error.to_string(),
            });
            return theory;
        }
    };
    strategy.theory_revision = Some(resolved.strategy_theory.revision_ref());
    strategy = strategy.with_authority_policy(authority_policy.clone());
    theory.outcome_mapping = Some(resolved.outcome_mapping.config.clone());
    theory.strategy = Some(strategy);
    theory.capability_runtime = Some(capability_runtime);
    theory.authority_policy = Some(authority_policy);
    theory.resolved = Some(resolved);
    theory
}

/// Bind built-in product capability implementations by exact contract.
///
/// This adapter composes implementation publishers. It never selects by
/// stewardship expression and it rejects any installed contract for which
/// the process has no exact invoker.
fn activate_exact_capabilities(
    stores: &OpenProductStores,
    binding: &PhysicalBinding,
    contracts: &[crate::capability::CapabilityTypeContract],
    closure: &crate::theory::PreparedActivationClosureV1,
    claim_policy: Option<&crate::docs::claim_validation::DocsClaimPolicyRevision>,
) -> Result<ProductCapabilityRuntime, crate::error::ApiError> {
    if closure.assignment.principal_id != binding.package.principal_id
        || closure.assignment.subject != binding.subject
    {
        return Err(crate::error::ApiError::ConfigError(
            "prepared product assignment differs from the physical stewardship binding".to_string(),
        ));
    }
    let topology_receipt = stores
        .pds_products
        .topology_receipt(&closure.agent_topology_receipt_id)
        .map_err(|error| crate::error::ApiError::ConfigError(error.to_string()))?
        .ok_or_else(|| {
            crate::error::ApiError::ConfigError(
                "prepared product closure cites a missing Agent topology receipt".to_string(),
            )
        })?;
    if topology_receipt.assignment_id != closure.assignment.assignment_id {
        return Err(crate::error::ApiError::ConfigError(
            "prepared Agent topology belongs to another assignment".to_string(),
        ));
    }
    let capability_receipt = stores
        .pds_products
        .capability_preparation(&closure.capability_preparation_receipt_id)
        .map_err(|error| crate::error::ApiError::ConfigError(error.to_string()))?
        .ok_or_else(|| {
            crate::error::ApiError::ConfigError(
                "prepared product closure cites a missing Capability preparation receipt"
                    .to_string(),
            )
        })?;
    let inventory: ProductCapabilityInventory =
        crate::capability::product_capability_inventory()
            .map_err(|error| crate::error::ApiError::ConfigError(error.to_string()))?;
    let selected_contracts = closure
        .activation
        .selected_implementations
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let mut owner_bindings = OwnerBindingView::new(binding.owner_binding_values());
    if let Some(policy) = claim_policy {
        owner_bindings = crate::docs::contribution::bind_claim_policy(owner_bindings, policy)?;
    }
    let prepared = inventory
        .prepare(
            ExactCapabilityActivationRequest {
                assignment_id: closure.assignment.assignment_id.clone(),
                activation_id: closure.activation.activation_id.clone(),
                selected_contracts,
                selected_implementations: closure.activation.selected_implementations.clone(),
                compatibility_policy_revision: "capability-compatibility.v1".to_string(),
            },
            &owner_bindings,
        )
        .map_err(|error| crate::error::ApiError::ConfigError(error.to_string()))?;
    if prepared.preparation_receipt != capability_receipt {
        return Err(crate::error::ApiError::ConfigError(
            "current physical bindings differ from the prepared capability closure".to_string(),
        ));
    }
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

/// Stewardship values shared by the composed actor factories.
struct ComposedStewardship {
    provider_id: Option<String>,
    workspace_root: Option<PathBuf>,
    lifecycle: Option<ActivationLifecycleStore>,
    bindings: StewardshipActorBindings,
    theory: HydratedStewardshipTheory,
    network: Option<Arc<Mutex<SledTaskNetworkStore>>>,
    cursor_registry: EventConsumerRegistryCapability,
    worker_id: String,
    dispatch_slot: DispatchRouteSlot,
    dispatch_route_seed: DispatchRouteSeed,
}

impl ComposedStewardship {
    fn admission_observer(
        &self,
        store: Arc<AgentStore>,
    ) -> Result<ProductAdmissionGenerationObserver, RuntimeAssemblyError> {
        let prepared = self
            .theory
            .resolved
            .as_ref()
            .and_then(|resolved| resolved.prepared_closure.as_ref())
            .ok_or_else(|| {
                RuntimeAssemblyError::RuntimeHandleConstruction(
                    "native admission requires a prepared assignment".into(),
                )
            })?;
        let lifecycle = self.lifecycle.clone().ok_or_else(|| {
            RuntimeAssemblyError::RuntimeHandleConstruction(
                "native admission requires the lifecycle authority".into(),
            )
        })?;
        Ok(ProductAdmissionGenerationObserver::new(
            store, lifecycle, prepared,
        ))
    }
}

/// Derive the production route seed from one validated physical binding.
fn dispatch_route_seed(
    _binding: &PhysicalBinding,
    bindings: &StewardshipActorBindings,
) -> DispatchRouteSeed {
    DispatchRouteSeed {
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
    lifecycle_binding: Option<ParticipantLifecycleContextV1>,
    last_report: Option<WorkerTickReport>,
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
    authority_port: Arc<dyn meld_world_model::curation::CurationAuthorityPort>,
    runtime_id: String,
    session_id: String,
    authority: CurationAuthority,
    rule: meld_world_model::curation::CurationRuleSource,
    store: Arc<CurationStore>,
    traversal: ProductCurationTraversalPort,
    events: ProductEventAppendPort,
}

#[derive(Clone)]
struct AgentActorFactory {
    runtime_id: String,
    intent: AgentReconciliationIntent,
    store: Arc<AgentStore>,
    planner: Arc<dyn meld_world_model::AgentPlannerPort>,
    authority_port: Arc<dyn meld_world_model::AgentAuthorityPort>,
    frozen_authority: AgentAuthorizationFence,
    curation: Arc<dyn meld_world_model::AgentCurationPort>,
    execution: Arc<dyn meld_world_model::AgentExecutionPort>,
    strategy: meld_world_model::AgentStrategyRuntimeConfig,
    authority: CurationAuthority,
    preparation: meld_world_model::agent::AgentPreparation,
    #[cfg(test)]
    planner_source_positions: Vec<PlannerSourcePosition>,
}

#[derive(Clone)]
struct TaskAdmissionFactory {
    catalog: CapabilityCatalog,
    network: Arc<Mutex<SledTaskNetworkStore>>,
    bindings: StewardshipActorBindings,
}

#[derive(Clone)]
struct DispatchFactory {
    /// Late-binding route slot: an unbound slot builds a body-less handle.
    routes: DispatchRouteSlot,
    execution_db: sled::Db,
    network: Arc<Mutex<SledTaskNetworkStore>>,
    worker_id: String,
    authority_policy: Option<AuthorityPolicyBinding>,
    admission_generation_observer:
        Option<Arc<dyn meld_execution::task_network::dispatch_actor::AdmissionGenerationObserver>>,
}

#[derive(Clone)]
struct PublicationFactory {
    event_append: ProductEventAppendPort,
    bindings: StewardshipActorBindings,
    worker_id: String,
    network: Option<Arc<Mutex<SledTaskNetworkStore>>>,
}

#[derive(Clone)]
enum RuntimeSemanticHandleFactory {
    DocsObservation(Box<crate::docs::runtime::DocsObservationBinding>),
    None,
    GraphReplay { graph_runtime: Arc<GraphRuntime> },
    EventAppend { port: ProductEventAppendPort },
    BeliefAssessment(Box<BeliefAssessmentFactory>),
    EvidenceIngestion(Box<EvidenceIngestionFactory>),
    StandingCuration(Box<StandingCurationFactory>),
    AgentActor(Box<AgentActorFactory>),
    TaskAdmission(Box<TaskAdmissionFactory>),
    Dispatch(Box<DispatchFactory>),
    AggregatePublication(Box<PublicationFactory>),
}

enum RuntimeSemanticHandle {
    DocsObservation(Box<crate::docs::runtime::DocsObservationActor>),
    None,
    GraphReplay(GraphReplayRuntimeHandle),
    EventAppend(EventAppendRuntimeHandle),
    BeliefAssessment(Box<BeliefAssessmentHandle>),
    EvidenceIngestion(Box<EvidenceIngestionHandle>),
    StandingCuration(Box<StandingCurationHandle>),
    AgentActor(Box<AgentActorHandle>),
    TaskAdmission(Box<TaskAdmissionHandle>),
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
    lifecycle: NativeOwnerLifecycleState,
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

struct TaskAdmissionHandle {
    actor: TaskAdmissionRuntimeActor<TaskCompiler>,
    network: Arc<Mutex<SledTaskNetworkStore>>,
    network_id: String,
}

struct DispatchHandle {
    actor: Option<DispatchRuntimeActor<SharedClaimedTaskInvoker>>,
    construction_error: Option<String>,
    tokio_runtime: Option<tokio::runtime::Runtime>,
    network: Arc<Mutex<SledTaskNetworkStore>>,
    sequence: u64,
}

struct PublicationHandle {
    runtime: PublicationRuntime,
    event_append: ProductEventAppendPort,
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
    /// Native-owner readiness when an exact lifecycle context was supplied.
    pub owner_readiness: Option<OwnerReadinessReceiptV1>,
}

/// Report returned when an inert handle accepts a stop request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandleStopReport {
    /// Runtime id that accepted the stop.
    pub runtime_id: String,
    /// Whether the handle was running before the stop request.
    pub was_started: bool,
    /// Native owner stop evidence when shutdown belongs to an activation generation.
    pub owner_stop: Option<OwnerStopReceiptV1>,
}

/// Report returned when an inert handle reaches a safe point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandleSafePointReport {
    /// Runtime id that reached the safe point.
    pub runtime_id: String,
    /// Whether the handle is safe for product flush.
    pub safe_for_flush: bool,
    /// Native-owner safe-point evidence for the exact stopped incarnation.
    pub owner_safe_point: Option<OwnerSafePointReceiptV1>,
}

#[derive(Debug, Clone)]
pub(crate) struct NativeOwnerLifecycleSnapshot {
    checkpoint_ref: String,
    installed_revision_refs: Vec<String>,
    binding_refs: Vec<String>,
    subscription_refs: Vec<String>,
    proof_position_ref: String,
    unresolved_operation_summary_ref: String,
}

impl From<meld_world_model::lifecycle::NativeLifecycleEvidence> for NativeOwnerLifecycleSnapshot {
    fn from(evidence: meld_world_model::lifecycle::NativeLifecycleEvidence) -> Self {
        Self {
            checkpoint_ref: evidence.checkpoint_ref,
            installed_revision_refs: evidence.installed_revision_refs,
            binding_refs: evidence.binding_refs,
            subscription_refs: evidence.subscription_refs,
            proof_position_ref: evidence.proof_position_ref,
            unresolved_operation_summary_ref: evidence.unresolved_operation_summary_ref,
        }
    }
}
impl From<meld_execution::lifecycle::NativeLifecycleEvidence> for NativeOwnerLifecycleSnapshot {
    fn from(evidence: meld_execution::lifecycle::NativeLifecycleEvidence) -> Self {
        Self {
            checkpoint_ref: evidence.checkpoint_ref,
            installed_revision_refs: evidence.installed_revision_refs,
            binding_refs: evidence.binding_refs,
            subscription_refs: evidence.subscription_refs,
            proof_position_ref: evidence.proof_position_ref,
            unresolved_operation_summary_ref: evidence.unresolved_operation_summary_ref,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeOwnerLifecyclePhase {
    Constructed,
    Running,
    SafePoint,
    Stopped,
    Released,
}

#[derive(Debug, Clone)]
struct NativeOwnerLifecycleState {
    phase: NativeOwnerLifecyclePhase,
    incarnation_id: Option<String>,
    last_transition_proof_ref: Option<String>,
}

impl Default for NativeOwnerLifecycleState {
    fn default() -> Self {
        Self {
            phase: NativeOwnerLifecyclePhase::Constructed,
            incarnation_id: None,
            last_transition_proof_ref: None,
        }
    }
}

impl NativeOwnerLifecycleState {
    fn transition(
        &mut self,
        context: &ParticipantLifecycleContextV1,
        checkpoint_ref: &str,
        expected: NativeOwnerLifecyclePhase,
        next: NativeOwnerLifecyclePhase,
    ) -> Result<String, RuntimeAssemblyError> {
        if self.phase == next
            && self.incarnation_id.as_deref() == Some(context.incarnation_id.as_str())
        {
            return self.last_transition_proof_ref.clone().ok_or_else(|| {
                RuntimeAssemblyError::SupervisorHandoff(
                    "native owner repeated a transition without retained proof".to_string(),
                )
            });
        }
        if self.phase != expected {
            return Err(RuntimeAssemblyError::SupervisorHandoff(format!(
                "native owner '{}' cannot transition from {:?} to {:?}",
                context.participant_id, self.phase, next
            )));
        }
        if self
            .incarnation_id
            .as_ref()
            .is_some_and(|incarnation_id| incarnation_id != &context.incarnation_id)
        {
            return Err(RuntimeAssemblyError::SupervisorHandoff(
                "native owner transition changed activation incarnation".to_string(),
            ));
        }
        let proof_ref = format!(
            "native-owner-transition::{}::{}::{}::{:?}-to-{:?}::{checkpoint_ref}",
            context.owner_domain, context.participant_id, context.incarnation_id, self.phase, next
        );
        self.phase = next;
        self.incarnation_id = Some(context.incarnation_id.clone());
        self.last_transition_proof_ref = Some(proof_ref.clone());
        Ok(proof_ref)
    }
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
    /// Exact registration projection composed from the prepared plan.
    pub registration_set: Option<&'a RegistrationSet>,
    /// Canonical activation lifecycle authority when a prepared product exists.
    pub lifecycle_store: Option<&'a ActivationLifecycleStore>,
    /// Exact inert closure whose participant plan composes this supervisor.
    pub prepared_activation: Option<&'a crate::theory::PreparedActivationClosureV1>,
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
    /// Describe a configured product or workspace without opening stores.
    pub fn describe_for_workspace(
        workspace_root: &Path,
        config: &MerkleConfig,
    ) -> Result<ProductRuntimeDescription, RuntimeAssemblyError> {
        let selected = PhysicalBinding::resolve_for_target(config, workspace_root)
            .map_err(|error| RuntimeAssemblyError::Config(error.to_string()))?;
        let product_root = match selected {
            Some(binding) => binding.storage_root,
            None => config
                .system
                .storage
                .resolve_product_root(workspace_root)
                .map_err(|error| RuntimeAssemblyError::Config(error.to_string()))?,
        };
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

        let has_stewardship = stewardship.is_some();
        let product_root = ProductStorageRoot::new(config.product_root);
        let layout = product_root.layout();
        let registry = RuntimeFactoryRegistry::first_proof_registry()?;
        validate_runtime_selection(&config.enabled_runtime_ids)?;
        validate_runtime_selection(&config.disabled_runtime_ids)?;

        let explicit_registration_set = config.registration_set.clone();
        let mut scope = explicit_registration_set
            .as_ref()
            .map(|set| scope_for_registration_set(set, &registry))
            .unwrap_or_else(StoreScope::all);
        if has_stewardship {
            scope.theory = true;
        }

        let stores = Arc::new(OpenProductStores::open_scoped(&layout, &scope)?);
        let supervisor_store_path = config
            .supervisor_store_path
            .unwrap_or_else(|| layout.root.join("supervisor.sled"));
        let fallback_desired_runtime_state = desired_runtime_state(
            &registry,
            config.enabled_runtime_ids.clone(),
            config.disabled_runtime_ids.clone(),
        )?;
        let mut provider = config.provider;
        provider.provider_required = provider_required(&registry, &fallback_desired_runtime_state);
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
                let theory = hydrate_stewardship_theory(
                    stores.as_ref(),
                    &composition.binding,
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
                let lifecycle = theory
                    .resolved
                    .as_ref()
                    .and_then(|resolved| resolved.prepared_closure.as_ref())
                    .map(|_| {
                        let db = stores.theory_db.opened().ok_or_else(|| {
                            RuntimeAssemblyError::RuntimeHandleConstruction(
                                "prepared activation requires the theory store".into(),
                            )
                        })?;
                        ActivationLifecycleStore::new(db.clone()).map_err(|error| {
                            RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                        })
                    })
                    .transpose()?;
                Some(ComposedStewardship {
                    provider_id: composition.binding.provider_id.clone(),
                    workspace_root: composition.binding.workspace_root.clone(),
                    lifecycle,
                    dispatch_slot: DispatchRouteSlot::default(),
                    network,
                    cursor_registry: event_authority.consumer_registry_capability(),
                    // Worker identity derives from the durable ledger
                    // identity, never process-random state, so interrupted
                    // claims are resumable across supervisor restarts.
                    worker_id: format!("runtime-worker::{}", event_authority.ledger_identity()),
                    theory,
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
        let prepared_activation = composed_stewardship
            .as_ref()
            .and_then(|composed| composed.theory.resolved.as_ref())
            .and_then(|resolved| resolved.prepared_closure.clone());
        let registration_set = match (
            &explicit_registration_set,
            &prepared_activation,
            has_stewardship,
        ) {
            (Some(explicit), Some(prepared), _) => {
                let projected = project_prepared_registrations(prepared, &registry)?;
                if explicit != &projected {
                    return Err(RuntimeAssemblyError::Config(
                        "explicit runtime registrations differ from the prepared participant plan"
                            .to_string(),
                    ));
                }
                Some(explicit.clone())
            }
            (Some(explicit), None, _) => Some(explicit.clone()),
            (None, Some(prepared), _) => Some(project_prepared_registrations(prepared, &registry)?),
            (None, None, true) => Some(RegistrationSet {
                registrations: Vec::new(),
            }),
            (None, None, false) => None,
        };
        let desired_runtime_state = match (&registration_set, &prepared_activation) {
            (Some(registrations), Some(_)) => desired_runtime_state_for_registration_set(
                &registry,
                registrations,
                &BTreeSet::new(),
            ),
            (Some(registrations), None) => {
                let disabled = registrations
                    .registrations
                    .iter()
                    .filter(|registration| {
                        !fallback_desired_runtime_state.iter().any(|state| {
                            state.runtime_id == registration.runtime_id && state.enabled
                        })
                    })
                    .map(|registration| registration.runtime_id.clone())
                    .collect();
                desired_runtime_state_for_registration_set(&registry, registrations, &disabled)
            }
            (None, None) => fallback_desired_runtime_state,
            (None, Some(_)) => unreachable!("prepared activation always projects registrations"),
        };
        let lifecycle_store = composed_stewardship
            .as_ref()
            .and_then(|composed| composed.lifecycle.clone());
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
            lifecycle_store,
            prepared_activation,
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

    /// Return the canonical activation lifecycle authority when composed.
    pub fn lifecycle_store(&self) -> Option<&ActivationLifecycleStore> {
        self.lifecycle_store.as_ref()
    }

    /// Return the exact prepared closure composing this runtime.
    pub fn prepared_activation(&self) -> Option<&crate::theory::PreparedActivationClosureV1> {
        self.prepared_activation.as_ref()
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

    pub fn bind_docs_claim_judge(
        &self,
        judge: Arc<dyn crate::docs::claim_validation::DocsClaimJudge>,
    ) -> bool {
        match self
            .handle_factories
            .get("docs.observation")
            .map(|factory| &factory.semantic)
        {
            Some(RuntimeSemanticHandleFactory::DocsObservation(binding)) => {
                binding.claim_judge.bind(judge)
            }
            _ => false,
        }
    }

    pub fn bind_production_docs_claim_judge(&self, api: Arc<crate::api::ContextApi>) -> bool {
        let Some(RuntimeSemanticHandleFactory::DocsObservation(binding)) = self
            .handle_factories
            .get("docs.observation")
            .map(|factory| &factory.semantic)
        else {
            return false;
        };
        let Some(config) = binding.claim_config.clone() else {
            return false;
        };
        self.bind_docs_claim_judge(Arc::new(crate::runtime::ports::ProductionDocsClaimJudge {
            api,
            config,
        }))
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
            registration_set: self.registration_set(),
            lifecycle_store: self.lifecycle_store(),
            prepared_activation: self.prepared_activation(),
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
            RuntimeFactoryDescriptor::new("workspace.source", vec![EventAppend, Workspace])?,
            RuntimeFactoryDescriptor::new(
                "docs.observation",
                vec![EventAppend, Workspace, Theory, WorldModel],
            )?,
            RuntimeFactoryDescriptor::new(
                "world_model.graph_replay",
                vec![EventAppend, EventReplay, EventConsumerRegistry, WorldModel],
            )?,
            RuntimeFactoryDescriptor::new(
                "world_model.standing_curation",
                vec![EventAppend, WorldModel],
            )?,
            RuntimeFactoryDescriptor::new("world_model.belief_assessment", vec![WorldModel])?,
            RuntimeFactoryDescriptor::new(AGENT_RECONCILIATION_RUNTIME_ID, vec![WorldModel])?,
            RuntimeFactoryDescriptor::new(
                "world_model.evidence_ingestion",
                vec![EventReplay, EventConsumerRegistry, WorldModel],
            )?,
            RuntimeFactoryDescriptor::new("execution.task_admission", vec![TaskNetworkFactory])?,
            RuntimeFactoryDescriptor::new(
                "execution.task_network_command",
                vec![TaskNetworkFactory],
            )?,
            RuntimeFactoryDescriptor::new(
                "execution.task_dispatch",
                vec![TaskNetworkFactory, TaskArtifactFactory],
            )?,
            RuntimeFactoryDescriptor::new(
                "execution.publication",
                vec![TaskNetworkFactory, EventAppend],
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
            lifecycle_binding: None,
            last_report: None,
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
        let report = self.semantic.tick(budget);
        self.last_report.clone_from(&report);
        report
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
        RuntimeHandleStopReport {
            runtime_id: self.runtime_id.clone(),
            was_started,
            owner_stop: None,
        }
    }

    /// Invoke the native owner stop hook for one exact activation incarnation.
    pub fn request_lifecycle_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<RuntimeHandleStopReport, RuntimeAssemblyError> {
        self.validate_lifecycle_context(context)?;
        let was_started = self.started;
        let owner_stop = self.semantic.request_stop(context)?;
        self.started = false;
        Ok(RuntimeHandleStopReport {
            runtime_id: self.runtime_id.clone(),
            was_started,
            owner_stop: Some(owner_stop),
        })
    }

    /// Wait for an inert handle safe point.
    pub fn wait_for_safe_point(&self) -> RuntimeHandleSafePointReport {
        RuntimeHandleSafePointReport {
            runtime_id: self.runtime_id.clone(),
            safe_for_flush: !self.started,
            owner_safe_point: None,
        }
    }

    /// Ask the native owner to prove its safe point before final stop.
    pub fn wait_for_lifecycle_safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<RuntimeHandleSafePointReport, RuntimeAssemblyError> {
        self.validate_lifecycle_context(context)?;
        let receipt = self.semantic.safe_point(context)?;
        Ok(RuntimeHandleSafePointReport {
            runtime_id: self.runtime_id.clone(),
            safe_for_flush: true,
            owner_safe_point: Some(receipt),
        })
    }

    /// Ask the native owner to acknowledge release of its exact lease.
    pub fn release_lifecycle(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
        self.validate_lifecycle_context(context)?;
        self.semantic.release(context)
    }

    /// Ask the native owner for its typed no-work account.
    pub fn lifecycle_wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
        self.validate_lifecycle_context(context)?;
        self.semantic.wait(context, report)
    }

    /// Return whether this native owner resolves one structural wake address.
    pub fn resolves_lifecycle_wake(
        &self,
        generation_id: &str,
        incarnation_id: &str,
        wake_ref: &StructuralWakeRef,
    ) -> Result<bool, RuntimeAssemblyError> {
        if !self.started
            || !self.lifecycle_binding.as_ref().is_some_and(|binding| {
                binding.generation_id == generation_id && binding.incarnation_id == incarnation_id
            })
        {
            return Ok(false);
        }
        self.semantic.resolves_wake(wake_ref)
    }

    /// Start only after the supervisor supplies a matching non-empty lease.
    pub fn start_after_lease(
        &mut self,
        lease: RuntimeLeaseContext,
    ) -> Result<RuntimeHandleStartReport, RuntimeAssemblyError> {
        self.start_after_lease_inner(lease, None)
    }

    /// Start and return native readiness for one exact activation incarnation.
    pub fn start_after_lifecycle_lease(
        &mut self,
        lease: RuntimeLeaseContext,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<RuntimeHandleStartReport, RuntimeAssemblyError> {
        self.start_after_lease_inner(lease, Some(context))
    }

    fn start_after_lease_inner(
        &mut self,
        lease: RuntimeLeaseContext,
        context: Option<&ParticipantLifecycleContextV1>,
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
        if !self.has_semantic_body() {
            return Err(RuntimeAssemblyError::SupervisorHandoff(format!(
                "runtime '{}' has no owner readiness evidence",
                self.runtime_id
            )));
        }
        let owner_readiness = match context {
            Some(context) => {
                self.validate_lifecycle_context(context)?;
                if context.lease_ref != lease.lease_id {
                    return Err(RuntimeAssemblyError::SupervisorHandoff(
                        "lifecycle incarnation does not own the supplied lease".to_string(),
                    ));
                }
                Some(self.semantic.readiness(context)?)
            }
            None => None,
        };
        self.started = true;
        self.lifecycle_binding = context.cloned();
        Ok(RuntimeHandleStartReport {
            runtime_id: self.runtime_id.clone(),
            owner_readiness,
        })
    }

    fn validate_lifecycle_context(
        &self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<(), RuntimeAssemblyError> {
        if context.participant_id != self.runtime_id {
            return Err(RuntimeAssemblyError::SupervisorHandoff(format!(
                "lifecycle participant '{}' does not match handle '{}'",
                context.participant_id, self.runtime_id
            )));
        }
        Ok(())
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
            "docs.observation" => {
                let Some(composed) = stewardship else {
                    return Ok(Self::None);
                };
                let Some(root) = &composed.workspace_root else {
                    return Ok(Self::None);
                };
                let Some(resolved) = composed.theory.resolved.as_ref() else {
                    return Ok(Self::None);
                };
                let Some(store) = stores.docs_observations.opened() else {
                    return Ok(Self::None);
                };
                let Some(agent) = stores
                    .agent_store
                    .get_agent(&composed.bindings.agent_id)
                    .map_err(|e| RuntimeAssemblyError::RuntimeHandleConstruction(e.to_string()))?
                else {
                    return Ok(Self::None);
                };
                let Some(crate::runtime::theory::PreparedCurationSelection::Installed(rule)) =
                    resolved
                        .native_curation_selection(stores, &agent)
                        .map_err(|e| {
                            RuntimeAssemblyError::RuntimeHandleConstruction(e.to_string())
                        })?
                else {
                    return Ok(Self::None);
                };
                let Some(route) = stores
                    .traversal_store
                    .owner_event_route("docs", crate::docs::publication::OBSERVATION_EVENT)
                    .map_err(|e| RuntimeAssemblyError::RuntimeHandleConstruction(e.to_string()))?
                else {
                    return Ok(Self::None);
                };
                Ok(Self::DocsObservation(Box::new(
                    crate::docs::runtime::DocsObservationBinding {
                        root: root.clone(),
                        claim_policy: resolved.claim_policy.clone(),
                        claim_config: composed
                            .provider_id
                            .as_ref()
                            .map(|provider| {
                                Ok::<_, RuntimeAssemblyError>(
                                    crate::docs::capability::DocsCapabilityConfig {
                                        target_root: root.clone(),
                                        subject_id: composed.bindings.subject.object_id.clone(),
                                        agent_id: composed.bindings.agent_id.clone(),
                                        provider: crate::execution::ProviderExecutionBinding::new(
                                            provider,
                                            Default::default(),
                                        )
                                        .map_err(|error| {
                                            RuntimeAssemblyError::Config(error.to_string())
                                        })?,
                                    },
                                )
                            })
                            .transpose()?,
                        claim_judge: Default::default(),
                        subject: composed.bindings.subject.clone(),
                        scope: rule.rule.scope.clone(),
                        session_id: composed.bindings.session_id.clone(),
                        store: Arc::clone(store),
                        events: ports.event_append().append_capability(),
                        route,
                    },
                )))
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
                let rule = match composed
                    .theory
                    .resolved
                    .as_ref()
                    .ok_or_else(|| {
                        RuntimeAssemblyError::RuntimeHandleConstruction(
                            "Curation requires prepared product theory".into(),
                        )
                    })?
                    .native_curation_selection(stores, &agent)
                {
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
                let Some(prepared) = composed
                    .theory
                    .resolved
                    .as_ref()
                    .and_then(|resolved| resolved.prepared_closure.as_ref())
                else {
                    return unresolved(
                        diagnostics,
                        "standing_curation_preparation_unresolved",
                        "standing Curation requires exact prepared genesis".into(),
                    );
                };
                let authority = CurationAuthority {
                    agent_id,
                    perspective: agent.perspective_key,
                    branch_scope: agent.branch_scope,
                    activation_generation: prepared.activation.activation_id.clone(),
                    admission_epoch: None,
                    subject: agent.subject,
                };
                authority.validate().map_err(|error| {
                    RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                })?;
                let rule = match rule {
                    crate::runtime::theory::PreparedCurationSelection::Installed(rule) => {
                        (*rule).into()
                    }
                    crate::runtime::theory::PreparedCurationSelection::Epoch(_) => {
                        meld_world_model::curation::CurationRuleSource::Producer(Arc::new(
                            meld_world_model::agent::AgentEpochCurationSource::new(
                                Arc::clone(agent_store),
                                authority.agent_id.clone(),
                            )
                            .map_err(|error| {
                                RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                            })?,
                        ))
                    }
                };
                Ok(Self::StandingCuration(Box::new(StandingCurationFactory {
                    authority_port: Arc::new(ProductCurationAuthorityPort {
                        admission: composed.admission_observer(Arc::clone(agent_store))?,
                        authority: authority.clone(),
                    }),
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
                ) = (
                    stores.belief_store.opened(),
                    stores.traversal_store.opened(),
                    stores.agent_store.opened(),
                    stores.curation_store.opened(),
                    stores.belief_family_registry.opened(),
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
                        "Agent reconciliation requires one prepared product compilation"
                            .to_string(),
                    );
                };
                let product_compilation_receipt_id = resolved
                    .product_compilation_receipt_id
                    .as_deref()
                    .ok_or_else(|| {
                        RuntimeAssemblyError::RuntimeHandleConstruction(
                            "live Agent theory has no prepared product compilation".to_string(),
                        )
                    })?;
                let Some(strategy) = composed.theory.strategy.clone() else {
                    return unresolved(
                        diagnostics,
                        "agent_strategy_unresolved",
                        "Agent reconciliation requires installed Strategy theory".to_string(),
                    );
                };
                let Some(network) = composed.network.as_ref() else {
                    return unresolved(
                        diagnostics,
                        "agent_execution_network_unresolved",
                        "Agent reconciliation requires the shared Execution Task Network"
                            .to_string(),
                    );
                };
                let Some(capability_runtime) = composed.theory.capability_runtime.as_ref() else {
                    return unresolved(
                        diagnostics,
                        "agent_execution_capabilities_unresolved",
                        "Agent reconciliation requires the exact executable Capability set"
                            .to_string(),
                    );
                };
                let expected_activation_id = resolved
                    .prepared_closure
                    .as_ref()
                    .map(|prepared| prepared.activation.activation_id.as_str())
                    .ok_or_else(|| {
                        RuntimeAssemblyError::RuntimeHandleConstruction(
                            "live Agent theory has no prepared activation identity".to_string(),
                        )
                    })?;
                let rule = resolved
                    .native_curation_selection(stores, &agent)
                    .map_err(|error| {
                        RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                    })?
                    .ok_or_else(|| {
                        RuntimeAssemblyError::RuntimeHandleConstruction(
                            "Agent reconciliation Curation rule is not installed".to_string(),
                        )
                    })?;
                let intent = AgentReconciliationIntent::MaintainedCondition(
                    resolved.maintained_condition.binding().map_err(|error| {
                        RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                    })?,
                );
                let condition = &resolved.maintained_condition.condition;
                let goal_id = intent.goal_id(&agent_id, expected_activation_id);
                let authority_scope_id = strategy
                    .authority_policy
                    .as_ref()
                    .map(|binding| binding.policy.policy_id.clone())
                    .unwrap_or_else(|| "world_model.agent_reconciliation".to_string());
                let context = PlannerDecisionContext {
                    observation_subject: None,
                    context_id: format!("agent-context::{goal_id}"),
                    agent_id: agent_id.clone(),
                    goal_id,
                    subject: agent.subject.clone(),
                    scope_id: match &rule {
                        crate::runtime::theory::PreparedCurationSelection::Installed(rule) => {
                            rule.rule.scope.scope_id.clone()
                        }
                        crate::runtime::theory::PreparedCurationSelection::Epoch(_) => {
                            agent.subject.object_id.clone()
                        }
                    },
                    branch_id: agent.branch_scope.branch_id.clone(),
                    perspective_id: agent.perspective_key.perspective_id.clone(),
                    authority_scope_id: authority_scope_id.clone(),
                    activation_generation: expected_activation_id.to_string(),
                    admission_epoch: None,
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
                    .resolve(
                        &resolved.belief_family.family_id,
                        &resolved.belief_family.content_hash,
                    )
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
                let curation_ref = match &rule {
                    crate::runtime::theory::PreparedCurationSelection::Installed(rule) => {
                        rule.revision_ref()
                    }
                    crate::runtime::theory::PreparedCurationSelection::Epoch(template) => {
                        template.revision_ref()
                    }
                };
                let planner_source_positions = vec![
                    source(
                        PlannerSourceKind::Directive,
                        "agent",
                        &agent.agent_id,
                        product_compilation_receipt_id,
                        product_compilation_receipt_id,
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
                        &curation_ref.id,
                        &curation_ref.content_hash,
                        &curation_ref.content_hash,
                    ),
                    source(
                        PlannerSourceKind::StrategyPolicy,
                        "strategy",
                        &strategy.package.evaluation_policy.policy_id,
                        &resolved.strategy_theory.content_hash,
                        &resolved.strategy_theory.content_hash,
                    ),
                ];
                let planner_policy = PlannerAssemblyPolicy {
                    policy_revision_id: format!(
                        "reasoning-policy::{}",
                        product_compilation_receipt_id
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
                };
                let (planner, preparation): (
                    Arc<dyn meld_world_model::AgentPlannerPort>,
                    meld_world_model::agent::AgentPreparation,
                ) = match rule {
                    crate::runtime::theory::PreparedCurationSelection::Installed(rule) => {
                        let planner_request = PlannerCurrentAssemblyRequest {
                            required_derived_evidence: rule.rule.publishes_source_judgments().then(
                                || meld_world_model::planner::PlannerDerivedEvidenceRequirement {
                                    curation_rule: rule.revision_ref(),
                                    belief_family: resolved.belief_family.revision_ref(),
                                    outcome_mappings: vec![resolved.outcome_mapping.revision_ref()],
                                },
                            ),
                            required_graph_evidence: rule.rule.source_readiness_requirements(),
                            context: context.clone(),
                            policy: planner_policy,
                            traversal_cut_request: TraversalCutRequest {
                                owners: {
                                    let mut owners = vec![
                                        TraversalOwnerRequirement {
                                            event_source: None,
                                            owner_id: rule.rule.source_owner_id.clone(),
                                            scope: rule.rule.scope.clone(),
                                            required: true,
                                        },
                                        TraversalOwnerRequirement {
                                            event_source: None,
                                            owner_id: meld_world_model::CURATION_OWNER_ID
                                                .to_string(),
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
                        (
                            Arc::new(ProductAgentPlannerPort::new(
                                Arc::clone(belief),
                                Arc::clone(traversal),
                                Arc::clone(curation_store),
                                ports.event_append().clone(),
                                planner_request,
                            )),
                            meld_world_model::agent::AgentPreparation::InstalledRule {
                                rule: Box::new(*rule),
                                subscriptions: Some(Arc::new(
                                    meld_world_model::belief::BeliefSubscriptionSource::new(
                                        Arc::clone(belief),
                                        Arc::clone(registry)
                                            as Arc<dyn BeliefFamilyRegistry + Send + Sync>,
                                    ),
                                )),
                            },
                        )
                    }
                    crate::runtime::theory::PreparedCurationSelection::Epoch(template) => {
                        if template.template.source_owner_id != crate::nonce::OWNER_ID {
                            return unresolved(
                                diagnostics,
                                "epoch_source_unresolved",
                                format!(
                                    "no epoch preparation adapter for source owner '{}'",
                                    template.template.source_owner_id
                                ),
                            );
                        }
                        let products = crate::runtime::epoch::ProductNonceEpochPreparation::new(
                            Arc::clone(curation_store),
                            Arc::clone(traversal),
                            template.revision_ref(),
                            &strategy.package,
                            ports.event_append().watermark_capability(),
                        )
                        .map_err(|error| {
                            RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
                        })?;
                        let planner = crate::runtime::ports::ProductEpochAgentPlannerPort::new(
                            Arc::clone(belief),
                            Arc::clone(traversal),
                            Arc::clone(curation_store),
                            ports.event_append().clone(),
                            crate::runtime::ports::ProductEpochPlannerBinding {
                                belief_family: resolved.belief_family.revision_ref(),
                                outcome_mappings: vec![resolved.outcome_mapping.revision_ref()],
                                context: context.clone(),
                                policy: planner_policy,
                                belief_key,
                                unanchored_belief: family_revision.config.anchor_requirement
                                    == meld_world_model::belief::AnchorRequirement::Unanchored,
                                source_positions: planner_source_positions.clone(),
                            },
                        );
                        (
                            Arc::new(planner),
                            meld_world_model::agent::AgentPreparation::Epoch {
                                products: Arc::new(products),
                                subscriptions: Arc::new(
                                    meld_world_model::belief::BeliefSubscriptionSource::new(
                                        Arc::clone(belief),
                                        Arc::clone(registry)
                                            as Arc<dyn BeliefFamilyRegistry + Send + Sync>,
                                    ),
                                ),
                            },
                        )
                    }
                };
                let authority = CurationAuthority {
                    agent_id: agent_id.clone(),
                    perspective: agent.perspective_key,
                    branch_scope: agent.branch_scope,
                    activation_generation: expected_activation_id.to_string(),
                    admission_epoch: None,
                    subject: agent.subject,
                };
                let frozen_authority = AgentAuthorizationFence {
                    activation_generation: authority.activation_generation.clone(),
                    admission_epoch: None,
                    authority_policy_content_hash: resolved
                        .receipt
                        .authority_policy
                        .content_hash
                        .clone(),
                };
                let authority_port: Arc<dyn meld_world_model::AgentAuthorityPort> =
                    Arc::new(ProductAgentAuthorityPort::new(
                        composed.admission_observer(Arc::clone(agent_store))?,
                        agent_id,
                        frozen_authority.authority_policy_content_hash.clone(),
                    ));
                Ok(Self::AgentActor(Box::new(AgentActorFactory {
                    runtime_id: runtime_id.to_string(),
                    intent,
                    store: Arc::clone(agent_store),
                    planner,
                    authority_port: Arc::clone(&authority_port),
                    frozen_authority: frozen_authority.clone(),
                    curation: Arc::new(ProductPlannedCurationPort::new(Arc::clone(curation_store))),
                    execution: Arc::new(ProductAgentExecutionPort::new(
                        Arc::clone(network),
                        capability_runtime.catalog.clone(),
                        authority_port,
                        frozen_authority.authority_policy_content_hash.clone(),
                    )),
                    strategy,
                    authority,
                    preparation,
                    #[cfg(test)]
                    planner_source_positions,
                })))
            }
            "execution.task_admission" => {
                let Some(composed) = stewardship else {
                    return Ok(Self::None);
                };
                let Some(capability_runtime) = composed.theory.capability_runtime.as_ref() else {
                    return unresolved(
                        diagnostics,
                        "task_admission_catalog_unresolved",
                        "no exact Capability catalog is installed; Task admission stays unresolved"
                            .to_string(),
                    );
                };
                let Some(network) = composed.network.as_ref() else {
                    return Ok(Self::None);
                };
                Ok(Self::TaskAdmission(Box::new(TaskAdmissionFactory {
                    catalog: capability_runtime.catalog.clone(),
                    network: Arc::clone(network),
                    bindings: composed.bindings.clone(),
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
                let admission_generation_observer = stores.agent_store.opened().filter(|_| composed.lifecycle.is_some()).map(|store| composed.admission_observer(Arc::clone(store)).map(|observer| Arc::new(observer) as Arc<dyn meld_execution::task_network::dispatch_actor::AdmissionGenerationObserver>)).transpose()?;
                Ok(Self::Dispatch(Box::new(DispatchFactory {
                    routes: composed.dispatch_slot.clone(),
                    execution_db: execution_db.clone(),
                    network: Arc::clone(network),
                    worker_id: composed.worker_id.clone(),
                    authority_policy: composed.theory.authority_policy.clone(),
                    admission_generation_observer,
                })))
            }
            "execution.publication" => {
                let Some(composed) = stewardship else {
                    return Ok(Self::None);
                };
                Ok(Self::AggregatePublication(Box::new(PublicationFactory {
                    event_append: ports.event_append().clone(),
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
            Self::AggregatePublication(factory) => factory.network.is_some(),
            _ => true,
        }
    }

    fn build_handle(&self) -> RuntimeSemanticHandle {
        match self {
            Self::DocsObservation(binding) => RuntimeSemanticHandle::DocsObservation(Box::new(
                crate::docs::runtime::DocsObservationActor::new(binding.as_ref().clone()),
            )),
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
                    lifecycle: NativeOwnerLifecycleState::default(),
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
                    .expect("standing Curation factory holds validated authority and rule")
                    .with_authority_port(Arc::clone(&factory.authority_port)),
                }))
            }
            Self::AgentActor(factory) => {
                let actor = AgentReconciliationActor::new(
                    factory.runtime_id.clone(),
                    factory.intent.clone(),
                    Arc::clone(&factory.store),
                    Arc::clone(&factory.planner),
                    Arc::clone(&factory.authority_port),
                    factory.frozen_authority.clone(),
                    Arc::clone(&factory.curation),
                    Arc::clone(&factory.execution),
                    factory.strategy.clone(),
                    factory.authority.clone(),
                    factory.preparation.clone(),
                )
                .expect("Agent reconciliation factory holds validated exact bindings");
                RuntimeSemanticHandle::AgentActor(Box::new(AgentActorHandle {
                    runtime_id: factory.runtime_id.clone(),
                    agent_id: factory.strategy.agent_id.clone(),
                    actor,
                }))
            }
            Self::TaskAdmission(factory) => {
                RuntimeSemanticHandle::TaskAdmission(Box::new(TaskAdmissionHandle {
                    actor: TaskAdmissionRuntimeActor::new(TaskAdmissionLowerer::new(
                        TaskCompiler::new(),
                        factory.catalog.clone(),
                    )),
                    network: Arc::clone(&factory.network),
                    network_id: factory.bindings.network_id.clone(),
                }))
            }
            Self::Dispatch(factory) => {
                // Body-less when routes are unbound: the supervisor projects
                // an unresolved required binding, never a healthy placeholder.
                let Some(routes) = factory.routes.current() else {
                    return RuntimeSemanticHandle::None;
                };
                let actor_result = DispatchRuntimeActor::new(
                    factory.worker_id.clone(),
                    factory.execution_db.clone(),
                    routes.claim_invoker,
                )
                .map(|actor| {
                    let actor = match &factory.authority_policy {
                        Some(policy) => actor.with_authority_policy(policy.clone()),
                        None => actor,
                    };
                    match &factory.admission_generation_observer {
                        Some(observer) => {
                            actor.with_admission_generation_observer(Arc::clone(observer))
                        }
                        None => actor,
                    }
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
                    sequence: 0,
                }))
            }
            Self::AggregatePublication(factory) => {
                RuntimeSemanticHandle::AggregatePublication(Box::new(PublicationHandle {
                    runtime: PublicationRuntime::new(),
                    event_append: factory.event_append.clone(),
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
    fn readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
        match self {
            Self::None => Err(missing_native_lifecycle_owner()),
            Self::DocsObservation(handle) => handle.native_readiness(context),
            Self::GraphReplay(handle) => handle.native_readiness(context),
            Self::EventAppend(handle) => handle.native_readiness(context),
            Self::BeliefAssessment(handle) => handle.native_readiness(context),
            Self::EvidenceIngestion(handle) => handle.native_readiness(context),
            Self::StandingCuration(handle) => handle.native_readiness(context),
            Self::AgentActor(handle) => handle.native_readiness(context),
            Self::TaskAdmission(handle) => handle.native_readiness(context),
            Self::Dispatch(handle) => handle.native_readiness(context),
            Self::AggregatePublication(handle) => handle.native_readiness(context),
        }
    }

    fn tick(&mut self, budget: WorkBudget) -> Option<WorkerTickReport> {
        match self {
            Self::None => None,
            Self::DocsObservation(handle) => Some(handle.tick(budget)),
            Self::GraphReplay(handle) => Some(handle.tick(budget)),
            Self::EventAppend(handle) => Some(handle.tick()),
            Self::BeliefAssessment(handle) => Some(handle.tick(budget)),
            Self::EvidenceIngestion(handle) => Some(handle.tick(budget)),
            Self::StandingCuration(handle) => Some(handle.tick(budget)),
            Self::AgentActor(handle) => Some(handle.tick(budget)),
            Self::TaskAdmission(handle) => Some(handle.tick(budget)),
            Self::Dispatch(handle) => Some(handle.tick(budget)),
            Self::AggregatePublication(handle) => Some(handle.tick(budget)),
        }
    }

    fn request_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
        match self {
            Self::None => Err(missing_native_lifecycle_owner()),
            Self::DocsObservation(handle) => handle.native_stop(context),
            Self::GraphReplay(handle) => handle.native_stop(context),
            Self::EventAppend(handle) => handle.native_stop(context),
            Self::BeliefAssessment(handle) => handle.native_stop(context),
            Self::EvidenceIngestion(handle) => handle.native_stop(context),
            Self::StandingCuration(handle) => handle.native_stop(context),
            Self::AgentActor(handle) => handle.native_stop(context),
            Self::TaskAdmission(handle) => handle.native_stop(context),
            Self::Dispatch(handle) => handle.native_stop(context),
            Self::AggregatePublication(handle) => handle.native_stop(context),
        }
    }

    fn safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError> {
        match self {
            Self::None => Err(missing_native_lifecycle_owner()),
            Self::DocsObservation(handle) => handle.native_safe_point(context),
            Self::GraphReplay(handle) => handle.native_safe_point(context),
            Self::EventAppend(handle) => handle.native_safe_point(context),
            Self::BeliefAssessment(handle) => handle.native_safe_point(context),
            Self::EvidenceIngestion(handle) => handle.native_safe_point(context),
            Self::StandingCuration(handle) => handle.native_safe_point(context),
            Self::AgentActor(handle) => handle.native_safe_point(context),
            Self::TaskAdmission(handle) => handle.native_safe_point(context),
            Self::Dispatch(handle) => handle.native_safe_point(context),
            Self::AggregatePublication(handle) => handle.native_safe_point(context),
        }
    }

    fn release(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
        match self {
            Self::None => Err(missing_native_lifecycle_owner()),
            Self::DocsObservation(handle) => handle.native_release(context),
            Self::GraphReplay(handle) => handle.native_release(context),
            Self::EventAppend(handle) => handle.native_release(context),
            Self::BeliefAssessment(handle) => handle.native_release(context),
            Self::EvidenceIngestion(handle) => handle.native_release(context),
            Self::StandingCuration(handle) => handle.native_release(context),
            Self::AgentActor(handle) => handle.native_release(context),
            Self::TaskAdmission(handle) => handle.native_release(context),
            Self::Dispatch(handle) => handle.native_release(context),
            Self::AggregatePublication(handle) => handle.native_release(context),
        }
    }

    fn wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
        match self {
            Self::None => Err(missing_native_lifecycle_owner()),
            Self::DocsObservation(handle) => handle.native_wait(context, report),
            Self::GraphReplay(handle) => handle.native_wait(context, report),
            Self::EventAppend(handle) => handle.native_wait(context, report),
            Self::BeliefAssessment(handle) => handle.native_wait(context, report),
            Self::EvidenceIngestion(handle) => handle.native_wait(context, report),
            Self::StandingCuration(handle) => handle.native_wait(context, report),
            Self::AgentActor(handle) => handle.native_wait(context, report),
            Self::TaskAdmission(handle) => handle.native_wait(context, report),
            Self::Dispatch(handle) => handle.native_wait(context, report),
            Self::AggregatePublication(handle) => handle.native_wait(context, report),
        }
    }

    fn resolves_wake(&self, wake_ref: &StructuralWakeRef) -> Result<bool, RuntimeAssemblyError> {
        match self {
            Self::None => Ok(false),
            Self::DocsObservation(handle) => handle.native_resolves_wake(wake_ref),
            Self::GraphReplay(handle) => handle.native_resolves_wake(wake_ref),
            Self::EventAppend(handle) => handle.native_resolves_wake(wake_ref),
            Self::BeliefAssessment(handle) => handle.native_resolves_wake(wake_ref),
            Self::EvidenceIngestion(handle) => handle.native_resolves_wake(wake_ref),
            Self::StandingCuration(handle) => handle.native_resolves_wake(wake_ref),
            Self::AgentActor(handle) => handle.native_resolves_wake(wake_ref),
            Self::TaskAdmission(handle) => handle.native_resolves_wake(wake_ref),
            Self::Dispatch(handle) => handle.native_resolves_wake(wake_ref),
            Self::AggregatePublication(handle) => handle.native_resolves_wake(wake_ref),
        }
    }
}

fn missing_native_lifecycle_owner() -> RuntimeAssemblyError {
    RuntimeAssemblyError::SupervisorHandoff("native lifecycle owner is absent".to_string())
}

pub(crate) trait NativeTransitionProof {
    fn generation_id(&self) -> &str;
    fn incarnation_id(&self) -> &str;
    fn proof_ref(&self) -> &str;
}

impl NativeTransitionProof for meld_world_model::lifecycle::NativeLifecycleTransition {
    fn generation_id(&self) -> &str {
        &self.generation_id
    }

    fn incarnation_id(&self) -> &str {
        &self.incarnation_id
    }

    fn proof_ref(&self) -> &str {
        &self.proof_ref
    }
}

impl NativeTransitionProof for meld_execution::lifecycle::NativeLifecycleTransition {
    fn generation_id(&self) -> &str {
        &self.generation_id
    }

    fn incarnation_id(&self) -> &str {
        &self.incarnation_id
    }

    fn proof_ref(&self) -> &str {
        &self.proof_ref
    }
}

pub(crate) fn verified_native_transition<T: NativeTransitionProof>(
    context: &ParticipantLifecycleContextV1,
    transition: T,
) -> Result<String, RuntimeAssemblyError> {
    if transition.generation_id() != context.generation_id
        || transition.incarnation_id() != context.incarnation_id
        || transition.proof_ref().trim().is_empty()
    {
        return Err(RuntimeAssemblyError::SupervisorHandoff(
            "native lifecycle transition belongs to another activation position".to_string(),
        ));
    }
    Ok(transition.proof_ref().to_string())
}

pub(crate) trait NativeOwnerLifecycle {
    fn native_snapshot(&self) -> Result<NativeOwnerLifecycleSnapshot, RuntimeAssemblyError>;
    fn native_readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError>;
    fn native_wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError>;
    fn native_safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError>;
    fn native_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError>;
    fn native_release(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError>;
    fn native_resolves_wake(
        &self,
        wake_ref: &StructuralWakeRef,
    ) -> Result<bool, RuntimeAssemblyError>;
}

pub(crate) fn owner_readiness_receipt(
    context: &ParticipantLifecycleContextV1,
    snapshot: NativeOwnerLifecycleSnapshot,
    transition_proof_ref: String,
) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
    let evidence = NativeOwnerReadinessEvidenceV1::new(
        context.participant_id.clone(),
        context.owner_domain.clone(),
        snapshot.checkpoint_ref,
        snapshot.installed_revision_refs,
        snapshot.binding_refs,
        snapshot.subscription_refs,
        transition_proof_ref,
    )
    .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))?;
    OwnerReadinessReceiptV1::from_native(context, evidence)
        .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
}

pub(crate) fn owner_wait_receipt(
    context: &ParticipantLifecycleContextV1,
    snapshot: NativeOwnerLifecycleSnapshot,
    report: &WorkerTickReport,
) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
    if report.made_progress()
        || !report.retryable_errors.is_empty()
        || !report.fatal_errors.is_empty()
        || report.budget_exhausted
    {
        return Err(RuntimeAssemblyError::SupervisorHandoff(
            "native owner cannot author a wait for an active or failed step".to_string(),
        ));
    }
    if report.waiting_on.is_empty()
        || report
            .waiting_on
            .iter()
            .any(|declaration| declaration.wake_refs.is_empty())
    {
        return Err(RuntimeAssemblyError::SupervisorHandoff(
            "native owner supplied incomplete wait evidence".to_string(),
        ));
    }
    let mut reasons = report
        .waiting_on
        .iter()
        .map(|wait| wait.condition.as_str())
        .collect::<Vec<_>>();
    reasons.sort_unstable();
    reasons.dedup();
    let reason = reasons.join("+");
    let wake_refs = report
        .waiting_on
        .iter()
        .flat_map(|wait| wait.wake_refs.iter().cloned())
        .collect();
    OwnerWaitReceiptV1::new(
        context.generation_id.clone(),
        context.incarnation_id.clone(),
        snapshot.checkpoint_ref,
        reason,
        wake_refs,
    )
    .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
}

pub(crate) fn owner_safe_point_receipt(
    context: &ParticipantLifecycleContextV1,
    snapshot: NativeOwnerLifecycleSnapshot,
    transition_proof_ref: String,
) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError> {
    OwnerSafePointReceiptV1::new(
        context,
        snapshot.checkpoint_ref,
        snapshot.unresolved_operation_summary_ref,
        vec![snapshot.proof_position_ref, transition_proof_ref],
    )
    .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
}

pub(crate) fn owner_stop_receipt(
    context: &ParticipantLifecycleContextV1,
    snapshot: NativeOwnerLifecycleSnapshot,
    transition_proof_ref: String,
) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
    OwnerStopReceiptV1::new(context, snapshot.checkpoint_ref, transition_proof_ref)
        .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
}

pub(crate) fn owner_release_receipt(
    context: &ParticipantLifecycleContextV1,
    _snapshot: NativeOwnerLifecycleSnapshot,
    transition_proof_ref: String,
) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
    OwnerReleaseReceiptV1::new(context, transition_proof_ref)
        .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
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

impl NativeOwnerLifecycle for GraphReplayRuntimeHandle {
    fn native_snapshot(&self) -> Result<NativeOwnerLifecycleSnapshot, RuntimeAssemblyError> {
        self.graph_runtime
            .lifecycle_evidence()
            .map(Into::into)
            .map_err(RuntimeAssemblyError::SupervisorHandoff)
    }

    fn native_readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .graph_runtime
            .lifecycle_start(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_readiness_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
        let snapshot = self.native_snapshot()?;
        owner_wait_receipt(context, snapshot, report)
    }

    fn native_safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .graph_runtime
            .lifecycle_safe_point(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_safe_point_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .graph_runtime
            .lifecycle_stop(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_stop_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_release(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .graph_runtime
            .lifecycle_release(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_release_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_resolves_wake(
        &self,
        wake_ref: &StructuralWakeRef,
    ) -> Result<bool, RuntimeAssemblyError> {
        match structural_wake_to_world_model(wake_ref) {
            Some(wake) => self
                .graph_runtime
                .resolves_wake(&wake)
                .map_err(RuntimeAssemblyError::SupervisorHandoff),
            None => Ok(false),
        }
    }
}

impl NativeOwnerLifecycle for EventAppendRuntimeHandle {
    fn native_snapshot(&self) -> Result<NativeOwnerLifecycleSnapshot, RuntimeAssemblyError> {
        let health = self
            .port
            .health()
            .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))?;
        let checkpoint_ref = format!(
            "event-authority::{}::{}",
            health.ledger_id, health.committed_watermark
        );
        Ok(NativeOwnerLifecycleSnapshot {
            checkpoint_ref: checkpoint_ref.clone(),
            installed_revision_refs: vec!["event-authority-schema::v1".to_string()],
            binding_refs: vec![format!("event-writer::{}", health.ledger_id)],
            subscription_refs: vec![format!("event-commit-watermark::{}", health.ledger_id)],
            proof_position_ref: checkpoint_ref.clone(),
            unresolved_operation_summary_ref: format!(
                "event-authority::committed-tip::{}",
                health.committed_watermark
            ),
        })
    }

    fn native_readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
        let snapshot = self.native_snapshot()?;
        let proof_ref = self.lifecycle.transition(
            context,
            &snapshot.proof_position_ref,
            NativeOwnerLifecyclePhase::Constructed,
            NativeOwnerLifecyclePhase::Running,
        )?;
        owner_readiness_receipt(context, snapshot, proof_ref)
    }

    fn native_wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
        let snapshot = self.native_snapshot()?;
        if report.made_progress()
            || !report.retryable_errors.is_empty()
            || !report.fatal_errors.is_empty()
            || report.budget_exhausted
        {
            return Err(RuntimeAssemblyError::SupervisorHandoff(
                "legacy Event observer cannot wait after an active or failed step".to_string(),
            ));
        }
        let subscription = snapshot.subscription_refs.first().ok_or_else(|| {
            RuntimeAssemblyError::SupervisorHandoff(
                "legacy Event observer subscription is absent".to_string(),
            )
        })?;
        OwnerWaitReceiptV1::new(
            context.generation_id.clone(),
            context.incarnation_id.clone(),
            snapshot.checkpoint_ref,
            "event-authority-awaiting-commit".to_string(),
            vec![StructuralWakeRef::EventPosition(format!(
                "{subscription}::after::{}",
                report.output_checkpoint.value
            ))],
        )
        .map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))
    }

    fn native_safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError> {
        let snapshot = self.native_snapshot()?;
        let proof_ref = self.lifecycle.transition(
            context,
            &snapshot.proof_position_ref,
            NativeOwnerLifecyclePhase::Running,
            NativeOwnerLifecyclePhase::SafePoint,
        )?;
        owner_safe_point_receipt(context, snapshot, proof_ref)
    }

    fn native_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
        let snapshot = self.native_snapshot()?;
        let proof_ref = self.lifecycle.transition(
            context,
            &snapshot.proof_position_ref,
            NativeOwnerLifecyclePhase::SafePoint,
            NativeOwnerLifecyclePhase::Stopped,
        )?;
        owner_stop_receipt(context, snapshot, proof_ref)
    }

    fn native_release(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
        let snapshot = self.native_snapshot()?;
        let proof_ref = self.lifecycle.transition(
            context,
            &snapshot.proof_position_ref,
            NativeOwnerLifecyclePhase::Stopped,
            NativeOwnerLifecyclePhase::Released,
        )?;
        owner_release_receipt(context, snapshot, proof_ref)
    }

    fn native_resolves_wake(
        &self,
        wake_ref: &StructuralWakeRef,
    ) -> Result<bool, RuntimeAssemblyError> {
        Ok(matches!(
            wake_ref,
            StructuralWakeRef::EventPosition(value)
                if value.strip_prefix(&format!("event-commit-watermark::{}::after::", self.port.watermark().map_err(|error| RuntimeAssemblyError::SupervisorHandoff(error.to_string()))?.ledger_id))
                    .is_some_and(|position| !position.is_empty() && position.bytes().all(|byte| byte.is_ascii_digit()) && position.parse::<u64>().is_ok())
        ))
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

impl NativeOwnerLifecycle for BeliefAssessmentHandle {
    fn native_snapshot(&self) -> Result<NativeOwnerLifecycleSnapshot, RuntimeAssemblyError> {
        self.actor
            .lifecycle_evidence()
            .map(Into::into)
            .map_err(RuntimeAssemblyError::SupervisorHandoff)
    }

    fn native_readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_start(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_readiness_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
        let snapshot = self.native_snapshot()?;
        owner_wait_receipt(context, snapshot, report)
    }

    fn native_safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_safe_point(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_safe_point_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_stop(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_stop_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_release(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_release(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_release_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_resolves_wake(
        &self,
        wake_ref: &StructuralWakeRef,
    ) -> Result<bool, RuntimeAssemblyError> {
        match structural_wake_to_world_model(wake_ref) {
            Some(wake) => self
                .actor
                .resolves_wake(&wake)
                .map_err(RuntimeAssemblyError::SupervisorHandoff),
            None => Ok(false),
        }
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

impl NativeOwnerLifecycle for EvidenceIngestionHandle {
    fn native_snapshot(&self) -> Result<NativeOwnerLifecycleSnapshot, RuntimeAssemblyError> {
        self.actor
            .lifecycle_evidence()
            .map(Into::into)
            .map_err(RuntimeAssemblyError::SupervisorHandoff)
    }

    fn native_readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_start(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_readiness_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
        let snapshot = self.native_snapshot()?;
        owner_wait_receipt(context, snapshot, report)
    }

    fn native_safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_safe_point(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_safe_point_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_stop(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_stop_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_release(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_release(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_release_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_resolves_wake(
        &self,
        wake_ref: &StructuralWakeRef,
    ) -> Result<bool, RuntimeAssemblyError> {
        match structural_wake_to_world_model(wake_ref) {
            Some(wake) => self
                .actor
                .resolves_wake(&wake)
                .map_err(RuntimeAssemblyError::SupervisorHandoff),
            None => Ok(false),
        }
    }
}

impl StandingCurationHandle {
    fn tick(&mut self, budget: WorkBudget) -> WorkerTickReport {
        standing_curation_worker_report(self.actor.bounded_step(budget.max_items))
    }
}

impl NativeOwnerLifecycle for StandingCurationHandle {
    fn native_snapshot(&self) -> Result<NativeOwnerLifecycleSnapshot, RuntimeAssemblyError> {
        self.actor
            .lifecycle_evidence()
            .map(Into::into)
            .map_err(RuntimeAssemblyError::SupervisorHandoff)
    }

    fn native_readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_start(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_readiness_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
        let snapshot = self.native_snapshot()?;
        owner_wait_receipt(context, snapshot, report)
    }

    fn native_safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_safe_point(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_safe_point_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_stop(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_stop_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_release(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_release(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_release_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_resolves_wake(
        &self,
        wake_ref: &StructuralWakeRef,
    ) -> Result<bool, RuntimeAssemblyError> {
        match structural_wake_to_world_model(wake_ref) {
            Some(wake) => self
                .actor
                .resolves_wake(&wake)
                .map_err(RuntimeAssemblyError::SupervisorHandoff),
            None => Ok(false),
        }
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

impl NativeOwnerLifecycle for AgentActorHandle {
    fn native_snapshot(&self) -> Result<NativeOwnerLifecycleSnapshot, RuntimeAssemblyError> {
        self.actor
            .lifecycle_evidence()
            .map(Into::into)
            .map_err(RuntimeAssemblyError::SupervisorHandoff)
    }

    fn native_readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_start(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_readiness_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
        let snapshot = self.native_snapshot()?;
        owner_wait_receipt(context, snapshot, report)
    }

    fn native_safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_safe_point(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_safe_point_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_stop(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_stop_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_release(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
        let (snapshot, transition) = self
            .actor
            .lifecycle_release(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_release_receipt(
            context,
            snapshot.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_resolves_wake(
        &self,
        wake_ref: &StructuralWakeRef,
    ) -> Result<bool, RuntimeAssemblyError> {
        match structural_wake_to_world_model(wake_ref) {
            Some(wake) => self
                .actor
                .resolves_wake(&wake)
                .map_err(RuntimeAssemblyError::SupervisorHandoff),
            None => Ok(false),
        }
    }
}

impl TaskAdmissionHandle {
    fn tick(&mut self, budget: WorkBudget) -> WorkerTickReport {
        let mut network = match self.network.lock() {
            Ok(network) => network,
            Err(_) => {
                return WorkerTickReport::fatal(
                    "execution.task_admission",
                    "execution",
                    Some("task_admission"),
                    "task_network_revision",
                    "network_lock_poisoned",
                    "shared task network mutex is poisoned".to_string(),
                )
            }
        };
        match self.actor.run_once(
            &mut network,
            TaskAdmissionRuntimeRequest {
                network_id: self.network_id.clone(),
                max_items: budget.max_items,
            },
        ) {
            Ok(report) => WorkerTickReport::from(report),
            Err(error) => WorkerTickReport::fatal(
                "execution.task_admission",
                "execution",
                Some("task_admission"),
                "task_network_revision",
                "task_admission_actor_failed",
                error.to_string(),
            ),
        }
    }
}

impl NativeOwnerLifecycle for TaskAdmissionHandle {
    fn native_snapshot(&self) -> Result<NativeOwnerLifecycleSnapshot, RuntimeAssemblyError> {
        let network = self.network.lock().map_err(|_| {
            RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
        })?;
        self.actor
            .lifecycle_evidence(&network)
            .map(Into::into)
            .map_err(RuntimeAssemblyError::SupervisorHandoff)
    }

    fn native_readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
        let network = self.network.lock().map_err(|_| {
            RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
        })?;
        let (evidence, transition) = self
            .actor
            .lifecycle_start(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &network,
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_readiness_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
        let snapshot = self.native_snapshot()?;
        owner_wait_receipt(context, snapshot, report)
    }

    fn native_safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError> {
        let network = self.network.lock().map_err(|_| {
            RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
        })?;
        let (evidence, transition) = self
            .actor
            .lifecycle_safe_point(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &network,
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_safe_point_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
        let network = self.network.lock().map_err(|_| {
            RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
        })?;
        let (evidence, transition) = self
            .actor
            .lifecycle_stop(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &network,
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_stop_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_release(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
        let network = self.network.lock().map_err(|_| {
            RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
        })?;
        let (evidence, transition) = self
            .actor
            .lifecycle_release(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &network,
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_release_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_resolves_wake(
        &self,
        wake_ref: &StructuralWakeRef,
    ) -> Result<bool, RuntimeAssemblyError> {
        let network = self.network.lock().map_err(|_| {
            RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
        })?;
        Ok(structural_wake_to_execution(wake_ref)
            .as_ref()
            .is_some_and(|wake| self.actor.resolves_wake(&network, wake)))
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

impl NativeOwnerLifecycle for DispatchHandle {
    fn native_snapshot(&self) -> Result<NativeOwnerLifecycleSnapshot, RuntimeAssemblyError> {
        let network = self.network.lock().map_err(|_| {
            RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
        })?;
        self.actor
            .as_ref()
            .ok_or_else(missing_native_lifecycle_owner)?
            .lifecycle_evidence(&network)
            .map(Into::into)
            .map_err(RuntimeAssemblyError::SupervisorHandoff)
    }

    fn native_readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
        let network = self.network.lock().map_err(|_| {
            RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
        })?;
        let (evidence, transition) = self
            .actor
            .as_ref()
            .ok_or_else(missing_native_lifecycle_owner)?
            .lifecycle_start(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &network,
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_readiness_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
        let snapshot = self.native_snapshot()?;
        owner_wait_receipt(context, snapshot, report)
    }

    fn native_safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError> {
        let network = self.network.lock().map_err(|_| {
            RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
        })?;
        let (evidence, transition) = self
            .actor
            .as_ref()
            .ok_or_else(missing_native_lifecycle_owner)?
            .lifecycle_safe_point(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &network,
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_safe_point_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
        let network = self.network.lock().map_err(|_| {
            RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
        })?;
        let (evidence, transition) = self
            .actor
            .as_ref()
            .ok_or_else(missing_native_lifecycle_owner)?
            .lifecycle_stop(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &network,
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_stop_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_release(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
        let network = self.network.lock().map_err(|_| {
            RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
        })?;
        let (evidence, transition) = self
            .actor
            .as_ref()
            .ok_or_else(missing_native_lifecycle_owner)?
            .lifecycle_release(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &network,
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_release_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_resolves_wake(
        &self,
        wake_ref: &StructuralWakeRef,
    ) -> Result<bool, RuntimeAssemblyError> {
        let network = self.network.lock().map_err(|_| {
            RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
        })?;
        let actor = self
            .actor
            .as_ref()
            .ok_or_else(missing_native_lifecycle_owner)?;
        Ok(structural_wake_to_execution(wake_ref)
            .as_ref()
            .is_some_and(|wake| actor.resolves_wake(&network, wake)))
    }
}

impl PublicationHandle {
    fn publication_request(&self, limit: Option<usize>) -> PublishPendingPublicationsRequest {
        PublishPendingPublicationsRequest {
            session_id: self.bindings.session_id.clone(),
            worker_id: self.worker_id.clone(),
            limit,
        }
    }

    fn tick(&mut self, budget: WorkBudget) -> WorkerTickReport {
        let actor_id = "execution.publication";
        let mut report = WorkerTickReport {
            actor_id: actor_id.to_string(),
            scope: worker_scope(
                "execution",
                Some("task_publication"),
                None,
                Some(self.bindings.subject.index_key()),
            ),
            input_checkpoint: WorkerCheckpoint {
                name: "task_network_revision".to_string(),
                value: 0,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "task_network_revision".to_string(),
                value: 0,
            },
            items_attempted: 0,
            items_committed: 0,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
            waiting_on: Vec::new(),
        };
        // The durable outbox the task
        // network reducer fills on every recorded outcome, appended to the
        // ledger through the NAG-2 bridge under deterministic record ids so
        // retry is idempotent.
        if let Some(network) = &self.network {
            let mut store = network.lock().unwrap_or_else(|e| e.into_inner());
            report.input_checkpoint.value = store.state().revision;
            match self.runtime.publish_pending(
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
                    report.output_checkpoint.value = store.state().revision;
                    report.waiting_on = execution_waiting(bridge.waiting_on);
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
        } else {
            report.fatal_errors.push(WorkerTickIssue {
                item_id: None,
                code: "publication_task_network_unresolved".to_string(),
                message: "publication task network is unresolved".to_string(),
            });
        }
        report
    }
}

impl NativeOwnerLifecycle for PublicationHandle {
    fn native_snapshot(&self) -> Result<NativeOwnerLifecycleSnapshot, RuntimeAssemblyError> {
        let network = self
            .network
            .as_ref()
            .ok_or_else(missing_native_lifecycle_owner)?
            .lock()
            .map_err(|_| {
                RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
            })?;
        self.runtime
            .lifecycle_evidence(
                &network,
                &self.event_append,
                &self.publication_request(None),
            )
            .map(Into::into)
            .map_err(RuntimeAssemblyError::SupervisorHandoff)
    }

    fn native_readiness(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReadinessReceiptV1, RuntimeAssemblyError> {
        let network = self
            .network
            .as_ref()
            .ok_or_else(missing_native_lifecycle_owner)?
            .lock()
            .map_err(|_| {
                RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
            })?;
        let (evidence, transition) = self
            .runtime
            .lifecycle_start(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &network,
                &self.event_append,
                &self.publication_request(None),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_readiness_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_wait(
        &self,
        context: &ParticipantLifecycleContextV1,
        report: &WorkerTickReport,
    ) -> Result<OwnerWaitReceiptV1, RuntimeAssemblyError> {
        let snapshot = self.native_snapshot()?;
        owner_wait_receipt(context, snapshot, report)
    }

    fn native_safe_point(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerSafePointReceiptV1, RuntimeAssemblyError> {
        let network = self
            .network
            .as_ref()
            .ok_or_else(missing_native_lifecycle_owner)?
            .lock()
            .map_err(|_| {
                RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
            })?;
        let (evidence, transition) = self
            .runtime
            .lifecycle_safe_point(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &network,
                &self.event_append,
                &self.publication_request(None),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_safe_point_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_stop(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerStopReceiptV1, RuntimeAssemblyError> {
        let network = self
            .network
            .as_ref()
            .ok_or_else(missing_native_lifecycle_owner)?
            .lock()
            .map_err(|_| {
                RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
            })?;
        let (evidence, transition) = self
            .runtime
            .lifecycle_stop(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &network,
                &self.event_append,
                &self.publication_request(None),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_stop_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_release(
        &mut self,
        context: &ParticipantLifecycleContextV1,
    ) -> Result<OwnerReleaseReceiptV1, RuntimeAssemblyError> {
        let network = self
            .network
            .as_ref()
            .ok_or_else(missing_native_lifecycle_owner)?
            .lock()
            .map_err(|_| {
                RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
            })?;
        let (evidence, transition) = self
            .runtime
            .lifecycle_release(
                (
                    context.generation_id.clone(),
                    context.incarnation_id.clone(),
                )
                    .into(),
                &network,
                &self.event_append,
                &self.publication_request(None),
            )
            .map_err(RuntimeAssemblyError::SupervisorHandoff)?;
        owner_release_receipt(
            context,
            evidence.into(),
            verified_native_transition(context, transition)?,
        )
    }

    fn native_resolves_wake(
        &self,
        wake_ref: &StructuralWakeRef,
    ) -> Result<bool, RuntimeAssemblyError> {
        let network = self
            .network
            .as_ref()
            .ok_or_else(missing_native_lifecycle_owner)?
            .lock()
            .map_err(|_| {
                RuntimeAssemblyError::SupervisorHandoff("native network binding is poisoned".into())
            })?;
        Ok(structural_wake_to_execution(wake_ref)
            .as_ref()
            .is_some_and(|wake| self.runtime.resolves_wake(&network, wake)))
    }
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
        .map(crate::runtime::contracts::WaitingOnDeclaration::from_world_model)
        .collect()
}

/// Translate execution waiting-on declarations into the carrier shape.
fn execution_waiting(
    declarations: Vec<meld_execution::WaitingOnDeclaration>,
) -> Vec<crate::runtime::contracts::WaitingOnDeclaration> {
    declarations
        .into_iter()
        .map(crate::runtime::contracts::WaitingOnDeclaration::from_execution)
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

fn desired_runtime_state_for_registration_set(
    registry: &RuntimeFactoryRegistry,
    registrations: &RegistrationSet,
    disabled_ids: &BTreeSet<String>,
) -> Vec<DesiredRuntimeState> {
    let mut states = registrations
        .registrations
        .iter()
        .map(|registration| DesiredRuntimeState {
            runtime_id: registration.runtime_id.clone(),
            enabled: !disabled_ids.contains(&registration.runtime_id),
            factory_available: registry.contains(&registration.runtime_id),
        })
        .collect::<Vec<_>>();
    states.sort_by(|left, right| left.runtime_id.cmp(&right.runtime_id));
    states
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
    fn dispatch_without_a_provider_still_requires_its_exact_invocation_binding() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["execution.task_dispatch".to_string()];
        config.disabled_runtime_ids = Vec::new();

        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        assert!(!assembly
            .handle_factories()
            .get("execution.task_dispatch")
            .unwrap()
            .has_semantic_body());
    }

    #[test]
    fn provider_availability_does_not_replace_dispatch_binding() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["execution.task_dispatch".to_string()];
        config.disabled_runtime_ids = Vec::new();
        config.provider.provider_available = true;

        let assembly = ProductRuntimeAssembly::load(config).unwrap();

        assert!(!assembly
            .handle_factories()
            .get("execution.task_dispatch")
            .unwrap()
            .has_semantic_body());
        assert!(!assembly.ports().adapters().provider().is_required());
        assert!(assembly.ports().adapters().provider().is_available());
    }

    #[test]
    fn optional_provider_accepts_present_environment_credentials() {
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
        assert!(!assembly
            .handle_factories()
            .get("execution.task_dispatch")
            .unwrap()
            .has_semantic_body());
        assert!(!assembly.ports().adapters().provider().is_required());
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
        assert_eq!(
            registry
                .descriptors()
                .filter(|descriptor| descriptor.runtime_id == "execution.task_admission")
                .count(),
            1
        );
        assert!(registry.get("execution.planning").is_none());
        assert!(registry.get("execution.goal_set").is_none());
        let descriptor = registry.get(AGENT_RECONCILIATION_RUNTIME_ID).unwrap();
        assert_eq!(
            descriptor.required_resources,
            vec![RuntimeResource::WorldModel]
        );
    }

    #[test]
    fn reopened_agent_does_not_first_admit_a_task_from_a_stale_plan() {
        for (changed, admitted) in [(true, false), (false, false), (true, true)] {
            let harness = StewardshipHarness::new();
            let subject = stewardship_subject_ref(&harness.binding).unwrap();
            {
                let assembly = harness.assembly();
                harness.run_workspace_fixture_genesis(&assembly);
                assembly
                    .ports()
                    .event_append()
                    .append_envelope_idempotent(workspace_owner_publication(
                        subject.clone(),
                        "workspace-before-interruption",
                    ))
                    .unwrap();
                assembly
                    .graph_runtime()
                    .catch_up_bounded(GraphCatchUpBudget { max_items: 128 })
                    .unwrap();
            }
            let attempted = Arc::new(Mutex::new(None));
            let (authorization, predecessor, fence) = {
                let mut assembly = harness.assembly();
                let RuntimeSemanticHandleFactory::AgentActor(factory) = &mut assembly
                    .handle_factories
                    .factories
                    .get_mut(AGENT_RECONCILIATION_RUNTIME_ID)
                    .unwrap()
                    .semantic
                else {
                    unreachable!()
                };
                factory.execution = Arc::new(InterruptedTaskOffer {
                    inner: factory.execution.clone(),
                    attempted: attempted.clone(),
                    admitted,
                });
                let _supervisor = harness.start_supervisor(&assembly);
                let mut handles: Vec<_> = [
                    "world_model.belief_assessment",
                    "world_model.standing_curation",
                    AGENT_RECONCILIATION_RUNTIME_ID,
                ]
                .into_iter()
                .map(|id| {
                    let mut handle = assembly.handle_factories().get(id).unwrap().build_handle();
                    handle
                        .start_after_lease(RuntimeLeaseContext {
                            runtime_id: id.into(),
                            lease_id: format!("interrupted-offer::{id}"),
                        })
                        .unwrap();
                    handle
                })
                .collect();
                for _ in 0..40 {
                    assembly
                        .graph_runtime()
                        .catch_up_bounded(GraphCatchUpBudget { max_items: 128 })
                        .unwrap();
                    for handle in &mut handles {
                        handle.tick(WorkBudget { max_items: 8 }).unwrap();
                    }
                    if attempted.lock().unwrap().is_some() {
                        break;
                    }
                }
                let authorization = attempted
                    .lock()
                    .unwrap()
                    .clone()
                    .expect("Agent must persist a Task offer before interruption");
                let predecessor = assembly
                    .stores()
                    .agent_store
                    .current_reconciliation_plan(&authorization.goal_id)
                    .unwrap()
                    .unwrap();
                let RuntimeSemanticHandleFactory::AgentActor(factory) = &assembly
                    .handle_factories()
                    .get(AGENT_RECONCILIATION_RUNTIME_ID)
                    .unwrap()
                    .semantic
                else {
                    unreachable!()
                };
                let fence = factory.authority_port.observe().unwrap().unwrap();
                assembly.flush_product_boundary().unwrap();
                (authorization, predecessor, fence)
            };
            let assembly = harness.assembly();
            let RuntimeSemanticHandleFactory::AgentActor(factory) = &assembly
                .handle_factories()
                .get(AGENT_RECONCILIATION_RUNTIME_ID)
                .unwrap()
                .semantic
            else {
                unreachable!()
            };
            assert_eq!(factory.authority_port.observe().unwrap(), Some(fence));
            if changed {
                assembly
                    .ports()
                    .event_append()
                    .append_envelope_idempotent(workspace_owner_publication(
                        subject,
                        "workspace-after-interruption",
                    ))
                    .unwrap();
                assembly
                    .graph_runtime()
                    .catch_up_bounded(GraphCatchUpBudget { max_items: 128 })
                    .unwrap();
            }
            let mut handle = assembly
                .handle_factories()
                .get(AGENT_RECONCILIATION_RUNTIME_ID)
                .unwrap()
                .build_handle();
            handle
                .start_after_lease(RuntimeLeaseContext {
                    runtime_id: AGENT_RECONCILIATION_RUNTIME_ID.into(),
                    lease_id: "resumed-offer".into(),
                })
                .unwrap();
            let report = handle.tick(WorkBudget { max_items: 8 }).unwrap();
            assert!(report.fatal_errors.is_empty(), "{report:?}");
            let RuntimeSemanticHandleFactory::TaskAdmission(execution) = &assembly
                .handle_factories()
                .get("execution.task_admission")
                .unwrap()
                .semantic
            else {
                unreachable!()
            };
            let network = execution.network.lock().unwrap();
            let old_admissions: Vec<_> = network
                .state()
                .admissions
                .values()
                .filter(|record| {
                    record.request.lineage.authorization_id == authorization.authorization_id
                })
                .collect();
            let current = assembly
                .stores()
                .agent_store
                .current_reconciliation_plan(&authorization.goal_id)
                .unwrap()
                .unwrap();
            if changed && !admitted {
                assert!(
                    old_admissions.is_empty(),
                    "stale recovery created the first predecessor admission"
                );
                assert!(network.state().tasks.is_empty());
                assert!(network.state().claims.is_empty());
                assert!(network.state().outcomes.is_empty());
                assert_ne!(
                    current.plan_revision_id, predecessor.plan_revision_id,
                    "{report:?}"
                );
                assert_eq!(
                    current.predecessor_plan_revision_id.as_ref(),
                    Some(&predecessor.plan_revision_id)
                );
            } else {
                assert_eq!(
                    old_admissions.len(),
                    1,
                    "current offers recover and admitted predecessors remain observable"
                );
                assert_eq!(current.plan_revision_id, predecessor.plan_revision_id);
            }
        }
    }

    struct InterruptedTaskOffer {
        inner: Arc<dyn meld_world_model::AgentExecutionPort>,
        attempted: Arc<Mutex<Option<meld_world_model::AgentProductAuthorization>>>,
        admitted: bool,
    }

    impl meld_world_model::AgentExecutionPort for InterruptedTaskOffer {
        fn submit(
            &self,
            authorization: &meld_world_model::AgentProductAuthorization,
        ) -> Result<meld_world_model::AgentExecutionPosition, meld_world_model::error::StorageError>
        {
            if self.admitted {
                self.inner.submit(authorization)?;
            }
            *self.attempted.lock().unwrap() = Some(authorization.clone());
            Err(meld_world_model::error::StorageError::InvalidPath(
                "injected interruption at the consumer boundary".into(),
            ))
        }

        fn advance(
            &self,
            authorization: &meld_world_model::AgentProductAuthorization,
        ) -> Result<meld_world_model::AgentExecutionPosition, meld_world_model::error::StorageError>
        {
            self.inner.advance(authorization)
        }

        fn observe(
            &self,
            authorization: &meld_world_model::AgentProductAuthorization,
        ) -> Result<
            Option<meld_world_model::AgentExecutionPosition>,
            meld_world_model::error::StorageError,
        > {
            self.inner.observe(authorization)
        }
    }

    #[test]
    fn root_handle_drives_agent_task_through_execution_terminal_absorption() {
        use meld_world_model::agent::AgentActivationRecord;

        let harness = StewardshipHarness::new();
        let subject = stewardship_subject_ref(&harness.binding).unwrap();
        let rule = standing_curation_rule(subject.clone());
        {
            let assembly = harness.assembly();
            harness.run_workspace_fixture_genesis(&assembly);
            assembly
                .stores()
                .agent_store
                .put_activation(&AgentActivationRecord {
                    activation_id: harness.prepared_activation_id(&assembly),
                    agent_id: STEWARD_AGENT_ID.to_string(),
                    started_at_seq: 6,
                    status: meld_world_model::AgentActivationStatus::Activated,
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

        let assembly = harness.assembly();
        harness.bind_production_routes(&assembly);
        let _supervisor = harness.start_supervisor(&assembly);
        let resolved = ResolvedStewardshipTheory::resolve_prepared_product(
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
        let goal_id = agent_semantic.intent.goal_id(
            &agent_semantic.strategy.agent_id,
            &agent_semantic
                .authority_port
                .observe()
                .unwrap()
                .unwrap()
                .reconciliation_scope(),
        );
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

        assembly.flush_product_boundary().unwrap();
        drop(belief);
        drop(_supervisor);
        drop(assembly);

        let mut last_position = 0;
        let mut exact_operation_id = None;
        let mut exact_acceptance = None;
        let mut exact_result = None;
        let mut exact_semantic_receipt = None;
        let mut exact_terminal_receipt = None;
        for boundary in 0..32 {
            let assembly = harness.assembly();
            let mut handle = assembly
                .handle_factories()
                .get(AGENT_RECONCILIATION_RUNTIME_ID)
                .unwrap()
                .build_handle();
            handle
                .start_after_lease(RuntimeLeaseContext {
                    runtime_id: AGENT_RECONCILIATION_RUNTIME_ID.to_string(),
                    lease_id: format!("agent-root-proof-boundary-{boundary}"),
                })
                .unwrap();
            let report = handle.tick(WorkBudget { max_items: 1 }).unwrap();
            assert_eq!(report.actor_id, AGENT_RECONCILIATION_RUNTIME_ID);
            assert!(
                report.fatal_errors.is_empty(),
                "boundary {boundary}: {report:#?}"
            );
            assert_eq!(report.input_checkpoint.value, last_position);
            assert!(
                report.output_checkpoint.value >= report.input_checkpoint.value,
                "boundary {boundary}: {report:#?}"
            );
            last_position = report.output_checkpoint.value;

            if let Some(operation) = assembly
                .stores()
                .curation_store
                .next_planned_operation(STEWARD_AGENT_ID)
                .unwrap()
            {
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
                let acceptance = assembly
                    .stores()
                    .curation_store
                    .acceptance_for_planned_operation(&operation.operation_id)
                    .unwrap()
                    .unwrap();
                let result = assembly
                    .stores()
                    .curation_store
                    .result_for_operation(&operation.operation_id)
                    .unwrap()
                    .unwrap();
                let semantic_receipt = assembly
                    .stores()
                    .curation_store
                    .publication_receipt(
                        &result.result_id,
                        meld_world_model::CurationPublicationKind::Semantic,
                    )
                    .unwrap();
                let terminal_receipt = assembly
                    .stores()
                    .curation_store
                    .publication_receipt(
                        &result.result_id,
                        meld_world_model::CurationPublicationKind::Terminal,
                    )
                    .unwrap()
                    .unwrap();
                if let Some(receipt) = &semantic_receipt {
                    assert_ne!(receipt.event_record_id, terminal_receipt.event_record_id);
                }
                let records = assembly
                    .ports()
                    .event_replay()
                    .read_after_limit(0, 32)
                    .unwrap();
                assert_eq!(
                    records
                        .iter()
                        .filter(|record| {
                            record.envelope.event_type == "world-model.agent-genesis.v1"
                        })
                        .count(),
                    1
                );
                if let Some(receipt) = &semantic_receipt {
                    assert!(records
                        .iter()
                        .any(|record| record.envelope.record_id.as_deref()
                            == Some(receipt.event_record_id.as_str())));
                }
                assert!(records.iter().any(|record| {
                    record.envelope.record_id.as_deref()
                        == Some(terminal_receipt.event_record_id.as_str())
                }));
                assembly
                    .graph_runtime()
                    .catch_up_bounded(GraphCatchUpBudget { max_items: 16 })
                    .unwrap();
                if exact_operation_id.is_none() {
                    exact_operation_id = Some(operation.operation_id);
                    exact_acceptance = Some(acceptance);
                    exact_result = Some(result);
                    exact_semantic_receipt = semantic_receipt;
                    exact_terminal_receipt = Some(terminal_receipt);
                }
            }
            let task_authorized = assembly
                .stores()
                .agent_store
                .product_authorizations_for_goal(&goal_id)
                .unwrap()
                .iter()
                .any(|record| {
                    matches!(
                        record.product,
                        meld_world_model::AgentAuthorizedProduct::Task(_)
                    )
                });
            assembly.flush_product_boundary().unwrap();
            drop(handle);
            drop(assembly);
            if task_authorized {
                break;
            }
        }

        let assembly = harness.assembly();
        harness.bind_production_routes(&assembly);
        let operation_id = exact_operation_id.unwrap();
        let result = exact_result.unwrap();
        assert_eq!(
            assembly
                .stores()
                .curation_store
                .acceptance_for_planned_operation(&operation_id)
                .unwrap(),
            exact_acceptance
        );
        assert_eq!(
            assembly
                .stores()
                .curation_store
                .result_for_operation(&operation_id)
                .unwrap(),
            Some(result.clone())
        );
        assert_eq!(
            assembly
                .stores()
                .curation_store
                .publication_receipt(
                    &result.result_id,
                    meld_world_model::CurationPublicationKind::Semantic,
                )
                .unwrap(),
            exact_semantic_receipt
        );
        assert_eq!(
            assembly
                .stores()
                .curation_store
                .publication_receipt(
                    &result.result_id,
                    meld_world_model::CurationPublicationKind::Terminal,
                )
                .unwrap(),
            exact_terminal_receipt
        );
        let records = assembly
            .ports()
            .event_replay()
            .read_after_limit(0, 32)
            .unwrap();
        assert_eq!(
            records
                .iter()
                .filter(|record| record.envelope.event_type == "world-model.agent-genesis.v1")
                .count(),
            1
        );
        assert!(records.iter().any(|record| {
            exact_semantic_receipt.as_ref().is_some_and(|receipt| {
                record.envelope.record_id.as_deref() == Some(receipt.event_record_id.as_str())
            })
        }));
        assert!(records.iter().any(|record| {
            exact_terminal_receipt.as_ref().is_some_and(|receipt| {
                record.envelope.record_id.as_deref() == Some(receipt.event_record_id.as_str())
            })
        }));
        let mut handle = assembly
            .handle_factories()
            .get(AGENT_RECONCILIATION_RUNTIME_ID)
            .unwrap()
            .build_handle();
        handle
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: AGENT_RECONCILIATION_RUNTIME_ID.to_string(),
                lease_id: "agent-root-proof-replay".to_string(),
            })
            .unwrap();
        let replay = handle.tick(WorkBudget { max_items: 8 }).unwrap();
        assert_eq!(replay.input_checkpoint.value, last_position);
        assert_eq!(
            replay.output_checkpoint.value,
            replay.input_checkpoint.value
        );
        assert_eq!(replay.items_committed, 0);
        assert!(replay
            .waiting_on
            .iter()
            .all(|wait| wait.subject_key.is_some()));
        assert!(
            replay
                .waiting_on
                .iter()
                .any(|waiting| waiting.condition == "execution_terminal_outcome"),
            "{replay:#?}"
        );
        let authorizations = assembly
            .stores()
            .agent_store
            .product_authorizations_for_goal(&goal_id)
            .unwrap();
        let task_authorization = authorizations
            .iter()
            .find(|authorization| {
                matches!(
                    authorization.product,
                    meld_world_model::AgentAuthorizedProduct::Task(_)
                )
            })
            .unwrap();
        assert!(task_authorization.authority_decision.is_some());
        let meld_world_model::AgentAuthorizedProduct::Task(task) = &task_authorization.product
        else {
            unreachable!("selected authorization must retain its exact Task body");
        };
        let mut capability_type_ids = task
            .composition
            .steps
            .iter()
            .map(|step| {
                let meld_lang::StepKind::Op(operator) = &step.kind else {
                    panic!("the accepted docs Task must contain only operational steps");
                };
                operator
                    .resolution
                    .specific
                    .as_ref()
                    .unwrap()
                    .capability_type_id
                    .as_str()
            })
            .collect::<Vec<_>>();
        capability_type_ids.sort_unstable();
        assert_eq!(
            capability_type_ids,
            vec![
                "docs.draft_patch_set",
                "docs.inspect_scope",
                "docs.publish_patch_set",
                "docs.validate_patch_set",
            ]
        );

        for capability_type_id in &capability_type_ids {
            assert!(assembly
                .capability_runtime()
                .unwrap()
                .catalog
                .get(capability_type_id, 1)
                .is_some());
        }
        let mut planning = assembly
            .handle_factories()
            .get("execution.task_admission")
            .unwrap()
            .build_handle();
        planning
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "execution.task_admission".to_string(),
                lease_id: "planning-root-proof".to_string(),
            })
            .unwrap();
        let planning_report = planning.tick(WorkBudget { max_items: 8 }).unwrap();
        assert!(
            planning_report.fatal_errors.is_empty(),
            "{planning_report:#?}"
        );
        assert_eq!(planning_report.items_committed, 1, "{planning_report:#?}");
        {
            let planning_factory = assembly
                .handle_factories()
                .get("execution.task_admission")
                .unwrap();
            let RuntimeSemanticHandleFactory::TaskAdmission(planning_semantic) =
                &planning_factory.semantic
            else {
                panic!("production Task admission descriptor must resolve direct Task admission");
            };
            let network = planning_semantic.network.lock().unwrap();
            let admitted_nodes = network
                .state()
                .tasks
                .values()
                .filter(|node| {
                    node.lineage.admission.as_ref().is_some_and(|admission| {
                        admission.authorization_id == task_authorization.authorization_id
                    })
                })
                .collect::<Vec<_>>();
            assert_eq!(admitted_nodes.len(), 4);
            let init_sources = admitted_nodes
                .iter()
                .flat_map(|node| &node.init_sources)
                .collect::<Vec<_>>();
            assert_eq!(init_sources.len(), 4);
            assert!(init_sources.iter().all(|source| matches!(
                source,
                meld_execution::task_network::TaskInitSource::UpstreamArtifact(_)
            )));
        }

        let mut dispatch = assembly
            .handle_factories()
            .get("execution.task_dispatch")
            .unwrap()
            .build_handle();
        dispatch
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "execution.task_dispatch".to_string(),
                lease_id: "dispatch-root-proof".to_string(),
            })
            .unwrap();
        let mut dispatch_commits = 0;
        let mut dispatch_reports = Vec::new();
        for _ in 0..8 {
            let dispatch_report = dispatch.tick(WorkBudget { max_items: 8 }).unwrap();
            assert!(
                dispatch_report.fatal_errors.is_empty(),
                "{dispatch_report:#?}"
            );
            dispatch_commits += dispatch_report.items_committed;
            let idle = dispatch_report.items_attempted == 0;
            dispatch_reports.push(dispatch_report);
            if idle {
                break;
            }
        }
        assert_eq!(dispatch_commits, 3, "{dispatch_reports:#?}");
        assert_eq!(
            dispatch_reports
                .iter()
                .map(|report| report.items_attempted)
                .sum::<usize>(),
            4,
            "{dispatch_reports:#?}"
        );

        let mut publication = assembly
            .handle_factories()
            .get("execution.publication")
            .unwrap()
            .build_handle();
        publication
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "execution.publication".to_string(),
                lease_id: "publication-root-proof".to_string(),
            })
            .unwrap();
        let publication_report = publication.tick(WorkBudget { max_items: 8 }).unwrap();
        assert!(
            publication_report.fatal_errors.is_empty(),
            "{publication_report:#?}"
        );
        assert!(
            publication_report.items_committed > 0,
            "{publication_report:#?}"
        );

        let graph_report = assembly
            .graph_runtime()
            .catch_up_bounded(GraphCatchUpBudget { max_items: 32 })
            .unwrap();
        assert!(graph_report.events_attempted > 0);
        assert!(
            graph_report.retryable_errors.is_empty(),
            "{graph_report:#?}"
        );
        assert!(graph_report.fatal_errors.is_empty(), "{graph_report:#?}");
        assert!(graph_report.output_event_seq > graph_report.input_event_seq);
        for _ in 0..8 {
            let report = assembly
                .graph_runtime()
                .catch_up_bounded(GraphCatchUpBudget { max_items: 32 })
                .unwrap();
            assert!(
                report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
                "{report:?}"
            );
            if report.output_event_seq
                == assembly
                    .event_authority()
                    .watermark_capability()
                    .snapshot()
                    .unwrap()
                    .committed_seq
            {
                break;
            }
        }
        let terminal_return = handle.tick(WorkBudget { max_items: 8 }).unwrap();
        assert!(
            terminal_return.fatal_errors.is_empty(),
            "{terminal_return:#?}"
        );
        assert!(terminal_return.items_committed > 0, "{terminal_return:#?}");
        assert!(
            !terminal_return
                .waiting_on
                .iter()
                .any(|waiting| waiting.condition == "execution_terminal_outcome"),
            "{terminal_return:#?}"
        );
        let planning_factory = assembly
            .handle_factories()
            .get("execution.task_admission")
            .unwrap();
        let RuntimeSemanticHandleFactory::TaskAdmission(planning_semantic) =
            &planning_factory.semantic
        else {
            panic!("production Task admission descriptor must resolve direct Task admission");
        };
        let terminal_outcome_id = {
            let network = planning_semantic.network.lock().unwrap();
            let terminal_outcome = network
                .state()
                .outcomes
                .values()
                .find(|outcome| {
                    outcome.admission.as_ref().is_some_and(|admission| {
                        admission.authorization_id == task_authorization.authorization_id
                    }) && outcome.status
                        == meld_execution::task_network::dispatch::OutcomeStatus::Failed
                })
                .unwrap();
            let publication = network
                .state()
                .publications
                .values()
                .find(|publication| publication.outcome.outcome_id == terminal_outcome.outcome_id)
                .unwrap();
            assert!(matches!(
                publication.state,
                meld_execution::task_network::outcome::PublicationState::Published {
                    receipt: Some(_),
                    ..
                }
            ));
            terminal_outcome.outcome_id.clone()
        };
        let requirement = task.return_milestone.as_ref().unwrap();
        assert_eq!(
            requirement,
            &meld_world_model::PlanMilestoneRequirement::ExecutionTerminal {
                task_id: task.task_id.clone(),
            }
        );
        let milestone_id = {
            let bytes = serde_json::to_vec(&(
                &task_authorization.agent_id,
                &task_authorization.goal_id,
                &task_authorization.plan_revision_id,
                &task.task_id,
                requirement,
                &terminal_outcome_id,
                &task_authorization.activation_generation,
            ))
            .unwrap();
            format!(
                "agent-task-milestone-acceptance-v1::{}",
                blake3::hash(&bytes).to_hex()
            )
        };
        let milestone = assembly
            .stores()
            .agent_store
            .milestone(&milestone_id)
            .unwrap()
            .unwrap();
        assert_eq!(milestone.requirement, requirement.clone());
        assert_eq!(milestone.owner_position_id, terminal_outcome_id);

        assembly
            .stores()
            .agent_store
            .put_activation(&AgentActivationRecord {
                activation_id: "activation-root-v2".to_string(),
                agent_id: STEWARD_AGENT_ID.to_string(),
                started_at_seq: 8,
                status: meld_world_model::AgentActivationStatus::Activated,
                last_error: None,
                lease_id: Some("agent-activation-root-v2".to_string()),
            })
            .unwrap();
        let stale = handle.tick(WorkBudget { max_items: 8 }).unwrap();
        assert!(!stale
            .waiting_on
            .iter()
            .any(|wait| wait.condition == "agent_authority_changed"));
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
    fn native_owner_transitions_author_receipts_and_reject_stop_before_safe_point() {
        let temp = tempfile::tempdir().unwrap();
        let assembly = ProductRuntimeAssembly::load_for_product_root(temp.path()).unwrap();
        let mut handle = assembly
            .handle_factories()
            .get("world_model.graph_replay")
            .unwrap()
            .build_handle();
        let context = ParticipantLifecycleContextV1 {
            generation_id: "generation-a".to_string(),
            incarnation_id: "incarnation-a".to_string(),
            realization_id: "realization-a".to_string(),
            participant_id: "world_model.graph_replay".to_string(),
            owner_domain: "world_state".to_string(),
            kind: crate::theory::ParticipantKind::BoundedActor,
            readiness_contract_ref: "readiness-a".to_string(),
            wake_contract_ref: "wake-a".to_string(),
            safe_point_contract_ref: "safe-a".to_string(),
            stop_contract_ref: "stop-a".to_string(),
            lease_ref: "lease-a".to_string(),
        };
        let start = handle
            .start_after_lifecycle_lease(
                RuntimeLeaseContext {
                    runtime_id: context.participant_id.clone(),
                    lease_id: context.lease_ref.clone(),
                },
                &context,
            )
            .unwrap();
        let first_readiness_proof = start
            .owner_readiness
            .as_ref()
            .unwrap()
            .native_evidence
            .proof_position_ref
            .clone();
        assert!(first_readiness_proof.contains("Constructed-to-Running"));

        let quiet = handle.tick(WorkBudget { max_items: 8 }).unwrap();
        let wait = handle.lifecycle_wait(&context, &quiet).unwrap();
        assert!(matches!(
            wait.wake_refs.as_slice(),
            [StructuralWakeRef::EventPosition(_)]
        ));
        assert!(handle
            .resolves_lifecycle_wake(
                &context.generation_id,
                &context.incarnation_id,
                &wait.wake_refs[0]
            )
            .unwrap());
        assert!(!handle
            .resolves_lifecycle_wake(
                &context.generation_id,
                "foreign-incarnation",
                &wait.wake_refs[0]
            )
            .unwrap());

        let premature = handle.request_lifecycle_stop(&context).unwrap_err();
        assert!(premature.to_string().contains("Running to Stopped"));
        assert!(handle.is_started());

        let safe = handle.wait_for_lifecycle_safe_point(&context).unwrap();
        assert!(safe
            .owner_safe_point
            .as_ref()
            .unwrap()
            .proof_refs
            .iter()
            .any(|proof| proof.contains("Running-to-SafePoint")));
        let stop = handle.request_lifecycle_stop(&context).unwrap();
        assert!(!handle
            .resolves_lifecycle_wake(
                &context.generation_id,
                &context.incarnation_id,
                &wait.wake_refs[0]
            )
            .unwrap());
        assert!(stop
            .owner_stop
            .as_ref()
            .unwrap()
            .owner_proof_ref
            .contains("SafePoint-to-Stopped"));
        let release = handle.release_lifecycle(&context).unwrap();
        assert!(release.owner_proof_ref.contains("Stopped-to-Released"));

        let mut successor = assembly
            .handle_factories()
            .get("world_model.graph_replay")
            .unwrap()
            .build_handle();
        let successor_context = ParticipantLifecycleContextV1 {
            incarnation_id: "incarnation-b".to_string(),
            lease_ref: "lease-b".to_string(),
            ..context.clone()
        };
        let successor_start = successor
            .start_after_lifecycle_lease(
                RuntimeLeaseContext {
                    runtime_id: successor_context.participant_id.clone(),
                    lease_id: successor_context.lease_ref.clone(),
                },
                &successor_context,
            )
            .unwrap();
        let successor_readiness = successor_start.owner_readiness.unwrap();
        assert_eq!(
            successor_readiness.incarnation_id,
            successor_context.incarnation_id
        );
        assert!(successor_readiness
            .native_evidence
            .proof_position_ref
            .contains("incarnation-b"));
        assert_ne!(
            successor_readiness.native_evidence.proof_position_ref,
            first_readiness_proof
        );
    }

    #[test]
    fn duplicate_runtime_ids_fail_registry_construction() {
        let descriptor = RuntimeFactoryDescriptor::new(
            "execution.task_admission",
            vec![RuntimeResource::Provider],
        )
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

    fn standing_curation_outcome_mapping() -> OutcomeMappingSetConfig {
        OutcomeMappingSetConfig {
            mapping_id: MAPPING_ID.to_string(),
            rules: vec![OutcomeMappingConfig {
                mapping_id: "standing-curation-applied".to_string(),
                source_kind: "docs_required_coverage".to_string(),
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
                    field: "coverage_probability".to_string(),
                    source: OutcomeValueSource::Constant { value: 1.0 },
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

    fn workspace_fixture_curation_template() -> meld_world_model::curation::CurationRuleTemplate {
        meld_world_model::curation::CurationRuleTemplate {
            rule_id: "workspace-fixture-curation".into(),
            source_owner_id: "workspace_fs".into(),
            traversal_direction:
                meld_world_model::world_state::graph::contracts::TraversalDirection::Incoming,
            bounds: meld_world_model::world_state::graph::contracts::TraversalBounds {
                max_depth: 4,
                max_objects: 32,
                max_occurrences: 32,
                max_paths: 32,
            },
            expected_object_kind: "assessment".into(),
            expected_object_key: "standing-assessment".into(),
            relation_type: "curation_assesses".into(),
            output_policy_revision: "workspace-fixture-output-v1".into(),
            realization: None,
            coverage: None,
        }
    }

    fn standing_curation_rule(subject: DomainObjectRef) -> meld_world_model::StandingCurationRule {
        workspace_fixture_curation_template()
            .ground(&meld_world_model::curation::CurationRuleBinding {
                agent_id: STEWARD_AGENT_ID.into(),
                subject,
                scope: standing_curation_scope(),
            })
            .unwrap()
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

    struct LostDispatchReturn {
        inner: SharedClaimedTaskInvoker,
        invoked: std::sync::atomic::AtomicBool,
        continuous_retry: bool,
        lose_return: Arc<std::sync::atomic::AtomicBool>,
    }

    #[async_trait::async_trait]
    impl meld_execution::task_network::dispatch_actor::ClaimedTaskInvoker for LostDispatchReturn {
        async fn invoke_claimed_task(
            &self,
            node: &meld_execution::task_network::state::TaskNode,
            claim: &meld_execution::task_network::dispatch::Claim,
            payload: &meld_execution::task::TaskInitializationPayload,
        ) -> Result<
            meld_execution::task_network::dispatch_actor::ClaimedInvocationOutcome,
            meld_execution::task_network::dispatch_actor::DispatchPortError,
        > {
            if !self.continuous_retry
                && self.lose_return.load(std::sync::atomic::Ordering::SeqCst)
                && self.invoked.swap(true, std::sync::atomic::Ordering::SeqCst)
            {
                return Err(
                    meld_execution::task_network::dispatch_actor::DispatchPortError::retryable(
                        "callback recovery unavailable",
                    ),
                );
            }
            let result = self
                .inner
                .0
                .invoke_claimed_task(node, claim, payload)
                .await?;
            if self.lose_return.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(
                    meld_execution::task_network::dispatch_actor::DispatchPortError::retryable(
                        "lost callback after real capability execution",
                    ),
                );
            }
            Ok(result)
        }
    }

    struct StewardshipHarness {
        _workspace: tempfile::TempDir,
        _external: tempfile::TempDir,
        binding: PhysicalBinding,
        authority: Arc<EventAuthority>,
    }

    impl StewardshipHarness {
        fn bind_production_routes(&self, assembly: &ProductRuntimeAssembly) {
            self.bind_production_routes_with_loss(assembly, None);
        }

        fn bind_production_routes_with_loss(
            &self,
            assembly: &ProductRuntimeAssembly,
            loss: Option<(Arc<std::sync::atomic::AtomicBool>, bool)>,
        ) -> Arc<crate::api::ContextApi> {
            let route_storage = self._external.path().join("claimed-route");
            std::fs::create_dir_all(&route_storage).unwrap();
            let api = Arc::new(
                crate::api::ContextApi::new(
                    Arc::new(
                        crate::store::SledNodeRecordStore::new(route_storage.join("nodes"))
                            .unwrap(),
                    ),
                    Arc::new(
                        crate::context::frame::FrameStorage::new(route_storage.join("frames"))
                            .unwrap(),
                    ),
                    Arc::new(parking_lot::RwLock::new(crate::heads::HeadIndex::new())),
                    Arc::new(
                        crate::prompt_context::PromptContextArtifactStorage::new(
                            route_storage.join("prompt-artifacts"),
                        )
                        .unwrap(),
                    ),
                    Arc::new(parking_lot::RwLock::new(crate::agent::AgentRegistry::new())),
                    Arc::new(parking_lot::RwLock::new(
                        crate::provider::ProviderRegistry::new(),
                    )),
                    Arc::new(crate::concurrency::NodeLockManager::new()),
                )
                .with_optional_workspace(self.binding.workspace_root.clone()),
            );
            api.bind_event_append(assembly.event_authority().append_capability())
                .unwrap();
            let seed = assembly.dispatch_route_seed().unwrap().clone();
            let capability_runtime = assembly.capability_runtime().unwrap().clone();
            assert!(capability_runtime
                .catalog
                .get("workspace_scan", 1)
                .is_none());
            let mut routes = DispatchRouteBindings::production(
                crate::runtime::ports::ProductionDispatchRouteContext {
                    api: api.clone(),
                    session_id: Some(seed.session_id),
                    catalog: capability_runtime.catalog,
                    registry: capability_runtime.registry,
                },
            );
            if let Some((lose_return, continuous_retry)) = loss {
                routes.claim_invoker = SharedClaimedTaskInvoker(Arc::new(LostDispatchReturn {
                    inner: routes.claim_invoker,
                    invoked: std::sync::atomic::AtomicBool::new(false),
                    continuous_retry,
                    lose_return,
                }));
            }
            assert!(assembly.bind_dispatch_routes(routes));
            api
        }

        fn start_supervisor<'a>(
            &self,
            assembly: &'a ProductRuntimeAssembly,
        ) -> RuntimeSupervisor<'a> {
            if !assembly
                .dispatch_route_slot
                .as_ref()
                .is_some_and(|slot| slot.is_bound())
            {
                assert!(assembly.bind_dispatch_routes(stub_routes()));
            }
            let mut command = SupervisorStartCommand::new("epoch-fixture", 100);
            command.registration_set = assembly.registration_set().cloned();
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap()
        }

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

        fn assembly(&self) -> ProductRuntimeAssembly {
            ProductRuntimeAssembly::load_composed(
                ProductRuntimeConfig::for_product_root(self.binding.storage_root.clone()),
                Arc::clone(&self.authority),
                Some(StewardshipComposition {
                    binding: self.binding.clone(),
                }),
            )
            .unwrap()
        }

        fn world_init_metadata(&self) -> crate::init::world::pipeline::WorldInitRunMetadata {
            crate::init::world::pipeline::WorldInitRunMetadata {
                provenance: "trusted init".to_string(),
                session_id: "assembly-test".to_string(),
                observed_seq: 5,
            }
        }

        fn prepared_activation_id(&self, assembly: &ProductRuntimeAssembly) -> String {
            let head = assembly
                .stores()
                .pds_products
                .prepared_head(&self.binding.package.expression)
                .unwrap()
                .unwrap();
            assembly
                .stores()
                .pds_products
                .prepared_closure(&head.prepared_id)
                .unwrap()
                .unwrap()
                .activation
                .activation_id
        }

        /// Product compilation, Agent genesis, and inert preparation.
        /// Shared Agent and Execution proofs use explicit workspace evidence. Their
        /// fixed Curation vocabulary is independent of the evolving Docs product.
        fn run_workspace_fixture_genesis(&self, assembly: &ProductRuntimeAssembly) {
            let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/docs_freshness");
            let package = tempfile::tempdir().unwrap();
            for entry in std::fs::read_dir(&source).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_file() {
                    std::fs::copy(entry.path(), package.path().join(entry.file_name())).unwrap();
                }
            }
            let template = workspace_fixture_curation_template();
            let bytes = serde_json::to_vec(&template).unwrap();
            std::fs::write(
                package.path().join("epistemic_rule.docs_freshness.json"),
                &bytes,
            )
            .unwrap();
            // This fixture proves prerequisite Curation and Task admission. The
            // native Docs repair test proves the installed confirmation sequence.
            let strategy_path = package.path().join("strategy_theory.docs_freshness.json");
            let mut strategy: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&strategy_path).unwrap()).unwrap();
            strategy["snapshot"]["settlement_rules"][0]["epistemic_placement"] =
                "prerequisite".into();
            let strategy_bytes = serde_json::to_vec(&strategy).unwrap();
            std::fs::write(strategy_path, &strategy_bytes).unwrap();
            let mut manifest: serde_json::Value = serde_json::from_slice(
                &std::fs::read(package.path().join("pds-package.json")).unwrap(),
            )
            .unwrap();
            for component in manifest["components"].as_array_mut().unwrap() {
                if component["content"]["path"] == "strategy_theory.docs_freshness.json" {
                    component["content"]["content_hash"] =
                        blake3::hash(&strategy_bytes).to_hex().to_string().into();
                }
                if component["content"]["path"] == "epistemic_rule.docs_freshness.json" {
                    component["owner_component_id"] = template.rule_id.clone().into();
                    component["content"]["content_hash"] =
                        blake3::hash(&bytes).to_hex().to_string().into();
                }
            }
            std::fs::write(
                package.path().join("pds-package.json"),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
            self.run_world_genesis_from(assembly, package.path());
        }

        fn run_world_genesis_with_claim_policy(
            &self,
            assembly: &ProductRuntimeAssembly,
            policy: &crate::docs::claim_validation::DocsClaimPolicy,
        ) {
            let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/docs_freshness");
            let package = tempfile::tempdir().unwrap();
            for entry in std::fs::read_dir(&source).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_file() {
                    std::fs::copy(entry.path(), package.path().join(entry.file_name())).unwrap();
                }
            }
            let policy_path = "claim_policy.docs-claims-strict-v1.json";
            let bytes = serde_json::to_vec(policy).unwrap();
            std::fs::write(package.path().join(policy_path), &bytes).unwrap();
            let mut manifest: serde_json::Value = serde_json::from_slice(
                &std::fs::read(package.path().join("pds-package.json")).unwrap(),
            )
            .unwrap();
            for component in manifest["components"].as_array_mut().unwrap() {
                if component["content"]["path"] == policy_path {
                    component["content"]["content_hash"] =
                        blake3::hash(&bytes).to_hex().to_string().into();
                }
            }
            std::fs::write(
                package.path().join("pds-package.json"),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
            self.run_world_genesis_from(assembly, package.path());
        }

        fn run_world_genesis(&self, assembly: &ProductRuntimeAssembly) {
            self.run_world_genesis_from(
                assembly,
                &Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/docs_freshness"),
            );
        }

        fn run_world_genesis_from(&self, assembly: &ProductRuntimeAssembly, package_root: &Path) {
            let stores = assembly.stores();
            let package_receipt =
                crate::init::world::product::install_package(stores, package_root, None, 5)
                    .unwrap();
            let mut registry = stores.belief_family_registry.as_ref().clone();
            let product = crate::init::world::tooling::compile_product_initialization(
                stores,
                &self.binding,
                &package_receipt,
                5,
            )
            .unwrap();
            let assignment_id = product.assignment.assignment_id.clone();
            let assignment_subject = product.assignment.subject.clone();
            let assignment_perspective = product.assignment.perspective_id.clone();
            let assignment_branch = product.assignment.branch_id.clone();
            let append = assembly.event_authority().append_capability();
            crate::init::world::pipeline::WorldInitPipeline::new(
                &mut registry,
                stores.belief_store.as_ref(),
                stores.agent_store.as_ref(),
                &append,
            )
            .with_complete_theory(crate::init::world::pipeline::CompleteTheoryInstall {
                routed: crate::init::world::pipeline::RoutedTheoryInstall {
                    receipt: package_receipt,
                    changed: true,
                },
            })
            .with_complete_product(product)
            .run(
                &crate::init::world::WorldInitRequest {
                    stages: vec![
                        crate::init::world::WorldInitStage::InstallTheory,
                        crate::init::world::WorldInitStage::GenesisIdentities,
                        crate::init::world::WorldInitStage::PrepareActivation,
                    ],
                },
                &self.world_init_metadata(),
            )
            .unwrap();
            let receipts = stores
                .agent_store
                .genesis_receipts_for_assignment(&assignment_id)
                .unwrap();
            assert!(!receipts.is_empty());
            assert!(receipts.iter().all(|receipt| {
                receipt.event_position.ledger_id == assembly.event_authority().ledger_identity()
            }));
            let agent = stores
                .agent_store
                .get_agent(&self.binding.agent_id)
                .unwrap()
                .unwrap();
            assert_eq!(agent.subject, assignment_subject);
            assert_eq!(agent.perspective_key.index_key(), assignment_perspective);
            assert_eq!(agent.branch_scope.branch_id, assignment_branch);
            assembly.flush_product_boundary().unwrap();
        }
    }

    #[test]
    fn docs_capabilities_reopen_with_prepared_policy_and_reject_substitution() {
        let harness = StewardshipHarness::new();
        let mut policy = crate::docs::claim_observation::test_support::policy();
        policy.minimum_claim_confidence = 0.93;
        let newer;
        {
            let assembly = harness.assembly();
            harness.run_world_genesis_with_claim_policy(&assembly, &policy);
            let mut other = policy.clone();
            other.minimum_claim_confidence = 0.97;
            newer = assembly
                .stores()
                .claim_policy_registry
                .install(other, 99)
                .unwrap()
                .1;
            assembly.flush_product_boundary().unwrap();
        }
        let assembly = harness.assembly();
        let subject = stewardship_subject_ref(&harness.binding).unwrap();
        let resolved = ResolvedStewardshipTheory::resolve_prepared_product(
            assembly.stores(),
            &harness.binding.package,
            &subject,
        )
        .unwrap();
        let selected = resolved.claim_policy.as_ref().unwrap();
        assert_eq!(selected.policy, policy);
        assert_ne!(selected.content_identity, newer.content_identity);
        let contracts = resolved
            .executable_contracts
            .iter()
            .map(|revision| revision.contract.clone())
            .collect::<Vec<_>>();
        let closure = resolved.prepared_closure.as_ref().unwrap();
        let runtime = activate_exact_capabilities(
            assembly.stores(),
            &harness.binding,
            &contracts,
            closure,
            Some(selected),
        )
        .unwrap();
        assert!(runtime
            .registry
            .get(crate::docs::capability::VALIDATE_PATCH_SET, 1)
            .is_some());
        assert!(runtime
            .registry
            .get(crate::docs::capability::PUBLISH_PATCH_SET, 1)
            .is_some());
        assert!(activate_exact_capabilities(
            assembly.stores(),
            &harness.binding,
            &contracts,
            closure,
            Some(&newer)
        )
        .is_err());
        assert!(activate_exact_capabilities(
            assembly.stores(),
            &harness.binding,
            &contracts,
            closure,
            None
        )
        .is_err());
        let mut diagnostics = Vec::new();
        let hydrated =
            hydrate_stewardship_theory(assembly.stores(), &harness.binding, &mut diagnostics);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert!(hydrated.capability_runtime.is_some());
        assert_eq!(
            hydrated.resolved.unwrap().claim_policy.as_ref().unwrap(),
            selected
        );
    }

    #[test]
    fn installed_docs_publishes_current_observations_before_task_admission() {
        use crate::docs::runtime::DocsObservationActor;
        use meld_world_model::world_state::graph::contracts::{
            TraversalCutRequest, TraversalCutStatus, TraversalOwnerRequirement,
        };
        let harness = StewardshipHarness::new();
        std::fs::write(
            harness._workspace.path().join("lib.rs"),
            "pub fn run() {}\n",
        )
        .unwrap();
        std::fs::write(
            harness._workspace.path().join("README.md"),
            "# Tool\n\n`run` starts it.\n",
        )
        .unwrap();
        {
            let assembly = harness.assembly();
            harness.run_world_genesis(&assembly);
        }
        let assembly = harness.assembly();
        harness.bind_production_routes(&assembly);
        assert_eq!(
            assembly
                .registration_set()
                .unwrap()
                .kind_of("docs.observation"),
            Some(RegistrationKind::ActiveActor)
        );
        let RuntimeSemanticHandleFactory::DocsObservation(binding) = &assembly
            .handle_factories()
            .get("docs.observation")
            .unwrap()
            .semantic
        else {
            panic!("native Docs factory absent")
        };
        let owner = DocsObservationActor::new(binding.as_ref().clone());
        assert!(owner.current_revision().unwrap().is_none());
        let mut supervisor = harness.start_supervisor(&assembly);
        for pass in 0..4 {
            supervisor.tick(1_000 + pass * 10).unwrap();
        }
        let first = owner.current_revision().unwrap().unwrap();
        assert_eq!(first.sequence, 1);
        let operation = first.publication().unwrap();
        assert!(operation
            .batch
            .objects
            .iter()
            .any(|object| object.object_ref.object_kind == "observed_claim"));
        let cut_request = |position| {
            TraversalCutRequest {
            owners: vec![TraversalOwnerRequirement {
                event_source: None,
                owner_id: "docs".into(),
                scope: first.scope.clone(),
                required: true,
            }],
            scope: first.scope.clone(),
            currentness: meld_world_model::world_state::graph::contracts::OwnerCurrentnessPolicy::LatestComplete,
            event_position: position,
        }
        };
        let cursor = assembly.graph_runtime().durable_event_cursor().unwrap();
        let cut = meld_world_model::TraversalQuery::new(assembly.stores().traversal_store.as_ref())
            .cut(&cut_request(cursor))
            .unwrap();
        assert_eq!(cut.status, TraversalCutStatus::Complete);
        assert_eq!(cut.receipts[0].revision_id, first.revision_id);
        let RuntimeSemanticHandleFactory::TaskAdmission(execution) = &assembly
            .handle_factories()
            .get("execution.task_admission")
            .unwrap()
            .semantic
        else {
            panic!("Execution factory absent")
        };
        assert!(execution
            .network
            .lock()
            .unwrap()
            .state()
            .admissions
            .is_empty());
        std::fs::remove_file(harness._workspace.path().join("README.md")).unwrap();
        for pass in 0..4 {
            supervisor.tick(1_100 + pass * 10).unwrap();
        }
        let next = owner.current_revision().unwrap().unwrap();
        assert_eq!(next.predecessor, Some(first.revision_id.clone()));
        assert!(matches!(
            next.evidence.observation.as_ref().unwrap().readmes[0].state,
            crate::docs::observation::ObservedReadmeState::Missing
        ));
        let cursor = assembly.graph_runtime().durable_event_cursor().unwrap();
        let cut = meld_world_model::TraversalQuery::new(assembly.stores().traversal_store.as_ref())
            .cut(&cut_request(cursor))
            .unwrap();
        assert_eq!(cut.status, TraversalCutStatus::Complete);
        assert_eq!(cut.receipts[0].revision_id, next.revision_id);
        std::fs::remove_dir_all(harness._workspace.path()).unwrap();
        for pass in 0..4 {
            supervisor.tick(1_150 + pass * 10).unwrap();
        }
        let unavailable = owner.current_revision().unwrap().unwrap();
        assert_eq!(unavailable.predecessor, Some(next.revision_id.clone()));
        assert!(!unavailable
            .publication()
            .unwrap()
            .batch
            .completeness
            .failures
            .is_empty());
        let cursor = assembly.graph_runtime().durable_event_cursor().unwrap();
        let cut = meld_world_model::TraversalQuery::new(assembly.stores().traversal_store.as_ref())
            .cut(&cut_request(cursor))
            .unwrap();
        assert_eq!(cut.status, TraversalCutStatus::Incomplete);
        assert!(cut.receipts.is_empty());
        std::fs::create_dir(harness._workspace.path()).unwrap();
        std::fs::write(
            harness._workspace.path().join("lib.rs"),
            "pub fn run() {}\n",
        )
        .unwrap();
        supervisor.request_shutdown(1_200).unwrap();
        drop(supervisor);
        drop(owner);
        drop(assembly);
        let reopened = harness.assembly();
        let RuntimeSemanticHandleFactory::DocsObservation(binding) = &reopened
            .handle_factories()
            .get("docs.observation")
            .unwrap()
            .semantic
        else {
            panic!("reopened Docs factory absent")
        };
        let owner = DocsObservationActor::new(binding.as_ref().clone());
        assert_eq!(owner.current_revision().unwrap(), Some(unavailable.clone()));
        harness.bind_production_routes(&reopened);
        let mut command = SupervisorStartCommand::new("docs-reopened", 2_000);
        command.registration_set = reopened.registration_set().cloned();
        let mut successor =
            RuntimeSupervisor::start(reopened.supervisor_startup_package(), command).unwrap();
        for pass in 0..4 {
            successor.tick(2_100 + pass * 10).unwrap();
        }
        let restored = owner.current_revision().unwrap().unwrap();
        assert_eq!(restored.sequence, unavailable.sequence + 1);
        assert_eq!(restored.predecessor, Some(unavailable.revision_id.clone()));
        assert_eq!(restored.evidence, next.evidence);
        assert_ne!(restored.revision_id, next.revision_id);
        let cursor = reopened.graph_runtime().durable_event_cursor().unwrap();
        let cut = meld_world_model::TraversalQuery::new(reopened.stores().traversal_store.as_ref())
            .cut(&cut_request(cursor))
            .unwrap();
        assert_eq!(cut.status, TraversalCutStatus::Complete);
        assert_eq!(cut.receipts[0].revision_id, restored.revision_id);
        successor.request_shutdown(2_200).unwrap();
        assert_eq!(
            binding.store.revision(&unavailable.revision_id).unwrap(),
            Some(unavailable)
        );
        assert_eq!(
            binding.store.revision(&first.revision_id).unwrap(),
            Some(first)
        );
    }

    #[test]
    fn native_docs_repair_returns_owner_evidence_and_satisfies_goal() {
        let provider = super::docs_fixture::ProviderServer::new();
        let harness = StewardshipHarness::new();
        std::fs::write(
            harness._workspace.path().join("lib.rs"),
            "pub fn run() {}\n",
        )
        .unwrap();
        {
            let assembly = harness.assembly();
            harness.run_world_genesis(&assembly);
        }
        let assembly = harness.assembly();
        let api = harness.bind_production_routes_with_loss(&assembly, None);
        let mut config =
            stewardship_merkle_config(harness._workspace.path(), &harness.binding.storage_root);
        config.providers.get_mut("main-provider").unwrap().endpoint = Some(provider.endpoint());
        api.provider_registry()
            .write()
            .load_from_config(&config)
            .unwrap();
        assert!(assembly.bind_production_docs_claim_judge(api));
        let mut supervisor = harness.start_supervisor(&assembly);
        assert!(assembly
            .capability_runtime()
            .unwrap()
            .catalog
            .get("docs.assess_published_scope", 1)
            .is_none());
        let mut reports = Vec::new();
        for pass in 0..60 {
            reports.push(supervisor.tick(1_000 + pass * 10).unwrap());
        }
        let readme = std::fs::read_to_string(harness._workspace.path().join("README.md"));
        assert_eq!(
            readme.as_deref().ok(),
            Some(super::docs_fixture::README),
            "calls: {:?}; reports: {reports:#?}",
            provider.calls()
        );
        let store = &assembly.stores().agent_store;
        let goals = store
            .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
            .unwrap();
        assert_eq!(goals.len(), 1);
        let plan = store
            .current_reconciliation_plan(&goals[0].goal.goal_id)
            .unwrap()
            .unwrap();
        assert!(
            store
                .goal_disposition_for_plan(&plan.plan_revision_id)
                .unwrap()
                .is_some(),
            "calls: {:?}; plan: {plan:#?}; reports: {reports:#?}",
            provider.calls()
        );
        let history = store
            .completed_history_for_goal(&goals[0].goal.goal_id)
            .unwrap();
        assert!(
            history.iter().any(|entry| matches!(
                entry.accepted_milestone,
                meld_world_model::strategy::PlanMilestoneRequirement::ExecutionTerminal { .. }
            )),
            "{history:#?}"
        );
        assert!(
            history.iter().any(|entry| matches!(
                entry.accepted_milestone,
                meld_world_model::strategy::PlanMilestoneRequirement::BeliefRevision { .. }
            )),
            "{history:#?}"
        );
        assert!(history.iter().any(|entry| matches!(
            entry.accepted_milestone,
            meld_world_model::strategy::PlanMilestoneRequirement::CurationTerminal { .. }
        )));
        let calls = provider.calls();
        assert_eq!(
            calls,
            [
                "source",
                "draft",
                "assessment",
                "assessment",
                "correspondence"
            ]
        );
        let goal_id = goals[0].goal.goal_id.clone();
        let readme_metadata =
            std::fs::metadata(harness._workspace.path().join("README.md")).unwrap();
        supervisor.request_shutdown(2_000).unwrap();
        drop(supervisor);
        drop(assembly);
        let reopened = harness.assembly();
        harness.bind_production_routes(&reopened);
        let mut command = SupervisorStartCommand::new("docs-repair-reopened", 3_000);
        command.registration_set = reopened.registration_set().cloned();
        let mut resumed =
            RuntimeSupervisor::start(reopened.supervisor_startup_package(), command).unwrap();
        for pass in 0..16 {
            resumed.tick(3_100 + pass * 10).unwrap();
        }
        assert_eq!(provider.calls(), calls);
        assert_eq!(
            reopened
                .stores()
                .agent_store
                .completed_history_for_goal(&goal_id)
                .unwrap(),
            history
        );
        assert_eq!(
            reopened
                .stores()
                .agent_store
                .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
                .unwrap()
                .len(),
            1
        );
        let after = reopened.stores().agent_store.condition_judgments().unwrap();
        assert_eq!(
            after.last().unwrap().evaluation,
            meld_lang::EvalResult::Satisfied
        );
        assert_eq!(
            std::fs::metadata(harness._workspace.path().join("README.md"))
                .unwrap()
                .modified()
                .unwrap(),
            readme_metadata.modified().unwrap()
        );
        resumed.request_shutdown(3_500).unwrap();
    }

    #[test]
    fn native_docs_changed_source_cannot_use_prior_coverage_before_curation_catches_up() {
        use meld_world_model::PlannerAssemblyOutcome;
        let harness = StewardshipHarness::new();
        std::fs::write(
            harness._workspace.path().join("lib.rs"),
            "pub fn run() {}\n",
        )
        .unwrap();
        std::fs::write(
            harness._workspace.path().join("README.md"),
            "`run` exists.\n",
        )
        .unwrap();
        {
            let assembly = harness.assembly();
            harness.run_world_genesis(&assembly);
        }
        let assembly = harness.assembly();
        harness.bind_production_routes(&assembly);
        assert!(assembly.bind_docs_claim_judge(Arc::new(
            crate::docs::claim_observation::test_support::FixtureJudge::default()
        )));
        let mut supervisor = harness.start_supervisor(&assembly);
        for pass in 0..16 {
            supervisor.tick(1_000 + pass * 10).unwrap();
        }
        let RuntimeSemanticHandleFactory::AgentActor(agent) = &assembly
            .handle_factories()
            .get(AGENT_RECONCILIATION_RUNTIME_ID)
            .unwrap()
            .semantic
        else {
            unreachable!()
        };
        let PlannerAssemblyOutcome::Complete(initial_cut) = agent.planner.assemble() else {
            panic!("initial admitted coverage missing")
        };
        let meld_world_model::agent::AgentPreparation::InstalledRule { rule, .. } =
            &agent.preparation
        else {
            unreachable!()
        };
        let curation =
            meld_world_model::CurationQuery::new(assembly.stores().curation_store.as_ref());
        let basis = curation
            .current_evidence_basis(&rule.revision_ref(), &initial_cut.traversal_cut)
            .unwrap()
            .unwrap();
        let genesis = assembly
            .stores()
            .agent_store
            .genesis_intent_for_agent(STEWARD_AGENT_ID)
            .unwrap()
            .unwrap();
        let family = genesis.required_subscriptions[0]
            .source_contract_revision
            .clone();
        let mappings: Vec<_> = genesis
            .installed_owner_revisions
            .iter()
            .filter(|revision| revision.registry == "outcome_mapping")
            .cloned()
            .collect();
        let belief =
            meld_world_model::belief::BeliefQuery::new(assembly.stores().belief_store.as_ref());
        let revision_id = &initial_cut.world_model_view.hydration_refs.revision_ids[0];
        assert!(belief
            .supports_current_curation(revision_id, &basis, &family, &mappings)
            .unwrap());
        let mut foreign_family = family.clone();
        foreign_family.content_hash = "foreign-family".into();
        assert!(!belief
            .supports_current_curation(revision_id, &basis, &foreign_family, &mappings)
            .unwrap());
        let mut foreign_mappings = mappings.clone();
        for mapping in &mut foreign_mappings {
            mapping.content_hash = "foreign-mapping".into();
        }
        assert!(!belief
            .supports_current_curation(revision_id, &basis, &family, &foreign_mappings)
            .unwrap());
        let mut foreign_rule = rule.revision_ref();
        foreign_rule.content_hash = "foreign-rule".into();
        assert!(curation
            .current_evidence_basis(&foreign_rule, &initial_cut.traversal_cut)
            .is_err());
        let meld_world_model::agent::AgentReconciliationIntent::MaintainedCondition(condition) =
            &agent.intent
        else {
            unreachable!()
        };
        let target = condition
            .condition
            .target_for(agent.authority.subject.clone())
            .unwrap();
        let judgments_before = assembly
            .stores()
            .agent_store
            .condition_judgments()
            .unwrap()
            .len();
        for (path, content, satisfied) in [
            ("lib.rs", Some("pub fn run() {}\npub fn stop() {}\n"), false),
            ("lib.rs", Some("pub fn run() {}\n"), true),
            ("README.md", None, false),
            ("README.md", Some("`run` exists.\n"), true),
        ] {
            if let Some(content) = content {
                std::fs::write(harness._workspace.path().join(path), content).unwrap();
            } else {
                std::fs::remove_file(harness._workspace.path().join(path)).unwrap();
            }
            // Hold Curation and Belief while the native observation incarnation
            // and Graph publish the changed source, in both truth directions.
            for _ in 0..8 {
                let report =
                    supervisor.step_owner_for_test("docs.observation", WorkBudget { max_items: 8 });
                assert!(report.fatal_errors.is_empty(), "{report:?}");
            }
            assembly
                .graph_runtime()
                .catch_up_bounded(GraphCatchUpBudget { max_items: 1024 })
                .unwrap();
            let RuntimeSemanticHandleFactory::DocsObservation(binding) = &assembly
                .handle_factories()
                .get("docs.observation")
                .unwrap()
                .semantic
            else {
                unreachable!()
            };
            let revision =
                crate::docs::runtime::DocsObservationActor::new(binding.as_ref().clone())
                    .current_revision()
                    .unwrap()
                    .unwrap();
            assert!(revision.correspondence.as_ref().unwrap().complete);
            let outcome = agent.planner.assemble();
            assert!(
                matches!(outcome, PlannerAssemblyOutcome::Refused(_)),
                "new source reused old coverage: {outcome:#?}"
            );
            supervisor
                .step_owner_for_test(AGENT_RECONCILIATION_RUNTIME_ID, WorkBudget { max_items: 8 });
            assert_eq!(
                assembly
                    .stores()
                    .agent_store
                    .condition_judgments()
                    .unwrap()
                    .len(),
                judgments_before
            );
            // Curation catches up, but its new result cannot borrow the old Belief.
            for _ in 0..3 {
                supervisor.step_owner_for_test(
                    "world_model.standing_curation",
                    WorkBudget { max_items: 8 },
                );
                assembly
                    .graph_runtime()
                    .catch_up_bounded(GraphCatchUpBudget { max_items: 1024 })
                    .unwrap();
            }
            let outcome = agent.planner.assemble();
            assert!(
                matches!(outcome, PlannerAssemblyOutcome::Refused(_)),
                "new coverage reused an old Belief: {outcome:#?}"
            );
            supervisor
                .step_owner_for_test(AGENT_RECONCILIATION_RUNTIME_ID, WorkBudget { max_items: 8 });
            assert_eq!(
                assembly
                    .stores()
                    .agent_store
                    .condition_judgments()
                    .unwrap()
                    .len(),
                judgments_before
            );
            for _ in 0..4 {
                supervisor.step_owner_for_test(
                    "world_model.evidence_ingestion",
                    WorkBudget { max_items: 64 },
                );
                supervisor.step_owner_for_test(
                    "world_model.belief_assessment",
                    WorkBudget { max_items: 64 },
                );
                assembly
                    .graph_runtime()
                    .catch_up_bounded(GraphCatchUpBudget { max_items: 1024 })
                    .unwrap();
            }
            let outcome = agent.planner.assemble();
            let PlannerAssemblyOutcome::Complete(cut) = outcome else {
                panic!("current native evidence did not unblock Planner: {outcome:#?}")
            };
            let evaluation = meld_lang::evaluate(&cut.world_model_view.world_state, &target);
            if satisfied {
                assert_eq!(evaluation, meld_lang::EvalResult::Satisfied);
            } else {
                assert!(
                    matches!(evaluation, meld_lang::EvalResult::Unsatisfied { .. }),
                    "{evaluation:?}"
                );
            }
            assert!(assembly
                .stores()
                .agent_store
                .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
                .unwrap()
                .is_empty());
        }
        supervisor
            .step_owner_for_test(AGENT_RECONCILIATION_RUNTIME_ID, WorkBudget { max_items: 8 });
        assert_eq!(
            assembly
                .stores()
                .agent_store
                .condition_judgments()
                .unwrap()
                .last()
                .unwrap()
                .evaluation,
            meld_lang::EvalResult::Satisfied
        );
        supervisor.request_shutdown(1_500).unwrap();
    }

    #[test]
    fn native_docs_correct_readme_requires_no_goal_or_task() {
        assert_native_docs_acceptance("`run` exists.\n", None, true);
    }

    #[test]
    fn native_docs_no_action_obeys_installed_acceptance_with_an_unsupported_extra_assertion() {
        let mut policy = crate::docs::claim_observation::test_support::policy();
        policy.minimum_groundedness = 0.4;
        policy.maximum_unsupported_claim_mass = 0.6;
        assert_native_docs_acceptance(
            "`run` exists.\n\nAn invented extra assertion.\n",
            Some(&policy),
            true,
        );
    }

    #[test]
    fn native_docs_strict_policy_requires_work_for_the_same_unsupported_extra_assertion() {
        assert_native_docs_acceptance(
            "`run` exists.\n\nAn invented extra assertion.\n",
            None,
            false,
        );
    }

    fn assert_native_docs_acceptance(
        content: &str,
        policy: Option<&crate::docs::claim_validation::DocsClaimPolicy>,
        expect_no_action: bool,
    ) {
        let harness = StewardshipHarness::new();
        std::fs::write(
            harness._workspace.path().join("lib.rs"),
            "pub fn run() {}\n",
        )
        .unwrap();
        std::fs::write(harness._workspace.path().join("README.md"), content).unwrap();
        {
            let assembly = harness.assembly();
            if let Some(policy) = policy {
                harness.run_world_genesis_with_claim_policy(&assembly, policy);
            } else {
                harness.run_world_genesis(&assembly);
            }
        }
        let assembly = harness.assembly();
        harness.bind_production_routes(&assembly);
        assert!(assembly.bind_docs_claim_judge(Arc::new(
            crate::docs::claim_observation::test_support::FixtureJudge::default()
        )));
        let mut supervisor = harness.start_supervisor(&assembly);
        for pass in 0..16 {
            supervisor.tick(1_000 + pass * 10).unwrap();
        }
        let RuntimeSemanticHandleFactory::TaskAdmission(execution) = &assembly
            .handle_factories()
            .get("execution.task_admission")
            .unwrap()
            .semantic
        else {
            panic!("Execution absent")
        };
        if !expect_no_action {
            let judgments = assembly.stores().agent_store.condition_judgments().unwrap();
            assert!(judgments.iter().any(|judgment| matches!(
                judgment.evaluation,
                meld_lang::EvalResult::Unsatisfied { .. }
            )));
            assert!(!assembly
                .stores()
                .agent_store
                .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
                .unwrap()
                .is_empty());
            assert!(!execution
                .network
                .lock()
                .unwrap()
                .state()
                .admissions
                .is_empty());
            supervisor.request_shutdown(1_400).unwrap();
            return;
        }
        assert!(execution
            .network
            .lock()
            .unwrap()
            .state()
            .admissions
            .is_empty());
        assert!(assembly
            .stores()
            .agent_store
            .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
            .unwrap()
            .is_empty());
        let judgments = assembly.stores().agent_store.condition_judgments().unwrap();
        assert!(
            judgments
                .iter()
                .any(|judgment| judgment.agent_id == STEWARD_AGENT_ID
                    && judgment.evaluation == meld_lang::EvalResult::Satisfied),
            "native admitted coverage must reach Agent, not merely leave it waiting: {judgments:?}"
        );
        assert_eq!(
            std::fs::read_to_string(harness._workspace.path().join("README.md")).unwrap(),
            content
        );
        supervisor.request_shutdown(1_400).unwrap();
        drop(supervisor);
        drop(assembly);
        let reopened = harness.assembly();
        harness.bind_production_routes(&reopened);
        let mut command = SupervisorStartCommand::new("docs-coverage-reopened", 2_000);
        command.registration_set = reopened.registration_set().cloned();
        let mut resumed =
            RuntimeSupervisor::start(reopened.supervisor_startup_package(), command).unwrap();
        for pass in 0..12 {
            resumed.tick(2_100 + pass * 10).unwrap();
        }
        assert!(reopened
            .stores()
            .agent_store
            .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
            .unwrap()
            .is_empty());
        let after = reopened.stores().agent_store.condition_judgments().unwrap();
        assert!(after.len() > judgments.len());
        assert!(after
            .iter()
            .all(|judgment| judgment.evaluation == meld_lang::EvalResult::Satisfied));
        resumed.request_shutdown(2_400).unwrap();
    }

    #[test]
    fn native_docs_curation_requires_source_claims_and_revises_coverage_from_observation() {
        use crate::docs::claim_observation::test_support::FixtureJudge;
        use crate::docs::claim_validation::*;
        use meld_world_model::world_state::graph::contracts::*;
        struct Judge(FixtureJudge);
        #[async_trait::async_trait]
        impl DocsClaimJudge for Judge {
            async fn extract_source(
                &self,
                request: &crate::docs::source_claims::DocsSourceClaimRequest<'_>,
            ) -> Result<crate::docs::source_claims::ProposedSourceClaims, crate::error::ApiError>
            {
                self.0.extract_source(request).await
            }
            async fn correspond(
                &self,
                request: &crate::docs::correspondence::DocsCorrespondenceRequest<'_>,
            ) -> Result<crate::docs::correspondence::ProposedCorrespondence, crate::error::ApiError>
            {
                self.0.correspond(request).await
            }
            async fn assess(
                &self,
                request: &DocsClaimJudgmentRequest<'_>,
            ) -> Result<Vec<ProviderClaimAssessment>, crate::error::ApiError> {
                Ok(request
                    .claims
                    .iter()
                    .map(|claim| ProviderClaimAssessment {
                        claim_id: claim.claim_id.clone(),
                        verdict: ClaimVerdict::Supported,
                        confidence: 1.0,
                        citations: vec![ClaimCitation {
                            scope: CitationScope::Direct,
                            quote: request.evidence.direct.clone(),
                        }],
                        rationale: "controlled declaration fixture".into(),
                    })
                    .collect())
            }
        }
        fn graph(
            assembly: &ProductRuntimeAssembly,
            binding: &crate::docs::runtime::DocsObservationBinding,
        ) -> TraversalResult {
            let query =
                meld_world_model::TraversalQuery::new(assembly.stores().traversal_store.as_ref());
            let cut = query
                .cut(&TraversalCutRequest {
                    owners: vec![
                        TraversalOwnerRequirement {
                            event_source: None,
                            owner_id: "docs".into(),
                            scope: binding.scope.clone(),
                            required: true,
                        },
                        TraversalOwnerRequirement {
                            event_source: None,
                            owner_id: "curation".into(),
                            scope: binding.scope.clone(),
                            required: false,
                        },
                    ],
                    scope: binding.scope.clone(),
                    currentness: OwnerCurrentnessPolicy::LatestComplete,
                    event_position: assembly.graph_runtime().durable_event_cursor().unwrap(),
                })
                .unwrap();
            assert_eq!(cut.status, TraversalCutStatus::Complete, "{cut:?}");
            let result = query
                .traverse(
                    &cut,
                    &BoundedTraversalRequest {
                        roots: vec![DomainObjectRef::new(
                            "docs",
                            "scope_observation",
                            &binding.scope.scope_id,
                        )
                        .unwrap()],
                        direction: TraversalDirection::Incoming,
                        relation_types: None,
                        bounds: TraversalBounds {
                            max_depth: 12,
                            max_objects: 4096,
                            max_occurrences: 8192,
                            max_paths: 8192,
                        },
                    },
                )
                .unwrap();
            assert!(!result.truncation.is_truncated(), "{result:?}");
            result
        }
        let harness = StewardshipHarness::new();
        std::fs::write(
            harness._workspace.path().join("lib.rs"),
            "pub fn run() {}\npub fn stop() {}\n",
        )
        .unwrap();
        std::fs::write(
            harness._workspace.path().join("README.md"),
            "`run` exists.\n",
        )
        .unwrap();
        {
            let assembly = harness.assembly();
            harness.run_world_genesis(&assembly);
        }
        let assembly = harness.assembly();
        harness.bind_production_routes(&assembly);
        assert!(assembly.bind_docs_claim_judge(Arc::new(Judge(FixtureJudge::default()))));
        let RuntimeSemanticHandleFactory::DocsObservation(binding) = &assembly
            .handle_factories()
            .get("docs.observation")
            .unwrap()
            .semantic
        else {
            panic!("Docs absent")
        };
        let mut supervisor = harness.start_supervisor(&assembly);
        for pass in 0..12 {
            supervisor.tick(1_000 + pass * 10).unwrap();
        }
        let missing = graph(&assembly, binding);
        let aggregate = missing
            .objects
            .iter()
            .find(|object| {
                object.object_ref.domain_id == "curation"
                    && object.object_ref.object_kind == "assessment"
            })
            .expect("Curation coverage aggregate");
        assert_eq!(
            aggregate.qualifications.get("coverage").map(String::as_str),
            Some("unsatisfied")
        );
        let expected = missing
            .objects
            .iter()
            .filter(|object| object.object_ref.object_kind == "expected_readme")
            .collect::<Vec<_>>();
        assert_eq!(expected.len(), 1);
        assert_eq!(
            missing
                .objects
                .iter()
                .filter(|object| object.object_ref.object_kind == "required_claim")
                .count(),
            2
        );
        assert!(missing
            .occurrences
            .iter()
            .any(|relation| relation.relation_type == "curation_requires_source_claim"));
        let expected_identity = expected[0].object_ref.clone();
        let first_publication = aggregate.publication_id.clone();
        std::fs::write(
            harness._workspace.path().join("README.md"),
            "`run` exists.\n`stop` exists.\n",
        )
        .unwrap();
        for pass in 0..12 {
            supervisor.tick(1_200 + pass * 10).unwrap();
        }
        let covered = graph(&assembly, binding);
        let aggregate = covered
            .objects
            .iter()
            .find(|object| {
                object.object_ref.domain_id == "curation"
                    && object.object_ref.object_kind == "assessment"
            })
            .unwrap();
        assert_eq!(
            aggregate.qualifications.get("coverage").map(String::as_str),
            Some("satisfied")
        );
        assert_ne!(aggregate.publication_id, first_publication);
        assert!(covered
            .objects
            .iter()
            .any(|object| object.object_ref == expected_identity
                && object
                    .qualifications
                    .get("coverage")
                    .is_some_and(|value| value == "satisfied")));
        std::fs::remove_file(harness._workspace.path().join("README.md")).unwrap();
        for pass in 0..12 {
            supervisor.tick(1_400 + pass * 10).unwrap();
        }
        let absent = graph(&assembly, binding);
        assert!(absent
            .objects
            .iter()
            .any(|object| object.object_ref == expected_identity
                && object
                    .qualifications
                    .get("coverage")
                    .is_some_and(|value| value == "unsatisfied")));
        assert!(!harness._workspace.path().join("README.md").exists());
        supervisor.request_shutdown(1_600).unwrap();
    }

    #[test]
    fn native_docs_claim_judgments_reach_graph_before_tasks_and_expire_on_source_change() {
        use crate::docs::claim_observation::{
            test_support::FixtureJudge, ObservedClaimDisposition,
        };
        use crate::docs::runtime::DocsObservationActor;
        let harness = StewardshipHarness::new();
        std::fs::write(
            harness._workspace.path().join("lib.rs"),
            "pub fn run() {}\npub fn stop() {}\n",
        )
        .unwrap();
        std::fs::write(
            harness._workspace.path().join("README.md"),
            "# run\n\n`run` exists.\n",
        )
        .unwrap();
        {
            let assembly = harness.assembly();
            harness.run_world_genesis(&assembly);
        }
        let assembly = harness.assembly();
        harness.bind_production_routes(&assembly);
        let judge = Arc::new(FixtureJudge::default());
        assert!(assembly.bind_docs_claim_judge(judge.clone()));
        let RuntimeSemanticHandleFactory::DocsObservation(binding) = &assembly
            .handle_factories()
            .get("docs.observation")
            .unwrap()
            .semantic
        else {
            panic!("Docs factory missing")
        };
        let owner = DocsObservationActor::new(binding.as_ref().clone());
        let mut supervisor = harness.start_supervisor(&assembly);
        for pass in 0..5 {
            supervisor.tick(1_000 + pass * 10).unwrap();
        }
        let assessed = owner.current_revision().unwrap().unwrap();
        let report = assessed
            .claim_report
            .as_ref()
            .expect("native pre-Task claim report");
        assert!(
            matches!(&report.readmes[0].disposition, ObservedClaimDisposition::Assessed { report } if report.accepted)
        );
        let source_report = assessed
            .source_claims
            .as_ref()
            .expect("native source claims before Task");
        assert!(source_report.complete);
        assert_eq!(source_report.files.len(), 1);
        assert!(!source_report.files[0].claims.is_empty());
        let correspondence = assessed
            .correspondence
            .as_ref()
            .expect("native correspondence before Task");
        assert!(correspondence.complete);
        let omitted = source_report.files[0]
            .claims
            .iter()
            .find(|claim| claim.statement.contains("`stop`"))
            .unwrap();
        assert!(correspondence.readmes[0]
            .claims
            .iter()
            .any(|claim| claim.source_claim_id == omitted.claim_id
                && claim.readme_claim_ids.is_empty()));
        assert!(correspondence.readmes[0]
            .claims
            .iter()
            .any(|claim| !claim.readme_claim_ids.is_empty()));
        let operation = assessed.publication().unwrap();
        assert!(operation.batch.objects.iter().any(|object| object
            .qualifications
            .get("correspondence")
            .is_some_and(|value| value == "missing")));
        assert!(operation
            .batch
            .relations
            .iter()
            .any(|relation| relation.relation_type == "docs_correspondence_match"));
        assert!(operation
            .batch
            .objects
            .iter()
            .any(|object| object.object_ref.object_kind == "observed_claim_assessment"));
        assert!(operation
            .batch
            .relations
            .iter()
            .any(|relation| relation.relation_type == "docs_judges_claim"));
        assert!(operation
            .batch
            .objects
            .iter()
            .any(|object| object.object_ref.object_kind == "claim_assessment"
                && object
                    .qualifications
                    .get("verdict")
                    .is_some_and(|verdict| verdict == "supported")));
        assert!(operation
            .batch
            .objects
            .iter()
            .any(|object| object.object_ref.object_kind == "source_claim"));
        assert!(operation
            .batch
            .relations
            .iter()
            .any(|relation| relation.relation_type == "docs_claim_from_source"));
        let query =
            meld_world_model::TraversalQuery::new(assembly.stores().traversal_store.as_ref());
        let cut = query.cut(&meld_world_model::world_state::graph::contracts::TraversalCutRequest {
            owners: vec![meld_world_model::world_state::graph::contracts::TraversalOwnerRequirement { event_source: None, owner_id: "docs".into(), scope: assessed.scope.clone(), required: true }],
            scope: assessed.scope.clone(), currentness: meld_world_model::world_state::graph::contracts::OwnerCurrentnessPolicy::LatestComplete,
            event_position: assembly.graph_runtime().durable_event_cursor().unwrap(),
        }).unwrap();
        assert_eq!(
            cut.status,
            meld_world_model::world_state::graph::contracts::TraversalCutStatus::Complete
        );
        assert_eq!(cut.receipts[0].revision_id, assessed.revision_id);
        let RuntimeSemanticHandleFactory::TaskAdmission(execution) = &assembly
            .handle_factories()
            .get("execution.task_admission")
            .unwrap()
            .semantic
        else {
            panic!("Execution absent")
        };
        assert!(execution
            .network
            .lock()
            .unwrap()
            .state()
            .admissions
            .is_empty());
        let calls = judge.calls.load(std::sync::atomic::Ordering::SeqCst);
        assert!(calls > 0);
        supervisor.tick(1_100).unwrap();
        assert_eq!(judge.calls.load(std::sync::atomic::Ordering::SeqCst), calls);
        let source_calls = judge.source_calls.load(std::sync::atomic::Ordering::SeqCst);
        std::fs::write(
            harness._workspace.path().join("README.md"),
            "# run\n\n`run` exists.\n\n",
        )
        .unwrap();
        for pass in 0..4 {
            supervisor.tick(1_101 + pass).unwrap();
        }
        let readme_changed = owner.current_revision().unwrap().unwrap();
        assert_eq!(
            readme_changed.source_claims.as_ref().unwrap().report_id,
            source_report.report_id
        );
        assert_eq!(
            judge.source_calls.load(std::sync::atomic::Ordering::SeqCst),
            source_calls
        );
        assert_ne!(
            readme_changed.correspondence.as_ref().unwrap().input_id,
            correspondence.input_id
        );
        std::fs::write(
            harness._workspace.path().join("lib.rs"),
            "pub fn replacement() {}\n",
        )
        .unwrap();
        supervisor.tick(1_110).unwrap();
        let changed = owner.current_revision().unwrap().unwrap();
        assert!(changed.claim_report.is_none());
        assert!(changed.correspondence.is_none());
        assert_eq!(changed.predecessor, Some(readme_changed.revision_id));
        supervisor.tick(1_120).unwrap();
        supervisor.tick(1_130).unwrap();
        supervisor.tick(1_140).unwrap();
        let successor = owner.current_revision().unwrap().unwrap();
        assert!(
            matches!(&successor.claim_report.as_ref().unwrap().readmes[0].disposition, ObservedClaimDisposition::Assessed { report } if !report.accepted)
        );
        assert_eq!(
            std::fs::read_to_string(harness._workspace.path().join("README.md")).unwrap(),
            "# run\n\n`run` exists.\n\n"
        );
        assert!(successor.correspondence.as_ref().unwrap().readmes[0]
            .claims
            .iter()
            .all(|claim| claim.readme_claim_ids.is_empty()));
        supervisor.request_shutdown(1_200).unwrap();
        drop(supervisor);
        drop(owner);
        drop(assembly);
        let reopened = harness.assembly();
        let RuntimeSemanticHandleFactory::DocsObservation(binding) = &reopened
            .handle_factories()
            .get("docs.observation")
            .unwrap()
            .semantic
        else {
            panic!("Docs factory missing")
        };
        let owner = DocsObservationActor::new(binding.as_ref().clone());
        assert_eq!(owner.current_revision().unwrap(), Some(successor));
        assert_eq!(
            binding.store.revision(&assessed.revision_id).unwrap(),
            Some(assessed)
        );
    }

    #[test]
    fn installed_startup_executes_confirms_and_accepts_belief_before_goal_satisfaction() {
        prove_installed_startup(StartupCallback::Normal);
    }

    #[test]
    fn startup_confirms_visible_nonce_while_execution_return_remains_uncertain() {
        prove_installed_startup(StartupCallback::Withheld);
    }

    #[test]
    fn startup_confirms_while_continuous_callback_retries_keep_producing_events() {
        prove_installed_startup(StartupCallback::ContinuousRetry);
    }

    #[test]
    fn startup_recovers_an_unreturned_effect_across_successor_epoch() {
        prove_installed_startup(StartupCallback::RestartWithheld);
    }

    #[test]
    fn startup_recovers_an_unreturned_effect_after_interruption_and_lease_expiry() {
        prove_installed_startup(StartupCallback::RestartInterrupted);
    }

    enum StartupCallback {
        Normal,
        Withheld,
        ContinuousRetry,
        RestartWithheld,
        RestartInterrupted,
    }

    fn prove_installed_startup(callback: StartupCallback) {
        let lose_callback = !matches!(callback, StartupCallback::Normal);
        let continuous_retry = matches!(callback, StartupCallback::ContinuousRetry);
        let restart_pending = matches!(
            callback,
            StartupCallback::RestartWithheld | StartupCallback::RestartInterrupted
        );
        let mut harness = StewardshipHarness::new();
        harness.binding.subject = DomainObjectRef::new("runtime", "instance", "meld").unwrap();
        harness.binding.workspace_root = None;
        harness.binding.provider_id = None;
        std::fs::remove_dir(harness._workspace.path()).unwrap();
        harness.binding.agent_id = "startup-agent".into();
        harness.binding.package = crate::config::SelectedStewardshipPackage {
            expression: "startup".into(),
            principal_id: "runtime-owner".into(),
            belief_family_id: "startup_realization".into(),
            evidence_mapping_id: "startup_realization_v1".into(),
            curation_rule_id: "startup_realization".into(),
            maintained_condition_id: "startup_realization".into(),
            strategy_theory_id: "startup_realization".into(),
            authority_policy_id: "startup_nonce_local".into(),
            claim_policy_id: String::new(),
        };
        let assembly = harness.assembly();
        harness.run_world_genesis_from(
            &assembly,
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/startup"),
        );
        let prepared = assembly
            .stores()
            .pds_products
            .prepared_head("startup")
            .unwrap()
            .unwrap();
        let prepared = assembly
            .stores()
            .pds_products
            .prepared_closure(&prepared.prepared_id)
            .unwrap()
            .unwrap();
        assert_eq!(prepared.assignment.subject, harness.binding.subject);
        assert!(!prepared.activation.bindings.contains_key("workspace"));
        assert!(!prepared.activation.bindings.contains_key("provider"));
        assert_eq!(prepared.participant_plan.participants.len(), 8);
        assert!(prepared
            .participant_plan
            .participants
            .iter()
            .all(|participant| participant.participant_id != "workspace.source"));
        let receipts = assembly
            .stores()
            .agent_store
            .genesis_receipts_for_assignment(&prepared.assignment.assignment_id)
            .unwrap();
        assert!(receipts.iter().all(|receipt| !receipt
            .installed_owner_revisions
            .iter()
            .any(|reference| reference.registry
                == meld_world_model::curation::CURATION_RULE_REGISTRY_ID)));
        let mut registry = assembly.stores().belief_family_registry.as_ref().clone();
        let mut unrelated_head = registry
            .current("startup_realization")
            .unwrap()
            .unwrap()
            .config;
        unrelated_head.dimension_id = "unselected-realization-dimension".into();
        registry.install(unrelated_head, 6).unwrap();
        drop(registry);
        drop(assembly);
        let assembly = harness.assembly();
        let loss = Arc::new(std::sync::atomic::AtomicBool::new(lose_callback));
        harness.bind_production_routes_with_loss(&assembly, Some((loss.clone(), continuous_retry)));
        let RuntimeSemanticHandleFactory::AgentActor(factory) = &assembly
            .handle_factories()
            .get(AGENT_RECONCILIATION_RUNTIME_ID)
            .unwrap()
            .semantic
        else {
            panic!("Startup Agent factory unresolved")
        };
        assert!(matches!(
            factory.preparation,
            meld_world_model::agent::AgentPreparation::Epoch { .. }
        ));
        let mut supervisor = harness.start_supervisor(&assembly);
        let mut reports = Vec::new();
        for pass in 0..40 {
            reports.push(supervisor.tick(1_100 + pass * 10).unwrap());
        }
        let watermark = harness.authority.watermark_capability().snapshot().unwrap();
        let events = harness
            .authority
            .replay_capability()
            .replay(meld_events::ReplayRequest {
                cursor: meld_events::LedgerCursor {
                    ledger_id: watermark.ledger_id,
                    after_seq: 0,
                },
                limit: 1024,
            })
            .unwrap()
            .records;
        let nonces: Vec<_> = events
            .iter()
            .filter(|record| record.event_type == crate::nonce::EVENT_TYPE)
            .collect();
        assert_eq!(
            nonces.len(),
            1,
            "Startup did not emit one nonce through production Execution: {events:#?}"
        );
        let goals = assembly
            .stores()
            .agent_store
            .reconciliation_goals_for_agent("startup-agent")
            .unwrap();
        assert_eq!(goals.len(), 1);
        let mut history = assembly
            .stores()
            .agent_store
            .completed_history_for_goal(&goals[0].goal.goal_id)
            .unwrap();
        assert!(
            history.iter().any(|entry| matches!(
                entry.accepted_milestone,
                meld_world_model::strategy::PlanMilestoneRequirement::CurationTerminal { .. }
            )),
            "Startup skipped planned confirmation: {history:#?}; reports: {reports:#?}"
        );
        assert!(
            history.iter().any(|entry| matches!(
                entry.accepted_milestone,
                meld_world_model::strategy::PlanMilestoneRequirement::BeliefRevision { .. }
            )),
            "Startup did not accept returned Belief evidence: {history:#?}"
        );
        assert!(history.iter().any(|entry| matches!(
            entry.accepted_milestone,
            meld_world_model::strategy::PlanMilestoneRequirement::GraphVisible { .. }
        )));
        let mut pending_owner = None;
        if lose_callback {
            assert!(!history.iter().any(|entry| matches!(
                entry.accepted_milestone,
                meld_world_model::strategy::PlanMilestoneRequirement::ExecutionTerminal { .. }
            )));
            let RuntimeSemanticHandleFactory::TaskAdmission(execution) = &assembly
                .handle_factories()
                .get("execution.task_admission")
                .unwrap()
                .semantic
            else {
                unreachable!()
            };
            let network = execution.network.lock().unwrap();
            assert!(!network.state().claims.is_empty());
            assert!(network.state().outcomes.is_empty());
            drop(network);
            let native_owner = AgentReconciliationActor::new(
                factory.runtime_id.clone(),
                factory.intent.clone(),
                factory.store.clone(),
                factory.planner.clone(),
                factory.authority_port.clone(),
                factory.frozen_authority.clone(),
                factory.curation.clone(),
                factory.execution.clone(),
                factory.strategy.clone(),
                factory.authority.clone(),
                factory.preparation.clone(),
            )
            .unwrap();
            let pending: Vec<_> = assembly
                .stores()
                .agent_store
                .product_authorizations_for_goal(&goals[0].goal.goal_id)
                .unwrap()
                .into_iter()
                .filter(|authorization| {
                    matches!(
                        authorization.product,
                        meld_world_model::agent::AgentAuthorizedProduct::Task(_)
                    )
                })
                .collect();
            assert_eq!(pending.len(), 1);
            let pending_evidence = native_owner
                .lifecycle_evidence()
                .unwrap()
                .unresolved_operation_summary_ref;
            pending_owner = Some((native_owner, pending_evidence));
        }
        let plan = assembly
            .stores()
            .agent_store
            .current_reconciliation_plan(&goals[0].goal.goal_id)
            .unwrap()
            .unwrap();
        assert!(
            assembly
                .stores()
                .agent_store
                .goal_disposition_for_plan(&plan.plan_revision_id)
                .unwrap()
                .is_some(),
            "Startup did not separately satisfy its Goal: {plan:#?}"
        );
        let disposition = assembly
            .stores()
            .agent_store
            .goal_disposition_for_plan(&plan.plan_revision_id)
            .unwrap()
            .unwrap();
        let milestones = assembly
            .stores()
            .agent_store
            .milestones_for_goal(&goals[0].goal.goal_id)
            .unwrap();
        assert!(milestones.iter().any(|milestone| matches!(
            milestone.requirement,
            meld_world_model::strategy::PlanMilestoneRequirement::BeliefRevision { .. }
        ) && disposition
            .accepted_milestone_ids
            .contains(&milestone.milestone_id)));
        let returned = history
            .iter()
            .find(|entry| {
                matches!(
                    entry.accepted_milestone,
                    meld_world_model::strategy::PlanMilestoneRequirement::BeliefRevision { .. }
                )
            })
            .unwrap();
        let meld_world_model::strategy::StrategyProduct::Epistemic(operation) =
            returned.product.as_ref().unwrap()
        else {
            unreachable!()
        };
        let result = assembly
            .stores()
            .curation_store
            .result_for_operation(&operation.operation.operation_id)
            .unwrap()
            .unwrap();
        let products = assembly
            .stores()
            .agent_store
            .epoch_products(&goals[0].goal.goal_id)
            .unwrap()
            .unwrap();
        let request = meld_world_model::belief::BeliefEvidenceReturnRequest {
            subscription: products.subscription_requests().unwrap().remove(0),
            revision_ids: vec![returned.owner_position_id.clone()],
            publication_record_id: result.event_record_id(),
            evidence_schema_id: operation
                .return_evidence
                .as_ref()
                .unwrap()
                .evidence_schema_id
                .clone(),
            mapping_revisions: products
                .specification
                .genesis
                .installed_owner_revisions
                .iter()
                .filter(|reference| reference.registry == "outcome_mapping")
                .cloned()
                .collect(),
        };
        let meld_world_model::agent::AgentPreparation::Epoch { subscriptions, .. } =
            &factory.preparation
        else {
            unreachable!()
        };
        assert!(subscriptions.returned_evidence(&request).unwrap().is_some());
        let mut foreign = request.clone();
        foreign.publication_record_id = "curation-result::foreign-epoch".into();
        assert!(subscriptions.returned_evidence(&foreign).unwrap().is_none());
        foreign = request.clone();
        foreign.mapping_revisions[0].content_hash = "foreign-mapping-revision".into();
        assert!(subscriptions.returned_evidence(&foreign).unwrap().is_none());
        foreign = request.clone();
        foreign.evidence_schema_id = "foreign-schema".into();
        assert!(subscriptions.returned_evidence(&foreign).unwrap().is_none());
        if lose_callback && !restart_pending {
            loss.store(false, std::sync::atomic::Ordering::SeqCst);
            for pass in 0..15 {
                supervisor.tick(1_600 + pass * 10).unwrap();
            }
            history = assembly
                .stores()
                .agent_store
                .completed_history_for_goal(&goals[0].goal.goal_id)
                .unwrap();
            assert!(
                history.iter().any(|entry| matches!(
                    entry.accepted_milestone,
                    meld_world_model::strategy::PlanMilestoneRequirement::ExecutionTerminal { .. }
                )),
                "late Execution return was abandoned: {history:#?}"
            );
            let (native_owner, pending_evidence) = pending_owner.take().unwrap();
            assert_ne!(
                native_owner
                    .lifecycle_evidence()
                    .unwrap()
                    .unresolved_operation_summary_ref,
                pending_evidence,
                "Graph visibility had hidden the outstanding Task from native lifecycle evidence"
            );
            let watermark = harness.authority.watermark_capability().snapshot().unwrap();
            let events = harness
                .authority
                .replay_capability()
                .replay(meld_events::ReplayRequest {
                    cursor: meld_events::LedgerCursor {
                        ledger_id: watermark.ledger_id,
                        after_seq: 0,
                    },
                    limit: 1024,
                })
                .unwrap()
                .records;
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.event_type == crate::nonce::EVENT_TYPE)
                    .count(),
                1
            );
        }
        for pass in 0..5 {
            supervisor.tick(2_000 + pass * 10).unwrap();
        }
        assert_eq!(
            assembly
                .stores()
                .agent_store
                .completed_history_for_goal(&goals[0].goal.goal_id)
                .unwrap(),
            history
        );
        assert_eq!(
            assembly
                .stores()
                .agent_store
                .goal_disposition_for_plan(&plan.plan_revision_id)
                .unwrap(),
            Some(disposition.clone())
        );
        if !matches!(callback, StartupCallback::RestartInterrupted) {
            supervisor.request_shutdown(2_100).unwrap();
        }
        drop(pending_owner);
        drop(supervisor);
        drop(assembly);
        let reopened = harness.assembly();
        assert_eq!(
            reopened
                .stores()
                .agent_store
                .completed_history_for_goal(&goals[0].goal.goal_id)
                .unwrap(),
            history
        );
        assert_eq!(
            reopened
                .stores()
                .agent_store
                .goal_disposition_for_plan(&plan.plan_revision_id)
                .unwrap(),
            Some(disposition.clone())
        );
        assert_eq!(
            reopened
                .stores()
                .agent_store
                .epoch_products(&goals[0].goal.goal_id)
                .unwrap(),
            Some(products.clone())
        );
        if !lose_callback || restart_pending {
            harness.bind_production_routes(&reopened);
            let mut command = SupervisorStartCommand::new("startup-successor", 1_000_000);
            command.registration_set = reopened.registration_set().cloned();
            let mut successor =
                RuntimeSupervisor::start(reopened.supervisor_startup_package(), command).unwrap();
            let mut successor_reports = Vec::new();
            for pass in 0..50 {
                successor_reports.push(successor.tick(1_000_100 + pass * 10).unwrap());
            }
            let successor_goals = reopened
                .stores()
                .agent_store
                .reconciliation_goals_for_agent("startup-agent")
                .unwrap();
            assert_eq!(successor_goals.len(), 2, "{successor_reports:#?}");
            let next = successor_goals
                .iter()
                .find(|goal| goal.goal.goal_id != goals[0].goal.goal_id)
                .unwrap();
            let next_products = reopened
                .stores()
                .agent_store
                .epoch_products(&next.goal.goal_id)
                .unwrap()
                .unwrap();
            assert_ne!(
                next_products.specification.fence.admission_epoch,
                products.specification.fence.admission_epoch
            );
            assert_ne!(
                next_products.observation_subject,
                products.observation_subject
            );
            assert_ne!(next_products.effect_visibility, products.effect_visibility);
            let next_history = reopened
                .stores()
                .agent_store
                .completed_history_for_goal(&next.goal.goal_id)
                .unwrap();
            assert!(
                next_history.iter().any(|entry| matches!(
                    entry.accepted_milestone,
                    meld_world_model::strategy::PlanMilestoneRequirement::GraphVisible { .. }
                )),
                "{successor_reports:#?}"
            );
            assert!(
                next_history.iter().any(|entry| matches!(
                    entry.accepted_milestone,
                    meld_world_model::strategy::PlanMilestoneRequirement::BeliefRevision { .. }
                )),
                "{successor_reports:#?}"
            );
            let next_plan = reopened
                .stores()
                .agent_store
                .current_reconciliation_plan(&next.goal.goal_id)
                .unwrap()
                .unwrap();
            let next_disposition = reopened
                .stores()
                .agent_store
                .goal_disposition_for_plan(&next_plan.plan_revision_id)
                .unwrap()
                .unwrap_or_else(|| panic!("{successor_reports:#?}"));
            assert!(next_history.iter().any(|entry| matches!(
                entry.accepted_milestone,
                meld_world_model::strategy::PlanMilestoneRequirement::ExecutionTerminal { .. }
            )));
            assert!(next_history.iter().any(|entry| matches!(
                entry.accepted_milestone,
                meld_world_model::strategy::PlanMilestoneRequirement::CurationTerminal { .. }
            )));
            assert!(next_history.iter().all(|entry| history
                .iter()
                .all(|prior| prior.product_id != entry.product_id)));
            for entry in next_history.iter().filter(|entry| {
                matches!(
                    entry.accepted_milestone,
                    meld_world_model::strategy::PlanMilestoneRequirement::BeliefRevision { .. }
                )
            }) {
                let next_milestones = reopened
                    .stores()
                    .agent_store
                    .milestones_for_goal(&next.goal.goal_id)
                    .unwrap();
                assert!(next_milestones.iter().any(|milestone| milestone.requirement
                    == entry.accepted_milestone
                    && next_disposition
                        .accepted_milestone_ids
                        .contains(&milestone.milestone_id)));
                assert!(history
                    .iter()
                    .all(|prior| prior.owner_position_id != entry.owner_position_id));
            }
            let watermark = harness.authority.watermark_capability().snapshot().unwrap();
            let events = harness
                .authority
                .replay_capability()
                .replay(meld_events::ReplayRequest {
                    cursor: meld_events::LedgerCursor {
                        ledger_id: watermark.ledger_id,
                        after_seq: 0,
                    },
                    limit: 1024,
                })
                .unwrap()
                .records;
            assert_eq!(
                events
                    .iter()
                    .filter(|event| event.event_type == crate::nonce::EVENT_TYPE)
                    .count(),
                2
            );
            let prior_history = reopened
                .stores()
                .agent_store
                .completed_history_for_goal(&goals[0].goal.goal_id)
                .unwrap();
            if restart_pending {
                assert!(prior_history.iter().any(|entry| matches!(
                    entry.accepted_milestone,
                    meld_world_model::strategy::PlanMilestoneRequirement::ExecutionTerminal { .. }
                )), "original Task return was lost across restart: {successor_reports:#?}");
                assert!(history.iter().all(|entry| prior_history.contains(entry)));
            } else {
                assert_eq!(prior_history, history);
            }
            assert_eq!(
                reopened
                    .stores()
                    .agent_store
                    .goal_disposition_for_plan(&plan.plan_revision_id)
                    .unwrap(),
                Some(disposition.clone())
            );
            successor.request_shutdown(1_001_000).unwrap();
            drop(successor);
            drop(reopened);
            let retained = harness.assembly();
            for (
                goal_id,
                retained_history,
                retained_plan,
                retained_disposition,
                retained_products,
            ) in [
                (
                    &goals[0].goal.goal_id,
                    &prior_history,
                    &plan,
                    &disposition,
                    &products,
                ),
                (
                    &next.goal.goal_id,
                    &next_history,
                    &next_plan,
                    &next_disposition,
                    &next_products,
                ),
            ] {
                assert_eq!(
                    retained
                        .stores()
                        .agent_store
                        .completed_history_for_goal(goal_id)
                        .unwrap(),
                    *retained_history
                );
                assert_eq!(
                    retained
                        .stores()
                        .agent_store
                        .goal_disposition_for_plan(&retained_plan.plan_revision_id)
                        .unwrap()
                        .as_ref(),
                    Some(retained_disposition)
                );
                assert_eq!(
                    retained
                        .stores()
                        .agent_store
                        .epoch_products(goal_id)
                        .unwrap()
                        .as_ref(),
                    Some(retained_products)
                );
            }
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

    #[test]
    fn product_genesis_requires_its_current_compilation_head() {
        let harness = StewardshipHarness::new();
        let assembly = harness.assembly();
        let stores = assembly.stores();
        let package_receipt = crate::docs::theory::install_package(
            stores,
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/docs_freshness"),
            5,
        )
        .unwrap();
        let product = crate::init::world::tooling::compile_product_initialization(
            stores,
            &harness.binding,
            &package_receipt,
            5,
        )
        .unwrap();
        let mut registry = stores.belief_family_registry.as_ref().clone();
        let append = assembly.event_authority().append_capability();
        let error = crate::init::world::pipeline::WorldInitPipeline::new(
            &mut registry,
            stores.belief_store.as_ref(),
            stores.agent_store.as_ref(),
            &append,
        )
        .with_complete_product(product)
        .run(
            &crate::init::world::WorldInitRequest {
                stages: vec![crate::init::world::WorldInitStage::GenesisIdentities],
            },
            &harness.world_init_metadata(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("installed current compilation"));
        assert!(stores
            .agent_store
            .get_agent(STEWARD_AGENT_ID)
            .unwrap()
            .is_none());
    }

    #[test]
    fn prepared_plan_projects_the_exact_required_runtime_set() {
        let harness = StewardshipHarness::new();
        {
            let assembly = harness.assembly();
            harness.run_world_genesis(&assembly);
        }
        let assembly = harness.assembly();

        let set = assembly.registration_set().unwrap();

        assert_eq!(set.registrations.len(), 10);
        assert_eq!(
            set.kind_of("workspace.source"),
            Some(RegistrationKind::PassiveService)
        );
        for active in [
            "docs.observation",
            "world_model.graph_replay",
            "world_model.belief_assessment",
            "world_model.evidence_ingestion",
            "world_model.standing_curation",
            AGENT_RECONCILIATION_RUNTIME_ID,
            "execution.task_admission",
            "execution.task_dispatch",
            "execution.publication",
        ] {
            assert_eq!(
                set.kind_of(active),
                Some(RegistrationKind::ActiveActor),
                "'{active}' must be an active actor"
            );
        }
        assert!(set
            .registrations
            .iter()
            .all(|registration| registration.registration_id.starts_with("prepared::")));
    }

    #[test]
    fn ungenesised_stewardship_boot_is_truthful_and_writes_no_semantic_state() {
        let harness = StewardshipHarness::new();
        let assembly = harness.assembly();
        let registration_set = assembly.registration_set().cloned().unwrap();

        let mut command = SupervisorStartCommand::new("instance-a", 100);
        command.registration_set = Some(registration_set);
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        let tick = supervisor.tick(1_100).unwrap();
        let status = supervisor.status_snapshot(1_100).unwrap();

        // Without an exact prepared closure there is no inferred participant
        // set and therefore no actor lifecycle to misreport as current.
        assert!(status.runtimes.is_empty());
        assert!(tick.actions.is_empty());

        // Boot and tick created no semantic state anywhere.
        assert!(assembly
            .ports()
            .event_replay()
            .read_after_limit(0, 10)
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
    fn routed_curation_template_drives_native_owners_from_exact_genesis_after_reopen() {
        use meld_world_model::agent::AgentActivationRecord;
        use meld_world_model::curation::{CurationRuleTemplate, CURATION_RULE_REGISTRY_ID};
        let harness = StewardshipHarness::new();
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/docs_freshness");
        let package_root = tempfile::tempdir().unwrap();
        for entry in std::fs::read_dir(&source).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                std::fs::copy(entry.path(), package_root.path().join(entry.file_name())).unwrap();
            }
        }
        let subject = stewardship_subject_ref(&harness.binding).unwrap();
        let legacy = standing_curation_rule(subject.clone());
        let template = CurationRuleTemplate {
            coverage: None,
            rule_id: "docs-native-epistemic-rule".into(),
            source_owner_id: "workspace_fs".into(),
            traversal_direction: legacy.traversal_direction,
            bounds: legacy.bounds,
            expected_object_kind: "assessment".into(),
            expected_object_key: "observed-condition".into(),
            relation_type: "curation_assesses".into(),
            output_policy_revision: "docs-epistemic-v1".into(),
            realization: None,
        };
        let value = serde_json::to_value(template).unwrap();
        let bytes = serde_json::to_vec(&value).unwrap();
        std::fs::write(package_root.path().join("epistemic-rule.json"), &bytes).unwrap();
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(source.join("pds-package.json")).unwrap())
                .unwrap();
        manifest["components"]
            .as_array_mut()
            .unwrap()
            .retain(|component| component["component_id"] != "docs-epistemic-rule");
        manifest["components"].as_array_mut().unwrap().push(serde_json::json!({
            "component_id": "docs-native-curation", "owner_component_id": "docs-native-epistemic-rule",
            "route": { "owner_domain": "world-model", "component_kind": "epistemic-curation-rule", "route_version": 1 },
            "component_schema_version": 1,
            "content": { "kind": "relative_file", "path": "epistemic-rule.json", "content_hash": blake3::hash(&bytes).to_hex().to_string() },
            "requires": []
        }));
        std::fs::write(
            package_root.path().join("pds-package.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let expected;
        {
            let assembly = harness.assembly();
            harness.run_world_genesis_from(&assembly, package_root.path());
            let prepared = assembly
                .stores()
                .pds_products
                .prepared_head(&harness.binding.package.expression)
                .unwrap()
                .unwrap();
            let closure = assembly
                .stores()
                .pds_products
                .prepared_closure(&prepared.prepared_id)
                .unwrap()
                .unwrap();
            let receipts = assembly
                .stores()
                .agent_store
                .genesis_receipts_for_assignment(&closure.assignment.assignment_id)
                .unwrap();
            expected = receipts[0]
                .installed_owner_revisions
                .iter()
                .find(|reference| reference.registry == CURATION_RULE_REGISTRY_ID)
                .unwrap()
                .clone();
            assert!(assembly
                .stores()
                .curation_store
                .active_rule(STEWARD_AGENT_ID)
                .unwrap()
                .is_none());
            // An unrelated mutable head cannot replace what this product prepared.
            assembly
                .stores()
                .curation_store
                .install_rule(standing_curation_rule(subject.clone()), 99)
                .unwrap();
            assembly
                .stores()
                .agent_store
                .put_activation(&AgentActivationRecord {
                    activation_id: closure.activation.activation_id,
                    agent_id: STEWARD_AGENT_ID.into(),
                    started_at_seq: 6,
                    status: meld_world_model::AgentActivationStatus::Activated,
                    last_error: None,
                    lease_id: Some("native-template-activation".into()),
                })
                .unwrap();
            assembly
                .ports()
                .event_append()
                .append_envelope_idempotent(workspace_owner_publication(
                    subject,
                    "native-template-source-v1",
                ))
                .unwrap();
            assembly.flush_product_boundary().unwrap();
        }
        let assembly = harness.assembly();
        let _supervisor = harness.start_supervisor(&assembly);
        let mut graph = assembly
            .handle_factories()
            .get("world_model.graph_replay")
            .unwrap()
            .build_handle();
        graph
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "world_model.graph_replay".into(),
                lease_id: "template-graph".into(),
            })
            .unwrap();
        graph.tick(WorkBudget { max_items: 64 }).unwrap();
        let mut curation = assembly
            .handle_factories()
            .get("world_model.standing_curation")
            .unwrap()
            .build_handle();
        curation
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "world_model.standing_curation".into(),
                lease_id: "template-curation".into(),
            })
            .unwrap();
        let tick = curation.tick(WorkBudget { max_items: 8 }).unwrap();
        assert!(tick.fatal_errors.is_empty(), "{tick:?}");
        assert!(tick.items_committed > 0, "{tick:?}");
        let records = assembly
            .ports()
            .event_replay()
            .read_after_limit(0, 128)
            .unwrap();
        let result: meld_world_model::CurationResult = serde_json::from_value(
            records
                .iter()
                .find(|record| {
                    record.envelope.event_type == meld_world_model::CURATION_RESULT_EVENT_TYPE
                })
                .unwrap()
                .envelope
                .data
                .clone(),
        )
        .unwrap();
        let publication = result.semantic_publication.unwrap();
        assert!(publication
            .batch
            .objects
            .iter()
            .all(
                |object| object.qualifications.get("rule_revision") == Some(&expected.content_hash)
            ));
        graph.tick(WorkBudget { max_items: 64 }).unwrap();
        let mut belief = assembly
            .handle_factories()
            .get("world_model.belief_assessment")
            .unwrap()
            .build_handle();
        belief
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "world_model.belief_assessment".into(),
                lease_id: "template-belief".into(),
            })
            .unwrap();
        let belief_tick = belief.tick(WorkBudget { max_items: 8 }).unwrap();
        assert!(belief_tick.fatal_errors.is_empty(), "{belief_tick:?}");
        graph.tick(WorkBudget { max_items: 64 }).unwrap();
        let factory = assembly
            .handle_factories()
            .get(AGENT_RECONCILIATION_RUNTIME_ID)
            .unwrap();
        let RuntimeSemanticHandleFactory::AgentActor(native_agent) = &factory.semantic else {
            panic!("Agent factory unresolved");
        };
        let meld_world_model::agent::AgentPreparation::InstalledRule { rule, .. } =
            &native_agent.preparation
        else {
            panic!("static observation must retain its exact prepared rule")
        };
        assert_eq!(rule.revision_ref(), expected);
        let mut agent = assembly
            .handle_factories()
            .get(AGENT_RECONCILIATION_RUNTIME_ID)
            .unwrap()
            .build_handle();
        agent
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: AGENT_RECONCILIATION_RUNTIME_ID.into(),
                lease_id: "template-agent".into(),
            })
            .unwrap();
        let tick = agent.tick(WorkBudget { max_items: 8 }).unwrap();
        assert!(tick.fatal_errors.is_empty(), "{tick:?}");
        assert!(tick.items_committed > 0, "{tick:?}");
        assembly.flush_product_boundary().unwrap();
    }

    #[test]
    fn prepared_genesis_starts_without_legacy_activation_and_fences_each_epoch() {
        use meld_execution::task_network::dispatch_actor::AdmissionGenerationObserver;
        let harness = StewardshipHarness::new();
        {
            let assembly = harness.assembly();
            harness.run_world_genesis(&assembly);
            assembly.flush_product_boundary().unwrap();
        }
        let assembly = harness.assembly();
        let prepared = assembly.prepared_activation().unwrap();
        let store = assembly.stores().agent_store.opened().unwrap().clone();
        assert!(store
            .activations_for_agent(STEWARD_AGENT_ID)
            .unwrap()
            .is_empty());
        let observer = ProductAdmissionGenerationObserver::new(
            store,
            assembly.lifecycle_store().unwrap().clone(),
            prepared,
        );
        let RuntimeSemanticHandleFactory::AgentActor(agent) = &assembly
            .handle_factories()
            .get(AGENT_RECONCILIATION_RUNTIME_ID)
            .unwrap()
            .semantic
        else {
            panic!("prepared Agent must construct");
        };
        let RuntimeSemanticHandleFactory::StandingCuration(curation) = &assembly
            .handle_factories()
            .get("world_model.standing_curation")
            .unwrap()
            .semantic
        else {
            panic!("prepared Curation must construct");
        };
        assert!(agent.authority_port.observe().unwrap().is_none());
        assert!(curation.authority_port.observe().unwrap().is_none());
        assert!(observer
            .active_generation(STEWARD_AGENT_ID)
            .unwrap()
            .is_none());
        let _supervisor = harness.start_supervisor(&assembly);
        let first = agent.authority_port.observe().unwrap().unwrap();
        let curation_first = curation.authority_port.observe().unwrap().unwrap();
        assert_eq!(
            first.activation_generation,
            curation_first.activation_generation
        );
        assert_eq!(first.admission_epoch, curation_first.admission_epoch);
        assert_ne!(
            first.activation_generation,
            prepared.activation.activation_id
        );
        assert!(first.admission_epoch.is_some());
        let mut attribution = meld_execution::task_network::TaskAdmissionAttribution {
            agent_id: STEWARD_AGENT_ID.into(),
            goal_id: "epoch-test-goal".into(),
            plan_revision_id: "epoch-test-plan".into(),
            task_id: "epoch-test-task".into(),
            authorization_id: "epoch-test-authorization".into(),
            admission_id: "epoch-test-admission".into(),
            authority_scope_id: "epoch-test-policy".into(),
            authority_policy_content_hash: first.authority_policy_content_hash.clone(),
            activation_generation: first.activation_generation.clone(),
            admission_epoch: first.admission_epoch.clone(),
        };
        assert!(observer.validates_admission(&attribution).unwrap());
        attribution.agent_id = "foreign-agent".into();
        assert!(!observer.validates_admission(&attribution).unwrap());
        attribution.agent_id = STEWARD_AGENT_ID.into();
        let lifecycle = assembly.lifecycle_store().unwrap();
        lifecycle
            .interrupt(
                &prepared.assignment.assignment_id,
                &first.activation_generation,
            )
            .unwrap();
        assert!(agent.authority_port.observe().unwrap().is_none());
        assert!(curation.authority_port.observe().unwrap().is_none());
        assert!(!observer.validates_admission(&attribution).unwrap());
        let second = lifecycle
            .reopen(
                &prepared.assignment.assignment_id,
                &first.activation_generation,
                prepared,
            )
            .unwrap();
        assert_ne!(Some(&second.epoch_id), first.admission_epoch.as_ref());
        assert!(!observer.validates_admission(&attribution).unwrap());
        let reopened = agent.authority_port.observe().unwrap().unwrap();
        assert_eq!(reopened.activation_generation, first.activation_generation);
        assert_eq!(reopened.admission_epoch.as_ref(), Some(&second.epoch_id));
        attribution.admission_epoch = Some(second.epoch_id);
        assert!(observer.validates_admission(&attribution).unwrap());
        attribution.admission_epoch = None;
        assert!(!observer.validates_admission(&attribution).unwrap());
        assert!(assembly
            .stores()
            .agent_store
            .activations_for_agent(STEWARD_AGENT_ID)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn genesised_world_binds_epistemic_actors_and_ticks_each_exactly_once() {
        let harness = StewardshipHarness::new();
        {
            let assembly = harness.assembly();
            harness.run_world_genesis(&assembly);
            assert!(assembly
                .stores()
                .curation_store
                .active_rule(STEWARD_AGENT_ID)
                .unwrap()
                .is_none());
            assert!(assembly
                .stores()
                .agent_store
                .activations_for_agent(STEWARD_AGENT_ID)
                .unwrap()
                .is_empty());
            assembly.flush_product_boundary().unwrap();
        }

        let assembly = harness.assembly();
        assert!(assembly.bind_dispatch_routes(stub_routes()));
        let registration_set = assembly.registration_set().cloned().unwrap();
        let mut command = SupervisorStartCommand::new("instance-b", 100);
        command.registration_set = Some(registration_set);
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        let tick = supervisor.tick(1_100).unwrap();
        let status = supervisor.status_snapshot(1_100).unwrap();
        assert_eq!(
            status
                .activation_liveness
                .as_ref()
                .expect("prepared activation must expose assignment liveness")
                .state,
            crate::runtime::lifecycle::ProjectedLifecycleState::ActiveWork,
            "{:?}",
            status.activation_liveness
        );

        let bound = [
            "docs.observation",
            "world_model.graph_replay",
            "world_model.belief_assessment",
            "world_model.evidence_ingestion",
            "world_model.standing_curation",
            AGENT_RECONCILIATION_RUNTIME_ID,
            "execution.task_admission",
            "execution.task_dispatch",
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
        // Exact prepared Capability bindings make Task admission available.
        assert_eq!(
            lifecycle_of(&status, "execution.task_admission"),
            Some(RegistrationLifecycle::ActiveIdle)
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
        let mut quiescent = false;
        for pass in 2..=24 {
            supervisor.tick(1_100 + pass * 10).unwrap();
            let projected = supervisor
                .status_snapshot(1_101 + pass * 10)
                .unwrap()
                .activation_liveness
                .expect("prepared activation must retain assignment liveness");
            if projected.state == crate::runtime::lifecycle::ProjectedLifecycleState::Quiescent {
                assert!(projected.incomplete_participants.is_empty());
                assert!(projected.broken_wake_refs.is_empty());
                quiescent = true;
                break;
            }
        }
        assert!(quiescent, "native waits must reach resolved quiescence");
        let lifecycle_store = assembly.lifecycle_store().unwrap();
        let assignment_id = assembly
            .prepared_activation()
            .unwrap()
            .assignment
            .assignment_id
            .clone();
        let current = lifecycle_store
            .current_generation(&assignment_id)
            .unwrap()
            .unwrap();
        assert!(current.admission_open());
        assert_eq!(current.readiness.len(), 10);
        assert!(current.readiness.values().all(|receipt| {
            !receipt.native_evidence.installed_revision_refs.is_empty()
                && !receipt.native_evidence.binding_refs.is_empty()
                && !receipt.native_evidence.subscription_refs.is_empty()
                && !receipt.native_evidence.proof_position_ref.is_empty()
        }));
        let wake_refs = current
            .waits
            .values()
            .flat_map(|wait| wait.wake_refs.iter())
            .collect::<Vec<_>>();
        assert!(wake_refs.iter().any(|wake| matches!(
            wake,
            crate::runtime::lifecycle::StructuralWakeRef::EventPosition(_)
        )));
        assert!(wake_refs.iter().any(|wake| matches!(
            wake,
            crate::runtime::lifecycle::StructuralWakeRef::OwnerRevision(_)
        )));
        assert!(wake_refs.iter().any(|wake| matches!(
            wake,
            crate::runtime::lifecycle::StructuralWakeRef::DurableOperation(_)
        )));
        let generation_id = current.generation_id.clone();
        let (participant_id, original_wait) = current.waits.iter().next().unwrap();
        for foreign in [
            StructuralWakeRef::EventPosition(format!(
                "event-ledger::{}::after::0",
                meld_events::LedgerIdentity::new()
            )),
            StructuralWakeRef::OwnerRevision("task-network::foreign-network::after::0".into()),
            StructuralWakeRef::DurableOperation(
                "publication-outbox::foreign-network::after::0".into(),
            ),
            StructuralWakeRef::OwnerRevision(format!(
                "world-model::{}::belief-dirty-work::world_model.belief_assessment::after::0",
                meld_events::LedgerIdentity::new()
            )),
        ] {
            lifecycle_store
                .record_wait(
                    &assignment_id,
                    &generation_id,
                    participant_id,
                    OwnerWaitReceiptV1::new(
                        generation_id.clone(),
                        original_wait.incarnation_id.clone(),
                        original_wait.owner_checkpoint_ref.clone(),
                        "adversarial-foreign-resource".into(),
                        vec![foreign.clone()],
                    )
                    .unwrap(),
                )
                .unwrap();
            let projection = supervisor
                .status_snapshot(1_900)
                .unwrap()
                .activation_liveness
                .unwrap();
            assert_eq!(
                projection.state,
                crate::runtime::lifecycle::ProjectedLifecycleState::Stalled,
                "{projection:?}"
            );
            assert_eq!(projection.broken_wake_refs, vec![foreign]);
        }
        lifecycle_store
            .record_wait(
                &assignment_id,
                &generation_id,
                participant_id,
                original_wait.clone(),
            )
            .unwrap();
        assert_eq!(
            supervisor
                .status_snapshot(1_901)
                .unwrap()
                .activation_liveness
                .unwrap()
                .state,
            crate::runtime::lifecycle::ProjectedLifecycleState::Quiescent
        );
        supervisor.request_shutdown(2_000).unwrap();
        let retired_status = supervisor.status_snapshot(2_001).unwrap();
        assert_eq!(
            retired_status
                .activation_liveness
                .as_ref()
                .expect("retired activation remains visible")
                .state,
            crate::runtime::lifecycle::ProjectedLifecycleState::Retired
        );
        let lifecycle = lifecycle_store.assignment(&assignment_id).unwrap().unwrap();
        assert!(lifecycle.current_generation_id.is_none());
        assert!(lifecycle
            .generations
            .values()
            .all(|generation| generation.status
                == crate::runtime::lifecycle::ActivationGenerationStatus::Retired));
        let retired = &lifecycle.generations[&generation_id];
        assert_eq!(retired.stop_receipts.len(), 10);
        assert_eq!(retired.release_receipts.len(), 10);
        assert_eq!(retired.safe_points.len(), 9);
        assert_eq!(retired.passive_fences.len(), 1);
        assert!(retired
            .retirement
            .as_ref()
            .is_some_and(
                |receipt| receipt.stop_receipts.len() == 10 && receipt.release_receipts.len() == 10
            ));
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
            let assembly = harness.assembly();
            harness.run_workspace_fixture_genesis(&assembly);
            assembly
                .stores()
                .agent_store
                .put_activation(&AgentActivationRecord {
                    activation_id: harness.prepared_activation_id(&assembly),
                    agent_id: STEWARD_AGENT_ID.to_string(),
                    started_at_seq: 5,
                    status: meld_world_model::AgentActivationStatus::Activated,
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

        let assembly = harness.assembly();
        let _supervisor = harness.start_supervisor(&assembly);
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
                    event_source: None,
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
                        event_source: None,
                        owner_id: "workspace_fs".to_string(),
                        scope: standing_curation_scope(),
                        required: true,
                    },
                    TraversalOwnerRequirement {
                        event_source: None,
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

        let family = assembly
            .stores()
            .belief_family_registry
            .current(FAMILY_ID)
            .unwrap()
            .unwrap();
        let mapping = Arc::new(
            ConfiguredOutcomeMappingSet::new(standing_curation_outcome_mapping()).unwrap(),
        );
        let mut ingestion = EvidenceIngestionActor::new(
            "world_model.evidence_ingestion",
            Arc::clone(&assembly.stores().belief_store),
            Arc::clone(&assembly.stores().traversal_store),
            Arc::clone(&assembly.stores().belief_family_registry)
                as Arc<dyn BeliefFamilyRegistry + Send + Sync>,
            FAMILY_ID,
            Arc::new(assembly.ports().event_replay().clone()) as Arc<dyn EvidenceEventReplaySource>,
            Arc::new(assembly.event_authority().consumer_registry_capability())
                as Arc<dyn DurableConsumerCursor + Send + Sync>,
            mapping as Arc<dyn OutcomeEvidenceMapping + Send + Sync>,
            MAPPING_ID,
            PerspectiveKey::new("default", "default").unwrap(),
            BranchScope::main(),
        )
        .with_family_revision(family.clone());
        let ingestion_report = ingestion.bounded_step(&EvidenceIngestionRequest { max_events: 32 });
        assert!(
            ingestion_report.fatal_errors.is_empty(),
            "{ingestion_report:#?}"
        );
        assert_eq!(ingestion_report.applicable_count, 1);
        assert_eq!(ingestion_report.revisions_committed, 1);
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
            1,
            "{ingestion_report:#?}"
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
        let unchanged_ingestion =
            ingestion.bounded_step(&EvidenceIngestionRequest { max_events: 32 });
        assert!(unchanged_ingestion.fatal_errors.is_empty());
        assert_eq!(unchanged_ingestion.revisions_committed, 0);
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
        drop(_supervisor);
        drop(assembly);

        let reopened = harness.assembly();
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
        let reopened_records = reopened
            .ports()
            .event_replay()
            .read_after_limit(0, 32)
            .unwrap();
        assert_eq!(
            reopened_records
                .iter()
                .filter(|record| record.envelope.event_type == "world-model.agent-genesis.v1")
                .count(),
            1
        );
        assert_eq!(
            reopened_records
                .iter()
                .filter(|record| {
                    record.envelope.event_type == meld_world_model::CURATION_RESULT_EVENT_TYPE
                })
                .count(),
            2
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
        use meld_execution::task::TaskInitializationPayload;
        use meld_execution::task_network::dispatch::Claim;
        use meld_execution::task_network::dispatch_actor::{
            ClaimedInvocationOutcome, ClaimedTaskInvoker, DispatchPortError,
        };
        use meld_execution::task_network::state::TaskNode;

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
            claim_invoker: SharedClaimedTaskInvoker(Arc::new(StubClaimInvoker)),
        }
    }

    #[test]
    fn late_bound_dispatch_routes_resolve_the_dispatch_actor() {
        let harness = StewardshipHarness::new();
        let assembly = harness.assembly();

        // Without composed routes the dispatch factory truthfully carries
        // no semantic body and the route seed names the composition inputs.
        let factory = assembly
            .handle_factories()
            .get("execution.task_dispatch")
            .unwrap();
        assert!(!factory.has_semantic_body());
        assert!(!assembly.dispatch_routes_bound());
        let seed = assembly.dispatch_route_seed().unwrap();
        assert_eq!(seed.session_id, "stewardship::docs_freshness");

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
            let assembly = harness.assembly();
            harness.run_world_genesis(&assembly);
        }

        let first_sequence = {
            let assembly = harness.assembly();
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
        let assembly = harness.assembly();
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
        assert!(!layout.task_artifacts_db.exists());
        assert!(!layout.task_networks_root.exists());
        assert!(!layout.frame_blob_root.exists());
        assert!(!layout.prompt_artifact_root.exists());
        assert!(assembly.stores().belief_store.is_open());
        assert!(!assembly.stores().node_store.is_open());
        assert!(assembly.try_graph_runtime().is_some());
    }

    #[test]
    fn stewardship_subject_ref_is_a_workspace_node() {
        let harness = StewardshipHarness::new();

        let subject = stewardship_subject_ref(&harness.binding).unwrap();

        assert_eq!(subject.domain_id, "workspace_fs");
        assert_eq!(subject.object_kind, "node");
        assert_eq!(subject.object_id, "docs");
    }

    /// The publication tick declares quiet only when the native outbox
    /// confirms emptiness. An absent network fails closed.
    #[test]
    fn publication_tick_declares_quiet_only_when_confirmed() {
        let harness = StewardshipHarness::new();
        let bindings = StewardshipActorBindings::derive(&harness.binding).unwrap();
        let store_dir = tempfile::tempdir().unwrap();
        let execution_db = sled::open(store_dir.path().join("execution")).unwrap();
        let network =
            SledTaskNetworkStore::open(execution_db, bindings.network_id.clone()).unwrap();

        let mut handle = PublicationHandle {
            runtime: PublicationRuntime::new(),
            event_append: crate::runtime::ports::ProductEventAppendPort::new(&harness.authority),
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
            .contains("no pending Task Network publications"));

        // Without a composed network no owner can prove an outbox wait.
        handle.network = None;
        let report = handle.tick(WorkBudget { max_items: 8 });
        assert!(report.waiting_on.is_empty(), "{report:?}");
        assert_eq!(report.fatal_errors.len(), 1, "{report:?}");
        assert_eq!(
            report.fatal_errors[0].code,
            "publication_task_network_unresolved"
        );
    }
}
