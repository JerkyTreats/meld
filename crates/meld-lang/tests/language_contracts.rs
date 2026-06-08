use std::time::Duration;

use meld_events::DomainObjectRef;
use meld_lang::{
    evaluate, substitute, unify, validate, Bindings, Composition, Condition, CostEstimate, Edge,
    EdgeKind, Effect, EvalResult, Literal, Operator, Proposition, Resolution, SlotConstraint, Step,
    StepKind, Term, ValidationError, ValidationWarning, WorldState,
};
use proptest::prelude::*;

fn object(id: &str) -> Term {
    Term::Object(DomainObjectRef::new("workspace", "node", id).unwrap())
}

fn dimension(id: &str) -> Term {
    Term::Dimension(id.to_string())
}

fn number(value: f64) -> Term {
    Term::Literal(Literal::Number(value))
}

fn text(value: &str) -> Term {
    Term::Literal(Literal::Text(value.to_string()))
}

fn holds(subject: Term, dimension: &str, condition: Condition) -> Proposition {
    Proposition::Holds {
        subject,
        dimension: Term::Dimension(dimension.to_string()),
        condition,
    }
}

fn accessible(id: &str) -> Proposition {
    Proposition::Accessible { scope: object(id) }
}

fn exists(scope: Term, artifact_type: &str) -> Proposition {
    Proposition::Exists {
        scope,
        artifact_type: Term::ArtifactType(artifact_type.to_string()),
    }
}

fn operator_step(step_id: &str, cost: CostEstimate, outputs: Vec<&str>) -> Step {
    Step {
        step_id: step_id.to_string(),
        kind: StepKind::Op(Operator {
            operator_id: step_id.to_string(),
            preconditions: Vec::new(),
            effects: vec![Effect::Update {
                subject: object("a"),
                dimension: dimension("confidence"),
                value: number(0.9),
            }],
            cost,
            resolution: Resolution {
                requires_inputs: Vec::new(),
                requires_outputs: outputs
                    .into_iter()
                    .map(|artifact_type| SlotConstraint {
                        artifact_type: Term::ArtifactType(artifact_type.to_string()),
                        required: true,
                    })
                    .collect(),
                scope_kind: None,
                tags: Vec::new(),
                specific: None,
            },
        }),
    }
}

fn zero_step(step_id: &str) -> Step {
    operator_step(step_id, CostEstimate::zero(), vec![])
}

fn warning_kind(result: &meld_lang::ValidationResult, expected: fn(&ValidationWarning) -> bool) {
    assert!(
        result.warnings.iter().any(expected),
        "missing warning in {result:?}"
    );
}

fn error_kind(result: &meld_lang::ValidationResult, expected: fn(&ValidationError) -> bool) {
    assert!(
        result.errors.iter().any(expected),
        "missing error in {result:?}"
    );
}

#[test]
fn grounding_contracts_cover_terms_conditions_and_effects() {
    let variable = Term::Variable("?node".to_string());
    let derived = Term::Derived {
        source_step: "observe".to_string(),
        field_path: "items[0].scope".to_string(),
    };

    assert!(!variable.is_ground());
    assert_eq!(variable.grounding_issue().as_deref(), Some("?node"));
    assert!(!derived.is_ground());
    assert_eq!(
        derived.grounding_issue().as_deref(),
        Some("observe.items[0].scope")
    );

    for condition in [
        Condition::Above(variable.clone()),
        Condition::Below(variable.clone()),
        Condition::Equals(variable.clone()),
        Condition::Within(variable.clone()),
        Condition::Exceeds(variable.clone()),
        Condition::In(vec![text("ready"), variable.clone()]),
    ] {
        assert!(!condition.is_ground(), "{condition:?}");
        assert_eq!(condition.grounding_issue().as_deref(), Some("?node"));
    }

    let effect = Effect::Update {
        subject: object("a"),
        dimension: dimension("confidence"),
        value: variable.clone(),
    };
    assert_eq!(effect.grounding_issue().as_deref(), Some("?node"));

    let nested = Effect::Assert(Proposition::Not(Box::new(exists(variable, "report"))));
    assert_eq!(nested.grounding_issue().as_deref(), Some("?node"));
}

#[test]
fn cost_exceeds_is_strict_and_dimension_wise() {
    let ceiling = CostEstimate {
        time_ms: 300,
        money_microdollars: 400,
        provider_calls: 5,
    };

    assert!(!ceiling.exceeds(&ceiling));
    assert!(!CostEstimate {
        time_ms: 299,
        money_microdollars: 399,
        provider_calls: 4,
    }
    .exceeds(&ceiling));
    assert!(CostEstimate {
        time_ms: 301,
        money_microdollars: 399,
        provider_calls: 4,
    }
    .exceeds(&ceiling));
    assert!(CostEstimate {
        time_ms: 299,
        money_microdollars: 401,
        provider_calls: 4,
    }
    .exceeds(&ceiling));
    assert!(CostEstimate {
        time_ms: 299,
        money_microdollars: 399,
        provider_calls: 6,
    }
    .exceeds(&ceiling));
}

#[test]
fn evaluation_requires_matching_subject_and_dimension() {
    let state = WorldState::new(vec![
        holds(object("a"), "confidence", Condition::Equals(number(0.8))),
        holds(object("a"), "freshness", Condition::Equals(number(0.1))),
        holds(object("b"), "confidence", Condition::Equals(number(0.2))),
    ])
    .unwrap();

    assert_eq!(
        evaluate(
            &state,
            &holds(object("a"), "confidence", Condition::Above(number(0.7)))
        ),
        EvalResult::Satisfied
    );
    assert!(matches!(
        evaluate(
            &state,
            &holds(object("b"), "confidence", Condition::Above(number(0.7)))
        ),
        EvalResult::Unsatisfied { .. }
    ));
    assert!(matches!(
        evaluate(
            &state,
            &holds(
                object("a"),
                "missing_dimension",
                Condition::Above(number(0.7))
            )
        ),
        EvalResult::Indeterminate { .. }
    ));
}

#[test]
fn evaluation_covers_present_absent_in_duration_and_not_semantics() {
    let state = WorldState::new(vec![
        holds(object("a"), "confidence", Condition::Equals(number(0.8))),
        holds(
            object("a"),
            "latency",
            Condition::Equals(Term::Literal(Literal::Duration(Duration::from_secs(5)))),
        ),
        holds(object("a"), "status", Condition::Equals(text("ready"))),
    ])
    .unwrap();

    assert_eq!(
        evaluate(
            &state,
            &holds(object("a"), "confidence", Condition::Present)
        ),
        EvalResult::Satisfied
    );
    assert!(matches!(
        evaluate(&state, &holds(object("a"), "confidence", Condition::Absent)),
        EvalResult::Unsatisfied { .. }
    ));
    assert_eq!(
        evaluate(&state, &holds(object("a"), "missing", Condition::Absent)),
        EvalResult::Satisfied
    );
    assert_eq!(
        evaluate(
            &state,
            &holds(
                object("a"),
                "latency",
                Condition::Below(Term::Literal(Literal::Duration(Duration::from_secs(10))))
            )
        ),
        EvalResult::Satisfied
    );
    assert_eq!(
        evaluate(
            &state,
            &holds(
                object("a"),
                "status",
                Condition::In(vec![text("blocked"), text("ready")])
            )
        ),
        EvalResult::Satisfied
    );
    assert!(matches!(
        evaluate(
            &state,
            &Proposition::Not(Box::new(holds(object("a"), "unknown", Condition::Present)))
        ),
        EvalResult::Unsatisfied { .. }
    ));
}

#[test]
fn evaluation_threshold_boundaries_are_strict() {
    let state = WorldState::new(vec![holds(
        object("a"),
        "confidence",
        Condition::Equals(number(0.7)),
    )])
    .unwrap();

    assert!(matches!(
        evaluate(
            &state,
            &holds(object("a"), "confidence", Condition::Above(number(0.7)))
        ),
        EvalResult::Unsatisfied { .. }
    ));
    assert!(matches!(
        evaluate(
            &state,
            &holds(object("a"), "confidence", Condition::Below(number(0.7)))
        ),
        EvalResult::Unsatisfied { .. }
    ));
    assert_eq!(
        evaluate(
            &state,
            &holds(object("a"), "confidence", Condition::In(vec![number(0.7)]))
        ),
        EvalResult::Satisfied
    );
    assert!(matches!(
        evaluate(
            &state,
            &holds(object("a"), "confidence", Condition::In(vec![number(0.8)]))
        ),
        EvalResult::Unsatisfied { .. }
    ));
    assert_eq!(
        evaluate(
            &state,
            &holds(object("a"), "confidence", Condition::Equals(number(0.7)))
        ),
        EvalResult::Satisfied
    );
    assert!(matches!(
        evaluate(
            &state,
            &holds(object("a"), "confidence", Condition::Equals(number(0.8)))
        ),
        EvalResult::Unsatisfied { .. }
    ));
    assert!(matches!(
        evaluate(
            &state,
            &holds(object("a"), "confidence", Condition::Exceeds(number(0.7)))
        ),
        EvalResult::Unsatisfied { .. }
    ));
    assert_eq!(
        evaluate(
            &state,
            &holds(object("a"), "confidence", Condition::Exceeds(number(0.6)))
        ),
        EvalResult::Satisfied
    );
}

#[test]
fn unification_covers_all_public_proposition_shapes() {
    let concrete_scope = object("a");
    let concrete_artifact = Term::ArtifactType("report".to_string());

    assert_eq!(
        unify(
            &Proposition::Accessible {
                scope: Term::Variable("?scope".to_string())
            },
            &Proposition::Accessible {
                scope: concrete_scope.clone()
            },
        )
        .unwrap()
        .get("?scope"),
        Some(&concrete_scope)
    );
    assert_eq!(
        unify(
            &Proposition::Exists {
                scope: Term::Variable("?scope".to_string()),
                artifact_type: Term::Variable("?artifact".to_string()),
            },
            &Proposition::Exists {
                scope: concrete_scope.clone(),
                artifact_type: concrete_artifact.clone(),
            },
        )
        .unwrap()
        .get("?artifact"),
        Some(&concrete_artifact)
    );
    assert!(unify(
        &Proposition::Not(Box::new(accessible("a"))),
        &Proposition::Not(Box::new(accessible("a"))),
    )
    .is_some());
    assert!(unify(
        &holds(
            Term::Variable("?scope".to_string()),
            "status",
            Condition::In(vec![Term::Variable("?status".to_string())])
        ),
        &holds(object("a"), "status", Condition::In(vec![text("ready")]))
    )
    .is_some());
    assert!(unify(
        &holds(object("a"), "confidence", Condition::Present),
        &holds(object("a"), "confidence", Condition::Present),
    )
    .is_some());
    assert!(unify(
        &holds(object("a"), "confidence", Condition::Absent),
        &holds(object("a"), "confidence", Condition::Absent),
    )
    .is_some());
    assert!(unify(
        &holds(object("a"), "status", Condition::In(vec![text("ready")])),
        &holds(
            object("a"),
            "status",
            Condition::In(vec![text("ready"), text("blocked")])
        ),
    )
    .is_none());
    assert!(unify(&accessible("a"), &accessible("b")).is_none());
}

#[test]
fn bindings_iterates_in_deterministic_key_order() {
    let bindings = Bindings::empty()
        .bind("?z".to_string(), text("last"))
        .unwrap()
        .bind("?a".to_string(), text("first"))
        .unwrap();

    assert_eq!(
        bindings
            .iter()
            .map(|(key, value)| (key.as_str(), value))
            .collect::<Vec<_>>(),
        vec![("?a", &text("first")), ("?z", &text("last"))]
    );
}

#[test]
fn validation_reports_cost_warning_boundaries() {
    let exact = Composition {
        steps: vec![operator_step(
            "exact",
            CostEstimate {
                time_ms: 300_000,
                money_microdollars: 1_000_000,
                provider_calls: 100,
            },
            vec![],
        )],
        edges: Vec::new(),
    };
    assert!(!validate(&exact)
        .warnings
        .iter()
        .any(|warning| matches!(warning, ValidationWarning::HighCost { .. })));

    for cost in [
        CostEstimate {
            time_ms: 300_001,
            money_microdollars: 0,
            provider_calls: 0,
        },
        CostEstimate {
            time_ms: 0,
            money_microdollars: 1_000_001,
            provider_calls: 0,
        },
        CostEstimate {
            time_ms: 0,
            money_microdollars: 0,
            provider_calls: 101,
        },
    ] {
        let result = validate(&Composition {
            steps: vec![operator_step("expensive", cost, vec![])],
            edges: Vec::new(),
        });
        warning_kind(&result, |warning| {
            matches!(warning, ValidationWarning::HighCost { .. })
        });
    }
}

#[test]
fn validation_reports_structural_errors_and_warnings() {
    let guarded_goal = Composition {
        steps: vec![
            Step {
                step_id: "goal".to_string(),
                kind: StepKind::Goal(accessible("a")),
            },
            zero_step("next"),
        ],
        edges: vec![Edge {
            from: "goal".to_string(),
            to: "next".to_string(),
            kind: EdgeKind::Conditional {
                field_path: "ok".to_string(),
                guard: Condition::Equals(Term::Literal(Literal::Bool(true))),
            },
        }],
    };
    error_kind(&validate(&guarded_goal), |error| {
        matches!(error, ValidationError::InvalidGuardOnGoalStep { .. })
    });

    let disconnected = Composition {
        steps: vec![
            zero_step("first"),
            zero_step("disconnected"),
            zero_step("connected"),
        ],
        edges: vec![Edge {
            from: "first".to_string(),
            to: "connected".to_string(),
            kind: EdgeKind::Ordering,
        }],
    };
    let result = validate(&disconnected);
    assert!(result
        .warnings
        .contains(&ValidationWarning::DisconnectedStep(
            "disconnected".to_string()
        )));
    assert!(!result
        .warnings
        .contains(&ValidationWarning::DisconnectedStep("first".to_string())));
    assert!(!result
        .warnings
        .contains(&ValidationWarning::DisconnectedStep(
            "connected".to_string()
        )));

    let acyclic = Composition {
        steps: vec![zero_step("a"), zero_step("b"), zero_step("c")],
        edges: vec![
            Edge {
                from: "a".to_string(),
                to: "b".to_string(),
                kind: EdgeKind::Ordering,
            },
            Edge {
                from: "b".to_string(),
                to: "c".to_string(),
                kind: EdgeKind::Ordering,
            },
        ],
    };
    assert!(!validate(&acyclic)
        .errors
        .iter()
        .any(|error| matches!(error, ValidationError::CycleDetected { .. })));

    let nested_cycle = Composition {
        steps: vec![zero_step("a"), zero_step("b"), zero_step("c")],
        edges: vec![
            Edge {
                from: "a".to_string(),
                to: "b".to_string(),
                kind: EdgeKind::Ordering,
            },
            Edge {
                from: "b".to_string(),
                to: "c".to_string(),
                kind: EdgeKind::Ordering,
            },
            Edge {
                from: "c".to_string(),
                to: "b".to_string(),
                kind: EdgeKind::Ordering,
            },
        ],
    };
    assert!(validate(&nested_cycle)
        .errors
        .contains(&ValidationError::CycleDetected {
            involved_steps: vec!["b".to_string(), "c".to_string()],
        }));
}

#[test]
fn artifact_type_terms_are_canonical_across_resolution_data_flow_and_effects() {
    let artifact_type = Term::ArtifactType("report".to_string());
    let source = Step {
        step_id: "produce".to_string(),
        kind: StepKind::Op(Operator {
            operator_id: "produce".to_string(),
            preconditions: Vec::new(),
            effects: vec![Effect::Assert(Proposition::Exists {
                scope: object("a"),
                artifact_type: artifact_type.clone(),
            })],
            cost: CostEstimate::zero(),
            resolution: Resolution {
                requires_inputs: Vec::new(),
                requires_outputs: vec![SlotConstraint {
                    artifact_type: artifact_type.clone(),
                    required: true,
                }],
                scope_kind: None,
                tags: Vec::new(),
                specific: None,
            },
        }),
    };
    let composition = Composition {
        steps: vec![source, zero_step("consume")],
        edges: vec![Edge {
            from: "produce".to_string(),
            to: "consume".to_string(),
            kind: EdgeKind::DataFlow {
                artifact_type: artifact_type.clone(),
            },
        }],
    };

    assert!(validate(&composition).valid);

    let StepKind::Op(operator) = &composition.steps[0].kind else {
        panic!("expected operator step");
    };
    assert_eq!(
        operator.resolution.requires_outputs[0].artifact_type,
        artifact_type
    );
    let EdgeKind::DataFlow {
        artifact_type: edge_artifact_type,
    } = &composition.edges[0].kind
    else {
        panic!("expected data flow edge");
    };
    assert_eq!(edge_artifact_type, &artifact_type);
    let Effect::Assert(Proposition::Exists {
        artifact_type: effect_artifact_type,
        ..
    }) = &operator.effects[0]
    else {
        panic!("expected exists effect");
    };
    assert_eq!(effect_artifact_type, &artifact_type);

    let variant_mismatch = Composition {
        edges: vec![Edge {
            kind: EdgeKind::DataFlow {
                artifact_type: Term::Dimension("report".to_string()),
            },
            ..composition.edges[0].clone()
        }],
        ..composition
    };
    let result = validate(&variant_mismatch);
    assert!(result
        .errors
        .contains(&ValidationError::ArtifactSourceMismatch {
            edge_from: "produce".to_string(),
            edge_to: "consume".to_string(),
            artifact_type: Term::Dimension("report".to_string()),
        }));
}

#[test]
fn validation_reports_unbound_variables_and_unused_effects() {
    let unbound = Composition {
        steps: vec![Step {
            step_id: "write".to_string(),
            kind: StepKind::Op(Operator {
                operator_id: "write".to_string(),
                preconditions: vec![holds(
                    object("a"),
                    "confidence",
                    Condition::Above(Term::Variable("?threshold".to_string())),
                )],
                effects: vec![Effect::Update {
                    subject: object("a"),
                    dimension: dimension("confidence"),
                    value: Term::Variable("?value".to_string()),
                }],
                cost: CostEstimate::zero(),
                resolution: Resolution {
                    requires_inputs: Vec::new(),
                    requires_outputs: Vec::new(),
                    scope_kind: None,
                    tags: Vec::new(),
                    specific: None,
                },
            }),
        }],
        edges: Vec::new(),
    };
    let result = validate(&unbound);
    assert!(result.errors.contains(&ValidationError::UnboundVariable {
        step_id: "write".to_string(),
        variable: "?threshold".to_string(),
    }));
    assert!(result.errors.contains(&ValidationError::UnboundVariable {
        step_id: "write".to_string(),
        variable: "?value".to_string(),
    }));

    let unused = Composition {
        steps: vec![
            zero_step("write"),
            Step {
                step_id: "goal".to_string(),
                kind: StepKind::Goal(holds(object("a"), "other", Condition::Present)),
            },
        ],
        edges: Vec::new(),
    };
    warning_kind(
        &validate(&unused),
        |warning| matches!(warning, ValidationWarning::UnusedEffects { step_id } if step_id == "write"),
    );

    let consumed_by_goal = Composition {
        steps: vec![
            zero_step("write"),
            Step {
                step_id: "goal".to_string(),
                kind: StepKind::Goal(holds(object("a"), "confidence", Condition::Present)),
            },
        ],
        edges: Vec::new(),
    };
    assert!(!validate(&consumed_by_goal)
        .warnings
        .iter()
        .any(|warning| matches!(warning, ValidationWarning::UnusedEffects { .. })));

    let unrelated_outgoing_edge = Composition {
        steps: vec![zero_step("write"), zero_step("other"), zero_step("target")],
        edges: vec![Edge {
            from: "other".to_string(),
            to: "target".to_string(),
            kind: EdgeKind::Ordering,
        }],
    };
    warning_kind(
        &validate(&unrelated_outgoing_edge),
        |warning| matches!(warning, ValidationWarning::UnusedEffects { step_id } if step_id == "write"),
    );

    let asserted_goal = Proposition::Exists {
        scope: object("a"),
        artifact_type: Term::ArtifactType("report".to_string()),
    };
    let consumed_by_asserted_goal = Composition {
        steps: vec![
            Step {
                step_id: "write".to_string(),
                kind: StepKind::Op(Operator {
                    operator_id: "write".to_string(),
                    preconditions: Vec::new(),
                    effects: vec![Effect::Assert(asserted_goal.clone())],
                    cost: CostEstimate::zero(),
                    resolution: Resolution {
                        requires_inputs: Vec::new(),
                        requires_outputs: Vec::new(),
                        scope_kind: None,
                        tags: Vec::new(),
                        specific: None,
                    },
                }),
            },
            Step {
                step_id: "goal".to_string(),
                kind: StepKind::Goal(asserted_goal),
            },
        ],
        edges: Vec::new(),
    };
    assert!(!validate(&consumed_by_asserted_goal)
        .warnings
        .iter()
        .any(|warning| matches!(warning, ValidationWarning::UnusedEffects { .. })));
}

#[test]
fn world_state_query_and_satisfies_cover_compound_patterns() {
    let state = WorldState::new(vec![
        accessible("a"),
        holds(object("a"), "confidence", Condition::Equals(number(0.8))),
        holds(object("b"), "confidence", Condition::Equals(number(0.9))),
    ])
    .unwrap();

    let all = Proposition::All(vec![
        Proposition::Accessible {
            scope: Term::Variable("?node".to_string()),
        },
        holds(
            Term::Variable("?node".to_string()),
            "confidence",
            Condition::Above(number(0.7)),
        ),
    ]);
    let any = Proposition::Any(vec![
        accessible("missing"),
        holds(
            Term::Variable("?node".to_string()),
            "confidence",
            Condition::Above(number(0.7)),
        ),
    ]);
    let not = Proposition::Not(Box::new(accessible("b")));

    let all_matches = state.query(&all);
    assert_eq!(all_matches.len(), 1);
    assert_eq!(all_matches[0].get("?node"), Some(&object("a")));
    assert_eq!(state.query(&any).len(), 2);
    assert_eq!(state.query(&not).len(), 1);
    assert!(state.satisfies(&all));
    assert!(state.satisfies(&any));
    assert!(state.satisfies(&not));
    assert!(!state.satisfies(&Proposition::All(vec![
        accessible("a"),
        accessible("missing"),
    ])));
    assert!(!state.satisfies(&Proposition::Any(vec![
        accessible("missing"),
        accessible("also_missing"),
    ])));
    assert!(!state.satisfies(&Proposition::Not(Box::new(accessible("a")))));
    assert!(!state.satisfies(&holds(
        Term::Variable("?node".to_string()),
        "confidence",
        Condition::Above(number(0.95)),
    )));
    assert!(state.satisfies(&Proposition::All(vec![
        Proposition::Accessible {
            scope: Term::Variable("?node".to_string()),
        },
        holds(
            Term::Variable("?other".to_string()),
            "confidence",
            Condition::Above(number(0.7)),
        ),
    ])));
    assert!(state.satisfies(&Proposition::Not(Box::new(holds(
        Term::Variable("?node".to_string()),
        "confidence",
        Condition::Above(number(0.95)),
    )))));
    assert!(!state.satisfies(&Proposition::Not(Box::new(holds(
        Term::Variable("?node".to_string()),
        "confidence",
        Condition::Above(number(0.7)),
    )))));
    assert!(!state.satisfies(&accessible("missing")));
}

#[test]
fn world_state_gap_and_retract_are_precise() {
    let state = WorldState::new(vec![
        holds(object("a"), "freshness", Condition::Equals(number(0.1))),
        holds(object("b"), "confidence", Condition::Equals(number(0.2))),
        holds(object("a"), "confidence", Condition::Equals(number(0.8))),
    ])
    .unwrap();
    let missing = holds(object("a"), "missing", Condition::Present);
    assert_eq!(state.gap(&missing), vec![missing.clone()]);

    let retracted = state
        .apply(&[Effect::Retract(holds(
            object("a"),
            "confidence",
            Condition::Equals(number(0.0)),
        ))])
        .unwrap();

    assert!(matches!(
        evaluate(
            &retracted,
            &holds(object("a"), "confidence", Condition::Present)
        ),
        EvalResult::Indeterminate { .. }
    ));
    assert_eq!(
        evaluate(
            &retracted,
            &holds(object("a"), "freshness", Condition::Present)
        ),
        EvalResult::Satisfied
    );
    assert_eq!(
        evaluate(
            &retracted,
            &holds(object("b"), "confidence", Condition::Present)
        ),
        EvalResult::Satisfied
    );

    let accessible_state = WorldState::new(vec![accessible("a")]).unwrap();
    let unchanged = accessible_state
        .apply(&[Effect::Retract(accessible("missing"))])
        .unwrap();
    assert_eq!(unchanged.propositions(), accessible_state.propositions());
}

#[test]
fn world_state_query_condition_patterns_are_precise() {
    let state = WorldState::new(vec![
        holds(object("a"), "confidence", Condition::Equals(number(0.8))),
        holds(object("b"), "confidence", Condition::Equals(number(0.2))),
        holds(object("a"), "status", Condition::Equals(text("ready"))),
    ])
    .unwrap();

    let present = state.query(&holds(
        Term::Variable("?node".to_string()),
        "confidence",
        Condition::Present,
    ));
    assert_eq!(present.len(), 2);

    let absent = state.query(&holds(
        Term::Variable("?node".to_string()),
        "confidence",
        Condition::Absent,
    ));
    assert!(absent.is_empty());

    let equals = state.query(&holds(
        object("a"),
        "status",
        Condition::Equals(Term::Variable("?status".to_string())),
    ));
    assert_eq!(equals.len(), 1);
    assert_eq!(equals[0].get("?status"), Some(&text("ready")));

    let below = state.query(&holds(
        Term::Variable("?node".to_string()),
        "confidence",
        Condition::Below(number(0.5)),
    ));
    assert_eq!(below.len(), 1);
    assert_eq!(below[0].get("?node"), Some(&object("b")));

    assert!(state
        .query(&holds(object("c"), "confidence", Condition::Present))
        .is_empty());
    assert!(state
        .query(&holds(
            object("a"),
            "confidence",
            Condition::Below(number(0.1))
        ))
        .is_empty());
}

#[test]
fn substitution_instantiates_artifact_type_terms_in_resolution_edges_and_effects() {
    let pattern = Term::Variable("?artifact".to_string());
    let expected = Term::ArtifactType("docs_patch".to_string());
    let composition = Composition {
        steps: vec![
            Step {
                step_id: "write".to_string(),
                kind: StepKind::Op(Operator {
                    operator_id: "write".to_string(),
                    preconditions: Vec::new(),
                    effects: vec![Effect::Assert(Proposition::Exists {
                        scope: object("a"),
                        artifact_type: pattern.clone(),
                    })],
                    cost: CostEstimate::zero(),
                    resolution: Resolution {
                        requires_inputs: Vec::new(),
                        requires_outputs: vec![SlotConstraint {
                            artifact_type: pattern.clone(),
                            required: true,
                        }],
                        scope_kind: None,
                        tags: Vec::new(),
                        specific: None,
                    },
                }),
            },
            zero_step("consume"),
        ],
        edges: vec![Edge {
            from: "write".to_string(),
            to: "consume".to_string(),
            kind: EdgeKind::DataFlow {
                artifact_type: pattern,
            },
        }],
    };

    let substituted = substitute(
        &composition,
        &Bindings::empty()
            .bind("?artifact".to_string(), expected.clone())
            .unwrap(),
    )
    .unwrap();

    let StepKind::Op(operator) = &substituted.steps[0].kind else {
        panic!("expected operator step");
    };
    assert_eq!(
        operator.resolution.requires_outputs[0].artifact_type,
        expected
    );
    let Effect::Assert(Proposition::Exists { artifact_type, .. }) = &operator.effects[0] else {
        panic!("expected exists effect");
    };
    assert_eq!(artifact_type, &expected);
    let EdgeKind::DataFlow { artifact_type } = &substituted.edges[0].kind else {
        panic!("expected data flow edge");
    };
    assert_eq!(artifact_type, &expected);
}

#[test]
fn substitution_reports_each_unbound_site_and_resolves_edges() {
    let composition = Composition {
        steps: vec![Step {
            step_id: "write".to_string(),
            kind: StepKind::Op(Operator {
                operator_id: "write".to_string(),
                preconditions: vec![holds(
                    Term::Variable("?node".to_string()),
                    "confidence",
                    Condition::Above(Term::Variable("?threshold".to_string())),
                )],
                effects: vec![Effect::Update {
                    subject: Term::Variable("?node".to_string()),
                    dimension: dimension("confidence"),
                    value: Term::Variable("?value".to_string()),
                }],
                cost: CostEstimate::zero(),
                resolution: Resolution {
                    requires_inputs: Vec::new(),
                    requires_outputs: Vec::new(),
                    scope_kind: None,
                    tags: Vec::new(),
                    specific: None,
                },
            }),
        }],
        edges: vec![Edge {
            from: "write".to_string(),
            to: "write".to_string(),
            kind: EdgeKind::Conditional {
                field_path: "ok".to_string(),
                guard: Condition::Equals(Term::Variable("?guard".to_string())),
            },
        }],
    };

    let error = substitute(
        &composition,
        &Bindings::empty()
            .bind("?node".to_string(), object("a"))
            .unwrap(),
    )
    .unwrap_err();
    let variables = error
        .unbound_variables
        .iter()
        .map(|entry| entry.variable.as_str())
        .collect::<Vec<_>>();

    assert!(variables.contains(&"?threshold"));
    assert!(variables.contains(&"?value"));
    assert!(variables.contains(&"?guard"));
}

proptest! {
    #[test]
    fn update_last_write_wins(value in 0.0f64..=1.0, replacement in 0.0f64..=1.0) {
        let initial = WorldState::empty()
            .apply(&[Effect::Update {
                subject: object("a"),
                dimension: dimension("confidence"),
                value: number(value),
            }])
            .unwrap();
        let updated = initial
            .apply(&[Effect::Update {
                subject: object("a"),
                dimension: dimension("confidence"),
                value: number(replacement),
            }])
            .unwrap();

        prop_assert_eq!(updated.propositions().len(), 1);
        prop_assert_eq!(
            evaluate(
                &updated,
                &holds(object("a"), "confidence", Condition::Equals(number(replacement)))
            ),
            EvalResult::Satisfied
        );
    }

    #[test]
    fn unify_then_substitute_ground_goal(
        node_id in "[a-z][a-z0-9_]{0,16}",
        threshold in 0.0f64..=1.0,
    ) {
        let pattern = holds(
            Term::Variable("?node".to_string()),
            "confidence",
            Condition::Above(Term::Variable("?threshold".to_string())),
        );
        let concrete = holds(
            object(&node_id),
            "confidence",
            Condition::Above(number(threshold)),
        );
        let bindings = unify(&pattern, &concrete).unwrap();
        let composition = Composition {
            steps: vec![Step {
                step_id: "goal".to_string(),
                kind: StepKind::Goal(pattern),
            }],
            edges: Vec::new(),
        };

        let substituted = substitute(&composition, &bindings).unwrap();
        let StepKind::Goal(goal) = &substituted.steps[0].kind else {
            panic!("expected goal step");
        };
        prop_assert_eq!(goal, &concrete);
        prop_assert!(goal.is_ground());
    }

    #[test]
    fn contract_shapes_round_trip_through_json(
        node_id in "[a-z][a-z0-9_]{0,16}",
        confidence in 0u32..=1_000,
        artifact in "[a-z][a-z0-9_]{0,16}",
    ) {
        let confidence = f64::from(confidence);
        let proposition = Proposition::All(vec![
            accessible(&node_id),
            holds(
                object(&node_id),
                "confidence",
                Condition::Equals(number(confidence)),
            ),
            exists(object(&node_id), &artifact),
        ]);
        let effect = Effect::Assert(proposition.clone());
        let composition = Composition {
            steps: vec![Step {
                step_id: "assert".to_string(),
                kind: StepKind::Goal(proposition.clone()),
            }],
            edges: Vec::new(),
        };

        prop_assert_eq!(
            serde_json::from_str::<Proposition>(&serde_json::to_string(&proposition).unwrap()).unwrap(),
            proposition
        );
        prop_assert_eq!(
            serde_json::from_str::<Effect>(&serde_json::to_string(&effect).unwrap()).unwrap(),
            effect
        );
        prop_assert_eq!(
            serde_json::from_str::<Composition>(&serde_json::to_string(&composition).unwrap()).unwrap(),
            composition
        );
    }
}
