use std::sync::Arc;

use crate::integration::outcome_evidence_support::{
    build_docs_task_success_evidence, DocsTaskSuccessEvidenceRequest,
};
use meld::execution::{
    satisfy_request_from_agent_mutation, GoalMutationError, GoalMutationRequest,
};
use meld_events::events::test_support::{EventStore, EventStoreTestSupport as _};
use meld_events::{DomainObjectRef, EventRecord};
use meld_execution::goals::{
    GoalAcceptanceLifecycle, GoalAcceptanceRequest, GoalCommandMetadata, GoalCommandOutcome,
    GoalSetApi, PersistentGoalSetStore,
};
use meld_execution::planning::{
    PlanningRequest, PlanningResult, PlanningRuntime, PlanningWorldStateFrameRef,
    PlanningWorldStateRequest,
};
use meld_execution::task_network::dispatch::{Outcome, OutcomeStatus};
use meld_execution::task_network::outcome::Publication;
use meld_execution::task_network::publication::build_publication_envelope;
use meld_lang::{Condition, GoalLifecycle, Literal, Proposition, Term, WorldState};
use meld_world_model::agent::{
    curate_goal_satisfaction, curate_threshold_rule, ActiveGoalSummary, AgentCuration,
    AgentCurationInput, AgentCurationInputRefs, AgentCurationRuleConfig, AgentDecisionKind,
    AgentGoalMutationCommand, AgentGoalSatisfactionInput, AgentQuery, AgentRegistration,
    AgentSatisfactionReview, AgentStore, AgentSubscription, AgentSubscriptionRecord,
    SeedAgentRegistration, SubscribeAgentCommand,
};
use meld_world_model::belief::{
    BeliefProvenanceSummary, BeliefQuery, BeliefStore, ContradictionState, FreshnessState,
    HydrationRefs, PlannerProjectionSummary, PosteriorSummary,
};
use meld_world_model::planner::{
    project_world_state, PlannerFieldProjectionConfig, PlannerProjectionContext,
    PlannerProjectionInput, PlannerQuery, PLANNER_PROJECTION_VERSION,
};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::{AgentGoalCommand, BeliefKey, BeliefStatus, BeliefView, TraversalQuery};

use super::docs_freshness_fixture::{
    DocsFreshnessFirstProofFixture, CONTENT_SOURCE_KIND, DIMENSION_ID, REQUIRED_ARTIFACT_TYPE_ID,
    TASK_NETWORK_ID, THRESHOLD,
};

fn subject() -> DomainObjectRef {
    DocsFreshnessFirstProofFixture::new().subject()
}

fn subject_term() -> Term {
    DocsFreshnessFirstProofFixture::new().subject_term()
}

fn agent_store() -> (tempfile::TempDir, AgentStore) {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = AgentStore::new(sled::open(temp_dir.path().join("agent")).unwrap()).unwrap();
    (temp_dir, store)
}

fn seed_agent_registration() -> SeedAgentRegistration {
    DocsFreshnessFirstProofFixture::new().seed_agent_registration()
}

fn belief_key() -> BeliefKey {
    DocsFreshnessFirstProofFixture::new().belief_key()
}

fn rule_config() -> AgentCurationRuleConfig {
    DocsFreshnessFirstProofFixture::new().curation_rule_config()
}

fn test_view(confidence: f64, revision_id: &str, seq: u64) -> BeliefView {
    let key = belief_key();
    BeliefView {
        view_id: format!("view-{revision_id}"),
        key,
        current_revision_id: Some(revision_id.to_string()),
        status: BeliefStatus::Settled,
        posterior: PosteriorSummary {
            probability: confidence,
            meaning: "probability".to_string(),
        },
        planner_projection: PlannerProjectionSummary {
            confidence_field: "confidence".to_string(),
            confidence,
            threshold: THRESHOLD,
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
        observation: None,
        assessment_state: "settled".to_string(),
        advisory_posture: "ready".to_string(),
        provenance: BeliefProvenanceSummary {
            evidence_ids: vec![format!("evidence-{revision_id}")],
            source_fact_ids: vec![format!("ledger-{seq}")],
            graph_anchor_ids: vec!["anchor-a".to_string()],
            objects: vec![subject()],
            relations: Vec::new(),
            revision_ids: vec![revision_id.to_string()],
        },
        hydration: HydrationRefs {
            evidence_ids: vec![format!("evidence-{revision_id}")],
            source_fact_ids: vec![format!("ledger-{seq}")],
            graph_anchor_ids: vec!["anchor-a".to_string()],
            revision_id: Some(revision_id.to_string()),
        },
        theory_revision: None,
    }
}

fn setup_agent(
    store: &AgentStore,
) -> (
    meld_world_model::AgentRecord,
    meld_world_model::AgentSubscriptionRecord,
) {
    let registration = AgentRegistration::new(store);
    let agent = registration
        .register_seed_agent(seed_agent_registration())
        .unwrap();
    let subscription = AgentSubscription::new(store)
        .subscribe(SubscribeAgentCommand {
            agent_id: agent.agent_id.clone(),
            belief_key: belief_key(),
            created_at_seq: 1,
        })
        .unwrap();
    (agent, subscription)
}

fn seeded_graph() -> (tempfile::TempDir, Arc<TraversalStore>) {
    let (temp_dir, store, _subject) = DocsFreshnessFirstProofFixture::new().seeded_graph();
    (temp_dir, store)
}

fn curation_input(confidence: f64) -> AgentCurationInput {
    let (_temp_dir, store) = agent_store();
    let (agent, subscription) = setup_agent(&store);
    let view = test_view(confidence, "revision-a", 7);
    let projection = project_world_state(PlannerProjectionInput {
        context: PlannerProjectionContext {
            subject: agent.subject.clone(),
            perspective: agent.perspective_key.clone(),
            branch_scope: agent.branch_scope.clone(),
            projection_version: PLANNER_PROJECTION_VERSION.to_string(),
        },
        belief_view: Some(view.clone()),
        graph_scope: None,
        field_config: PlannerFieldProjectionConfig::default(),
    })
    .unwrap();
    AgentCurationInput {
        agent,
        subscription,
        delivered_seq: 7,
        rule_config: rule_config(),
        belief_view: Some(view),
        planner_projection: projection,
        active_goals: ActiveGoalSummary::default(),
        input_refs: AgentCurationInputRefs {
            belief_revision_id: Some("revision-a".to_string()),
            belief_key: belief_key(),
            planner_projection_version: PLANNER_PROJECTION_VERSION.to_string(),
            planner_source_refs: Vec::new(),
            planner_warnings: Vec::new(),
        },
    }
}

fn low_confidence_goal_command() -> AgentGoalCommand {
    let outcome = curate_threshold_rule(curation_input(0.2)).unwrap();
    assert_eq!(outcome.decision.decision, AgentDecisionKind::GoalCommand);
    outcome.goal_command.unwrap()
}

fn acceptance_request_from_agent_command(
    command: AgentGoalCommand,
    accepted_at_seq: u64,
) -> GoalAcceptanceRequest {
    command.validate().unwrap();

    GoalAcceptanceRequest {
        metadata: GoalCommandMetadata {
            command_id: command.command_id,
            source_identity: Some(command.dedupe_key.index_key()),
            seq: accepted_at_seq,
        },
        goal: command.goal,
        lifecycle_policy: GoalAcceptanceLifecycle::RequireProposedThenActivate,
    }
}

fn planning_request(goal: meld_lang::Goal, world_state: WorldState) -> PlanningRequest {
    PlanningRequest {
        request_id: "request-1".to_string(),
        world_state_request: PlanningWorldStateRequest {
            goal_id: goal.goal_id.clone(),
            agent_id: goal.agent_id.clone(),
            target: goal.target.clone(),
            perspective_id: "default".to_string(),
            branch_id: "main".to_string(),
            requested_dimensions: vec![DIMENSION_ID.to_string()],
            required_preconditions: vec![],
        },
        goal,
        world_state,
        world_state_frame: PlanningWorldStateFrameRef {
            frame_id: "frame-1".to_string(),
            projection_version: PLANNER_PROJECTION_VERSION.to_string(),
            perspective_id: "default".to_string(),
            branch_id: "main".to_string(),
            source_refs: vec!["source".to_string()],
            warnings: vec![],
        },
    }
}

fn world_state_below_threshold() -> WorldState {
    WorldState::new(vec![
        Proposition::Accessible {
            scope: subject_term(),
        },
        Proposition::Holds {
            subject: subject_term(),
            dimension: Term::Dimension(DIMENSION_ID.to_string()),
            condition: Condition::Equals(Term::Literal(Literal::Number(0.2))),
        },
    ])
    .unwrap()
}

fn world_state_with_confidence(confidence: f64) -> WorldState {
    WorldState::new(vec![
        Proposition::Accessible {
            scope: subject_term(),
        },
        Proposition::Holds {
            subject: subject_term(),
            dimension: Term::Dimension(DIMENSION_ID.to_string()),
            condition: Condition::Equals(Term::Literal(Literal::Number(confidence))),
        },
    ])
    .unwrap()
}

fn satisfaction_input_for_goal(
    goal: meld_lang::Goal,
    world_state: WorldState,
    review_seq: u64,
) -> AgentGoalSatisfactionInput {
    let mut input = curation_input(0.95);
    input.planner_projection.world_state = world_state;
    input.input_refs.planner_source_refs = vec!["integration-source".to_string()];
    input.input_refs.planner_warnings = vec!["integration-warning".to_string()];
    AgentGoalSatisfactionInput {
        agent: input.agent,
        subscription: input.subscription,
        review_seq,
        planner_projection: input.planner_projection,
        active_goals: ActiveGoalSummary {
            goals: vec![goal],
            lifecycle_epochs: std::collections::BTreeMap::new(),
        },
        input_refs: input.input_refs,
    }
}

fn valid_goal_mutation_command() -> AgentGoalMutationCommand {
    let mut goal = low_confidence_goal_command().goal;
    goal.lifecycle = GoalLifecycle::Active;
    curate_goal_satisfaction(satisfaction_input_for_goal(
        goal,
        world_state_with_confidence(0.95),
        22,
    ))
    .unwrap()
    .goal_mutation_command
    .unwrap()
}

fn satisfaction_review(
    subscription: &AgentSubscriptionRecord,
    review_seq: u64,
) -> AgentSatisfactionReview {
    AgentSatisfactionReview {
        agent_id: subscription.agent_id.clone(),
        subscription_id: subscription.subscription_id.clone(),
        review_seq,
    }
}

fn planning_runtime() -> PlanningRuntime {
    DocsFreshnessFirstProofFixture::new().planning_runtime()
}

fn no_method_planning_runtime() -> PlanningRuntime {
    DocsFreshnessFirstProofFixture::new().empty_planning_runtime()
}

fn failed_task_event_record() -> EventRecord {
    let outcome = Outcome {
        outcome_id: "outcome-failure".to_string(),
        task_instance_id: "task-failure".to_string(),
        lifecycle_epoch: 1,
        claim_id: "claim-failure".to_string(),
        claim_revision: 1,
        status: OutcomeStatus::Failed,
        error: Some("runtime failed".to_string()),
        artifact_records: Vec::new(),
        task_events: Vec::new(),
    };
    let publication = Publication::pending_for_outcome(TASK_NETWORK_ID, &outcome);
    assert_eq!(publication.event_type(), "execution.task.failed");
    let envelope = build_publication_envelope("session-nag-5", &publication).unwrap();
    let event_dir = tempfile::tempdir().unwrap();
    let events = EventStore::new(sled::open(event_dir.path()).unwrap()).unwrap();
    let seq = events.append_envelope_idempotent(envelope).unwrap();
    let records = events.read_events("session-nag-5").unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].seq, seq);
    assert_eq!(records[0].envelope.event_type, "execution.task.failed");
    assert_eq!(
        records[0].envelope.data["error"],
        serde_json::json!("runtime failed")
    );
    records[0].clone()
}

#[test]
fn producer_neutral_goal_acceptance_stores_active_plannable_goal() {
    let goal_command = low_confidence_goal_command();
    assert_eq!(goal_command.goal.lifecycle, GoalLifecycle::Proposed);

    let temp = tempfile::tempdir().unwrap();
    let mut store = PersistentGoalSetStore::new(sled::open(temp.path()).unwrap()).unwrap();
    let acceptance_request = acceptance_request_from_agent_command(goal_command.clone(), 7);

    let outcome = GoalSetApi::new(&mut store)
        .accept_goal(acceptance_request.clone())
        .unwrap();
    let GoalCommandOutcome::Applied(record) = &outcome else {
        panic!("expected applied goal acceptance");
    };
    assert_eq!(record.goal.lifecycle, GoalLifecycle::Active);
    assert_eq!(
        record.source_command_id.as_deref(),
        Some(goal_command.command_id.as_str())
    );
    assert_eq!(
        record.source_identity.as_deref(),
        Some(goal_command.dedupe_key.index_key().as_str())
    );

    let replay = GoalSetApi::new(&mut store)
        .accept_goal(acceptance_request)
        .unwrap();
    assert_eq!(replay, outcome);

    let mut duplicate_command = goal_command.clone();
    duplicate_command.command_id = "goal-command-duplicate".to_string();
    let duplicate = GoalSetApi::new(&mut store)
        .accept_goal(acceptance_request_from_agent_command(duplicate_command, 8))
        .unwrap();
    assert!(matches!(
        duplicate,
        GoalCommandOutcome::Duplicate { existing_goal_id }
            if existing_goal_id == goal_command.goal.goal_id
    ));

    let active_goal = store
        .active_goal(&goal_command.goal.goal_id)
        .unwrap()
        .expect("active goal");
    let result = planning_runtime()
        .plan_goal(planning_request(active_goal, world_state_below_threshold()))
        .unwrap();
    assert!(matches!(result, PlanningResult::Composed(_)));
}

#[test]
fn agent_satisfaction_curation_marks_goal_satisfied_only_after_world_state_match() {
    let goal_command = low_confidence_goal_command();
    let goal_id = goal_command.goal.goal_id.clone();

    let temp = tempfile::tempdir().unwrap();
    let mut store = PersistentGoalSetStore::new(sled::open(temp.path()).unwrap()).unwrap();
    GoalSetApi::new(&mut store)
        .accept_goal(acceptance_request_from_agent_command(
            goal_command.clone(),
            7,
        ))
        .unwrap();
    let (_decision_temp, decision_store) = agent_store();
    let (_agent, subscription) = setup_agent(&decision_store);
    let belief_temp = tempfile::tempdir().unwrap();
    let belief_store =
        BeliefStore::new(sled::open(belief_temp.path().join("belief")).unwrap()).unwrap();
    let (_graph_temp, graph_store) = seeded_graph();
    let curation = AgentCuration::new(&decision_store);
    let belief_query = BeliefQuery::new(&belief_store);
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(&belief_store),
        TraversalQuery::new(&graph_store),
    );

    let active_goal = store.active_goal(&goal_id).unwrap().expect("active goal");
    assert_eq!(active_goal.lifecycle, GoalLifecycle::Active);

    belief_store
        .put_view(&test_view(0.2, "revision-below", 21))
        .unwrap();
    let below_review = satisfaction_review(&subscription, 21);
    let below = curation
        .handle_satisfaction_review(
            below_review.clone(),
            &belief_query,
            &planner_query,
            ActiveGoalSummary {
                goals: vec![active_goal.clone()],

                lifecycle_epochs: std::collections::BTreeMap::new(),
            },
        )
        .unwrap();
    assert!(below.goal_mutation_command.is_none());
    assert_eq!(
        AgentQuery::new(&decision_store)
            .decision_by_satisfaction_review(&below_review)
            .unwrap(),
        Some(below.decision)
    );
    assert!(store.active_goal(&goal_id).unwrap().is_some());

    belief_store
        .put_view(&test_view(0.95, "revision-satisfied", 22))
        .unwrap();
    let satisfied_review = satisfaction_review(&subscription, 22);
    let satisfied = curation
        .handle_satisfaction_review(
            satisfied_review.clone(),
            &belief_query,
            &planner_query,
            ActiveGoalSummary {
                goals: vec![active_goal.clone()],

                lifecycle_epochs: std::collections::BTreeMap::new(),
            },
        )
        .unwrap();
    assert_eq!(
        AgentQuery::new(&decision_store)
            .decision_by_satisfaction_review(&satisfied_review)
            .unwrap(),
        Some(satisfied.decision.clone())
    );
    let command = satisfied.goal_mutation_command.expect("mutation command");
    let satisfy = satisfy_request_from_agent_mutation(GoalMutationRequest { command }).unwrap();
    let first_outcome = GoalSetApi::new(&mut store)
        .satisfy_goal(satisfy.clone())
        .unwrap();

    let record = store.get_goal(&goal_id).unwrap().expect("goal record");
    assert_eq!(
        record.goal.lifecycle,
        GoalLifecycle::Satisfied { at_seq: 22 }
    );

    let replayed = curation
        .handle_satisfaction_review(
            satisfied_review,
            &belief_query,
            &planner_query,
            ActiveGoalSummary {
                goals: vec![active_goal],

                lifecycle_epochs: std::collections::BTreeMap::new(),
            },
        )
        .unwrap();
    let replay_command = replayed.goal_mutation_command.expect("mutation command");
    let replay_satisfy = satisfy_request_from_agent_mutation(GoalMutationRequest {
        command: replay_command,
    })
    .unwrap();
    let replay_outcome = GoalSetApi::new(&mut store)
        .satisfy_goal(replay_satisfy)
        .unwrap();
    assert_eq!(replay_outcome, first_outcome);
}

#[test]
fn failure_outcome_does_not_satisfy_goal() {
    let goal_command = low_confidence_goal_command();
    let goal_id = goal_command.goal.goal_id.clone();

    let temp = tempfile::tempdir().unwrap();
    let mut store = PersistentGoalSetStore::new(sled::open(temp.path()).unwrap()).unwrap();
    GoalSetApi::new(&mut store)
        .accept_goal(acceptance_request_from_agent_command(
            goal_command.clone(),
            7,
        ))
        .unwrap();

    let active_goal = store.active_goal(&goal_id).unwrap().expect("active goal");
    let planned = planning_runtime()
        .plan_goal(planning_request(
            active_goal.clone(),
            world_state_below_threshold(),
        ))
        .unwrap();
    assert!(matches!(planned, PlanningResult::Composed(_)));

    let failed_event = failed_task_event_record();
    let failure_evidence = build_docs_task_success_evidence(DocsTaskSuccessEvidenceRequest {
        event: failed_event.clone(),
        subject: subject(),
        stale_probability: 0.0,
        review_probability: 0.2,
        source_kind: CONTENT_SOURCE_KIND.to_string(),
        required_artifact_type_id: Some(REQUIRED_ARTIFACT_TYPE_ID.to_string()),
    })
    .unwrap();
    assert!(failure_evidence.is_none());

    let failure_review = curate_goal_satisfaction(satisfaction_input_for_goal(
        active_goal.clone(),
        world_state_below_threshold(),
        failed_event.seq,
    ))
    .unwrap();
    assert!(failure_review.goal_mutation_command.is_none());
    assert_eq!(
        failure_review.decision.decision,
        AgentDecisionKind::Absorbed
    );
    assert!(failure_review.decision.reason.contains("unsatisfied"));

    let stored_goal = store.active_goal(&goal_id).unwrap().expect("active goal");
    assert_eq!(stored_goal.lifecycle, GoalLifecycle::Active);

    let no_method = no_method_planning_runtime()
        .plan_goal(planning_request(
            stored_goal.clone(),
            world_state_below_threshold(),
        ))
        .unwrap();
    assert!(matches!(no_method, PlanningResult::NoApplicableMethod(_)));

    let indeterminate = planning_runtime()
        .plan_goal(planning_request(stored_goal, WorldState::empty()))
        .unwrap();
    assert!(matches!(indeterminate, PlanningResult::Indeterminate(_)));
}

#[test]
fn agent_satisfaction_curation_requires_agent_owned_goal() {
    let mut goal = low_confidence_goal_command().goal;
    goal.lifecycle = GoalLifecycle::Active;
    goal.agent_id = "other-agent".to_string();

    let outcome = curate_goal_satisfaction(satisfaction_input_for_goal(
        goal,
        world_state_with_confidence(0.95),
        22,
    ))
    .unwrap();

    assert!(outcome.goal_mutation_command.is_none());
    assert_eq!(outcome.decision.decision, AgentDecisionKind::Absorbed);
}

#[test]
fn agent_satisfaction_adapter_rejects_invalid_mutation() {
    let invalid_dedupe = invalid_mutation_case(|command| {
        command.dedupe_key.agent_id = "other-agent".to_string();
    });
    let invalid_projection = invalid_mutation_case(|command| {
        command.projection_version.clear();
    });

    for command in [invalid_dedupe, invalid_projection] {
        let error = satisfy_request_from_agent_mutation(GoalMutationRequest { command })
            .expect_err("invalid mutation");
        assert!(matches!(error, GoalMutationError::InvalidCommand(_)));
    }
}

#[test]
fn producer_neutral_goal_acceptance_rejects_invalid_agent_command_before_execution() {
    let valid = low_confidence_goal_command();
    let cases = [
        invalid_case(&valid, |command| command.command_id.clear()),
        invalid_case(&valid, |command| command.goal.agent_id.clear()),
        invalid_case(&valid, |command| {
            command.goal.target = Proposition::Accessible {
                scope: Term::Variable("?node".to_string()),
            };
        }),
        invalid_case(&valid, |command| {
            command.goal.lifecycle = GoalLifecycle::Active;
        }),
        invalid_case(&valid, |command| {
            command.goal.lifecycle = GoalLifecycle::Satisfied { at_seq: 9 };
        }),
        invalid_case(&valid, |command| {
            command.dedupe_key.agent_id = "other-agent".to_string();
        }),
        invalid_case(&valid, |command| {
            command.dedupe_key.subject_key = "workspace_fs::node::other-node".to_string();
        }),
        invalid_case(&valid, |command| {
            command.dedupe_key.dimension_id = "other_dimension".to_string();
        }),
        invalid_case(&valid, |command| {
            command.dedupe_key.target_condition_key = "other-condition".to_string();
        }),
    ];

    let temp = tempfile::tempdir().unwrap();
    let store = PersistentGoalSetStore::new(sled::open(temp.path()).unwrap()).unwrap();

    for goal_command in cases {
        assert!(goal_command.validate().is_err());
    }
    assert!(store.goal_records().unwrap().is_empty());
}

#[test]
fn producer_neutral_goal_acceptance_builds_stable_request_from_agent_command() {
    let goal_command = low_confidence_goal_command();
    let request = acceptance_request_from_agent_command(goal_command.clone(), 11);

    assert_eq!(
        request.metadata.command_id.as_str(),
        goal_command.command_id.as_str()
    );
    assert_eq!(
        request.metadata.source_identity.as_deref(),
        Some(goal_command.dedupe_key.index_key().as_str())
    );
    assert_eq!(request.metadata.seq, 11);
    assert_eq!(
        request.goal.goal_id.as_str(),
        goal_command.goal.goal_id.as_str()
    );
    assert_eq!(request.goal.lifecycle, GoalLifecycle::Proposed);
    assert_eq!(
        request.lifecycle_policy,
        GoalAcceptanceLifecycle::RequireProposedThenActivate
    );
}

fn invalid_case(
    valid: &AgentGoalCommand,
    mutate: impl FnOnce(&mut AgentGoalCommand),
) -> AgentGoalCommand {
    let mut command = valid.clone();
    mutate(&mut command);
    command
}

fn invalid_mutation_case(
    mutate: impl FnOnce(&mut AgentGoalMutationCommand),
) -> AgentGoalMutationCommand {
    let mut command = valid_goal_mutation_command();
    mutate(&mut command);
    command
}
