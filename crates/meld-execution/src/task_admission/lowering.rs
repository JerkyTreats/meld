//! Direct lowering of durable Agent-authorized Task admissions.

use std::collections::{BTreeMap, BTreeSet};

use super::admission::{exact_contract_binding, TaskAdmissionDecision, TaskAdmissionRecord};
use crate::capability::{
    BoundBindingValue, BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource,
    CapabilityCatalog, CapabilityTypeContract, InputCardinality,
};
use crate::task::{TaskCompiler, TaskDefinition, TaskDefinitionCompiler, TaskRunContext};
use crate::task_network::{
    command,
    contracts::stable_id,
    mutation,
    state::{
        DependencyEdge, DependencyEdgeOrigin, DependencyKind, StaticSeedInitSource,
        TaskAdmissionAttribution, TaskInitSource, TaskLineage, TaskNode,
        UpstreamArtifactInitSource,
    },
    InMemoryTaskNetworkStore, SledTaskNetworkStore,
};
use crate::waiting::{conditions, StructuralWakeAddress, WaitingOnDeclaration};
use meld_lang::{Bindings, EdgeKind, StepKind, Term};
use serde::{Deserialize, Serialize};

/// One deterministic direct-lowering output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskAdmissionLoweringPlan {
    /// Exact admission selected for lowering.
    pub admission_id: String,
    /// Target unified Task Network.
    pub network_id: String,
    /// Proposed graph mutation set.
    pub mutations: mutation::Set,
    /// Blocking consumer diagnostics, empty for a valid plan.
    pub diagnostics: Vec<String>,
}

/// Region write accepted only from the canonical admitted-Task lowerer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidatedTaskRegionWrite {
    request: command::Request,
}

impl ValidatedTaskRegionWrite {
    fn new(request: command::Request) -> Self {
        Self { request }
    }

    pub(crate) fn into_request(self) -> command::Request {
        self.request
    }

    pub(crate) fn request(&self) -> &command::Request {
        &self.request
    }
}

/// Direct lowerer that treats the admitted Task as the sole semantic body.
#[derive(Debug, Clone)]
pub struct TaskAdmissionLowerer<C = TaskCompiler> {
    compiler: C,
    catalog: CapabilityCatalog,
}

impl<C> TaskAdmissionLowerer<C>
where
    C: TaskDefinitionCompiler,
{
    /// Bind one compiler and the exact installed Capability catalog.
    pub fn new(compiler: C, catalog: CapabilityCatalog) -> Self {
        Self { compiler, catalog }
    }

    /// Validate and lower one admitted Task without Method search or repair.
    pub fn lower(
        &self,
        network_id: &str,
        admission: &TaskAdmissionRecord,
    ) -> TaskAdmissionLoweringPlan {
        let mut diagnostics = Vec::new();
        if admission.decision != TaskAdmissionDecision::Admitted {
            diagnostics.push("only admitted Tasks can be lowered".to_string());
            return empty_plan(network_id, admission, diagnostics);
        }
        let task = &admission.request.task;
        let mut resolved = BTreeMap::new();
        for step in &task.composition.steps {
            let StepKind::Op(operator) = &step.kind else {
                diagnostics.push(format!(
                    "Task step '{}' is a recursive Goal and cannot be lowered",
                    step.step_id
                ));
                continue;
            };
            let Some(specific) = &operator.resolution.specific else {
                diagnostics.push(format!(
                    "operator '{}' has no exact Capability reference",
                    operator.operator_id
                ));
                continue;
            };
            let Some(contract) = self
                .catalog
                .get(&specific.capability_type_id, specific.capability_version)
            else {
                diagnostics.push(format!(
                    "Capability '{}' version {} is unavailable",
                    specific.capability_type_id, specific.capability_version
                ));
                continue;
            };
            resolved.insert(
                step.step_id.clone(),
                ResolvedStep {
                    operator_id: operator.operator_id.clone(),
                    contract: contract.clone(),
                },
            );
        }
        if !diagnostics.is_empty() || resolved.is_empty() {
            if resolved.is_empty() && diagnostics.is_empty() {
                diagnostics.push("Task has no executable operator step".to_string());
            }
            return empty_plan(network_id, admission, diagnostics);
        }
        let step_ids = resolved.keys().cloned().collect::<BTreeSet<_>>();
        for edge in &task.composition.edges {
            if !step_ids.contains(&edge.from) || !step_ids.contains(&edge.to) {
                diagnostics.push(format!(
                    "Task edge '{} -> {}' has a non-executable endpoint",
                    edge.from, edge.to
                ));
            }
            if matches!(edge.kind, EdgeKind::Conditional { .. }) {
                diagnostics.push(format!(
                    "Task edge '{} -> {}' uses deferred conditional semantics",
                    edge.from, edge.to
                ));
            }
        }
        if !diagnostics.is_empty() {
            return empty_plan(network_id, admission, diagnostics);
        }

        let mut mutations = Vec::new();
        for step in &task.composition.steps {
            let Some(resolved_step) = resolved.get(&step.step_id) else {
                continue;
            };
            match self.lower_step(network_id, admission, step, resolved_step, &resolved) {
                Ok(node) => {
                    let incoming = incoming_edges(network_id, admission, &step.step_id);
                    mutations.push(mutation::Mutation::Inject(mutation::Inject::new(
                        node, incoming,
                    )));
                }
                Err(error) => diagnostics.push(error),
            }
        }
        if !diagnostics.is_empty() {
            return empty_plan(network_id, admission, diagnostics);
        }
        TaskAdmissionLoweringPlan {
            admission_id: admission.admission_id.clone(),
            network_id: network_id.to_string(),
            mutations: mutation::Set::new(
                network_id,
                &task.task_id,
                format!("lower::{}", admission.admission_id),
                mutations,
            ),
            diagnostics,
        }
    }

    fn lower_step(
        &self,
        network_id: &str,
        admission: &TaskAdmissionRecord,
        step: &meld_lang::Step,
        resolved: &ResolvedStep,
        all_resolved: &BTreeMap<String, ResolvedStep>,
    ) -> Result<TaskNode, String> {
        let task = &admission.request.task;
        let task_instance_id = task_instance_id(network_id, &admission.admission_id, &step.step_id);
        let (init_slots, init_sources) =
            exact_input_sources(network_id, admission, &step.step_id, resolved, all_resolved)?;
        let input_wiring = init_slots
            .iter()
            .map(|slot| BoundInputWiring {
                slot_id: slot.init_slot_id.clone(),
                sources: vec![BoundInputWiringSource::TaskInitSlot {
                    init_slot_id: slot.init_slot_id.clone(),
                    artifact_type_id: slot.artifact_type_id.clone(),
                    schema_version: slot.schema_version,
                }],
            })
            .collect();
        let binding_values = resolved
            .contract
            .binding_contract
            .iter()
            .filter_map(|binding| {
                match exact_contract_binding(&task.bindings, &binding.binding_id) {
                    Ok(Some(term)) => Some(
                        serde_json::to_value(term)
                            .map(|value| BoundBindingValue {
                                binding_id: binding.binding_id.clone(),
                                value,
                            })
                            .map_err(|error| {
                                format!("Task binding '{}' is invalid: {error}", binding.binding_id)
                            }),
                    ),
                    Ok(None) if binding.required => Some(Err(format!(
                        "required binding '{}' is absent from the admitted Task",
                        binding.binding_id
                    ))),
                    Ok(None) => None,
                    Err(error) => Some(Err(error)),
                }
            })
            .collect::<Result<Vec<_>, String>>()?;
        let definition = TaskDefinition {
            task_id: task_instance_id.clone(),
            task_version: 1,
            init_slots,
            capability_instances: vec![BoundCapabilityInstance {
                capability_instance_id: step.step_id.clone(),
                capability_type_id: resolved.contract.capability_type_id.clone(),
                capability_version: resolved.contract.capability_version,
                scope_ref: scope_ref(&task.bindings, admission)?,
                scope_kind: resolved.contract.scope_contract.scope_kind.clone(),
                binding_values,
                input_wiring,
            }],
        };
        let compiled_task = self
            .compiler
            .compile_task_definition(&definition, &self.catalog)
            .map_err(|error| {
                format!(
                    "operator '{}' failed compilation: {error}",
                    resolved.operator_id
                )
            })?;
        let attribution = TaskAdmissionAttribution::from_record(admission);
        Ok(TaskNode {
            task_instance_id: task_instance_id.clone(),
            lifecycle_epoch: 0,
            compiled_task,
            init_sources,
            task_run_context: TaskRunContext {
                task_run_id: stable_id(
                    "task-admission-run",
                    &(&admission.admission_id, &step.step_id),
                ),
                session_id: None,
                trigger: "agent_task_admission".to_string(),
            },
            lineage: TaskLineage::admitted(
                step.step_id.clone(),
                resolved.operator_id.clone(),
                resolved.contract.capability_type_id.clone(),
                resolved.contract.capability_version,
                admission.request.lineage.authority_decision.clone(),
                attribution,
            ),
        })
    }
}

#[derive(Debug, Clone)]
struct ResolvedStep {
    operator_id: String,
    contract: CapabilityTypeContract,
}

/// Request for one bounded direct Task admission pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskAdmissionRuntimeRequest {
    /// Target unified network.
    pub network_id: String,
    /// Maximum admissions to attempt.
    pub max_items: usize,
}

/// Result for one selected admission.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskAdmissionRuntimeItem {
    /// Exact admission identity.
    pub admission_id: String,
    /// Lowering or command response.
    pub result: Result<command::Response, Vec<String>>,
}

/// Truthful bounded report for direct admitted-Task realization.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskAdmissionRuntimeReport {
    /// Network revision before selection.
    pub input_revision: u64,
    /// Network revision after work.
    pub output_revision: u64,
    /// Admissions attempted.
    pub attempted: usize,
    /// Mutation commands accepted or replayed.
    pub committed: usize,
    /// True when more admitted unlowered Tasks remain.
    pub budget_exhausted: bool,
    /// Per-admission results.
    pub items: Vec<TaskAdmissionRuntimeItem>,
    /// Owner-authored conditions that can make admission eligible again.
    pub waiting_on: Vec<WaitingOnDeclaration>,
}

/// The one root Execution Task admission participant for Agent Tasks.
pub struct TaskAdmissionRuntimeActor<C = TaskCompiler> {
    lowerer: TaskAdmissionLowerer<C>,
    lifecycle: crate::lifecycle::NativeLifecycle,
}

impl<C> TaskAdmissionRuntimeActor<C>
where
    C: TaskDefinitionCompiler,
{
    /// Bind direct lowering to the Task admission participant.
    pub fn new(lowerer: TaskAdmissionLowerer<C>) -> Self {
        Self {
            lowerer,
            lifecycle: crate::lifecycle::NativeLifecycle::new("execution.task_admission"),
        }
    }

    /// Resolve the durable admissions stream for the network this owner observes.
    pub fn resolves_wake(
        &self,
        network: &SledTaskNetworkStore,
        wake: &crate::waiting::StructuralWakeAddress,
    ) -> bool {
        matches!(wake, crate::waiting::StructuralWakeAddress::OwnerRevision(value)
            if crate::waiting::after_position(value, &format!("agent-admissions::{}", network.state().network_id)))
    }

    /// Account for the native durable work in the borrowed, exclusively bound network.
    pub fn lifecycle_evidence(
        &self,
        network: &crate::task_network::store::SledTaskNetworkStore,
    ) -> Result<crate::lifecycle::NativeLifecycleEvidence, String> {
        network.flush().map_err(|error| error.to_string())?;
        let state = network.state();
        let pending: Vec<_> = state
            .admissions
            .values()
            .filter(|admission| {
                admission.decision == TaskAdmissionDecision::Admitted
                    && !state.tasks.values().any(|node| {
                        node.lineage.admission.as_ref().is_some_and(|attribution| {
                            attribution.admission_id == admission.admission_id
                        })
                    })
            })
            .collect();
        let contracts: Vec<_> = self.lowerer.catalog.iter().collect();
        let checkpoint_ref = format!("task-network::{}::{}", state.network_id, state.revision);
        Ok(crate::lifecycle::NativeLifecycleEvidence {
            checkpoint_ref: checkpoint_ref.clone(),
            installed_revision_refs: vec![crate::lifecycle::evidence_ref(
                "admission-capability-catalog",
                &contracts,
            )?],
            binding_refs: vec![format!("task-network::{}", state.network_id)],
            subscription_refs: vec![format!("agent-admissions::{}", state.network_id)],
            proof_position_ref: checkpoint_ref,
            unresolved_operation_summary_ref: crate::lifecycle::evidence_ref(
                "admission-pending-regions",
                &pending,
            )?,
        })
    }

    /// Author native start evidence while the network is borrowed.
    pub fn lifecycle_start(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
        network: &crate::task_network::store::SledTaskNetworkStore,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let evidence = self.lifecycle_evidence(network)?;
        let transition = self
            .lifecycle
            .start(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native safe point evidence while the network is borrowed.
    pub fn lifecycle_safe_point(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
        network: &crate::task_network::store::SledTaskNetworkStore,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let evidence = self.lifecycle_evidence(network)?;
        let transition = self
            .lifecycle
            .safe_point(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native stop evidence while the network is borrowed.
    pub fn lifecycle_stop(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
        network: &crate::task_network::store::SledTaskNetworkStore,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let evidence = self.lifecycle_evidence(network)?;
        let transition = self
            .lifecycle
            .stop(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Author native release evidence while the network is borrowed.
    pub fn lifecycle_release(
        &self,
        identity: crate::lifecycle::NativeLifecycleIdentity,
        network: &crate::task_network::store::SledTaskNetworkStore,
    ) -> Result<
        (
            crate::lifecycle::NativeLifecycleEvidence,
            crate::lifecycle::NativeLifecycleTransition,
        ),
        String,
    > {
        let evidence = self.lifecycle_evidence(network)?;
        let transition = self
            .lifecycle
            .release(identity, evidence.proof_position_ref.clone())?;
        Ok((evidence, transition))
    }

    /// Select durable admissions and commit distinct operational regions.
    pub fn run_once(
        &self,
        store: &mut SledTaskNetworkStore,
        request: TaskAdmissionRuntimeRequest,
    ) -> Result<TaskAdmissionRuntimeReport, String> {
        self.run_once_store(store, request)
    }

    /// Run the same canonical lowering boundary against an in-memory store.
    pub fn run_once_in_memory(
        &self,
        store: &mut InMemoryTaskNetworkStore,
        request: TaskAdmissionRuntimeRequest,
    ) -> Result<TaskAdmissionRuntimeReport, String> {
        self.run_once_store(store, request)
    }

    fn run_once_store<S>(
        &self,
        store: &mut S,
        request: TaskAdmissionRuntimeRequest,
    ) -> Result<TaskAdmissionRuntimeReport, String>
    where
        S: TaskAdmissionRegionStore,
    {
        if request.max_items == 0 || request.network_id != store.network_state().network_id {
            return Err("direct Task admission request is invalid".to_string());
        }
        let input_revision = store.network_state().revision;
        let mut pending = store
            .network_state()
            .admissions
            .values()
            .filter(|admission| admission.decision == TaskAdmissionDecision::Admitted)
            .filter(|admission| {
                !store.network_state().tasks.values().any(|node| {
                    node.lineage.admission.as_ref().is_some_and(|attribution| {
                        attribution.admission_id == admission.admission_id
                    })
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        pending.sort_by(|left, right| left.admission_id.cmp(&right.admission_id));
        let budget_exhausted = pending.len() > request.max_items;
        pending.truncate(request.max_items);
        let mut items = Vec::new();
        let mut committed = 0;
        for admission in pending {
            let plan = self.lowerer.lower(&request.network_id, &admission);
            if !plan.diagnostics.is_empty() || plan.mutations.mutations.is_empty() {
                items.push(TaskAdmissionRuntimeItem {
                    admission_id: admission.admission_id,
                    result: Err(plan.diagnostics),
                });
                continue;
            }
            let state = store.network_state();
            let response =
                store.submit_task_region(ValidatedTaskRegionWrite::new(command::Request {
                    command_id: format!("task-lowering::{}", admission.admission_id),
                    network_id: request.network_id.clone(),
                    base_revision: state.revision,
                    base_state_hash: state.state_hash.clone(),
                    read_preconditions: plan
                        .mutations
                        .mutations
                        .iter()
                        .map(|mutation| match mutation {
                            mutation::Mutation::Inject(inject) => {
                                mutation::ReadPrecondition::NodeAbsent(
                                    inject.task_node.task_instance_id.clone(),
                                )
                            }
                        })
                        .collect(),
                    command: command::Command::ApplyMutationSet(plan.mutations),
                }))?;
            if matches!(
                response,
                command::Response::Accepted { .. } | command::Response::Duplicate { .. }
            ) {
                committed += 1;
            }
            items.push(TaskAdmissionRuntimeItem {
                admission_id: admission.admission_id,
                result: Ok(response),
            });
        }
        let waiting_on = if items.is_empty() {
            vec![WaitingOnDeclaration::broad(
                conditions::NO_ACTIVE_GOALS,
                format!("no admitted unlowered Task at network revision {input_revision}"),
                vec![StructuralWakeAddress::OwnerRevision(format!(
                    "agent-admissions::{}::after::{input_revision}",
                    request.network_id
                ))],
            )]
        } else {
            Vec::new()
        };
        Ok(TaskAdmissionRuntimeReport {
            input_revision,
            output_revision: store.network_state().revision,
            attempted: items.len(),
            committed,
            budget_exhausted,
            items,
            waiting_on,
        })
    }
}

trait TaskAdmissionRegionStore {
    fn network_state(&self) -> &crate::task_network::state::NetworkState;
    fn submit_task_region(
        &mut self,
        request: ValidatedTaskRegionWrite,
    ) -> Result<command::Response, String>;
}

impl TaskAdmissionRegionStore for SledTaskNetworkStore {
    fn network_state(&self) -> &crate::task_network::state::NetworkState {
        self.state()
    }

    fn submit_task_region(
        &mut self,
        request: ValidatedTaskRegionWrite,
    ) -> Result<command::Response, String> {
        self.submit_validated_task_region(request)
            .map_err(|error| error.to_string())
    }
}

impl TaskAdmissionRegionStore for InMemoryTaskNetworkStore {
    fn network_state(&self) -> &crate::task_network::state::NetworkState {
        self.state()
    }

    fn submit_task_region(
        &mut self,
        request: ValidatedTaskRegionWrite,
    ) -> Result<command::Response, String> {
        Ok(self.submit_validated_task_region(request))
    }
}

fn empty_plan(
    network_id: &str,
    admission: &TaskAdmissionRecord,
    diagnostics: Vec<String>,
) -> TaskAdmissionLoweringPlan {
    TaskAdmissionLoweringPlan {
        admission_id: admission.admission_id.clone(),
        network_id: network_id.to_string(),
        mutations: mutation::Set::empty(
            network_id,
            &admission.request.task.task_id,
            format!("lower::{}", admission.admission_id),
        ),
        diagnostics,
    }
}

fn incoming_edges(
    network_id: &str,
    admission: &TaskAdmissionRecord,
    step_id: &str,
) -> Vec<DependencyEdge> {
    admission
        .request
        .task
        .composition
        .edges
        .iter()
        .filter(|edge| edge.to == step_id)
        .filter_map(|edge| {
            let kind = match &edge.kind {
                EdgeKind::Ordering => DependencyKind::Ordering,
                EdgeKind::DataFlow {
                    artifact_type: Term::ArtifactType(artifact_type_id),
                } => DependencyKind::DataFlow {
                    artifact_type_id: artifact_type_id.clone(),
                },
                EdgeKind::DataFlow { .. } | EdgeKind::Conditional { .. } => return None,
            };
            Some(DependencyEdge {
                from: task_instance_id(network_id, &admission.admission_id, &edge.from),
                to: task_instance_id(network_id, &admission.admission_id, &edge.to),
                kind,
                origin: DependencyEdgeOrigin::Semantic,
            })
        })
        .collect()
}

fn exact_input_sources(
    network_id: &str,
    admission: &TaskAdmissionRecord,
    step_id: &str,
    consumer: &ResolvedStep,
    resolved: &BTreeMap<String, ResolvedStep>,
) -> Result<(Vec<crate::task::TaskInitSlotSpec>, Vec<TaskInitSource>), String> {
    let edges = &admission.request.task.composition.edges;
    let mut selected = BTreeMap::new();
    for input in admission
        .request
        .task
        .initial_inputs
        .iter()
        .filter(|input| input.step_id == step_id)
    {
        input.validate()?;
        let slot = consumer
            .contract
            .input_contract
            .iter()
            .find(|slot| slot.slot_id == input.slot_id)
            .ok_or_else(|| "frozen Task input names an unknown Capability slot".to_string())?;
        if !slot
            .accepted_artifact_type_ids
            .contains(&input.artifact_type_id)
            || !slot.schema_versions.accepts(input.schema_version)
        {
            return Err("frozen Task input does not match its Capability schema".into());
        }
        let spec = crate::task::TaskInitSlotSpec {
            init_slot_id: input.slot_id.clone(),
            artifact_type_id: input.artifact_type_id.clone(),
            schema_version: input.schema_version,
            required: slot.required,
        };
        let source = TaskInitSource::StaticSeed(StaticSeedInitSource {
            init_slot_id: input.slot_id.clone(),
            artifact_type_id: input.artifact_type_id.clone(),
            schema_version: input.schema_version,
            content: input.content.clone(),
        });
        if selected
            .insert(input.slot_id.clone(), (spec, source))
            .is_some()
        {
            return Err("several frozen Task inputs target one Capability slot".into());
        }
    }
    for edge in edges.iter().filter(|edge| edge.to == step_id) {
        let EdgeKind::DataFlow {
            artifact_type: Term::ArtifactType(artifact_type_id),
        } = &edge.kind
        else {
            if matches!(edge.kind, EdgeKind::DataFlow { .. }) {
                return Err(format!(
                    "data flow edge '{} -> {}' has an ungrounded artifact type",
                    edge.from, edge.to
                ));
            }
            continue;
        };
        let matching_slots = consumer
            .contract
            .input_contract
            .iter()
            .filter(|slot| {
                slot.accepted_artifact_type_ids
                    .iter()
                    .any(|accepted| accepted == artifact_type_id)
            })
            .collect::<Vec<_>>();
        let [slot] = matching_slots.as_slice() else {
            return Err(format!(
                "data flow artifact '{}' does not identify exactly one input slot for step '{}'",
                artifact_type_id, step_id
            ));
        };
        if slot.cardinality != InputCardinality::One {
            return Err(format!(
                "input slot '{}' uses unsupported data flow cardinality",
                slot.slot_id
            ));
        }
        let upstream = resolved
            .get(&edge.from)
            .ok_or_else(|| format!("data flow source '{}' is not executable", edge.from))?;
        let outputs = upstream
            .contract
            .output_contract
            .iter()
            .filter(|output| output.artifact_type_id == *artifact_type_id)
            .collect::<Vec<_>>();
        let [output] = outputs.as_slice() else {
            return Err(format!(
                "data flow source '{}' does not publish exactly one '{}' output",
                edge.from, artifact_type_id
            ));
        };
        if !slot.schema_versions.accepts(output.schema_version) {
            return Err(format!(
                "data flow source '{}' publishes schema {} outside input slot '{}' range",
                edge.from, output.schema_version, slot.slot_id
            ));
        }
        let spec = crate::task::TaskInitSlotSpec {
            init_slot_id: slot.slot_id.clone(),
            artifact_type_id: artifact_type_id.clone(),
            schema_version: output.schema_version,
            required: slot.required,
        };
        let source = TaskInitSource::UpstreamArtifact(UpstreamArtifactInitSource {
            init_slot_id: slot.slot_id.clone(),
            artifact_type_id: artifact_type_id.clone(),
            schema_version: output.schema_version,
            upstream_task_instance_id: task_instance_id(
                network_id,
                &admission.admission_id,
                &edge.from,
            ),
            upstream_artifact_type_id: artifact_type_id.clone(),
        });
        if selected
            .insert(slot.slot_id.clone(), (spec, source))
            .is_some()
        {
            return Err(format!(
                "several data flow edges target input slot '{}'",
                slot.slot_id
            ));
        }
    }
    let mut slots = Vec::new();
    let mut sources = Vec::new();
    for contract_slot in &consumer.contract.input_contract {
        match selected.remove(&contract_slot.slot_id) {
            Some((slot, source)) => {
                slots.push(slot);
                sources.push(source);
            }
            None if contract_slot.required => {
                return Err(format!(
                    "required input slot '{}' has no exact Task data flow edge",
                    contract_slot.slot_id
                ));
            }
            None => {}
        }
    }
    Ok((slots, sources))
}

fn task_instance_id(network_id: &str, admission_id: &str, step_id: &str) -> String {
    stable_id("task-admission-node", &(network_id, admission_id, step_id))
}

fn scope_ref(bindings: &Bindings, admission: &TaskAdmissionRecord) -> Result<String, String> {
    for candidate in ["?subject", "subject", "?node", "node", "?scope", "scope"] {
        if let Some(term) = bindings.get(candidate) {
            return Ok(term_to_scope_ref(term));
        }
    }
    admission
        .request
        .lineage
        .authority_decision
        .as_ref()
        .map(|decision| decision.subject.object_id.clone())
        .ok_or_else(|| "admitted Task has no exact scope binding or authority subject".to_string())
}

fn term_to_scope_ref(term: &Term) -> String {
    match term {
        Term::Object(object) => object.object_id.clone(),
        Term::ArtifactType(value) | Term::Dimension(value) | Term::Variable(value) => value.clone(),
        Term::Literal(value) => serde_json::to_string(value).expect("literal is serializable"),
        Term::Derived {
            source_step,
            field_path,
        } => format!("{source_step}.{field_path}"),
    }
}
