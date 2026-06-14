use meld::execution::{
    satisfy_request_from_agent_mutation, GoalMutationError, GoalMutationRequest,
};
use meld_events::DomainObjectRef;
use meld_execution::capability::{
    ArtifactSchemaVersionRange, CapabilityCatalog, CapabilityTypeContract, ExecutionClass,
    ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec, ScopeContract,
};
use meld_execution::goals::{
    GoalAcceptanceLifecycle, GoalAcceptanceRequest, GoalCommandMetadata, GoalCommandOutcome,
    GoalSetApi, PersistentGoalSetStore,
};
use meld_execution::planning::{
    MethodLibrary, PlanningRequest, PlanningResult, PlanningRuntime, PlanningWorldStateFrameRef,
    PlanningWorldStateRequest,
};
use meld_lang::{
    Composition, Condition, CostEstimate, Effect, GoalLifecycle, Literal, Method, Operator,
    Proposition, Resolution, SlotConstraint, Step, StepKind, Term, WorldState,
};
use meld_world_model::agent::{
    curate_goal_satisfaction, curate_threshold_rule, ActiveGoalSummary, AgentCurationInput,
    AgentCurationInputRefs, AgentCurationRuleConfig, AgentDecisionKind, AgentGoalMutationCommand,
    AgentGoalSatisfactionInput, AgentRegistration, AgentStore, AgentSubscription,
    SeedAgentRegistration, SubscribeAgentCommand,
};
use meld_world_model::belief::{
    BeliefProvenanceSummary, BranchScope, ContradictionState, FreshnessState, HydrationRefs,
    PlannerProjectionSummary, PosteriorSummary,
};
use meld_world_model::planner::{
    project_world_state, PlannerFieldProjectionConfig, PlannerProjectionContext,
    PlannerProjectionInput, PLANNER_PROJECTION_VERSION,
};
use meld_world_model::{AgentGoalCommand, BeliefKey, BeliefStatus, BeliefView, PerspectiveKey};

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

fn subject_term() -> Term {
    Term::Object(subject())
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

fn belief_key() -> BeliefKey {
    BeliefKey {
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
            objects: vec![subject()],
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
        active_goals: ActiveGoalSummary { goals: vec![goal] },
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

fn planning_runtime() -> PlanningRuntime {
    let catalog = capability_catalog();
    let library = MethodLibrary::from_methods(vec![docs_method()], &catalog);
    PlanningRuntime::new(library, catalog)
}

fn capability_catalog() -> CapabilityCatalog {
    let mut catalog = CapabilityCatalog::new();
    catalog
        .register(CapabilityTypeContract {
            capability_type_id: "docs.write".to_string(),
            capability_version: 1,
            owning_domain: "docs".to_string(),
            scope_contract: ScopeContract {
                scope_kind: "filesystem".to_string(),
                scope_ref_kind: "node_id".to_string(),
                allow_fan_out: false,
            },
            binding_contract: vec![],
            input_contract: vec![InputSlotSpec {
                slot_id: "source".to_string(),
                accepted_artifact_type_ids: vec!["source_doc".to_string()],
                schema_versions: ArtifactSchemaVersionRange { min: 1, max: 1 },
                required: false,
                cardinality: InputCardinality::One,
            }],
            output_contract: vec![OutputSlotSpec {
                slot_id: "patch".to_string(),
                artifact_type_id: "docs_patch".to_string(),
                schema_version: 1,
                guaranteed: true,
            }],
            effect_contract: vec![],
            execution_contract: ExecutionContract {
                execution_class: ExecutionClass::Queued,
                completion_semantics: "result_or_failure".to_string(),
                retry_class: "provider_io".to_string(),
                cancellation_supported: true,
            },
        })
        .unwrap();
    catalog
}

fn docs_method() -> Method {
    Method {
        method_id: "refresh".to_string(),
        trigger: Proposition::Holds {
            subject: Term::Variable("?node".to_string()),
            dimension: Term::Dimension(DIMENSION_ID.to_string()),
            condition: Condition::Above(Term::Literal(Literal::Number(THRESHOLD))),
        },
        preconditions: vec![Proposition::Accessible {
            scope: Term::Variable("?node".to_string()),
        }],
        composition: Composition {
            steps: vec![Step {
                step_id: "write".to_string(),
                kind: StepKind::Op(Operator {
                    operator_id: "write".to_string(),
                    preconditions: vec![Proposition::Accessible {
                        scope: Term::Variable("?node".to_string()),
                    }],
                    effects: vec![Effect::Update {
                        subject: Term::Variable("?node".to_string()),
                        dimension: Term::Dimension(DIMENSION_ID.to_string()),
                        value: Term::Literal(Literal::Number(0.95)),
                    }],
                    cost: CostEstimate {
                        time_ms: 10_000,
                        money_microdollars: 25_000,
                        provider_calls: 1,
                    },
                    resolution: Resolution {
                        requires_inputs: vec![],
                        requires_outputs: vec![SlotConstraint {
                            artifact_type: Term::ArtifactType("docs_patch".to_string()),
                            required: true,
                        }],
                        scope_kind: Some("filesystem".to_string()),
                        tags: vec!["docs".to_string()],
                        specific: None,
                    },
                }),
            }],
            edges: vec![],
        },
        net_effects: vec![Effect::Update {
            subject: Term::Variable("?node".to_string()),
            dimension: Term::Dimension(DIMENSION_ID.to_string()),
            value: Term::Literal(Literal::Number(0.95)),
        }],
        cost: CostEstimate {
            time_ms: 10_000,
            money_microdollars: 25_000,
            provider_calls: 1,
        },
        preference: 1,
    }
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

    let active_goal = store.active_goal(&goal_id).unwrap().expect("active goal");
    assert_eq!(active_goal.lifecycle, GoalLifecycle::Active);

    let below = curate_goal_satisfaction(satisfaction_input_for_goal(
        active_goal.clone(),
        world_state_below_threshold(),
        21,
    ))
    .unwrap();
    assert!(below.goal_mutation_command.is_none());
    assert!(store.active_goal(&goal_id).unwrap().is_some());

    let satisfied_input =
        satisfaction_input_for_goal(active_goal.clone(), world_state_with_confidence(0.95), 22);
    let satisfied = curate_goal_satisfaction(satisfied_input.clone()).unwrap();
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

    let replayed = curate_goal_satisfaction(satisfied_input).unwrap();
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
