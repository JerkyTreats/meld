use meld_events::DomainObjectRef;
use meld_lang::{
    CapabilityRef, Composition, Condition, CostEstimate, Effect, Goal, GoalLifecycle, GoalPriority,
    GoalSource, Method, Operator, Proposition, Resolution, SlotConstraint, Step, StepKind, Term,
    WorldState,
};

use super::*;
use crate::agent::{
    authorize_curation_outcome, authorize_curation_outcome_with_theory, AgentCurationDecision,
    AgentCurationDedupeKey, AgentCurationInputRefs, AgentCurationOutcome, AgentDecisionKind,
    AgentGoalCommand,
};
use crate::belief::{BeliefKey, BranchScope};
use crate::world_state::graph::PerspectiveKey;

fn subject() -> Term {
    Term::Object(DomainObjectRef::new("docs", "repository", "meld").unwrap())
}

fn goal_pattern() -> Proposition {
    Proposition::Holds {
        subject: Term::Variable("?subject".into()),
        dimension: Term::Dimension("docs_freshness".into()),
        condition: Condition::Above(Term::Literal(meld_lang::Literal::Number(0.9))),
    }
}

fn goal() -> Goal {
    Goal {
        goal_id: "goal-docs".into(),
        agent_id: "agent-docs".into(),
        target: Proposition::Holds {
            subject: subject(),
            dimension: Term::Dimension("docs_freshness".into()),
            condition: Condition::Above(Term::Literal(meld_lang::Literal::Number(0.9))),
        },
        priority: GoalPriority {
            urgency: 1,
            cost_ceiling: None,
        },
        source: GoalSource::Maintenance {
            invariant_description: "documentation remains fresh".into(),
        },
        lifecycle: GoalLifecycle::Proposed,
    }
}

fn settlement() -> Proposition {
    Proposition::Exists {
        scope: Term::Variable("?subject".into()),
        artifact_type: Term::ArtifactType("freshness_evidence".into()),
    }
}

fn route() -> ProspectiveEvidenceRoute {
    ProspectiveEvidenceRoute {
        route_id: "route-docs-evidence".into(),
        dimension_id: "docs_freshness".into(),
        outcome_contract_id: "docs-evaluated".into(),
        evidence_schema_id: "docs-freshness-evidence-v1".into(),
    }
}

fn operator(
    id: &str,
    capability_id: &str,
    input: Option<&str>,
    output: &str,
    effects: Vec<Effect>,
) -> Operator {
    Operator {
        operator_id: id.into(),
        preconditions: vec![Proposition::Accessible { scope: subject() }],
        effects,
        cost: CostEstimate {
            time_ms: 1,
            money_microdollars: 0,
            provider_calls: 0,
        },
        resolution: Resolution {
            requires_inputs: input
                .into_iter()
                .map(|artifact| SlotConstraint {
                    artifact_type: Term::ArtifactType(artifact.into()),
                    required: true,
                })
                .collect(),
            requires_outputs: vec![SlotConstraint {
                artifact_type: Term::ArtifactType(output.into()),
                required: true,
            }],
            scope_kind: Some("repository".into()),
            tags: Vec::new(),
            specific: Some(CapabilityRef {
                capability_type_id: capability_id.into(),
                capability_version: 1,
            }),
        },
    }
}

fn capabilities() -> Vec<StrategyCapability> {
    vec![
        StrategyCapability {
            contract_id: "contract-evaluate-v1".into(),
            operator: operator(
                "evaluate-docs",
                "docs.evaluate",
                Some("draft_docs"),
                "freshness_evidence",
                vec![Effect::Assert(Proposition::Exists {
                    scope: subject(),
                    artifact_type: Term::ArtifactType("freshness_evidence".into()),
                })],
            ),
            outcome_contract_id: "docs-evaluated".into(),
        },
        StrategyCapability {
            contract_id: "contract-write-v1".into(),
            operator: operator(
                "write-docs",
                "docs.write",
                None,
                "draft_docs",
                vec![Effect::Assert(Proposition::Exists {
                    scope: subject(),
                    artifact_type: Term::ArtifactType("draft_docs".into()),
                })],
            ),
            outcome_contract_id: "docs-written".into(),
        },
    ]
}

fn problem() -> StrategyProblem {
    StrategyProblem {
        problem_id: "problem-docs-v1".into(),
        goal: goal(),
        world_state: WorldState::new(vec![Proposition::Accessible { scope: subject() }]).unwrap(),
        planner_snapshot_id: "planner-docs-v1".into(),
        theory: StrategyTheorySnapshot {
            theory_id: "theory-docs-v1".into(),
            settlement_rules: vec![StrategySettlementRule {
                goal_pattern: goal_pattern(),
                settlement_obligation: settlement(),
                evidence_route: route(),
            }],
        },
        capabilities: capabilities(),
        methods: Vec::new(),
        evaluation_policy: StrategyEvaluationPolicy {
            policy_id: "minimal-lexical-v1".into(),
            prefer_fewer_steps: true,
        },
    }
}

fn request(problem: StrategyProblem) -> StrategySearchRequest {
    StrategySearchRequest {
        problem,
        bounds: StrategySearchBounds {
            max_expansions: 8,
            max_depth: 4,
        },
    }
}

#[test]
fn constructs_and_verifies_a_novel_capability_chain() {
    let request = request(problem());
    let result = search(&request);
    let candidate = result.recommendation.expect("candidate");

    assert_eq!(result.completion, StrategySearchCompletion::Exhaustive);
    assert_eq!(candidate.origin, StrategyCandidateOrigin::Direct);
    assert_eq!(candidate.composition.steps.len(), 2);
    assert_eq!(candidate.composition.edges.len(), 1);
    assert_eq!(
        verify_candidate(&request.problem, &candidate),
        CandidateVerification::Valid {
            evaluation: candidate.evaluation.clone()
        }
    );
}

#[test]
fn direct_search_reuses_one_producer_across_a_multi_input_dag() {
    let mut validate = operator(
        "validate-docs",
        "docs.validate",
        Some("draft_docs"),
        "validated_docs",
        Vec::new(),
    );
    validate.resolution.requires_inputs.push(SlotConstraint {
        artifact_type: Term::ArtifactType("source_evidence".into()),
        required: true,
    });
    let capabilities = vec![
        StrategyCapability {
            contract_id: "contract-assess-v1".into(),
            operator: operator(
                "assess-docs",
                "docs.assess",
                Some("validated_docs"),
                "freshness_evidence",
                vec![Effect::Assert(Proposition::Exists {
                    scope: subject(),
                    artifact_type: Term::ArtifactType("freshness_evidence".into()),
                })],
            ),
            outcome_contract_id: "docs-evaluated".into(),
        },
        StrategyCapability {
            contract_id: "contract-validate-v1".into(),
            operator: validate,
            outcome_contract_id: "docs-validated".into(),
        },
        StrategyCapability {
            contract_id: "contract-draft-v1".into(),
            operator: operator(
                "draft-docs",
                "docs.draft",
                Some("source_evidence"),
                "draft_docs",
                Vec::new(),
            ),
            outcome_contract_id: "docs-drafted".into(),
        },
        StrategyCapability {
            contract_id: "contract-inspect-v1".into(),
            operator: operator(
                "inspect-docs",
                "docs.inspect",
                None,
                "source_evidence",
                Vec::new(),
            ),
            outcome_contract_id: "docs-inspected".into(),
        },
    ];
    let mut problem = problem();
    problem.capabilities = capabilities;
    let request = StrategySearchRequest {
        problem,
        bounds: StrategySearchBounds {
            max_expansions: 32,
            max_depth: 8,
        },
    };

    let result = search(&request);
    let candidate = result.recommendation.expect("candidate");

    assert_eq!(candidate.composition.steps.len(), 4);
    assert_eq!(candidate.composition.edges.len(), 4);
    assert_eq!(
        candidate
            .composition
            .steps
            .iter()
            .filter(|step| step.step_id == "inspect-docs")
            .count(),
        1
    );
    assert!(matches!(
        verify_candidate(&request.problem, &candidate),
        CandidateVerification::Valid { .. }
    ));
}

#[test]
fn method_seed_uses_the_same_candidate_and_verification_shape() {
    let mut problem = problem();
    let evaluate = problem.capabilities[0].operator.clone();
    let write = problem.capabilities[1].operator.clone();
    problem.methods.push(Method {
        method_id: "method-docs".into(),
        trigger: goal_pattern(),
        preconditions: Vec::new(),
        composition: Composition {
            steps: vec![
                Step {
                    step_id: write.operator_id.clone(),
                    kind: StepKind::Op(write),
                },
                Step {
                    step_id: evaluate.operator_id.clone(),
                    kind: StepKind::Op(evaluate),
                },
            ],
            edges: vec![meld_lang::Edge {
                from: "write-docs".into(),
                to: "evaluate-docs".into(),
                kind: meld_lang::EdgeKind::DataFlow {
                    artifact_type: Term::ArtifactType("draft_docs".into()),
                },
            }],
        },
        net_effects: vec![Effect::Assert(Proposition::Exists {
            scope: subject(),
            artifact_type: Term::ArtifactType("freshness_evidence".into()),
        })],
        cost: CostEstimate::zero(),
        preference: 0,
    });
    let request = request(problem);
    let candidate = search(&request).recommendation.expect("candidate");

    assert_eq!(
        candidate.origin,
        StrategyCandidateOrigin::Method {
            method_id: "method-docs".into()
        }
    );
    assert!(matches!(
        verify_candidate(&request.problem, &candidate),
        CandidateVerification::Valid { .. }
    ));
}

#[test]
fn reports_unclosed_artifacts_without_admitting_a_candidate() {
    let mut problem = problem();
    problem
        .capabilities
        .retain(|capability| capability.contract_id != "contract-write-v1");
    let result = search(&request(problem));

    assert!(result.recommendation.is_none());
    assert!(result
        .rejections
        .iter()
        .any(|ground| matches!(ground, StrategyRejectionGround::UnclosedArtifact { .. })));
}

#[test]
fn reports_bounded_completion_honestly() {
    let mut request = request(problem());
    request.bounds.max_expansions = 1;
    let result = search(&request);

    assert_eq!(result.completion, StrategySearchCompletion::Bounded);
    assert!(result.recommendation.is_none());
    assert!(result
        .rejections
        .contains(&StrategyRejectionGround::BoundsExceeded));
}

#[test]
fn verifier_rejects_tampered_candidate_content() {
    let request = request(problem());
    let mut candidate = search(&request).recommendation.expect("candidate");
    candidate.composition.steps.pop();

    assert!(matches!(
        verify_candidate(&request.problem, &candidate),
        CandidateVerification::Invalid { .. }
    ));
}

#[test]
fn agent_settles_the_exact_verified_candidate_into_command_and_decision() {
    let goal = goal();
    let dedupe_key = AgentCurationDedupeKey {
        agent_id: goal.agent_id.clone(),
        subject_key: match &goal.target {
            Proposition::Holds {
                subject: Term::Object(subject),
                ..
            } => subject.index_key(),
            _ => unreachable!(),
        },
        branch_id: "main".into(),
        dimension_id: "docs_freshness".into(),
        target_condition_key: serde_json::to_string(&Condition::Above(Term::Literal(
            meld_lang::Literal::Number(0.9),
        )))
        .unwrap(),
        source_kind: "maintenance".into(),
    };
    let decision = AgentCurationDecision {
        decision_id: "decision-docs".into(),
        agent_id: goal.agent_id.clone(),
        subscription_id: "subscription-docs".into(),
        decision: AgentDecisionKind::GoalCommand,
        goal_command_id: Some("command-docs".into()),
        goal_mutation_command_id: None,
        strategy_authorization: None,
        curation_rule_revision: None,
        dedupe_key: dedupe_key.clone(),
        input_refs: AgentCurationInputRefs {
            belief_revision_id: Some("belief-revision-docs".into()),
            belief_key: BeliefKey {
                subject: match &goal.target {
                    Proposition::Holds {
                        subject: Term::Object(subject),
                        ..
                    } => subject.clone(),
                    _ => unreachable!(),
                },
                dimension_id: "docs_freshness".into(),
                predicate_id: "confidence".into(),
                perspective: PerspectiveKey::new("agent", "docs").unwrap(),
                branch_scope: BranchScope::main(),
                evidence_policy_id: "default".into(),
            },
            planner_projection_version: "planner-docs-v1".into(),
            planner_source_refs: Vec::new(),
            planner_warnings: Vec::new(),
        },
        reason: "belief divergence".into(),
        created_at_seq: 7,
    };
    let outcome = AgentCurationOutcome {
        decision,
        goal_command: Some(AgentGoalCommand {
            command_id: "command-docs".into(),
            goal,
            dedupe_key,
            strategy_authorization: None,
        }),
        goal_mutation_command: None,
    };
    let revision = crate::belief::TheoryRevisionRef {
        registry: "strategy_theory".to_string(),
        id: "docs_freshness".to_string(),
        content_hash: "exact-revision".to_string(),
    };
    let exact = authorize_curation_outcome_with_theory(
        outcome.clone(),
        problem(),
        StrategySearchBounds {
            max_expansions: 8,
            max_depth: 4,
        },
        Some(revision.clone()),
    )
    .unwrap();
    let exact_authorization = exact.decision.strategy_authorization.unwrap();

    let authorized = authorize_curation_outcome(
        outcome,
        problem(),
        StrategySearchBounds {
            max_expansions: 8,
            max_depth: 4,
        },
    )
    .unwrap();
    let decision_authorization = authorized.decision.strategy_authorization.unwrap();
    let command_authorization = authorized
        .goal_command
        .unwrap()
        .strategy_authorization
        .unwrap();

    assert_eq!(decision_authorization, command_authorization);
    assert_eq!(exact_authorization.strategy_theory_revision, Some(revision));
    assert_ne!(
        exact_authorization.authorization_id,
        command_authorization.authorization_id
    );
    assert_eq!(command_authorization.agent_decision_id, "decision-docs");
    assert!(matches!(
        verify_candidate(&problem(), &command_authorization.candidate),
        CandidateVerification::Valid { .. }
    ));
}
