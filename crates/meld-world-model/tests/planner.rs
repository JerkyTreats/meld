use std::sync::Arc;

use meld_lang::{
    condition::Condition,
    effect::Effect,
    evaluate::{evaluate, EvalResult},
    proposition::Proposition,
    term::{Literal, Term},
};
use meld_world_model::belief::{
    BeliefProvenanceSummary, BeliefQuery, BeliefRuntime, BeliefStore, BranchScope,
    ContradictionState, FreshnessState, HydrationRefs, ObservationOpportunity, ObservationReason,
    PlannerProjectionSummary, PosteriorSummary,
};
use meld_world_model::events::{DomainObjectRef, EventRelation};
use meld_world_model::planner::{
    project_world_state, PlannerFieldProjectionConfig, PlannerGraphScope, PlannerHydrationRefs,
    PlannerProjectionContext, PlannerProjectionInput, PlannerProjectionWarning, PlannerQuery,
    PlannerSourceRef, PLANNER_PROJECTION_VERSION,
};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::{
    AnchorSelectionRecord, BeliefStatus, BeliefView, PerspectiveKey, TraversalFactRecord,
    TraversalQuery,
};
use proptest::prelude::*;

fn object(domain_id: &str, object_kind: &str, object_id: &str) -> DomainObjectRef {
    DomainObjectRef::new(domain_id, object_kind, object_id).unwrap()
}

fn subject() -> DomainObjectRef {
    object("workspace_fs", "node", "node-a")
}

fn config_json() -> &'static str {
    r#"{
        "family_id": "docs_freshness",
        "dimension_id": "docs_freshness",
        "predicate_id": "confidence",
        "evidence_policy_id": "default_policy",
        "evidence_schemas": [
            {
                "schema_id": "graph_anchor_signal",
                "required": true,
                "role": "Support",
                "reliability": 1.0,
                "precision": 1.0
            }
        ],
        "source_mappings": [
            {
                "mapping_id": "anchor_to_signal",
                "source_kind": "graph_anchor",
                "evidence_schema_id": "graph_anchor_signal",
                "subject_from": "anchor.subject",
                "value_field": "ended",
                "factor_id": "freshness_signal"
            }
        ],
        "comparator": {
            "engine_id": "weighted_bayesian",
            "engine_version": "1",
            "factors": [
                {
                    "factor_id": "freshness_signal",
                    "evidence_schema_id": "graph_anchor_signal",
                    "weight": 1.0,
                    "polarity": "Supports"
                }
            ],
            "missing_evidence_uncertainty": 0.9
        },
        "default_prior": 0.8,
        "planner_projection": {
            "confidence_field": "confidence",
            "threshold": 0.7,
            "posterior_meaning": "stale_probability"
        },
        "config_version": "1"
    }"#
}

fn seeded_graph() -> (tempfile::TempDir, Arc<TraversalStore>, DomainObjectRef) {
    let temp_dir = tempfile::tempdir().unwrap();
    let store =
        Arc::new(TraversalStore::new(sled::open(temp_dir.path().join("graph")).unwrap()).unwrap());
    let node = subject();
    let frame = object("context", "frame", "frame-a");
    let anchor_ref = object("context", "head", "node-a::analysis");
    let relation = EventRelation::new("selected", node.clone(), frame.clone()).unwrap();
    let fact = TraversalFactRecord {
        fact_id: "fact-a".to_string(),
        source_spine_fact_id: "ledger-a".to_string(),
        seq: 1,
        event_type: "context.head.selected".to_string(),
        objects: vec![node.clone(), frame.clone()],
        relations: vec![relation],
    };
    let anchor = AnchorSelectionRecord {
        anchor_id: "anchor-a".to_string(),
        anchor_ref,
        subject: node.clone(),
        perspective: PerspectiveKey::new("frame_type", "analysis").unwrap(),
        target: frame,
        source_fact_ids: vec!["ledger-a".to_string()],
        created_by_fact_id: "fact-a".to_string(),
        selected_at_seq: 1,
        ended_at_seq: None,
        ended_by_anchor_id: None,
        ended_by_fact_id: None,
    };
    store.put_fact(&fact).unwrap();
    store.put_anchor(&anchor).unwrap();
    store.set_current_anchor(&anchor).unwrap();
    (temp_dir, store, node)
}

fn test_view(dimension_id: &str, confidence: f64, stale: bool, observation: bool) -> BeliefView {
    let subject = subject();
    let perspective = PerspectiveKey::new("default", "default").unwrap();
    let branch_scope = BranchScope::main();
    let key = meld_world_model::BeliefKey {
        subject: subject.clone(),
        dimension_id: dimension_id.to_string(),
        predicate_id: "confidence".to_string(),
        perspective: perspective.clone(),
        branch_scope: branch_scope.clone(),
        evidence_policy_id: "default_policy".to_string(),
    };
    let observation = observation.then(|| ObservationOpportunity {
        opportunity_id: "observe-a".to_string(),
        belief_key: key.clone(),
        target_evidence_schema_id: "graph_anchor_signal".to_string(),
        reason: ObservationReason::MissingRequiredEvidence,
        detail: "missing evidence".to_string(),
        source_revision_id: Some("revision-a".to_string()),
        open: true,
    });

    BeliefView {
        view_id: "view-a".to_string(),
        key,
        current_revision_id: Some("revision-a".to_string()),
        status: BeliefStatus::Settled,
        posterior: PosteriorSummary {
            probability: confidence,
            meaning: "probability".to_string(),
        },
        planner_projection: PlannerProjectionSummary {
            confidence_field: "confidence".to_string(),
            confidence,
            threshold: 0.7,
        },
        uncertainty: 1.0 - confidence,
        precision: 1.0,
        freshness: FreshnessState {
            stale,
            reasons: Vec::new(),
            high_water_seq: 1,
        },
        contradiction: ContradictionState {
            contradicted: false,
            reasons: Vec::new(),
            supporting_evidence_ids: Vec::new(),
            contradicted_evidence_ids: Vec::new(),
        },
        observation,
        assessment_state: "settled".to_string(),
        advisory_posture: "ready".to_string(),
        provenance: BeliefProvenanceSummary {
            evidence_ids: vec!["evidence-a".to_string()],
            source_fact_ids: vec!["ledger-a".to_string()],
            graph_anchor_ids: vec!["anchor-a".to_string()],
            objects: vec![subject],
            relations: Vec::new(),
            revision_ids: vec!["revision-a".to_string()],
        },
        hydration: HydrationRefs {
            evidence_ids: vec!["evidence-a".to_string()],
            source_fact_ids: vec!["ledger-a".to_string()],
            graph_anchor_ids: vec!["anchor-a".to_string()],
            revision_id: Some("revision-a".to_string()),
        },
        theory_revision: None,
    }
}

fn projection_input(view: Option<BeliefView>) -> PlannerProjectionInput {
    PlannerProjectionInput {
        additional_beliefs: Vec::new(),
        context: PlannerProjectionContext::first_slice(subject()),
        belief_view: view,
        graph_scope: Some(PlannerGraphScope {
            accessible: true,
            anchor_ids: vec!["anchor-a".to_string()],
            source_fact_ids: vec!["ledger-a".to_string()],
        }),
        field_config: PlannerFieldProjectionConfig::default(),
    }
}

#[test]
fn planner_module_boundary() {
    assert!(!std::path::Path::new("src/planner/mod.rs").exists());
}

#[test]
fn planner_boundary() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in [
        "src/planner.rs",
        "src/planner/contracts.rs",
        "src/planner/projection.rs",
        "src/planner/query.rs",
    ] {
        let source = std::fs::read_to_string(manifest_dir.join(path)).unwrap();
        for forbidden in [
            "meld_execution",
            "Goal",
            "Method",
            "Composition",
            "Operator",
            "Effect",
            "Task",
        ] {
            assert!(!source.contains(forbidden), "{path} contains {forbidden}");
        }
    }
}

#[test]
fn planner_no_family_specific_identifiers() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in [
        "src/planner.rs",
        "src/planner/contracts.rs",
        "src/planner/projection.rs",
        "src/planner/query.rs",
    ] {
        let source = std::fs::read_to_string(manifest_dir.join(path)).unwrap();
        assert!(!source.contains("DocsFreshness"));
        assert!(!source.contains("ContentFreshness"));
        assert!(!source.contains("DocsFreshnessBayesianComparator"));
    }
}

#[test]
fn planner_contracts_round_trip() {
    let input = projection_input(Some(test_view("custom_dimension", 0.6, false, true)));
    let output = project_world_state(input.clone()).unwrap();
    let source_ref = PlannerSourceRef::ProjectionRule {
        rule_id: "rule-a".to_string(),
    };
    let hydration = PlannerHydrationRefs {
        evidence_ids: vec!["evidence-a".to_string()],
        source_fact_ids: vec!["ledger-a".to_string()],
        graph_anchor_ids: vec!["anchor-a".to_string()],
        revision_ids: vec!["revision-a".to_string()],
    };
    let warning = PlannerProjectionWarning::MissingGraphScope { subject: subject() };

    assert_eq!(
        serde_json::from_str::<PlannerProjectionInput>(&serde_json::to_string(&input).unwrap())
            .unwrap(),
        input
    );
    assert_eq!(
        serde_json::from_str::<PlannerSourceRef>(&serde_json::to_string(&source_ref).unwrap())
            .unwrap(),
        source_ref
    );
    assert_eq!(
        serde_json::from_str::<PlannerHydrationRefs>(&serde_json::to_string(&hydration).unwrap())
            .unwrap(),
        hydration
    );
    assert_eq!(
        serde_json::from_str::<PlannerProjectionWarning>(&serde_json::to_string(&warning).unwrap())
            .unwrap(),
        warning
    );
    assert_eq!(
        serde_json::from_str::<meld_world_model::WorldModelView>(
            &serde_json::to_string(&output).unwrap()
        )
        .unwrap(),
        output
    );
}

#[test]
fn planner_projection_shape() {
    let output = project_world_state(projection_input(Some(test_view(
        "dimension_a",
        0.8,
        true,
        true,
    ))))
    .unwrap();
    let propositions = output.world_state.propositions();

    assert!(!propositions.contains(&Proposition::Holds {
        subject: Term::Object(subject()),
        dimension: Term::Dimension("dimension_a".to_string()),
        condition: Condition::Equals(Term::Literal(Literal::Number(0.8))),
    }));
    assert!(propositions.contains(&Proposition::Holds {
        subject: Term::Object(subject()),
        dimension: Term::Dimension("dimension_a.stale".to_string()),
        condition: Condition::Equals(Term::Literal(Literal::Bool(true))),
    }));
    assert!(propositions.contains(&Proposition::Holds {
        subject: Term::Object(subject()),
        dimension: Term::Dimension("dimension_a.observation_needed".to_string()),
        condition: Condition::Equals(Term::Literal(Literal::Bool(true))),
    }));
    assert!(propositions.contains(&Proposition::Accessible {
        scope: Term::Object(subject()),
    }));
}

#[test]
fn unsettled_prior_cannot_establish_a_satisfied_proposition() {
    for status in [
        BeliefStatus::NeedsObservation,
        BeliefStatus::NeedsAssessment,
        BeliefStatus::AssessmentPending,
        BeliefStatus::Stale,
        BeliefStatus::Invalid,
    ] {
        let mut view = test_view("dimension_a", 1.0, false, false);
        view.status = status;
        let output = project_world_state(projection_input(Some(view))).unwrap();
        let query = Proposition::Holds {
            subject: Term::Object(subject()),
            dimension: Term::Dimension("dimension_a".to_string()),
            condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
        };
        assert!(matches!(
            evaluate(&output.world_state, &query),
            EvalResult::Indeterminate { .. }
        ));
        assert!(output
            .source_refs
            .contains(&PlannerSourceRef::BeliefRevision {
                revision_id: "revision-a".to_string(),
            }));
    }
}

#[test]
fn planner_belief_projection() {
    for confidence in [0.69, 0.7, 0.71] {
        let output = project_world_state(projection_input(Some(test_view(
            "dimension_a",
            confidence,
            false,
            false,
        ))))
        .unwrap();
        let query = Proposition::Holds {
            subject: Term::Object(subject()),
            dimension: Term::Dimension("dimension_a".to_string()),
            condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
        };
        let result = evaluate(&output.world_state, &query);
        if confidence > 0.7 {
            assert_eq!(result, EvalResult::Satisfied);
        } else {
            assert!(matches!(result, EvalResult::Unsatisfied { .. }));
        }
    }
}

#[test]
fn planner_indeterminate_projection() {
    let output = project_world_state(projection_input(None)).unwrap();
    let query = Proposition::Holds {
        subject: Term::Object(subject()),
        dimension: Term::Dimension("dimension_a".to_string()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
    };

    assert!(matches!(
        evaluate(&output.world_state, &query),
        EvalResult::Indeterminate { .. }
    ));
    assert!(output
        .warnings
        .contains(&PlannerProjectionWarning::MissingBelief { subject: subject() }));
}

#[test]
fn planner_graph_projection() {
    let output = project_world_state(PlannerProjectionInput {
        additional_beliefs: Vec::new(),
        context: PlannerProjectionContext::first_slice(subject()),
        belief_view: None,
        graph_scope: Some(PlannerGraphScope {
            accessible: true,
            anchor_ids: vec!["anchor-a".to_string()],
            source_fact_ids: vec!["ledger-a".to_string()],
        }),
        field_config: PlannerFieldProjectionConfig::default(),
    })
    .unwrap();

    assert_eq!(output.world_state.propositions().len(), 1);
    assert_eq!(
        output.world_state.propositions()[0],
        Proposition::Accessible {
            scope: Term::Object(subject())
        }
    );
    assert!(output
        .hydration_refs
        .graph_anchor_ids
        .contains(&"anchor-a".to_string()));
}

#[test]
fn planner_graph_does_not_fabricate_belief() {
    let output = project_world_state(projection_input(None)).unwrap();

    assert!(!output
        .world_state
        .propositions()
        .iter()
        .any(|proposition| matches!(proposition, Proposition::Holds { .. })));
}

#[test]
fn planner_world_state_grounding() {
    let output = project_world_state(projection_input(Some(test_view(
        "dimension_a",
        0.5,
        false,
        false,
    ))))
    .unwrap();

    assert!(output
        .world_state
        .propositions()
        .iter()
        .all(Proposition::is_ground));
}

#[test]
fn planner_determinism() {
    let mut input = projection_input(Some(test_view("dimension_a", 0.5, false, false)));
    input.graph_scope = Some(PlannerGraphScope {
        accessible: true,
        anchor_ids: vec![
            "anchor-b".to_string(),
            "anchor-a".to_string(),
            "anchor-a".to_string(),
        ],
        source_fact_ids: vec![
            "ledger-b".to_string(),
            "ledger-a".to_string(),
            "ledger-a".to_string(),
        ],
    });

    let first = project_world_state(input.clone()).unwrap();
    let second = project_world_state(input).unwrap();

    assert_eq!(first, second);
    assert_eq!(
        first.hydration_refs.graph_anchor_ids,
        vec!["anchor-a".to_string(), "anchor-b".to_string()]
    );
}

#[test]
fn planner_query() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph.clone(), config_json())
            .unwrap();
    runtime
        .assess_subject(&node, "frame_type", "analysis", "worker-a")
        .unwrap();

    let query = PlannerQuery::new(
        BeliefQuery::new(belief_store.as_ref()),
        TraversalQuery::new(graph.as_ref()),
    );
    let output = query
        .project_current_world_state(&node, "docs_freshness", None, None)
        .unwrap();

    assert_eq!(output.projection_version, PLANNER_PROJECTION_VERSION);
    assert!(output
        .world_state
        .propositions()
        .iter()
        .any(|proposition| matches!(proposition, Proposition::Holds { .. })));
    assert!(output
        .world_state
        .propositions()
        .contains(&Proposition::Accessible {
            scope: Term::Object(node)
        }));
}

#[test]
fn planner_query_reopen() {
    let graph_dir = tempfile::tempdir().unwrap();
    let belief_dir = tempfile::tempdir().unwrap();
    let graph_path = graph_dir.path().join("graph");
    let belief_path = belief_dir.path().join("belief");
    let node = subject();

    {
        let graph = Arc::new(TraversalStore::new(sled::open(&graph_path).unwrap()).unwrap());
        let frame = object("context", "frame", "frame-a");
        let anchor_ref = object("context", "head", "node-a::analysis");
        let relation = EventRelation::new("selected", node.clone(), frame.clone()).unwrap();
        let fact = TraversalFactRecord {
            fact_id: "fact-a".to_string(),
            source_spine_fact_id: "ledger-a".to_string(),
            seq: 1,
            event_type: "context.head.selected".to_string(),
            objects: vec![node.clone(), frame.clone()],
            relations: vec![relation],
        };
        let anchor = AnchorSelectionRecord {
            anchor_id: "anchor-a".to_string(),
            anchor_ref,
            subject: node.clone(),
            perspective: PerspectiveKey::new("frame_type", "analysis").unwrap(),
            target: frame,
            source_fact_ids: vec!["ledger-a".to_string()],
            created_by_fact_id: "fact-a".to_string(),
            selected_at_seq: 1,
            ended_at_seq: None,
            ended_by_anchor_id: None,
            ended_by_fact_id: None,
        };
        graph.put_fact(&fact).unwrap();
        graph.put_anchor(&anchor).unwrap();
        graph.set_current_anchor(&anchor).unwrap();
        let belief_store = Arc::new(BeliefStore::new(sled::open(&belief_path).unwrap()).unwrap());
        let runtime =
            BeliefRuntime::from_json_config(belief_store.clone(), graph.clone(), config_json())
                .unwrap();
        runtime
            .assess_subject(&node, "frame_type", "analysis", "worker-a")
            .unwrap();
        graph.flush().unwrap();
        belief_store.flush().unwrap();
    }

    let first_graph = TraversalStore::new(sled::open(&graph_path).unwrap()).unwrap();
    let first_belief = BeliefStore::new(sled::open(&belief_path).unwrap()).unwrap();
    let first = PlannerQuery::new(
        BeliefQuery::new(&first_belief),
        TraversalQuery::new(&first_graph),
    )
    .project_current_world_state(&node, "docs_freshness", None, None)
    .unwrap();
    drop(first_graph);
    drop(first_belief);

    let second_graph = TraversalStore::new(sled::open(&graph_path).unwrap()).unwrap();
    let second_belief = BeliefStore::new(sled::open(&belief_path).unwrap()).unwrap();
    let second = PlannerQuery::new(
        BeliefQuery::new(&second_belief),
        TraversalQuery::new(&second_graph),
    )
    .project_current_world_state(&node, "docs_freshness", None, None)
    .unwrap();

    assert_eq!(first, second);
}

#[test]
fn planner_typed_loop_handoff() {
    let output = project_world_state(projection_input(Some(test_view(
        "dimension_a",
        0.8,
        false,
        false,
    ))))
    .unwrap();
    let goal_target = Proposition::Holds {
        subject: Term::Object(subject()),
        dimension: Term::Dimension("dimension_a".to_string()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
    };

    assert_eq!(
        evaluate(&output.world_state, &goal_target),
        EvalResult::Satisfied
    );
}

// Characterization for slice six (flywheel parity workstream): this is the
// hole the slice guards. A method's declared Update effect, applied to the
// planner projection when no evidence has been admitted, satisfies the goal
// target by itself — declared effects currently stand in for observation.
#[test]
fn declared_effects_alone_currently_satisfy_goal() {
    let projected = project_world_state(projection_input(None)).unwrap();
    let goal_target = Proposition::Holds {
        subject: Term::Object(subject()),
        dimension: Term::Dimension("docs_freshness".to_string()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
    };

    // With no admitted evidence the projection carries no Holds proposition,
    // so the goal is indeterminate rather than satisfied.
    assert!(matches!(
        evaluate(&projected.world_state, &goal_target),
        EvalResult::Indeterminate { .. }
    ));
    assert!(projected
        .warnings
        .contains(&PlannerProjectionWarning::MissingBelief { subject: subject() }));

    // refresh_docs_v1's declared effect Update(?node, docs_freshness, 0.95),
    // grounded to the projection subject.
    let after_declared = projected
        .world_state
        .apply(&[Effect::Update {
            subject: Term::Object(subject()),
            dimension: Term::Dimension("docs_freshness".to_string()),
            value: Term::Literal(Literal::Number(0.95)),
        }])
        .unwrap();

    assert_eq!(
        evaluate(&after_declared, &goal_target),
        EvalResult::Satisfied
    );
}

proptest! {
    #[test]
    fn planner_projection_proptest(
        confidence in 0.0f64..=1.0,
        dimension in "[a-z][a-z0-9_]{0,24}",
        stale in any::<bool>(),
        observation in any::<bool>(),
    ) {
        let output = project_world_state(projection_input(Some(test_view(
            &dimension,
            confidence,
            stale,
            observation,
        ))))
        .unwrap();

        prop_assert!(output.world_state.propositions().iter().all(Proposition::is_ground));
        let found_dimension = output.world_state.propositions().iter().any(|proposition| {
            matches!(
                proposition,
                Proposition::Holds {
                    dimension: Term::Dimension(found),
                    ..
                } if found == &dimension
            )
        });
        prop_assert_eq!(found_dimension, !stale && !observation);
    }

    #[test]
    fn planner_rejects_invalid_field_suffixes(raw in "\\PC*") {
        let mut input = projection_input(Some(test_view("dimension_a", 0.5, false, false)));
        input.field_config.stale_field_suffix = raw.clone();
        let result = project_world_state(input);

        if raw.trim().is_empty() || raw.contains(char::is_whitespace) || raw.contains("..") {
            prop_assert!(result.is_err());
        } else {
            prop_assert!(result.is_ok());
        }
    }
}

#[test]
fn selected_beliefs_preserve_independent_evidence_absence_and_exact_lineage() {
    use meld_world_model::planner::{PlannerBeliefSelection, PlannerSelectedBeliefView};
    let mut coverage = test_view("coverage", 1.0, false, false);
    coverage.current_revision_id = Some("coverage-revision".into());
    coverage.theory_revision = Some(meld_world_model::belief::TheoryRevisionRef {
        registry: "belief_family".into(),
        id: "coverage-family".into(),
        content_hash: "installed-coverage".into(),
    });
    let selected = PlannerSelectedBeliefView {
        selection: PlannerBeliefSelection {
            key: coverage.key.clone(),
            family: coverage.theory_revision.clone().unwrap(),
        },
        view: Some(coverage),
    };
    let mut input = projection_input(Some(test_view("posture", 0.0, false, false)));
    input.additional_beliefs = vec![selected.clone()];
    let guard = Proposition::Holds {
        subject: Term::Object(subject()),
        dimension: Term::Dimension("coverage".into()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.9))),
    };
    let admitted = project_world_state(input.clone()).unwrap();
    assert_eq!(
        evaluate(&admitted.world_state, &guard),
        EvalResult::Satisfied
    );
    assert!(admitted
        .hydration_refs
        .revision_ids
        .contains(&"coverage-revision".into()));
    input.additional_beliefs[0].view = None;
    let absent = project_world_state(input.clone()).unwrap();
    assert!(matches!(
        evaluate(&absent.world_state, &guard),
        EvalResult::Indeterminate { .. }
    ));
    assert!(absent.source_refs.iter().any(|source| matches!(source,
        PlannerSourceRef::BeliefSelection { content_hash, .. } if content_hash == "installed-coverage"
    )));
    input.additional_beliefs = vec![selected.clone()];
    input.additional_beliefs[0]
        .view
        .as_mut()
        .unwrap()
        .freshness
        .stale = true;
    assert!(matches!(
        evaluate(
            &project_world_state(input.clone()).unwrap().world_state,
            &guard
        ),
        EvalResult::Indeterminate { .. }
    ));
    input.additional_beliefs = vec![selected.clone()];
    input.additional_beliefs[0]
        .view
        .as_mut()
        .unwrap()
        .theory_revision
        .as_mut()
        .unwrap()
        .content_hash = "foreign-family-revision".into();
    assert!(project_world_state(input.clone()).is_err());
    input.additional_beliefs = vec![selected.clone(), selected.clone()];
    assert!(project_world_state(input.clone()).is_err());
    input.additional_beliefs = vec![selected];
    input.additional_beliefs[0]
        .selection
        .key
        .branch_scope
        .branch_id = "foreign-branch".into();
    assert!(project_world_state(input).is_err());
}
