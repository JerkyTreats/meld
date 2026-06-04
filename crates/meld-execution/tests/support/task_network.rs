#![allow(dead_code)]

use meld_events::DomainObjectRef;
use meld_execution::capability::{
    BindingSpec, BindingValueKind, CapabilityCatalog, CapabilityTypeContract, ExecutionClass,
    ExecutionContract, OutputSlotSpec, ScopeContract,
};
use meld_execution::planning::{
    CompositionLoweringDiagnosticCode, ExecutionCompositionLowerer, OperatorResolutionReport,
    OperatorResolutionStatus, PlanningWorldStateFrameRef,
};
use meld_execution::task::{
    CompiledTaskRecord, TaskCompiler, TaskInitializationPayload, TaskRunContext,
};
use meld_execution::task_network::command::{Command, Request as CommandRequest};
use meld_execution::task_network::dispatch::{
    Claim, Outcome, OutcomeStatus, Request as DispatchRequest,
};
use meld_execution::task_network::mutation::{Inject, Mutation, ReadPrecondition, Set};
use meld_execution::task_network::readiness::compute_ready_set;
use meld_execution::task_network::state::{
    ArtifactAvailability, DependencyEdge, DependencyKind, TaskLineage, TaskNode,
};
use meld_execution::task_network::store::{InMemoryTaskNetworkStore, SledTaskNetworkStore};
use meld_lang::{
    Bindings, Composition, CostEstimate, Effect, Goal, GoalLifecycle, GoalPriority, GoalSource,
    Operator, Proposition, Resolution, SlotConstraint, Step, StepKind, Term, ValidationResult,
};

pub fn catalog() -> CapabilityCatalog {
    let mut catalog = CapabilityCatalog::new();
    catalog
        .register(CapabilityTypeContract {
            capability_type_id: "docs.write".to_string(),
            capability_version: 1,
            owning_domain: "docs".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "filesystem".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![],
            input_contract: vec![],
            output_contract: vec![OutputSlotSpec {
                slot_id: "patch".to_string(),
                artifact_type_id: "docs_patch".to_string(),
                schema_version: 1,
                guaranteed: true,
            }],
            effect_contract: vec![],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Queued,
                completion_semantics: "result_or_failure".to_string(),
                retry_class: "provider_io".to_string(),
                cancellation_supported: true,
            },
        })
        .unwrap();
    catalog
}

pub fn catalog_with_required_node_binding() -> CapabilityCatalog {
    let mut catalog = CapabilityCatalog::new();
    catalog
        .register(CapabilityTypeContract {
            capability_type_id: "docs.write".to_string(),
            capability_version: 1,
            owning_domain: "docs".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "filesystem".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![BindingSpec {
                binding_id: "node".to_string(),
                value_kind: BindingValueKind::Literal,
                required: true,
                affects_deterministic_identity: true,
            }],
            input_contract: vec![],
            output_contract: vec![OutputSlotSpec {
                slot_id: "patch".to_string(),
                artifact_type_id: "docs_patch".to_string(),
                schema_version: 1,
                guaranteed: true,
            }],
            effect_contract: vec![],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Queued,
                completion_semantics: "result_or_failure".to_string(),
                retry_class: "provider_io".to_string(),
                cancellation_supported: true,
            },
        })
        .unwrap();
    catalog
}

pub fn goal() -> Goal {
    Goal {
        goal_id: "goal-docs".to_string(),
        agent_id: "agent-docs".to_string(),
        target: Proposition::Accessible {
            scope: Term::Object(DomainObjectRef::new("workspace", "node", "readme").unwrap()),
        },
        priority: GoalPriority {
            urgency: 1,
            cost_ceiling: None,
        },
        source: GoalSource::UserDirected {
            directive: "refresh docs".to_string(),
        },
        lifecycle: GoalLifecycle::Active,
    }
}

pub fn frame() -> PlanningWorldStateFrameRef {
    PlanningWorldStateFrameRef {
        frame_id: "frame-docs".to_string(),
        projection_version: "world_model.planner.v1".to_string(),
        perspective_id: "default".to_string(),
        branch_id: "main".to_string(),
        source_refs: vec!["source".to_string()],
        warnings: vec![],
    }
}

pub fn composition() -> meld_execution::planning::ExecutionComposition {
    meld_execution::planning::ExecutionComposition {
        composition_id: "composition-docs".to_string(),
        goal: goal(),
        world_state_frame: frame(),
        method_id: "refresh_docs_v1".to_string(),
        bindings: Bindings::empty(),
        composition: Composition {
            steps: vec![Step {
                step_id: "write".to_string(),
                kind: StepKind::Op(Operator {
                    operator_id: "write".to_string(),
                    preconditions: vec![Proposition::Accessible {
                        scope: Term::Object(
                            DomainObjectRef::new("workspace", "node", "readme").unwrap(),
                        ),
                    }],
                    effects: vec![Effect::Assert(Proposition::Accessible {
                        scope: Term::Object(
                            DomainObjectRef::new("workspace", "node", "readme").unwrap(),
                        ),
                    })],
                    cost: CostEstimate::zero(),
                    resolution: Resolution {
                        requires_inputs: vec![],
                        requires_outputs: vec![SlotConstraint {
                            artifact_type_id: "docs_patch".to_string(),
                            required: true,
                        }],
                        scope_kind: Some("filesystem".to_string()),
                        tags: vec![],
                        specific: None,
                    },
                }),
            }],
            edges: vec![],
        },
        projected_effects: vec![],
        operator_resolutions: vec![OperatorResolutionReport {
            operator_id: "write".to_string(),
            status: OperatorResolutionStatus::Resolved,
            capability_type_id: Some("docs.write".to_string()),
            capability_version: Some(1),
            tags: vec![],
            diagnostics: vec![],
        }],
        validation: ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
        },
        diagnostics: vec![],
    }
}

pub fn unresolved_composition() -> meld_execution::planning::ExecutionComposition {
    let mut composition = composition();
    composition.operator_resolutions[0].status = OperatorResolutionStatus::Unresolved;
    composition
}

pub fn lower() -> meld_execution::planning::CompositionLoweringPlan {
    let lowerer = ExecutionCompositionLowerer::new(TaskCompiler::new(), catalog());
    lowerer
        .lower(meld_execution::planning::CompositionLoweringRequest {
            request_id: "lower-docs".to_string(),
            network_id: "network-docs".to_string(),
            composition: composition(),
            idempotency_key: "lower-once".to_string(),
        })
        .unwrap()
}

pub fn lower_unresolved() -> meld_execution::planning::CompositionLoweringPlan {
    let lowerer = ExecutionCompositionLowerer::new(TaskCompiler::new(), catalog());
    lowerer
        .lower(meld_execution::planning::CompositionLoweringRequest {
            request_id: "lower-docs".to_string(),
            network_id: "network-docs".to_string(),
            composition: unresolved_composition(),
            idempotency_key: "lower-once".to_string(),
        })
        .unwrap()
}

pub fn command_for_state(
    network_id: &str,
    revision: u64,
    state_hash: &str,
    command_id: &str,
    command: Command,
) -> CommandRequest {
    CommandRequest {
        command_id: command_id.to_string(),
        network_id: network_id.to_string(),
        base_revision: revision,
        base_state_hash: state_hash.to_string(),
        read_preconditions: vec![ReadPrecondition::RevisionIs(revision)],
        command,
    }
}

pub fn apply_memory_command(
    store: &InMemoryTaskNetworkStore,
    command_id: &str,
    command: Command,
) -> CommandRequest {
    command_for_state(
        &store.state().network_id,
        store.state().revision,
        &store.state().state_hash,
        command_id,
        command,
    )
}

pub fn apply_sled_command(
    store: &SledTaskNetworkStore,
    command_id: &str,
    command: Command,
) -> CommandRequest {
    command_for_state(
        &store.state().network_id,
        store.state().revision,
        &store.state().state_hash,
        command_id,
        command,
    )
}

pub fn single_task_node(task_instance_id: &str) -> TaskNode {
    let task_id = format!("compiled-{task_instance_id}");
    TaskNode {
        task_instance_id: task_instance_id.to_string(),
        lifecycle_epoch: 1,
        compiled_task: CompiledTaskRecord {
            task_id: task_id.clone(),
            task_version: 1,
            init_slots: vec![],
            capability_instances: vec![],
            dependency_edges: vec![],
        },
        init_payload: TaskInitializationPayload {
            task_id: task_id.clone(),
            compiled_task_ref: format!("{task_id}@1"),
            init_artifacts: vec![],
            task_run_context: TaskRunContext {
                task_run_id: format!("run-{task_instance_id}"),
                session_id: Some("session-task-network".to_string()),
                trigger: "task-network.test".to_string(),
            },
        },
        lineage: TaskLineage {
            composition_id: "composition-fixture".to_string(),
            goal_id: "goal-fixture".to_string(),
            method_id: "method-fixture".to_string(),
            step_id: format!("step-{task_instance_id}"),
            operator_id: format!("operator-{task_instance_id}"),
            world_state_frame_id: "frame-fixture".to_string(),
            capability_type_id: "docs.write".to_string(),
            capability_version: 1,
        },
    }
}

pub fn inject_for_node(task_node: TaskNode, incoming_edges: Vec<DependencyEdge>) -> Inject {
    let lineage = task_node.lineage.clone();
    Inject::new(task_node, incoming_edges, lineage)
}

pub fn single_task_mutation_set(task_instance_id: &str) -> Set {
    Set::new(
        "network-docs",
        "composition-fixture",
        format!("inject-{task_instance_id}"),
        vec![Mutation::Inject(inject_for_node(
            single_task_node(task_instance_id),
            vec![],
        ))],
        vec![],
    )
}

pub fn two_task_ordering_set(upstream: &str, downstream: &str) -> Set {
    Set::new(
        "network-docs",
        "composition-fixture",
        "inject-two-ordering",
        vec![
            Mutation::Inject(inject_for_node(single_task_node(upstream), vec![])),
            Mutation::Inject(inject_for_node(
                single_task_node(downstream),
                vec![DependencyEdge {
                    from: upstream.to_string(),
                    to: downstream.to_string(),
                    kind: DependencyKind::Ordering,
                }],
            )),
        ],
        vec![],
    )
}

pub fn commit_single_task(store: &mut InMemoryTaskNetworkStore, task_instance_id: &str) {
    let request = apply_memory_command(
        store,
        &format!("command-commit-{task_instance_id}"),
        Command::ApplyMutationSet(single_task_mutation_set(task_instance_id)),
    );
    store.submit(request);
}

pub fn commit_single_task_sled(store: &mut SledTaskNetworkStore, task_instance_id: &str) {
    let request = apply_sled_command(
        store,
        &format!("command-commit-{task_instance_id}"),
        Command::ApplyMutationSet(single_task_mutation_set(task_instance_id)),
    );
    store.submit(request).unwrap();
}

pub fn outcome_for_claim(outcome_id: &str, task_instance_id: &str, claim: &Claim) -> Outcome {
    Outcome {
        outcome_id: outcome_id.to_string(),
        task_instance_id: task_instance_id.to_string(),
        lifecycle_epoch: claim.lifecycle_epoch,
        claim_id: claim.claim_id.clone(),
        claim_revision: claim.claim_revision,
        status: OutcomeStatus::Succeeded,
        error: None,
        artifacts: vec![ArtifactAvailability {
            task_instance_id: task_instance_id.to_string(),
            artifact_type_id: "docs_patch".to_string(),
            artifact_id: format!("artifact-{outcome_id}"),
            schema_version: 1,
        }],
        artifact_records: vec![],
        task_events: vec![],
    }
}

pub fn claim_ready_memory(
    store: &mut InMemoryTaskNetworkStore,
    command_id: &str,
    claim_id: &str,
) -> String {
    let ready = compute_ready_set(store.state());
    let task_instance_id = ready.task_instance_ids[0].clone();
    let claim = DispatchRequest {
        claim_id: claim_id.to_string(),
        task_instance_id: task_instance_id.clone(),
        worker_id: "worker-a".to_string(),
        idempotency_key: format!("{claim_id}-once"),
    };
    let request = apply_memory_command(store, command_id, Command::ClaimReadyTask(claim));
    store.submit(request);
    task_instance_id
}

pub fn claim_ready_sled(
    store: &mut SledTaskNetworkStore,
    command_id: &str,
    claim_id: &str,
) -> String {
    let ready = compute_ready_set(store.state());
    let task_instance_id = ready.task_instance_ids[0].clone();
    let claim = DispatchRequest {
        claim_id: claim_id.to_string(),
        task_instance_id: task_instance_id.clone(),
        worker_id: "worker-a".to_string(),
        idempotency_key: format!("{claim_id}-once"),
    };
    let request = apply_sled_command(store, command_id, Command::ClaimReadyTask(claim));
    store.submit(request).unwrap();
    task_instance_id
}

pub fn memory_store_with_pending_publication() -> (InMemoryTaskNetworkStore, String, String) {
    let mut store = InMemoryTaskNetworkStore::new("network-docs");
    commit_single_task(&mut store, "task-alpha");
    let task_instance_id = claim_ready_memory(&mut store, "command-claim", "claim-alpha");
    let claim = store.state().claims.get("claim-alpha").unwrap().clone();
    let outcome = outcome_for_claim("outcome-alpha", &task_instance_id, &claim);
    let request = apply_memory_command(
        &store,
        "command-outcome",
        Command::RecordTaskOutcome(outcome),
    );
    store.submit(request);
    let publication_id = store.state().publications.keys().next().unwrap().clone();
    (store, task_instance_id, publication_id)
}

pub fn sled_store_with_one_committed_task(tempdir: &tempfile::TempDir) -> SledTaskNetworkStore {
    let db = sled::open(tempdir.path()).unwrap();
    let mut store = SledTaskNetworkStore::open(db, "network-docs").unwrap();
    commit_single_task_sled(&mut store, "task-alpha");
    store
}

pub fn assert_unresolved_operator_diagnostic(
    plan: &meld_execution::planning::CompositionLoweringPlan,
) {
    assert_eq!(
        plan.diagnostics[0].code,
        CompositionLoweringDiagnosticCode::OperatorUnresolved
    );
}
