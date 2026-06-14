use std::sync::Arc;

use meld_lang::{
    evaluate, Condition, Effect, EvalResult, Goal, GoalLifecycle, GoalPriority, GoalSource,
    Literal, Proposition, Term, WorldState,
};
use meld_world_model::agent::{
    curate_goal_satisfaction, curate_threshold_rule, ActiveGoalSummary, AgentActivationRecord,
    AgentActivationStatus, AgentCuration, AgentCurationDedupeKey, AgentCurationInput,
    AgentCurationInputRefs, AgentCurationRuleConfig, AgentDecisionKind, AgentDelivery,
    AgentGoalCommand, AgentGoalMutationCommand, AgentGoalMutationKind, AgentGoalSatisfactionInput,
    AgentQuery, AgentRegistration, AgentSatisfactionReview, AgentStatus, AgentStore,
    AgentSubscription, AgentSubscriptionRecord, AgentSubscriptionStatus, SeedAgentRegistration,
    SubscribeAgentCommand,
};
use meld_world_model::belief::{
    BeliefProvenanceSummary, BeliefQuery, BeliefStore, BranchScope, ContradictionState,
    FreshnessState, HydrationRefs, PlannerProjectionSummary, PosteriorSummary,
};
use meld_world_model::events::{DomainObjectRef, EventRelation};
use meld_world_model::planner::{PlannerQuery, PLANNER_PROJECTION_VERSION};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::{
    AnchorSelectionRecord, BeliefStatus, BeliefView, PerspectiveKey, TraversalFactRecord,
    TraversalQuery,
};
use proptest::prelude::*;

const AGENT_ID: &str = "seed.docs_freshness";
const DIMENSION_ID: &str = "docs_freshness";
const PREDICATE_ID: &str = "confidence";
const EVIDENCE_POLICY_ID: &str = "default_policy";
const THRESHOLD: f64 = 0.7;
const PRIORITY_URGENCY: u32 = 50;

fn object(domain_id: &str, object_kind: &str, object_id: &str) -> DomainObjectRef {
    DomainObjectRef::new(domain_id, object_kind, object_id).unwrap()
}

fn subject() -> DomainObjectRef {
    object("workspace_fs", "node", "node-a")
}

fn agent_store() -> (tempfile::TempDir, AgentStore) {
    let temp_dir = tempfile::tempdir().unwrap();
    let store = AgentStore::new(sled::open(temp_dir.path().join("agent")).unwrap()).unwrap();
    (temp_dir, store)
}

fn seed_agent_registration() -> SeedAgentRegistration {
    SeedAgentRegistration {
        agent_id: AGENT_ID.to_string(),
        perspective_key: PerspectiveKey::new("default", "default").unwrap(),
        subject: subject(),
        branch_scope: BranchScope::main(),
        observation_scope: DIMENSION_ID.to_string(),
        directive: "curate docs freshness goals".to_string(),
        seed_provenance: "trusted init".to_string(),
        created_at_seq: 0,
    }
}

fn belief_key() -> meld_world_model::BeliefKey {
    meld_world_model::BeliefKey {
        subject: subject(),
        dimension_id: DIMENSION_ID.to_string(),
        predicate_id: PREDICATE_ID.to_string(),
        perspective: PerspectiveKey::new("default", "default").unwrap(),
        branch_scope: BranchScope::main(),
        evidence_policy_id: EVIDENCE_POLICY_ID.to_string(),
    }
}

fn rule_config() -> AgentCurationRuleConfig {
    AgentCurationRuleConfig {
        dimension_id: DIMENSION_ID.to_string(),
        threshold: THRESHOLD,
        priority_urgency: PRIORITY_URGENCY,
        desired_summary: "confidence>0.7".to_string(),
        source_kind: "belief_divergence".to_string(),
    }
}

fn test_view(confidence: f64, revision_id: &str, seq: u64) -> BeliefView {
    let subject = subject();
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
            source_fact_ids: vec![format!("spine-{seq}")],
            graph_anchor_ids: vec!["anchor-a".to_string()],
            objects: vec![subject],
            relations: Vec::new(),
            revision_ids: vec![revision_id.to_string()],
        },
        hydration: HydrationRefs {
            evidence_ids: vec![format!("evidence-{revision_id}")],
            source_fact_ids: vec![format!("spine-{seq}")],
            graph_anchor_ids: vec!["anchor-a".to_string()],
            revision_id: Some(revision_id.to_string()),
        },
    }
}

fn seeded_graph() -> (tempfile::TempDir, Arc<TraversalStore>) {
    let temp_dir = tempfile::tempdir().unwrap();
    let store =
        Arc::new(TraversalStore::new(sled::open(temp_dir.path().join("graph")).unwrap()).unwrap());
    let node = subject();
    let frame = object("context", "frame", "frame-a");
    let relation = EventRelation::new("selected", node.clone(), frame.clone()).unwrap();
    let fact = TraversalFactRecord {
        fact_id: "fact-a".to_string(),
        source_spine_fact_id: "spine-a".to_string(),
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
        source_fact_ids: vec!["spine-a".to_string()],
        created_by_fact_id: "fact-a".to_string(),
        selected_at_seq: 1,
        ended_at_seq: None,
        ended_by_anchor_id: None,
        ended_by_fact_id: None,
    };
    store.put_fact(&fact).unwrap();
    store.put_anchor(&anchor).unwrap();
    store.set_current_anchor(&anchor).unwrap();
    (temp_dir, store)
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

fn curation_input(confidence: f64) -> AgentCurationInput {
    let (_temp_dir, store) = agent_store();
    let (agent, subscription) = setup_agent(&store);
    let view = test_view(confidence, "revision-a", 7);
    let projection = meld_world_model::planner::project_world_state(
        meld_world_model::planner::PlannerProjectionInput {
            context: meld_world_model::planner::PlannerProjectionContext {
                subject: agent.subject.clone(),
                perspective: agent.perspective_key.clone(),
                branch_scope: agent.branch_scope.clone(),
                projection_version: PLANNER_PROJECTION_VERSION.to_string(),
            },
            belief_view: Some(view.clone()),
            graph_scope: None,
            field_config: meld_world_model::planner::PlannerFieldProjectionConfig::default(),
        },
    )
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
    curate_threshold_rule(curation_input(0.2))
        .unwrap()
        .goal_command
        .unwrap()
}

fn active_goal_from_command(command: AgentGoalCommand) -> Goal {
    let mut goal = command.goal;
    goal.lifecycle = GoalLifecycle::Active;
    goal
}

fn world_state_with_confidence(confidence: f64) -> WorldState {
    WorldState::new(vec![
        Proposition::Accessible {
            scope: Term::Object(subject()),
        },
        Proposition::Holds {
            subject: Term::Object(subject()),
            dimension: Term::Dimension(DIMENSION_ID.to_string()),
            condition: Condition::Equals(Term::Literal(Literal::Number(confidence))),
        },
    ])
    .unwrap()
}

fn satisfaction_input(confidence: f64, review_seq: u64) -> AgentGoalSatisfactionInput {
    let goal = active_goal_from_command(low_confidence_goal_command());
    let mut input = curation_input(confidence);
    input.planner_projection.world_state = world_state_with_confidence(confidence);
    input.input_refs.planner_source_refs = vec!["source-a".to_string()];
    input.input_refs.planner_warnings = vec!["warning-a".to_string()];
    AgentGoalSatisfactionInput {
        agent: input.agent,
        subscription: input.subscription,
        review_seq,
        planner_projection: input.planner_projection,
        active_goals: ActiveGoalSummary { goals: vec![goal] },
        input_refs: input.input_refs,
    }
}

fn valid_goal_mutation_command() -> AgentGoalMutationCommand {
    curate_goal_satisfaction(satisfaction_input(0.95, 22))
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

fn invalid_goal_mutation_command(
    mutate: impl FnOnce(&mut AgentGoalMutationCommand),
) -> AgentGoalMutationCommand {
    let mut command = valid_goal_mutation_command();
    mutate(&mut command);
    command
}

fn invalid_goal_command(mutate: impl FnOnce(&mut AgentGoalCommand)) -> AgentGoalCommand {
    let mut command = low_confidence_goal_command();
    mutate(&mut command);
    command
}

fn expected_hash_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

fn expected_deterministic_id(prefix: &str, key: &str) -> String {
    format!("{prefix}-{}", expected_hash_hex(key.as_bytes()))
}

fn matching_active_goal_for(dedupe: &AgentCurationDedupeKey) -> Goal {
    Goal {
        goal_id: "goal-existing".to_string(),
        agent_id: dedupe.agent_id.clone(),
        target: Proposition::Holds {
            subject: Term::Object(subject()),
            dimension: Term::Dimension(dedupe.dimension_id.clone()),
            condition: Condition::Above(Term::Literal(Literal::Number(THRESHOLD))),
        },
        priority: GoalPriority {
            urgency: PRIORITY_URGENCY,
            cost_ceiling: None,
        },
        source: GoalSource::BeliefDivergence {
            dimension: dedupe.dimension_id.clone(),
            observed: "confidence=0.2".to_string(),
            desired: "confidence>0.7".to_string(),
        },
        lifecycle: GoalLifecycle::Active,
    }
}

#[test]
fn agent_public_imports_compile() {
    let (_temp_dir, store) = agent_store();
    let query = AgentQuery::new(&store);
    assert!(query.agent("missing").unwrap().is_none());
}

#[test]
fn agent_module_boundary_has_no_mod_rs() {
    assert!(!std::path::Path::new("src/agent/mod.rs").exists());
}

#[test]
fn agent_source_scans_reject_execution_internals_and_private_store_imports() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in [
        "src/agent.rs",
        "src/agent/contracts.rs",
        "src/agent/curation.rs",
        "src/agent/query.rs",
        "src/agent/registration.rs",
        "src/agent/subscription.rs",
    ] {
        let source = std::fs::read_to_string(manifest_dir.join(path)).unwrap();
        assert!(!source.contains("meld_execution"));
        assert!(!source.contains("docs_freshness"));
        assert!(!source.contains("seed.docs_freshness"));
        assert!(!source.contains("default_policy"));
    }
    for path in [
        "src/lib.rs",
        "src/belief.rs",
        "src/planner.rs",
        "src/world_state.rs",
    ] {
        let source = std::fs::read_to_string(manifest_dir.join(path)).unwrap();
        assert!(!source.contains("agent::store"));
    }
}

#[test]
fn agent_contracts_round_trip_and_validate() {
    let (_temp_dir, store) = agent_store();
    let (agent, subscription) = setup_agent(&store);
    let activation = AgentActivationRecord {
        activation_id: "activation-a".to_string(),
        agent_id: agent.agent_id.clone(),
        started_at_seq: 3,
        status: AgentActivationStatus::Started,
        last_error: None,
        lease_id: None,
    };
    let input = curation_input(0.2);
    let outcome = curate_threshold_rule(input).unwrap();
    for json in [
        serde_json::to_string(&agent).unwrap(),
        serde_json::to_string(&subscription).unwrap(),
        serde_json::to_string(&activation).unwrap(),
        serde_json::to_string(&outcome.decision).unwrap(),
        serde_json::to_string(&outcome.goal_command).unwrap(),
    ] {
        assert!(!json.is_empty());
    }
    assert!(agent.validate().is_ok());
    assert!(subscription.validate().is_ok());
    assert!(activation.validate().is_ok());
    assert!(outcome.decision.validate().is_ok());
}

#[test]
fn agent_contract_index_keys_are_stable() {
    assert_eq!(AgentStatus::Registered.index_key(), "registered");
    assert_eq!(AgentStatus::Operational.index_key(), "operational");
    assert_eq!(AgentStatus::Suspended.index_key(), "suspended");
    assert_eq!(AgentSubscriptionStatus::Active.index_key(), "active");
    assert_eq!(AgentSubscriptionStatus::Suspended.index_key(), "suspended");

    let key = belief_key();
    assert_eq!(
        AgentSubscriptionRecord::natural_key(AGENT_ID, &key),
        format!("{AGENT_ID}::{}", key.index_key())
    );
}

#[test]
fn agent_contract_validation_rejects_invalid_public_shapes() {
    let (_temp_dir, store) = agent_store();
    let (mut agent, mut subscription) = setup_agent(&store);
    agent.directive.clear();
    assert!(agent.validate().is_err());
    subscription.subscription_id.clear();
    assert!(subscription.validate().is_err());

    let mut activation = AgentActivationRecord {
        activation_id: "activation-a".to_string(),
        agent_id: AGENT_ID.to_string(),
        started_at_seq: 1,
        status: AgentActivationStatus::Started,
        last_error: None,
        lease_id: None,
    };
    activation.activation_id.clear();
    assert!(activation.validate().is_err());

    let mut dedupe = AgentCurationDedupeKey::threshold_rule(
        AGENT_ID,
        &subject(),
        &BranchScope::main(),
        &rule_config(),
    );
    dedupe.subject_key.clear();
    assert!(dedupe.validate().is_err());

    let mut rule = rule_config();
    rule.threshold = 1.2;
    assert!(rule.validate().is_err());

    let mut input_refs = curation_input(0.2).input_refs;
    input_refs.planner_projection_version.clear();
    assert!(input_refs.validate().is_err());

    let mut decision = curate_threshold_rule(curation_input(0.2)).unwrap().decision;
    decision.reason.clear();
    assert!(decision.validate().is_err());

    let mut subscribe = SubscribeAgentCommand {
        agent_id: AGENT_ID.to_string(),
        belief_key: belief_key(),
        created_at_seq: 1,
    };
    subscribe.agent_id.clear();
    assert!(subscribe.validate().is_err());

    let mut advance = meld_world_model::AdvanceSubscriptionCommand {
        agent_id: AGENT_ID.to_string(),
        subscription_id: "subscription-a".to_string(),
        delivered_revision_id: "revision-a".to_string(),
        delivered_seq: 7,
    };
    advance.delivered_revision_id.clear();
    assert!(advance.validate().is_err());

    let mut delivery = AgentDelivery {
        agent_id: AGENT_ID.to_string(),
        subscription_id: "subscription-a".to_string(),
        belief_revision_id: "revision-a".to_string(),
        revision_seq: 7,
    };
    delivery.belief_revision_id.clear();
    assert!(delivery.validate().is_err());
}

#[test]
fn agent_goal_mutation_command_validate_rejects_invalid_fields() {
    let cases = [
        invalid_goal_mutation_command(|command| command.command_id.clear()),
        invalid_goal_mutation_command(|command| command.agent_id.clear()),
        invalid_goal_mutation_command(|command| command.goal_id.clear()),
        invalid_goal_mutation_command(|command| command.review_seq = 0),
        invalid_goal_mutation_command(|command| command.projection_version.clear()),
        invalid_goal_mutation_command(|command| {
            command.dedupe_key.agent_id = "other-agent".to_string();
        }),
        invalid_goal_mutation_command(|command| {
            command.kind = AgentGoalMutationKind::Satisfy { at_seq: 21 };
        }),
    ];

    for command in cases {
        assert!(command.validate().is_err());
    }
}

#[test]
fn agent_goal_command_ids_are_stable_from_dedupe_identity() {
    let outcome = curate_threshold_rule(curation_input(0.2)).unwrap();
    let command = outcome.goal_command.as_ref().expect("goal command");
    let condition_key =
        serde_json::to_string(&Condition::Above(Term::Literal(Literal::Number(THRESHOLD))))
            .unwrap();
    let dedupe_key = format!(
        "{AGENT_ID}::{}::main::{DIMENSION_ID}::{condition_key}::belief_divergence",
        subject().index_key()
    );
    let decision_key = format!("{}::{:?}", dedupe_key, Some("revision-a".to_string()));

    assert_eq!(command.dedupe_key.index_key(), dedupe_key);
    assert_eq!(
        outcome.decision.decision_id,
        expected_deterministic_id("decision", &decision_key)
    );
    assert_eq!(
        command.command_id,
        expected_deterministic_id("goal-command", &decision_key)
    );
    assert_eq!(
        command.goal.goal_id,
        expected_deterministic_id("goal", &dedupe_key)
    );
}

#[test]
fn agent_satisfaction_curation_emits_mutation_for_satisfied_goal() {
    let input = satisfaction_input(0.95, 22);
    let goal = input.active_goals.goals[0].clone();
    let dedupe_key = AgentCurationDedupeKey::threshold_rule(
        AGENT_ID,
        &subject(),
        &BranchScope::main(),
        &rule_config(),
    );
    let decision_key = format!(
        "{}::satisfy::{}::{}",
        dedupe_key.index_key(),
        goal.goal_id,
        22
    );

    let outcome = curate_goal_satisfaction(input).unwrap();

    assert_eq!(
        outcome.decision.decision,
        AgentDecisionKind::GoalMutationCommand
    );
    let command = outcome.goal_mutation_command.unwrap();
    assert_eq!(command.kind, AgentGoalMutationKind::Satisfy { at_seq: 22 });
    assert_eq!(
        command.command_id,
        expected_deterministic_id("goal-mutation-command", &decision_key)
    );
    assert_eq!(
        outcome.decision.goal_mutation_command_id.as_deref(),
        Some(command.command_id.as_str())
    );
    assert_eq!(command.planner_source_refs, vec!["source-a".to_string()]);
    assert_eq!(command.planner_warnings, vec!["warning-a".to_string()]);
}

#[test]
fn agent_satisfaction_curation_leaves_unsatisfied_goal_active() {
    let outcome = curate_goal_satisfaction(satisfaction_input(0.2, 22)).unwrap();

    assert!(outcome.goal_mutation_command.is_none());
    assert_eq!(outcome.decision.decision, AgentDecisionKind::Absorbed);
    assert!(outcome.decision.reason.contains("unsatisfied"));
}

#[test]
fn agent_satisfaction_curation_does_not_satisfy_indeterminate_goal() {
    let mut input = satisfaction_input(0.95, 22);
    input.planner_projection.world_state = WorldState::empty();

    let outcome = curate_goal_satisfaction(input).unwrap();

    assert!(outcome.goal_mutation_command.is_none());
    assert_eq!(outcome.decision.decision, AgentDecisionKind::Indeterminate);
    assert!(outcome.decision.reason.contains("indeterminate"));
}

#[test]
fn agent_satisfaction_curation_is_deterministic_for_same_review_seq() {
    let input = satisfaction_input(0.95, 22);

    let first = curate_goal_satisfaction(input.clone()).unwrap();
    let replayed = curate_goal_satisfaction(input).unwrap();

    assert_eq!(first.decision, replayed.decision);
    assert_eq!(first.goal_mutation_command, replayed.goal_mutation_command);
}

#[test]
fn active_goal_summary_requires_active_matching_goal_from_same_agent() {
    let dedupe = AgentCurationDedupeKey::threshold_rule(
        AGENT_ID,
        &subject(),
        &BranchScope::main(),
        &rule_config(),
    );
    let matching = matching_active_goal_for(&dedupe);
    assert!(ActiveGoalSummary {
        goals: vec![matching.clone()]
    }
    .has_matching_goal(&dedupe));

    let mut satisfied = matching.clone();
    satisfied.lifecycle = GoalLifecycle::Satisfied { at_seq: 9 };
    assert!(!ActiveGoalSummary {
        goals: vec![satisfied]
    }
    .has_matching_goal(&dedupe));

    let mut other_agent = matching.clone();
    other_agent.agent_id = "other-agent".to_string();
    assert!(!ActiveGoalSummary {
        goals: vec![other_agent]
    }
    .has_matching_goal(&dedupe));

    let mut other_subject = matching.clone();
    other_subject.target = Proposition::Holds {
        subject: Term::Object(object("workspace_fs", "node", "node-b")),
        dimension: Term::Dimension(DIMENSION_ID.to_string()),
        condition: Condition::Above(Term::Literal(Literal::Number(THRESHOLD))),
    };
    assert!(!ActiveGoalSummary {
        goals: vec![other_subject]
    }
    .has_matching_goal(&dedupe));

    let mut other_dimension = matching.clone();
    other_dimension.target = Proposition::Holds {
        subject: Term::Object(subject()),
        dimension: Term::Dimension("other_dimension".to_string()),
        condition: Condition::Above(Term::Literal(Literal::Number(THRESHOLD))),
    };
    assert!(!ActiveGoalSummary {
        goals: vec![other_dimension]
    }
    .has_matching_goal(&dedupe));

    let mut other_condition = matching;
    other_condition.target = Proposition::Holds {
        subject: Term::Object(subject()),
        dimension: Term::Dimension(DIMENSION_ID.to_string()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.9))),
    };
    assert!(!ActiveGoalSummary {
        goals: vec![other_condition]
    }
    .has_matching_goal(&dedupe));
}

#[test]
fn agent_decision_serializes_input_refs_as_single_input_source() {
    let outcome = curate_threshold_rule(curation_input(0.2)).unwrap();
    let decision = serde_json::to_value(&outcome.decision).unwrap();
    let input_refs = decision.get("input_refs").expect("input refs");

    assert!(decision.get("belief_revision_id").is_none());
    assert!(decision.get("belief_key").is_none());
    assert!(decision.get("projection_version").is_none());
    assert_eq!(
        input_refs
            .get("belief_revision_id")
            .and_then(serde_json::Value::as_str),
        Some("revision-a")
    );
    assert!(input_refs.get("belief_key").is_some());
    assert_eq!(
        input_refs
            .get("planner_projection_version")
            .and_then(serde_json::Value::as_str),
        Some(PLANNER_PROJECTION_VERSION)
    );
}

#[test]
fn agent_curation_deserializes_without_goal_mutation_fields() {
    let outcome = curate_threshold_rule(curation_input(0.2)).unwrap();
    let mut decision_value = serde_json::to_value(&outcome.decision).unwrap();
    decision_value
        .as_object_mut()
        .unwrap()
        .remove("goal_mutation_command_id");
    let decision: meld_world_model::AgentCurationDecision =
        serde_json::from_value(decision_value).unwrap();
    assert!(decision.goal_mutation_command_id.is_none());

    let mut outcome_value = serde_json::to_value(&outcome).unwrap();
    outcome_value
        .as_object_mut()
        .unwrap()
        .remove("goal_mutation_command");
    let decoded: meld_world_model::AgentCurationOutcome =
        serde_json::from_value(outcome_value).unwrap();
    assert!(decoded.goal_mutation_command.is_none());
}

#[test]
fn agent_validation_rejects_empty_ids() {
    let mut request = seed_agent_registration();
    request.agent_id.clear();
    assert!(request.validate().is_err());
}

#[test]
fn agent_seed_registration_and_subscription_are_idempotent() {
    let (_temp_dir, store) = agent_store();
    let registration = AgentRegistration::new(&store);
    let request = seed_agent_registration();
    let first = registration.register_seed_agent(request.clone()).unwrap();
    let second = registration.register_seed_agent(request).unwrap();
    assert_eq!(first, second);

    let command = SubscribeAgentCommand {
        agent_id: first.agent_id.clone(),
        belief_key: belief_key(),
        created_at_seq: 2,
    };
    let subscriptions = AgentSubscription::new(&store);
    let first_subscription = subscriptions.subscribe(command.clone()).unwrap();
    let second_subscription = subscriptions.subscribe(command).unwrap();
    assert_eq!(first_subscription, second_subscription);
    assert_eq!(
        AgentQuery::new(&store)
            .subscriptions(&first.agent_id)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn agent_subscription_rejects_missing_agent() {
    let (_temp_dir, store) = agent_store();
    let result = AgentSubscription::new(&store).subscribe(SubscribeAgentCommand {
        agent_id: "missing".to_string(),
        belief_key: belief_key(),
        created_at_seq: 1,
    });
    assert!(result.is_err());
}

#[test]
fn agent_store_query_ordering_and_reopen() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("agent");
    {
        let store = AgentStore::new(sled::open(&path).unwrap()).unwrap();
        let (agent, subscription) = setup_agent(&store);
        store
            .put_activation(&AgentActivationRecord {
                activation_id: "activation-a".to_string(),
                agent_id: agent.agent_id.clone(),
                started_at_seq: 4,
                status: AgentActivationStatus::Activated,
                last_error: None,
                lease_id: Some("lease-a".to_string()),
            })
            .unwrap();
        let outcome = curate_threshold_rule(curation_input(0.1)).unwrap();
        store.put_decision(&outcome.decision).unwrap();
        store.flush().unwrap();
        assert_eq!(
            AgentQuery::new(&store)
                .pending_subscriptions(&agent.agent_id)
                .unwrap(),
            vec![subscription]
        );
    }
    let reopened = AgentStore::new(sled::open(&path).unwrap()).unwrap();
    let query = AgentQuery::new(&reopened);
    assert!(query.agent(AGENT_ID).unwrap().is_some());
    assert_eq!(query.subscriptions(AGENT_ID).unwrap().len(), 1);
    assert_eq!(query.recent_decisions(AGENT_ID, 10).unwrap().len(), 1);
    assert_eq!(reopened.activations_for_agent(AGENT_ID).unwrap().len(), 1);
}

#[test]
fn agent_low_confidence_emits_ground_goal_command() {
    let input = curation_input(0.2);
    let initial_world_state = input.planner_projection.world_state.clone();
    let subject = input.agent.subject.clone();
    let outcome = curate_threshold_rule(input).unwrap();
    assert_eq!(outcome.decision.decision, AgentDecisionKind::GoalCommand);
    let command = outcome.goal_command.unwrap();
    assert!(command.goal.target.is_ground());
    assert_eq!(command.goal.lifecycle, GoalLifecycle::Proposed);
    assert!(matches!(
        command.goal.source,
        GoalSource::BeliefDivergence { .. }
    ));
    assert!(matches!(
        evaluate(&initial_world_state, &command.goal.target),
        EvalResult::Unsatisfied { .. }
    ));
    let satisfied = initial_world_state
        .apply(&[Effect::Update {
            subject: Term::Object(subject),
            dimension: Term::Dimension(DIMENSION_ID.to_string()),
            value: Term::Literal(Literal::Number(0.8)),
        }])
        .unwrap();
    assert_eq!(
        evaluate(&satisfied, &command.goal.target),
        EvalResult::Satisfied
    );
}

#[test]
fn agent_goal_command_validate_requires_proposed_lifecycle() {
    let command = invalid_goal_command(|command| {
        command.goal.lifecycle = GoalLifecycle::Active;
    });

    let error = command.validate().unwrap_err();

    assert!(error.to_string().contains("lifecycle must be proposed"));
}

#[test]
fn agent_goal_command_validate_rejects_dedupe_agent_mismatch() {
    let command = invalid_goal_command(|command| {
        command.dedupe_key.agent_id = "other-agent".to_string();
    });

    let error = command.validate().unwrap_err();

    assert!(error.to_string().contains("agent id"));
}

#[test]
fn agent_goal_command_validate_rejects_dedupe_subject_mismatch() {
    let command = invalid_goal_command(|command| {
        command.dedupe_key.subject_key = "workspace_fs::node::other-node".to_string();
    });

    let error = command.validate().unwrap_err();

    assert!(error.to_string().contains("goal target subject"));
}

#[test]
fn agent_goal_command_validate_rejects_dedupe_dimension_mismatch() {
    let command = invalid_goal_command(|command| {
        command.dedupe_key.dimension_id = "other_dimension".to_string();
    });

    let error = command.validate().unwrap_err();

    assert!(error.to_string().contains("goal target dimension"));
}

#[test]
fn agent_goal_command_validate_rejects_dedupe_condition_mismatch() {
    let command = invalid_goal_command(|command| {
        command.dedupe_key.target_condition_key = "other-condition".to_string();
    });

    let error = command.validate().unwrap_err();

    assert!(error.to_string().contains("goal target condition"));
}

#[test]
fn agent_high_confidence_absorbs() {
    let outcome = curate_threshold_rule(curation_input(0.9)).unwrap();
    assert_eq!(outcome.decision.decision, AgentDecisionKind::Absorbed);
    assert!(outcome.goal_command.is_none());
}

#[test]
fn agent_matching_active_goal_absorbs() {
    let mut input = curation_input(0.2);
    let dedupe = AgentCurationDedupeKey::threshold_rule(
        input.agent.agent_id.clone(),
        &input.agent.subject,
        &input.agent.branch_scope,
        &rule_config(),
    );
    let goal = Goal {
        goal_id: "goal-existing".to_string(),
        agent_id: input.agent.agent_id.clone(),
        target: Proposition::Holds {
            subject: Term::Object(input.agent.subject.clone()),
            dimension: Term::Dimension(DIMENSION_ID.to_string()),
            condition: Condition::Above(Term::Literal(Literal::Number(THRESHOLD))),
        },
        priority: GoalPriority {
            urgency: 50,
            cost_ceiling: None,
        },
        source: GoalSource::BeliefDivergence {
            dimension: dedupe.dimension_id,
            observed: "confidence=0.2".to_string(),
            desired: "confidence>0.7".to_string(),
        },
        lifecycle: GoalLifecycle::Active,
    };
    input.active_goals = ActiveGoalSummary { goals: vec![goal] };
    let outcome = curate_threshold_rule(input).unwrap();
    assert_eq!(outcome.decision.decision, AgentDecisionKind::Absorbed);
    assert!(outcome.goal_command.is_none());
}

#[test]
fn agent_delivery_is_idempotent_and_advances_cursor_after_decision() {
    let (_agent_temp, agent_store) = agent_store();
    let (_agent, subscription) = setup_agent(&agent_store);
    let belief_temp = tempfile::tempdir().unwrap();
    let belief_store =
        BeliefStore::new(sled::open(belief_temp.path().join("belief")).unwrap()).unwrap();
    belief_store
        .put_view(&test_view(0.2, "revision-a", 7))
        .unwrap();
    let belief_query = BeliefQuery::new(&belief_store);
    let (_graph_temp, graph_store) = seeded_graph();
    let traversal_query = TraversalQuery::new(&graph_store);
    let planner_query = PlannerQuery::new(BeliefQuery::new(&belief_store), traversal_query);
    let curation = meld_world_model::agent::AgentCuration::new(&agent_store);
    let delivery = AgentDelivery {
        agent_id: AGENT_ID.to_string(),
        subscription_id: subscription.subscription_id.clone(),
        belief_revision_id: "revision-a".to_string(),
        revision_seq: 7,
    };

    let first = curation
        .handle_delivery(
            delivery.clone(),
            &belief_query,
            &planner_query,
            ActiveGoalSummary::default(),
            rule_config(),
        )
        .unwrap()
        .unwrap();
    assert!(first.goal_command.is_some());
    let updated = agent_store
        .get_subscription(&subscription.subscription_id)
        .unwrap()
        .unwrap();
    assert_eq!(updated.last_delivered_seq, 7);

    let duplicate = curation
        .handle_delivery(
            delivery,
            &belief_query,
            &planner_query,
            ActiveGoalSummary::default(),
            rule_config(),
        )
        .unwrap();
    assert!(duplicate.is_none());
    assert_eq!(
        AgentQuery::new(&agent_store)
            .recent_decisions(AGENT_ID, 10)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn agent_satisfaction_review_persists_decision_before_returning_mutation() {
    let (agent_temp, agent_store) = agent_store();
    let (_agent, subscription) = setup_agent(&agent_store);
    let belief_temp = tempfile::tempdir().unwrap();
    let belief_store =
        BeliefStore::new(sled::open(belief_temp.path().join("belief")).unwrap()).unwrap();
    belief_store
        .put_view(&test_view(0.95, "revision-b", 22))
        .unwrap();
    let belief_query = BeliefQuery::new(&belief_store);
    let (_graph_temp, graph_store) = seeded_graph();
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(&belief_store),
        TraversalQuery::new(&graph_store),
    );
    let mut goal = low_confidence_goal_command().goal;
    goal.lifecycle = GoalLifecycle::Active;
    let review = satisfaction_review(&subscription, 22);
    let curation = AgentCuration::new(&agent_store);

    let outcome = curation
        .handle_satisfaction_review(
            review.clone(),
            &belief_query,
            &planner_query,
            ActiveGoalSummary { goals: vec![goal] },
        )
        .unwrap();

    assert_eq!(
        outcome.decision.decision,
        AgentDecisionKind::GoalMutationCommand
    );
    let command = outcome.goal_mutation_command.expect("mutation command");
    assert_eq!(
        outcome.decision.goal_mutation_command_id.as_deref(),
        Some(command.command_id.as_str())
    );
    let stored = AgentQuery::new(&agent_store)
        .decision_by_satisfaction_review(&review)
        .unwrap()
        .expect("stored satisfaction decision");
    assert_eq!(stored, outcome.decision);
    drop(curation);
    agent_store.flush().unwrap();
    drop(agent_store);

    let reopened = AgentStore::new(sled::open(agent_temp.path().join("agent")).unwrap()).unwrap();
    assert_eq!(
        AgentQuery::new(&reopened)
            .decision_by_satisfaction_review(&review)
            .unwrap(),
        Some(stored)
    );
}

#[test]
fn agent_satisfaction_review_replays_by_review_identity() {
    let (_agent_temp, agent_store) = agent_store();
    let (_agent, subscription) = setup_agent(&agent_store);
    let belief_temp = tempfile::tempdir().unwrap();
    let belief_store =
        BeliefStore::new(sled::open(belief_temp.path().join("belief")).unwrap()).unwrap();
    belief_store
        .put_view(&test_view(0.95, "revision-b", 22))
        .unwrap();
    let belief_query = BeliefQuery::new(&belief_store);
    let (_graph_temp, graph_store) = seeded_graph();
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(&belief_store),
        TraversalQuery::new(&graph_store),
    );
    let mut goal = low_confidence_goal_command().goal;
    goal.lifecycle = GoalLifecycle::Active;
    let active_goals = ActiveGoalSummary {
        goals: vec![goal.clone()],
    };
    let review = satisfaction_review(&subscription, 22);
    let curation = AgentCuration::new(&agent_store);

    let first = curation
        .handle_satisfaction_review(
            review.clone(),
            &belief_query,
            &planner_query,
            active_goals.clone(),
        )
        .unwrap();
    let retry_before_execution = curation
        .handle_satisfaction_review(review.clone(), &belief_query, &planner_query, active_goals)
        .unwrap();
    let retry_after_execution = curation
        .handle_satisfaction_review(
            review.clone(),
            &belief_query,
            &planner_query,
            ActiveGoalSummary::default(),
        )
        .unwrap();

    assert_eq!(
        retry_before_execution.goal_mutation_command,
        first.goal_mutation_command
    );
    assert_eq!(retry_after_execution.decision, first.decision);
    assert!(retry_after_execution.goal_mutation_command.is_none());
    assert_eq!(
        AgentQuery::new(&agent_store)
            .recent_decisions(AGENT_ID, 10)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn agent_satisfaction_review_uses_review_seq_not_belief_revision_for_dedupe() {
    let (_agent_temp, agent_store) = agent_store();
    let (_agent, subscription) = setup_agent(&agent_store);
    let belief_temp = tempfile::tempdir().unwrap();
    let belief_store =
        BeliefStore::new(sled::open(belief_temp.path().join("belief")).unwrap()).unwrap();
    belief_store
        .put_view(&test_view(0.95, "revision-b", 22))
        .unwrap();
    let belief_query = BeliefQuery::new(&belief_store);
    let (_graph_temp, graph_store) = seeded_graph();
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(&belief_store),
        TraversalQuery::new(&graph_store),
    );
    let mut goal = low_confidence_goal_command().goal;
    goal.lifecycle = GoalLifecycle::Active;
    let active_goals = ActiveGoalSummary { goals: vec![goal] };
    let curation = AgentCuration::new(&agent_store);

    let first = curation
        .handle_satisfaction_review(
            satisfaction_review(&subscription, 22),
            &belief_query,
            &planner_query,
            active_goals.clone(),
        )
        .unwrap();
    let second = curation
        .handle_satisfaction_review(
            satisfaction_review(&subscription, 23),
            &belief_query,
            &planner_query,
            active_goals,
        )
        .unwrap();

    assert_ne!(first.decision.decision_id, second.decision.decision_id);
    assert_eq!(
        AgentQuery::new(&agent_store)
            .recent_decisions(AGENT_ID, 10)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn agent_satisfaction_review_persists_absorbed_without_mutation() {
    let (_agent_temp, agent_store) = agent_store();
    let (_agent, subscription) = setup_agent(&agent_store);
    let belief_temp = tempfile::tempdir().unwrap();
    let belief_store =
        BeliefStore::new(sled::open(belief_temp.path().join("belief")).unwrap()).unwrap();
    belief_store
        .put_view(&test_view(0.2, "revision-c", 21))
        .unwrap();
    let belief_query = BeliefQuery::new(&belief_store);
    let (_graph_temp, graph_store) = seeded_graph();
    let planner_query = PlannerQuery::new(
        BeliefQuery::new(&belief_store),
        TraversalQuery::new(&graph_store),
    );
    let mut goal = low_confidence_goal_command().goal;
    goal.lifecycle = GoalLifecycle::Active;
    let review = satisfaction_review(&subscription, 21);

    let outcome = AgentCuration::new(&agent_store)
        .handle_satisfaction_review(
            review.clone(),
            &belief_query,
            &planner_query,
            ActiveGoalSummary { goals: vec![goal] },
        )
        .unwrap();

    assert_eq!(outcome.decision.decision, AgentDecisionKind::Absorbed);
    assert!(outcome.goal_mutation_command.is_none());
    assert_eq!(
        AgentQuery::new(&agent_store)
            .decision_by_satisfaction_review(&review)
            .unwrap(),
        Some(outcome.decision)
    );
}

#[test]
fn agent_cursor_regression_is_rejected() {
    let (_temp_dir, store) = agent_store();
    let (_agent, subscription) = setup_agent(&store);
    let commands = AgentSubscription::new(&store);
    commands
        .advance_subscription(meld_world_model::AdvanceSubscriptionCommand {
            agent_id: AGENT_ID.to_string(),
            subscription_id: subscription.subscription_id.clone(),
            delivered_revision_id: "revision-a".to_string(),
            delivered_seq: 10,
        })
        .unwrap();
    let result = commands.advance_subscription(meld_world_model::AdvanceSubscriptionCommand {
        agent_id: AGENT_ID.to_string(),
        subscription_id: subscription.subscription_id,
        delivered_revision_id: "revision-b".to_string(),
        delivered_seq: 9,
    });
    assert!(result.is_err());
}

#[test]
fn agent_replay_recomputes_same_decision() {
    let input = curation_input(0.2);
    let first = curate_threshold_rule(input.clone()).unwrap();
    let replayed = curate_threshold_rule(input).unwrap();
    assert_eq!(first.decision, replayed.decision);
    assert_eq!(first.goal_command, replayed.goal_command);
}

proptest! {
    #[test]
    fn agent_dedupe_key_is_stable(branch in "[a-z][a-z0-9_]{0,12}") {
        let branch_scope = BranchScope::new(branch).unwrap();
        let left = AgentCurationDedupeKey::threshold_rule("agent-a", &subject(), &branch_scope, &rule_config());
        let right = AgentCurationDedupeKey::threshold_rule("agent-a", &subject(), &branch_scope, &rule_config());
        prop_assert_eq!(left.index_key(), right.index_key());
        prop_assert!(left.validate().is_ok());
    }

    #[test]
    fn agent_threshold_rule_is_stable(confidence in 0.0f64..1.0f64) {
        let outcome = curate_threshold_rule(curation_input(confidence)).unwrap();
        if confidence < THRESHOLD {
            prop_assert_eq!(outcome.decision.decision, AgentDecisionKind::GoalCommand);
        } else {
            prop_assert_eq!(outcome.decision.decision, AgentDecisionKind::Absorbed);
        }
    }
}
