use std::cell::{Cell, RefCell};
use std::sync::Arc;

use meld_lang::GoalSource;
use meld_world_model::activation::{
    AgentBootstrapReceipt, AgentCurationRuleRecord, BeliefActivationReceipt,
};
use meld_world_model::agent::{
    ActiveGoalSummary, AgentActiveGoalQueryError, AgentAuthoredCommand, AgentBootstrapProgress,
    AgentBootstrapProgressStatus, AgentBootstrapStage, AgentCommandOutcomeQueryError,
    AgentCurationDecision, AgentCurationRuleConfig, AgentCurationTickRequest, AgentGoalCommand,
    AgentGoalCurationActor, AgentGoalMutationCommand, AgentQuery, AgentRegistration,
    AgentSatisfactionCurationActor, AgentSatisfactionCursorIdentity, AgentSinkError,
    AgentSinkSubmission, AgentStatus, AgentStore, AgentSubscription, AgentSubscriptionRecord,
    SeedAgentRegistration, SubscribeAgentCommand, AGENT_GOAL_CURATION_ACTOR_ID,
    AGENT_SATISFACTION_CURATION_ACTOR_ID,
};
use meld_world_model::belief::{
    AssessmentLease, BeliefProvenanceSummary, BeliefRevision, BeliefStore, BranchScope,
    ContradictionState, EvidenceItem, EvidenceRole, EvidenceValue, FreshnessState, LeaseStatus,
    PlannerProjectionSummary, PosteriorSummary,
};
use meld_world_model::events::DomainObjectRef;
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::{BeliefKey, BeliefStatus, PerspectiveKey};

const DIMENSION_ID: &str = "actor_confidence";

fn subject() -> DomainObjectRef {
    DomainObjectRef::new("workspace", "node", "actor-a").expect("subject")
}

fn belief_key() -> BeliefKey {
    BeliefKey {
        subject: subject(),
        dimension_id: DIMENSION_ID.to_string(),
        predicate_id: "confidence".to_string(),
        perspective: PerspectiveKey::new("default", "actor").expect("perspective"),
        branch_scope: BranchScope::main(),
        evidence_policy_id: "actor-policy".to_string(),
    }
}

fn rule_config(agent_id: &str, urgency: u32) -> AgentCurationRuleConfig {
    AgentCurationRuleConfig {
        dimension_id: DIMENSION_ID.to_string(),
        threshold: 0.7,
        priority_urgency: urgency,
        desired_summary: format!("durable target for {agent_id}"),
        source_kind: "durable-actor-rule".to_string(),
    }
}

fn stable_hash_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn register_operational_agent(
    db: &sled::Db,
    store: &AgentStore,
    agent_id: &str,
    created_at_seq: u64,
    urgency: u32,
) -> AgentSubscriptionRecord {
    let mut agent = AgentRegistration::new(store)
        .register_seed_agent(SeedAgentRegistration {
            agent_id: agent_id.to_string(),
            perspective_key: belief_key().perspective,
            subject: subject(),
            branch_scope: BranchScope::main(),
            observation_scope: DIMENSION_ID.to_string(),
            directive_id: format!("directive-{agent_id}"),
            seed_provenance: "agent orchestration fixture".to_string(),
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

    install_bootstrap_rule(db, agent_id, &subscription, urgency);
    subscription
}

fn install_bootstrap_rule(
    db: &sled::Db,
    agent_id: &str,
    subscription: &AgentSubscriptionRecord,
    urgency: u32,
) {
    let bootstrap_id = format!("bootstrap-{agent_id}");
    let activation_id = format!("activation-{agent_id}");
    let activation_hash = "a".repeat(64);
    let input_hash = "b".repeat(64);
    let completed_at_seq = subscription.updated_at_seq + 1;
    let rule = AgentCurationRuleRecord {
        rule_id: format!("rule-{agent_id}"),
        agent_id: agent_id.to_string(),
        config: rule_config(agent_id, urgency),
    };
    let receipt = AgentBootstrapReceipt {
        receipt_id: format!(
            "agent-bootstrap-receipt-{}",
            stable_hash_hex(bootstrap_id.as_bytes())
        ),
        bootstrap_id: bootstrap_id.clone(),
        activation_hash: activation_hash.clone(),
        activation_id: activation_id.clone(),
        input_hash: input_hash.clone(),
        belief: BeliefActivationReceipt {
            family_id: DIMENSION_ID.to_string(),
            config_snapshot_hash: "c".repeat(64),
            activation_hash: activation_hash.clone(),
            activation_id: activation_id.clone(),
        },
        directive_id: format!("directive-{agent_id}"),
        agent_id: agent_id.to_string(),
        rule_id: rule.rule_id.clone(),
        subscription_id: subscription.subscription_id.clone(),
        completed_at_seq,
    };
    let progress = AgentBootstrapProgress {
        bootstrap_id: bootstrap_id.clone(),
        activation_id,
        activation_hash,
        input_hash,
        stage: AgentBootstrapStage::Completed,
        status: AgentBootstrapProgressStatus::Completed,
        updated_at_seq: completed_at_seq,
    };

    db.open_tree("agent_curation_rules")
        .expect("curation rules")
        .insert(
            rule.rule_id.as_bytes(),
            serde_json::to_vec(&rule).expect("encode rule"),
        )
        .expect("write rule");
    db.open_tree("agent_bootstrap_receipts")
        .expect("bootstrap receipts")
        .insert(
            bootstrap_id.as_bytes(),
            serde_json::to_vec(&receipt).expect("encode receipt"),
        )
        .expect("write receipt");
    db.open_tree("agent_bootstrap_receipts_by_agent")
        .expect("bootstrap receipt index")
        .insert(agent_id.as_bytes(), bootstrap_id.as_bytes())
        .expect("index receipt");
    db.open_tree("agent_bootstrap_progress")
        .expect("bootstrap progress")
        .insert(
            bootstrap_id.as_bytes(),
            serde_json::to_vec(&progress).expect("encode progress"),
        )
        .expect("write progress");
}

fn install_belief(store: &BeliefStore, confidence: f64, revision_id: &str, seq: u64) {
    let key = belief_key();
    let evidence_id = format!("evidence-{revision_id}");
    store
        .put_evidence_once(&EvidenceItem {
            evidence_id: evidence_id.clone(),
            candidate_key: key.clone(),
            source_fact_ids: vec![format!("source-{revision_id}")],
            graph_anchor_ids: Vec::new(),
            source_cursor_start: seq,
            source_cursor_end: seq,
            role: EvidenceRole::Support,
            evidence_schema_id: "agent-actor-test".to_string(),
            typed_value: EvidenceValue::Scalar(confidence),
            reliability: 1.0,
            precision: 1.0,
            reference_time: None,
            transaction_seq: seq,
            content_hash: None,
            provenance: BeliefProvenanceSummary::empty(),
        })
        .expect("persist belief evidence");
    store.mark_dirty(&key, seq).expect("mark belief dirty");
    let dirty = store
        .dirty_state(&key)
        .expect("dirty belief query")
        .expect("dirty belief state");
    let prior_revision_id = store
        .current_revision(&key)
        .expect("current revision query")
        .map(|revision| revision.revision_id);
    let lease = store
        .acquire_lease(AssessmentLease {
            lease_id: format!("agent-actor-lease-{revision_id}"),
            belief_key: key.clone(),
            epoch: dirty.mutation_generation,
            owner_id: "agent-actor-test".to_string(),
            input_cursor_start: seq,
            input_cursor_end: seq,
            assignment_cursor_start: dirty.assessment_cursor.clone(),
            assignment_cursor_end: dirty.assessment_cursor,
            assignment_window_complete: true,
            started_at_seq: seq,
            expires_at_seq: seq + 20,
            comparator_engine_id: "agent-actor-test".to_string(),
            config_snapshot_hash: "agent-actor-config".to_string(),
            status: LeaseStatus::Queued,
        })
        .expect("acquire belief lease");
    let revision = BeliefRevision {
        revision_id: revision_id.to_string(),
        belief_key: key.clone(),
        prior_revision_id,
        comparator_engine_id: "agent-actor-test".to_string(),
        comparator_engine_version: "1".to_string(),
        config_snapshot_hash: "agent-actor-config".to_string(),
        evidence_ids: vec![evidence_id.clone()],
        supporting_evidence_ids: vec![evidence_id],
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
    store
        .commit_revision(&lease, &revision)
        .expect("commit belief revision");
}

fn stores(db: &sled::Db) -> (Arc<AgentStore>, Arc<BeliefStore>, Arc<TraversalStore>) {
    (
        Arc::new(AgentStore::new(db.clone()).expect("agent store")),
        Arc::new(BeliefStore::new(db.clone()).expect("belief store")),
        Arc::new(TraversalStore::new(db.clone()).expect("traversal store")),
    )
}

#[test]
fn actors_return_concrete_no_work_reports() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db = sled::open(temp.path()).expect("database");
    let (agent_store, belief_store, traversal_store) = stores(&db);
    let goal_actor = AgentGoalCurationActor::new(
        agent_store.clone(),
        belief_store.clone(),
        traversal_store.clone(),
    );
    let satisfaction_actor =
        AgentSatisfactionCurationActor::new(agent_store, belief_store, traversal_store);
    let goal_query_called = Cell::new(false);
    let mut goal_query = |_agent_id: &str| {
        goal_query_called.set(true);
        Ok::<ActiveGoalSummary, AgentActiveGoalQueryError>(ActiveGoalSummary::default())
    };
    let outcome_query_called = Cell::new(false);
    let mut outcome_query = |_decision: &AgentCurationDecision, _command: &AgentAuthoredCommand| {
        outcome_query_called.set(true);
        Ok::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(None)
    };
    let mut goal_sink = |_command: &AgentGoalCommand| {
        Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::fatal(
            "no-work goal actor reached its sink",
        ))
    };

    let goal_report = goal_actor.tick(
        AgentCurationTickRequest { max_items: 1 },
        &mut goal_query,
        &mut outcome_query,
        &mut goal_sink,
    );
    let mut mutation_sink = |_command: &AgentGoalMutationCommand| {
        Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::fatal(
            "no-work satisfaction actor reached its sink",
        ))
    };
    let satisfaction_report = satisfaction_actor.tick(
        AgentCurationTickRequest { max_items: 1 },
        &mut goal_query,
        &mut outcome_query,
        &mut mutation_sink,
    );

    for (report, actor_id) in [
        (goal_report, AGENT_GOAL_CURATION_ACTOR_ID),
        (satisfaction_report, AGENT_SATISFACTION_CURATION_ACTOR_ID),
    ] {
        assert_eq!(report.actor_id, actor_id);
        assert_eq!(report.input_sequence, 0);
        assert_eq!(report.output_sequence, 0);
        assert_eq!(report.selected_count, 0);
        assert_eq!(report.decision_count, 0);
        assert_eq!(report.sink_receipt_count, 0);
        assert_eq!(report.cursor_advanced_count, 0);
        assert!(report.retryable_errors.is_empty());
        assert!(report.fatal_errors.is_empty());
        assert!(!report.budget_exhausted);
    }
    assert!(!goal_query_called.get());
    assert!(!outcome_query_called.get());
}

#[test]
fn goal_actor_bounds_selection_and_resolves_the_durable_rule() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db = sled::open(temp.path()).expect("database");
    let (agent_store, belief_store, traversal_store) = stores(&db);
    let selected = register_operational_agent(&db, &agent_store, "agent-a", 1, 17);
    let pending = register_operational_agent(&db, &agent_store, "agent-z", 10, 99);
    install_belief(&belief_store, 0.2, "revision-7", 7);
    let actor = AgentGoalCurationActor::new(agent_store.clone(), belief_store, traversal_store);
    let mut goal_query = |_agent_id: &str| {
        Ok::<ActiveGoalSummary, AgentActiveGoalQueryError>(ActiveGoalSummary::default())
    };
    let mut outcome_query = |_decision: &AgentCurationDecision, _command: &AgentAuthoredCommand| {
        Ok::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(None)
    };
    let submitted = RefCell::new(Vec::new());
    let mut sink = |command: &AgentGoalCommand| {
        assert_eq!(command.goal.agent_id, "agent-a");
        assert_eq!(command.goal.priority.urgency, 17);
        match &command.goal.source {
            GoalSource::BeliefDivergence { desired, .. } => {
                assert_eq!(desired, "durable target for agent-a")
            }
            source => panic!("unexpected durable rule source: {source:?}"),
        }
        submitted.borrow_mut().push(command.command_id.clone());
        Ok::<AgentSinkSubmission, AgentSinkError>(AgentSinkSubmission::new(
            &command.command_id,
            &command.goal.goal_id,
            "applied",
        ))
    };

    let report = actor.tick(
        AgentCurationTickRequest { max_items: 1 },
        &mut goal_query,
        &mut outcome_query,
        &mut sink,
    );

    assert_eq!(report.actor_id, AGENT_GOAL_CURATION_ACTOR_ID);
    assert_eq!(report.selected_count, 1);
    assert_eq!(report.input_sequence, 7);
    assert_eq!(report.output_sequence, 7);
    assert_eq!(report.decision_count, 1);
    assert_eq!(report.sink_receipt_count, 1);
    assert_eq!(report.cursor_advanced_count, 1);
    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    assert!(report.budget_exhausted);
    assert_eq!(submitted.borrow().len(), 1);
    assert_eq!(
        agent_store
            .get_subscription(&selected.subscription_id)
            .expect("selected subscription query")
            .expect("selected subscription")
            .last_delivered_seq,
        7
    );
    assert_eq!(
        agent_store
            .get_subscription(&pending.subscription_id)
            .expect("pending subscription query")
            .expect("pending subscription")
            .last_delivered_seq,
        0
    );
}

#[test]
fn satisfaction_actor_advances_only_the_exact_selected_cursor() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db = sled::open(temp.path()).expect("database");
    let (agent_store, belief_store, traversal_store) = stores(&db);
    let selected = register_operational_agent(&db, &agent_store, "agent-a", 1, 17);
    let pending = register_operational_agent(&db, &agent_store, "agent-z", 10, 99);
    install_belief(&belief_store, 0.95, "revision-7", 7);
    let actor =
        AgentSatisfactionCurationActor::new(agent_store.clone(), belief_store, traversal_store);
    let mut goal_query = |_agent_id: &str| {
        Ok::<ActiveGoalSummary, AgentActiveGoalQueryError>(ActiveGoalSummary::default())
    };
    let outcome_query_called = Cell::new(false);
    let mut outcome_query = |_decision: &AgentCurationDecision, _command: &AgentAuthoredCommand| {
        outcome_query_called.set(true);
        Ok::<Option<AgentSinkSubmission>, AgentCommandOutcomeQueryError>(None)
    };
    let mutation_sink_called = Cell::new(false);
    let mut sink = |_command: &AgentGoalMutationCommand| {
        mutation_sink_called.set(true);
        Err::<AgentSinkSubmission, AgentSinkError>(AgentSinkError::fatal(
            "absorbed satisfaction review reached its sink",
        ))
    };

    let report = actor.tick(
        AgentCurationTickRequest { max_items: 1 },
        &mut goal_query,
        &mut outcome_query,
        &mut sink,
    );

    assert_eq!(report.actor_id, AGENT_SATISFACTION_CURATION_ACTOR_ID);
    assert_eq!(report.selected_count, 1);
    assert_eq!(report.input_sequence, 7);
    assert_eq!(report.output_sequence, 7);
    assert_eq!(report.decision_count, 1);
    assert_eq!(report.sink_receipt_count, 0);
    assert_eq!(report.cursor_advanced_count, 1);
    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    assert!(report.budget_exhausted);
    assert!(!outcome_query_called.get());
    assert!(!mutation_sink_called.get());

    let selected_identity = AgentSatisfactionCursorIdentity::belief_revision(
        "agent-a",
        &selected.subscription_id,
        selected.belief_key.branch_scope.branch_id.clone(),
    );
    let pending_identity = AgentSatisfactionCursorIdentity::belief_revision(
        "agent-z",
        &pending.subscription_id,
        pending.belief_key.branch_scope.branch_id.clone(),
    );
    assert_eq!(
        AgentQuery::new(&agent_store)
            .satisfaction_review_cursor(&selected_identity)
            .expect("selected cursor query")
            .expect("selected cursor")
            .last_reviewed_seq,
        7
    );
    assert!(AgentQuery::new(&agent_store)
        .satisfaction_review_cursor(&pending_identity)
        .expect("pending cursor query")
        .is_none());
    assert_eq!(
        agent_store
            .get_subscription(&selected.subscription_id)
            .expect("selected delivery cursor query")
            .expect("selected delivery cursor")
            .last_delivered_seq,
        0
    );
    assert_eq!(
        agent_store
            .get_subscription(&pending.subscription_id)
            .expect("pending delivery cursor query")
            .expect("pending delivery cursor")
            .last_delivered_seq,
        0
    );
}
