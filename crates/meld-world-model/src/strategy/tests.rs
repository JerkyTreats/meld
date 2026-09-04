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
            owner_id: "workspace_fs".into(),
            scope: scope.clone(),
            required: true,
        }],
        receipts: vec![OwnerGraphRevisionReceipt {
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
            source_event: meld_events::EventRecordRef { ledger_id, seq: 7 },
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
        context_id: "context-docs-v1".into(),
        agent_id: "agent-docs".into(),
        goal_id: "goal-docs".into(),
        subject: subject_ref(),
        scope_id: "meld".into(),
        branch_id: "main".into(),
        perspective_id: "default".into(),
        authority_scope_id: "authority-docs".into(),
        activation_generation: "activation-docs".into(),
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
        problem_id: "problem-docs-v1".into(),
        goal: goal(),
        planner_cut: cut,
        theory: StrategyTheorySnapshot {
            theory_id: "theory-docs-v1".into(),
            settlement_rules: vec![StrategySettlementRule {
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
        PlanMilestoneRequirement::CurationTerminal { .. }
    ));
    assert_eq!(
        verify_plan(&problem, &plan),
        PlanVerification::Valid {
            evaluation: plan.evaluation.clone()
        }
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
