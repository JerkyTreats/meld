//! Execution composition lowering into task network mutation proposals.
//!
//! Owner: planning lowering.
//! Inputs: one execution composition and a capability catalog.
//! Outputs: a deterministic task network mutation set plus lowering diagnostics.
//! Does not own: this module does not accept task network commands, dispatch
//! tasks, or publish outcomes.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::capability::CapabilityCatalog;
//! use meld_execution::planning::lowering::{Lowerer, Request};
//! use meld_execution::planning::{ExecutionComposition, PlanningWorldStateFrameRef};
//! use meld_execution::task::TaskCompiler;
//! use meld_lang::{Bindings, Composition, Goal, GoalLifecycle, GoalPriority, GoalSource};
//! use meld_lang::{Condition, Proposition, Term, ValidationResult};
//!
//! let goal = Goal {
//!     goal_id: "goal-a".to_string(),
//!     agent_id: "agent-a".to_string(),
//!     target: Proposition::Accessible {
//!         scope: Term::Dimension("workspace".to_string()),
//!     },
//!     priority: GoalPriority {
//!         urgency: 1,
//!         cost_ceiling: None,
//!     },
//!     source: GoalSource::UserDirected {
//!         directive: "inspect".to_string(),
//!     },
//!     lifecycle: GoalLifecycle::Active,
//! };
//! let composition = ExecutionComposition {
//!     composition_id: "composition-a".to_string(),
//!     goal,
//!     world_state_frame: PlanningWorldStateFrameRef {
//!         frame_id: "frame-a".to_string(),
//!         projection_version: "world_model.planner.v1".to_string(),
//!         perspective_id: "default".to_string(),
//!         branch_id: "main".to_string(),
//!         source_refs: vec![],
//!         warnings: vec![],
//!     },
//!     method_id: "method-a".to_string(),
//!     bindings: Bindings::empty(),
//!     composition: Composition {
//!         steps: vec![],
//!         edges: vec![],
//!     },
//!     projected_effects: vec![],
//!     operator_resolutions: vec![],
//!     validation: ValidationResult {
//!         valid: true,
//!         errors: vec![],
//!         warnings: vec![],
//!     },
//!     diagnostics: vec![],
//! };
//! let lowerer = Lowerer::new(TaskCompiler::new(), CapabilityCatalog::new());
//! let plan = lowerer
//!     .lower(Request {
//!         request_id: "request-a".to_string(),
//!         network_id: "network-a".to_string(),
//!         composition,
//!         idempotency_key: "once".to_string(),
//!     })
//!     .expect("empty composition lowers to diagnostics");
//!
//! assert_eq!(plan.network_id, "network-a");
//! ```

use crate::capability::{
    BoundBindingValue, BoundCapabilityInstance, BoundInputWiring, BoundInputWiringSource,
    CapabilityCatalog, CapabilityTypeContract, InputCardinality, InputSlotSpec,
};
use crate::error::ApiError;
use crate::planning::{ExecutionComposition, OperatorResolutionStatus};
use crate::task::{
    CompiledTaskRecord, TaskDefinition, TaskDefinitionCompiler, TaskInitSlotSpec, TaskRunContext,
};
use crate::task_network::{
    contracts::stable_id,
    mutation,
    state::{
        DependencyEdge, DependencyKind, StaticSeedInitSource, TaskInitSource, TaskLineage,
        TaskNode, UpstreamArtifactInitSource,
    },
};
use meld_lang::{Bindings, Edge, EdgeKind, Operator, Step, StepKind, Term};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

/// Lowering request for one execution composition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    /// Caller supplied request id.
    pub request_id: String,
    /// Target task network id.
    pub network_id: String,
    /// Execution composition produced by planning.
    pub composition: ExecutionComposition,
    /// Caller supplied idempotency key for the mutation set.
    pub idempotency_key: String,
}

/// Lowering output ready for task network command acceptance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    /// Caller supplied request id.
    pub request_id: String,
    /// Target task network id.
    pub network_id: String,
    /// Execution composition id.
    pub composition_id: String,
    /// Goal id preserved from the composition.
    pub goal_id: String,
    /// Method id preserved from the composition.
    pub method_id: String,
    /// Proposed mutation set.
    pub mutations: mutation::Set,
    /// Deterministic lowering diagnostics.
    pub diagnostics: Vec<Diagnostic>,
}

/// Deterministic lowering diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Stable diagnostic code.
    pub code: DiagnosticCode,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Composition step related to the diagnostic.
    pub step_id: Option<String>,
    /// Operator related to the diagnostic.
    pub operator_id: Option<String>,
}

impl Diagnostic {
    /// Creates a diagnostic without step or operator context.
    pub fn new(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            step_id: None,
            operator_id: None,
        }
    }

    /// Attaches step context to the diagnostic.
    pub fn with_step(mut self, step_id: impl Into<String>) -> Self {
        self.step_id = Some(step_id.into());
        self
    }

    /// Attaches operator context to the diagnostic.
    pub fn with_operator(mut self, operator_id: impl Into<String>) -> Self {
        self.operator_id = Some(operator_id.into());
        self
    }
}

/// Stable lowering diagnostic code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticCode {
    /// Composition had no operator step to lower.
    NoOperatorStep,
    /// Recursive goal steps are deferred to later slices.
    GoalStepDeferred,
    /// Operator did not have a matching resolution report.
    MissingOperatorResolution,
    /// Operator resolution report was unresolved.
    OperatorUnresolved,
    /// Resolved capability was not present in the catalog supplied to lowering.
    CapabilityContractMissing,
    /// A resolved operator could not compile into a task node.
    TaskCompilationFailed,
    /// An executable edge references a step that is not lowered in this slice.
    MissingExecutableEdgeEndpoint,
    /// A data flow edge does not map to one compiled target init slot.
    DataFlowInitSourceInvalid,
    /// Conditional edge semantics are deferred to later slices.
    ConditionalEdgeDeferred,
}

/// Lowerer from execution compositions to task network mutation sets.
#[derive(Debug, Clone)]
pub struct Lowerer<C> {
    compiler: C,
    catalog: CapabilityCatalog,
}

impl<C> Lowerer<C>
where
    C: TaskDefinitionCompiler,
{
    /// Creates a lowerer with an injectable task definition compiler.
    pub fn new(compiler: C, catalog: CapabilityCatalog) -> Self {
        Self { compiler, catalog }
    }

    /// Lowers one execution composition into a task network mutation proposal.
    pub fn lower(&self, request: Request) -> Result<Plan, ApiError> {
        let mut diagnostics = Vec::new();
        let mut blocking_diagnostic = false;
        for step in &request.composition.composition.steps {
            if matches!(step.kind, StepKind::Goal(_)) {
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::GoalStepDeferred,
                        "recursive goal step lowering is deferred",
                    )
                    .with_step(step.step_id.clone()),
                );
            }
        }

        let operator_steps = request
            .composition
            .composition
            .steps
            .iter()
            .filter_map(|step| match &step.kind {
                StepKind::Op(operator) => Some((step, operator)),
                StepKind::Goal(_) => None,
            })
            .collect::<Vec<_>>();

        if operator_steps.is_empty() {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::NoOperatorStep,
                "composition does not contain an operator step",
            ));
            return Ok(plan_with_mutations(request, Vec::new(), diagnostics));
        };

        let operator_step_ids = operator_steps
            .iter()
            .map(|(step, _)| step.step_id.as_str())
            .collect::<BTreeSet<_>>();
        let mut resolved_steps = Vec::new();
        for (step, operator) in &operator_steps {
            match self.resolve_operator_step(&request, step, operator) {
                Ok(resolved) => resolved_steps.push(resolved),
                Err(diagnostic) => {
                    diagnostics.push(diagnostic);
                    blocking_diagnostic = true;
                }
            }
        }

        validate_executable_edges(
            &request.composition.composition.edges,
            &operator_step_ids,
            &mut diagnostics,
            &mut blocking_diagnostic,
        );

        if blocking_diagnostic {
            return Ok(plan_with_mutations(request, Vec::new(), diagnostics));
        }

        let resolved_by_step = resolved_steps
            .iter()
            .map(|step| (step.step_id.as_str(), step))
            .collect::<BTreeMap<_, _>>();
        let mut drafts = Vec::new();
        for resolved in &resolved_steps {
            match self.lower_resolved_operator_step(&request, resolved, &resolved_by_step) {
                Ok(draft) => drafts.push(draft),
                Err(step_diagnostics) => {
                    diagnostics.extend(step_diagnostics);
                    blocking_diagnostic = true;
                }
            }
        }

        if blocking_diagnostic {
            return Ok(plan_with_mutations(request, Vec::new(), diagnostics));
        }

        let mut mutations = Vec::new();
        for draft in &drafts {
            let incoming_edges =
                incoming_edges(&request.composition, &draft.step_id, &request.network_id);
            let init_sources = init_sources_for_task(
                &request,
                draft,
                &request.composition.composition.edges,
                &mut diagnostics,
                &mut blocking_diagnostic,
            );
            if blocking_diagnostic {
                continue;
            }
            let task_node = TaskNode {
                task_instance_id: draft.task_instance_id.clone(),
                lifecycle_epoch: 0,
                compiled_task: draft.compiled_task.clone(),
                init_sources,
                task_run_context: TaskRunContext {
                    task_run_id: task_run_id(
                        &request.network_id,
                        &request.composition.composition_id,
                        &draft.step_id,
                    ),
                    session_id: None,
                    trigger: "execution_composition_lowering".to_string(),
                },
                lineage: draft.lineage.clone(),
            };
            let inject = mutation::Inject::new(task_node, incoming_edges);
            mutations.push(mutation::Mutation::Inject(inject));
        }

        if blocking_diagnostic {
            return Ok(plan_with_mutations(request, Vec::new(), diagnostics));
        }

        Ok(plan_with_mutations(request, mutations, diagnostics))
    }

    fn resolve_operator_step(
        &self,
        request: &Request,
        step: &Step,
        operator: &Operator,
    ) -> Result<ResolvedOperatorStep, Diagnostic> {
        let resolution = request
            .composition
            .operator_resolutions
            .iter()
            .find(|resolution| resolution.operator_id == operator.operator_id)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::MissingOperatorResolution,
                    "operator has no resolution report",
                )
                .with_step(step.step_id.clone())
                .with_operator(operator.operator_id.clone())
            })?;

        if resolution.status != OperatorResolutionStatus::Resolved {
            return Err(Diagnostic::new(
                DiagnosticCode::OperatorUnresolved,
                "operator resolution report is unresolved",
            )
            .with_step(step.step_id.clone())
            .with_operator(operator.operator_id.clone()));
        }

        let capability_type_id = resolution.capability_type_id.as_deref().ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::CapabilityContractMissing,
                "resolved operator did not name a capability type",
            )
            .with_step(step.step_id.clone())
            .with_operator(operator.operator_id.clone())
        })?;
        let capability_version = resolution.capability_version.ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::CapabilityContractMissing,
                "resolved operator did not name a capability version",
            )
            .with_step(step.step_id.clone())
            .with_operator(operator.operator_id.clone())
        })?;
        let contract = self
            .catalog
            .get(capability_type_id, capability_version)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::CapabilityContractMissing,
                    "resolved capability was not found in the supplied catalog",
                )
                .with_step(step.step_id.clone())
                .with_operator(operator.operator_id.clone())
            })?;

        Ok(ResolvedOperatorStep {
            step_id: step.step_id.clone(),
            operator_id: operator.operator_id.clone(),
            capability_type_id: capability_type_id.to_string(),
            capability_version,
            contract: contract.clone(),
        })
    }

    fn lower_resolved_operator_step(
        &self,
        request: &Request,
        resolved: &ResolvedOperatorStep,
        resolved_by_step: &BTreeMap<&str, &ResolvedOperatorStep>,
    ) -> Result<LoweredTaskDraft, Vec<Diagnostic>> {
        let task_instance_id = task_instance_id(
            &request.network_id,
            &request.composition.composition_id,
            &resolved.step_id,
        );
        let init_slots = init_slots_for_operator(request, resolved, resolved_by_step)?;
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
            .collect::<Vec<_>>();
        let binding_values = resolved
            .contract
            .binding_contract
            .iter()
            .filter(|binding| binding.required)
            .map(|binding| BoundBindingValue {
                binding_id: binding.binding_id.clone(),
                value: binding_value(&request.composition.bindings, &binding.binding_id),
            })
            .collect::<Vec<_>>();
        let definition = TaskDefinition {
            task_id: task_instance_id.clone(),
            task_version: 1,
            init_slots,
            capability_instances: vec![BoundCapabilityInstance {
                capability_instance_id: resolved.step_id.clone(),
                capability_type_id: resolved.capability_type_id.clone(),
                capability_version: resolved.capability_version,
                scope_ref: scope_ref(
                    &request.composition.bindings,
                    &request.composition.goal.goal_id,
                ),
                scope_kind: resolved.contract.scope_contract.scope_kind.clone(),
                binding_values,
                input_wiring,
            }],
        };
        let compiled_task = self
            .compiler
            .compile_task_definition(&definition, &self.catalog)
            .map_err(|error| {
                Diagnostic::new(
                    DiagnosticCode::TaskCompilationFailed,
                    format!("operator task compilation failed: {error}"),
                )
                .with_step(resolved.step_id.clone())
                .with_operator(resolved.operator_id.clone())
            })
            .map_err(|diagnostic| vec![diagnostic])?;
        let lineage = TaskLineage {
            composition_id: request.composition.composition_id.clone(),
            goal_id: request.composition.goal.goal_id.clone(),
            method_id: request.composition.method_id.clone(),
            step_id: resolved.step_id.clone(),
            operator_id: resolved.operator_id.clone(),
            world_state_frame_id: request.composition.world_state_frame.frame_id.clone(),
            capability_type_id: resolved.capability_type_id.clone(),
            capability_version: resolved.capability_version,
        };

        Ok(LoweredTaskDraft {
            step_id: resolved.step_id.clone(),
            task_instance_id,
            compiled_task,
            lineage,
        })
    }
}

#[derive(Debug, Clone)]
struct ResolvedOperatorStep {
    step_id: String,
    operator_id: String,
    capability_type_id: String,
    capability_version: u32,
    contract: CapabilityTypeContract,
}

#[derive(Debug, Clone)]
struct LoweredTaskDraft {
    step_id: String,
    task_instance_id: String,
    compiled_task: CompiledTaskRecord,
    lineage: TaskLineage,
}

fn init_slots_for_operator(
    request: &Request,
    resolved: &ResolvedOperatorStep,
    resolved_by_step: &BTreeMap<&str, &ResolvedOperatorStep>,
) -> Result<Vec<TaskInitSlotSpec>, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut data_flow_slots = BTreeMap::new();
    let mut data_flow_slot_counts = BTreeMap::<String, usize>::new();

    for edge in request
        .composition
        .composition
        .edges
        .iter()
        .filter(|edge| edge.to == resolved.step_id)
    {
        let EdgeKind::DataFlow { artifact_type } = &edge.kind else {
            continue;
        };
        let Some(artifact_type_id) =
            data_flow_artifact_type_id(edge, artifact_type, &mut diagnostics)
        else {
            continue;
        };
        let Some(slot) = unique_data_flow_input_slot(
            &resolved.contract.input_contract,
            &artifact_type_id,
            resolved,
            edge,
            &mut diagnostics,
        ) else {
            continue;
        };
        if slot.cardinality != InputCardinality::One {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::DataFlowInitSourceInvalid,
                    "data flow edge maps to an input slot with unsupported cardinality",
                )
                .with_step(edge.to.clone()),
            );
            continue;
        }
        let Some(schema_version) =
            data_flow_output_schema(edge, &artifact_type_id, resolved_by_step, &mut diagnostics)
        else {
            continue;
        };
        if !slot.schema_versions.accepts(schema_version) {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::DataFlowInitSourceInvalid,
                    "data flow edge schema is not accepted by the target input slot",
                )
                .with_step(edge.to.clone()),
            );
            continue;
        }

        *data_flow_slot_counts
            .entry(slot.slot_id.clone())
            .or_default() += 1;
        data_flow_slots.insert(
            slot.slot_id.clone(),
            TaskInitSlotSpec {
                init_slot_id: slot.slot_id.clone(),
                artifact_type_id,
                schema_version,
                required: slot.required,
            },
        );
    }

    for count in data_flow_slot_counts.into_values() {
        if count > 1 {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::DataFlowInitSourceInvalid,
                    "multiple data flow edges target the same input slot",
                )
                .with_step(resolved.step_id.clone()),
            );
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let mut init_slots = Vec::new();
    for slot in &resolved.contract.input_contract {
        if let Some(data_flow_slot) = data_flow_slots.remove(&slot.slot_id) {
            init_slots.push(data_flow_slot);
            continue;
        }
        if slot.required {
            init_slots.push(TaskInitSlotSpec {
                init_slot_id: slot.slot_id.clone(),
                artifact_type_id: slot.accepted_artifact_type_ids[0].clone(),
                schema_version: slot.schema_versions.min,
                required: true,
            });
        }
    }

    Ok(init_slots)
}

fn unique_data_flow_input_slot<'a>(
    input_slots: &'a [InputSlotSpec],
    artifact_type: &str,
    resolved: &ResolvedOperatorStep,
    edge: &Edge,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a InputSlotSpec> {
    let matching_slots = input_slots
        .iter()
        .filter(|slot| {
            slot.accepted_artifact_type_ids
                .iter()
                .any(|accepted| accepted == artifact_type)
        })
        .collect::<Vec<_>>();

    match matching_slots.as_slice() {
        [slot] => Some(slot),
        [] => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::DataFlowInitSourceInvalid,
                    "data flow edge does not map to a target input slot",
                )
                .with_step(edge.to.clone())
                .with_operator(resolved.operator_id.clone()),
            );
            None
        }
        _ => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::DataFlowInitSourceInvalid,
                    "data flow edge maps to multiple target input slots",
                )
                .with_step(edge.to.clone())
                .with_operator(resolved.operator_id.clone()),
            );
            None
        }
    }
}

fn data_flow_artifact_type_id(
    edge: &Edge,
    artifact_type: &Term,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    match artifact_type {
        Term::ArtifactType(artifact_type_id) => Some(artifact_type_id.clone()),
        _ => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::DataFlowInitSourceInvalid,
                    "data flow edge artifact type is not grounded",
                )
                .with_step(edge.to.clone()),
            );
            None
        }
    }
}

fn data_flow_output_schema(
    edge: &Edge,
    artifact_type: &str,
    resolved_by_step: &BTreeMap<&str, &ResolvedOperatorStep>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<u32> {
    let Some(source) = resolved_by_step.get(edge.from.as_str()) else {
        diagnostics.push(
            Diagnostic::new(
                DiagnosticCode::MissingExecutableEdgeEndpoint,
                "data flow edge source is not lowered",
            )
            .with_step(edge.to.clone()),
        );
        return None;
    };
    let matching_outputs = source
        .contract
        .output_contract
        .iter()
        .filter(|slot| slot.artifact_type_id == artifact_type)
        .collect::<Vec<_>>();

    match matching_outputs.as_slice() {
        [output] => Some(output.schema_version),
        [] => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::DataFlowInitSourceInvalid,
                    "data flow edge does not map to an upstream output contract",
                )
                .with_step(edge.to.clone())
                .with_operator(source.operator_id.clone()),
            );
            None
        }
        _ => {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::DataFlowInitSourceInvalid,
                    "data flow edge maps to multiple upstream output contracts",
                )
                .with_step(edge.to.clone())
                .with_operator(source.operator_id.clone()),
            );
            None
        }
    }
}

fn plan_with_mutations(
    request: Request,
    mutations: Vec<mutation::Mutation>,
    diagnostics: Vec<Diagnostic>,
) -> Plan {
    let mutation_set = mutation::Set::new(
        request.network_id.clone(),
        request.composition.composition_id.clone(),
        request.idempotency_key,
        mutations,
        request.composition.diagnostics.clone(),
    );

    Plan {
        request_id: request.request_id,
        network_id: request.network_id,
        composition_id: request.composition.composition_id,
        goal_id: request.composition.goal.goal_id,
        method_id: request.composition.method_id,
        mutations: mutation_set,
        diagnostics,
    }
}

fn validate_executable_edges(
    edges: &[Edge],
    operator_step_ids: &BTreeSet<&str>,
    diagnostics: &mut Vec<Diagnostic>,
    blocking_diagnostic: &mut bool,
) {
    for edge in edges {
        match &edge.kind {
            EdgeKind::Conditional { .. } => diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::ConditionalEdgeDeferred,
                    "conditional edge lowering is deferred",
                )
                .with_step(edge.to.clone()),
            ),
            EdgeKind::Ordering | EdgeKind::DataFlow { .. } => {
                if !operator_step_ids.contains(edge.from.as_str())
                    || !operator_step_ids.contains(edge.to.as_str())
                {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::MissingExecutableEdgeEndpoint,
                            "executable edge references a step that is not lowered",
                        )
                        .with_step(edge.to.clone()),
                    );
                    *blocking_diagnostic = true;
                }
            }
        }
    }
}

fn incoming_edges(
    composition: &ExecutionComposition,
    step_id: &str,
    network_id: &str,
) -> Vec<DependencyEdge> {
    composition
        .composition
        .edges
        .iter()
        .filter(|edge| edge.to == step_id)
        .filter_map(|edge| {
            let kind = match &edge.kind {
                EdgeKind::Ordering => DependencyKind::Ordering,
                EdgeKind::DataFlow {
                    artifact_type: Term::ArtifactType(artifact_type),
                } => DependencyKind::DataFlow {
                    artifact_type_id: artifact_type.clone(),
                },
                EdgeKind::DataFlow { .. } => return None,
                EdgeKind::Conditional { .. } => return None,
            };
            Some(DependencyEdge {
                from: task_instance_id(network_id, &composition.composition_id, &edge.from),
                to: task_instance_id(network_id, &composition.composition_id, &edge.to),
                kind,
            })
        })
        .collect()
}

fn init_sources_for_task(
    request: &Request,
    draft: &LoweredTaskDraft,
    edges: &[Edge],
    diagnostics: &mut Vec<Diagnostic>,
    blocking_diagnostic: &mut bool,
) -> Vec<TaskInitSource> {
    let incoming_data_flow = edges
        .iter()
        .filter(|edge| edge.to == draft.step_id)
        .filter_map(|edge| match &edge.kind {
            EdgeKind::DataFlow {
                artifact_type: Term::ArtifactType(artifact_type),
            } => Some((edge, artifact_type.as_str())),
            _ => None,
        })
        .collect::<Vec<_>>();

    for (edge, artifact_type) in &incoming_data_flow {
        let matching_slot_count = draft
            .compiled_task
            .init_slots
            .iter()
            .filter(|slot| slot.artifact_type_id == *artifact_type)
            .count();
        if matching_slot_count != 1 {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::DataFlowInitSourceInvalid,
                    "data flow edge does not map to exactly one target init slot",
                )
                .with_step(edge.to.clone()),
            );
            *blocking_diagnostic = true;
        }
    }

    let mut sources = Vec::new();
    for slot in &draft.compiled_task.init_slots {
        let matching_edges = incoming_data_flow
            .iter()
            .filter(|(_, artifact_type)| slot.artifact_type_id == **artifact_type)
            .collect::<Vec<_>>();
        match matching_edges.as_slice() {
            [] => sources.push(TaskInitSource::StaticSeed(StaticSeedInitSource {
                init_slot_id: slot.init_slot_id.clone(),
                artifact_type_id: slot.artifact_type_id.clone(),
                schema_version: slot.schema_version,
                content: json!({
                    "composition_id": request.composition.composition_id,
                    "goal_id": request.composition.goal.goal_id,
                    "method_id": request.composition.method_id,
                    "step_id": draft.step_id,
                    "slot_id": slot.init_slot_id,
                }),
            })),
            [(edge, artifact_type)] => {
                sources.push(TaskInitSource::UpstreamArtifact(
                    UpstreamArtifactInitSource {
                        init_slot_id: slot.init_slot_id.clone(),
                        artifact_type_id: slot.artifact_type_id.clone(),
                        schema_version: slot.schema_version,
                        upstream_task_instance_id: task_instance_id(
                            &request.network_id,
                            &request.composition.composition_id,
                            &edge.from,
                        ),
                        upstream_artifact_type_id: (*artifact_type).to_string(),
                    },
                ));
            }
            _ => {
                diagnostics.push(
                    Diagnostic::new(
                        DiagnosticCode::DataFlowInitSourceInvalid,
                        "multiple data flow edges target the same init slot",
                    )
                    .with_step(draft.step_id.clone()),
                );
                *blocking_diagnostic = true;
            }
        }
    }

    sources
}

fn task_instance_id(network_id: &str, composition_id: &str, step_id: &str) -> String {
    #[derive(Serialize)]
    struct Identity<'a> {
        network_id: &'a str,
        composition_id: &'a str,
        step_id: &'a str,
    }

    stable_id(
        "task-network-task",
        &Identity {
            network_id,
            composition_id,
            step_id,
        },
    )
}

fn task_run_id(network_id: &str, composition_id: &str, step_id: &str) -> String {
    #[derive(Serialize)]
    struct Identity<'a> {
        network_id: &'a str,
        composition_id: &'a str,
        step_id: &'a str,
    }

    stable_id(
        "task-network-run",
        &Identity {
            network_id,
            composition_id,
            step_id,
        },
    )
}

fn binding_value(bindings: &Bindings, binding_id: &str) -> serde_json::Value {
    let candidates = [
        binding_id.to_string(),
        format!("?{binding_id}"),
        binding_id.trim_start_matches('?').to_string(),
    ];
    for candidate in candidates {
        if let Some(term) = bindings.get(&candidate) {
            return serde_json::to_value(term).expect("binding term is serializable");
        }
    }
    json!({
        "source": "composition_lowering_default",
        "binding_id": binding_id,
    })
}

fn scope_ref(bindings: &Bindings, fallback_goal_id: &str) -> String {
    for candidate in ["?node", "node", "?scope", "scope"] {
        if let Some(term) = bindings.get(candidate) {
            return term_to_scope_ref(term);
        }
    }

    bindings
        .iter()
        .next()
        .map(|(_, term)| term_to_scope_ref(term))
        .unwrap_or_else(|| fallback_goal_id.to_string())
}

fn term_to_scope_ref(term: &Term) -> String {
    match term {
        Term::Object(object) => object.object_id.clone(),
        Term::ArtifactType(artifact_type) => artifact_type.clone(),
        Term::Dimension(dimension) => dimension.clone(),
        Term::Variable(variable) => variable.clone(),
        Term::Literal(literal) => serde_json::to_string(literal).expect("literal is serializable"),
        Term::Derived {
            source_step,
            field_path,
        } => format!("{source_step}.{field_path}"),
    }
}
