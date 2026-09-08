use meld_events::{DomainObjectRef, LedgerCursor, LedgerIdentity};
use meld_lang::{
    CapabilityRef, Condition, CostEstimate, Effect, Goal, GoalLifecycle, GoalPriority, GoalSource,
    Operator, Proposition, Resolution, SlotConstraint, Term,
};

use super::*;
use crate::belief::BranchScope;
use crate::curation::{CurationAuthority, CurationOperation};
use crate::planner::*;
use crate::world_state::graph::contracts::*;
use crate::world_state::graph::PerspectiveKey;

fn subject_ref() -> DomainObjectRef {
    DomainObjectRef::new("workspace_fs", "node", "meld").unwrap()
}

fn subject() -> Term {
    Term::Object(subject_ref())
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

fn operator(id: &str, capability_id: &str, input: Option<&str>, output: &str) -> Operator {
    Operator {
        operator_id: id.into(),
        preconditions: vec![Proposition::Accessible { scope: subject() }],
        effects: vec![Effect::Assert(Proposition::Exists {
            scope: subject(),
            artifact_type: Term::ArtifactType(output.into()),
        })],
        cost: CostEstimate {
            time_ms: 1,
            money_microdollars: 0,
            provider_calls: 0,
        },
        resolution: Resolution {
            requires_inputs: input
                .into_iter()
                .map(|value| SlotConstraint {
                    artifact_type: Term::ArtifactType(value.into()),
                    required: true,
                })
                .collect(),
            requires_outputs: vec![SlotConstraint {
                artifact_type: Term::ArtifactType(output.into()),
                required: true,
            }],
            scope_kind: Some("node".into()),
            tags: Vec::new(),
            specific: Some(CapabilityRef {
                capability_type_id: capability_id.into(),
                capability_version: 1,
            }),
        },
    }
}

fn traversal() -> (TraversalCut, BoundedTraversalRequest, TraversalResult) {
    let ledger_id = LedgerIdentity::new();
    let scope = OwnerPublicationScope {
        scope_id: "meld".into(),
        branch_id: Some("main".into()),
        perspective_id: Some("default".into()),
        valid_at: None,
    };
    let mut cut = TraversalCut {
        cut_id: String::new(),
        owners: vec![TraversalOwnerRequirement {
            event_source: None,
            owner_id: "workspace_fs".into(),
            scope: scope.clone(),
            required: true,
        }],
        receipts: vec![OwnerGraphRevisionReceipt {
            event_coverage: None,
            owner_id: "workspace_fs".into(),
            revision_id: "workspace-v1".into(),
            scope: scope.clone(),
            completeness: OwnerCompletenessReceipt {
                receipt_id: "complete-v1".into(),
                scope: scope.clone(),
                included_ids: Vec::new(),
                exclusions: Vec::new(),
                failures: Vec::new(),
                status: OwnerCompletenessStatus::Complete,
            },
            source_event: Some(meld_events::EventRecordRef { ledger_id, seq: 7 }),
            projection_position: LedgerCursor {
                ledger_id,
                after_seq: 7,
            },
        }],
        scope,
        currentness: OwnerCurrentnessPolicy::LatestComplete,
        event_position: LedgerCursor {
            ledger_id,
            after_seq: 7,
        },
        graph_position: LedgerCursor {
            ledger_id,
            after_seq: 7,
        },
        status: TraversalCutStatus::Complete,
        issues: Vec::new(),
    };
    cut.cut_id = traversal_cut_identity(&cut).unwrap();
    let request = BoundedTraversalRequest {
        roots: vec![subject_ref()],
        direction: TraversalDirection::Incoming,
        relation_types: None,
        bounds: TraversalBounds {
            max_depth: 1,
            max_objects: 4,
            max_occurrences: 4,
            max_paths: 4,
        },
    };
    let result = TraversalResult {
        absent_roots: Vec::new(),
        result_id: traversal_result_identity(&cut.cut_id, &request).unwrap(),
        cut_id: cut.cut_id.clone(),
        objects: Vec::new(),
        occurrences: Vec::new(),
        paths: Vec::new(),
        receipts: cut.receipts.clone(),
        frontier: Vec::new(),
        truncation: TraversalTruncation::default(),
    };
    (cut, request, result)
}

fn planner_cut() -> PlannerCut {
    planner_cut_at("v1")
}

fn planner_cut_at(revision: &str) -> PlannerCut {
    let (traversal_cut, traversal_request, traversal_result) = traversal();
    let context = PlannerDecisionContext {
        observation_subject: None,
        context_id: "context-docs-v1".into(),
        agent_id: "agent-docs".into(),
        goal_id: "goal-docs".into(),
        subject: subject_ref(),
        scope_id: "meld".into(),
        branch_id: "main".into(),
        perspective_id: "default".into(),
        authority_scope_id: "authority-docs".into(),
        activation_generation: "activation-docs".into(),
        admission_epoch: None,
    };
    let kinds = [
        PlannerSourceKind::Graph,
        PlannerSourceKind::Belief,
        PlannerSourceKind::Directive,
        PlannerSourceKind::MaintainedCondition,
        PlannerSourceKind::CapabilityCatalog,
        PlannerSourceKind::CurationCatalog,
        PlannerSourceKind::StrategyPolicy,
    ];
    match PlannerCut::assemble(PlannerAssemblyRequest {
        context: context.clone(),
        policy: PlannerAssemblyPolicy {
            policy_revision_id: "planner-policy-v1".into(),
            required_sources: kinds.to_vec(),
            explicitly_not_required: vec![PlannerSourceKind::Causation, PlannerSourceKind::Regime],
        },
        traversal_cut,
        traversal_request,
        traversal_result,
        source_positions: kinds
            .into_iter()
            .map(|kind| PlannerSourcePosition {
                kind,
                owner_id: format!("{kind:?}"),
                source_id: format!("{kind:?}-source"),
                revision_id: format!("{kind:?}-revision-{revision}"),
                content_hash: format!("{kind:?}-hash-{revision}"),
                scope_id: context.scope_id.clone(),
                branch_id: context.branch_id.clone(),
                perspective_id: context.perspective_id.clone(),
                authority_scope_id: context.authority_scope_id.clone(),
                invalidated_by_revision_id: None,
            })
            .collect(),
        view_input: PlannerProjectionInput {
            additional_beliefs: Vec::new(),
            context: PlannerProjectionContext {
                subject: subject_ref(),
                perspective: PerspectiveKey::new("frame", "default").unwrap(),
                branch_scope: BranchScope::main(),
                projection_version: PLANNER_PROJECTION_VERSION.into(),
            },
            belief_view: None,
            graph_scope: Some(PlannerGraphScope {
                accessible: true,
                anchor_ids: Vec::new(),
                source_fact_ids: Vec::new(),
            }),
            field_config: PlannerFieldProjectionConfig::default(),
        },
    }) {
        PlannerAssemblyOutcome::Complete(cut) => *cut,
        PlannerAssemblyOutcome::Refused(refusal) => panic!("unexpected refusal: {refusal:?}"),
    }
}

fn problem() -> StrategyProblem {
    let cut = planner_cut();
    let operation = CurationOperation::reconstruct(
        CurationAuthority {
            agent_id: "agent-docs".into(),
            perspective: PerspectiveKey::new("frame", "default").unwrap(),
            branch_scope: BranchScope::main(),
            activation_generation: "activation-docs".into(),
            admission_epoch: None,
            subject: subject_ref(),
        },
        crate::belief::TheoryRevisionRef {
            registry: crate::curation::CURATION_RULE_REGISTRY_ID.into(),
            id: "rule-docs".into(),
            content_hash: "rule-docs-hash".into(),
        },
        cut.traversal_cut.clone(),
        cut.traversal_request.clone(),
    )
    .unwrap();
    StrategyProblem {
        effect_visibility: None,
        task_inputs: Vec::new(),
        problem_id: "problem-docs-v1".into(),
        goal: goal(),
        planner_cut: cut,
        theory: StrategyTheorySnapshot {
            theory_id: "theory-docs-v1".into(),
            settlement_rules: vec![StrategySettlementRule {
                task_ordering: Vec::new(),
                epistemic_placement: StrategyEpistemicPlacement::Prerequisite,
                goal_pattern: goal_pattern(),
                settlement_obligation: Proposition::Exists {
                    scope: Term::Variable("?subject".into()),
                    artifact_type: Term::ArtifactType("freshness_evidence".into()),
                },
                evidence_route: ProspectiveEvidenceRoute {
                    route_id: "route-docs-evidence".into(),
                    dimension_id: "docs_freshness".into(),
                    outcome_contract_id: "docs-evaluated".into(),
                    evidence_schema_id: "docs-freshness-evidence-v1".into(),
                },
            }],
        },
        capabilities: vec![
            StrategyCapability {
                contract_id: "contract-evaluate-v1".into(),
                operator: operator(
                    "evaluate-docs",
                    "docs.evaluate",
                    Some("draft_docs"),
                    "freshness_evidence",
                ),
                outcome_contract_id: "docs-evaluated".into(),
            },
            StrategyCapability {
                contract_id: "contract-write-v1".into(),
                operator: operator("write-docs", "docs.write", None, "draft_docs"),
                outcome_contract_id: "docs-written".into(),
            },
        ],
        methods: Vec::new(),
        evaluation_policy: StrategyEvaluationPolicy {
            policy_id: "minimal-lexical-v1".into(),
            prefer_fewer_steps: true,
        },
        curation_operations: vec![operation],
    }
}

#[test]
fn constructs_a_mixed_plan_with_typed_curation_dependency() {
    let problem = problem();
    let result = search(&StrategySearchRequest {
        problem: problem.clone(),
        bounds: StrategySearchBounds {
            max_expansions: 8,
            max_depth: 4,
        },
    });
    let plan = result.recommendation.expect("complete Plan");
    assert_eq!(plan.tasks.len(), 1);
    assert_eq!(plan.epistemic_operations.len(), 1);
    assert_eq!(plan.dependencies.len(), 1);
    assert!(matches!(
        plan.dependencies[0].required_milestone,
        PlanMilestoneRequirement::CurationVisible { .. }
    ));
    assert_eq!(
        verify_plan(&problem, &plan),
        PlanVerification::Valid {
            evaluation: plan.evaluation.clone()
        }
    );
    let mut narrowed = plan.clone();
    narrowed.dependencies[0].required_milestone = PlanMilestoneRequirement::CurationTerminal {
        operation_id: narrowed.epistemic_operations[0]
            .operation
            .operation_id
            .clone(),
    };
    narrowed.plan_revision_id = super::search::plan_revision_identity(&narrowed);
    assert!(
        matches!(
            verify_plan(&problem, &narrowed),
            PlanVerification::Invalid { .. }
        ),
        "terminal persistence cannot replace the prerequisite return"
    );
}

#[test]
fn plan_identity_is_stable_and_non_circular() {
    let request = StrategySearchRequest {
        problem: problem(),
        bounds: StrategySearchBounds {
            max_expansions: 8,
            max_depth: 4,
        },
    };
    let first = search(&request).recommendation.unwrap();
    let second = search(&request).recommendation.unwrap();
    assert_eq!(first.plan_revision_id, second.plan_revision_id);
    assert_eq!(first.predecessor_plan_revision_id, None);
    assert!(!first.plan_family_id.contains(&first.plan_revision_id));
}

#[test]
fn satisfied_goal_constructs_no_work_without_a_capability_or_settlement_recipe() {
    let mut problem = problem();
    problem.planner_cut.world_model_view.world_state =
        meld_lang::WorldState::new(vec![Proposition::Holds {
            subject: subject(),
            dimension: Term::Dimension("docs_freshness".into()),
            condition: Condition::Equals(Term::Literal(meld_lang::Literal::Number(1.0))),
        }])
        .unwrap();
    problem.capabilities.clear();
    problem.theory.settlement_rules.clear();
    let result = search(&StrategySearchRequest {
        problem: problem.clone(),
        bounds: StrategySearchBounds {
            max_expansions: 1,
            max_depth: 1,
        },
    });
    let plan = result.recommendation.unwrap();
    assert_eq!(plan.origin, StrategyPlanOrigin::Satisfied);
    assert!(plan.tasks.is_empty());
    assert!(plan.epistemic_operations.is_empty());
    assert!(plan.evidence_route.is_none());
    assert!(matches!(
        verify_plan(&problem, &plan),
        PlanVerification::Valid { .. }
    ));
    problem.planner_cut.world_model_view.world_state = meld_lang::WorldState::empty();
    assert!(matches!(
        verify_plan(&problem, &plan),
        PlanVerification::Invalid { .. }
    ));
}

#[test]
fn successor_preserves_history_and_non_circular_product_identity() {
    let initial_request = StrategySearchRequest {
        problem: problem(),
        bounds: StrategySearchBounds {
            max_expansions: 8,
            max_depth: 4,
        },
    };
    let predecessor = search(&initial_request).recommendation.unwrap();
    let completed_history = vec![StrategyCompletedHistoryEntry {
        product: None,
        source_plan_revision_id: predecessor.plan_revision_id.clone(),
        product_id: predecessor.epistemic_operations[0].product_id.clone(),
        accepted_milestone: predecessor.dependencies[0].required_milestone.clone(),
        owner_position_id: "curation-result-docs-v1".into(),
    }];
    let mut successor_problem = problem();
    successor_problem.problem_id = "problem-docs-v2".into();
    successor_problem.planner_cut = planner_cut_at("v2");
    let request = StrategySuccessorRequest {
        search: StrategySearchRequest {
            problem: successor_problem,
            bounds: StrategySearchBounds {
                max_expansions: 8,
                max_depth: 4,
            },
        },
        predecessor_plan: Box::new(predecessor.clone()),
        completed_history: completed_history.clone(),
    };

    let first = search_successor(&request).recommendation.unwrap();
    let second = search_successor(&request).recommendation.unwrap();
    assert_eq!(first, second);
    assert_eq!(first.completed_history, completed_history);
    assert_eq!(first.plan.plan_family_id, predecessor.plan_family_id);
    assert_eq!(
        first.plan.predecessor_plan_revision_id.as_deref(),
        Some(predecessor.plan_revision_id.as_str())
    );
    assert_ne!(first.plan.plan_revision_id, predecessor.plan_revision_id);
    assert!(!first.plan.tasks[0]
        .task_id
        .contains(&first.plan.plan_revision_id));
    assert!(!first.plan.epistemic_operations[0]
        .product_id
        .contains(&first.plan.plan_revision_id));
    assert_eq!(
        verify_successor_plan(&request, &first),
        PlanVerification::Valid {
            evaluation: first.plan.evaluation.clone()
        }
    );
    let mut rewritten = first.clone();
    rewritten.completed_history[0].owner_position_id = "rewritten-position".into();
    assert_eq!(
        verify_successor_plan(&request, &rewritten),
        PlanVerification::Invalid {
            grounds: vec![StrategyRejectionGround::InvalidPredecessor]
        }
    );
}

#[test]
fn successor_replay_is_stable_and_keeps_completed_history() {
    let initial_request = StrategySearchRequest {
        problem: problem(),
        bounds: StrategySearchBounds {
            max_expansions: 8,
            max_depth: 4,
        },
    };
    let predecessor = search(&initial_request).recommendation.unwrap();
    let request = StrategySuccessorRequest {
        search: initial_request,
        completed_history: vec![StrategyCompletedHistoryEntry {
            product: None,
            source_plan_revision_id: predecessor.plan_revision_id.clone(),
            product_id: predecessor.tasks[0].task_id.clone(),
            accepted_milestone: PlanMilestoneRequirement::ExecutionTerminal {
                task_id: predecessor.tasks[0].task_id.clone(),
            },
            owner_position_id: "agent-milestone-docs-v1".into(),
        }],
        predecessor_plan: Box::new(predecessor),
    };
    let encoded = serde_json::to_vec(&request).unwrap();
    let replayed: StrategySuccessorRequest = serde_json::from_slice(&encoded).unwrap();

    assert_eq!(search_successor(&request), search_successor(&replayed));
}

#[test]
fn successor_rejects_a_tampered_predecessor() {
    let search_request = StrategySearchRequest {
        problem: problem(),
        bounds: StrategySearchBounds {
            max_expansions: 8,
            max_depth: 4,
        },
    };
    let mut predecessor = search(&search_request).recommendation.unwrap();
    predecessor.explanation.push_str(" rewritten");
    let result = search_successor(&StrategySuccessorRequest {
        search: search_request,
        predecessor_plan: Box::new(predecessor),
        completed_history: Vec::new(),
    });

    assert_eq!(result.recommendation, None);
    assert_eq!(
        result.rejections,
        vec![StrategyRejectionGround::InvalidPredecessor]
    );
}

#[test]
fn pure_verification_rejects_missing_milestone_dependency() {
    let problem = problem();
    let mut plan = search(&StrategySearchRequest {
        problem: problem.clone(),
        bounds: StrategySearchBounds {
            max_expansions: 8,
            max_depth: 4,
        },
    })
    .recommendation
    .unwrap();
    plan.dependencies.clear();
    assert!(matches!(
        verify_plan(&problem, &plan),
        PlanVerification::Invalid { .. }
    ));
}

#[test]
fn confirmation_successor_keeps_completed_task_and_verifies_remaining_epistemic_work() {
    prove_confirmation_successor(StrategyEpistemicPlacement::Confirmation, false);
}

#[test]
fn graph_confirmation_successor_requires_visibility_instead_of_execution_terminality() {
    prove_confirmation_successor(StrategyEpistemicPlacement::GraphConfirmation, false);
}

#[test]
fn compound_confirmation_requires_all_task_returns() {
    prove_confirmation_successor(StrategyEpistemicPlacement::Confirmation, true);
}

#[test]
fn compound_graph_confirmation_requires_all_task_visibility() {
    prove_confirmation_successor(StrategyEpistemicPlacement::GraphConfirmation, true);
}

fn prove_confirmation_successor(placement: StrategyEpistemicPlacement, compound: bool) {
    let mut problem = if compound {
        compound_problem()
    } else {
        problem()
    };
    problem.theory.settlement_rules[0].epistemic_placement = placement;
    if placement == StrategyEpistemicPlacement::GraphConfirmation {
        problem.effect_visibility = Some(
            crate::world_state::graph::contracts::OwnerPublicationExpectation {
                owner_id: "test-owner".into(),
                revision_id: "expected-effect".into(),
                scope: problem.planner_cut.traversal_cut.scope.clone(),
                event_record_id: "exact-owner-publication".into(),
            },
        );
    }
    let mut search_request = StrategySearchRequest {
        problem,
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    let predecessor = search(&search_request).recommendation.unwrap();
    assert!(matches!(
        verify_plan(&search_request.problem, &predecessor),
        PlanVerification::Valid { .. }
    ));
    let task = predecessor.tasks[0].clone();
    assert_eq!(
        predecessor.dependencies[0].producer_product_id,
        task.task_id
    );
    assert_eq!(
        predecessor.dependencies[0].consumer_product_id,
        predecessor.epistemic_operations[0].product_id
    );
    search_request.problem.planner_cut = planner_cut_at("after-task");
    let request = StrategySuccessorRequest {
        search: search_request,
        completed_history: predecessor
            .tasks
            .iter()
            .map(|task| StrategyCompletedHistoryEntry {
                source_plan_revision_id: predecessor.plan_revision_id.clone(),
                product_id: task.task_id.clone(),
                accepted_milestone: task.confirmation_milestone(),
                owner_position_id: "execution-outcome-v1".into(),
                product: Some(StrategyProduct::Task(Box::new(task.clone()))),
            })
            .collect(),
        predecessor_plan: Box::new(predecessor),
    };
    let successor = search_successor(&request).recommendation.unwrap();
    assert_eq!(successor.plan.origin, StrategyPlanOrigin::Confirmation);
    assert!(successor.plan.tasks.is_empty());
    assert_eq!(successor.completed_history, request.completed_history);
    assert!(matches!(
        verify_successor_plan(&request, &successor),
        PlanVerification::Valid { .. }
    ));
    assert!(matches!(
        verify_plan(&request.search.problem, &successor.plan),
        PlanVerification::Invalid { .. }
    ));
    assert_eq!(
        search_successor(&request).recommendation,
        Some(successor.clone())
    );
    if placement == StrategyEpistemicPlacement::GraphConfirmation {
        let mut terminal_only = request.clone();
        terminal_only.completed_history[0].accepted_milestone =
            task.return_milestone.clone().unwrap();
        assert!(matches!(
            verify_successor_plan(&terminal_only, &successor),
            PlanVerification::Invalid { .. }
        ));
        let mut foreign = request.clone();
        foreign
            .search
            .problem
            .effect_visibility
            .as_mut()
            .unwrap()
            .event_record_id = "foreign-publication".into();
        assert!(matches!(
            verify_successor_plan(&foreign, &successor),
            PlanVerification::Invalid { .. }
        ));
    }
    let mut positive = request.clone();
    positive
        .search
        .problem
        .planner_cut
        .world_model_view
        .world_state = meld_lang::WorldState::new(vec![Proposition::Holds {
        subject: subject(),
        dimension: Term::Dimension("docs_freshness".into()),
        condition: Condition::Equals(Term::Literal(meld_lang::Literal::Number(1.0))),
    }])
    .unwrap();
    let confirmation = search_successor(&positive).recommendation.unwrap();
    assert_eq!(confirmation.plan.origin, StrategyPlanOrigin::Confirmation);
    assert!(matches!(
        verify_successor_plan(&positive, &confirmation),
        PlanVerification::Valid { .. }
    ));
    let mut bypass = search(&positive.search).recommendation.unwrap();
    assert_eq!(bypass.origin, StrategyPlanOrigin::Satisfied);
    bypass.predecessor_plan_revision_id = Some(positive.predecessor_plan.plan_revision_id.clone());
    bypass.plan_revision_id = super::search::plan_revision_identity(&bypass);
    assert!(matches!(
        verify_successor_plan(
            &positive,
            &StrategySuccessorPlan {
                plan: bypass,
                completed_history: positive.completed_history.clone(),
            }
        ),
        PlanVerification::Invalid { .. }
    ));
    let mut forged = request.clone();
    let Some(StrategyProduct::Task(body)) = &mut forged.completed_history[0].product else {
        unreachable!()
    };
    body.idempotency_key = "substituted-product".into();
    let forged_candidate = search_successor(&forged).recommendation.unwrap();
    assert!(matches!(
        verify_successor_plan(&forged, &forged_candidate),
        PlanVerification::Invalid { .. }
    ));
    if compound {
        let mut partial = request.clone();
        partial.completed_history.pop();
        assert!(search_successor(&partial)
            .recommendation
            .is_none_or(|candidate| candidate.plan.origin != StrategyPlanOrigin::Confirmation));
        assert!(matches!(
            verify_successor_plan(&partial, &successor),
            PlanVerification::Invalid { .. }
        ));
    }
    let mut missing_dependency = successor;
    missing_dependency.plan.dependencies.pop();
    missing_dependency.plan.plan_revision_id =
        super::search::plan_revision_identity(&missing_dependency.plan);
    assert!(matches!(
        verify_successor_plan(&request, &missing_dependency),
        PlanVerification::Invalid { .. }
    ));
}

#[test]
fn unsuccessful_confirmation_stops_unchanged_work_but_allows_source_advance_and_alternatives() {
    let mut problem = problem();
    problem.theory.settlement_rules[0].epistemic_placement =
        StrategyEpistemicPlacement::Confirmation;
    problem.curation_operations[0] = problem.curation_operations[0]
        .clone()
        .for_request("confirm-current-source".into())
        .unwrap();
    let search_request = StrategySearchRequest {
        problem,
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    let task_plan = search(&search_request).recommendation.unwrap();
    let mut request = StrategySuccessorRequest {
        search: search_request,
        completed_history: task_plan
            .tasks
            .iter()
            .map(|task| StrategyCompletedHistoryEntry {
                source_plan_revision_id: task_plan.plan_revision_id.clone(),
                product_id: task.task_id.clone(),
                accepted_milestone: task.confirmation_milestone(),
                owner_position_id: "execution-complete".into(),
                product: Some(StrategyProduct::Task(Box::new(task.clone()))),
            })
            .collect(),
        predecessor_plan: Box::new(task_plan),
    };
    let confirmation = search_successor(&request).recommendation.unwrap().plan;
    assert_eq!(confirmation.origin, StrategyPlanOrigin::Confirmation);
    request
        .completed_history
        .extend(confirmation.epistemic_operations.iter().map(|operation| {
            StrategyCompletedHistoryEntry {
                source_plan_revision_id: confirmation.plan_revision_id.clone(),
                product_id: operation.product_id.clone(),
                accepted_milestone: PlanMilestoneRequirement::BeliefRevision {
                    belief_key: "docs-current".into(),
                    revision_id: "not-satisfied".into(),
                },
                owner_position_id: "belief-negative".into(),
                product: Some(StrategyProduct::Epistemic(Box::new(operation.clone()))),
            }
        }));
    request.predecessor_plan = Box::new(confirmation);
    assert!(super::search::confirmation_is_current(
        &request.search.problem,
        &request.completed_history
    ));
    let blocked = search_successor(&request);
    assert!(blocked.recommendation.is_none(), "{blocked:?}");
    // Rehashing a standalone search candidate cannot bypass successor verification.
    let mut repeated = search(&request.search).recommendation.unwrap();
    repeated.predecessor_plan_revision_id = Some(request.predecessor_plan.plan_revision_id.clone());
    repeated.plan_revision_id = super::search::plan_revision_identity(&repeated);
    assert!(
        matches!(verify_successor_plan(&request, &StrategySuccessorPlan {
        plan: repeated, completed_history: request.completed_history.clone(),
    }), PlanVerification::Invalid { grounds } if grounds.contains(&StrategyRejectionGround::UnchangedCompletedWork))
    );

    let mut alternative = request.clone();
    let mut capability = alternative.search.problem.capabilities[0].clone();
    capability.contract_id = "alternative-assessment-contract".into();
    capability.operator.operator_id = "alternative-assessment".into();
    capability
        .operator
        .resolution
        .specific
        .as_mut()
        .unwrap()
        .capability_type_id = "docs.alternative".into();
    alternative.search.problem.capabilities.push(capability);
    let alternative_plan = search_successor(&alternative).recommendation.unwrap();
    assert!(!alternative_plan.plan.tasks.is_empty());
    assert!(matches!(
        verify_successor_plan(&alternative, &alternative_plan),
        PlanVerification::Valid { .. }
    ));

    let mut reactivated = request.clone();
    let operation = &reactivated.search.problem.curation_operations[0];
    let mut authority = operation.authority.clone();
    authority.activation_generation = "successor-activation".into();
    reactivated.search.problem.curation_operations[0] = CurationOperation::reconstruct(
        authority,
        operation.rule_revision.clone(),
        operation.source_cut.clone(),
        operation.traversal_request.clone(),
    )
    .unwrap()
    .for_request(operation.request_id.clone().unwrap())
    .unwrap();
    let confirmation = search_successor(&reactivated).recommendation.unwrap();
    assert_eq!(confirmation.plan.origin, StrategyPlanOrigin::Confirmation);
    assert!(confirmation.plan.tasks.is_empty());
    assert!(matches!(
        verify_successor_plan(&reactivated, &confirmation),
        PlanVerification::Valid { .. }
    ));

    let mut advanced = request;
    let operation = &advanced.search.problem.curation_operations[0];
    let mut cut = operation.source_cut.clone();
    cut.receipts[0].revision_id = "new-owner-source".into();
    cut.cut_id = traversal_cut_identity(&cut).unwrap();
    advanced.search.problem.curation_operations[0] = CurationOperation::reconstruct(
        operation.authority.clone(),
        operation.rule_revision.clone(),
        cut,
        operation.traversal_request.clone(),
    )
    .unwrap()
    .for_request(operation.request_id.clone().unwrap())
    .unwrap();
    let fresh = search_successor(&advanced).recommendation.unwrap();
    assert!(
        !fresh.plan.tasks.is_empty(),
        "changed source permits fresh work"
    );
    assert!(matches!(
        verify_successor_plan(&advanced, &fresh),
        PlanVerification::Valid { .. }
    ));

    // Current owner evidence can satisfy the condition while this Goal still
    // owes its independently authorized confirmation of completed work.
    advanced
        .search
        .problem
        .planner_cut
        .world_model_view
        .world_state = meld_lang::WorldState::new(vec![Proposition::Holds {
        subject: subject(),
        dimension: Term::Dimension("docs_freshness".into()),
        condition: Condition::Equals(Term::Literal(meld_lang::Literal::Number(1.0))),
    }])
    .unwrap();
    let remaining_confirmation = search_successor(&advanced).recommendation.unwrap();
    assert_eq!(
        remaining_confirmation.plan.origin,
        StrategyPlanOrigin::Confirmation
    );
    assert!(remaining_confirmation.plan.tasks.is_empty());
    assert!(matches!(
        verify_successor_plan(&advanced, &remaining_confirmation),
        PlanVerification::Valid { .. }
    ));
}

#[test]
fn named_confirmation_tracks_owner_evidence_beyond_its_own_curation_return() {
    let mut problem = problem();
    problem.curation_operations[0] = problem.curation_operations[0]
        .clone()
        .for_request("confirmation".into())
        .unwrap();
    let original = super::search::epistemic_products(&problem).remove(0);
    let mut advanced = original.clone();
    let mut own = advanced.operation.source_cut.receipts[0].clone();
    own.owner_id = crate::curation::CURATION_OWNER_ID.into();
    advanced.operation.source_cut.receipts.push(own);
    assert!(original.same_request_as(&advanced));
    advanced.operation.source_cut.receipts[1].scope.scope_id = "foreign-curation-source".into();
    assert!(!original.same_request_as(&advanced));
    advanced.operation.source_cut.receipts.pop();
    advanced.operation.source_cut.receipts[0].revision_id = "successor-source".into();
    assert!(!original.same_request_as(&advanced));
    advanced = original.clone();
    advanced.operation.source_cut.receipts.clear();
    assert!(!original.same_request_as(&advanced));
}

#[test]
fn verifier_rejects_cycles_and_foreign_endpoints_even_with_valid_content_identity() {
    let problem = problem();
    let original = search(&StrategySearchRequest {
        problem: problem.clone(),
        bounds: StrategySearchBounds {
            max_expansions: 8,
            max_depth: 4,
        },
    })
    .recommendation
    .unwrap();
    for foreign in [false, true] {
        let mut plan = original.clone();
        plan.dependencies.push(StrategyPlanDependency {
            dependency_id: "adversarial-dependency".into(),
            producer_product_id: if foreign {
                "foreign-task".into()
            } else {
                plan.tasks[0].task_id.clone()
            },
            consumer_product_id: plan.epistemic_operations[0].product_id.clone(),
            required_milestone: PlanMilestoneRequirement::ExecutionTerminal {
                task_id: plan.tasks[0].task_id.clone(),
            },
        });
        plan.plan_revision_id = super::search::plan_revision_identity(&plan);
        assert!(matches!(
            verify_plan(&problem, &plan),
            PlanVerification::Invalid { .. }
        ));
    }
}

#[test]
fn frozen_task_input_is_carried_and_verified_against_the_planning_request() {
    let mut problem = problem();
    problem
        .capabilities
        .retain(|capability| capability.operator.operator_id == "evaluate-docs");
    problem.task_inputs.push(meld_lang::TaskInput {
        step_id: "evaluate-docs".into(),
        slot_id: "draft".into(),
        artifact_type_id: "draft_docs".into(),
        schema_version: 1,
        content: serde_json::json!({"text": "prepared draft"}),
    });
    let mut request = StrategySearchRequest {
        problem,
        bounds: StrategySearchBounds {
            max_expansions: 8,
            max_depth: 4,
        },
    };
    let plan = search(&request)
        .recommendation
        .expect("frozen input closes Task");
    assert_eq!(plan.tasks[0].composition.steps.len(), 1);
    assert!(plan.tasks[0].composition.edges.is_empty());
    assert_eq!(plan.tasks[0].initial_inputs, request.problem.task_inputs);
    assert_eq!(
        plan.tasks[0].execution_subject.as_ref(),
        Some(&request.problem.planner_cut.context.subject)
    );
    assert!(matches!(
        verify_plan(&request.problem, &plan),
        PlanVerification::Valid { .. }
    ));

    let mut tampered = plan.clone();
    tampered.tasks[0].execution_subject =
        Some(meld_events::DomainObjectRef::new("foreign", "subject", "other").unwrap());
    tampered.plan_revision_id = super::search::plan_revision_identity(&tampered);
    assert!(matches!(
        verify_plan(&request.problem, &tampered),
        PlanVerification::Invalid { .. }
    ));
    let mut tampered = plan.clone();
    tampered.tasks[0].initial_inputs[0].content = serde_json::json!({"text": "substituted"});
    tampered.plan_revision_id = super::search::plan_revision_identity(&tampered);
    assert!(matches!(
        verify_plan(&request.problem, &tampered),
        PlanVerification::Invalid { .. }
    ));

    request.problem.task_inputs[0].content = serde_json::json!({"text": "another prepared draft"});
    let changed = search(&request).recommendation.unwrap();
    assert_ne!(changed.tasks[0].task_id, plan.tasks[0].task_id);
    assert_ne!(changed.plan_revision_id, plan.plan_revision_id);
    assert_eq!(search(&request).recommendation, Some(changed));
}

#[test]
fn artifact_existence_without_a_value_does_not_close_an_executable_task() {
    let mut problem = problem();
    problem
        .capabilities
        .retain(|capability| capability.operator.operator_id == "evaluate-docs");
    problem.planner_cut.world_model_view.world_state = meld_lang::WorldState::new(vec![
        Proposition::Accessible { scope: subject() },
        Proposition::Exists {
            scope: subject(),
            artifact_type: Term::ArtifactType("draft_docs".into()),
        },
    ])
    .unwrap();
    let result = search(&StrategySearchRequest {
        problem,
        bounds: StrategySearchBounds {
            max_expansions: 8,
            max_depth: 4,
        },
    });
    assert!(result.recommendation.is_none());
}

fn compound_problem() -> StrategyProblem {
    let mut problem = problem();
    let extra = StrategyCapability {
        contract_id: "contract-index-v1".into(),
        operator: operator("index-docs", "docs.index", None, "index_evidence"),
        outcome_contract_id: "docs-indexed".into(),
    };
    let rule = &mut problem.theory.settlement_rules[0];
    rule.settlement_obligation = Proposition::All(vec![
        rule.settlement_obligation.clone(),
        Proposition::Exists {
            scope: Term::Variable("?subject".into()),
            artifact_type: Term::ArtifactType("index_evidence".into()),
        },
    ]);
    problem.capabilities.push(extra);
    problem
}

#[test]
fn compound_settlement_constructs_independently_complete_tasks() {
    let problem = compound_problem();
    let result = search(&StrategySearchRequest {
        problem: problem.clone(),
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    });
    let plan = result
        .recommendation
        .expect("compound settlement must produce a complete Plan");
    assert_eq!(plan.tasks.len(), 2);
    let endpoints: std::collections::BTreeSet<_> = plan
        .tasks
        .iter()
        .map(|task| task.expected_outcome_contract_id.as_str())
        .collect();
    assert_eq!(
        endpoints,
        std::collections::BTreeSet::from(["docs-evaluated", "docs-indexed"])
    );
    assert_eq!(plan.dependencies.len(), 2);
    assert!(matches!(
        verify_plan(&problem, &plan),
        PlanVerification::Valid { .. }
    ));
    for task in &plan.tasks {
        let ids: std::collections::BTreeSet<_> = task
            .composition
            .steps
            .iter()
            .map(|step| step.step_id.as_str())
            .collect();
        assert!(task
            .composition
            .edges
            .iter()
            .all(|edge| ids.contains(edge.from.as_str()) && ids.contains(edge.to.as_str())));
    }
}

#[test]
fn compound_plan_rejects_a_missing_prerequisite_or_confirmation_edge() {
    for placement in [
        StrategyEpistemicPlacement::Prerequisite,
        StrategyEpistemicPlacement::Confirmation,
    ] {
        let mut problem = compound_problem();
        problem.theory.settlement_rules[0].epistemic_placement = placement;
        let mut plan = search(&StrategySearchRequest {
            problem: problem.clone(),
            bounds: StrategySearchBounds {
                max_expansions: 64,
                max_depth: 8,
            },
        })
        .recommendation
        .unwrap();
        plan.dependencies.pop();
        plan.plan_revision_id = super::search::plan_revision_identity(&plan);
        assert!(matches!(
            verify_plan(&problem, &plan),
            PlanVerification::Invalid { .. }
        ));
    }
}

#[test]
fn method_components_preserve_explicit_operational_dependencies() {
    let mut request = StrategySearchRequest {
        problem: compound_problem(),
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    let direct = search(&request).recommendation.unwrap();
    let method = meld_lang::Method {
        method_id: "compound-method".into(),
        trigger: request.problem.goal.target.clone(),
        preconditions: Vec::new(),
        composition: direct.composition.clone(),
        net_effects: Vec::new(),
        cost: meld_lang::CostEstimate::zero(),
        preference: 0,
    };
    request.problem.methods.push(method);
    request.bounds.max_expansions = 1;
    let independent = search(&request).recommendation.unwrap();
    assert!(matches!(
        independent.origin,
        StrategyPlanOrigin::Method { .. }
    ));
    assert_eq!(independent.tasks, direct.tasks);
    assert!(matches!(
        verify_plan(&request.problem, &independent),
        PlanVerification::Valid { .. }
    ));

    let first = &independent.tasks[0].composition;
    let sink = first
        .steps
        .iter()
        .find(|step| !first.edges.iter().any(|edge| edge.from == step.step_id))
        .unwrap();
    request.problem.methods[0]
        .composition
        .edges
        .push(meld_lang::Edge {
            from: sink.step_id.clone(),
            to: independent.tasks[1].composition.steps[0].step_id.clone(),
            kind: meld_lang::EdgeKind::Ordering,
        });
    let connected = search(&request).recommendation.unwrap();
    assert_eq!(connected.tasks.len(), 1);
    assert_eq!(
        connected.composition,
        request.problem.methods[0].composition
    );
    assert_eq!(
        connected.tasks[0].expected_outcome_contract_id,
        "docs-indexed"
    );
    assert!(matches!(
        verify_plan(&request.problem, &connected),
        PlanVerification::Valid { .. }
    ));
}

#[test]
fn method_guards_require_ground_current_evidence_in_search_and_verification() {
    let mut request = StrategySearchRequest {
        problem: compound_problem(),
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    let direct = search(&request).recommendation.unwrap();
    request.problem.methods.push(meld_lang::Method {
        method_id: "guarded-method".into(),
        trigger: goal_pattern(),
        preconditions: vec![Proposition::Accessible {
            scope: Term::Variable("?subject".into()),
        }],
        composition: direct.composition,
        net_effects: Vec::new(),
        cost: CostEstimate::zero(),
        preference: 0,
    });
    request.bounds.max_expansions = 1;
    let accepted = search(&request).recommendation.unwrap();
    assert!(matches!(accepted.origin, StrategyPlanOrigin::Method { .. }));
    assert!(matches!(
        verify_plan(&request.problem, &accepted),
        PlanVerification::Valid { .. }
    ));
    for guard in [
        Proposition::Not(Box::new(Proposition::Accessible { scope: subject() })),
        Proposition::Holds {
            subject: subject(),
            dimension: Term::Dimension("missing-coverage".into()),
            condition: Condition::Above(Term::Literal(meld_lang::Literal::Number(0.9))),
        },
        Proposition::Accessible {
            scope: Term::Variable("?unbound".into()),
        },
    ] {
        request.problem.methods[0].preconditions = vec![guard];
        assert!(search(&request).recommendation.is_none());
        assert!(matches!(
            verify_plan(&request.problem, &accepted),
            PlanVerification::Invalid { .. }
        ));
    }
}

#[test]
fn method_cannot_remove_a_canonical_capability_guard() {
    let mut request = StrategySearchRequest {
        problem: compound_problem(),
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    let direct = search(&request).recommendation.unwrap();
    let mut composition = direct.composition;
    for step in &mut composition.steps {
        let meld_lang::StepKind::Op(operator) = &mut step.kind else {
            unreachable!()
        };
        operator.preconditions.clear();
    }
    request.problem.methods.push(meld_lang::Method {
        method_id: "guard-stripping-template".into(),
        trigger: request.problem.goal.target.clone(),
        preconditions: Vec::new(),
        composition,
        net_effects: Vec::new(),
        cost: CostEstimate::zero(),
        preference: 0,
    });
    request.bounds.max_expansions = 1;
    let mut candidate = search(&request).recommendation.unwrap();
    assert!(matches!(
        verify_plan(&request.problem, &candidate),
        PlanVerification::Invalid { .. }
    ));
    candidate.origin = StrategyPlanOrigin::Direct;
    candidate.plan_revision_id = super::search::plan_revision_identity(&candidate);
    assert!(matches!(
        verify_plan(&request.problem, &candidate),
        PlanVerification::Invalid { .. }
    ));
}

#[test]
fn installed_task_ordering_requires_distinct_tasks_and_cannot_be_dropped() {
    let mut request = StrategySearchRequest {
        problem: compound_problem(),
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    request.problem.theory.settlement_rules[0].task_ordering = vec![StrategyTaskOrdering {
        before_contract_id: "contract-index-v1".into(),
        after_contract_id: "contract-evaluate-v1".into(),
    }];
    let plan = search(&request).recommendation.unwrap();
    assert_eq!(plan.tasks.len(), 2);
    assert_eq!(plan.dependencies.len(), 3);
    assert!(matches!(
        verify_plan(&request.problem, &plan),
        PlanVerification::Valid { .. }
    ));
    let mut unordered = plan.clone();
    unordered.dependencies.retain(|dependency| {
        !matches!(
            dependency.required_milestone,
            PlanMilestoneRequirement::ExecutionTerminal { .. }
        )
    });
    unordered.plan_revision_id = super::search::plan_revision_identity(&unordered);
    assert!(matches!(
        verify_plan(&request.problem, &unordered),
        PlanVerification::Invalid { .. }
    ));
    request.problem.theory.settlement_rules[0]
        .task_ordering
        .push(StrategyTaskOrdering {
            before_contract_id: "contract-evaluate-v1".into(),
            after_contract_id: "contract-index-v1".into(),
        });
    assert!(search(&request).recommendation.is_none());
}

#[test]
fn successor_verification_retains_completed_task_as_its_causal_predecessor() {
    let mut request = StrategySearchRequest {
        problem: compound_problem(),
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    let seed = search(&request).recommendation.unwrap();
    request.problem.theory.settlement_rules[0].settlement_obligation =
        problem().theory.settlement_rules[0]
            .settlement_obligation
            .clone();
    request.problem.theory.settlement_rules[0].task_ordering = vec![StrategyTaskOrdering {
        before_contract_id: "contract-index-v1".into(),
        after_contract_id: "contract-evaluate-v1".into(),
    }];
    request.problem.methods.push(meld_lang::Method {
        method_id: "prepare-then-evaluate".into(),
        trigger: request.problem.goal.target.clone(),
        preconditions: Vec::new(),
        composition: seed.composition,
        net_effects: Vec::new(),
        cost: CostEstimate::zero(),
        preference: 0,
    });
    request.bounds.max_expansions = 1;
    let predecessor = search(&request).recommendation.unwrap();
    let completed = predecessor
        .tasks
        .iter()
        .find(|task| {
            task.capability_contract_ids
                .contains(&"contract-index-v1".into())
        })
        .unwrap()
        .clone();
    request.bounds.max_expansions = 64;
    request.problem.planner_cut = planner_cut_at("after-preparation");
    let successor_request = StrategySuccessorRequest {
        search: request,
        predecessor_plan: Box::new(predecessor.clone()),
        completed_history: vec![StrategyCompletedHistoryEntry {
            source_plan_revision_id: predecessor.plan_revision_id,
            product_id: completed.task_id.clone(),
            accepted_milestone: PlanMilestoneRequirement::ExecutionTerminal {
                task_id: completed.task_id.clone(),
            },
            owner_position_id: "accepted-preparation-outcome".into(),
            product: Some(StrategyProduct::Task(Box::new(completed.clone()))),
        }],
    };
    let mut successor = search_successor(&successor_request).recommendation.unwrap();
    assert_eq!(successor.plan.tasks.len(), 1);
    assert!(successor
        .plan
        .dependencies
        .iter()
        .any(|dependency| dependency.producer_product_id == completed.task_id));
    assert!(matches!(
        verify_successor_plan(&successor_request, &successor),
        PlanVerification::Valid { .. }
    ));
    successor
        .plan
        .dependencies
        .retain(|dependency| dependency.producer_product_id != completed.task_id);
    successor.plan.plan_revision_id = super::search::plan_revision_identity(&successor.plan);
    assert!(matches!(
        verify_successor_plan(&successor_request, &successor),
        PlanVerification::Invalid { .. }
    ));
}
