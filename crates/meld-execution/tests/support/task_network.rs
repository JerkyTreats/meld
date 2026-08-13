#![allow(dead_code)]

use meld_events::DomainObjectRef;
use meld_execution::capability::{
    ArtifactSchemaVersionRange, BindingSpec, BindingValueKind, CapabilityCatalog,
    CapabilityTypeContract, ExecutionClass, ExecutionContract, InputCardinality, InputSlotSpec,
    OutputSlotSpec, ScopeContract,
};
use meld_execution::planning::{
    CompositionLoweringDiagnosticCode, ExecutionCompositionLowerer, OperatorResolutionReport,
    OperatorResolutionStatus, PlanningWorldStateFrameRef,
};
use meld_execution::task::{
    ArtifactProducerRef, ArtifactRecord, CompiledTaskRecord, TaskCompiler, TaskInitSlotSpec,
    TaskRunContext,
};
use meld_execution::task_network::command::{Command, Request as CommandRequest};
use meld_execution::task_network::dispatch::{
    Claim, Outcome, OutcomeStatus, Request as DispatchRequest,
};
use meld_execution::task_network::mutation::{Inject, Mutation, ReadPrecondition, Set};
use meld_execution::task_network::readiness::compute_ready_set;
use meld_execution::task_network::state::{
    DependencyEdge, DependencyEdgeOrigin, DependencyKind, StaticSeedInitSource, TaskInitSource,
    TaskLineage, TaskNode, UpstreamArtifactInitSource,
};
use meld_execution::task_network::store::{InMemoryTaskNetworkStore, SledTaskNetworkStore};
use meld_lang::{
    Bindings, Composition, CostEstimate, Edge, EdgeKind, Effect, Goal, GoalLifecycle, GoalPriority,
    GoalSource, Operator, Proposition, Resolution, SlotConstraint, Step, StepKind, Term,
    ValidationResult,
};
use serde_json::json;

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

pub fn phase8_catalog() -> CapabilityCatalog {
    let mut catalog = CapabilityCatalog::new();
    for (capability_type_id, artifact_type_id) in [
        ("docs.prepare_metadata", "metadata_doc"),
        ("docs.collect_context", "context_bundle"),
    ] {
        catalog
            .register(CapabilityTypeContract {
                capability_type_id: capability_type_id.to_string(),
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
                    slot_id: artifact_type_id.to_string(),
                    artifact_type_id: artifact_type_id.to_string(),
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
    }
    catalog
        .register(CapabilityTypeContract {
            capability_type_id: "docs.write_summary".to_string(),
            capability_version: 1,
            owning_domain: "docs".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "filesystem".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![],
            input_contract: vec![
                InputSlotSpec {
                    slot_id: "metadata".to_string(),
                    accepted_artifact_type_ids: vec!["metadata_doc".to_string()],
                    schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
                    required: true,
                    cardinality: InputCardinality::One,
                },
                InputSlotSpec {
                    slot_id: "context".to_string(),
                    accepted_artifact_type_ids: vec!["context_bundle".to_string()],
                    schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
                    required: true,
                    cardinality: InputCardinality::One,
                },
            ],
            output_contract: vec![OutputSlotSpec {
                slot_id: "summary_doc".to_string(),
                artifact_type_id: "summary_doc".to_string(),
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

pub fn single_input_dataflow_catalog() -> CapabilityCatalog {
    let mut catalog = CapabilityCatalog::new();
    catalog
        .register(CapabilityTypeContract {
            capability_type_id: "docs.prepare_metadata".to_string(),
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
                slot_id: "metadata_doc".to_string(),
                artifact_type_id: "metadata_doc".to_string(),
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
        .register(CapabilityTypeContract {
            capability_type_id: "docs.write_metadata".to_string(),
            capability_version: 1,
            owning_domain: "docs".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "filesystem".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![],
            input_contract: vec![InputSlotSpec {
                slot_id: "metadata".to_string(),
                accepted_artifact_type_ids: vec!["metadata_doc".to_string()],
                schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
                required: true,
                cardinality: InputCardinality::One,
            }],
            output_contract: vec![OutputSlotSpec {
                slot_id: "summary_doc".to_string(),
                artifact_type_id: "summary_doc".to_string(),
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

pub fn optional_input_dataflow_catalog() -> CapabilityCatalog {
    let mut catalog = CapabilityCatalog::new();
    catalog
        .register(CapabilityTypeContract {
            capability_type_id: "docs.produce_optional_note".to_string(),
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
                slot_id: "optional_note".to_string(),
                artifact_type_id: "optional_note".to_string(),
                schema_version: 2,
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
        .register(CapabilityTypeContract {
            capability_type_id: "docs.consume_optional_note".to_string(),
            capability_version: 1,
            owning_domain: "docs".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "filesystem".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![],
            input_contract: vec![InputSlotSpec {
                slot_id: "note".to_string(),
                accepted_artifact_type_ids: vec![
                    "other_note".to_string(),
                    "optional_note".to_string(),
                ],
                schema_versions: ArtifactSchemaVersionRange { min: 1, max: 3 },
                required: false,
                cardinality: InputCardinality::One,
            }],
            output_contract: vec![OutputSlotSpec {
                slot_id: "summary_doc".to_string(),
                artifact_type_id: "summary_doc".to_string(),
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
                            artifact_type: Term::ArtifactType("docs_patch".to_string()),
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
        authority_decision: None,
    }
}

pub fn phase8_composition() -> meld_execution::planning::ExecutionComposition {
    let mut composition = composition();
    composition.composition.steps = vec![
        operator_step("prepare_metadata", "metadata_doc"),
        operator_step("collect_context", "context_bundle"),
        operator_step("write_summary", "summary_doc"),
    ];
    composition.composition.edges = vec![
        Edge {
            from: "prepare_metadata".to_string(),
            to: "write_summary".to_string(),
            kind: EdgeKind::DataFlow {
                artifact_type: Term::ArtifactType("metadata_doc".to_string()),
            },
        },
        Edge {
            from: "collect_context".to_string(),
            to: "write_summary".to_string(),
            kind: EdgeKind::DataFlow {
                artifact_type: Term::ArtifactType("context_bundle".to_string()),
            },
        },
    ];
    composition.operator_resolutions = vec![
        phase8_resolution("prepare_metadata", "docs.prepare_metadata"),
        phase8_resolution("collect_context", "docs.collect_context"),
        phase8_resolution("write_summary", "docs.write_summary"),
    ];
    composition
}

pub fn single_input_dataflow_composition() -> meld_execution::planning::ExecutionComposition {
    let mut composition = composition();
    composition.composition.steps = vec![
        operator_step("prepare_metadata", "metadata_doc"),
        operator_step("write_metadata", "summary_doc"),
    ];
    composition.composition.edges = vec![Edge {
        from: "prepare_metadata".to_string(),
        to: "write_metadata".to_string(),
        kind: EdgeKind::DataFlow {
            artifact_type: Term::ArtifactType("metadata_doc".to_string()),
        },
    }];
    composition.operator_resolutions = vec![
        phase8_resolution("prepare_metadata", "docs.prepare_metadata"),
        phase8_resolution("write_metadata", "docs.write_metadata"),
    ];
    composition
}

pub fn optional_input_dataflow_composition() -> meld_execution::planning::ExecutionComposition {
    let mut composition = composition();
    composition.composition.steps = vec![
        operator_step("produce_optional_note", "optional_note"),
        operator_step("consume_optional_note", "summary_doc"),
    ];
    composition.composition.edges = vec![Edge {
        from: "produce_optional_note".to_string(),
        to: "consume_optional_note".to_string(),
        kind: EdgeKind::DataFlow {
            artifact_type: Term::ArtifactType("optional_note".to_string()),
        },
    }];
    composition.operator_resolutions = vec![
        phase8_resolution("produce_optional_note", "docs.produce_optional_note"),
        phase8_resolution("consume_optional_note", "docs.consume_optional_note"),
    ];
    composition
}

pub fn lower_phase8() -> meld_execution::planning::CompositionLoweringPlan {
    let lowerer = ExecutionCompositionLowerer::new(TaskCompiler::new(), phase8_catalog());
    lowerer
        .lower(meld_execution::planning::CompositionLoweringRequest {
            request_id: "lower-docs".to_string(),
            network_id: "network-docs".to_string(),
            composition: phase8_composition(),
            idempotency_key: "lower-once".to_string(),
        })
        .unwrap()
}

fn operator_step(step_id: &str, output_artifact_type_id: &str) -> Step {
    Step {
        step_id: step_id.to_string(),
        kind: StepKind::Op(Operator {
            operator_id: step_id.to_string(),
            preconditions: vec![Proposition::Accessible {
                scope: Term::Object(DomainObjectRef::new("workspace", "node", "readme").unwrap()),
            }],
            effects: vec![Effect::Assert(Proposition::Accessible {
                scope: Term::Object(DomainObjectRef::new("workspace", "node", "readme").unwrap()),
            })],
            cost: CostEstimate::zero(),
            resolution: Resolution {
                requires_inputs: vec![],
                requires_outputs: vec![SlotConstraint {
                    artifact_type: Term::ArtifactType(output_artifact_type_id.to_string()),
                    required: true,
                }],
                scope_kind: Some("filesystem".to_string()),
                tags: vec![],
                specific: None,
            },
        }),
    }
}

fn phase8_resolution(operator_id: &str, capability_type_id: &str) -> OperatorResolutionReport {
    OperatorResolutionReport {
        operator_id: operator_id.to_string(),
        status: OperatorResolutionStatus::Resolved,
        capability_type_id: Some(capability_type_id.to_string()),
        capability_version: Some(1),
        tags: vec![],
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
        init_sources: vec![],
        task_run_context: TaskRunContext {
            task_run_id: format!("run-{task_instance_id}"),
            session_id: Some("session-task-network".to_string()),
            trigger: "task-network.test".to_string(),
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
            authority_decision: None,
        },
    }
}

pub fn task_node_with_static_seed(
    task_instance_id: &str,
    init_slot_id: &str,
    artifact_type_id: &str,
    schema_version: u32,
) -> TaskNode {
    let mut node = single_task_node(task_instance_id);
    node.compiled_task.init_slots = vec![TaskInitSlotSpec {
        init_slot_id: init_slot_id.to_string(),
        artifact_type_id: artifact_type_id.to_string(),
        schema_version,
        required: true,
    }];
    node.init_sources = vec![TaskInitSource::StaticSeed(StaticSeedInitSource {
        init_slot_id: init_slot_id.to_string(),
        artifact_type_id: artifact_type_id.to_string(),
        schema_version,
        content: json!({
            "seed": task_instance_id,
        }),
    })];
    node
}

pub fn task_node_with_upstream_source(
    task_instance_id: &str,
    upstream_task_instance_id: &str,
    init_slot_id: &str,
    artifact_type_id: &str,
    schema_version: u32,
) -> TaskNode {
    let mut node = single_task_node(task_instance_id);
    node.compiled_task.init_slots = vec![TaskInitSlotSpec {
        init_slot_id: init_slot_id.to_string(),
        artifact_type_id: artifact_type_id.to_string(),
        schema_version,
        required: true,
    }];
    node.init_sources = vec![TaskInitSource::UpstreamArtifact(
        UpstreamArtifactInitSource {
            init_slot_id: init_slot_id.to_string(),
            artifact_type_id: artifact_type_id.to_string(),
            schema_version,
            upstream_task_instance_id: upstream_task_instance_id.to_string(),
            upstream_artifact_type_id: artifact_type_id.to_string(),
        },
    )];
    node
}

pub fn inject_for_node(task_node: TaskNode, incoming_edges: Vec<DependencyEdge>) -> Inject {
    Inject::new(task_node, incoming_edges)
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

                    origin: DependencyEdgeOrigin::Unrecorded,
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
    let artifact_id = format!("artifact-{outcome_id}");
    Outcome {
        outcome_id: outcome_id.to_string(),
        task_instance_id: task_instance_id.to_string(),
        lifecycle_epoch: claim.lifecycle_epoch,
        claim_id: claim.claim_id.clone(),
        claim_revision: claim.claim_revision,
        status: OutcomeStatus::Succeeded,
        error: None,
        artifact_records: vec![ArtifactRecord {
            artifact_id,
            artifact_type_id: "docs_patch".to_string(),
            schema_version: 1,
            content: json!({
                "outcome_id": outcome_id,
                "task_instance_id": task_instance_id,
            }),
            producer: ArtifactProducerRef {
                task_id: format!("compiled-{task_instance_id}"),
                capability_instance_id: "write".to_string(),
                invocation_id: Some(format!("invoke-{outcome_id}")),
                output_slot_id: Some("patch".to_string()),
            },
        }],
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
