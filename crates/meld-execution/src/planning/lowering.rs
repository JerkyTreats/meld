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
    CapabilityCatalog,
};
use crate::error::ApiError;
use crate::planning::{ExecutionComposition, OperatorResolutionStatus};
use crate::task::{
    InitArtifactValue, TaskDefinition, TaskDefinitionCompiler, TaskInitSlotSpec,
    TaskInitializationPayload, TaskRunContext,
};
use crate::task_network::{
    contracts::stable_id,
    mutation,
    state::{DependencyEdge, DependencyKind, TaskLineage, TaskNode},
};
use meld_lang::{Bindings, EdgeKind, StepKind, Term};
use serde::{Deserialize, Serialize};
use serde_json::json;

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

        let Some((step, operator)) =
            request
                .composition
                .composition
                .steps
                .iter()
                .find_map(|step| match &step.kind {
                    StepKind::Op(operator) => Some((step, operator)),
                    StepKind::Goal(_) => None,
                })
        else {
            diagnostics.push(Diagnostic::new(
                DiagnosticCode::NoOperatorStep,
                "composition does not contain an operator step",
            ));
            return Ok(plan_with_mutations(request, Vec::new(), diagnostics));
        };

        let Some(resolution) = request
            .composition
            .operator_resolutions
            .iter()
            .find(|resolution| resolution.operator_id == operator.operator_id)
        else {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::MissingOperatorResolution,
                    "operator has no resolution report",
                )
                .with_step(step.step_id.clone())
                .with_operator(operator.operator_id.clone()),
            );
            return Ok(plan_with_mutations(request, Vec::new(), diagnostics));
        };

        if resolution.status != OperatorResolutionStatus::Resolved {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::OperatorUnresolved,
                    "operator resolution report is unresolved",
                )
                .with_step(step.step_id.clone())
                .with_operator(operator.operator_id.clone()),
            );
            return Ok(plan_with_mutations(request, Vec::new(), diagnostics));
        }

        let Some(capability_type_id) = resolution.capability_type_id.as_deref() else {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::CapabilityContractMissing,
                    "resolved operator did not name a capability type",
                )
                .with_step(step.step_id.clone())
                .with_operator(operator.operator_id.clone()),
            );
            return Ok(plan_with_mutations(request, Vec::new(), diagnostics));
        };
        let Some(capability_version) = resolution.capability_version else {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::CapabilityContractMissing,
                    "resolved operator did not name a capability version",
                )
                .with_step(step.step_id.clone())
                .with_operator(operator.operator_id.clone()),
            );
            return Ok(plan_with_mutations(request, Vec::new(), diagnostics));
        };
        let Some(contract) = self.catalog.get(capability_type_id, capability_version) else {
            diagnostics.push(
                Diagnostic::new(
                    DiagnosticCode::CapabilityContractMissing,
                    "resolved capability was not found in the supplied catalog",
                )
                .with_step(step.step_id.clone())
                .with_operator(operator.operator_id.clone()),
            );
            return Ok(plan_with_mutations(request, Vec::new(), diagnostics));
        };

        let task_instance_id = task_instance_id(
            &request.network_id,
            &request.composition.composition_id,
            &step.step_id,
        );
        let task_id = task_instance_id.clone();
        let init_slots = contract
            .input_contract
            .iter()
            .filter(|slot| slot.required)
            .map(|slot| TaskInitSlotSpec {
                init_slot_id: slot.slot_id.clone(),
                artifact_type_id: slot.accepted_artifact_type_ids[0].clone(),
                schema_version: slot.schema_versions.min,
                required: true,
            })
            .collect::<Vec<_>>();
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
        let binding_values = contract
            .binding_contract
            .iter()
            .filter(|binding| binding.required)
            .map(|binding| BoundBindingValue {
                binding_id: binding.binding_id.clone(),
                value: binding_value(&request.composition.bindings, &binding.binding_id),
            })
            .collect::<Vec<_>>();
        let definition = TaskDefinition {
            task_id: task_id.clone(),
            task_version: 1,
            init_slots,
            capability_instances: vec![BoundCapabilityInstance {
                capability_instance_id: step.step_id.clone(),
                capability_type_id: capability_type_id.to_string(),
                capability_version,
                scope_ref: scope_ref(
                    &request.composition.bindings,
                    &request.composition.goal.goal_id,
                ),
                scope_kind: contract.scope_contract.scope_kind.clone(),
                binding_values,
                input_wiring,
            }],
        };
        let compiled_task = self
            .compiler
            .compile_task_definition(&definition, &self.catalog)?;
        let init_payload = TaskInitializationPayload {
            task_id: task_id.clone(),
            compiled_task_ref: compiled_task.task_id.clone(),
            init_artifacts: compiled_task
                .init_slots
                .iter()
                .map(|slot| InitArtifactValue {
                    init_slot_id: slot.init_slot_id.clone(),
                    artifact_type_id: slot.artifact_type_id.clone(),
                    schema_version: slot.schema_version,
                    content: json!({
                        "composition_id": request.composition.composition_id,
                        "goal_id": request.composition.goal.goal_id,
                        "method_id": request.composition.method_id,
                        "step_id": step.step_id,
                        "slot_id": slot.init_slot_id,
                    }),
                })
                .collect(),
            task_run_context: TaskRunContext {
                task_run_id: task_run_id(
                    &request.network_id,
                    &request.composition.composition_id,
                    &step.step_id,
                ),
                session_id: None,
                trigger: "execution_composition_lowering".to_string(),
            },
        };
        let lineage = TaskLineage {
            composition_id: request.composition.composition_id.clone(),
            goal_id: request.composition.goal.goal_id.clone(),
            method_id: request.composition.method_id.clone(),
            step_id: step.step_id.clone(),
            operator_id: operator.operator_id.clone(),
            world_state_frame_id: request.composition.world_state_frame.frame_id.clone(),
            capability_type_id: capability_type_id.to_string(),
            capability_version,
        };
        let task_node = TaskNode {
            task_instance_id,
            lifecycle_epoch: 0,
            compiled_task,
            init_payload,
            lineage: lineage.clone(),
        };
        let incoming_edges = incoming_edges(
            &request.composition,
            &step.step_id,
            &request.network_id,
            &mut diagnostics,
        );
        let inject = mutation::Inject::new(task_node, incoming_edges, lineage);
        let mutations = vec![mutation::Mutation::Inject(inject)];

        Ok(plan_with_mutations(request, mutations, diagnostics))
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

fn incoming_edges(
    composition: &ExecutionComposition,
    step_id: &str,
    network_id: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<DependencyEdge> {
    composition
        .composition
        .edges
        .iter()
        .filter(|edge| edge.to == step_id)
        .map(|edge| {
            let kind = match &edge.kind {
                EdgeKind::Ordering => DependencyKind::Ordering,
                EdgeKind::DataFlow { artifact_type } => DependencyKind::DataFlow {
                    artifact_type_id: artifact_type.clone(),
                },
                EdgeKind::Conditional { field_path, guard } => {
                    diagnostics.push(
                        Diagnostic::new(
                            DiagnosticCode::ConditionalEdgeDeferred,
                            "conditional edge lowering is deferred",
                        )
                        .with_step(step_id.to_string()),
                    );
                    DependencyKind::Conditional {
                        field_path: field_path.clone(),
                        guard_json: serde_json::to_string(guard)
                            .expect("conditional guard is serializable"),
                    }
                }
            };
            DependencyEdge {
                from: task_instance_id(network_id, &composition.composition_id, &edge.from),
                to: task_instance_id(network_id, &composition.composition_id, &edge.to),
                kind,
            }
        })
        .collect()
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
