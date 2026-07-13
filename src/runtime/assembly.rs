//! Product runtime assembly for durable flywheel infrastructure.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use meld_events::EventAuthority;
#[cfg(test)]
use meld_events::EventAuthorityOpenOptions;
use meld_execution::activation::{
    validate_execution_activation, ExecutionActivationInput, ExecutionActivationValidationReceipt,
};
use meld_world_model::activation::{validate_world_model_activation, WorldModelActivationInput};
use meld_world_model::world_state::graph::runtime::{GraphCatchUpBudget, GraphRuntime};
use meld_world_model::AgentBootstrapRuntime;

use crate::config::MerkleConfig;
use crate::runtime::activation::RuntimeActivationInput;
use crate::runtime::contracts::{
    RuntimeImplementationState, RuntimeRoleClass, WorkBudget, WorkerTickReport,
};
use crate::runtime::error::{RuntimeAssemblyError, RuntimeRegistryError};
use crate::runtime::ports::{ProductRuntimePorts, ProviderPortConfig};
use crate::runtime::storage::{OpenProductStores, ProductStorageLayout, ProductStorageRoot};
use crate::runtime::supervisor::SupervisorStore;

/// Root product runtime assembly.
///
/// This type opens product stores, opens supervisor lifecycle storage, builds
/// direct handoff ports, and prepares runtime factory metadata. It does
/// not run semantic work or own domain progress.
pub struct ProductRuntimeAssembly {
    product_root: ProductStorageRoot,
    layout: ProductStorageLayout,
    stores: Arc<OpenProductStores>,
    event_authority: Arc<EventAuthority>,
    graph_runtime: Arc<GraphRuntime>,
    supervisor_store: SupervisorStore,
    ports: ProductRuntimePorts,
    registry: RuntimeFactoryRegistry,
    handle_factories: RuntimeHandleFactoryRegistry,
    desired_runtime_state: Vec<DesiredRuntimeState>,
    lifecycle_config: RuntimeLifecycleConfig,
    default_work_budget: WorkBudget,
    process_services: RuntimeProcessServices,
    diagnostics: Vec<AssemblyDiagnostic>,
    activation_execution: Option<ExecutionActivationState>,
}

/// Pure execution activation products retained without opening execution stores.
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionActivationState {
    /// Execution-owned source-neutral validated input.
    pub input: ExecutionActivationInput,
    /// Pure deterministic execution validation receipt.
    pub receipt: ExecutionActivationValidationReceipt,
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
    /// Runtime ids desired by configuration. Empty uses registry defaults.
    pub enabled_runtime_ids: Vec<String>,
    /// Runtime ids kept disabled but visible in desired state.
    pub disabled_runtime_ids: Vec<String>,
    /// Passive provider availability check.
    pub provider: ProviderPortConfig,
    /// Supervisor lifecycle timing defaults.
    pub lifecycle_config: RuntimeLifecycleConfig,
    /// Work budget defaults passed to ticking runtime factories.
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
    /// Canonical lifecycle shape for this runtime id.
    pub role_class: RuntimeRoleClass,
    /// Honest implementation posture after desired-state resolution.
    pub implementation_state: RuntimeImplementationState,
}

impl DesiredRuntimeState {
    /// Return whether supervisor lifecycle may host this configured role.
    pub fn host_eligible(&self) -> bool {
        self.enabled
            && self.factory_available
            && self.implementation_state == RuntimeImplementationState::Concrete
            && matches!(
                self.role_class,
                RuntimeRoleClass::Actor | RuntimeRoleClass::PassiveService
            )
    }

    /// Return whether this hosted role owns bounded semantic ticks.
    pub fn tick_eligible(&self) -> bool {
        self.host_eligible() && self.role_class == RuntimeRoleClass::Actor
    }
}

/// Passive runtime factory metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFactoryDescriptor {
    /// Stable runtime id.
    pub runtime_id: String,
    /// Passive resources required by the future runtime handle.
    pub required_resources: Vec<RuntimeResource>,
    /// Legacy or requirement-era names accepted only at ingress.
    pub aliases: Vec<String>,
    /// Lifecycle shape owned by this role.
    pub role_class: RuntimeRoleClass,
    /// Current hosted implementation posture.
    pub implementation_state: RuntimeImplementationState,
    /// Whether empty configuration starts this role.
    pub default_enabled: bool,
}

/// Resource requirement advertised by a runtime factory descriptor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
}

/// Process-local registry of runtime factories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFactoryRegistry {
    descriptors: BTreeMap<String, RuntimeFactoryDescriptor>,
    aliases: BTreeMap<String, String>,
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
pub struct RuntimeHandle {
    runtime_id: String,
    role_class: RuntimeRoleClass,
    required_resources: Vec<RuntimeResource>,
    started: bool,
    semantic: RuntimeSemanticHandle,
    #[cfg(test)]
    fail_safe_point: bool,
    #[cfg(test)]
    fail_flush: bool,
}

// TODO compat-shim: remove after W3B production factories and downstream
// callers use RuntimeHandle and the runtime assembly parity suite stays green.
/// Compatibility name for callers compiled against the pre-W3A handle type.
#[deprecated(since = "2.7.0", note = "use RuntimeHandle")]
pub type InertRuntimeHandle = RuntimeHandle;

#[derive(Clone)]
enum RuntimeSemanticHandleFactory {
    None,
    GraphReplay {
        graph_runtime: Arc<GraphRuntime>,
    },
    EventAppend {
        port: crate::runtime::ports::ProductEventAppendPort,
    },
    AgentBootstrap {
        runtime: Arc<AgentBootstrapRuntime>,
        input: Arc<WorldModelActivationInput>,
    },
}

enum RuntimeSemanticHandle {
    None,
    GraphReplay(GraphReplayRuntimeHandle),
    EventAppend(EventAppendRuntimeHandle),
    AgentBootstrap(AgentBootstrapRuntimeHandle),
}

#[derive(Clone)]
struct GraphReplayRuntimeHandle {
    graph_runtime: Arc<GraphRuntime>,
}

struct AgentBootstrapRuntimeHandle {
    runtime: Arc<AgentBootstrapRuntime>,
    input: Arc<WorldModelActivationInput>,
}

/// Diagnostics-only handle publishing ledger ingress health through the
/// standard tick report path: the watermark is its checkpoint and drop
/// bursts surface as retryable issues, so heartbeats and health snapshots
/// carry ledger state without new publisher plumbing.
struct EventAppendRuntimeHandle {
    port: crate::runtime::ports::ProductEventAppendPort,
    // Baseline sampled on the first tick so a restart or an existing ledger
    // never misreports history as fresh work or fresh drops.
    last: Option<(u64, u64)>,
}

/// Lease context supplied by the supervisor before a handle starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLeaseContext {
    /// Runtime id whose lease was acquired.
    pub runtime_id: String,
    /// Non-empty supervisor lease id.
    pub lease_id: String,
}

/// Report returned when a runtime handle accepts a supervisor start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandleStartReport {
    /// Runtime id that accepted the start.
    pub runtime_id: String,
}

/// Report returned when a runtime handle accepts a stop request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandleStopReport {
    /// Runtime id that accepted the stop.
    pub runtime_id: String,
    /// Whether the handle was running before the stop request.
    pub was_started: bool,
}

/// Report returned when a runtime handle reaches a safe point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandleSafePointReport {
    /// Runtime id that reached the safe point.
    pub runtime_id: String,
    /// Whether the handle is safe for product flush.
    pub safe_for_flush: bool,
}

/// Diagnostic snapshot from one runtime handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHandleDiagnostic {
    /// Runtime id that produced the diagnostic.
    pub runtime_id: String,
    /// Whether the supervisor has started the handle.
    pub started: bool,
    /// Passive resources referenced by the handle.
    pub required_resources: Vec<RuntimeResource>,
}

/// Report returned by a runtime handle flush hook.
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
    /// Only concrete actor roles are enabled by default.
    pub fn for_product_root(product_root: impl Into<PathBuf>) -> Self {
        Self {
            product_root: product_root.into(),
            supervisor_store_path: None,
            enabled_runtime_ids: Vec::new(),
            disabled_runtime_ids: Vec::new(),
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
    /// Validate activated owner inputs and concrete semantic factory support.
    ///
    /// This boundary is store-free and must complete before event binding,
    /// compatibility migration, or product store construction.
    pub fn validate_activated_inputs(
        runtime: &RuntimeActivationInput,
        world_model: &WorldModelActivationInput,
        execution: &ExecutionActivationInput,
        execution_receipt: &ExecutionActivationValidationReceipt,
    ) -> Result<(), RuntimeAssemblyError> {
        validate_world_model_activation(world_model).map_err(|error| {
            RuntimeAssemblyError::Config(format!(
                "world-model activation validation failed at '{}': {}",
                error.field, error.message
            ))
        })?;
        let derived_execution_receipt =
            validate_execution_activation(execution).map_err(|error| {
                RuntimeAssemblyError::Config(format!(
                    "execution activation validation failed: {error}"
                ))
            })?;
        if &derived_execution_receipt != execution_receipt {
            return Err(RuntimeAssemblyError::Config(
                "execution activation receipt does not match validated owner input".to_string(),
            ));
        }
        if runtime.activation_id != world_model.activation_id
            || runtime.activation_id != execution.selection.activation_id
            || runtime.activation_id != execution_receipt.activation_id
        {
            return Err(RuntimeAssemblyError::Config(
                "activated owner packages disagree on activation id".to_string(),
            ));
        }
        if runtime.activation_hash != world_model.activation_hash
            || runtime.activation_hash != execution.selection.activation_hash
            || runtime.activation_hash != execution_receipt.activation_hash
        {
            return Err(RuntimeAssemblyError::Config(
                "activated owner packages disagree on activation hash".to_string(),
            ));
        }
        if runtime.bootstrap_runtime_id != world_model.bootstrap_id {
            return Err(RuntimeAssemblyError::Config(
                "runtime and world-model bootstrap ids disagree".to_string(),
            ));
        }
        if runtime.bootstrap_runtime_id != "world_model.agent.bootstrap.docs_freshness" {
            return Err(RuntimeAssemblyError::Config(
                "activation selected an unsupported bootstrap runtime id".to_string(),
            ));
        }
        if !runtime
            .enabled_runtime_ids
            .contains(&runtime.bootstrap_runtime_id)
        {
            return Err(RuntimeAssemblyError::Config(
                "bootstrap runtime id must be enabled by activation".to_string(),
            ));
        }

        validate_runtime_selection(&runtime.enabled_runtime_ids)?;
        let registry = RuntimeFactoryRegistry::first_proof_registry()?;
        let enabled = runtime
            .enabled_runtime_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let supported = BTreeSet::from([
            "world_model.agent.bootstrap.docs_freshness",
            "world_model.graph_replay",
        ]);
        if enabled != supported {
            return Err(RuntimeAssemblyError::Config(
                "activation must enable only graph replay and docs freshness bootstrap".to_string(),
            ));
        }
        for runtime_id in &runtime.enabled_runtime_ids {
            let descriptor = registry
                .get(runtime_id)
                .ok_or_else(|| RuntimeAssemblyError::UnsupportedRuntimeId(runtime_id.clone()))?;
            if registry.canonical_id(runtime_id) != Some(runtime_id.as_str()) {
                return Err(RuntimeAssemblyError::Config(format!(
                    "activated runtime id '{runtime_id}' is not canonical"
                )));
            }
            if descriptor.role_class != RuntimeRoleClass::Actor
                || descriptor.implementation_state != RuntimeImplementationState::Concrete
                || !RuntimeSemanticHandleFactory::supports_activated_runtime(runtime_id)
            {
                return Err(RuntimeAssemblyError::RuntimeHandleConstruction(format!(
                    "activated runtime '{runtime_id}' has no concrete semantic factory"
                )));
            }
        }
        Ok(())
    }

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

    /// Open an activation-selected product runtime over one supplied authority.
    #[allow(clippy::too_many_arguments)]
    pub fn load_activated_with_authority(
        workspace_root: &Path,
        config: &MerkleConfig,
        event_authority: Arc<EventAuthority>,
        runtime: RuntimeActivationInput,
        world_model: WorldModelActivationInput,
        execution: ExecutionActivationInput,
        execution_receipt: ExecutionActivationValidationReceipt,
    ) -> Result<Self, RuntimeAssemblyError> {
        Self::validate_activated_inputs(&runtime, &world_model, &execution, &execution_receipt)?;
        let product_root = config
            .system
            .storage
            .resolve_product_root(workspace_root)
            .map_err(|error| RuntimeAssemblyError::Config(error.to_string()))?;
        let mut product_config = ProductRuntimeConfig::for_product_root(product_root);
        product_config.enabled_runtime_ids = runtime.enabled_runtime_ids.clone();
        Self::load_with_authority_inner(
            product_config,
            event_authority,
            Some((runtime, world_model)),
            Some(ExecutionActivationState {
                input: execution,
                receipt: execution_receipt,
            }),
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
        prevalidate_runtime_assembly_config(&config, false)?;
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

    /// Open non-event product stores and compose runtime around one supplied authority.
    pub fn load_with_authority(
        config: ProductRuntimeConfig,
        event_authority: Arc<EventAuthority>,
    ) -> Result<Self, RuntimeAssemblyError> {
        Self::load_with_authority_inner(config, event_authority, None, None)
    }

    fn load_with_authority_inner(
        config: ProductRuntimeConfig,
        event_authority: Arc<EventAuthority>,
        activation: Option<(RuntimeActivationInput, WorldModelActivationInput)>,
        activation_execution: Option<ExecutionActivationState>,
    ) -> Result<Self, RuntimeAssemblyError> {
        let (registry, desired_runtime_state) =
            prevalidate_runtime_assembly_config(&config, activation.is_some())?;
        let product_root = ProductStorageRoot::new(config.product_root);
        let layout = product_root.layout();
        let supervisor_store_path = config
            .supervisor_store_path
            .unwrap_or_else(|| layout.root.join("supervisor.sled"));
        let stores = Arc::new(OpenProductStores::open(&layout)?);
        let mut provider = config.provider;
        provider.provider_required = provider_required(&registry, &desired_runtime_state);
        let supervisor_store = SupervisorStore::open(supervisor_store_path)?;
        let ports = ProductRuntimePorts::from_authority(
            stores.as_ref(),
            event_authority.as_ref(),
            provider,
        )?;
        let graph_runtime = Arc::new(
            GraphRuntime::from_ports(
                Arc::new(ports.event_replay().clone()),
                Arc::new(ports.event_append().clone()),
                Arc::new(ports.graph_cursor().clone()),
                Arc::clone(&stores.traversal_store),
            )
            .map_err(|error| RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string()))?,
        );
        let handle_factories = match activation {
            Some((runtime, world_model)) => RuntimeHandleFactoryRegistry::from_activated_registry(
                &registry,
                &ports,
                &graph_runtime,
                stores.agent_store.as_ref(),
                runtime,
                world_model,
            )?,
            None => RuntimeHandleFactoryRegistry::from_registry(&registry, &ports, &graph_runtime)?,
        };

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
            desired_runtime_state,
            lifecycle_config: config.lifecycle_config,
            default_work_budget: config.default_work_budget,
            process_services: config.process_services,
            diagnostics: Vec::new(),
            activation_execution,
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
    pub fn graph_runtime(&self) -> Arc<GraphRuntime> {
        Arc::clone(&self.graph_runtime)
    }

    /// Return supervisor lifecycle storage.
    pub fn supervisor_store(&self) -> &SupervisorStore {
        &self.supervisor_store
    }

    /// Return direct handoff and adapter ports.
    pub fn ports(&self) -> &ProductRuntimePorts {
        &self.ports
    }

    /// Return the runtime factory registry.
    pub fn registry(&self) -> &RuntimeFactoryRegistry {
        &self.registry
    }

    /// Return runtime handle factories.
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

    /// Return retained pure execution activation products when configured.
    pub fn activation_execution(&self) -> Option<&ExecutionActivationState> {
        self.activation_execution.as_ref()
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
            aliases: Vec::new(),
            role_class: RuntimeRoleClass::Actor,
            implementation_state: RuntimeImplementationState::Inert,
            default_enabled: false,
        })
    }

    /// Construct one fully classified canonical runtime descriptor.
    pub fn classified(
        runtime_id: impl Into<String>,
        aliases: &[&str],
        role_class: RuntimeRoleClass,
        implementation_state: RuntimeImplementationState,
        default_enabled: bool,
        required_resources: Vec<RuntimeResource>,
    ) -> Result<Self, RuntimeRegistryError> {
        let runtime_id = runtime_id.into();
        validate_runtime_id(&runtime_id)?;
        let aliases = aliases
            .iter()
            .map(|alias| (*alias).to_string())
            .map(|alias| {
                validate_runtime_id(&alias)?;
                Ok(alias)
            })
            .collect::<Result<Vec<_>, RuntimeRegistryError>>()?;
        if default_enabled
            && (role_class != RuntimeRoleClass::Actor
                || implementation_state != RuntimeImplementationState::Concrete)
        {
            return Err(RuntimeRegistryError::InvalidDefaultRuntimeRole(runtime_id));
        }
        Ok(Self {
            runtime_id,
            required_resources,
            aliases,
            role_class,
            implementation_state,
            default_enabled,
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
            if descriptor.default_enabled
                && (descriptor.role_class != RuntimeRoleClass::Actor
                    || descriptor.implementation_state != RuntimeImplementationState::Concrete)
            {
                return Err(RuntimeRegistryError::InvalidDefaultRuntimeRole(
                    descriptor.runtime_id,
                ));
            }
            if registry.contains_key(&descriptor.runtime_id) {
                return Err(RuntimeRegistryError::DuplicateRuntimeId(
                    descriptor.runtime_id,
                ));
            }
            registry.insert(descriptor.runtime_id.clone(), descriptor);
        }
        let canonical_ids = registry.keys().cloned().collect::<BTreeSet<_>>();
        let mut aliases = BTreeMap::new();
        for descriptor in registry.values() {
            for alias in &descriptor.aliases {
                validate_runtime_id(alias)?;
                if canonical_ids.contains(alias) {
                    return Err(RuntimeRegistryError::RuntimeAliasCollision(alias.clone()));
                }
                if aliases
                    .insert(alias.clone(), descriptor.runtime_id.clone())
                    .is_some()
                {
                    return Err(RuntimeRegistryError::DuplicateRuntimeAlias(alias.clone()));
                }
            }
        }
        Ok(Self {
            descriptors: registry,
            aliases,
        })
    }

    /// Build the first durable flywheel proof runtime registry.
    pub fn first_proof_registry() -> Result<Self, RuntimeRegistryError> {
        use RuntimeImplementationState::{Concrete, Inert};
        use RuntimeResource::*;
        use RuntimeRoleClass::{Actor, PassiveService, PortOnly};
        Self::from_descriptors([
            RuntimeFactoryDescriptor::classified(
                "event.append",
                &["events.ledger"],
                PassiveService,
                Concrete,
                false,
                vec![EventAppend],
            )?,
            RuntimeFactoryDescriptor::classified(
                "event.replay",
                &[],
                PortOnly,
                Concrete,
                false,
                vec![EventReplay],
            )?,
            RuntimeFactoryDescriptor::classified(
                "world_model.graph_replay",
                &["world_model.graph.replay"],
                Actor,
                Concrete,
                true,
                vec![EventAppend, EventReplay, EventConsumerRegistry],
            )?,
            RuntimeFactoryDescriptor::classified(
                "world_model.agent.bootstrap.docs_freshness",
                &[],
                Actor,
                Concrete,
                false,
                vec![],
            )?,
            RuntimeFactoryDescriptor::classified(
                "world_model.belief_assessment",
                &["world_model.belief.assessment"],
                Actor,
                Inert,
                false,
                vec![],
            )?,
            RuntimeFactoryDescriptor::classified(
                "world_model.agent_goal_curation",
                &["world_model.agent.goal_curation"],
                Actor,
                Inert,
                false,
                vec![GoalCommand, PlannerProjection],
            )?,
            RuntimeFactoryDescriptor::classified(
                "world_model.agent_hydration",
                &[],
                Actor,
                Inert,
                false,
                vec![PlannerProjection],
            )?,
            RuntimeFactoryDescriptor::classified(
                "world_model.evidence_ingestion",
                &["world_model.belief.event_evidence_ingestion"],
                Actor,
                Inert,
                false,
                vec![EventReplay],
            )?,
            RuntimeFactoryDescriptor::classified(
                "world_model.planner_projection",
                &[],
                Actor,
                Inert,
                false,
                vec![],
            )?,
            RuntimeFactoryDescriptor::classified(
                "world_model.satisfaction_curation",
                &["world_model.agent.satisfaction_curation"],
                Actor,
                Inert,
                false,
                vec![GoalMutation, PlannerProjection],
            )?,
            RuntimeFactoryDescriptor::classified(
                "execution.goal_set",
                &["execution.goal.set"],
                PortOnly,
                Concrete,
                false,
                vec![GoalCommand],
            )?,
            RuntimeFactoryDescriptor::classified(
                "execution.planning",
                &[],
                Actor,
                Inert,
                false,
                vec![PlannerProjection, TaskNetworkFactory],
            )?,
            RuntimeFactoryDescriptor::classified(
                "execution.task_network_command",
                &["execution.task_network.command"],
                PortOnly,
                Concrete,
                false,
                vec![TaskNetworkFactory],
            )?,
            RuntimeFactoryDescriptor::classified(
                "execution.task_dispatch",
                &["execution.task.dispatch"],
                Actor,
                Inert,
                false,
                vec![
                    TaskNetworkFactory,
                    TaskArtifactFactory,
                    Context,
                    Provider,
                    Prompt,
                    Workspace,
                ],
            )?,
            RuntimeFactoryDescriptor::classified(
                "execution.publication",
                &["execution.task_network.publication"],
                Actor,
                Inert,
                false,
                vec![TaskNetworkFactory, EventAppend],
            )?,
        ])
    }

    /// Return one descriptor by runtime id.
    pub fn get(&self, runtime_id: &str) -> Option<&RuntimeFactoryDescriptor> {
        let canonical = self.canonical_id(runtime_id)?;
        self.descriptors.get(canonical)
    }

    /// Return descriptors in stable runtime id order.
    pub fn descriptors(&self) -> impl Iterator<Item = &RuntimeFactoryDescriptor> {
        self.descriptors.values()
    }

    /// Return whether the registry has a runtime id.
    pub fn contains(&self, runtime_id: &str) -> bool {
        self.canonical_id(runtime_id).is_some()
    }

    /// Resolve a canonical id or ingress alias to the persisted id.
    pub fn canonical_id<'a>(&'a self, runtime_id: &'a str) -> Option<&'a str> {
        if self.descriptors.contains_key(runtime_id) {
            Some(runtime_id)
        } else {
            self.aliases.get(runtime_id).map(String::as_str)
        }
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
    /// Build handle factories from the runtime factory registry.
    pub fn from_registry(
        registry: &RuntimeFactoryRegistry,
        ports: &ProductRuntimePorts,
        graph_runtime: &Arc<GraphRuntime>,
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
                            graph_runtime,
                        )?,
                    },
                ))
            })
            .collect::<Result<_, RuntimeAssemblyError>>()?;
        Ok(Self { factories })
    }

    fn from_activated_registry(
        registry: &RuntimeFactoryRegistry,
        ports: &ProductRuntimePorts,
        graph_runtime: &Arc<GraphRuntime>,
        agent_store: &meld_world_model::AgentStore,
        runtime_input: RuntimeActivationInput,
        world_model_input: WorldModelActivationInput,
    ) -> Result<Self, RuntimeAssemblyError> {
        let bootstrap_runtime = Arc::new(
            AgentBootstrapRuntime::from_agent_store(agent_store).map_err(|error| {
                RuntimeAssemblyError::RuntimeHandleConstruction(error.to_string())
            })?,
        );
        let bootstrap_runtime_id = runtime_input.bootstrap_runtime_id;
        let world_model_input = Arc::new(world_model_input);
        let factories = registry
            .descriptors()
            .map(|descriptor| {
                Ok((
                    descriptor.runtime_id.clone(),
                    RuntimeHandleFactory {
                        descriptor: descriptor.clone(),
                        semantic: RuntimeSemanticHandleFactory::for_activated_descriptor(
                            descriptor,
                            ports,
                            graph_runtime,
                            &bootstrap_runtime_id,
                            &bootstrap_runtime,
                            &world_model_input,
                        )?,
                    },
                ))
            })
            .collect::<Result<_, RuntimeAssemblyError>>()?;
        Ok(Self { factories })
    }

    /// Return one handle factory by runtime id.
    pub fn get(&self, runtime_id: &str) -> Option<&RuntimeHandleFactory> {
        self.factories.get(runtime_id)
    }

    /// Return the number of handle factories.
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

    /// Build a runtime handle.
    pub fn build_handle(&self) -> RuntimeHandle {
        RuntimeHandle {
            runtime_id: self.descriptor.runtime_id.clone(),
            role_class: self.descriptor.role_class,
            required_resources: self.descriptor.required_resources.clone(),
            started: false,
            semantic: self.semantic.build_handle(),
            #[cfg(test)]
            fail_safe_point: false,
            #[cfg(test)]
            fail_flush: false,
        }
    }
}

impl RuntimeHandle {
    /// Return this handle's runtime id.
    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    /// Return passive resources required by this runtime handle.
    pub fn required_resources(&self) -> &[RuntimeResource] {
        &self.required_resources
    }

    /// Return the lifecycle role assigned by the canonical registry.
    pub fn role_class(&self) -> RuntimeRoleClass {
        self.role_class
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
        #[cfg(test)]
        if self.fail_flush {
            return Err(RuntimeAssemblyError::SupervisorHandoff(format!(
                "runtime '{}' injected flush failure",
                self.runtime_id
            )));
        }
        Ok(RuntimeHandleFlushReport {
            runtime_id: self.runtime_id.clone(),
            flushed_resource: false,
        })
    }

    /// Request that a runtime handle stop at its next safe point.
    pub fn request_stop(&mut self) -> RuntimeHandleStopReport {
        let was_started = self.started;
        self.started = false;
        self.semantic.request_stop();
        RuntimeHandleStopReport {
            runtime_id: self.runtime_id.clone(),
            was_started,
        }
    }

    /// Wait for a runtime handle safe point.
    pub fn wait_for_safe_point(&self) -> RuntimeHandleSafePointReport {
        RuntimeHandleSafePointReport {
            runtime_id: self.runtime_id.clone(),
            safe_for_flush: !self.started && {
                #[cfg(test)]
                {
                    !self.fail_safe_point
                }
                #[cfg(not(test))]
                {
                    true
                }
            },
        }
    }

    #[cfg(test)]
    pub(crate) fn set_test_lifecycle_failures(&mut self, fail_safe_point: bool, fail_flush: bool) {
        self.fail_safe_point = fail_safe_point;
        self.fail_flush = fail_flush;
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
    fn supports_activated_runtime(runtime_id: &str) -> bool {
        matches!(
            runtime_id,
            "world_model.graph_replay" | "world_model.agent.bootstrap.docs_freshness"
        )
    }

    fn for_descriptor(
        descriptor: &RuntimeFactoryDescriptor,
        ports: &ProductRuntimePorts,
        graph_runtime: &Arc<GraphRuntime>,
    ) -> Result<Self, RuntimeAssemblyError> {
        match descriptor.runtime_id.as_str() {
            "world_model.graph_replay" => Ok(Self::GraphReplay {
                graph_runtime: Arc::clone(graph_runtime),
            }),
            "event.append" => Ok(Self::EventAppend {
                port: ports.event_append().clone(),
            }),
            _ => Ok(Self::None),
        }
    }

    fn for_activated_descriptor(
        descriptor: &RuntimeFactoryDescriptor,
        ports: &ProductRuntimePorts,
        graph_runtime: &Arc<GraphRuntime>,
        bootstrap_runtime_id: &str,
        bootstrap_runtime: &Arc<AgentBootstrapRuntime>,
        world_model_input: &Arc<WorldModelActivationInput>,
    ) -> Result<Self, RuntimeAssemblyError> {
        if descriptor.runtime_id == bootstrap_runtime_id {
            return Ok(Self::AgentBootstrap {
                runtime: Arc::clone(bootstrap_runtime),
                input: Arc::clone(world_model_input),
            });
        }
        Self::for_descriptor(descriptor, ports, graph_runtime)
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
            Self::AgentBootstrap { runtime, input } => {
                RuntimeSemanticHandle::AgentBootstrap(AgentBootstrapRuntimeHandle {
                    runtime: Arc::clone(runtime),
                    input: Arc::clone(input),
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
            Self::EventAppend(handle) => Some(handle.tick()),
            Self::AgentBootstrap(handle) => Some(handle.tick(budget)),
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
                    issues.push(crate::runtime::contracts::WorkerTickIssue {
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
            retryable_errors.push(crate::runtime::contracts::WorkerTickIssue {
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
            scope: crate::runtime::contracts::WorkerScope {
                domain_id: "events".to_string(),
                stream_id: None,
                work_key: None,
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: None,
            },
            input_checkpoint: crate::runtime::contracts::WorkerCheckpoint {
                name: "event_commit_watermark".to_string(),
                value: input_watermark,
            },
            output_checkpoint: crate::runtime::contracts::WorkerCheckpoint {
                name: "event_commit_watermark".to_string(),
                value: watermark,
            },
            items_attempted: 0,
            items_committed: 0,
            retryable_errors,
            fatal_errors: Vec::new(),
            budget_exhausted: false,
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

fn prevalidate_runtime_assembly_config(
    config: &ProductRuntimeConfig,
    has_activation: bool,
) -> Result<(RuntimeFactoryRegistry, Vec<DesiredRuntimeState>), RuntimeAssemblyError> {
    if config.product_root.as_os_str().is_empty() {
        return Err(RuntimeAssemblyError::Config(
            "product root must not be empty".to_string(),
        ));
    }
    let registry = RuntimeFactoryRegistry::first_proof_registry()?;
    validate_runtime_selection(&config.enabled_runtime_ids)?;
    validate_runtime_selection(&config.disabled_runtime_ids)?;
    let desired_runtime_state = desired_runtime_state(
        &registry,
        config.enabled_runtime_ids.clone(),
        config.disabled_runtime_ids.clone(),
    )?;
    if !has_activation
        && desired_runtime_state.iter().any(|state| {
            state.enabled && state.runtime_id == "world_model.agent.bootstrap.docs_freshness"
        })
    {
        return Err(RuntimeAssemblyError::RuntimeHandleConstruction(
            "bootstrap runtime requires activated world-model owner input".to_string(),
        ));
    }
    Ok((registry, desired_runtime_state))
}

impl AgentBootstrapRuntimeHandle {
    fn tick(&self, budget: WorkBudget) -> WorkerTickReport {
        let prior_progress = self.runtime.progress(&self.input.bootstrap_id);
        let input_sequence = prior_progress
            .as_ref()
            .ok()
            .and_then(|progress| progress.as_ref())
            .map(|progress| progress.updated_at_seq)
            .unwrap_or(0);
        if budget.max_items == 0 {
            return self.report(
                input_sequence,
                input_sequence,
                0,
                0,
                Vec::new(),
                Vec::new(),
                true,
            );
        }
        if let Err(error) = prior_progress {
            return self.failure_report(input_sequence, error);
        }

        match self.runtime.bootstrap(&self.input) {
            Ok(report) => self.report(
                input_sequence,
                report.progress.updated_at_seq,
                1,
                usize::from(report.work_performed),
                Vec::new(),
                Vec::new(),
                false,
            ),
            Err(error) => self.failure_report(input_sequence, error),
        }
    }

    fn failure_report(
        &self,
        input_checkpoint: u64,
        error: meld_world_model::AgentBootstrapError,
    ) -> WorkerTickReport {
        let output_checkpoint = self
            .runtime
            .progress(&self.input.bootstrap_id)
            .ok()
            .flatten()
            .map(|progress| progress.updated_at_seq)
            .unwrap_or(input_checkpoint);
        let (classification, issue) = bootstrap_worker_issue(&self.input.bootstrap_id, error);
        let (retryable_errors, fatal_errors) = match classification {
            meld_world_model::AgentBootstrapErrorClass::Retryable => (vec![issue], Vec::new()),
            meld_world_model::AgentBootstrapErrorClass::Fatal => (Vec::new(), vec![issue]),
        };
        self.report(
            input_checkpoint,
            output_checkpoint,
            1,
            0,
            retryable_errors,
            fatal_errors,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn report(
        &self,
        input_sequence: u64,
        output_sequence: u64,
        items_attempted: usize,
        items_committed: usize,
        retryable_errors: Vec<crate::runtime::contracts::WorkerTickIssue>,
        fatal_errors: Vec<crate::runtime::contracts::WorkerTickIssue>,
        budget_exhausted: bool,
    ) -> WorkerTickReport {
        WorkerTickReport {
            actor_id: "world_model.agent.bootstrap.docs_freshness".to_string(),
            scope: crate::runtime::contracts::WorkerScope {
                domain_id: "world_model".to_string(),
                stream_id: None,
                work_key: Some(self.input.bootstrap_id.clone()),
                agent_id: Some(self.input.seed_agent.agent_id.clone()),
                perspective_key: Some(format!(
                    "{}:{}",
                    self.input.seed_agent.perspective_key.perspective_kind,
                    self.input.seed_agent.perspective_key.perspective_id
                )),
                branch_id: Some(self.input.seed_agent.branch_scope.branch_id.clone()),
                subject_key: Some(format!(
                    "{}:{}:{}",
                    self.input.seed_agent.subject.domain_id,
                    self.input.seed_agent.subject.object_kind,
                    self.input.seed_agent.subject.object_id
                )),
            },
            input_checkpoint: crate::runtime::contracts::WorkerCheckpoint {
                name: "agent_bootstrap_sequence".to_string(),
                value: input_sequence,
            },
            output_checkpoint: crate::runtime::contracts::WorkerCheckpoint {
                name: "agent_bootstrap_sequence".to_string(),
                value: output_sequence,
            },
            items_attempted,
            items_committed,
            retryable_errors,
            fatal_errors,
            budget_exhausted,
        }
    }
}

fn bootstrap_worker_issue(
    bootstrap_id: &str,
    error: meld_world_model::AgentBootstrapError,
) -> (
    meld_world_model::AgentBootstrapErrorClass,
    crate::runtime::contracts::WorkerTickIssue,
) {
    let classification = error.classification();
    let issue = crate::runtime::contracts::WorkerTickIssue {
        item_id: Some(bootstrap_id.to_string()),
        code: error.diagnostic_code().to_string(),
        message: bounded_bootstrap_error(&error),
    };
    (classification, issue)
}

fn bounded_bootstrap_error(error: &meld_world_model::AgentBootstrapError) -> String {
    const MAX_BYTES: usize = 512;
    let message = error.to_string();
    if message.len() <= MAX_BYTES {
        return message;
    }
    let mut boundary = MAX_BYTES;
    while !message.is_char_boundary(boundary) {
        boundary -= 1;
    }
    format!("{}...", &message[..boundary])
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
    let disabled_ids = canonical_runtime_selection(registry, disabled_runtime_ids)?;
    let enabled_ids = if enabled_runtime_ids.is_empty() {
        registry
            .descriptors()
            .filter(|descriptor| descriptor.default_enabled)
            .map(|descriptor| descriptor.runtime_id.clone())
            .filter(|runtime_id| !disabled_ids.contains(runtime_id))
            .collect::<BTreeSet<_>>()
    } else {
        canonical_runtime_selection(registry, enabled_runtime_ids)?
    };
    if let Some(overlap) = enabled_ids.intersection(&disabled_ids).next() {
        return Err(RuntimeAssemblyError::Config(format!(
            "runtime id '{overlap}' cannot be both enabled and disabled"
        )));
    }

    let runtime_ids = registry
        .descriptors()
        .map(|descriptor| descriptor.runtime_id.clone())
        .chain(enabled_ids.iter().cloned())
        .chain(disabled_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    let mut states = runtime_ids
        .into_iter()
        .map(|runtime_id| {
            let descriptor = registry.get(&runtime_id);
            let enabled = enabled_ids.contains(&runtime_id) && !disabled_ids.contains(&runtime_id);
            DesiredRuntimeState {
                factory_available: descriptor.is_some(),
                role_class: descriptor
                    .map(|descriptor| descriptor.role_class)
                    .unwrap_or(RuntimeRoleClass::Unknown),
                implementation_state: descriptor
                    .map(|descriptor| descriptor.implementation_state)
                    .unwrap_or(RuntimeImplementationState::Unavailable),
                runtime_id,
                enabled,
            }
        })
        .collect::<Vec<_>>();
    states.sort_by(|left, right| left.runtime_id.cmp(&right.runtime_id));
    Ok(states)
}

fn canonical_runtime_selection(
    registry: &RuntimeFactoryRegistry,
    runtime_ids: Vec<String>,
) -> Result<BTreeSet<String>, RuntimeAssemblyError> {
    let mut canonical_ids = BTreeSet::new();
    for runtime_id in runtime_ids {
        let canonical_id = registry
            .canonical_id(&runtime_id)
            .map(str::to_string)
            .unwrap_or(runtime_id);
        if !canonical_ids.insert(canonical_id.clone()) {
            return Err(RuntimeRegistryError::DuplicateRuntimeId(canonical_id).into());
        }
    }
    Ok(canonical_ids)
}

fn provider_required(
    registry: &RuntimeFactoryRegistry,
    desired_runtime_state: &[DesiredRuntimeState],
) -> bool {
    desired_runtime_state.iter().any(|state| {
        state.tick_eligible()
            && registry
                .get(&state.runtime_id)
                .is_some_and(|descriptor| descriptor.requires_resource(RuntimeResource::Provider))
    })
}

fn validate_runtime_id(runtime_id: &str) -> Result<(), RuntimeRegistryError> {
    crate::runtime::supervisor::contracts::validate_runtime_id(runtime_id)
        .map_err(|_| RuntimeRegistryError::InvalidRuntimeId(runtime_id.to_string()))
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use meld_events::EventEnvelope;
    use meld_execution::goals::GoalCommandOutcome;
    use meld_execution::task_network::EventAppendSink;
    use meld_lang::{Goal, GoalLifecycle, GoalPriority, GoalSource, Proposition, Term};
    use meld_world_model::PerspectiveKey;
    use proptest::prelude::*;

    use super::*;
    use crate::runtime::error::RuntimePortError;

    fn reopen_assembly_after_lock_release(path: &Path) -> ProductRuntimeAssembly {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match ProductRuntimeAssembly::load_for_product_root(path) {
                Ok(assembly) => return assembly,
                Err(RuntimeAssemblyError::PortConstruction(message))
                    if message.contains("could not acquire lock") && Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(error) => panic!("failed to reopen product runtime assembly: {error}"),
            }
        }
    }

    #[test]
    fn bootstrap_worker_issue_preserves_retry_class_code_and_message_bound() {
        let storage = meld_world_model::AgentBootstrapError::Storage {
            message: "x".repeat(600),
        };
        let (storage_class, storage_issue) = bootstrap_worker_issue("bootstrap-a", storage);
        assert_eq!(
            storage_class,
            meld_world_model::AgentBootstrapErrorClass::Retryable
        );
        assert_eq!(storage_issue.item_id.as_deref(), Some("bootstrap-a"));
        assert_eq!(storage_issue.code, "agent_bootstrap_storage_failed");
        assert!(storage_issue
            .message
            .starts_with("bootstrap storage failure"));
        assert!(storage_issue.message.len() <= 515);

        let conflict = meld_world_model::AgentBootstrapError::Conflict {
            field: "bootstrap.activation_hash".to_string(),
            configured_value_hash: "configured".to_string(),
            durable_value_hash: "durable".to_string(),
        };
        let (conflict_class, conflict_issue) = bootstrap_worker_issue("bootstrap-a", conflict);
        assert_eq!(
            conflict_class,
            meld_world_model::AgentBootstrapErrorClass::Fatal
        );
        assert_eq!(conflict_issue.code, "agent_bootstrap_conflict");
        assert_eq!(
            conflict_issue.message,
            "bootstrap content conflict at 'bootstrap.activation_hash'"
        );
    }

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
        assert_eq!(assembly.registry().len(), 15);
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
        assert_eq!(
            task_dispatch.implementation_state,
            RuntimeImplementationState::Inert
        );
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
        assert_eq!(description.desired_runtime_state.len(), 15);
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

        let second = reopen_assembly_after_lock_release(temp.path());

        assert_eq!(second.product_root(), temp.path());
        assert!(second.registry().contains("execution.publication"));
        assert_eq!(second.desired_runtime_state().len(), 15);
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
            1
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
    fn ordinary_assembly_rejects_enabled_bootstrap_before_store_creation() {
        let temp = tempfile::tempdir().unwrap();
        let product_root = temp.path().join("product");
        let mut config = ProductRuntimeConfig::for_product_root(&product_root);
        config.enabled_runtime_ids = vec![
            "world_model.graph_replay".to_string(),
            "world_model.agent.bootstrap.docs_freshness".to_string(),
        ];

        let error = match ProductRuntimeAssembly::load(config) {
            Ok(_) => panic!("bootstrap without activation input should fail"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            RuntimeAssemblyError::RuntimeHandleConstruction(message)
                if message.contains("requires activated world-model owner input")
        ));
        assert!(!product_root.exists());
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
    fn enabled_inert_runtime_does_not_require_provider() {
        let temp = tempfile::tempdir().unwrap();
        let mut config = ProductRuntimeConfig::for_product_root(temp.path());
        config.enabled_runtime_ids = vec!["execution.task_dispatch".to_string()];
        config.disabled_runtime_ids = Vec::new();

        let assembly = ProductRuntimeAssembly::load(config).unwrap();
        let dispatch = assembly
            .desired_runtime_state()
            .iter()
            .find(|state| state.runtime_id == "execution.task_dispatch")
            .unwrap();

        assert!(dispatch.enabled);
        assert!(dispatch.factory_available);
        assert_eq!(
            dispatch.implementation_state,
            RuntimeImplementationState::Inert
        );
        assert!(!assembly.ports().adapters().provider().is_required());
    }

    #[test]
    fn concrete_provider_actor_requires_provider_availability() {
        let registry =
            RuntimeFactoryRegistry::from_descriptors([RuntimeFactoryDescriptor::classified(
                "execution.provider_test",
                &[],
                RuntimeRoleClass::Actor,
                RuntimeImplementationState::Concrete,
                true,
                vec![RuntimeResource::Provider],
            )
            .unwrap()])
            .unwrap();
        let desired = desired_runtime_state(&registry, Vec::new(), Vec::new()).unwrap();

        assert!(provider_required(&registry, &desired));
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
        assert_eq!(package.handle_factories.len(), 15);
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
    fn first_proof_registry_has_one_default_concrete_actor() {
        let registry = RuntimeFactoryRegistry::first_proof_registry().unwrap();
        let default_actors = registry
            .descriptors()
            .filter(|descriptor| descriptor.default_enabled)
            .collect::<Vec<_>>();

        assert_eq!(registry.len(), 15);
        assert_eq!(default_actors.len(), 1);
        assert_eq!(default_actors[0].runtime_id, "world_model.graph_replay");
        assert_eq!(default_actors[0].role_class, RuntimeRoleClass::Actor);
        assert_eq!(
            default_actors[0].implementation_state,
            RuntimeImplementationState::Concrete
        );
        let bootstrap = registry
            .get("world_model.agent.bootstrap.docs_freshness")
            .unwrap();
        assert_eq!(bootstrap.role_class, RuntimeRoleClass::Actor);
        assert_eq!(
            bootstrap.implementation_state,
            RuntimeImplementationState::Concrete
        );
        assert!(!bootstrap.default_enabled);
    }

    #[test]
    fn ingress_aliases_resolve_to_canonical_persisted_ids() {
        let registry = RuntimeFactoryRegistry::first_proof_registry().unwrap();

        for descriptor in registry.descriptors() {
            for alias in &descriptor.aliases {
                assert_eq!(
                    crate::runtime::supervisor::RuntimeId::new(alias.clone())
                        .unwrap()
                        .as_str(),
                    descriptor.runtime_id
                );
            }
        }

        assert_eq!(
            registry.canonical_id("world_model.graph.replay"),
            Some("world_model.graph_replay")
        );
        assert_eq!(
            registry.canonical_id("execution.task.dispatch"),
            Some("execution.task_dispatch")
        );
        assert_eq!(registry.canonical_id("events.ledger"), Some("event.append"));
    }

    #[test]
    fn alias_collisions_fail_registry_construction() {
        let canonical_collision = RuntimeFactoryRegistry::from_descriptors([
            RuntimeFactoryDescriptor::classified(
                "execution.one",
                &["execution.two"],
                RuntimeRoleClass::Actor,
                RuntimeImplementationState::Inert,
                false,
                Vec::new(),
            )
            .unwrap(),
            RuntimeFactoryDescriptor::new("execution.two", Vec::new()).unwrap(),
        ])
        .unwrap_err();
        assert!(matches!(
            canonical_collision,
            RuntimeRegistryError::RuntimeAliasCollision(_)
        ));

        let duplicate_alias = RuntimeFactoryRegistry::from_descriptors([
            RuntimeFactoryDescriptor::classified(
                "execution.one",
                &["execution.legacy"],
                RuntimeRoleClass::Actor,
                RuntimeImplementationState::Inert,
                false,
                Vec::new(),
            )
            .unwrap(),
            RuntimeFactoryDescriptor::classified(
                "execution.two",
                &["execution.legacy"],
                RuntimeRoleClass::Actor,
                RuntimeImplementationState::Inert,
                false,
                Vec::new(),
            )
            .unwrap(),
        ])
        .unwrap_err();
        assert!(matches!(
            duplicate_alias,
            RuntimeRegistryError::DuplicateRuntimeAlias(_)
        ));
    }

    #[test]
    fn registry_rejects_malformed_literal_aliases() {
        let descriptor = RuntimeFactoryDescriptor {
            runtime_id: "execution.one".to_string(),
            required_resources: Vec::new(),
            aliases: vec!["Execution Legacy".to_string()],
            role_class: RuntimeRoleClass::Actor,
            implementation_state: RuntimeImplementationState::Inert,
            default_enabled: false,
        };

        let error = RuntimeFactoryRegistry::from_descriptors([descriptor]).unwrap_err();

        assert!(matches!(error, RuntimeRegistryError::InvalidRuntimeId(_)));
    }

    #[test]
    fn only_concrete_actors_can_be_default_enabled() {
        for (role_class, implementation_state) in [
            (RuntimeRoleClass::Actor, RuntimeImplementationState::Inert),
            (
                RuntimeRoleClass::PassiveService,
                RuntimeImplementationState::Concrete,
            ),
            (
                RuntimeRoleClass::PortOnly,
                RuntimeImplementationState::Concrete,
            ),
        ] {
            let error = RuntimeFactoryDescriptor::classified(
                "execution.invalid_default",
                &[],
                role_class,
                implementation_state,
                true,
                Vec::new(),
            )
            .unwrap_err();
            assert!(matches!(
                error,
                RuntimeRegistryError::InvalidDefaultRuntimeRole(_)
            ));
        }

        let literal = RuntimeFactoryDescriptor {
            runtime_id: "execution.literal_default".to_string(),
            required_resources: Vec::new(),
            aliases: Vec::new(),
            role_class: RuntimeRoleClass::PortOnly,
            implementation_state: RuntimeImplementationState::Concrete,
            default_enabled: true,
        };
        assert!(matches!(
            RuntimeFactoryRegistry::from_descriptors([literal]).unwrap_err(),
            RuntimeRegistryError::InvalidDefaultRuntimeRole(_)
        ));
    }

    #[test]
    fn hosting_and_ticking_are_distinct_runtime_eligibility_contracts() {
        let state = |role_class, implementation_state| DesiredRuntimeState {
            runtime_id: "execution.role".to_string(),
            enabled: true,
            factory_available: true,
            role_class,
            implementation_state,
        };

        let actor = state(
            RuntimeRoleClass::Actor,
            RuntimeImplementationState::Concrete,
        );
        assert!(actor.host_eligible());
        assert!(actor.tick_eligible());

        let service = state(
            RuntimeRoleClass::PassiveService,
            RuntimeImplementationState::Concrete,
        );
        assert!(service.host_eligible());
        assert!(!service.tick_eligible());

        let port = state(
            RuntimeRoleClass::PortOnly,
            RuntimeImplementationState::Concrete,
        );
        assert!(!port.host_eligible());
        assert!(!port.tick_eligible());

        let inert = state(RuntimeRoleClass::Actor, RuntimeImplementationState::Inert);
        assert!(!inert.host_eligible());
        assert!(!inert.tick_eligible());
    }

    #[test]
    fn configured_alias_emits_only_the_canonical_runtime_id() {
        let registry = RuntimeFactoryRegistry::first_proof_registry().unwrap();
        let desired = desired_runtime_state(
            &registry,
            vec!["execution.task.dispatch".to_string()],
            Vec::new(),
        )
        .unwrap();

        assert_eq!(desired.len(), 15);
        assert!(desired
            .iter()
            .any(|state| state.runtime_id == "execution.task_dispatch" && state.enabled));
        assert!(!desired
            .iter()
            .any(|state| state.runtime_id == "execution.task.dispatch"));
    }

    #[test]
    fn canonical_and_alias_selection_is_rejected_as_duplicate_ingress() {
        let registry = RuntimeFactoryRegistry::first_proof_registry().unwrap();
        let error = desired_runtime_state(
            &registry,
            vec![
                "world_model.graph_replay".to_string(),
                "world_model.graph.replay".to_string(),
            ],
            Vec::new(),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            RuntimeAssemblyError::RuntimeRegistry(RuntimeRegistryError::DuplicateRuntimeId(
                runtime_id
            )) if runtime_id == "world_model.graph_replay"
        ));
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
                        ch.is_ascii_lowercase()
                            || ch.is_ascii_digit()
                            || ch == '.'
                            || ch == '-'
                            || ch == '_'
                    });
                prop_assert_eq!(result.is_ok(), expected);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn duplicate_enabled_and_disabled_runtime_ids_fail_selection() {
        let enabled_temp = tempfile::tempdir().unwrap();
        let mut enabled = ProductRuntimeConfig::for_product_root(enabled_temp.path());
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

        let disabled_temp = tempfile::tempdir().unwrap();
        let mut disabled = ProductRuntimeConfig::for_product_root(disabled_temp.path());
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

        let second = reopen_assembly_after_lock_release(temp.path());
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
            .append_envelope_idempotent(
                meld_events::EventEnvelope::with_now(
                    "session-a",
                    "session.tick",
                    serde_json::json!({}),
                )
                .with_record_id("runtime-assembly-status-tick"),
            )
            .unwrap();
        let second = handle.tick(WorkBudget { max_items: 8 }).unwrap();
        assert!(second.output_checkpoint.value > second.input_checkpoint.value);
        assert!(second.made_progress());
        assert_eq!(second.items_committed, 0);
    }
}
