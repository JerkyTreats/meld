//! Product runtime assembly for durable flywheel infrastructure.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use meld_world_model::world_state::graph::runtime::{GraphCatchUpBudget, GraphRuntime};

use crate::config::MerkleConfig;
use crate::runtime::contracts::{WorkBudget, WorkerTickReport};
use crate::runtime::error::{RuntimeAssemblyError, RuntimeRegistryError};
use crate::runtime::ports::{ProductRuntimePorts, ProviderPortConfig};
use crate::runtime::storage::{OpenProductStores, ProductStorageLayout, ProductStorageRoot};
use crate::runtime::supervisor::SupervisorStore;

/// Root product runtime assembly.
///
/// This type opens product stores, opens supervisor lifecycle storage, builds
/// direct handoff ports, and prepares inert runtime factory metadata. It does
/// not run semantic work or own domain progress.
pub struct ProductRuntimeAssembly {
    product_root: ProductStorageRoot,
    layout: ProductStorageLayout,
    stores: Arc<OpenProductStores>,
    supervisor_store: SupervisorStore,
    ports: ProductRuntimePorts,
    registry: RuntimeFactoryRegistry,
    handle_factories: RuntimeHandleFactoryRegistry,
    desired_runtime_state: Vec<DesiredRuntimeState>,
    lifecycle_config: RuntimeLifecycleConfig,
    default_work_budget: WorkBudget,
    process_services: RuntimeProcessServices,
    diagnostics: Vec<AssemblyDiagnostic>,
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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuntimeResource {
    /// Event append port.
    EventAppend,
    /// Event replay port.
    EventReplay,
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

#[derive(Clone)]
enum RuntimeSemanticHandleFactory {
    None,
    GraphReplay { graph_runtime: Arc<GraphRuntime> },
}

enum RuntimeSemanticHandle {
    None,
    GraphReplay(GraphReplayRuntimeHandle),
}

#[derive(Clone)]
struct GraphReplayRuntimeHandle {
    graph_runtime: Arc<GraphRuntime>,
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

    /// Build assembly from a workspace and repository configuration.
    pub fn load_for_workspace(
        workspace_root: &Path,
        config: &MerkleConfig,
    ) -> Result<Self, RuntimeAssemblyError> {
        let product_root = config
            .system
            .storage
            .resolve_product_root(workspace_root)
            .map_err(|error| RuntimeAssemblyError::Config(error.to_string()))?;
        Self::load(ProductRuntimeConfig::for_product_root(product_root))
    }

    /// Build assembly from an explicit product root.
    pub fn load_for_product_root(
        product_root: impl Into<PathBuf>,
    ) -> Result<Self, RuntimeAssemblyError> {
        Self::load(ProductRuntimeConfig::for_product_root(product_root))
    }

    /// Open stores, build ports, and prepare inert runtime factory metadata.
    pub fn load(config: ProductRuntimeConfig) -> Result<Self, RuntimeAssemblyError> {
        if config.product_root.as_os_str().is_empty() {
            return Err(RuntimeAssemblyError::Config(
                "product root must not be empty".to_string(),
            ));
        }

        let product_root = ProductStorageRoot::new(config.product_root);
        let layout = product_root.layout();
        let stores = Arc::new(OpenProductStores::open(&layout)?);
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
        let mut provider = config.provider;
        provider.provider_required = provider_required(&registry, &desired_runtime_state);
        let supervisor_store = SupervisorStore::open(supervisor_store_path)?;
        let ports = ProductRuntimePorts::from_stores(stores.as_ref(), provider)?;
        let handle_factories =
            RuntimeHandleFactoryRegistry::from_registry(&registry, stores.as_ref(), &ports);

        Ok(Self {
            product_root,
            layout,
            stores,
            supervisor_store,
            ports,
            registry,
            handle_factories,
            desired_runtime_state,
            lifecycle_config: config.lifecycle_config,
            default_work_budget: config.default_work_budget,
            process_services: config.process_services,
            diagnostics: Vec::new(),
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

    /// Return desired runtime states prepared for supervisor handoff.
    pub fn desired_runtime_state(&self) -> &[DesiredRuntimeState] {
        &self.desired_runtime_state
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
            RuntimeFactoryDescriptor::new("world_model.graph_replay", vec![EventReplay])?,
            RuntimeFactoryDescriptor::new("world_model.belief_assessment", vec![])?,
            RuntimeFactoryDescriptor::new(
                "world_model.agent_goal_curation",
                vec![GoalCommand, PlannerProjection],
            )?,
            RuntimeFactoryDescriptor::new("world_model.evidence_ingestion", vec![EventReplay])?,
            RuntimeFactoryDescriptor::new(
                "world_model.satisfaction_curation",
                vec![GoalMutation, PlannerProjection],
            )?,
            RuntimeFactoryDescriptor::new("execution.goal_set", vec![GoalCommand])?,
            RuntimeFactoryDescriptor::new(
                "execution.planning",
                vec![PlannerProjection, TaskNetworkFactory],
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
    pub fn from_registry(
        registry: &RuntimeFactoryRegistry,
        stores: &OpenProductStores,
        ports: &ProductRuntimePorts,
    ) -> Self {
        let factories = registry
            .descriptors()
            .map(|descriptor| {
                (
                    descriptor.runtime_id.clone(),
                    RuntimeHandleFactory {
                        descriptor: descriptor.clone(),
                        semantic: RuntimeSemanticHandleFactory::for_descriptor(
                            descriptor, stores, ports,
                        ),
                    },
                )
            })
            .collect();
        Self { factories }
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
    fn for_descriptor(
        descriptor: &RuntimeFactoryDescriptor,
        stores: &OpenProductStores,
        _ports: &ProductRuntimePorts,
    ) -> Self {
        match descriptor.runtime_id.as_str() {
            "world_model.graph_replay" => Self::GraphReplay {
                graph_runtime: Arc::new(GraphRuntime::from_stores(
                    Arc::clone(&stores.event_store),
                    Arc::clone(&stores.traversal_store),
                )),
            },
            _ => Self::None,
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
        }
    }
}

impl RuntimeSemanticHandle {
    fn tick(&mut self, budget: WorkBudget) -> Option<WorkerTickReport> {
        match self {
            Self::None => None,
            Self::GraphReplay(handle) => Some(handle.tick(budget)),
        }
    }

    fn request_stop(&mut self) {}
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

        assert_eq!(description.product_root, workspace.join(".meld-runtime"));
        assert_eq!(
            description.supervisor_store_path,
            workspace.join(".meld-runtime").join("supervisor.sled")
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
    fn event_append_and_replay_ports_are_wired_to_event_store() {
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

        assert_eq!(seq, 1);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].seq, 1);
        assert_eq!(records[0].record_id.as_deref(), Some("record-a"));
        assert!(assembly
            .ports()
            .event_replay()
            .read_after_limit(0, 0)
            .unwrap()
            .is_empty());
        assert!(assembly
            .ports()
            .event_replay()
            .read_after_limit(0, crate::runtime::ports::MAX_EVENT_REPLAY_LIMIT)
            .is_ok());
        assert!(assembly
            .ports()
            .event_replay()
            .read_after_limit(0, crate::runtime::ports::MAX_EVENT_REPLAY_LIMIT + 1)
            .is_err());
        assert!(assembly
            .ports()
            .event_replay()
            .read_after_limit(0, usize::MAX)
            .is_err());
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
            .stores()
            .event_store
            .read_all_events_after(0)
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
            kind: meld_world_model::AgentGoalMutationKind::Satisfy { at_seq: review_seq },
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
}
