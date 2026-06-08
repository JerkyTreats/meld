use meld_events::DomainObjectRef;
use meld_execution::capability::{
    ArtifactSchemaVersionRange, CapabilityCatalog, CapabilityTypeContract, ExecutionClass,
    ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec, ScopeContract,
};
use meld_execution::planning::{
    CandidateStatus, MethodLibrary, MethodSourceRef, MethodVerification, PlanningDiagnosticCode,
    PlanningInputError, PlanningRequest, PlanningResult, PlanningRuntime,
    PlanningWorldStateFrameRef, PlanningWorldStateRequest, VerifiedMethodEntry,
};
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

fn frame() -> PlanningWorldStateFrameRef {
    PlanningWorldStateFrameRef {
        frame_id: "frame-1".to_string(),
        projection_version: "world_model.planner.v1".to_string(),
        perspective_id: "default".to_string(),
        branch_id: "main".to_string(),
        source_refs: vec!["source".to_string()],
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
