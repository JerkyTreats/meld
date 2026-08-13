//! Agent convergence proofs: bounded delivery eligibility, satisfaction
//! trigger eligibility with the no-hot-loop rule, the future-drift reopen
//! path, the curation-rule durable home, and legacy serde compatibility.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use meld_lang::{Goal, GoalLifecycle};
use meld_world_model::agent::{
    ActiveGoalSummary, AgentActiveGoalQueryError, AgentCurationRuleBinding,
    AgentCurationRuleConfig, AgentGoalCommand, AgentGoalCurationActor, AgentGoalMutationCommand,
    AgentGoalMutationKind, AgentRegistration, AgentSatisfactionCheckpoint,
    AgentSatisfactionCurationActor, AgentSinkError, AgentSinkSubmission, AgentStepRequest,
    AgentStore, AgentSubscription, AgentWorkSelector, CurationGoalSetPort, SeedAgentRegistration,
    SubscribeAgentCommand, CURATION_GOAL_SET_PORT_ID,
};
use meld_world_model::belief::{
    BeliefProvenanceSummary, BeliefQuery, BeliefRevision, BeliefStore, BranchScope,
    ContradictionState, FreshnessState, HydrationRefs, PlannerProjectionSummary, PosteriorSummary,
};
use meld_world_model::events::{DomainObjectRef, EventRelation};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::{
    AnchorSelectionRecord, AssessmentLease, BeliefKey, BeliefStatus, BeliefView, LeaseStatus,
    PerspectiveKey, TraversalFactRecord,
};

const AGENT_ID: &str = "seed.docs_freshness";
const DIMENSION_ID: &str = "docs_freshness";
const SECOND_DIMENSION_ID: &str = "docs_coverage";
const PREDICATE_ID: &str = "confidence";
const EVIDENCE_POLICY_ID: &str = "default_policy";
const THRESHOLD: f64 = 0.7;

fn reopen_sled_after_close(path: &Path) -> sled::Db {
    const MAX_ATTEMPTS: usize = 50;
    for attempt in 0..MAX_ATTEMPTS {
        match sled::open(path) {
            Ok(db) => return db,
            Err(error)
                if error.to_string().contains("could not acquire lock")
                    && attempt + 1 < MAX_ATTEMPTS =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("sled reopen failed: {error}"),
        }
    }
    unreachable!("the final open attempt returns its error")
}

fn object(domain_id: &str, object_kind: &str, object_id: &str) -> DomainObjectRef {
    DomainObjectRef::new(domain_id, object_kind, object_id).unwrap()
}

fn subject() -> DomainObjectRef {
    object("workspace_fs", "node", "node-a")
}

fn rule_config() -> AgentCurationRuleConfig {
    AgentCurationRuleConfig {
        maintained_condition_id: None,
        dimension_id: DIMENSION_ID.to_string(),
        threshold: THRESHOLD,
        priority_urgency: 50,
        desired_summary: "confidence>0.7".to_string(),
        source_kind: "belief_divergence".to_string(),
    }
}

fn belief_key_for(dimension_id: &str) -> BeliefKey {
    BeliefKey {
        subject: subject(),
        dimension_id: dimension_id.to_string(),
        predicate_id: PREDICATE_ID.to_string(),
        perspective: PerspectiveKey::new("default", "default").unwrap(),
        branch_scope: BranchScope::main(),
        evidence_policy_id: EVIDENCE_POLICY_ID.to_string(),
    }
}

fn belief_key() -> BeliefKey {
    belief_key_for(DIMENSION_ID)
}

fn registration(with_rule: bool) -> SeedAgentRegistration {
    SeedAgentRegistration {
        agent_id: AGENT_ID.to_string(),
        perspective_key: PerspectiveKey::new("default", "default").unwrap(),
        subject: subject(),
        branch_scope: BranchScope::main(),
        observation_scope: DIMENSION_ID.to_string(),
        directive: "curate docs freshness goals".to_string(),
        seed_provenance: "trusted init".to_string(),
        curation_rule: with_rule
            .then(|| AgentCurationRuleBinding::for_rule(rule_config()).unwrap()),
        curation_rule_revision: None,
        maintained_condition: None,
        maintained_condition_revision: None,
        created_at_seq: 0,
    }
}

fn test_view(key: &BeliefKey, confidence: f64, revision_id: &str, seq: u64) -> BeliefView {
    BeliefView {
        view_id: format!("view-{revision_id}"),
        key: key.clone(),
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
        provenance: BeliefProvenanceSummary::empty(),
        hydration: HydrationRefs {
            evidence_ids: Vec::new(),
            source_fact_ids: Vec::new(),
            graph_anchor_ids: Vec::new(),
            revision_id: Some(revision_id.to_string()),
        },
        theory_revision: None,
    }
}

/// Commit a real durable revision plus its planner-safe view for one key.
fn commit_revision(
    belief_store: &BeliefStore,
    key: &BeliefKey,
    revision_id: &str,
    confidence: f64,
    start: u64,
    end: u64,
    prior_revision_id: Option<String>,
) {
    let lease = belief_store
        .acquire_lease(AssessmentLease {
            lease_id: format!("lease-{revision_id}"),
            belief_key: key.clone(),
            epoch: end,
            owner_id: "test-worker".to_string(),
            input_cursor_start: start,
            input_cursor_end: end,
            started_at_seq: start,
            expires_at_seq: end + 100,
            comparator_engine_id: "weighted_bayesian".to_string(),
            config_snapshot_hash: "hash".to_string(),
            status: LeaseStatus::Queued,
        })
        .unwrap();
    let revision = BeliefRevision {
        revision_id: revision_id.to_string(),
        belief_key: key.clone(),
        prior_revision_id,
        comparator_engine_id: "weighted_bayesian".to_string(),
        comparator_engine_version: "1".to_string(),
        config_snapshot_hash: "hash".to_string(),
        evidence_ids: Vec::new(),
        supporting_evidence_ids: Vec::new(),
        contradicted_evidence_ids: Vec::new(),
        source_cursor_start: start,
        source_cursor_end: end,
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
            high_water_seq: end,
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
        theory_revision: None,
    };
    belief_store.commit_revision(&lease, &revision).unwrap();
    belief_store.complete_lease(&lease).unwrap();
    belief_store
        .put_view(&test_view(key, confidence, revision_id, end))
        .unwrap();
    belief_store.flush().unwrap();
}

fn seeded_graph(db: sled::Db) -> Arc<TraversalStore> {
    let store = Arc::new(TraversalStore::new(db).unwrap());
    let node = subject();
    let frame = object("context", "frame", "frame-a");
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
        anchor_ref: object("context", "head", "node-a::analysis"),
        subject: node,
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
    store
}

/// Test double for the named curation-to-goal-set port.
///
/// Implements both sink traits, so the blanket impl makes it the port.
#[derive(Default)]
struct RecordingPort {
    goal_commands: Vec<AgentGoalCommand>,
    mutation_commands: Vec<AgentGoalMutationCommand>,
    fail_mutations: bool,
}

impl meld_world_model::agent::AgentGoalCommandSink for RecordingPort {
    fn submit_goal_command(
        &mut self,
        command: &AgentGoalCommand,
    ) -> Result<AgentSinkSubmission, AgentSinkError> {
        self.goal_commands.push(command.clone());
        Ok(AgentSinkSubmission::new(
            command.command_id.clone(),
            command.goal.goal_id.clone(),
            "applied",
        ))
    }
}

impl meld_world_model::agent::AgentGoalMutationSink for RecordingPort {
    fn submit_goal_mutation(
        &mut self,
        command: &AgentGoalMutationCommand,
    ) -> Result<AgentSinkSubmission, AgentSinkError> {
        self.mutation_commands.push(command.clone());
        if self.fail_mutations {
            return Err(AgentSinkError::retryable("mutation sink unavailable"));
        }
        Ok(AgentSinkSubmission::new(
            command.command_id.clone(),
            command.goal_id.clone(),
            "applied",
        ))
    }
}

struct Fixture {
    temp_dir: tempfile::TempDir,
    agent_store: Arc<AgentStore>,
    belief_store: Arc<BeliefStore>,
    traversal_store: Arc<TraversalStore>,
    subscription_id: String,
}

impl Fixture {
    /// Open stores, register the seed agent, and subscribe the primary key.
    fn open(with_rule: bool) -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let agent_store =
            Arc::new(AgentStore::new(sled::open(temp_dir.path().join("agent")).unwrap()).unwrap());
        let belief_store = Arc::new(
            BeliefStore::new(sled::open(temp_dir.path().join("belief")).unwrap()).unwrap(),
        );
        let traversal_store = seeded_graph(sled::open(temp_dir.path().join("graph")).unwrap());
        AgentRegistration::new(&agent_store)
            .register_seed_agent(registration(with_rule))
            .unwrap();
        let subscription = AgentSubscription::new(&agent_store)
            .subscribe(SubscribeAgentCommand {
                agent_id: AGENT_ID.to_string(),
                belief_key: belief_key(),
                created_at_seq: 1,
            })
            .unwrap();
        Self {
            temp_dir,
            agent_store,
            belief_store,
            traversal_store,
            subscription_id: subscription.subscription_id,
        }
    }

    fn goal_actor(&self) -> AgentGoalCurationActor {
        AgentGoalCurationActor::new(
            "world_model.agent.goal_curation",
            AGENT_ID,
            Arc::clone(&self.agent_store),
            Arc::clone(&self.belief_store),
            Arc::clone(&self.traversal_store),
        )
    }

    fn satisfaction_actor(&self) -> AgentSatisfactionCurationActor {
        AgentSatisfactionCurationActor::new(
            "world_model.agent.satisfaction_curation",
            AGENT_ID,
            Arc::clone(&self.agent_store),
            Arc::clone(&self.belief_store),
            Arc::clone(&self.traversal_store),
        )
    }

    /// Reopen the agent store from disk, simulating a process crash.
    fn crash_and_reopen_agent_store(&mut self) {
        let path = self.temp_dir.path().join("agent");
        // Release the only live handle before reopening: assignment order
        // would otherwise hold the sled file lock during the reopen. A
        // scratch store occupies the field for the duration of the swap.
        let scratch = Arc::new(
            AgentStore::new(sled::open(self.temp_dir.path().join("scratch-agent")).unwrap())
                .unwrap(),
        );
        drop(std::mem::replace(&mut self.agent_store, scratch));
        self.agent_store = Arc::new(AgentStore::new(reopen_sled_after_close(&path)).unwrap());
    }
}

fn goal_query_returning(
    summary: ActiveGoalSummary,
) -> impl FnMut(&str) -> Result<ActiveGoalSummary, AgentActiveGoalQueryError> {
    move |_: &str| Ok(summary.clone())
}

fn request(sequence: u64, max_items: usize) -> AgentStepRequest {
    AgentStepRequest {
        sequence,
        max_items,
    }
}

fn curated_goal(fixture: &Fixture, lifecycle: GoalLifecycle) -> Goal {
    // Derive the goal exactly as curation would emit it so satisfaction
    // reviews target the same dedupe identity.
    let mut port = RecordingPort::default();
    let mut goal_query = goal_query_returning(ActiveGoalSummary::default());
    let report = fixture
        .goal_actor()
        .bounded_step(&request(1, 8), &mut goal_query, &mut port);
    assert!(report.fatal_errors.is_empty(), "{:?}", report.fatal_errors);
    let mut goal = port.goal_commands[0].goal.clone();
    goal.lifecycle = lifecycle;
    goal
}

#[test]
fn named_port_identity_is_exposed_on_any_dual_sink() {
    let port = RecordingPort::default();
    assert_eq!(port.port_id(), CURATION_GOAL_SET_PORT_ID);
}

#[test]
fn delivery_selection_returns_only_newer_revisions_and_respects_budget() {
    let fixture = Fixture::open(true);
    let second_key = belief_key_for(SECOND_DIMENSION_ID);
    let second_subscription = AgentSubscription::new(&fixture.agent_store)
        .subscribe(SubscribeAgentCommand {
            agent_id: AGENT_ID.to_string(),
            belief_key: second_key.clone(),
            created_at_seq: 2,
        })
        .unwrap();
    commit_revision(
        &fixture.belief_store,
        &belief_key(),
        "revision-a",
        0.2,
        1,
        7,
        None,
    );
    commit_revision(
        &fixture.belief_store,
        &second_key,
        "revision-x",
        0.9,
        1,
        5,
        None,
    );
    let belief_query = BeliefQuery::new(&fixture.belief_store);
    let selector = AgentWorkSelector::new(&fixture.agent_store);

    let bounded = selector
        .select_deliveries(AGENT_ID, &belief_query, 1)
        .unwrap();
    assert_eq!(bounded.items.len(), 1);
    assert!(bounded.more_available);
    assert_eq!(bounded.items[0].subscription_id, fixture.subscription_id);
    assert_eq!(bounded.items[0].belief_revision_id, "revision-a");
    assert_eq!(bounded.items[0].revision_seq, 7);

    let full = selector
        .select_deliveries(AGENT_ID, &belief_query, 8)
        .unwrap();
    assert_eq!(full.items.len(), 2);
    assert!(!full.more_available);
    assert_eq!(
        full.items[1].subscription_id,
        second_subscription.subscription_id
    );

    // Curate everything, then confirm the cursors make both streams stale.
    let mut port = RecordingPort::default();
    let mut goal_query = goal_query_returning(ActiveGoalSummary::default());
    let report = fixture
        .goal_actor()
        .bounded_step(&request(1, 8), &mut goal_query, &mut port);
    assert!(report.fatal_errors.is_empty(), "{:?}", report.fatal_errors);
    assert_eq!(report.items_attempted, 2);

    let drained = selector
        .select_deliveries(AGENT_ID, &belief_query, 8)
        .unwrap();
    assert!(drained.items.is_empty());
    assert!(!drained.more_available);

    // Only a strictly newer revision restores eligibility.
    commit_revision(
        &fixture.belief_store,
        &belief_key(),
        "revision-b",
        0.3,
        8,
        9,
        Some("revision-a".to_string()),
    );
    let woken = selector
        .select_deliveries(AGENT_ID, &belief_query, 8)
        .unwrap();
    assert_eq!(woken.items.len(), 1);
    assert_eq!(woken.items[0].belief_revision_id, "revision-b");
    assert_eq!(woken.items[0].revision_seq, 9);
}

#[test]
fn goal_actor_curates_through_port_and_unchanged_ticks_attempt_no_work() {
    let fixture = Fixture::open(true);
    commit_revision(
        &fixture.belief_store,
        &belief_key(),
        "revision-a",
        0.2,
        1,
        7,
        None,
    );
    let mut port = RecordingPort::default();
    let mut goal_query = goal_query_returning(ActiveGoalSummary::default());
    let actor = fixture.goal_actor();

    let first = actor.bounded_step(&request(1, 4), &mut goal_query, &mut port);
    assert!(first.fatal_errors.is_empty(), "{:?}", first.fatal_errors);
    assert_eq!(first.items_selected, 1);
    assert_eq!(first.items_attempted, 1);
    assert_eq!(first.decisions_persisted, 1);
    assert_eq!(first.sink_submissions, 1);
    assert_eq!(first.sink_receipts.len(), 1);
    assert_eq!(port.goal_commands.len(), 1);

    // Unchanged durable state: repeated ticks select and commit nothing.
    for sequence in 2..5 {
        let idle = actor.bounded_step(&request(sequence, 4), &mut goal_query, &mut port);
        assert!(idle.fatal_errors.is_empty(), "{:?}", idle.fatal_errors);
        assert_eq!(idle.items_selected, 0);
        assert_eq!(idle.items_attempted, 0);
        assert_eq!(idle.decisions_persisted, 0);
        assert_eq!(idle.sink_submissions, 0);
        assert!(!idle.budget_exhausted);
    }
    assert_eq!(port.goal_commands.len(), 1);
}

#[test]
fn goal_actor_resolves_rule_from_record_and_fails_truthfully_when_absent() {
    let fixture = Fixture::open(false);
    commit_revision(
        &fixture.belief_store,
        &belief_key(),
        "revision-a",
        0.2,
        1,
        7,
        None,
    );
    let mut port = RecordingPort::default();
    let mut goal_query = goal_query_returning(ActiveGoalSummary::default());

    let report = fixture
        .goal_actor()
        .bounded_step(&request(1, 4), &mut goal_query, &mut port);

    assert_eq!(report.fatal_errors.len(), 1);
    assert_eq!(report.fatal_errors[0].code, "curation_rule_missing");
    assert!(report.fatal_errors[0]
        .message
        .contains("has no installed curation rule"));
    assert_eq!(report.items_selected, 0);
    assert!(port.goal_commands.is_empty());
    assert_eq!(
        fixture
            .agent_store
            .get_subscription(&fixture.subscription_id)
            .unwrap()
            .unwrap()
            .last_delivered_seq,
        0
    );
}

#[test]
fn satisfaction_of_active_goal_emits_satisfy_with_observed_epoch() {
    let fixture = Fixture::open(true);
    commit_revision(
        &fixture.belief_store,
        &belief_key(),
        "revision-a",
        0.2,
        1,
        7,
        None,
    );
    let goal = curated_goal(&fixture, GoalLifecycle::Active);
    commit_revision(
        &fixture.belief_store,
        &belief_key(),
        "revision-b",
        0.95,
        8,
        9,
        Some("revision-a".to_string()),
    );
    let summary = ActiveGoalSummary {
        goals: vec![goal.clone()],
        lifecycle_epochs: BTreeMap::from([(goal.goal_id.clone(), 3)]),
    };
    let mut port = RecordingPort::default();
    let mut goal_query = goal_query_returning(summary);

    let report =
        fixture
            .satisfaction_actor()
            .bounded_step(&request(10, 4), &mut goal_query, &mut port);

    assert!(report.fatal_errors.is_empty(), "{:?}", report.fatal_errors);
    assert_eq!(report.items_selected, 1);
    assert_eq!(report.decisions_persisted, 1);
    assert_eq!(report.sink_receipts.len(), 1);
    assert_eq!(port.mutation_commands.len(), 1);
    let command = &port.mutation_commands[0];
    assert_eq!(command.goal_id, goal.goal_id);
    assert_eq!(command.review_seq, 10);
    assert_eq!(
        command.kind,
        AgentGoalMutationKind::Satisfy {
            at_seq: 10,
            lifecycle_epoch: 3,
        }
    );

    // The mutation trigger is complete: unchanged ticks attempt no work.
    let idle =
        fixture
            .satisfaction_actor()
            .bounded_step(&request(11, 4), &mut goal_query, &mut port);
    assert_eq!(idle.items_selected, 0);
    assert_eq!(idle.decisions_persisted, 0);
    assert_eq!(port.mutation_commands.len(), 1);
}

#[test]
fn unchanged_low_confidence_revision_is_ineligible_after_one_absorbed_review() {
    let fixture = Fixture::open(true);
    commit_revision(
        &fixture.belief_store,
        &belief_key(),
        "revision-a",
        0.2,
        1,
        7,
        None,
    );
    let goal = curated_goal(&fixture, GoalLifecycle::Active);
    let mut port = RecordingPort::default();
    let mut goal_query = goal_query_returning(ActiveGoalSummary::from_goals(vec![goal]));
    let actor = fixture.satisfaction_actor();

    let first = actor.bounded_step(&request(10, 4), &mut goal_query, &mut port);
    assert!(first.fatal_errors.is_empty(), "{:?}", first.fatal_errors);
    assert_eq!(first.items_selected, 1);
    assert_eq!(first.decisions_persisted, 1);
    assert!(port.mutation_commands.is_empty());

    // One absorbed review of the unchanged revision closes the window.
    for sequence in 11..14 {
        let idle = actor.bounded_step(&request(sequence, 4), &mut goal_query, &mut port);
        assert!(idle.fatal_errors.is_empty(), "{:?}", idle.fatal_errors);
        assert_eq!(idle.items_selected, 0);
        assert_eq!(idle.decisions_persisted, 0);
    }

    // A new revision on the goal's key restores eligibility.
    commit_revision(
        &fixture.belief_store,
        &belief_key(),
        "revision-b",
        0.25,
        8,
        9,
        Some("revision-a".to_string()),
    );
    let woken = actor.bounded_step(&request(20, 4), &mut goal_query, &mut port);
    assert_eq!(woken.items_selected, 1);
    assert_eq!(woken.decisions_persisted, 1);
    let checkpoint = fixture
        .agent_store
        .get_satisfaction_checkpoint(AGENT_ID, &fixture.subscription_id)
        .unwrap()
        .unwrap();
    assert_eq!(checkpoint.belief_revision_id, "revision-b");
    assert_eq!(checkpoint.review_seq, 20);

    let idle = actor.bounded_step(&request(21, 4), &mut goal_query, &mut port);
    assert_eq!(idle.items_selected, 0);
    assert!(port.mutation_commands.is_empty());
}

#[test]
fn drift_on_satisfied_goal_emits_reopen_and_replays_idempotently_after_crash() {
    let mut fixture = Fixture::open(true);
    commit_revision(
        &fixture.belief_store,
        &belief_key(),
        "revision-a",
        0.2,
        1,
        7,
        None,
    );
    let goal = curated_goal(&fixture, GoalLifecycle::Satisfied { at_seq: 9 });
    // Later drift: the maintained condition is violated by a new revision.
    commit_revision(
        &fixture.belief_store,
        &belief_key(),
        "revision-c",
        0.1,
        8,
        12,
        Some("revision-a".to_string()),
    );
    let summary = ActiveGoalSummary {
        goals: vec![goal.clone()],
        lifecycle_epochs: BTreeMap::from([(goal.goal_id.clone(), 5)]),
    };
    let mut goal_query = goal_query_returning(summary);

    // The mutation sink fails after the decision is durable, standing in for
    // a crash between decision persistence and receipt.
    let mut failing_port = RecordingPort {
        fail_mutations: true,
        ..RecordingPort::default()
    };
    let interrupted = fixture.satisfaction_actor().bounded_step(
        &request(20, 4),
        &mut goal_query,
        &mut failing_port,
    );
    assert!(
        interrupted.fatal_errors.is_empty(),
        "{:?}",
        interrupted.fatal_errors
    );
    assert_eq!(interrupted.retryable_errors.len(), 1);
    assert_eq!(interrupted.decisions_persisted, 1);
    assert!(interrupted.sink_receipts.is_empty());
    assert_eq!(failing_port.mutation_commands.len(), 1);
    let first_command = failing_port.mutation_commands[0].clone();
    assert_eq!(
        first_command.kind,
        AgentGoalMutationKind::Reopen {
            triggering_belief_revision_id: "revision-c".to_string(),
            observed_lifecycle_epoch: 5,
        }
    );

    fixture.crash_and_reopen_agent_store();

    // Replay keeps the claimed review identity: same decision, same command,
    // one receipt, no second review sequence.
    let mut port = RecordingPort::default();
    let replay =
        fixture
            .satisfaction_actor()
            .bounded_step(&request(21, 4), &mut goal_query, &mut port);
    assert!(replay.fatal_errors.is_empty(), "{:?}", replay.fatal_errors);
    assert_eq!(replay.items_selected, 1);
    assert_eq!(replay.sink_receipts.len(), 1);
    assert_eq!(port.mutation_commands.len(), 1);
    let replayed_command = &port.mutation_commands[0];
    assert_eq!(replayed_command.command_id, first_command.command_id);
    assert_eq!(replayed_command.review_seq, 20);
    assert_eq!(replayed_command.kind, first_command.kind);
    let checkpoint = fixture
        .agent_store
        .get_satisfaction_checkpoint(AGENT_ID, &fixture.subscription_id)
        .unwrap()
        .unwrap();
    assert_eq!(checkpoint.review_seq, 20);
    let receipt = fixture
        .agent_store
        .sink_receipt_by_decision(&replay.sink_receipts[0].decision_id)
        .unwrap()
        .unwrap();
    assert_eq!(receipt.submission.command_id, first_command.command_id);
    assert_eq!(
        fixture
            .agent_store
            .sink_receipts_by_command(&first_command.command_id)
            .unwrap()
            .len(),
        1
    );

    // Completed reopen: unchanged ticks attempt no work.
    let idle =
        fixture
            .satisfaction_actor()
            .bounded_step(&request(22, 4), &mut goal_query, &mut port);
    assert_eq!(idle.items_selected, 0);
    assert_eq!(port.mutation_commands.len(), 1);
}

#[test]
fn reopen_command_identity_is_stable_across_review_sequences() {
    // Two reviews of the same drift under different review sequences must
    // author the same reopen command identity for execution to absorb.
    let fixture = Fixture::open(true);
    commit_revision(
        &fixture.belief_store,
        &belief_key(),
        "revision-a",
        0.2,
        1,
        7,
        None,
    );
    let goal = curated_goal(&fixture, GoalLifecycle::Satisfied { at_seq: 9 });
    let summary = ActiveGoalSummary::from_goals(vec![goal]);
    let belief_query = BeliefQuery::new(&fixture.belief_store);
    let planner_query = meld_world_model::planner::PlannerQuery::new(
        BeliefQuery::new(&fixture.belief_store),
        meld_world_model::TraversalQuery::new(&fixture.traversal_store),
    );
    let curation = meld_world_model::agent::AgentCuration::new(&fixture.agent_store);

    let review = |seq| meld_world_model::agent::AgentSatisfactionReview {
        agent_id: AGENT_ID.to_string(),
        subscription_id: fixture.subscription_id.clone(),
        review_seq: seq,
    };
    let first = curation
        .assemble_satisfaction_input(&review(30), &belief_query, &planner_query, summary.clone())
        .map(meld_world_model::agent::curate_goal_satisfaction)
        .unwrap()
        .unwrap();
    let second = curation
        .assemble_satisfaction_input(&review(31), &belief_query, &planner_query, summary)
        .map(meld_world_model::agent::curate_goal_satisfaction)
        .unwrap()
        .unwrap();

    let first_command = first.goal_mutation_command.unwrap();
    let second_command = second.goal_mutation_command.unwrap();
    assert_ne!(first.decision.decision_id, second.decision.decision_id);
    assert_eq!(first_command.command_id, second_command.command_id);
    assert_eq!(first_command.kind, second_command.kind);
}

#[test]
fn seed_registration_installs_rule_binding_idempotently_and_verifies_hash() {
    let fixture = Fixture::open(true);
    let registration_facade = AgentRegistration::new(&fixture.agent_store);

    let replay = registration_facade
        .register_seed_agent(registration(true))
        .unwrap();
    let binding = replay.curation_rule.clone().expect("installed binding");
    assert_eq!(binding.rule, rule_config());
    assert_eq!(
        binding.content_hash,
        AgentCurationRuleBinding::content_hash_for(&rule_config()).unwrap()
    );
    assert_eq!(
        replay.installed_curation_rule().unwrap().clone(),
        rule_config()
    );

    let mut tampered = registration(true);
    tampered.agent_id = "seed.other".to_string();
    tampered.curation_rule.as_mut().unwrap().content_hash = "0000000000000000".to_string();
    let error = registration_facade
        .register_seed_agent(tampered)
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("content hash does not match rule content"));
}

#[test]
fn legacy_records_without_new_fields_still_load() {
    // A stored mutation command from before epochs: Satisfy without the
    // lifecycle epoch field and no Reopen variant anywhere.
    let legacy_command = serde_json::json!({
        "command_id": "mutation-legacy",
        "agent_id": AGENT_ID,
        "goal_id": "goal-legacy",
        "kind": { "Satisfy": { "at_seq": 22 } },
        "dedupe_key": {
            "agent_id": AGENT_ID,
            "subject_key": subject().index_key(),
            "branch_id": "main",
            "dimension_id": DIMENSION_ID,
            "target_condition_key": "{\"Above\":{\"Literal\":{\"Number\":0.7}}}",
            "source_kind": "belief_divergence"
        },
        "review_seq": 22,
        "projection_version": "world_model.planner.v1",
        "planner_source_refs": [],
        "planner_warnings": []
    });
    let command: AgentGoalMutationCommand = serde_json::from_value(legacy_command).unwrap();
    assert_eq!(
        command.kind,
        AgentGoalMutationKind::Satisfy {
            at_seq: 22,
            lifecycle_epoch: 0,
        }
    );
    assert!(command.validate().is_ok());

    // A stored goal snapshot from before the epoch observation map existed.
    let legacy_summary: ActiveGoalSummary = serde_json::from_str("{\"goals\":[]}").unwrap();
    assert!(legacy_summary.lifecycle_epochs.is_empty());
    assert_eq!(legacy_summary.lifecycle_epoch("goal-any"), 0);

    // The new checkpoint record round-trips.
    let checkpoint = AgentSatisfactionCheckpoint {
        agent_id: AGENT_ID.to_string(),
        subscription_id: "subscription-a".to_string(),
        belief_revision_id: "revision-a".to_string(),
        review_seq: 10,
        claimed_at_seq: 10,
    };
    let round_tripped: AgentSatisfactionCheckpoint =
        serde_json::from_str(&serde_json::to_string(&checkpoint).unwrap()).unwrap();
    assert_eq!(round_tripped, checkpoint);
}
