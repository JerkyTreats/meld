use meld_events::DomainObjectRef;
use meld_execution::capability::{
    ArtifactSchemaVersionRange, CapabilityCatalog, CapabilityTypeContract, ExecutionClass,
    ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec, ScopeContract,
};
use meld_execution::goals::{
    AddGoalCommand, ExecutionStrategyAuthorization, GoalCommandMetadata, PersistentGoalSetStore,
};
use meld_execution::planning::{
    ActionArtifactMeaning, ActionOutcomeContractRef, ActionRealizationRoute,
    AvailableActionBinding, AvailableActionSet, CandidateStatus, ExecutionCompositionLowerer,
    MethodLibrary, MethodRealizationBinding, MethodSourceRef, MethodVerification,
    PlanningDiagnosticCode, PlanningInputError, PlanningProjectionError, PlanningRequest,
    PlanningResult, PlanningRuntime, PlanningRuntimeActor, PlanningRuntimeActorGoalResult,
    PlanningRuntimeActorRequest, PlanningWorldStateFrameRef, PlanningWorldStateProjection,
    PlanningWorldStateRequest, VerifiedMethodEntry,
};
use meld_execution::task::TaskCompiler;
use meld_execution::task_network::{Response, SledTaskNetworkStore};
use meld_lang::{
    Bindings, CapabilityRef, Composition, Condition, CostEstimate, Effect, Goal, GoalLifecycle,
    GoalPriority, GoalSource, Literal, Method, Operator, Proposition, Resolution, SlotConstraint,
    Step, StepKind, Term, WorldState,
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

fn frame() -> PlanningWorldStateFrameRef {
    revised_frame("frame-1", "source")
}

fn revised_frame(frame_id: &str, source_ref: &str) -> PlanningWorldStateFrameRef {
    PlanningWorldStateFrameRef {
        frame_id: frame_id.to_string(),
        projection_version: "world_model.planner.v1".to_string(),
        perspective_id: "default".to_string(),
        branch_id: "main".to_string(),
        source_refs: vec![source_ref.to_string()],
        warnings: vec![],
    }
}

fn request(goal: Goal, world_state: WorldState) -> PlanningRequest {
    PlanningRequest {
        request_id: "request-1".to_string(),
        world_state_request: PlanningWorldStateRequest {
            goal_id: goal.goal_id.clone(),
            agent_id: goal.agent_id.clone(),
            target: goal.target.clone(),
            perspective_id: "default".to_string(),
            branch_id: "main".to_string(),
            requested_dimensions: vec!["docs_freshness".to_string()],
            required_preconditions: vec![],
        },
        goal,
        world_state,
        world_state_frame: frame(),
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

#[test]
fn authorized_planning_bypasses_method_search_and_revalidates_exact_composition() {
    let runtime = runtime(Vec::new());
    let goal = goal_with_ceiling(None);
    let composition = Composition {
        steps: vec![Step {
            step_id: "write".into(),
            kind: StepKind::Op(Operator {
                operator_id: "write".into(),
                preconditions: vec![Proposition::Accessible {
                    scope: node("readme"),
                }],
                effects: vec![Effect::Assert(Proposition::Exists {
                    scope: node("readme"),
                    artifact_type: Term::ArtifactType("docs_patch".into()),
                })],
                cost: CostEstimate::zero(),
                resolution: Resolution {
                    requires_inputs: Vec::new(),
                    requires_outputs: vec![SlotConstraint {
                        artifact_type: Term::ArtifactType("docs_patch".into()),
                        required: true,
                    }],
                    scope_kind: Some("filesystem".into()),
                    tags: Vec::new(),
                    specific: Some(CapabilityRef {
                        capability_type_id: "docs.write".into(),
                        capability_version: 1,
                    }),
                },
            }),
        }],
        edges: Vec::new(),
    };
    let authorization = ExecutionStrategyAuthorization {
        authorization_id: "authorization-1".into(),
        agent_decision_id: "decision-1".into(),
        candidate_id: "candidate-1".into(),
        goal_id: goal.goal_id.clone(),
        planner_snapshot_id: frame().frame_id,
        composition: composition.clone(),
        bindings: Bindings::empty()
            .bind("scope".into(), node("readme"))
            .unwrap(),
        capability_contract_ids: vec![catalog().get("docs.write", 1).unwrap().content_identity()],
        method_id: None,
    };
    let world_state = WorldState::new(vec![Proposition::Accessible {
        scope: node("readme"),
    }])
    .unwrap();
    let result = runtime
        .plan_authorized_goal(request(goal, world_state), &authorization)
        .unwrap();
    let PlanningResult::Composed(composed) = result else {
        panic!("expected exact authorized composition");
    };

    assert_eq!(composed.composition, composition);
    assert_eq!(composed.method_id, "candidate-1");
    assert_eq!(composed.bindings.get("scope"), Some(&node("readme")));
}

#[test]
fn authorized_planning_rejects_drifted_contract_identity() {
    let runtime = runtime(Vec::new());
    let goal = goal_with_ceiling(None);
    let composition = Composition {
        steps: vec![Step {
            step_id: "write".into(),
            kind: StepKind::Op(Operator {
                operator_id: "write".into(),
                preconditions: Vec::new(),
                effects: Vec::new(),
                cost: CostEstimate::zero(),
                resolution: Resolution {
                    requires_inputs: Vec::new(),
                    requires_outputs: vec![SlotConstraint {
                        artifact_type: Term::ArtifactType("docs_patch".into()),
                        required: true,
                    }],
                    scope_kind: Some("filesystem".into()),
                    tags: Vec::new(),
                    specific: Some(CapabilityRef {
                        capability_type_id: "docs.write".into(),
                        capability_version: 1,
                    }),
                },
            }),
        }],
        edges: Vec::new(),
    };
    let authorization = ExecutionStrategyAuthorization {
        authorization_id: "authorization-drift".into(),
        agent_decision_id: "decision-drift".into(),
        candidate_id: "candidate-drift".into(),
        goal_id: goal.goal_id.clone(),
        planner_snapshot_id: frame().frame_id,
        composition,
        bindings: Bindings::empty(),
        capability_contract_ids: vec!["stale-contract-identity".into()],
        method_id: None,
    };
    let result = runtime
        .plan_authorized_goal(request(goal, WorldState::empty()), &authorization)
        .unwrap();
    assert!(matches!(result, PlanningResult::InvalidMethod(_)));
}

fn planning_actor() -> PlanningRuntimeActor<TaskCompiler> {
    PlanningRuntimeActor::new(
        runtime(vec![docs_method("refresh")]),
        ExecutionCompositionLowerer::new(TaskCompiler::new(), catalog()),
    )
}

fn actor_request(limit: Option<usize>) -> PlanningRuntimeActorRequest {
    actor_request_with_actions(limit, AvailableActionSet { actions: vec![] }, vec![])
}

fn actor_request_with_actions(
    limit: Option<usize>,
    available_actions: AvailableActionSet,
    method_realizations: Vec<MethodRealizationBinding>,
) -> PlanningRuntimeActorRequest {
    PlanningRuntimeActorRequest {
        network_id: "network-docs".to_string(),
        perspective_id: "default".to_string(),
        branch_id: "main".to_string(),
        requested_dimensions: vec!["docs_freshness".to_string()],
        required_preconditions: vec![],
        available_actions,
        method_realizations,
        limit,
    }
}

fn package_route_action(action_id: &str, workflow_id: &str) -> AvailableActionBinding {
    AvailableActionBinding {
        action_id: action_id.to_string(),
        artifact: ActionArtifactMeaning {
            artifact_type_id: "docs_patch".to_string(),
            schema_version: 1,
        },
        outcome_contract: ActionOutcomeContractRef {
            outcome_contract_id: "execution.package.aggregate.v1".to_string(),
        },
        realization: ActionRealizationRoute::TaskPackageWorkflow {
            package_id: "docs_writer".to_string(),
            workflow_id: workflow_id.to_string(),
        },
    }
}

fn refresh_association(action_id: &str) -> Vec<MethodRealizationBinding> {
    vec![MethodRealizationBinding {
        method_id: "refresh".to_string(),
        action_id: action_id.to_string(),
    }]
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
    assert!(matches!(
        runtime
            .plan_goal(request(nonground_goal, unsatisfied_state()))
            .unwrap_err(),
        PlanningInputError::NonGroundGoal { .. }
    ));
}

#[test]
fn planning_actor_reads_active_goals_and_submits_lowered_composition() {
    let mut goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let mut task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection_requests = Vec::new();
    let mut projection = |request: PlanningWorldStateRequest| {
        projection_requests.push(request.clone());
        Ok(PlanningWorldStateProjection {
            world_state: unsatisfied_state(),
            frame: frame(),
        })
    };

    let report = actor
        .run_once(
            &mut goal_store,
            &mut task_network,
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(projection_requests.len(), 1);
    assert_eq!(projection_requests[0].goal_id, "goal-docs");
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
    let mut goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let mut task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection = |_request: PlanningWorldStateRequest| {
        Ok(PlanningWorldStateProjection {
            world_state: unsatisfied_state(),
            frame: frame(),
        })
    };

    let first = actor
        .run_once(
            &mut goal_store,
            &mut task_network,
            &mut projection,
            actor_request(None),
        )
        .unwrap();
    let first_output_revision = task_network.state().revision;
    let second = actor
        .run_once(
            &mut goal_store,
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
fn planning_actor_projection_failure_does_not_mutate_task_network_state() {
    let mut goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
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
            &mut goal_store,
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

fn named_goal(goal_id: &str) -> Goal {
    let mut goal = goal_with_ceiling(None);
    goal.goal_id = goal_id.to_string();
    goal
}

fn add_active_goal(store: &PersistentGoalSetStore, goal: Goal) {
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
}

fn fixed_projection(
    frame_ref: PlanningWorldStateFrameRef,
) -> impl FnMut(PlanningWorldStateRequest) -> Result<PlanningWorldStateProjection, PlanningProjectionError>
{
    move |_request| {
        Ok(PlanningWorldStateProjection {
            world_state: unsatisfied_state(),
            frame: frame_ref.clone(),
        })
    }
}

#[test]
fn planning_actor_limit_is_budget_honest_and_reports_exhaustion() {
    let mut goal_store = open_goal_store_with_active_goal(named_goal("goal-a"));
    add_active_goal(&goal_store, named_goal("goal-b"));
    let mut task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection = fixed_projection(frame());

    let report = actor
        .run_once(
            &mut goal_store,
            &mut task_network,
            &mut projection,
            actor_request(Some(1)),
        )
        .unwrap();

    assert_eq!(report.attempted, 1);
    assert!(report.budget_exhausted);
    // The bounded query reads at most limit + 1 records to report
    // exhaustion without an unbounded scan.
    assert_eq!(report.active_goal_count, 2);
    assert!(matches!(
        &report.results[0],
        PlanningRuntimeActorGoalResult::Submitted { goal_id, .. } if goal_id == "goal-a"
    ));
}

#[test]
fn revised_frame_produces_causally_distinct_plan_and_new_submission() {
    let mut goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let mut task_network = open_task_network_store();
    let actor = planning_actor();

    let composition_id_of =
        |report: &meld_execution::planning::PlanningRuntimeActorReport| match &report.results[0] {
            PlanningRuntimeActorGoalResult::Submitted { plan, .. } => plan.composition_id.clone(),
            other => panic!("expected submitted, got {other:?}"),
        };

    let mut projection = fixed_projection(revised_frame("frame-1", "belief-rev-1"));
    let first = actor
        .run_once(
            &mut goal_store,
            &mut task_network,
            &mut projection,
            actor_request(None),
        )
        .unwrap();
    let replay = actor
        .run_once(
            &mut goal_store,
            &mut task_network,
            &mut projection,
            actor_request(None),
        )
        .unwrap();
    let mut revised_projection = fixed_projection(revised_frame("frame-2", "belief-rev-2"));
    let revised = actor
        .run_once(
            &mut goal_store,
            &mut task_network,
            &mut revised_projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(first.committed, 1);
    assert_eq!(replay.committed, 0);
    assert!(matches!(
        &replay.results[0],
        PlanningRuntimeActorGoalResult::Lowered { .. }
    ));
    assert_eq!(revised.committed, 1);
    assert_ne!(composition_id_of(&first), composition_id_of(&revised));
    assert_eq!(task_network.journal().len(), 2);
}

#[test]
fn revised_source_refs_alone_produce_distinct_composition_identity() {
    let runtime = runtime(vec![docs_method("refresh")]);
    let composition_id = |source_ref: &str| {
        let mut planning_request = request(goal_with_ceiling(None), unsatisfied_state());
        planning_request.world_state_frame.source_refs = vec![source_ref.to_string()];
        match runtime.plan_goal(planning_request).unwrap() {
            PlanningResult::Composed(composed) => composed.composition_id,
            other => panic!("expected composed, got {other:?}"),
        }
    };

    assert_ne!(
        composition_id("belief-rev-1"),
        composition_id("belief-rev-2")
    );
}

#[test]
fn missing_frame_identity_is_rejected_before_planning() {
    let runtime = runtime(vec![docs_method("refresh")]);
    let mut planning_request = request(goal_with_ceiling(None), unsatisfied_state());
    planning_request.world_state_frame.frame_id.clear();

    assert!(matches!(
        runtime.plan_goal(planning_request).unwrap_err(),
        PlanningInputError::MissingWorldStateFrameIdentity { .. }
    ));
}

#[test]
fn docs_action_association_selects_builtin_package_route() {
    let mut goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let mut task_network = open_task_network_store();
    let actor = planning_actor();
    let request = || {
        actor_request_with_actions(
            None,
            AvailableActionSet {
                actions: vec![package_route_action(
                    "docs.refresh_subtree",
                    "docs_writer_thread_v1",
                )],
            },
            refresh_association("docs.refresh_subtree"),
        )
    };
    let package_plan_of =
        |report: &meld_execution::planning::PlanningRuntimeActorReport| match &report.results[0] {
            PlanningRuntimeActorGoalResult::PackageRouteSelected { plan, .. } => plan.clone(),
            other => panic!("expected package route, got {other:?}"),
        };

    let mut projection = fixed_projection(revised_frame("frame-1", "belief-rev-1"));
    let first = actor
        .run_once(
            &mut goal_store,
            &mut task_network,
            &mut projection,
            request(),
        )
        .unwrap();
    let replay = actor
        .run_once(
            &mut goal_store,
            &mut task_network,
            &mut projection,
            request(),
        )
        .unwrap();
    let mut revised_projection = fixed_projection(revised_frame("frame-2", "belief-rev-2"));
    let revised = actor
        .run_once(
            &mut goal_store,
            &mut task_network,
            &mut revised_projection,
            request(),
        )
        .unwrap();

    let plan = package_plan_of(&first);
    assert_eq!(plan.package_id, "docs_writer");
    assert_eq!(plan.workflow_id, "docs_writer_thread_v1");
    assert_eq!(plan.outcome_contract_id, "execution.package.aggregate.v1");
    assert_eq!(plan.goal_id, "goal-docs");
    assert_eq!(plan.method_id, "refresh");
    assert_eq!(plan.action_id, "docs.refresh_subtree");
    assert_eq!(plan.world_state_frame.frame_id, "frame-1");
    // Selection and handoff only: the package owns its own traversal and
    // fan-out, so planning proposes no task network mutation here.
    assert_eq!(first.committed, 0);
    assert!(task_network.state().tasks.is_empty());
    assert!(task_network.journal().is_empty());
    // Identical input reproduces the same plan identity; a revised frame
    // produces a causally distinct one.
    assert_eq!(package_plan_of(&replay).plan_id, plan.plan_id);
    assert_ne!(package_plan_of(&revised).plan_id, plan.plan_id);
    assert!(first.fatal_errors.is_empty() && first.retryable_errors.is_empty());
}

#[test]
fn capability_routed_action_association_keeps_existing_lowering() {
    let mut goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let mut task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection = fixed_projection(frame());
    let request = actor_request_with_actions(
        None,
        AvailableActionSet {
            actions: vec![AvailableActionBinding {
                action_id: "docs.write_node".to_string(),
                artifact: ActionArtifactMeaning {
                    artifact_type_id: "docs_patch".to_string(),
                    schema_version: 1,
                },
                outcome_contract: ActionOutcomeContractRef {
                    outcome_contract_id: "execution.task.outcome.v1".to_string(),
                },
                realization: ActionRealizationRoute::OperatorCapability {
                    operator_id: "write".to_string(),
                    capability_type_id: "docs.write".to_string(),
                    capability_version: 1,
                },
            }],
        },
        refresh_association("docs.write_node"),
    );

    let report = actor
        .run_once(&mut goal_store, &mut task_network, &mut projection, request)
        .unwrap();

    assert_eq!(report.committed, 1);
    assert_eq!(task_network.state().tasks.len(), 1);
    assert!(matches!(
        &report.results[0],
        PlanningRuntimeActorGoalResult::Submitted { .. }
    ));
}

#[test]
fn unavailable_action_association_fails_deterministically() {
    let mut goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let mut task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection = fixed_projection(frame());
    let request = actor_request_with_actions(
        None,
        AvailableActionSet { actions: vec![] },
        refresh_association("docs.refresh_subtree"),
    );

    let report = actor
        .run_once(&mut goal_store, &mut task_network, &mut projection, request)
        .unwrap();

    assert_eq!(report.committed, 0);
    assert!(task_network.journal().is_empty());
    assert_eq!(report.fatal_errors.len(), 1);
    assert_eq!(report.fatal_errors[0].code, "realization_selection_failed");
    assert!(matches!(
        &report.results[0],
        PlanningRuntimeActorGoalResult::RealizationFailed { .. }
    ));
}

#[test]
fn mismatched_workflow_route_fails_deterministically() {
    let mut goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let mut task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection = fixed_projection(frame());
    let request = actor_request_with_actions(
        None,
        AvailableActionSet {
            actions: vec![package_route_action(
                "docs.refresh_subtree",
                "not_the_docs_workflow",
            )],
        },
        refresh_association("docs.refresh_subtree"),
    );

    let report = actor
        .run_once(&mut goal_store, &mut task_network, &mut projection, request)
        .unwrap();

    assert_eq!(report.committed, 0);
    assert!(task_network.journal().is_empty());
    let PlanningRuntimeActorGoalResult::RealizationFailed { error, .. } = &report.results[0] else {
        panic!("expected realization failure, got {:?}", report.results[0]);
    };
    assert!(error.contains("docs_writer_thread_v1"));
}

/// A goal planning attempted but did not compose narrates itself in the
/// result's own vocabulary instead of returning a silent Planned result.
#[test]
fn planning_actor_declares_why_a_goal_did_not_compose() {
    // An unknown dimension leaves the goal indeterminate: the projected
    // world state carries no fact that could decide it.
    let mut goal = goal_with_ceiling(None);
    goal.target = Proposition::Holds {
        subject: node("readme"),
        dimension: Term::Dimension("code_health".to_string()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
    };
    let mut goal_store = open_goal_store_with_active_goal(goal);
    let mut task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection = fixed_projection(frame());

    let report = actor
        .run_once(
            &mut goal_store,
            &mut task_network,
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(report.attempted, 1);
    assert_eq!(report.committed, 0);
    assert_eq!(report.waiting_on.len(), 1, "{report:?}");
    assert_eq!(
        report.waiting_on[0].condition,
        meld_execution::waiting::conditions::WORLD_STATE_INDETERMINATE
    );
    assert_eq!(
        report.waiting_on[0].subject_key.as_deref(),
        Some("goal-docs")
    );

    // A matched trigger whose method preconditions fail declares
    // no_applicable_method with the candidate dispositions.
    let mut goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let mut task_network = open_task_network_store();
    let mut inaccessible_projection = |_request: PlanningWorldStateRequest| {
        Ok(PlanningWorldStateProjection {
            world_state: WorldState::new(vec![Proposition::Holds {
                subject: node("readme"),
                dimension: Term::Dimension("docs_freshness".to_string()),
                condition: Condition::Equals(Term::Literal(Literal::Number(0.2))),
            }])
            .unwrap(),
            frame: frame(),
        })
    };

    let report = actor
        .run_once(
            &mut goal_store,
            &mut task_network,
            &mut inaccessible_projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(report.waiting_on.len(), 1, "{report:?}");
    let declaration = &report.waiting_on[0];
    assert_eq!(
        declaration.condition,
        meld_execution::waiting::conditions::NO_APPLICABLE_METHOD
    );
    assert_eq!(declaration.subject_key.as_deref(), Some("goal-docs"));
    assert!(declaration.detail.contains("refresh"), "{declaration:?}");
}
