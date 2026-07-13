use meld_events::DomainObjectRef;
use meld_execution::capability::{
    ArtifactSchemaVersionRange, CapabilityCatalog, CapabilityTypeContract, ExecutionClass,
    ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec, ScopeContract,
};
use meld_execution::goals::{AddGoalCommand, GoalCommandMetadata, PersistentGoalSetStore};
use meld_execution::planning::{
    CandidateStatus, ExecutionCompositionLowerer, MethodLibrary, MethodSourceRef,
    MethodVerification, PlanningDiagnosticCode, PlanningInputError, PlanningPerspectiveRef,
    PlanningProjectionError, PlanningRequest, PlanningResult, PlanningRuntime,
    PlanningRuntimeActor, PlanningRuntimeActorGoalResult, PlanningRuntimeActorRequest,
    PlanningWorldStateFrameRef, PlanningWorldStateProjection, PlanningWorldStateRequest,
    VerifiedMethodEntry,
};
use meld_execution::task::TaskCompiler;
use meld_execution::task_network::{Response, SledTaskNetworkStore};
use meld_lang::{
    Composition, Condition, CostEstimate, Effect, Goal, GoalLifecycle, GoalPriority, GoalSource,
    Literal, Method, Operator, Proposition, Resolution, SlotConstraint, Step, StepKind, Term,
    WorldState,
};

fn node(id: &str) -> Term {
    Term::Object(DomainObjectRef::new("workspace", "node", id).unwrap())
}

fn docs_target(scope: Term) -> Proposition {
    Proposition::Holds {
        subject: scope,
        dimension: Term::Dimension("docs_freshness".to_string()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
    }
}

fn goal_with_ceiling(ceiling: Option<CostEstimate>) -> Goal {
    Goal {
        goal_id: "goal-docs".to_string(),
        agent_id: "docs-agent".to_string(),
        target: docs_target(node("readme")),
        priority: GoalPriority {
            urgency: 1,
            cost_ceiling: ceiling,
        },
        source: GoalSource::BeliefDivergence {
            dimension: "docs_freshness".to_string(),
            observed: "low confidence".to_string(),
            desired: "fresh docs".to_string(),
        },
        lifecycle: GoalLifecycle::Active,
    }
}

fn derived_frame(
    request: &PlanningWorldStateRequest,
    world_state: &WorldState,
) -> PlanningWorldStateFrameRef {
    PlanningWorldStateFrameRef::from_authority(
        format!("frame-{}", request.goal_id),
        format!("projection-request-{}", request.goal_id),
        request.canonical_hash().unwrap(),
        "world_model.planner.v1",
        format!("projection-hash-{}", request.goal_id),
        meld_execution::planning::world_state::canonical_world_state_hash(world_state).unwrap(),
        request,
        world_state,
        vec!["source".to_string()],
        Vec::new(),
    )
    .unwrap()
}

fn request(goal: Goal, world_state: WorldState) -> PlanningRequest {
    let subject = match &goal.target {
        Proposition::Holds {
            subject: Term::Object(subject),
            ..
        } => subject.clone(),
        _ => panic!("planning fixture goal must carry one object subject"),
    };
    let world_state_request = PlanningWorldStateRequest {
        goal_id: goal.goal_id.clone(),
        agent_id: goal.agent_id.clone(),
        subject,
        source_seq: 1,
        target: goal.target.clone(),
        perspective: PlanningPerspectiveRef::new("agent", "default").unwrap(),
        branch_id: "main".to_string(),
        requested_dimensions: vec!["docs_freshness".to_string()],
        required_preconditions: vec![],
    };
    let world_state_frame = derived_frame(&world_state_request, &world_state);
    PlanningRequest {
        request_id: "request-1".to_string(),
        world_state_request,
        goal,
        world_state,
        world_state_frame,
    }
}

fn catalog() -> CapabilityCatalog {
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
            input_contract: vec![InputSlotSpec {
                slot_id: "source".to_string(),
                accepted_artifact_type_ids: vec!["source_doc".to_string()],
                schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
                required: false,
                cardinality: InputCardinality::One,
            }],
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

fn docs_method(method_id: &str) -> Method {
    Method {
        method_id: method_id.to_string(),
        trigger: docs_target(Term::Variable("?node".to_string())),
        preconditions: vec![Proposition::Accessible {
            scope: Term::Variable("?node".to_string()),
        }],
        composition: Composition {
            steps: vec![Step {
                step_id: "write".to_string(),
                kind: StepKind::Op(Operator {
                    operator_id: "write".to_string(),
                    preconditions: vec![Proposition::Accessible {
                        scope: Term::Variable("?node".to_string()),
                    }],
                    effects: vec![Effect::Update {
                        subject: Term::Variable("?node".to_string()),
                        dimension: Term::Dimension("docs_freshness".to_string()),
                        value: Term::Literal(Literal::Number(0.95)),
                    }],
                    cost: CostEstimate {
                        time_ms: 10_000,
                        money_microdollars: 25_000,
                        provider_calls: 1,
                    },
                    resolution: Resolution {
                        requires_inputs: vec![],
                        requires_outputs: vec![SlotConstraint {
                            artifact_type: Term::ArtifactType("docs_patch".to_string()),
                            required: true,
                        }],
                        scope_kind: Some("filesystem".to_string()),
                        tags: vec!["docs".to_string()],
                        specific: None,
                    },
                }),
            }],
            edges: vec![],
        },
        net_effects: vec![Effect::Update {
            subject: Term::Variable("?node".to_string()),
            dimension: Term::Dimension("docs_freshness".to_string()),
            value: Term::Literal(Literal::Number(0.95)),
        }],
        cost: CostEstimate {
            time_ms: 10_000,
            money_microdollars: 25_000,
            provider_calls: 1,
        },
        preference: 1,
    }
}

fn runtime(methods: Vec<Method>) -> PlanningRuntime {
    let catalog = catalog();
    let library = MethodLibrary::from_methods(methods, &catalog);
    PlanningRuntime::new(library, catalog)
}

fn planning_actor() -> PlanningRuntimeActor<TaskCompiler> {
    PlanningRuntimeActor::new(
        runtime(vec![docs_method("refresh")]),
        ExecutionCompositionLowerer::new(TaskCompiler::new(), catalog()),
    )
}

fn actor_request(limit: Option<usize>) -> PlanningRuntimeActorRequest {
    PlanningRuntimeActorRequest {
        network_id: "network-docs".to_string(),
        perspective: PlanningPerspectiveRef::new("agent", "default").unwrap(),
        branch_id: "main".to_string(),
        requested_dimensions: vec!["docs_freshness".to_string()],
        required_preconditions: vec![],
        limit,
    }
}

fn open_goal_store_with_active_goal(goal: Goal) -> PersistentGoalSetStore {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = PersistentGoalSetStore::new(db).unwrap();
    store
        .add_goal(AddGoalCommand {
            metadata: GoalCommandMetadata {
                command_id: format!("command-add-{}", goal.goal_id),
                source_identity: None,
                seq: 1,
            },
            goal,
        })
        .unwrap();
    store
}

fn open_task_network_store() -> SledTaskNetworkStore {
    let db = sled::Config::new().temporary(true).open().unwrap();
    SledTaskNetworkStore::open(db, "network-docs").unwrap()
}

fn unsatisfied_state() -> WorldState {
    WorldState::new(vec![
        Proposition::Accessible {
            scope: node("readme"),
        },
        Proposition::Holds {
            subject: node("readme"),
            dimension: Term::Dimension("docs_freshness".to_string()),
            condition: Condition::Equals(Term::Literal(Literal::Number(0.2))),
        },
    ])
    .unwrap()
}

#[test]
fn satisfied_goal_returns_satisfied() {
    let state = WorldState::new(vec![Proposition::Holds {
        subject: node("readme"),
        dimension: Term::Dimension("docs_freshness".to_string()),
        condition: Condition::Equals(Term::Literal(Literal::Number(0.9))),
    }])
    .unwrap();
    let result = runtime(vec![docs_method("refresh")])
        .plan_goal(request(goal_with_ceiling(None), state))
        .unwrap();

    assert!(matches!(result, PlanningResult::Satisfied(_)));
}

#[test]
fn missing_projection_data_returns_indeterminate() {
    let result = runtime(vec![docs_method("refresh")])
        .plan_goal(request(goal_with_ceiling(None), WorldState::empty()))
        .unwrap();

    assert!(matches!(result, PlanningResult::Indeterminate(_)));
}

#[test]
fn low_confidence_docs_goal_returns_composed() {
    let result = runtime(vec![docs_method("refresh")])
        .plan_goal(request(goal_with_ceiling(None), unsatisfied_state()))
        .unwrap();

    let PlanningResult::Composed(composed) = result else {
        panic!("expected composed");
    };
    assert_eq!(composed.method_id, "refresh");
    assert!(composed
        .composition_id
        .starts_with("execution-composition-"));
    assert_eq!(composed.projected_effects.len(), 1);
}

#[test]
fn no_trigger_match_returns_no_applicable_method() {
    let mut method = docs_method("other");
    method.trigger = Proposition::Accessible {
        scope: Term::Variable("?node".to_string()),
    };
    let result = runtime(vec![method])
        .plan_goal(request(goal_with_ceiling(None), unsatisfied_state()))
        .unwrap();

    let PlanningResult::NoApplicableMethod(report) = result else {
        panic!("expected no applicable method");
    };
    assert_eq!(report.candidates[0].status, CandidateStatus::TriggerMiss);
}

#[test]
fn failed_precondition_returns_no_applicable_method() {
    let state = WorldState::new(vec![Proposition::Holds {
        subject: node("readme"),
        dimension: Term::Dimension("docs_freshness".to_string()),
        condition: Condition::Equals(Term::Literal(Literal::Number(0.2))),
    }])
    .unwrap();
    let result = runtime(vec![docs_method("refresh")])
        .plan_goal(request(goal_with_ceiling(None), state))
        .unwrap();

    let PlanningResult::NoApplicableMethod(report) = result else {
        panic!("expected no applicable method");
    };
    assert_eq!(
        report.candidates[0].status,
        CandidateStatus::PreconditionsUnsatisfied
    );
}

#[test]
fn indeterminate_precondition_returns_no_applicable_method() {
    let mut method = docs_method("refresh");
    method.preconditions = vec![Proposition::Holds {
        subject: Term::Variable("?node".to_string()),
        dimension: Term::Dimension("reviewed".to_string()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
    }];
    let result = runtime(vec![method])
        .plan_goal(request(goal_with_ceiling(None), unsatisfied_state()))
        .unwrap();

    let PlanningResult::NoApplicableMethod(report) = result else {
        panic!("expected no applicable method");
    };
    assert_eq!(
        report.candidates[0].status,
        CandidateStatus::PreconditionsIndeterminate
    );
}

#[test]
fn cost_ceiling_rejection_returns_no_applicable_method() {
    let result = runtime(vec![docs_method("refresh")])
        .plan_goal(request(
            goal_with_ceiling(Some(CostEstimate {
                time_ms: 1,
                money_microdollars: 1,
                provider_calls: 0,
            })),
            unsatisfied_state(),
        ))
        .unwrap();

    let PlanningResult::NoApplicableMethod(report) = result else {
        panic!("expected no applicable method");
    };
    assert_eq!(report.candidates[0].status, CandidateStatus::CostRejected);
}

#[test]
fn projected_effects_miss_returns_no_applicable_method() {
    let mut method = docs_method("refresh");
    method.net_effects = vec![Effect::Update {
        subject: Term::Variable("?node".to_string()),
        dimension: Term::Dimension("other_dimension".to_string()),
        value: Term::Literal(Literal::Number(0.95)),
    }];
    let result = runtime(vec![method])
        .plan_goal(request(goal_with_ceiling(None), unsatisfied_state()))
        .unwrap();

    let PlanningResult::NoApplicableMethod(report) = result else {
        panic!("expected no applicable method");
    };
    assert_eq!(report.candidates[0].status, CandidateStatus::EffectMiss);
}

#[test]
fn invalid_composition_after_substitution_returns_invalid_method() {
    let mut method = docs_method("refresh");
    method
        .composition
        .steps
        .push(method.composition.steps[0].clone());
    let catalog = catalog();
    let library = MethodLibrary {
        entries: vec![VerifiedMethodEntry {
            method,
            source_ref: MethodSourceRef::InMemory {
                label: "bypass".to_string(),
            },
            verification: MethodVerification {
                diagnostics: vec![],
                operator_resolutions: vec![],
            },
        }],
        invalid: vec![],
    };
    let result = PlanningRuntime::new(library, catalog)
        .plan_goal(request(goal_with_ceiling(None), unsatisfied_state()))
        .unwrap();

    let PlanningResult::InvalidMethod(report) = result else {
        panic!("expected invalid method");
    };
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == PlanningDiagnosticCode::CompositionValidationFailed
    }));
}

#[test]
fn repeated_same_input_returns_same_composition_and_diagnostics() {
    let runtime = runtime(vec![docs_method("refresh")]);
    let first = runtime
        .plan_goal(request(goal_with_ceiling(None), unsatisfied_state()))
        .unwrap();
    let second = runtime
        .plan_goal(request(goal_with_ceiling(None), unsatisfied_state()))
        .unwrap();

    assert_eq!(first, second);
}

#[test]
fn invalid_request_shape_returns_input_errors() {
    let runtime = runtime(vec![docs_method("refresh")]);

    let mut missing_id = request(goal_with_ceiling(None), unsatisfied_state());
    missing_id.request_id.clear();
    assert_eq!(
        runtime.plan_goal(missing_id).unwrap_err(),
        PlanningInputError::MissingRequestId
    );

    let mut proposed_goal = goal_with_ceiling(None);
    proposed_goal.lifecycle = GoalLifecycle::Proposed;
    assert!(matches!(
        runtime
            .plan_goal(request(proposed_goal, unsatisfied_state()))
            .unwrap_err(),
        PlanningInputError::NonActiveGoal { .. }
    ));

    let mut nonground_goal = goal_with_ceiling(None);
    nonground_goal.target = Proposition::Accessible {
        scope: Term::Variable("?node".to_string()),
    };
    let mut nonground_request = request(goal_with_ceiling(None), unsatisfied_state());
    nonground_request.goal = nonground_goal;
    assert!(matches!(
        runtime.plan_goal(nonground_request).unwrap_err(),
        PlanningInputError::NonGroundGoal { .. }
    ));
}

#[test]
fn planning_actor_reads_active_goals_and_submits_lowered_composition() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let expected_goal_sequence = goal_store.goal_records().unwrap()[0].updated_at_seq;
    let mut task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection_requests = Vec::new();
    let mut projection = |request: PlanningWorldStateRequest| {
        projection_requests.push(request.clone());
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };

    let report = actor
        .run_once(
            &goal_store,
            &mut task_network,
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(projection_requests.len(), 1);
    assert_eq!(projection_requests[0].goal_id, "goal-docs");
    assert_eq!(projection_requests[0].source_seq, expected_goal_sequence);
    assert_eq!(
        projection_requests[0].subject,
        DomainObjectRef::new("workspace", "node", "readme").unwrap()
    );
    assert_eq!(report.actor_id, "execution.planning.runtime");
    assert_eq!(report.active_goal_count, 1);
    assert_eq!(report.attempted, 1);
    assert_eq!(report.committed, 1);
    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    assert!(!report.budget_exhausted);
    assert!(report.input_revision < report.output_revision);
    assert_eq!(task_network.state().tasks.len(), 1);
    assert_eq!(task_network.journal().len(), 1);
    assert!(matches!(
        &report.results[0],
        PlanningRuntimeActorGoalResult::Submitted {
            response: Response::Accepted { .. },
            ..
        }
    ));
}

#[test]
fn planning_actor_repeated_tick_skips_already_materialized_plan() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let mut task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection = |request: PlanningWorldStateRequest| {
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };

    let first = actor
        .run_once(
            &goal_store,
            &mut task_network,
            &mut projection,
            actor_request(None),
        )
        .unwrap();
    let first_output_revision = task_network.state().revision;
    let second = actor
        .run_once(
            &goal_store,
            &mut task_network,
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(first.committed, 1);
    assert_eq!(second.committed, 0);
    assert!(second.retryable_errors.is_empty());
    assert!(second.fatal_errors.is_empty());
    assert_eq!(second.input_revision, first_output_revision);
    assert_eq!(second.output_revision, first_output_revision);
    assert_eq!(task_network.journal().len(), 1);
    assert!(matches!(
        &second.results[0],
        PlanningRuntimeActorGoalResult::Lowered { .. }
    ));
}

#[test]
fn planning_actor_bounded_selection_uses_urgency_then_stable_goal_id() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let goal_store = PersistentGoalSetStore::new(db).unwrap();
    for (seq, goal_id, urgency) in [
        (1, "goal-later", 5),
        (2, "goal-urgent-b", 1),
        (3, "goal-urgent-a", 1),
    ] {
        let mut goal = goal_with_ceiling(None);
        goal.goal_id = goal_id.to_string();
        goal.priority.urgency = urgency;
        goal_store
            .add_goal(AddGoalCommand {
                metadata: GoalCommandMetadata {
                    command_id: format!("command-add-{goal_id}"),
                    source_identity: None,
                    seq,
                },
                goal,
            })
            .unwrap();
    }
    let mut task_network = open_task_network_store();
    let actor = planning_actor();
    let mut selected_goal_ids = Vec::new();
    let mut projection = |request: PlanningWorldStateRequest| {
        selected_goal_ids.push(request.goal_id.clone());
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };

    let report = actor
        .run_once(
            &goal_store,
            &mut task_network,
            &mut projection,
            actor_request(Some(2)),
        )
        .unwrap();

    assert_eq!(selected_goal_ids, vec!["goal-urgent-a", "goal-urgent-b"]);
    assert_eq!(report.attempted, 2);
    assert!(report.budget_exhausted);
}

#[test]
fn planning_actor_projection_failure_does_not_mutate_task_network_state() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let mut task_network = open_task_network_store();
    let input_revision = task_network.state().revision;
    let input_hash = task_network.state().state_hash.clone();
    let actor = planning_actor();
    let mut projection_requests = Vec::new();
    let mut projection = |request: PlanningWorldStateRequest| {
        projection_requests.push(request);
        Err(PlanningProjectionError::retryable("projection unavailable"))
    };

    let report = actor
        .run_once(
            &goal_store,
            &mut task_network,
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(projection_requests.len(), 1);
    assert_eq!(report.active_goal_count, 1);
    assert_eq!(report.attempted, 1);
    assert_eq!(report.committed, 0);
    assert_eq!(report.input_revision, input_revision);
    assert_eq!(report.output_revision, input_revision);
    assert_eq!(task_network.state().revision, input_revision);
    assert_eq!(task_network.state().state_hash, input_hash);
    assert!(task_network.state().tasks.is_empty());
    assert!(task_network.journal().is_empty());
    assert_eq!(report.retryable_errors.len(), 1);
    assert!(report.fatal_errors.is_empty());
    assert!(matches!(
        &report.results[0],
        PlanningRuntimeActorGoalResult::ProjectionFailed {
            retryable: true,
            ..
        }
    ));
}

#[test]
fn planning_actor_rejects_ambiguous_projection_subject_before_port_handoff() {
    let first = DomainObjectRef::new("workspace", "node", "first").unwrap();
    let second = DomainObjectRef::new("workspace", "node", "second").unwrap();
    let mut goal = goal_with_ceiling(None);
    goal.target = Proposition::Related {
        src: Term::Object(first),
        relation: Term::Literal(Literal::Text("depends_on".to_string())),
        dst: Term::Object(second),
    };
    let goal_store = open_goal_store_with_active_goal(goal);
    let mut task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection_called = false;
    let mut projection = |_request: PlanningWorldStateRequest| {
        projection_called = true;
        Err(PlanningProjectionError::fatal("must not be called"))
    };

    let report = actor
        .run_once(
            &goal_store,
            &mut task_network,
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert!(!projection_called);
    assert_eq!(report.committed, 0);
    assert!(report.retryable_errors.is_empty());
    assert_eq!(report.fatal_errors.len(), 1);
    assert_eq!(
        report.fatal_errors[0].code,
        "planning_projection_request_invalid"
    );
    assert!(report.fatal_errors[0].message.contains("ambiguous"));
    assert!(matches!(
        &report.results[0],
        PlanningRuntimeActorGoalResult::PlanningFailed {
            error: PlanningInputError::IdentityMismatch { .. },
            ..
        }
    ));
    assert_eq!(task_network.state().revision, 0);
}
