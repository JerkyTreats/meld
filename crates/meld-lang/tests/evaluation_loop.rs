use meld_events::DomainObjectRef;
use meld_lang::{
    evaluate, substitute, validate, Bindings, Composition, Condition, CostEstimate, Edge, EdgeKind,
    Effect, EvalResult, Goal, GoalLifecycle, GoalPriority, GoalSource, Literal, Method, Operator,
    Proposition, Resolution, SlotConstraint, Step, StepKind, Term, WorldState,
};

fn node(id: &str) -> Term {
    Term::Object(DomainObjectRef::new("workspace", "node", id).unwrap())
}

fn confidence_goal(scope: Term) -> Proposition {
    Proposition::Holds {
        subject: scope,
        dimension: Term::Dimension("docs_freshness".into()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
    }
}

fn docs_method() -> Method {
    let write_operator = Operator {
        operator_id: "write_docs".into(),
        preconditions: vec![Proposition::Accessible {
            scope: Term::Variable("?node".into()),
        }],
        effects: vec![
            Effect::Assert(Proposition::Exists {
                scope: Term::Variable("?node".into()),
                artifact_type: Term::ArtifactType("docs_patch".into()),
            }),
            Effect::Update {
                subject: Term::Variable("?node".into()),
                dimension: Term::Dimension("docs_freshness".into()),
                value: Term::Literal(Literal::Number(0.95)),
            },
        ],
        cost: CostEstimate {
            time_ms: 10_000,
            money_microdollars: 25_000,
            provider_calls: 1,
        },
        resolution: Resolution {
            requires_inputs: vec![],
            requires_outputs: vec![SlotConstraint {
                artifact_type_id: "docs_patch".into(),
                required: true,
            }],
            scope_kind: Some("filesystem".into()),
            tags: vec!["write".into(), "docs".into()],
            specific: None,
        },
    };

    Method {
        method_id: "refresh_docs".into(),
        trigger: confidence_goal(Term::Variable("?node".into())),
        preconditions: vec![Proposition::Accessible {
            scope: Term::Variable("?node".into()),
        }],
        composition: Composition {
            steps: vec![Step {
                step_id: "write".into(),
                kind: StepKind::Op(write_operator),
            }],
            edges: vec![],
        },
        net_effects: vec![Effect::Update {
            subject: Term::Variable("?node".into()),
            dimension: Term::Dimension("docs_freshness".into()),
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

#[test]
fn full_docs_freshness_loop() {
    let scope = node("readme");
    let goal = Goal {
        goal_id: "goal-docs".into(),
        agent_id: "docs-agent".into(),
        target: confidence_goal(scope.clone()),
        priority: GoalPriority {
            urgency: 5,
            cost_ceiling: Some(CostEstimate {
                time_ms: 30_000,
                money_microdollars: 100_000,
                provider_calls: 3,
            }),
        },
        source: GoalSource::BeliefDivergence {
            dimension: "docs_freshness".into(),
            observed: "low confidence".into(),
            desired: "confidence above threshold".into(),
        },
        lifecycle: GoalLifecycle::Active,
    };
    let initial = WorldState::new(vec![
        Proposition::Accessible {
            scope: scope.clone(),
        },
        Proposition::Holds {
            subject: scope.clone(),
            dimension: Term::Dimension("docs_freshness".into()),
            condition: Condition::Equals(Term::Literal(Literal::Number(0.2))),
        },
    ])
    .unwrap();
    assert_eq!(
        evaluate(&initial, &goal.target),
        EvalResult::Unsatisfied {
            gap: vec![goal.target.clone()]
        }
    );

    let method = docs_method();
    let bindings = meld_lang::unify(&method.trigger, &goal.target).unwrap();
    assert_eq!(bindings.get("?node"), Some(&scope));
    assert!(!method
        .cost
        .exceeds(goal.priority.cost_ceiling.as_ref().unwrap()));

    let composition = substitute(&method.composition, &bindings).unwrap();
    let validation = validate(&composition);
    assert!(validation.valid, "{validation:?}");

    let effects = composition
        .steps
        .iter()
        .flat_map(|step| match &step.kind {
            StepKind::Op(operator) => operator.effects.clone(),
            StepKind::Goal(_) => vec![],
        })
        .collect::<Vec<_>>();
    let projected = initial.apply(&effects).unwrap();
    assert_eq!(evaluate(&projected, &goal.target), EvalResult::Satisfied);
}

#[test]
fn indeterminate_path_becomes_satisfied_after_observation() {
    let scope = node("unknown");
    let goal = confidence_goal(scope.clone());
    let empty = WorldState::empty();

    assert_eq!(
        evaluate(&empty, &goal),
        EvalResult::Indeterminate {
            missing: vec![Term::Dimension("docs_freshness".into())]
        }
    );

    let observed = empty
        .apply(&[Effect::Update {
            subject: scope,
            dimension: Term::Dimension("docs_freshness".into()),
            value: Term::Literal(Literal::Number(0.8)),
        }])
        .unwrap();

    assert_eq!(evaluate(&observed, &goal), EvalResult::Satisfied);
}

#[test]
fn method_loaded_from_json_matches_programmatic_method() {
    let method = docs_method();
    let encoded = serde_json::to_string(&method).unwrap();
    let decoded: Method = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded, method);
    assert!(meld_lang::unify(&decoded.trigger, &confidence_goal(node("readme"))).is_some());
}

#[test]
fn multi_step_composition_with_conditional_edge_validates() {
    let observe = Step {
        step_id: "observe".into(),
        kind: StepKind::Op(Operator {
            operator_id: "observe".into(),
            preconditions: vec![],
            effects: vec![Effect::Assert(Proposition::Exists {
                scope: node("readme"),
                artifact_type: Term::ArtifactType("change_summary".into()),
            })],
            cost: CostEstimate::zero(),
            resolution: Resolution {
                requires_inputs: vec![],
                requires_outputs: vec![SlotConstraint {
                    artifact_type_id: "change_summary".into(),
                    required: true,
                }],
                scope_kind: Some("filesystem".into()),
                tags: vec!["observe".into()],
                specific: None,
            },
        }),
    };
    let write = substitute(
        &docs_method().composition,
        &Bindings::empty()
            .bind("?node".into(), node("readme"))
            .unwrap(),
    )
    .unwrap()
    .steps
    .into_iter()
    .next()
    .unwrap();
    let composition = Composition {
        steps: vec![observe, write],
        edges: vec![Edge {
            from: "observe".into(),
            to: "write".into(),
            kind: EdgeKind::Conditional {
                field_path: "should_execute".into(),
                guard: Condition::Equals(Term::Literal(Literal::Bool(true))),
            },
        }],
    };

    assert!(validate(&composition).valid);
}

#[test]
fn recursive_decomposition_with_goal_steps_validates() {
    let composition = Composition {
        steps: vec![Step {
            step_id: "subgoal".into(),
            kind: StepKind::Goal(Proposition::Exists {
                scope: node("readme"),
                artifact_type: Term::ArtifactType("docs_patch".into()),
            }),
        }],
        edges: vec![],
    };

    assert!(validate(&composition).valid);
}

#[test]
fn bindings_merge_for_method_flow() {
    let a = Bindings::empty()
        .bind("?node".into(), node("readme"))
        .unwrap();
    let b = Bindings::empty()
        .bind("?artifact".into(), Term::ArtifactType("docs_patch".into()))
        .unwrap();
    let merged = a.merge(&b).unwrap();

    assert!(merged.get("?node").is_some());
    assert!(merged.get("?artifact").is_some());
}
