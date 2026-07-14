use std::cell::Cell;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use meld_lang::{Goal, GoalLifecycle};
use meld_world_model::agent::{
    curate_goal_satisfaction, curate_threshold_rule, ActiveGoalSummary, AgentActiveGoalQueryError,
    AgentAuthoredCommand, AgentCommandOutcomeQueryError, AgentCuration, AgentCurationDecision,
    AgentCurationRuleConfig, AgentGoalCommand, AgentGoalCurationRuntime, AgentGoalMutationCommand,
    AgentQuery, AgentRegistration, AgentSatisfactionCurationRuntime,
    AgentSatisfactionReviewSelection, AgentSelectedGoalTick, AgentSemanticSelector, AgentSinkError,
    AgentSinkReceipt, AgentSinkReceiptKind, AgentSinkSubmission, AgentStatus, AgentStore,
    AgentSubscription, AgentSubscriptionRecord, SeedAgentRegistration, SubscribeAgentCommand,
    BELIEF_REVISION_REVIEW_SOURCE,
};
use meld_world_model::belief::{
    BeliefProvenanceSummary, BeliefQuery, BeliefRevision, BeliefStore, BranchScope,
    ContradictionState, FreshnessState, HydrationRefs, PlannerProjectionSummary, PosteriorSummary,
};
use meld_world_model::events::DomainObjectRef;
use meld_world_model::planner::PlannerQuery;
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::{BeliefKey, BeliefStatus, BeliefView, PerspectiveKey, TraversalQuery};

const DIMENSION_ID: &str = "semantic_confidence";

fn reopen_database(path: &Path) -> sled::Db {
    for attempt in 0..50 {
        match sled::open(path) {
            Ok(db) => return db,
            Err(error) if error.to_string().contains("could not acquire lock") && attempt < 49 => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("reopen database: {error}"),
        }
    }
    unreachable!("the final reopen attempt returns")
}

fn subject() -> DomainObjectRef {
    DomainObjectRef::new("workspace", "node", "semantic-a").expect("subject")
}

fn belief_key() -> BeliefKey {
    BeliefKey {
        subject: subject(),
        dimension_id: DIMENSION_ID.to_string(),
        predicate_id: "confidence".to_string(),
        perspective: PerspectiveKey::new("default", "default").expect("perspective"),
        branch_scope: BranchScope::main(),
        evidence_policy_id: "semantic-policy".to_string(),
    }
}

fn rule_config() -> AgentCurationRuleConfig {
    AgentCurationRuleConfig {
        dimension_id: DIMENSION_ID.to_string(),
        threshold: 0.7,
        priority_urgency: 50,
        desired_summary: "confidence above threshold".to_string(),
        source_kind: "semantic-test".to_string(),
    }
}

fn register_operational_agent(
    db: &sled::Db,
    store: &AgentStore,
    agent_id: &str,
    created_at_seq: u64,
) -> AgentSubscriptionRecord {
    let mut agent = AgentRegistration::new(store)
        .register_seed_agent(SeedAgentRegistration {
            agent_id: agent_id.to_string(),
            perspective_key: belief_key().perspective,
            subject: subject(),
            branch_scope: BranchScope::main(),
            observation_scope: DIMENSION_ID.to_string(),
            directive_id: format!("directive-{agent_id}"),
            seed_provenance: "semantic test fixture".to_string(),
            created_at_seq,
        })
        .expect("register agent");
    let subscription = AgentSubscription::new(store)
        .subscribe(SubscribeAgentCommand {
            agent_id: agent_id.to_string(),
            belief_key: belief_key(),
            created_at_seq: created_at_seq + 1,
        })
        .expect("subscribe agent");

    // The focused selector tests do not exercise hydration. They install the
    // exact durable post-hydration record and its status index as fixture state.
    agent.status = AgentStatus::Operational;
    agent.updated_at_seq = created_at_seq + 2;
    db.open_tree("agent_records")
        .expect("agent records")
        .insert(
            agent.agent_id.as_bytes(),
            serde_json::to_vec(&agent).expect("encode agent"),
        )
        .expect("write operational agent");
    db.open_tree("agent_by_status")
        .expect("agent status index")
        .insert(
            format!(
                "operational::{:020}::{}",
                agent.updated_at_seq, agent.agent_id
            )
            .as_bytes(),
            agent.agent_id.as_bytes(),
        )
        .expect("index operational agent");
    subscription
}

fn install_belief(db: &sled::Db, confidence: f64, revision_id: &str, seq: u64) {
    let key = belief_key();
    let revision = BeliefRevision {
        revision_id: revision_id.to_string(),
        belief_key: key.clone(),
        prior_revision_id: None,
        comparator_engine_id: "semantic-test".to_string(),
        comparator_engine_version: "1".to_string(),
        config_snapshot_hash: "semantic-config".to_string(),
        evidence_ids: vec![format!("evidence-{revision_id}")],
        supporting_evidence_ids: vec![format!("evidence-{revision_id}")],
        contradicted_evidence_ids: Vec::new(),
        source_cursor_start: seq,
        source_cursor_end: seq,
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
            stale: false,
            reasons: Vec::new(),
            high_water_seq: seq,
        },
        contradiction: ContradictionState {
            contradicted: false,
            reasons: Vec::new(),
            supporting_evidence_ids: Vec::new(),
            contradicted_evidence_ids: Vec::new(),
        },
        status: BeliefStatus::Settled,
        observation: None,
        provenance: BeliefProvenanceSummary::empty(),
    };
    db.open_tree("belief_revisions")
        .expect("belief revisions")
        .insert(
            revision.revision_id.as_bytes(),
            serde_json::to_vec(&revision).expect("encode revision"),
        )
        .expect("write revision");
    BeliefStore::new(db.clone())
        .expect("belief store")
        .put_view(&BeliefView {
            view_id: format!("view-{revision_id}"),
            key,
            current_revision_id: Some(revision_id.to_string()),
            status: BeliefStatus::Settled,
            posterior: revision.posterior,
            planner_projection: revision.planner_projection,
            uncertainty: revision.uncertainty,
            precision: revision.precision,
            freshness: revision.freshness,
            contradiction: revision.contradiction,
            observation: None,
            assessment_state: "settled".to_string(),
            advisory_posture: "ready".to_string(),
            provenance: BeliefProvenanceSummary {
                revision_ids: vec![revision_id.to_string()],
                ..BeliefProvenanceSummary::empty()
            },
            hydration: HydrationRefs {
                evidence_ids: Vec::new(),
                source_fact_ids: Vec::new(),
                graph_anchor_ids: Vec::new(),
                revision_id: Some(revision_id.to_string()),
            },
        })
        .expect("write belief view");
}

fn prepare_mutation_fixture(
    db: &sled::Db,
    store: &AgentStore,
) -> (
    BeliefStore,
    Arc<TraversalStore>,
    AgentSatisfactionReviewSelection,
    Goal,
) {
    register_operational_agent(db, store, "agent-a", 1);
    install_belief(db, 0.2, "revision-7", 7);
    let belief_store = BeliefStore::new(db.clone()).expect("belief store");
    let traversal_store = Arc::new(TraversalStore::new(db.clone()).expect("traversal store"));
    let belief_query = BeliefQuery::new(&belief_store);
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(&belief_store),
        TraversalQuery::new(&traversal_store),
    );
    let delivery = AgentSemanticSelector::new(store)
        .select_deliveries(&belief_query, 1)
        .expect("select low-confidence delivery")
        .pop()
        .expect("low-confidence delivery");
    let input = AgentCuration::new(store)
        .assemble_input(
            &delivery.delivery,
            &belief_query,
            &planner_query,
            ActiveGoalSummary::default(),
            rule_config(),
        )
        .expect("assemble low-confidence curation");
    let mut goal = curate_threshold_rule(input)
        .expect("curate low confidence")
        .goal_command
        .expect("proposed goal")
        .goal;
    goal.lifecycle = GoalLifecycle::Active;

    install_belief(db, 0.95, "revision-8", 8);
    let selection = AgentSemanticSelector::new(store)
        .select_satisfaction_reviews(
            &BeliefQuery::new(&belief_store),
            BELIEF_REVISION_REVIEW_SOURCE,
            1,
        )
        .expect("select satisfaction mutation")
        .pop()
        .expect("pending satisfaction mutation");
    (belief_store, traversal_store, selection, goal)
}

fn persist_goal_command_decision(db: &sled::Db, store: &AgentStore) -> AgentCurationDecision {
    register_operational_agent(db, store, "agent-a", 1);
    install_belief(db, 0.2, "revision-7", 7);
    let belief_store = BeliefStore::new(db.clone()).expect("belief store");
    let traversal_store = Arc::new(TraversalStore::new(db.clone()).expect("traversal store"));
    let belief_query = BeliefQuery::new(&belief_store);
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(&belief_store),
        TraversalQuery::new(&traversal_store),
    );
    let selection = AgentSemanticSelector::new(store)
        .select_deliveries(&belief_query, 1)
        .expect("select delivery")
        .pop()
        .expect("pending delivery");
    let input = AgentCuration::new(store)
        .assemble_input(
            &selection.delivery,
            &belief_query,
            &planner_query,
            ActiveGoalSummary::default(),
            rule_config(),
        )
        .expect("assemble curation input");
    let outcome = curate_threshold_rule(input).expect("curate delivery");
    store
        .put_selected_delivery_outcome(&selection, &outcome)
        .expect("persist selected outcome")
        .decision
}

#[test]
fn selector_uses_operational_agents_and_stable_global_order() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db = sled::open(temp.path()).expect("database");
    let store = AgentStore::new(db.clone()).expect("agent store");
    register_operational_agent(&db, &store, "agent-z", 10);
    register_operational_agent(&db, &store, "agent-a", 20);
    AgentRegistration::new(&store)
        .register_seed_agent(SeedAgentRegistration {
            agent_id: "agent-registered".to_string(),
            perspective_key: belief_key().perspective,
            subject: subject(),
            branch_scope: BranchScope::main(),
            observation_scope: DIMENSION_ID.to_string(),
            directive_id: "directive-registered".to_string(),
            seed_provenance: "semantic test fixture".to_string(),
            created_at_seq: 30,
        })
        .expect("registered-only agent");
    install_belief(&db, 0.2, "revision-7", 7);
    let belief_store = BeliefStore::new(db.clone()).expect("belief store");
    let belief_query = BeliefQuery::new(&belief_store);

    let selections = AgentSemanticSelector::new(&store)
        .select_deliveries(&belief_query, 10)
        .expect("select deliveries");

    assert_eq!(selections.len(), 2);
    assert_eq!(selections[0].delivery.agent_id, "agent-a");
    assert_eq!(selections[1].delivery.agent_id, "agent-z");
    assert!(selections
        .iter()
        .all(|selection| selection.expected_delivered_seq == 0));
}

#[test]
fn selected_delivery_recovers_execution_outcome_after_reopen_without_resubmit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let selection;
    {
        let db = sled::open(temp.path()).expect("database");
        let store = AgentStore::new(db.clone()).expect("agent store");
        let subscription = register_operational_agent(&db, &store, "agent-a", 1);
        install_belief(&db, 0.2, "revision-7", 7);
        let belief_store = BeliefStore::new(db.clone()).expect("belief store");
        let traversal_store = Arc::new(TraversalStore::new(db.clone()).expect("traversal store"));
        let belief_query = BeliefQuery::new(&belief_store);
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(&belief_store),
            TraversalQuery::new(&traversal_store),
        );
        selection = AgentSemanticSelector::new(&store)
            .select_deliveries(&belief_query, 1)
            .expect("select delivery")
            .pop()
            .expect("pending delivery");
        assert_eq!(
            selection.delivery.subscription_id,
            subscription.subscription_id
        );
        let mut goal_query = |_agent_id: &str| {
            Ok::<ActiveGoalSummary, AgentActiveGoalQueryError>(ActiveGoalSummary::default())
        };
        let mut outcome_query = |_decision: &AgentCurationDecision,
                                 _command: &AgentAuthoredCommand| {
            Ok::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(None)
        };
        let mut sink = |_command: &AgentGoalCommand| {
            Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::retryable(
                "execution result was lost",
            ))
        };

        let report = AgentGoalCurationRuntime::new(&store).handle_selected_delivery(
            AgentSelectedGoalTick {
                selection: selection.clone(),
                belief_query: &belief_query,
                planner_query: &planner_query,
                rule_config: rule_config(),
            },
            &mut goal_query,
            &mut outcome_query,
            &mut sink,
        );

        assert_eq!(report.decision_count, 1);
        assert_eq!(report.sink_submission_count, 1);
        assert_eq!(report.retryable_errors, vec!["execution result was lost"]);
        let decision_id = AgentQuery::new(&store)
            .recent_decisions("agent-a", 1)
            .expect("decisions")[0]
            .decision_id
            .clone();
        assert!(AgentQuery::new(&store)
            .decision_outbox(&decision_id)
            .expect("outbox")
            .is_some());
        assert_eq!(
            store
                .get_subscription(&subscription.subscription_id)
                .expect("subscription")
                .expect("subscription record")
                .last_delivered_seq,
            0
        );
        db.open_tree("agent_sink_receipts")
            .expect("sink receipts")
            .insert(decision_id.as_bytes(), b"not-json")
            .expect("write corrupt receipt");
        let outcome_called = Cell::new(false);
        let sink_called = Cell::new(false);
        let mut replay_goal_query = |_agent_id: &str| {
            Err::<ActiveGoalSummary, AgentActiveGoalQueryError>(AgentActiveGoalQueryError::fatal(
                "durable replay must bypass active goals",
            ))
        };
        let mut replay_outcome_query =
            |_decision: &AgentCurationDecision, _command: &AgentAuthoredCommand| {
                outcome_called.set(true);
                Err::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(
                    AgentCommandOutcomeQueryError::fatal(
                        "corrupt receipt must stop before outcome recovery",
                    ),
                )
            };
        let mut replay_sink = |_command: &AgentGoalCommand| {
            sink_called.set(true);
            Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::fatal(
                "corrupt receipt must stop before sink submission",
            ))
        };
        let corrupt_report = AgentGoalCurationRuntime::new(&store).handle_selected_delivery(
            AgentSelectedGoalTick {
                selection: selection.clone(),
                belief_query: &belief_query,
                planner_query: &planner_query,
                rule_config: rule_config(),
            },
            &mut replay_goal_query,
            &mut replay_outcome_query,
            &mut replay_sink,
        );
        assert_eq!(corrupt_report.retryable_errors.len(), 1);
        assert!(!outcome_called.get());
        assert!(!sink_called.get());
        db.open_tree("agent_sink_receipts")
            .expect("sink receipts")
            .remove(decision_id.as_bytes())
            .expect("remove corrupt receipt");
        store.flush().expect("flush first attempt");
    }

    {
        let db = sled::open(temp.path()).expect("reopen database");
        let store = AgentStore::new(db.clone()).expect("reopen agent store");
        let belief_store = BeliefStore::new(db.clone()).expect("reopen belief store");
        let traversal_store = Arc::new(TraversalStore::new(db).expect("reopen traversal store"));
        let belief_query = BeliefQuery::new(&belief_store);
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(&belief_store),
            TraversalQuery::new(&traversal_store),
        );
        let mut goal_query = |_agent_id: &str| {
            Err::<ActiveGoalSummary, AgentActiveGoalQueryError>(AgentActiveGoalQueryError::fatal(
                "durable replay must bypass active goals",
            ))
        };
        let mut outcome_query = |_decision: &AgentCurationDecision,
                                 command: &AgentAuthoredCommand| {
            Ok::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(Some(
                AgentSinkSubmission::new(command.command_id(), command.goal_id(), "recovered"),
            ))
        };
        let sink_called = Cell::new(false);
        let mut sink = |_command: &AgentGoalCommand| {
            sink_called.set(true);
            Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::fatal(
                "sink must not be called after outcome recovery",
            ))
        };

        let report = AgentGoalCurationRuntime::new(&store).handle_selected_delivery(
            AgentSelectedGoalTick {
                selection: selection.clone(),
                belief_query: &belief_query,
                planner_query: &planner_query,
                rule_config: rule_config(),
            },
            &mut goal_query,
            &mut outcome_query,
            &mut sink,
        );

        assert!(report.retryable_errors.is_empty());
        assert!(report.fatal_errors.is_empty());
        assert_eq!(report.sink_submission_count, 0);
        assert!(!sink_called.get());
        assert_eq!(report.output_sequence, 7);
        assert_eq!(report.sink_receipts.len(), 1);
        assert_eq!(
            store
                .get_subscription(&selection.delivery.subscription_id)
                .expect("subscription")
                .expect("subscription record")
                .last_delivered_seq,
            7
        );
        AgentQuery::new(&store)
            .semantic_enablement_audit()
            .expect("semantic state is enableable");
    }
}

#[test]
fn satisfaction_cursor_advances_independently_from_delivery_progress() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db = sled::open(temp.path()).expect("database");
    let store = AgentStore::new(db.clone()).expect("agent store");
    let subscription = register_operational_agent(&db, &store, "agent-a", 1);
    install_belief(&db, 0.95, "revision-7", 7);
    let belief_store = BeliefStore::new(db.clone()).expect("belief store");
    let traversal_store = Arc::new(TraversalStore::new(db.clone()).expect("traversal store"));
    let belief_query = BeliefQuery::new(&belief_store);
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(&belief_store),
        TraversalQuery::new(&traversal_store),
    );
    let selector = AgentSemanticSelector::new(&store);
    let selection = selector
        .select_satisfaction_reviews(&belief_query, BELIEF_REVISION_REVIEW_SOURCE, 1)
        .expect("select satisfaction review")
        .pop()
        .expect("pending satisfaction review");
    let mut goal_query = |_agent_id: &str| {
        Ok::<ActiveGoalSummary, AgentActiveGoalQueryError>(ActiveGoalSummary::default())
    };
    let mut outcome_query = |_decision: &AgentCurationDecision, _command: &AgentAuthoredCommand| {
        Err::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(
            AgentCommandOutcomeQueryError::fatal("command outcome must not be queried"),
        )
    };
    let mut sink = |_command: &AgentGoalMutationCommand| {
        Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::fatal(
            "mutation sink must not be called",
        ))
    };

    let report = AgentSatisfactionCurationRuntime::new(&store).handle_selected_review(
        selection.clone(),
        &belief_query,
        &planner_query,
        &mut goal_query,
        &mut outcome_query,
        &mut sink,
    );

    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    assert_eq!(report.output_sequence, 7);
    assert_eq!(
        AgentQuery::new(&store)
            .satisfaction_review_cursor(&selection.cursor_identity)
            .expect("satisfaction cursor")
            .expect("cursor record")
            .last_reviewed_seq,
        7
    );
    assert_eq!(
        store
            .get_subscription(&subscription.subscription_id)
            .expect("subscription")
            .expect("subscription record")
            .last_delivered_seq,
        0
    );
    assert!(selector
        .select_satisfaction_reviews(&belief_query, BELIEF_REVISION_REVIEW_SOURCE, 1)
        .expect("satisfaction replay selection")
        .is_empty());
    assert_eq!(
        selector
            .select_deliveries(&belief_query, 1)
            .expect("delivery remains pending")
            .len(),
        1
    );

    let delivery_selection = selector
        .select_deliveries(&belief_query, 1)
        .expect("select independent delivery")
        .pop()
        .expect("pending independent delivery");
    let mut delivery_goal_query = |_agent_id: &str| {
        Ok::<ActiveGoalSummary, AgentActiveGoalQueryError>(ActiveGoalSummary::default())
    };
    let mut delivery_outcome_query =
        |_decision: &AgentCurationDecision, _command: &AgentAuthoredCommand| {
            Err::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(
                AgentCommandOutcomeQueryError::fatal("absorbed delivery has no command"),
            )
        };
    let mut delivery_sink = |_command: &AgentGoalCommand| {
        Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::fatal(
            "absorbed delivery has no sink work",
        ))
    };
    let delivery_report = AgentGoalCurationRuntime::new(&store).handle_selected_delivery(
        AgentSelectedGoalTick {
            selection: delivery_selection,
            belief_query: &belief_query,
            planner_query: &planner_query,
            rule_config: rule_config(),
        },
        &mut delivery_goal_query,
        &mut delivery_outcome_query,
        &mut delivery_sink,
    );

    assert!(delivery_report.fatal_errors.is_empty());
    assert_eq!(delivery_report.output_sequence, 7);
    assert_eq!(
        AgentQuery::new(&store)
            .recent_decisions("agent-a", 10)
            .expect("independent decisions")
            .len(),
        2
    );
    db.open_tree("agent_runtime_schema")
        .expect("runtime schema")
        .remove(b"semantic_runtime")
        .expect("remove schema marker for migration replay");
    db.open_tree("agent_runtime_schema")
        .expect("runtime schema")
        .remove(b"goal_command_sequence_migration")
        .expect("remove later migration checkpoint");
    store.flush().expect("flush independent decisions");
    drop(belief_store);
    drop(traversal_store);
    drop(store);
    drop(db);

    let reopened = AgentStore::new(sled::open(temp.path()).expect("reopen database"))
        .expect("reopen independent decision indexes");
    assert_eq!(
        AgentQuery::new(&reopened)
            .recent_decisions("agent-a", 10)
            .expect("reopened independent decisions")
            .len(),
        2
    );
}

#[test]
fn selected_delivery_resumes_from_receipt_before_cursor_after_reopen() {
    let temp = tempfile::tempdir().expect("tempdir");
    let selection;
    {
        let db = sled::open(temp.path()).expect("database");
        let store = AgentStore::new(db.clone()).expect("agent store");
        register_operational_agent(&db, &store, "agent-a", 1);
        install_belief(&db, 0.2, "revision-7", 7);
        let belief_store = BeliefStore::new(db.clone()).expect("belief store");
        let traversal_store = Arc::new(TraversalStore::new(db).expect("traversal store"));
        let belief_query = BeliefQuery::new(&belief_store);
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(&belief_store),
            TraversalQuery::new(&traversal_store),
        );
        selection = AgentSemanticSelector::new(&store)
            .select_deliveries(&belief_query, 1)
            .expect("select delivery")
            .pop()
            .expect("pending delivery");
        let input = AgentCuration::new(&store)
            .assemble_input(
                &selection.delivery,
                &belief_query,
                &planner_query,
                ActiveGoalSummary::default(),
                rule_config(),
            )
            .expect("assemble curation input");
        let outcome = curate_threshold_rule(input).expect("curate delivery");
        let mut invalid_matrix = outcome.clone();
        invalid_matrix.decision.goal_command_id = None;
        assert!(store
            .put_selected_delivery_outcome(&selection, &invalid_matrix)
            .is_err());
        let mut divergent = outcome.clone();
        let divergent_command = divergent.goal_command.as_mut().expect("goal command");
        divergent_command.goal.agent_id = "agent-b".to_string();
        divergent_command.dedupe_key.agent_id = "agent-b".to_string();
        assert!(store
            .put_selected_delivery_outcome(&selection, &divergent)
            .is_err());
        let outcome = store
            .put_selected_delivery_outcome(&selection, &outcome)
            .expect("persist selected outcome");
        let command = outcome.goal_command.as_ref().expect("goal command");
        let receipt = AgentSinkReceipt::new(
            &outcome.decision,
            AgentSinkReceiptKind::GoalCommand,
            AgentSinkSubmission::new(&command.command_id, &command.goal.goal_id, "applied"),
        );
        let mut wrong_kind = receipt.clone();
        wrong_kind.kind = AgentSinkReceiptKind::GoalMutationCommand;
        assert!(store.put_sink_receipt(&wrong_kind).is_err());
        store
            .put_sink_receipt(&receipt)
            .expect("persist receipt before cursor");
        store.flush().expect("flush receipt");
        assert_eq!(
            store
                .get_subscription(&selection.delivery.subscription_id)
                .expect("subscription")
                .expect("subscription record")
                .last_delivered_seq,
            0
        );
    }

    let db = sled::open(temp.path()).expect("reopen database");
    let store = AgentStore::new(db.clone()).expect("reopen agent store");
    let belief_store = BeliefStore::new(db.clone()).expect("reopen belief store");
    let traversal_store = Arc::new(TraversalStore::new(db).expect("reopen traversal store"));
    let belief_query = BeliefQuery::new(&belief_store);
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(&belief_store),
        TraversalQuery::new(&traversal_store),
    );
    let mut goal_query = |_agent_id: &str| {
        Err::<ActiveGoalSummary, AgentActiveGoalQueryError>(AgentActiveGoalQueryError::fatal(
            "receipt replay must bypass active goals",
        ))
    };
    let mut outcome_query = |_decision: &AgentCurationDecision, _command: &AgentAuthoredCommand| {
        Err::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(
            AgentCommandOutcomeQueryError::fatal("receipt must bypass outcome query"),
        )
    };
    let sink_called = Cell::new(false);
    let mut sink = |_command: &AgentGoalCommand| {
        sink_called.set(true);
        Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::fatal(
            "receipt must bypass sink",
        ))
    };

    let report = AgentGoalCurationRuntime::new(&store).handle_selected_delivery(
        AgentSelectedGoalTick {
            selection,
            belief_query: &belief_query,
            planner_query: &planner_query,
            rule_config: rule_config(),
        },
        &mut goal_query,
        &mut outcome_query,
        &mut sink,
    );

    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    assert_eq!(report.sink_submission_count, 0);
    assert!(!sink_called.get());
    assert_eq!(report.output_sequence, 7);
}

#[test]
fn selected_command_outcome_rejects_registered_suspended_and_changed_owners() {
    for owner_change in ["registered", "suspended", "sequence"] {
        let temp = tempfile::tempdir().expect("tempdir");
        let db = sled::open(temp.path()).expect("database");
        let store = AgentStore::new(db.clone()).expect("agent store");
        register_operational_agent(&db, &store, "agent-a", 1);
        install_belief(&db, 0.2, "revision-7", 7);
        let belief_store = BeliefStore::new(db.clone()).expect("belief store");
        let traversal_store = Arc::new(TraversalStore::new(db.clone()).expect("traversal store"));
        let belief_query = BeliefQuery::new(&belief_store);
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(&belief_store),
            TraversalQuery::new(&traversal_store),
        );
        let selection = AgentSemanticSelector::new(&store)
            .select_deliveries(&belief_query, 1)
            .expect("select delivery")
            .pop()
            .expect("pending delivery");
        let mut agent = store
            .get_agent("agent-a")
            .expect("agent query")
            .expect("agent record");
        agent.status = match owner_change {
            "registered" => AgentStatus::Registered,
            "suspended" => AgentStatus::Suspended,
            "sequence" => AgentStatus::Operational,
            _ => unreachable!("owner change"),
        };
        agent.updated_at_seq += 1;
        db.open_tree("agent_records")
            .expect("agent records")
            .insert(
                agent.agent_id.as_bytes(),
                serde_json::to_vec(&agent).expect("encode changed agent"),
            )
            .expect("change selected agent");

        let goal_query_called = Cell::new(false);
        let mut goal_query = |_agent_id: &str| {
            goal_query_called.set(true);
            Ok::<ActiveGoalSummary, AgentActiveGoalQueryError>(ActiveGoalSummary::default())
        };
        let outcome_query_called = Cell::new(false);
        let mut outcome_query = |_decision: &AgentCurationDecision,
                                 _command: &AgentAuthoredCommand| {
            outcome_query_called.set(true);
            Ok::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(None)
        };
        let sink_called = Cell::new(false);
        let mut sink = |_command: &AgentGoalCommand| {
            sink_called.set(true);
            Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::fatal(
                "stale owner reached sink",
            ))
        };

        let report = AgentGoalCurationRuntime::new(&store).handle_selected_delivery(
            AgentSelectedGoalTick {
                selection,
                belief_query: &belief_query,
                planner_query: &planner_query,
                rule_config: rule_config(),
            },
            &mut goal_query,
            &mut outcome_query,
            &mut sink,
        );

        assert_eq!(report.decision_count, 0, "{owner_change}");
        assert_eq!(report.retryable_errors.len(), 1, "{owner_change}");
        assert!(
            report.retryable_errors[0].contains("lifecycle changed"),
            "{owner_change}"
        );
        assert!(!goal_query_called.get(), "{owner_change}");
        assert!(!outcome_query_called.get(), "{owner_change}");
        assert!(!sink_called.get(), "{owner_change}");
        assert!(
            AgentQuery::new(&store)
                .recent_decisions("agent-a", 1)
                .expect("decisions")
                .is_empty(),
            "{owner_change}"
        );
    }
}

#[test]
fn selected_command_outcome_rechecks_owner_after_cross_domain_goal_query() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db = sled::open(temp.path()).expect("database");
    let store = AgentStore::new(db.clone()).expect("agent store");
    register_operational_agent(&db, &store, "agent-a", 1);
    install_belief(&db, 0.2, "revision-7", 7);
    let belief_store = BeliefStore::new(db.clone()).expect("belief store");
    let traversal_store = Arc::new(TraversalStore::new(db.clone()).expect("traversal store"));
    let belief_query = BeliefQuery::new(&belief_store);
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(&belief_store),
        TraversalQuery::new(&traversal_store),
    );
    let selection = AgentSemanticSelector::new(&store)
        .select_deliveries(&belief_query, 1)
        .expect("select delivery")
        .pop()
        .expect("pending delivery");
    let mut changed_agent = store
        .get_agent("agent-a")
        .expect("agent query")
        .expect("agent record");
    changed_agent.updated_at_seq += 1;
    let agent_records = db.open_tree("agent_records").expect("agent records");
    let mut goal_query = move |_agent_id: &str| {
        agent_records
            .insert(
                changed_agent.agent_id.as_bytes(),
                serde_json::to_vec(&changed_agent).expect("encode changed agent"),
            )
            .expect("change agent during goal query");
        Ok::<ActiveGoalSummary, AgentActiveGoalQueryError>(ActiveGoalSummary::default())
    };
    let outcome_query_called = Cell::new(false);
    let mut outcome_query = |_decision: &AgentCurationDecision, _command: &AgentAuthoredCommand| {
        outcome_query_called.set(true);
        Ok::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(None)
    };
    let sink_called = Cell::new(false);
    let mut sink = |_command: &AgentGoalCommand| {
        sink_called.set(true);
        Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::fatal(
            "changed owner reached sink",
        ))
    };

    let report = AgentGoalCurationRuntime::new(&store).handle_selected_delivery(
        AgentSelectedGoalTick {
            selection,
            belief_query: &belief_query,
            planner_query: &planner_query,
            rule_config: rule_config(),
        },
        &mut goal_query,
        &mut outcome_query,
        &mut sink,
    );

    assert_eq!(report.delivered_count, 1);
    assert_eq!(report.decision_count, 0);
    assert_eq!(report.retryable_errors.len(), 1);
    assert!(report.retryable_errors[0].contains("lifecycle changed"));
    assert!(!outcome_query_called.get());
    assert!(!sink_called.get());
    assert!(AgentQuery::new(&store)
        .recent_decisions("agent-a", 1)
        .expect("decisions")
        .is_empty());
}

#[test]
fn semantic_enablement_rejects_legacy_command_without_exact_outbox() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db = sled::open(temp.path()).expect("database");
    let store = AgentStore::new(db.clone()).expect("agent store");
    register_operational_agent(&db, &store, "agent-a", 1);
    install_belief(&db, 0.2, "revision-7", 7);
    let belief_store = BeliefStore::new(db.clone()).expect("belief store");
    let traversal_store = Arc::new(TraversalStore::new(db).expect("traversal store"));
    let belief_query = BeliefQuery::new(&belief_store);
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(&belief_store),
        TraversalQuery::new(&traversal_store),
    );
    let selection = AgentSemanticSelector::new(&store)
        .select_deliveries(&belief_query, 1)
        .expect("select delivery")
        .pop()
        .expect("pending delivery");
    let input = AgentCuration::new(&store)
        .assemble_input(
            &selection.delivery,
            &belief_query,
            &planner_query,
            ActiveGoalSummary::default(),
            rule_config(),
        )
        .expect("assemble curation input");
    let outcome = curate_threshold_rule(input).expect("curate delivery");
    store
        .put_decision(&outcome.decision)
        .expect("persist legacy decision");

    let error = AgentQuery::new(&store)
        .semantic_enablement_audit()
        .expect_err("legacy command decision must fail closed");

    assert!(error.to_string().contains("no exact durable outbox"));
}

#[test]
fn mutation_outcome_requires_exact_satisfaction_review_identity() {
    let temp = tempfile::tempdir().expect("tempdir");
    {
        let db = sled::open(temp.path()).expect("database");
        let store = AgentStore::new(db.clone()).expect("agent store");
        let (belief_store, traversal_store, selection, goal) =
            prepare_mutation_fixture(&db, &store);
        let belief_query = BeliefQuery::new(&belief_store);
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(&belief_store),
            TraversalQuery::new(&traversal_store),
        );
        let input = AgentCuration::new(&store)
            .assemble_satisfaction_input(
                &selection.review,
                &belief_query,
                &planner_query,
                ActiveGoalSummary { goals: vec![goal] },
            )
            .expect("assemble satisfaction mutation");
        let outcome = curate_goal_satisfaction(input).expect("curate satisfaction mutation");

        let error = store
            .put_curation_outcome(&outcome)
            .expect_err("generic mutation persistence must fail closed");
        assert!(error.to_string().contains("selected satisfaction review"));
        assert!(store
            .get_decision(&outcome.decision.decision_id)
            .expect("decision query")
            .is_none());

        store
            .put_selected_satisfaction_outcome(&selection, &outcome)
            .expect("persist selected satisfaction outcome");
        db.open_tree("agent_satisfaction_decisions_by_review")
            .expect("satisfaction review index")
            .remove(selection.review.index_key().as_bytes())
            .expect("remove satisfaction identity");
        let audit_error = AgentQuery::new(&store)
            .semantic_enablement_audit()
            .expect_err("mutation without review identity must not be enableable");
        assert!(audit_error
            .to_string()
            .contains("no exact satisfaction review identity"));
        db.open_tree("agent_runtime_schema")
            .expect("runtime schema")
            .remove(b"semantic_runtime")
            .expect("remove schema marker");
        db.open_tree("agent_runtime_schema")
            .expect("runtime schema")
            .remove(b"goal_command_sequence_migration")
            .expect("remove later migration checkpoint");
        store.flush().expect("flush missing review identity");
    }

    let reopened = AgentStore::new(sled::open(temp.path()).expect("reopen database"));
    assert!(reopened.is_err());
    let reopen_error = reopened.err().expect("reopen migration error");
    assert!(reopen_error
        .to_string()
        .contains("no exact satisfaction review identity"));
}

#[test]
fn selected_mutation_rechecks_owner_after_cross_domain_goal_query() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db = sled::open(temp.path()).expect("database");
    let store = AgentStore::new(db.clone()).expect("agent store");
    let (belief_store, traversal_store, selection, goal) = prepare_mutation_fixture(&db, &store);
    let belief_query = BeliefQuery::new(&belief_store);
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(&belief_store),
        TraversalQuery::new(&traversal_store),
    );
    let mut changed_agent = store
        .get_agent("agent-a")
        .expect("agent query")
        .expect("agent record");
    changed_agent.updated_at_seq += 1;
    let agent_records = db.open_tree("agent_records").expect("agent records");
    let mut goal_query = move |_agent_id: &str| {
        agent_records
            .insert(
                changed_agent.agent_id.as_bytes(),
                serde_json::to_vec(&changed_agent).expect("encode changed agent"),
            )
            .expect("change agent during goal query");
        Ok::<ActiveGoalSummary, AgentActiveGoalQueryError>(ActiveGoalSummary {
            goals: vec![goal.clone()],
        })
    };
    let outcome_query_called = Cell::new(false);
    let mut outcome_query = |_decision: &AgentCurationDecision, _command: &AgentAuthoredCommand| {
        outcome_query_called.set(true);
        Ok::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(None)
    };
    let sink_called = Cell::new(false);
    let mut sink = |_command: &AgentGoalMutationCommand| {
        sink_called.set(true);
        Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::fatal(
            "changed owner reached mutation sink",
        ))
    };

    let report = AgentSatisfactionCurationRuntime::new(&store).handle_selected_review(
        selection.clone(),
        &belief_query,
        &planner_query,
        &mut goal_query,
        &mut outcome_query,
        &mut sink,
    );

    assert_eq!(report.delivered_count, 1);
    assert_eq!(report.decision_count, 0);
    assert_eq!(report.retryable_errors.len(), 1);
    assert!(report.retryable_errors[0].contains("lifecycle changed"));
    assert!(!outcome_query_called.get());
    assert!(!sink_called.get());
    assert!(AgentQuery::new(&store)
        .decision_by_satisfaction_review(&selection.review)
        .expect("decision query")
        .is_none());
    assert!(AgentQuery::new(&store)
        .satisfaction_review_cursor(&selection.cursor_identity)
        .expect("cursor query")
        .is_none());
}

#[test]
fn selected_mutation_recovers_execution_outcome_after_reopen() {
    let temp = tempfile::tempdir().expect("tempdir");
    let selection;
    {
        let db = sled::open(temp.path()).expect("database");
        let store = AgentStore::new(db.clone()).expect("agent store");
        let (belief_store, traversal_store, selected, goal) = prepare_mutation_fixture(&db, &store);
        selection = selected;
        let belief_query = BeliefQuery::new(&belief_store);
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(&belief_store),
            TraversalQuery::new(&traversal_store),
        );
        let mut goal_query = |_agent_id: &str| {
            Ok::<ActiveGoalSummary, AgentActiveGoalQueryError>(ActiveGoalSummary {
                goals: vec![goal.clone()],
            })
        };
        let mut outcome_query = |_decision: &AgentCurationDecision,
                                 _command: &AgentAuthoredCommand| {
            Ok::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(None)
        };
        let mut sink = |_command: &AgentGoalMutationCommand| {
            Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::retryable(
                "mutation execution result was lost",
            ))
        };

        let report = AgentSatisfactionCurationRuntime::new(&store).handle_selected_review(
            selection.clone(),
            &belief_query,
            &planner_query,
            &mut goal_query,
            &mut outcome_query,
            &mut sink,
        );

        assert_eq!(report.decision_count, 1);
        assert_eq!(report.sink_submission_count, 1);
        assert_eq!(
            report.retryable_errors,
            vec!["mutation execution result was lost"]
        );
        assert!(AgentQuery::new(&store)
            .satisfaction_review_cursor(&selection.cursor_identity)
            .expect("satisfaction cursor")
            .is_none());
        store.flush().expect("flush mutation decision");
    }

    let db = sled::open(temp.path()).expect("reopen database");
    let store = AgentStore::new(db.clone()).expect("reopen agent store");
    let belief_store = BeliefStore::new(db.clone()).expect("reopen belief store");
    let traversal_store = Arc::new(TraversalStore::new(db).expect("reopen traversal store"));
    let belief_query = BeliefQuery::new(&belief_store);
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(&belief_store),
        TraversalQuery::new(&traversal_store),
    );
    let mut goal_query = |_agent_id: &str| {
        Err::<ActiveGoalSummary, AgentActiveGoalQueryError>(AgentActiveGoalQueryError::fatal(
            "durable mutation replay must bypass active goals",
        ))
    };
    let mut outcome_query = |_decision: &AgentCurationDecision, command: &AgentAuthoredCommand| {
        Ok::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(Some(
            AgentSinkSubmission::new(command.command_id(), command.goal_id(), "recovered"),
        ))
    };
    let sink_called = Cell::new(false);
    let mut sink = |_command: &AgentGoalMutationCommand| {
        sink_called.set(true);
        Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::fatal(
            "mutation sink must not run after outcome recovery",
        ))
    };

    let report = AgentSatisfactionCurationRuntime::new(&store).handle_selected_review(
        selection.clone(),
        &belief_query,
        &planner_query,
        &mut goal_query,
        &mut outcome_query,
        &mut sink,
    );

    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    assert_eq!(report.sink_submission_count, 0);
    assert!(!sink_called.get());
    assert_eq!(report.output_sequence, 8);
    assert_eq!(report.sink_receipts.len(), 1);
}

#[test]
fn selected_mutation_resumes_from_receipt_before_cursor() {
    let temp = tempfile::tempdir().expect("tempdir");
    let selection;
    {
        let db = sled::open(temp.path()).expect("database");
        let store = AgentStore::new(db.clone()).expect("agent store");
        let (belief_store, traversal_store, selected, goal) = prepare_mutation_fixture(&db, &store);
        selection = selected;
        let belief_query = BeliefQuery::new(&belief_store);
        let planner_query = PlannerQuery::new(
            BeliefQuery::new(&belief_store),
            TraversalQuery::new(&traversal_store),
        );
        let input = AgentCuration::new(&store)
            .assemble_satisfaction_input(
                &selection.review,
                &belief_query,
                &planner_query,
                ActiveGoalSummary { goals: vec![goal] },
            )
            .expect("assemble satisfaction mutation");
        let outcome = curate_goal_satisfaction(input).expect("curate satisfaction mutation");
        let outcome = store
            .put_selected_satisfaction_outcome(&selection, &outcome)
            .expect("persist selected mutation");
        let command = outcome
            .goal_mutation_command
            .as_ref()
            .expect("mutation command");
        store
            .put_sink_receipt(&AgentSinkReceipt::new(
                &outcome.decision,
                AgentSinkReceiptKind::GoalMutationCommand,
                AgentSinkSubmission::new(&command.command_id, &command.goal_id, "applied"),
            ))
            .expect("persist mutation receipt");
        assert!(AgentQuery::new(&store)
            .satisfaction_review_cursor(&selection.cursor_identity)
            .expect("satisfaction cursor")
            .is_none());
        store.flush().expect("flush mutation receipt");
    }

    let db = sled::open(temp.path()).expect("reopen database");
    let store = AgentStore::new(db.clone()).expect("reopen agent store");
    let belief_store = BeliefStore::new(db.clone()).expect("reopen belief store");
    let traversal_store = Arc::new(TraversalStore::new(db).expect("reopen traversal store"));
    let belief_query = BeliefQuery::new(&belief_store);
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(&belief_store),
        TraversalQuery::new(&traversal_store),
    );
    let mut goal_query = |_agent_id: &str| {
        Err::<ActiveGoalSummary, AgentActiveGoalQueryError>(AgentActiveGoalQueryError::fatal(
            "mutation receipt replay must bypass active goals",
        ))
    };
    let mut outcome_query = |_decision: &AgentCurationDecision, _command: &AgentAuthoredCommand| {
        Err::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(
            AgentCommandOutcomeQueryError::fatal("mutation receipt must bypass outcome query"),
        )
    };
    let mut sink = |_command: &AgentGoalMutationCommand| {
        Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::fatal(
            "mutation receipt must bypass sink",
        ))
    };

    let report = AgentSatisfactionCurationRuntime::new(&store).handle_selected_review(
        selection,
        &belief_query,
        &planner_query,
        &mut goal_query,
        &mut outcome_query,
        &mut sink,
    );

    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    assert_eq!(report.output_sequence, 8);
    assert_eq!(report.sink_submission_count, 0);
}

#[test]
fn semantic_schema_migration_rejects_orphaned_indexes() {
    let temp = tempfile::tempdir().expect("tempdir");
    {
        let db = sled::open(temp.path()).expect("database");
        let store = AgentStore::new(db.clone()).expect("agent store");
        db.open_tree("agent_decisions_by_agent")
            .expect("decision agent index")
            .insert(b"orphaned-index", b"missing-decision")
            .expect("write orphaned index");
        db.open_tree("agent_runtime_schema")
            .expect("runtime schema")
            .remove(b"semantic_runtime")
            .expect("remove schema marker");
        db.open_tree("agent_runtime_schema")
            .expect("runtime schema")
            .remove(b"goal_command_sequence_migration")
            .expect("remove later migration checkpoint");
        store.flush().expect("flush corrupt migration fixture");
    }

    let result = AgentStore::new(reopen_database(temp.path()));
    assert!(result.is_err());
    assert!(result
        .err()
        .expect("migration error")
        .to_string()
        .contains("missing decision"));
}

#[test]
fn semantic_schema_migration_rejects_noncanonical_dedupe_alias() {
    let temp = tempfile::tempdir().expect("tempdir");
    {
        let db = sled::open(temp.path()).expect("database");
        let store = AgentStore::new(db.clone()).expect("agent store");
        let decision = persist_goal_command_decision(&db, &store);
        db.open_tree("agent_decisions_by_dedupe")
            .expect("decision dedupe index")
            .insert(
                format!("{}::wrong-revision", decision.dedupe_key.index_key()).as_bytes(),
                decision.decision_id.as_bytes(),
            )
            .expect("write noncanonical dedupe alias");
        db.open_tree("agent_runtime_schema")
            .expect("runtime schema")
            .remove(b"semantic_runtime")
            .expect("remove schema marker");
        db.open_tree("agent_runtime_schema")
            .expect("runtime schema")
            .remove(b"goal_command_sequence_migration")
            .expect("remove later migration checkpoint");
        store.flush().expect("flush corrupt migration fixture");
    }

    let result = AgentStore::new(reopen_database(temp.path()));
    assert!(result.is_err());
    assert!(result
        .err()
        .expect("migration error")
        .to_string()
        .contains("dedupe index diverges"));
}

#[test]
fn semantic_schema_migration_rejects_goal_decision_satisfaction_alias() {
    let temp = tempfile::tempdir().expect("tempdir");
    {
        let db = sled::open(temp.path()).expect("database");
        let store = AgentStore::new(db.clone()).expect("agent store");
        let decision = persist_goal_command_decision(&db, &store);
        db.open_tree("agent_satisfaction_decisions_by_review")
            .expect("satisfaction review index")
            .insert(
                b"agent-a::subscription-alias::00000000000000000007",
                decision.decision_id.as_bytes(),
            )
            .expect("write invalid satisfaction alias");
        db.open_tree("agent_runtime_schema")
            .expect("runtime schema")
            .remove(b"semantic_runtime")
            .expect("remove schema marker");
        db.open_tree("agent_runtime_schema")
            .expect("runtime schema")
            .remove(b"goal_command_sequence_migration")
            .expect("remove later migration checkpoint");
        store.flush().expect("flush corrupt migration fixture");
    }

    let result = AgentStore::new(reopen_database(temp.path()));
    assert!(result.is_err());
    assert!(result
        .err()
        .expect("migration error")
        .to_string()
        .contains("diverges"));
}
