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
            work_input_basis_id: Some("source-input-v1".into()),
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
            acquisition_question: None,
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
            unassessed_belief: None,
            pending_derived_evidence: None,
            additional_beliefs: Vec::new(),
            context: PlannerProjectionContext {
                subject: subject_ref(),
                perspective: PerspectiveKey::new("frame", "default").unwrap(),
                branch_scope: BranchScope::main(),
                projection_version: PLANNER_PROJECTION_VERSION.into(),
            },
            belief_view: None,
            graph_scope: Some(PlannerGraphScope { accessible: true }),
            field_config: PlannerFieldProjectionConfig::default(),
        },
    }) {
        PlannerAssemblyOutcome::Complete(cut) => *cut,
        PlannerAssemblyOutcome::Refused(refusal) => panic!("unexpected refusal: {refusal:?}"),
    }
}

pub(super) fn problem() -> StrategyProblem {
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
        unavailable_curation_operation_ids: Vec::new(),
        effect_visibility: None,
        task_inputs: Vec::new(),
        problem_id: "problem-docs-v1".into(),
        goal: goal(),
        planner_cut: cut,
        theory: StrategyTheorySnapshot {
            theory_id: "theory-docs-v1".into(),
            settlement_rules: vec![StrategySettlementRule {
                repeat_on_changed_owners: Vec::new(),
                historical_wire: None,
                construction: StrategyConstruction::Executable,
                epistemic_selections: vec![StrategyEpistemicSelection {
                    rule_revision: None,
                    evidence_return: false,
                }],
                product_ordering: vec![StrategyProductOrdering {
                    condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
                    before: StrategyProductSelector::AllEpistemic,
                    after: StrategyProductSelector::AllTasks,
                    milestone: StrategyDependencyMilestone::CurationVisible,
                }],
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
    prove_confirmation_successor(DependencyFixture::OperationalConfirmation, false);
}

#[test]
fn graph_confirmation_successor_requires_visibility_instead_of_execution_terminality() {
    prove_confirmation_successor(DependencyFixture::VisibleConfirmation, false);
}

#[test]
fn compound_confirmation_requires_all_task_returns() {
    prove_confirmation_successor(DependencyFixture::OperationalConfirmation, true);
}

#[test]
fn compound_graph_confirmation_requires_all_task_visibility() {
    prove_confirmation_successor(DependencyFixture::VisibleConfirmation, true);
}

fn prove_confirmation_successor(placement: DependencyFixture, compound: bool) {
    let mut problem = if compound {
        compound_problem()
    } else {
        problem()
    };
    placement.apply(&mut problem.theory.settlement_rules[0]);
    if placement == DependencyFixture::VisibleConfirmation {
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
    assert!(predecessor
        .dependencies
        .iter()
        .any(|dependency| dependency.producer_product_id == task.task_id
            && dependency.consumer_product_id == predecessor.epistemic_operations[0].product_id));
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
    if placement == DependencyFixture::OperationalConfirmation {
        let mut interrupted = request.clone();
        let mut return_history = interrupted.completed_history.clone();
        for entry in &mut return_history {
            entry.accepted_milestone = PlanMilestoneRequirement::ExecutionInterrupted {
                task_id: entry.product_id.clone(),
            };
        }
        // Preserve an earlier generic terminal receipt while applying the exact
        // owner disposition that remaining work never ran.
        interrupted.completed_history.extend(return_history);
        let replacement = search_successor(&interrupted).recommendation.unwrap();
        assert!(!replacement.plan.tasks.is_empty());
        assert_ne!(replacement.plan.origin, StrategyPlanOrigin::Confirmation);
        assert!(matches!(
            verify_successor_plan(&interrupted, &replacement),
            PlanVerification::Valid { .. }
        ));
    }
    if placement == DependencyFixture::VisibleConfirmation {
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
    DependencyFixture::OperationalConfirmation.apply(&mut problem.theory.settlement_rules[0]);
    problem.theory.settlement_rules[0].repeat_on_changed_owners =
        vec![problem.planner_cut.traversal_cut.receipts[0]
            .owner_id
            .clone()];
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
        &request.search.problem.theory.settlement_rules[0],
        &request.completed_history
    ));
    let mut satisfied_alternative = request.clone();
    satisfied_alternative
        .search
        .problem
        .planner_cut
        .world_model_view
        .world_state = meld_lang::WorldState::new(vec![satisfied_alternative
        .search
        .problem
        .goal
        .target
        .clone()])
    .unwrap();
    let first_operation = &satisfied_alternative.search.problem.curation_operations[0];
    let mut other_revision = first_operation.rule_revision.clone();
    other_revision.id = "unselected-confirmation".into();
    let other_operation = CurationOperation::reconstruct(
        first_operation.authority.clone(),
        other_revision.clone(),
        first_operation.source_cut.clone(),
        first_operation.traversal_request.clone(),
    )
    .unwrap();
    let mut other_rule = satisfied_alternative.search.problem.theory.settlement_rules[0].clone();
    other_rule.epistemic_selections = vec![StrategyEpistemicSelection {
        rule_revision: Some(other_revision),
        evidence_return: true,
    }];
    // The selected route must also name its own operation rather than select the whole catalog.
    let selected_revision = first_operation.rule_revision.clone();
    satisfied_alternative.search.problem.theory.settlement_rules[0].epistemic_selections =
        vec![StrategyEpistemicSelection {
            rule_revision: Some(selected_revision),
            evidence_return: true,
        }];
    satisfied_alternative.predecessor_plan.settlement_rule_id =
        super::search::settlement_rule_identity(
            &satisfied_alternative.search.problem.theory.settlement_rules[0],
        );
    satisfied_alternative.predecessor_plan.plan_revision_id =
        super::search::plan_revision_identity(&satisfied_alternative.predecessor_plan);
    satisfied_alternative
        .search
        .problem
        .curation_operations
        .push(other_operation);
    satisfied_alternative
        .search
        .problem
        .theory
        .settlement_rules
        .push(other_rule);
    let satisfied = search_successor(&satisfied_alternative)
        .recommendation
        .unwrap();
    assert_eq!(satisfied.plan.origin, StrategyPlanOrigin::Satisfied);
    assert!(matches!(
        verify_successor_plan(&satisfied_alternative, &satisfied),
        PlanVerification::Valid { .. }
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

    let mut forged_basis = search(&request.search).recommendation.unwrap();
    forged_basis.tasks[0].source_basis_id = Some("foreign-source-basis".into());
    forged_basis.plan_revision_id = super::search::plan_revision_identity(&forged_basis);
    assert!(
        matches!(verify_plan(&request.search.problem, &forged_basis),
        PlanVerification::Invalid { grounds } if grounds.contains(&StrategyRejectionGround::InvalidComposition))
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
    cut.receipts[0].work_input_basis_id = Some("new-owner-input".into());
    cut.cut_id = traversal_cut_identity(&cut).unwrap();
    advanced.search.problem.planner_cut.traversal_cut = cut.clone();
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
    let operation = &mut problem.curation_operations[0];
    operation.source_cut.owners.push(TraversalOwnerRequirement {
        owner_id: crate::curation::CURATION_OWNER_ID.into(),
        scope: operation.source_cut.scope.clone(),
        required: false,
        event_source: None,
    });
    operation.source_cut.owners.sort();
    operation.source_cut.cut_id = traversal_cut_identity(&operation.source_cut).unwrap();
    problem.curation_operations[0] = problem.curation_operations[0]
        .clone()
        .for_request("confirmation".into())
        .unwrap();
    let original =
        super::search::epistemic_products(&problem, &problem.theory.settlement_rules[0]).remove(0);
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
        DependencyFixture::Preparation,
        DependencyFixture::OperationalConfirmation,
    ] {
        let mut problem = compound_problem();
        placement.apply(&mut problem.theory.settlement_rules[0]);
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
        composition: method_body(&direct),
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
        connected.tasks[0].composition,
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
        composition: method_body(&direct),
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
    let mut composition = method_body(&direct);
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
    request.problem.theory.settlement_rules[0]
        .product_ordering
        .push(StrategyProductOrdering {
            condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
            before: StrategyProductSelector::Task {
                contract_id: "contract-index-v1".into(),
            },
            after: StrategyProductSelector::Task {
                contract_id: "contract-evaluate-v1".into(),
            },
            milestone: StrategyDependencyMilestone::ExecutionTerminal,
        });
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
        .product_ordering
        .push(StrategyProductOrdering {
            condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
            before: StrategyProductSelector::Task {
                contract_id: "contract-evaluate-v1".into(),
            },
            after: StrategyProductSelector::Task {
                contract_id: "contract-index-v1".into(),
            },
            milestone: StrategyDependencyMilestone::ExecutionTerminal,
        });
    assert!(search(&request).recommendation.is_none());
}

#[test]
fn successor_verification_retains_completed_task_as_its_causal_predecessor() {
    assert_completed_task_successor(false);
}

#[test]
fn successor_retains_evidence_route_from_completed_task() {
    assert_completed_task_successor(true);
}

fn assert_completed_task_successor(route_from_completed: bool) {
    let mut request = StrategySearchRequest {
        problem: compound_problem(),
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    if route_from_completed {
        request.problem.theory.settlement_rules[0]
            .evidence_route
            .outcome_contract_id = "docs-indexed".into();
    }
    let seed = search(&request).recommendation.unwrap();
    request.problem.theory.settlement_rules[0].settlement_obligation =
        problem().theory.settlement_rules[0]
            .settlement_obligation
            .clone();
    request.problem.theory.settlement_rules[0]
        .product_ordering
        .push(StrategyProductOrdering {
            condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
            before: StrategyProductSelector::Task {
                contract_id: "contract-index-v1".into(),
            },
            after: StrategyProductSelector::Task {
                contract_id: "contract-evaluate-v1".into(),
            },
            milestone: StrategyDependencyMilestone::ExecutionTerminal,
        });
    request.problem.methods.push(meld_lang::Method {
        method_id: "prepare-then-evaluate".into(),
        trigger: request.problem.goal.target.clone(),
        preconditions: Vec::new(),
        composition: method_body(&seed),
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
    if route_from_completed {
        for foreign_subject in [false, true] {
            let mut stale_request = successor_request.clone();
            let Some(StrategyProduct::Task(task)) = &mut stale_request.completed_history[0].product
            else {
                panic!("completed Task history required");
            };
            if foreign_subject {
                task.execution_subject =
                    Some(meld_events::DomainObjectRef::new("other", "subject", "foreign").unwrap());
            } else {
                task.source_basis_id = Some("different-input-basis".into());
            }
            let mut stale_successor = successor.clone();
            stale_successor.completed_history = stale_request.completed_history.clone();
            assert!(matches!(
                verify_successor_plan(&stale_request, &stale_successor),
                PlanVerification::Invalid { grounds } if grounds.contains(&StrategyRejectionGround::InvalidEvidenceRoute)
            ));
        }
    }
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

#[test]
fn completed_work_repetition_uses_only_the_installed_owner_selection() {
    let mut problem = problem();
    assert!(
        super::search::task_source_basis(&problem, &problem.theory.settlement_rules[0]).is_none()
    );
    assert!(!problem.planner_cut.traversal_cut.receipts.is_empty());
    let owner = problem.planner_cut.traversal_cut.receipts[0]
        .owner_id
        .clone();
    assert_ne!(owner, crate::curation::CURATION_OWNER_ID);
    problem.theory.settlement_rules[0].repeat_on_changed_owners = vec![owner];
    let selected =
        super::search::task_source_basis(&problem, &problem.theory.settlement_rules[0]).unwrap();
    let mut unrelated = problem.planner_cut.traversal_cut.receipts[0].clone();
    unrelated.owner_id = "unselected-owner".into();
    unrelated.revision_id = "unrelated-result".into();
    problem.planner_cut.traversal_cut.receipts.push(unrelated);
    assert_eq!(
        super::search::task_source_basis(&problem, &problem.theory.settlement_rules[0]).as_ref(),
        Some(&selected)
    );
    problem.planner_cut.traversal_cut.receipts[0].revision_id = "changed-owner-evidence".into();
    problem.planner_cut.traversal_cut.receipts[0].work_input_basis_id =
        Some("changed-input".into());
    assert_ne!(
        super::search::task_source_basis(&problem, &problem.theory.settlement_rules[0]).as_ref(),
        Some(&selected)
    );
    problem.theory.settlement_rules[0]
        .repeat_on_changed_owners
        .clear();
    assert!(
        super::search::task_source_basis(&problem, &problem.theory.settlement_rules[0]).is_none()
    );
}

#[test]
fn repetition_requires_complete_comparable_owner_inputs() {
    let mut problem = problem();
    let owner = problem.planner_cut.traversal_cut.receipts[0]
        .owner_id
        .clone();
    problem.theory.settlement_rules[0].repeat_on_changed_owners = vec![owner];
    let basis =
        super::search::task_source_basis(&problem, &problem.theory.settlement_rules[0]).unwrap();
    let request = StrategySearchRequest {
        problem: problem.clone(),
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    let task = search(&request).recommendation.unwrap().tasks.remove(0);
    assert_eq!(task.source_basis_id.as_ref(), Some(&basis));
    for incomparable in [None, Some("strategy-task-input-v1::old".into())] {
        let mut prior = task.clone();
        prior.source_basis_id = incomparable;
        assert!(super::search::same_work(&task, &prior));
        assert!(super::search::same_work(&prior, &task));
    }
    let mut changed = task.clone();
    changed.source_basis_id = Some("strategy-task-input-v2::changed".into());
    assert!(!super::search::same_work(&task, &changed));

    let mut missing = problem.planner_cut.traversal_cut.receipts[0].clone();
    missing.owner_id = "another-selected-owner".into();
    missing.work_input_basis_id = None;
    problem.theory.settlement_rules[0]
        .repeat_on_changed_owners
        .push(missing.owner_id.clone());
    assert!(
        super::search::task_source_basis(&problem, &problem.theory.settlement_rules[0]).is_none()
    );
    problem.planner_cut.traversal_cut.receipts.push(missing);
    assert!(
        super::search::task_source_basis(&problem, &problem.theory.settlement_rules[0]).is_none()
    );
}

// Method inputs may describe several Tasks; finished Plans store only those Tasks.
fn method_body(plan: &StrategyPlan) -> meld_lang::Composition {
    meld_lang::Composition {
        steps: plan
            .tasks
            .iter()
            .flat_map(|task| task.composition.steps.clone())
            .collect(),
        edges: plan
            .tasks
            .iter()
            .flat_map(|task| task.composition.edges.clone())
            .collect(),
    }
}

#[test]
fn applicable_rule_alternatives_use_the_same_ranking_and_exact_rule_verification() {
    let mut request = StrategySearchRequest {
        problem: problem(),
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    let valid = request.problem.theory.settlement_rules[0].clone();
    let mut impossible = valid.clone();
    impossible.evidence_route.outcome_contract_id = "unavailable-outcome".into();
    request.problem.theory.settlement_rules = vec![impossible.clone(), valid.clone()];
    let plan = search(&request)
        .recommendation
        .expect("later valid rule remains applicable");
    assert_eq!(
        plan.settlement_rule_id,
        super::search::settlement_rule_identity(&valid)
    );
    assert!(matches!(
        verify_plan(&request.problem, &plan),
        PlanVerification::Valid { .. }
    ));
    request.problem.theory.settlement_rules.reverse();
    assert_eq!(search(&request).recommendation.unwrap(), plan);
    let mut forged = plan.clone();
    forged.settlement_rule_id = super::search::settlement_rule_identity(&impossible);
    forged.plan_revision_id = super::search::plan_revision_identity(&forged);
    assert!(matches!(
        verify_plan(&request.problem, &forged),
        PlanVerification::Invalid { .. }
    ));
    let wire = serde_json::to_value(&plan).unwrap();
    assert!(wire.get("composition").is_none());
    assert!(wire.get("capability_contract_ids").is_none());
    assert!(wire["tasks"][0].get("composition").is_some());
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DependencyFixture {
    Preparation,
    OperationalConfirmation,
    VisibleConfirmation,
}
impl DependencyFixture {
    pub(crate) fn apply(self, rule: &mut StrategySettlementRule) {
        rule.epistemic_selections = vec![StrategyEpistemicSelection {
            rule_revision: None,
            evidence_return: self != Self::Preparation,
        }];
        rule.product_ordering = vec![match self {
            Self::Preparation => StrategyProductOrdering {
                condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
                before: StrategyProductSelector::AllEpistemic,
                after: StrategyProductSelector::AllTasks,
                milestone: StrategyDependencyMilestone::CurationVisible,
            },
            Self::OperationalConfirmation => StrategyProductOrdering {
                condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
                before: StrategyProductSelector::AllTasks,
                after: StrategyProductSelector::AllEpistemic,
                milestone: StrategyDependencyMilestone::ExecutionTerminal,
            },
            Self::VisibleConfirmation => StrategyProductOrdering {
                condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
                before: StrategyProductSelector::AllTasks,
                after: StrategyProductSelector::AllEpistemic,
                milestone: StrategyDependencyMilestone::EffectVisible,
            },
        }];
    }
}

#[test]
fn unknown_goal_selects_only_authorized_epistemic_products_without_dummy_tasks() {
    let mut request = StrategySearchRequest {
        problem: problem(),
        bounds: StrategySearchBounds {
            max_expansions: 32,
            max_depth: 8,
        },
    };
    request.problem.planner_cut.world_model_view.world_state = meld_lang::WorldState::empty();
    let rule = &mut request.problem.theory.settlement_rules[0];
    rule.construction = StrategyConstruction::ObserveUnknown;
    rule.epistemic_selections[0].evidence_return = true;
    rule.product_ordering.clear();
    request.problem.capabilities.clear();
    let plan = search(&request)
        .recommendation
        .expect("bounded epistemic response");
    assert_eq!(plan.origin, StrategyPlanOrigin::Epistemic);
    assert!(plan.tasks.is_empty());
    assert_eq!(plan.epistemic_operations.len(), 1);
    assert!(plan.epistemic_operations[0].return_evidence.is_some());
    assert!(matches!(
        verify_plan(&request.problem, &plan),
        PlanVerification::Valid { .. }
    ));
    request.problem.curation_operations.clear();
    assert!(search(&request).recommendation.is_none());
    assert!(matches!(
        verify_plan(&request.problem, &plan),
        PlanVerification::Invalid { .. }
    ));
}

pub(super) fn mixed_problem() -> StrategyProblem {
    let mut problem = compound_problem();
    let first = problem.curation_operations[0].clone();
    let mut next_rule = first.rule_revision.clone();
    next_rule.id = "rule-after-index".into();
    next_rule.content_hash = "after-index-hash".into();
    let second = CurationOperation::reconstruct(
        first.authority.clone(),
        next_rule,
        first.source_cut.clone(),
        first.traversal_request.clone(),
    )
    .unwrap();
    problem.curation_operations.push(second);
    let rule = &mut problem.theory.settlement_rules[0];
    rule.product_ordering = vec![
        StrategyProductOrdering {
            condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
            before: StrategyProductSelector::Epistemic {
                rule_id: "rule-docs".into(),
            },
            after: StrategyProductSelector::Task {
                contract_id: "contract-index-v1".into(),
            },
            milestone: StrategyDependencyMilestone::CurationVisible,
        },
        StrategyProductOrdering {
            condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
            before: StrategyProductSelector::Task {
                contract_id: "contract-index-v1".into(),
            },
            after: StrategyProductSelector::Epistemic {
                rule_id: "rule-after-index".into(),
            },
            milestone: StrategyDependencyMilestone::ExecutionTerminal,
        },
        StrategyProductOrdering {
            condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
            before: StrategyProductSelector::Epistemic {
                rule_id: "rule-after-index".into(),
            },
            after: StrategyProductSelector::Task {
                contract_id: "contract-evaluate-v1".into(),
            },
            milestone: StrategyDependencyMilestone::CurationVisible,
        },
    ];
    problem
}

#[test]
fn heterogeneous_products_have_local_dependencies_and_no_unrelated_serialization() {
    let request = StrategySearchRequest {
        problem: mixed_problem(),
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    let plan = search(&request).recommendation.expect("mixed Plan");
    assert_eq!(plan.tasks.len(), 2);
    assert_eq!(plan.epistemic_operations.len(), 2);
    assert_eq!(plan.dependencies.len(), 3);
    assert!(matches!(
        verify_plan(&request.problem, &plan),
        PlanVerification::Valid { .. }
    ));
    let first = &plan.epistemic_operations[0];
    let second = &plan.epistemic_operations[1];
    let index = plan
        .tasks
        .iter()
        .find(|task| {
            task.capability_contract_ids
                .contains(&"contract-index-v1".into())
        })
        .unwrap();
    let evaluate = plan
        .tasks
        .iter()
        .find(|task| {
            task.capability_contract_ids
                .contains(&"contract-evaluate-v1".into())
        })
        .unwrap();
    for (before, after) in [
        (&first.product_id, &index.task_id),
        (&index.task_id, &second.product_id),
        (&second.product_id, &evaluate.task_id),
    ] {
        assert!(plan
            .dependencies
            .iter()
            .any(|edge| &edge.producer_product_id == before && &edge.consumer_product_id == after));
    }
    assert!(!plan
        .dependencies
        .iter()
        .any(|edge| edge.producer_product_id == first.product_id
            && edge.consumer_product_id == evaluate.task_id));
    assert!(!plan
        .dependencies
        .iter()
        .any(|edge| edge.producer_product_id == second.product_id
            && edge.consumer_product_id == index.task_id));
    let mut missing = plan.clone();
    missing.dependencies.remove(1);
    missing.plan_revision_id = super::search::plan_revision_identity(&missing);
    assert!(matches!(
        verify_plan(&request.problem, &missing),
        PlanVerification::Invalid { .. }
    ));
}

#[test]
fn observation_construction_prunes_unused_preparation_and_respects_unavailable_alternatives() {
    let mut problem = mixed_problem();
    problem.planner_cut.world_model_view.world_state = meld_lang::WorldState::empty();
    problem.capabilities.clear();
    let rule = &mut problem.theory.settlement_rules[0];
    rule.construction = StrategyConstruction::ObserveUnknown;
    rule.product_ordering.clear();
    rule.epistemic_selections = problem
        .curation_operations
        .iter()
        .enumerate()
        .map(|(index, operation)| StrategyEpistemicSelection {
            rule_revision: Some(operation.rule_revision.clone()),
            evidence_return: index == 0,
        })
        .collect();
    let mut request = StrategySearchRequest {
        problem,
        bounds: StrategySearchBounds {
            max_expansions: 32,
            max_depth: 8,
        },
    };
    let first = search(&request).recommendation.unwrap();
    assert_eq!(first.epistemic_operations.len(), 1);
    assert!(matches!(
        verify_plan(&request.problem, &first),
        PlanVerification::Valid { .. }
    ));
    request
        .problem
        .unavailable_curation_operation_ids
        .push(first.epistemic_operations[0].operation.operation_id.clone());
    assert!(search(&request).recommendation.is_none());
    assert!(matches!(
        verify_plan(&request.problem, &first),
        PlanVerification::Invalid { .. }
    ));
    let mut alternative = request.problem.theory.settlement_rules[0].clone();
    alternative.epistemic_selections.remove(0);
    alternative.epistemic_selections[0].evidence_return = true;
    request.problem.theory.settlement_rules.push(alternative);
    let next = search_successor(&StrategySuccessorRequest {
        search: request.clone(),
        predecessor_plan: Box::new(first),
        completed_history: Vec::new(),
    })
    .recommendation
    .unwrap();
    assert_eq!(
        next.plan.epistemic_operations[0].operation.rule_revision,
        request.problem.curation_operations[1].rule_revision
    );
    assert!(matches!(
        verify_plan(&request.problem, &next.plan),
        PlanVerification::Valid { .. }
    ));
}

#[test]
fn confirmation_uses_only_the_required_return_closure() {
    let mut problem = problem();
    let first = problem.curation_operations[0].clone();
    let mut extra_revision = first.rule_revision.clone();
    extra_revision.id = "unused-preparation".into();
    let extra = CurationOperation::reconstruct(
        first.authority.clone(),
        extra_revision.clone(),
        first.source_cut.clone(),
        first.traversal_request.clone(),
    )
    .unwrap();
    problem.curation_operations.push(extra.clone());
    problem
        .unavailable_curation_operation_ids
        .push(extra.operation_id.clone());
    let rule = &mut problem.theory.settlement_rules[0];
    rule.epistemic_selections = vec![
        StrategyEpistemicSelection {
            rule_revision: Some(first.rule_revision.clone()),
            evidence_return: true,
        },
        StrategyEpistemicSelection {
            rule_revision: Some(extra_revision),
            evidence_return: false,
        },
    ];
    rule.product_ordering = vec![StrategyProductOrdering {
        condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
        before: StrategyProductSelector::AllTasks,
        after: StrategyProductSelector::Epistemic {
            rule_id: first.rule_revision.id.clone(),
        },
        milestone: StrategyDependencyMilestone::ExecutionTerminal,
    }];
    rule.product_ordering.push(StrategyProductOrdering {
        condition: crate::strategy::StrategyOrderingCondition::ConsumerSelected,
        before: StrategyProductSelector::Task {
            contract_id: "unused-contract".into(),
        },
        after: StrategyProductSelector::Epistemic {
            rule_id: "unused-preparation".into(),
        },
        milestone: StrategyDependencyMilestone::ExecutionTerminal,
    });
    let mut unused_capability = problem.capabilities[0].clone();
    unused_capability.contract_id = "unused-contract".into();
    unused_capability.operator.effects.clear();
    unused_capability.operator.operator_id = "unused-operator".into();
    unused_capability
        .operator
        .resolution
        .specific
        .as_mut()
        .unwrap()
        .capability_type_id = "unused-mechanism".into();
    problem.capabilities.push(unused_capability);
    let search_request = StrategySearchRequest {
        problem,
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    let plan = search(&search_request).recommendation.unwrap();
    assert_eq!(plan.epistemic_operations.len(), 1);
    let mut request = StrategySuccessorRequest {
        search: search_request,
        completed_history: plan
            .tasks
            .iter()
            .map(|task| StrategyCompletedHistoryEntry {
                source_plan_revision_id: plan.plan_revision_id.clone(),
                product_id: task.task_id.clone(),
                accepted_milestone: task.confirmation_milestone(),
                owner_position_id: "execution-return".into(),
                product: Some(StrategyProduct::Task(Box::new(task.clone()))),
            })
            .collect(),
        predecessor_plan: Box::new(plan),
    };
    let confirmation = search_successor(&request).recommendation.unwrap();
    assert_eq!(confirmation.plan.origin, StrategyPlanOrigin::Confirmation);
    assert_eq!(confirmation.plan.epistemic_operations.len(), 1);
    assert!(matches!(
        verify_successor_plan(&request, &confirmation),
        PlanVerification::Valid { .. }
    ));
    let operation = &confirmation.plan.epistemic_operations[0];
    request
        .completed_history
        .push(StrategyCompletedHistoryEntry {
            source_plan_revision_id: confirmation.plan.plan_revision_id.clone(),
            product_id: operation.product_id.clone(),
            accepted_milestone: PlanMilestoneRequirement::BeliefRevision {
                belief_key: "confirmed".into(),
                revision_id: "positive".into(),
            },
            owner_position_id: "belief-return".into(),
            product: Some(StrategyProduct::Epistemic(Box::new(operation.clone()))),
        });
    *request.predecessor_plan = confirmation.plan;
    request
        .search
        .problem
        .planner_cut
        .world_model_view
        .world_state =
        meld_lang::WorldState::new(vec![request.search.problem.goal.target.clone()]).unwrap();
    let satisfied = search_successor(&request).recommendation.unwrap();
    assert_eq!(satisfied.plan.origin, StrategyPlanOrigin::Satisfied);
    assert!(matches!(
        verify_successor_plan(&request, &satisfied),
        PlanVerification::Valid { .. }
    ));
}

#[test]
fn conditional_ordering_does_not_require_an_unselected_task() {
    let mut problem = problem();
    let consumer = problem.capabilities[0].contract_id.clone();
    let mut unused = problem.capabilities[0].clone();
    unused.contract_id = "optional-producer".into();
    unused.operator.operator_id = "optional-producer".into();
    unused.operator.effects.clear();
    unused
        .operator
        .resolution
        .specific
        .as_mut()
        .unwrap()
        .capability_type_id = "optional-producer".into();
    problem.capabilities.push(unused);
    problem.theory.settlement_rules[0]
        .product_ordering
        .push(StrategyProductOrdering {
            condition: StrategyOrderingCondition::ConsumerSelected,
            before: StrategyProductSelector::Task {
                contract_id: "optional-producer".into(),
            },
            after: StrategyProductSelector::Task {
                contract_id: consumer,
            },
            milestone: StrategyDependencyMilestone::ExecutionTerminal,
        });
    let mut request = StrategySearchRequest {
        problem,
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    assert!(search(&request).recommendation.is_none());
    request.problem.theory.settlement_rules[0]
        .product_ordering
        .last_mut()
        .unwrap()
        .condition = StrategyOrderingCondition::BothSelected;
    let plan = search(&request).recommendation.unwrap();
    assert_eq!(plan.tasks.len(), 1);
    assert!(matches!(
        verify_plan(&request.problem, &plan),
        PlanVerification::Valid { .. }
    ));
}

#[test]
fn current_negative_confirmation_does_not_repeat_its_own_effect_publication() {
    let mut problem = problem();
    DependencyFixture::OperationalConfirmation.apply(&mut problem.theory.settlement_rules[0]);
    problem.theory.settlement_rules[0].repeat_on_changed_owners =
        vec![problem.planner_cut.traversal_cut.receipts[0]
            .owner_id
            .clone()];
    let search_request = StrategySearchRequest {
        problem,
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    let plan = search(&search_request).recommendation.unwrap();
    let mut request = StrategySuccessorRequest {
        search: search_request,
        completed_history: plan
            .tasks
            .iter()
            .map(|task| StrategyCompletedHistoryEntry {
                source_plan_revision_id: plan.plan_revision_id.clone(),
                product_id: task.task_id.clone(),
                accepted_milestone: task.confirmation_milestone(),
                owner_position_id: "effect-completed".into(),
                product: Some(StrategyProduct::Task(Box::new(task.clone()))),
            })
            .collect(),
        predecessor_plan: Box::new(plan),
    };
    let prior_operation = &request.search.problem.curation_operations[0];
    let mut effect_cut = prior_operation.source_cut.clone();
    effect_cut.receipts[0].revision_id = "task-own-effect".into();
    effect_cut.cut_id =
        crate::world_state::graph::contracts::traversal_cut_identity(&effect_cut).unwrap();
    request.search.problem.planner_cut.traversal_cut = effect_cut.clone();
    request.search.problem.curation_operations = vec![CurationOperation::reconstruct(
        prior_operation.authority.clone(),
        prior_operation.rule_revision.clone(),
        effect_cut,
        prior_operation.traversal_request.clone(),
    )
    .unwrap()];
    let confirmation = search_successor(&request).recommendation.unwrap();
    assert_eq!(confirmation.plan.origin, StrategyPlanOrigin::Confirmation);
    let operation = &confirmation.plan.epistemic_operations[0];
    request
        .completed_history
        .push(StrategyCompletedHistoryEntry {
            source_plan_revision_id: confirmation.plan.plan_revision_id.clone(),
            product_id: operation.product_id.clone(),
            accepted_milestone: PlanMilestoneRequirement::BeliefRevision {
                belief_key: "negative".into(),
                revision_id: "negative-current".into(),
            },
            owner_position_id: "negative-current".into(),
            product: Some(StrategyProduct::Epistemic(Box::new(operation.clone()))),
        });
    *request.predecessor_plan = confirmation.plan;
    let standalone = search(&request.search).recommendation.unwrap();
    assert_eq!(
        standalone.tasks[0].source_basis_id,
        match &request.completed_history[0].product {
            Some(StrategyProduct::Task(task)) => task.source_basis_id.clone(),
            _ => unreachable!(),
        }
    );
    assert!(search_successor(&request).recommendation.is_none());
}

#[test]
fn missing_current_derived_evidence_allows_observation_but_not_executable_work() {
    let mut request = StrategySearchRequest {
        problem: problem(),
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 8,
        },
    };
    let retained = request
        .problem
        .planner_cut
        .world_model_view
        .world_state
        .propositions()
        .iter()
        .filter(|p| !matches!(p, Proposition::Holds { .. }))
        .cloned()
        .collect();
    request.problem.planner_cut.world_model_view.world_state =
        meld_lang::WorldState::new(retained).unwrap();
    let executable = search(&request).recommendation.unwrap();
    assert!(matches!(
        verify_plan(&request.problem, &executable),
        PlanVerification::Valid { .. }
    ));
    assert!(!executable.tasks.is_empty());
    request
        .problem
        .planner_cut
        .world_model_view
        .pending_derived_evidence = Some(crate::planner::PlannerDerivedEvidenceRequirement {
        curation_rule: request.problem.curation_operations[0].rule_revision.clone(),
        belief_family: crate::belief::TheoryRevisionRef {
            registry: "belief_family".into(),
            id: "docs".into(),
            content_hash: "family".into(),
        },
        outcome_mappings: vec![],
    });
    assert!(search(&request).recommendation.is_none());
    assert!(matches!(
        verify_plan(&request.problem, &executable),
        PlanVerification::Invalid { grounds } if grounds == vec![StrategyRejectionGround::InvalidComposition]
    ));
    let mut acquisition = request.problem.theory.settlement_rules[0].clone();
    acquisition.construction = StrategyConstruction::ObserveUnknown;
    acquisition.product_ordering.clear();
    acquisition.epistemic_selections[0].evidence_return = true;
    request.problem.theory.settlement_rules.push(acquisition);
    let observation = search(&request).recommendation.unwrap();
    assert!(observation.tasks.is_empty());
    assert!(!observation.epistemic_operations.is_empty());
    assert!(matches!(
        verify_plan(&request.problem, &observation),
        PlanVerification::Valid { .. }
    ));
}

fn hierarchical_problem() -> StrategyProblem {
    use meld_lang::{Composition, Method, Step, StepKind};
    let mut problem = problem();
    let draft = Proposition::Exists {
        scope: subject(),
        artifact_type: Term::ArtifactType("draft_docs".into()),
    };
    let receipt = Proposition::Exists {
        scope: subject(),
        artifact_type: Term::ArtifactType("freshness_evidence".into()),
    };
    problem.capabilities[0]
        .operator
        .preconditions
        .push(draft.clone());
    let method = |id: &str, trigger: Proposition, steps: Vec<Step>, edges| Method {
        method_id: id.into(),
        trigger,
        preconditions: vec![],
        composition: Composition { steps, edges },
        net_effects: vec![],
        cost: CostEstimate::zero(),
        preference: 0,
    };
    problem.methods = vec![
        method(
            "maintain",
            problem.goal.target.clone(),
            vec![Step {
                step_id: "repair".into(),
                kind: StepKind::Goal(receipt.clone()),
            }],
            vec![],
        ),
        method(
            "repair",
            receipt,
            vec![
                Step {
                    step_id: "candidate".into(),
                    kind: StepKind::Goal(draft.clone()),
                },
                Step {
                    step_id: "validate".into(),
                    kind: StepKind::Op(problem.capabilities[0].operator.clone()),
                },
            ],
            vec![meld_lang::Edge {
                from: "candidate".into(),
                to: "validate".into(),
                kind: meld_lang::EdgeKind::Ordering,
            }],
        ),
        method(
            "draft",
            draft,
            vec![Step {
                step_id: "write".into(),
                kind: StepKind::Op(problem.capabilities[1].operator.clone()),
            }],
            vec![],
        ),
    ];
    problem
}

#[test]
fn nested_methods_establish_later_preconditions_without_mutating_the_cut() {
    let problem = hierarchical_problem();
    let before = problem.planner_cut.world_model_view.world_state.clone();
    let result = search(&StrategySearchRequest {
        problem: problem.clone(),
        bounds: StrategySearchBounds {
            max_expansions: 256,
            max_depth: 8,
        },
    });
    let plan = result.recommendation.expect("nested plan");
    let proof = plan.decomposition.as_ref().expect("selected hierarchy");
    assert_eq!(proof.refinements.len(), 3);
    assert_eq!(proof.primitives.len(), 2);
    assert_eq!(plan.tasks.len(), 1);
    assert!(matches!(
        verify_plan(&problem, &plan),
        PlanVerification::Valid { .. }
    ));
    assert_eq!(problem.planner_cut.world_model_view.world_state, before);
    let decoded: StrategyPlan =
        serde_json::from_slice(&serde_json::to_vec(&plan).unwrap()).unwrap();
    assert_eq!(decoded, plan);
    assert_eq!(
        decoded.plan_revision_id,
        super::search::plan_revision_identity(&decoded)
    );
}

#[test]
fn refinement_rejects_forged_method_lineage_and_missing_causal_order() {
    let problem = hierarchical_problem();
    let plan = search(&StrategySearchRequest {
        problem: problem.clone(),
        bounds: StrategySearchBounds {
            max_expansions: 256,
            max_depth: 8,
        },
    })
    .recommendation
    .unwrap();
    let mut forged = plan.clone();
    if let StrategyRefinement::Method { method_id, .. } =
        &mut forged.decomposition.as_mut().unwrap().refinements[1]
    {
        *method_id = "invented".into();
    }
    forged.plan_revision_id = super::search::plan_revision_identity(&forged);
    assert!(matches!(
        verify_plan(&problem, &forged),
        PlanVerification::Invalid { .. }
    ));
    let mut unordered = plan;
    unordered.tasks[0].composition.edges.clear();
    unordered.plan_revision_id = super::search::plan_revision_identity(&unordered);
    assert!(matches!(
        verify_plan(&problem, &unordered),
        PlanVerification::Invalid { .. }
    ));
}

#[test]
fn method_applicability_changes_with_the_frozen_state() {
    let mut problem = hierarchical_problem();
    let token = Proposition::Exists {
        scope: subject(),
        artifact_type: Term::ArtifactType("preferred_route".into()),
    };
    let mut preferred = problem.methods[2].clone();
    preferred.method_id = "preferred-draft".into();
    preferred.preconditions = vec![token.clone()];
    preferred.composition.steps[0].step_id = "preferred-write".into();
    problem.methods[2].preconditions = vec![Proposition::Not(Box::new(token.clone()))];
    problem.planner_cut.world_model_view.world_state = problem
        .planner_cut
        .world_model_view
        .world_state
        .apply(&[Effect::Assert(Proposition::Not(Box::new(token.clone())))])
        .unwrap();
    problem.methods.push(preferred);
    let request = |problem| StrategySearchRequest {
        problem,
        bounds: StrategySearchBounds {
            max_expansions: 256,
            max_depth: 8,
        },
    };
    let unavailable = search(&request(problem.clone()));
    assert!(unavailable.rejections.iter().any(|r| matches!(r, StrategyRejectionGround::IndeterminateMethodPrecondition { method_id } | StrategyRejectionGround::UnsatisfiedMethodPrecondition { method_id } if method_id == "preferred-draft")));
    problem.planner_cut.world_model_view.world_state = problem
        .planner_cut
        .world_model_view
        .world_state
        .apply(&[
            Effect::Retract(Proposition::Not(Box::new(token.clone()))),
            Effect::Assert(token),
        ])
        .unwrap();
    let plan = search(&request(problem.clone())).recommendation.unwrap();
    assert!(plan.decomposition.as_ref().unwrap().refinements.iter().any(|r| matches!(r, StrategyRefinement::Method { method_id, .. } if method_id == "preferred-draft")));
    assert!(matches!(
        verify_plan(&problem, &plan),
        PlanVerification::Valid { .. }
    ));
}

#[test]
fn recursive_method_cycles_are_bounded_without_claiming_exhaustion() {
    let mut problem = hierarchical_problem();
    let root = problem.methods[0].clone();
    problem.methods[0].composition.steps[0].kind = meld_lang::StepKind::Goal(root.trigger);
    let result = search(&StrategySearchRequest {
        problem,
        bounds: StrategySearchBounds {
            max_expansions: 64,
            max_depth: 3,
        },
    });
    assert_eq!(result.completion, StrategySearchCompletion::Bounded);
    assert!(result
        .rejections
        .contains(&StrategyRejectionGround::BoundsExceeded));
}
