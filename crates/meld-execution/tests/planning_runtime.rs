use meld_events::DomainObjectRef;
use meld_execution::capability::{
    ArtifactSchemaVersionRange, CapabilityCatalog, CapabilityTypeContract, ExecutionClass,
    ExecutionContract, InputCardinality, InputSlotSpec, OutputSlotSpec, ScopeContract,
};
use meld_execution::goals::{
    AddGoalCommand, ExecutionGoalRecord, GoalCommandMetadata, GoalPlanningClaim,
    GoalPlanningSelection, GoalPlanningSelectionError, GoalPlanningSelectionPort,
    PersistentGoalSetStore, SuspendGoalCommand,
};
use meld_execution::planning::{
    CandidateStatus, ExecutionCompositionLowerer, MethodLibrary, MethodSourceRef,
    MethodVerification, PlanningDiagnosticCode, PlanningInputError, PlanningPerspectiveRef,
    PlanningProjectionError, PlanningRequest, PlanningResult, PlanningRuntime,
    PlanningRuntimeActor, PlanningRuntimeActorGoalResult, PlanningRuntimeActorRequest,
    PlanningWorldStateFrameRef, PlanningWorldStateProjection, PlanningWorldStateRequest,
    VerifiedMethodEntry,
};
use meld_execution::task::TaskCompiler;
use meld_execution::task_network::state::{DependencyEdge, DependencyKind};
use meld_execution::task_network::store::TaskNetworkStoreFactory;
use meld_execution::task_network::{
    mutation::{Inject, Mutation, ReadPrecondition, Rejection, Set},
    Command as TaskNetworkCommand, CommandRequest, JournalRecord, NetworkState, Response,
    TaskNetworkAuthority, TaskNetworkAuthorityPorts, TaskNetworkCommandPort,
};
use meld_lang::{
    Composition, Condition, CostEstimate, Effect, Goal, GoalLifecycle, GoalPriority, GoalSource,
    Literal, Method, Operator, Proposition, Resolution, SlotConstraint, Step, StepKind, Term,
    WorldState,
};

fn node(id: &str) -> Term {
    Term::Object(DomainObjectRef::new("workspace", "node", id).unwrap())
}

fn docs_target(scope: Term) -> Proposition {
    Proposition::Holds {
        subject: scope,
        dimension: Term::Dimension("docs_freshness".to_string()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
    }
}

fn goal_with_ceiling(ceiling: Option<CostEstimate>) -> Goal {
    Goal {
        goal_id: "goal-docs".to_string(),
        agent_id: "docs-agent".to_string(),
        target: docs_target(node("readme")),
        priority: GoalPriority {
            urgency: 1,
            cost_ceiling: ceiling,
        },
        source: GoalSource::BeliefDivergence {
            dimension: "docs_freshness".to_string(),
            observed: "low confidence".to_string(),
            desired: "fresh docs".to_string(),
        },
        lifecycle: GoalLifecycle::Active,
    }
}

fn derived_frame(
    request: &PlanningWorldStateRequest,
    world_state: &WorldState,
) -> PlanningWorldStateFrameRef {
    PlanningWorldStateFrameRef::from_authority(
        format!("frame-{}", request.goal_id),
        format!("projection-request-{}", request.goal_id),
        request.canonical_hash().unwrap(),
        "world_model.planner.v1",
        format!("projection-hash-{}", request.goal_id),
        meld_execution::planning::world_state::canonical_world_state_hash(world_state).unwrap(),
        request,
        world_state,
        vec!["source".to_string()],
        Vec::new(),
    )
    .unwrap()
}

fn request(goal: Goal, world_state: WorldState) -> PlanningRequest {
    let subject = match &goal.target {
        Proposition::Holds {
            subject: Term::Object(subject),
            ..
        } => subject.clone(),
        _ => panic!("planning fixture goal must carry one object subject"),
    };
    let world_state_request = PlanningWorldStateRequest {
        goal_id: goal.goal_id.clone(),
        agent_id: goal.agent_id.clone(),
        subject,
        source_seq: 1,
        target: goal.target.clone(),
        perspective: PlanningPerspectiveRef::new("agent", "default").unwrap(),
        branch_id: "main".to_string(),
        requested_dimensions: vec!["docs_freshness".to_string()],
        required_preconditions: vec![],
    };
    let world_state_frame = derived_frame(&world_state_request, &world_state);
    PlanningRequest {
        request_id: "request-1".to_string(),
        world_state_request,
        goal,
        world_state,
        world_state_frame,
    }
}

fn catalog() -> CapabilityCatalog {
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

fn docs_method(method_id: &str) -> Method {
    Method {
        method_id: method_id.to_string(),
        trigger: docs_target(Term::Variable("?node".to_string())),
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
                        dimension: Term::Dimension("docs_freshness".to_string()),
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
            dimension: Term::Dimension("docs_freshness".to_string()),
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

fn runtime(methods: Vec<Method>) -> PlanningRuntime {
    let catalog = catalog();
    let library = MethodLibrary::from_methods(methods, &catalog);
    PlanningRuntime::new(library, catalog)
}

fn planning_actor() -> PlanningRuntimeActor<TaskCompiler> {
    PlanningRuntimeActor::new(
        runtime(vec![docs_method("refresh")]),
        ExecutionCompositionLowerer::new(TaskCompiler::new(), catalog()),
    )
}

fn actor_request(limit: Option<usize>) -> PlanningRuntimeActorRequest {
    PlanningRuntimeActorRequest {
        network_id: "network-docs".to_string(),
        perspective: PlanningPerspectiveRef::new("agent", "default").unwrap(),
        branch_id: "main".to_string(),
        requested_dimensions: vec!["docs_freshness".to_string()],
        required_preconditions: vec![],
        limit,
    }
}

fn open_goal_store_with_active_goal(goal: Goal) -> PersistentGoalSetStore {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let store = PersistentGoalSetStore::new(db).unwrap();
    store
        .add_goal(AddGoalCommand {
            metadata: GoalCommandMetadata {
                command_id: format!("command-add-{}", goal.goal_id),
                source_identity: None,
                seq: 1,
            },
            goal,
        })
        .unwrap();
    store
}

struct TaskNetworkHarness {
    authority: TaskNetworkAuthority,
    ports: TaskNetworkAuthorityPorts,
    _temp: tempfile::TempDir,
}

struct StaleRaceGoalStore<'a> {
    inner: &'a PersistentGoalSetStore,
    command_port: TaskNetworkCommandPort,
    winner: std::sync::Mutex<Option<CommandRequest>>,
}

impl GoalPlanningSelectionPort for StaleRaceGoalStore<'_> {
    fn claim_active_goals(
        &self,
        limit: usize,
    ) -> Result<GoalPlanningSelection, GoalPlanningSelectionError> {
        self.inner.claim_active_goals(limit)
    }

    fn release_goal_claim(
        &self,
        claim: &GoalPlanningClaim,
    ) -> Result<(), GoalPlanningSelectionError> {
        self.inner.release_goal_claim(claim)
    }

    fn planning_goal_record(
        &self,
        goal_id: &str,
    ) -> Result<Option<ExecutionGoalRecord>, GoalPlanningSelectionError> {
        if let Some(winner) = self.winner.lock().unwrap().take() {
            let response = self
                .command_port
                .try_submit(winner)
                .map_err(|error| GoalPlanningSelectionError::TransientStorage(error.to_string()))?;
            assert!(matches!(response, Response::Accepted { revision: 1, .. }));
        }
        self.inner.planning_goal_record(goal_id)
    }
}

impl TaskNetworkHarness {
    fn state(&self) -> NetworkState {
        self.ports.query().state().unwrap()
    }

    fn journal(&self) -> Vec<JournalRecord> {
        self.ports.query().journal().unwrap()
    }

    fn shutdown(&mut self) {
        self.authority.shutdown().unwrap();
    }
}

fn open_task_network_store() -> TaskNetworkHarness {
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    let authority = TaskNetworkAuthority::open(&factory, "network-docs", 32).unwrap();
    let ports = authority.ports();
    TaskNetworkHarness {
        authority,
        ports,
        _temp: temp,
    }
}

fn unsatisfied_state() -> WorldState {
    WorldState::new(vec![
        Proposition::Accessible {
            scope: node("readme"),
        },
        Proposition::Holds {
            subject: node("readme"),
            dimension: Term::Dimension("docs_freshness".to_string()),
            condition: Condition::Equals(Term::Literal(Literal::Number(0.2))),
        },
    ])
    .unwrap()
}

#[test]
fn satisfied_goal_returns_satisfied() {
    let state = WorldState::new(vec![Proposition::Holds {
        subject: node("readme"),
        dimension: Term::Dimension("docs_freshness".to_string()),
        condition: Condition::Equals(Term::Literal(Literal::Number(0.9))),
    }])
    .unwrap();
    let result = runtime(vec![docs_method("refresh")])
        .plan_goal(request(goal_with_ceiling(None), state))
        .unwrap();

    assert!(matches!(result, PlanningResult::Satisfied(_)));
}

#[test]
fn missing_projection_data_returns_indeterminate() {
    let result = runtime(vec![docs_method("refresh")])
        .plan_goal(request(goal_with_ceiling(None), WorldState::empty()))
        .unwrap();

    assert!(matches!(result, PlanningResult::Indeterminate(_)));
}

#[test]
fn low_confidence_docs_goal_returns_composed() {
    let result = runtime(vec![docs_method("refresh")])
        .plan_goal(request(goal_with_ceiling(None), unsatisfied_state()))
        .unwrap();

    let PlanningResult::Composed(composed) = result else {
        panic!("expected composed");
    };
    assert_eq!(composed.method_id, "refresh");
    assert!(composed
        .composition_id
        .starts_with("execution-composition-"));
    assert_eq!(composed.projected_effects.len(), 1);
}

#[test]
fn no_trigger_match_returns_no_applicable_method() {
    let mut method = docs_method("other");
    method.trigger = Proposition::Accessible {
        scope: Term::Variable("?node".to_string()),
    };
    let result = runtime(vec![method])
        .plan_goal(request(goal_with_ceiling(None), unsatisfied_state()))
        .unwrap();

    let PlanningResult::NoApplicableMethod(report) = result else {
        panic!("expected no applicable method");
    };
    assert_eq!(report.candidates[0].status, CandidateStatus::TriggerMiss);
}

#[test]
fn failed_precondition_returns_no_applicable_method() {
    let state = WorldState::new(vec![Proposition::Holds {
        subject: node("readme"),
        dimension: Term::Dimension("docs_freshness".to_string()),
        condition: Condition::Equals(Term::Literal(Literal::Number(0.2))),
    }])
    .unwrap();
    let result = runtime(vec![docs_method("refresh")])
        .plan_goal(request(goal_with_ceiling(None), state))
        .unwrap();

    let PlanningResult::NoApplicableMethod(report) = result else {
        panic!("expected no applicable method");
    };
    assert_eq!(
        report.candidates[0].status,
        CandidateStatus::PreconditionsUnsatisfied
    );
}

#[test]
fn indeterminate_precondition_returns_no_applicable_method() {
    let mut method = docs_method("refresh");
    method.preconditions = vec![Proposition::Holds {
        subject: Term::Variable("?node".to_string()),
        dimension: Term::Dimension("reviewed".to_string()),
        condition: Condition::Above(Term::Literal(Literal::Number(0.7))),
    }];
    let result = runtime(vec![method])
        .plan_goal(request(goal_with_ceiling(None), unsatisfied_state()))
        .unwrap();

    let PlanningResult::NoApplicableMethod(report) = result else {
        panic!("expected no applicable method");
    };
    assert_eq!(
        report.candidates[0].status,
        CandidateStatus::PreconditionsIndeterminate
    );
}

#[test]
fn cost_ceiling_rejection_returns_no_applicable_method() {
    let result = runtime(vec![docs_method("refresh")])
        .plan_goal(request(
            goal_with_ceiling(Some(CostEstimate {
                time_ms: 1,
                money_microdollars: 1,
                provider_calls: 0,
            })),
            unsatisfied_state(),
        ))
        .unwrap();

    let PlanningResult::NoApplicableMethod(report) = result else {
        panic!("expected no applicable method");
    };
    assert_eq!(report.candidates[0].status, CandidateStatus::CostRejected);
}

#[test]
fn projected_effects_miss_returns_no_applicable_method() {
    let mut method = docs_method("refresh");
    method.net_effects = vec![Effect::Update {
        subject: Term::Variable("?node".to_string()),
        dimension: Term::Dimension("other_dimension".to_string()),
        value: Term::Literal(Literal::Number(0.95)),
    }];
    let result = runtime(vec![method])
        .plan_goal(request(goal_with_ceiling(None), unsatisfied_state()))
        .unwrap();

    let PlanningResult::NoApplicableMethod(report) = result else {
        panic!("expected no applicable method");
    };
    assert_eq!(report.candidates[0].status, CandidateStatus::EffectMiss);
}

#[test]
fn invalid_composition_after_substitution_returns_invalid_method() {
    let mut method = docs_method("refresh");
    method
        .composition
        .steps
        .push(method.composition.steps[0].clone());
    let catalog = catalog();
    let library = MethodLibrary {
        entries: vec![VerifiedMethodEntry {
            method,
            source_ref: MethodSourceRef::InMemory {
                label: "bypass".to_string(),
            },
            verification: MethodVerification {
                diagnostics: vec![],
                operator_resolutions: vec![],
            },
        }],
        invalid: vec![],
    };
    let result = PlanningRuntime::new(library, catalog)
        .plan_goal(request(goal_with_ceiling(None), unsatisfied_state()))
        .unwrap();

    let PlanningResult::InvalidMethod(report) = result else {
        panic!("expected invalid method");
    };
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == PlanningDiagnosticCode::CompositionValidationFailed
    }));
}

#[test]
fn repeated_same_input_returns_same_composition_and_diagnostics() {
    let runtime = runtime(vec![docs_method("refresh")]);
    let first = runtime
        .plan_goal(request(goal_with_ceiling(None), unsatisfied_state()))
        .unwrap();
    let second = runtime
        .plan_goal(request(goal_with_ceiling(None), unsatisfied_state()))
        .unwrap();

    assert_eq!(first, second);
}

#[test]
fn invalid_request_shape_returns_input_errors() {
    let runtime = runtime(vec![docs_method("refresh")]);

    let mut missing_id = request(goal_with_ceiling(None), unsatisfied_state());
    missing_id.request_id.clear();
    assert_eq!(
        runtime.plan_goal(missing_id).unwrap_err(),
        PlanningInputError::MissingRequestId
    );

    let mut proposed_goal = goal_with_ceiling(None);
    proposed_goal.lifecycle = GoalLifecycle::Proposed;
    assert!(matches!(
        runtime
            .plan_goal(request(proposed_goal, unsatisfied_state()))
            .unwrap_err(),
        PlanningInputError::NonActiveGoal { .. }
    ));

    let mut nonground_goal = goal_with_ceiling(None);
    nonground_goal.target = Proposition::Accessible {
        scope: Term::Variable("?node".to_string()),
    };
    let mut nonground_request = request(goal_with_ceiling(None), unsatisfied_state());
    nonground_request.goal = nonground_goal;
    assert!(matches!(
        runtime.plan_goal(nonground_request).unwrap_err(),
        PlanningInputError::NonGroundGoal { .. }
    ));
}

#[test]
fn planning_actor_reads_active_goals_and_submits_lowered_composition() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let expected_goal_sequence = goal_store.goal_records().unwrap()[0].updated_at_seq;
    let task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection_requests = Vec::new();
    let mut projection = |request: PlanningWorldStateRequest| {
        projection_requests.push(request.clone());
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };

    let report = actor
        .run_once(
            &goal_store,
            &task_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(projection_requests.len(), 1);
    assert_eq!(projection_requests[0].goal_id, "goal-docs");
    assert_eq!(projection_requests[0].source_seq, expected_goal_sequence);
    assert_eq!(
        projection_requests[0].subject,
        DomainObjectRef::new("workspace", "node", "readme").unwrap()
    );
    assert_eq!(report.actor_id, "execution.planning.runtime");
    assert_eq!(report.active_goal_count, 1);
    assert_eq!(report.attempted, 1);
    assert_eq!(report.committed, 1);
    assert!(report.retryable_errors.is_empty());
    assert!(report.fatal_errors.is_empty());
    assert!(!report.budget_exhausted);
    assert!(report.input_revision < report.output_revision.unwrap());
    assert_eq!(task_network.state().tasks.len(), 1);
    assert_eq!(task_network.journal().len(), 1);
    assert!(matches!(
        &report.results[0],
        PlanningRuntimeActorGoalResult::Submitted {
            response: Response::Accepted { .. },
            ..
        }
    ));
}

#[test]
fn planning_actor_repeated_tick_replays_exact_committed_command() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection = |request: PlanningWorldStateRequest| {
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };

    let first = actor
        .run_once(
            &goal_store,
            &task_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap();
    let first_output_revision = task_network.state().revision;
    let second = actor
        .run_once(
            &goal_store,
            &task_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(first.committed, 1);
    assert_eq!(second.committed, 1);
    assert!(second.retryable_errors.is_empty());
    assert!(second.fatal_errors.is_empty());
    assert_eq!(second.input_revision, first_output_revision);
    assert_eq!(second.output_revision, Some(first_output_revision));
    assert_eq!(task_network.journal().len(), 1);
    assert!(matches!(
        &second.results[0],
        PlanningRuntimeActorGoalResult::Submitted {
            response: Response::Accepted { .. },
            ..
        }
    ));
}

#[test]
fn planning_actor_replays_exact_authenticated_outcome_after_authority_reopen() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let actor = planning_actor();
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    let first_response;
    {
        let mut authority = TaskNetworkAuthority::open(&factory, "network-docs", 16).unwrap();
        let ports = authority.ports();
        let mut projection = |request: PlanningWorldStateRequest| {
            let world_state = unsatisfied_state();
            Ok(PlanningWorldStateProjection {
                frame: derived_frame(&request, &world_state),
                world_state,
            })
        };
        let report = actor
            .run_once(&goal_store, &ports, &mut projection, actor_request(None))
            .unwrap();
        let PlanningRuntimeActorGoalResult::Submitted { response, .. } = &report.results[0] else {
            panic!("initial planning pass must commit");
        };
        first_response = response.clone();
        authority.shutdown().unwrap();
    }

    let mut reopened = TaskNetworkAuthority::open(&factory, "network-docs", 16).unwrap();
    let mut projection = |request: PlanningWorldStateRequest| {
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };
    let replay = actor
        .run_once(
            &goal_store,
            &reopened.ports(),
            &mut projection,
            actor_request(None),
        )
        .unwrap();
    let PlanningRuntimeActorGoalResult::Submitted { response, .. } = &replay.results[0] else {
        panic!("reopened planning pass must replay its exact outcome");
    };

    assert_eq!(response, &first_response);
    assert_eq!(replay.committed, 1);
    assert!(replay.fatal_errors.is_empty());
    assert_eq!(reopened.query_port().head().unwrap().revision, 1);
    reopened.shutdown().unwrap();
}

#[test]
fn planning_actor_rejects_divergent_foreign_materialization() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let actor = planning_actor();
    let reference_network = open_task_network_store();
    let mut reference_projection = |request: PlanningWorldStateRequest| {
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };
    let reference = actor
        .run_once(
            &goal_store,
            &reference_network.ports,
            &mut reference_projection,
            actor_request(None),
        )
        .unwrap();
    let PlanningRuntimeActorGoalResult::Submitted { plan, .. } = &reference.results[0] else {
        panic!("reference planning pass must submit a task package");
    };
    let Mutation::Inject(planned_inject) = &plan.mutations.mutations[0];
    let mut divergent_node = planned_inject.task_node.clone();
    divergent_node.task_run_context.trigger = "foreign_materialization".to_string();

    let target_network = open_task_network_store();
    let target_state = target_network.state();
    let foreign = CommandRequest {
        command_id: "foreign-command".to_string(),
        network_id: target_state.network_id.clone(),
        base_revision: target_state.revision,
        base_state_hash: target_state.state_hash.clone(),
        read_preconditions: vec![],
        command: TaskNetworkCommand::ApplyMutationSet(Set::new(
            target_state.network_id,
            "foreign-composition",
            "foreign-once",
            vec![Mutation::Inject(Inject::new(
                divergent_node,
                planned_inject.incoming_edges.clone(),
            ))],
            vec![],
        )),
    };
    assert!(matches!(
        target_network.ports.commands().try_submit(foreign).unwrap(),
        Response::Accepted { revision: 1, .. }
    ));

    let mut target_projection = |request: PlanningWorldStateRequest| {
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };
    let report = actor
        .run_once(
            &goal_store,
            &target_network.ports,
            &mut target_projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(target_network.state().revision, 1);
    assert_eq!(report.committed, 0);
    assert!(report.retryable_errors.is_empty());
    assert_eq!(report.fatal_errors.len(), 1);
    assert_eq!(
        report.fatal_errors[0].code,
        "planning_materialization_diverged"
    );
    assert!(matches!(
        &report.results[0],
        PlanningRuntimeActorGoalResult::CommandFailed {
            retryable: false,
            ..
        }
    ));
}

#[test]
fn planning_actor_rejects_exact_foreign_materialization_without_command_outcome() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let actor = planning_actor();
    let reference_network = open_task_network_store();
    let mut projection = |request: PlanningWorldStateRequest| {
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };
    let reference = actor
        .run_once(
            &goal_store,
            &reference_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap();
    let PlanningRuntimeActorGoalResult::Submitted { plan, .. } = &reference.results[0] else {
        panic!("reference planning pass must submit a task package");
    };

    let target_network = open_task_network_store();
    let state = target_network.state();
    let foreign = CommandRequest {
        command_id: "foreign-exact-command".to_string(),
        network_id: state.network_id.clone(),
        base_revision: state.revision,
        base_state_hash: state.state_hash.clone(),
        read_preconditions: vec![
            ReadPrecondition::RevisionIs(state.revision),
            ReadPrecondition::StateHashIs(state.state_hash),
        ],
        command: TaskNetworkCommand::ApplyMutationSet(plan.mutations.clone()),
    };
    assert!(matches!(
        target_network.ports.commands().try_submit(foreign).unwrap(),
        Response::Accepted { .. }
    ));

    let report = actor
        .run_once(
            &goal_store,
            &target_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(report.committed, 0);
    assert_eq!(report.fatal_errors.len(), 1);
    assert_eq!(
        report.fatal_errors[0].code,
        "planning_materialization_foreign"
    );
    assert!(matches!(
        &report.results[0],
        PlanningRuntimeActorGoalResult::CommandFailed {
            retryable: false,
            ..
        }
    ));
}

#[test]
fn planning_actor_rejects_foreign_expected_command_product_after_reopen() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let actor = planning_actor();
    let reference_network = open_task_network_store();
    let mut projection = |request: PlanningWorldStateRequest| {
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };
    let reference = actor
        .run_once(
            &goal_store,
            &reference_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap();
    let PlanningRuntimeActorGoalResult::Submitted { command_id, .. } = &reference.results[0] else {
        panic!("reference planning pass must submit a task package");
    };

    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    {
        let mut authority = TaskNetworkAuthority::open(&factory, "network-docs", 16).unwrap();
        let ports = authority.ports();
        let head = ports.query().head().unwrap();
        let foreign = CommandRequest {
            command_id: command_id.clone(),
            network_id: head.network_id.clone(),
            base_revision: head.revision,
            base_state_hash: head.state_hash.clone(),
            read_preconditions: vec![
                ReadPrecondition::RevisionIs(head.revision),
                ReadPrecondition::StateHashIs(head.state_hash),
            ],
            command: TaskNetworkCommand::ApplyMutationSet(Set::empty(
                head.network_id,
                "foreign-composition",
                "foreign-command-product",
            )),
        };
        assert!(matches!(
            ports.commands().try_submit(foreign).unwrap(),
            Response::Accepted { .. }
        ));
        authority.shutdown().unwrap();
    }

    let mut reopened = TaskNetworkAuthority::open(&factory, "network-docs", 16).unwrap();
    let report = actor
        .run_once(
            &goal_store,
            &reopened.ports(),
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(report.committed, 0);
    assert_eq!(report.fatal_errors.len(), 1);
    assert_eq!(
        report.fatal_errors[0].code,
        "planning_command_outcome_diverged"
    );
    assert!(report.fatal_errors[0]
        .message
        .contains("foreign mutation set"));
    assert_eq!(reopened.query_port().head().unwrap().revision, 1);
    reopened.shutdown().unwrap();
}

#[test]
fn planning_actor_rejects_divergent_foreign_incoming_edges() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let actor = planning_actor();
    let reference_network = open_task_network_store();
    let mut reference_projection = |request: PlanningWorldStateRequest| {
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };
    let reference = actor
        .run_once(
            &goal_store,
            &reference_network.ports,
            &mut reference_projection,
            actor_request(None),
        )
        .unwrap();
    let PlanningRuntimeActorGoalResult::Submitted { plan, .. } = &reference.results[0] else {
        panic!("reference planning pass must submit a task package");
    };
    let Mutation::Inject(planned_inject) = &plan.mutations.mutations[0];
    let mut upstream_node = planned_inject.task_node.clone();
    upstream_node.task_instance_id = "foreign-upstream".to_string();
    upstream_node.task_run_context.task_run_id = "foreign-upstream-run".to_string();
    let foreign_edge = DependencyEdge {
        from: upstream_node.task_instance_id.clone(),
        to: planned_inject.task_node.task_instance_id.clone(),
        kind: DependencyKind::Ordering,
    };

    let target_network = open_task_network_store();
    let target_state = target_network.state();
    let foreign = CommandRequest {
        command_id: "foreign-edge-command".to_string(),
        network_id: target_state.network_id.clone(),
        base_revision: target_state.revision,
        base_state_hash: target_state.state_hash.clone(),
        read_preconditions: vec![],
        command: TaskNetworkCommand::ApplyMutationSet(Set::new(
            target_state.network_id,
            "foreign-edge-composition",
            "foreign-edge-once",
            vec![
                Mutation::Inject(Inject::new(upstream_node, vec![])),
                Mutation::Inject(Inject::new(
                    planned_inject.task_node.clone(),
                    vec![foreign_edge],
                )),
            ],
            vec![],
        )),
    };
    assert!(matches!(
        target_network.ports.commands().try_submit(foreign).unwrap(),
        Response::Accepted { revision: 1, .. }
    ));

    let mut target_projection = |request: PlanningWorldStateRequest| {
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };
    let report = actor
        .run_once(
            &goal_store,
            &target_network.ports,
            &mut target_projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(target_network.state().revision, 1);
    assert_eq!(report.committed, 0);
    assert_eq!(report.fatal_errors.len(), 1);
    assert!(report.fatal_errors[0]
        .message
        .contains("divergent incoming edges"));
    assert!(matches!(
        &report.results[0],
        PlanningRuntimeActorGoalResult::CommandFailed {
            retryable: false,
            ..
        }
    ));
}

#[test]
fn planning_actor_revalidates_goal_sequence_after_projection() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let mut task_network = open_task_network_store();
    let actor = planning_actor();
    let goal_writer = goal_store.clone();
    let mut projection = move |request: PlanningWorldStateRequest| {
        goal_writer
            .suspend_goal(SuspendGoalCommand {
                metadata: GoalCommandMetadata {
                    command_id: "command-suspend-goal-docs".to_string(),
                    source_identity: None,
                    seq: 2,
                },
                goal_id: request.goal_id.clone(),
                reason: "projection observed superseding work".to_string(),
            })
            .unwrap();
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };

    let report = actor
        .run_once(
            &goal_store,
            &task_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(task_network.state().revision, 0);
    assert_eq!(report.committed, 0);
    assert_eq!(report.retryable_errors.len(), 1);
    assert!(report.fatal_errors.is_empty());
    assert_eq!(report.retryable_errors[0].code, "planning_goal_fence_stale");
    assert!(matches!(
        &report.results[0],
        PlanningRuntimeActorGoalResult::CommandFailed {
            retryable: true,
            ..
        }
    ));
    task_network.shutdown();
}

#[test]
fn planning_actor_bounded_selection_uses_urgency_then_stable_goal_id() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let goal_store = PersistentGoalSetStore::new(db).unwrap();
    for (seq, goal_id, urgency) in [
        (1, "goal-later", 5),
        (2, "goal-urgent-b", 1),
        (3, "goal-urgent-a", 1),
    ] {
        let mut goal = goal_with_ceiling(None);
        goal.goal_id = goal_id.to_string();
        goal.priority.urgency = urgency;
        goal_store
            .add_goal(AddGoalCommand {
                metadata: GoalCommandMetadata {
                    command_id: format!("command-add-{goal_id}"),
                    source_identity: None,
                    seq,
                },
                goal,
            })
            .unwrap();
    }
    let task_network = open_task_network_store();
    let actor = planning_actor();
    let mut selected_goal_ids = Vec::new();
    let mut projection = |request: PlanningWorldStateRequest| {
        selected_goal_ids.push(request.goal_id.clone());
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };

    let report = actor
        .run_once(
            &goal_store,
            &task_network.ports,
            &mut projection,
            actor_request(Some(2)),
        )
        .unwrap();

    assert_eq!(selected_goal_ids, vec!["goal-urgent-a", "goal-urgent-b"]);
    assert_eq!(report.attempted, 2);
    assert!(report.budget_exhausted);
}

#[test]
fn planning_actor_selection_cursor_survives_reopen_and_prevents_starvation() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let goal_store = PersistentGoalSetStore::new(db.clone()).unwrap();
    for (seq, goal_id, urgency) in [
        (1, "goal-later", 5),
        (2, "goal-urgent-b", 1),
        (3, "goal-urgent-a", 1),
    ] {
        let mut goal = goal_with_ceiling(None);
        goal.goal_id = goal_id.to_string();
        goal.priority.urgency = urgency;
        goal_store
            .add_goal(AddGoalCommand {
                metadata: GoalCommandMetadata {
                    command_id: format!("command-add-{goal_id}"),
                    source_identity: None,
                    seq,
                },
                goal,
            })
            .unwrap();
    }
    let task_network = open_task_network_store();
    let actor = planning_actor();
    let mut selected_goal_ids = Vec::new();
    let mut projection = |request: PlanningWorldStateRequest| {
        selected_goal_ids.push(request.goal_id.clone());
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };

    actor
        .run_once(
            &goal_store,
            &task_network.ports,
            &mut projection,
            actor_request(Some(1)),
        )
        .unwrap();
    drop(goal_store);
    let reopened_goal_store = PersistentGoalSetStore::new(db).unwrap();
    for _ in 0..2 {
        actor
            .run_once(
                &reopened_goal_store,
                &task_network.ports,
                &mut projection,
                actor_request(Some(1)),
            )
            .unwrap();
    }

    assert_eq!(
        selected_goal_ids,
        vec!["goal-urgent-a", "goal-urgent-b", "goal-later"]
    );
}

#[test]
fn planning_actor_rejects_unbounded_or_empty_goal_limits() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection_called = false;
    let mut projection = |_request: PlanningWorldStateRequest| {
        projection_called = true;
        Err(PlanningProjectionError::fatal("must not be called"))
    };

    for limit in [0, 1_025] {
        let error = actor
            .run_once(
                &goal_store,
                &task_network.ports,
                &mut projection,
                actor_request(Some(limit)),
            )
            .unwrap_err();

        assert!(matches!(
            error,
            meld_execution::planning::PlanningRuntimeActorError::InvalidRequest(_)
        ));
    }
    assert!(!projection_called);
    assert_eq!(task_network.state().revision, 0);
}

#[test]
fn planning_actor_rejects_oversized_projection_request_before_port_handoff() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection_called = false;
    let mut projection = |_request: PlanningWorldStateRequest| {
        projection_called = true;
        Err(PlanningProjectionError::fatal("must not be called"))
    };
    let mut request = actor_request(None);
    request.requested_dimensions = vec!["dimension".to_string(); 4_097];

    let error = actor
        .run_once(&goal_store, &task_network.ports, &mut projection, request)
        .unwrap_err();

    assert!(matches!(
        error,
        meld_execution::planning::PlanningRuntimeActorError::InvalidRequest(_)
    ));
    assert!(!projection_called);
    assert_eq!(task_network.state().revision, 0);
}

#[test]
fn planning_actor_rejects_oversized_projection_product_before_planning() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection = |request: PlanningWorldStateRequest| {
        let world_state = unsatisfied_state();
        let mut frame = derived_frame(&request, &world_state);
        frame.source_refs = vec!["source".to_string(); 4_097];
        Ok(PlanningWorldStateProjection { world_state, frame })
    };

    let report = actor
        .run_once(
            &goal_store,
            &task_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(report.committed, 0);
    assert!(report.retryable_errors.is_empty());
    assert_eq!(report.fatal_errors.len(), 1);
    assert_eq!(
        report.fatal_errors[0].code,
        "planning_projection_bounds_exceeded"
    );
    assert!(matches!(
        &report.results[0],
        PlanningRuntimeActorGoalResult::ProjectionFailed {
            retryable: false,
            ..
        }
    ));
    assert_eq!(task_network.state().revision, 0);
}

#[test]
fn planning_actor_uses_bounded_default_when_limit_is_omitted() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let goal_store = PersistentGoalSetStore::new(db).unwrap();
    for index in 0..65 {
        let mut goal = goal_with_ceiling(None);
        goal.goal_id = format!("goal-{index:02}");
        goal_store
            .add_goal(AddGoalCommand {
                metadata: GoalCommandMetadata {
                    command_id: format!("command-{index:02}"),
                    source_identity: None,
                    seq: index + 1,
                },
                goal,
            })
            .unwrap();
    }
    let task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection = |_request: PlanningWorldStateRequest| {
        Err(PlanningProjectionError::retryable("projection unavailable"))
    };

    let report = actor
        .run_once(
            &goal_store,
            &task_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(report.active_goal_count, 65);
    assert_eq!(report.attempted, 64);
    assert_eq!(report.results.len(), 64);
    assert!(report.budget_exhausted);
    assert_eq!(task_network.state().revision, 0);
}

#[test]
fn planning_actor_projection_failure_does_not_mutate_task_network_state() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let task_network = open_task_network_store();
    let input_revision = task_network.state().revision;
    let input_hash = task_network.state().state_hash.clone();
    let actor = planning_actor();
    let mut projection_requests = Vec::new();
    let mut projection = |request: PlanningWorldStateRequest| {
        projection_requests.push(request);
        Err(PlanningProjectionError::retryable("projection unavailable"))
    };

    let report = actor
        .run_once(
            &goal_store,
            &task_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert_eq!(projection_requests.len(), 1);
    assert_eq!(report.active_goal_count, 1);
    assert_eq!(report.attempted, 1);
    assert_eq!(report.committed, 0);
    assert_eq!(report.input_revision, input_revision);
    assert_eq!(report.output_revision, Some(input_revision));
    assert_eq!(task_network.state().revision, input_revision);
    assert_eq!(task_network.state().state_hash, input_hash);
    assert!(task_network.state().tasks.is_empty());
    assert!(task_network.journal().is_empty());
    assert_eq!(report.retryable_errors.len(), 1);
    assert!(report.fatal_errors.is_empty());
    assert!(matches!(
        &report.results[0],
        PlanningRuntimeActorGoalResult::ProjectionFailed {
            retryable: true,
            ..
        }
    ));
}

#[test]
fn planning_actor_rejects_ambiguous_projection_subject_before_port_handoff() {
    let first = DomainObjectRef::new("workspace", "node", "first").unwrap();
    let second = DomainObjectRef::new("workspace", "node", "second").unwrap();
    let mut goal = goal_with_ceiling(None);
    goal.target = Proposition::Related {
        src: Term::Object(first),
        relation: Term::Literal(Literal::Text("depends_on".to_string())),
        dst: Term::Object(second),
    };
    let goal_store = open_goal_store_with_active_goal(goal);
    let task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection_called = false;
    let mut projection = |_request: PlanningWorldStateRequest| {
        projection_called = true;
        Err(PlanningProjectionError::fatal("must not be called"))
    };

    let report = actor
        .run_once(
            &goal_store,
            &task_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap();

    assert!(!projection_called);
    assert_eq!(report.committed, 0);
    assert!(report.retryable_errors.is_empty());
    assert_eq!(report.fatal_errors.len(), 1);
    assert_eq!(
        report.fatal_errors[0].code,
        "planning_projection_request_invalid"
    );
    assert!(report.fatal_errors[0].message.contains("ambiguous"));
    assert!(matches!(
        &report.results[0],
        PlanningRuntimeActorGoalResult::PlanningFailed {
            error: PlanningInputError::IdentityMismatch { .. },
            ..
        }
    ));
    assert_eq!(task_network.state().revision, 0);
}

#[test]
fn planning_actor_reports_closed_initial_authority_as_retryable() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let mut task_network = open_task_network_store();
    task_network.shutdown();
    let actor = planning_actor();
    let mut projection = |_request: PlanningWorldStateRequest| {
        Err(PlanningProjectionError::fatal("must not be called"))
    };

    let error = actor
        .run_once(
            &goal_store,
            &task_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap_err();

    assert!(error.retryable());
    assert!(error.to_string().contains("closed"));
}

#[test]
fn planning_actor_rejects_authority_for_another_network_before_projection() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    let mut authority = TaskNetworkAuthority::open(&factory, "network-other", 8).unwrap();
    let ports = authority.ports();
    let actor = planning_actor();
    let mut projection_called = false;
    let mut projection = |_request: PlanningWorldStateRequest| {
        projection_called = true;
        Err(PlanningProjectionError::fatal("must not be called"))
    };

    let error = actor
        .run_once(&goal_store, &ports, &mut projection, actor_request(None))
        .unwrap_err();

    assert!(!projection_called);
    assert!(!error.retryable());
    assert!(error.to_string().contains("network-other"));
    assert_eq!(ports.query().state().unwrap().revision, 0);
    authority.shutdown().unwrap();
}

#[test]
fn planning_actor_reports_corrupt_goal_records_as_fatal_before_projection() {
    let goal_db = sled::Config::new().temporary(true).open().unwrap();
    goal_db
        .open_tree("execution_goal_records")
        .unwrap()
        .insert(b"goal-corrupt", b"not-json")
        .unwrap();
    goal_db.flush().unwrap();
    let error = match PersistentGoalSetStore::new(goal_db) {
        Ok(_) => panic!("corrupt goal record must fail reopen"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("goal store JSON failed"));
}

#[test]
fn planning_actor_reports_corrupt_selection_cursor_as_fatal_before_projection() {
    let goal_db = sled::Config::new().temporary(true).open().unwrap();
    let goal_store = PersistentGoalSetStore::new(goal_db.clone()).unwrap();
    let goal = goal_with_ceiling(None);
    goal_store
        .add_goal(AddGoalCommand {
            metadata: GoalCommandMetadata {
                command_id: "command-add-goal-docs".to_string(),
                source_identity: None,
                seq: 1,
            },
            goal,
        })
        .unwrap();
    goal_db
        .open_tree("execution_goal_planning_selection")
        .unwrap()
        .insert(b"state", b"not-json")
        .unwrap();
    goal_db.flush().unwrap();
    let task_network = open_task_network_store();
    let actor = planning_actor();
    let mut projection_called = false;
    let mut projection = |_request: PlanningWorldStateRequest| {
        projection_called = true;
        Err(PlanningProjectionError::fatal("must not be called"))
    };

    let error = actor
        .run_once(
            &goal_store,
            &task_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap_err();

    assert!(!projection_called);
    assert!(matches!(
        error,
        meld_execution::planning::PlanningRuntimeActorError::GoalStore {
            retryable: false,
            ..
        }
    ));
    assert_eq!(task_network.state().revision, 0);
}

#[test]
fn planning_actor_rejects_goal_record_key_mismatch_before_projection() {
    let goal_db = sled::Config::new().temporary(true).open().unwrap();
    let record = ExecutionGoalRecord {
        goal: goal_with_ceiling(None),
        source_command_id: Some("command-add-goal-docs".to_string()),
        source_identity: None,
        created_at_seq: 1,
        updated_at_seq: 1,
    };
    goal_db
        .open_tree("execution_goal_records")
        .unwrap()
        .insert(b"goal-other", serde_json::to_vec(&record).unwrap())
        .unwrap();
    goal_db.flush().unwrap();
    let error = match PersistentGoalSetStore::new(goal_db) {
        Ok(_) => panic!("mismatched goal record key must fail reopen"),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("does not match embedded goal id"));
}

#[test]
fn planning_actor_retries_stable_command_identity_after_authority_replacement() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    let mut first_authority = TaskNetworkAuthority::open(&factory, "network-docs", 16).unwrap();
    let first_ports = first_authority.ports();
    let actor = planning_actor();
    let mut replacement = None;
    let first = {
        let mut first_projection = |request: PlanningWorldStateRequest| {
            first_authority.shutdown().unwrap();
            replacement = Some(TaskNetworkAuthority::open(&factory, "network-docs", 16).unwrap());
            let world_state = unsatisfied_state();
            Ok(PlanningWorldStateProjection {
                frame: derived_frame(&request, &world_state),
                world_state,
            })
        };
        actor
            .run_once(
                &goal_store,
                &first_ports,
                &mut first_projection,
                actor_request(None),
            )
            .unwrap()
    };
    let PlanningRuntimeActorGoalResult::CommandFailed {
        command_id: Some(first_command_id),
        retryable: true,
        ..
    } = &first.results[0]
    else {
        panic!("closed authority must retain the stable planning command identity");
    };
    assert_eq!(first.output_revision, None);

    let replacement = replacement.as_mut().unwrap();
    let replacement_ports = replacement.ports();
    let mut second_projection = |request: PlanningWorldStateRequest| {
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };
    let second = actor
        .run_once(
            &goal_store,
            &replacement_ports,
            &mut second_projection,
            actor_request(None),
        )
        .unwrap();
    let PlanningRuntimeActorGoalResult::Submitted {
        command_id: second_command_id,
        response: Response::Accepted { .. },
        ..
    } = &second.results[0]
    else {
        panic!("replacement authority must accept the unchanged planning attempt");
    };

    assert_eq!(second_command_id, first_command_id);
    assert_eq!(second.output_revision, Some(1));
    replacement.shutdown().unwrap();
}

#[test]
fn planning_actor_rebases_after_natural_stale_race_and_replays_successful_attempt() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let actor = planning_actor();
    let mut target_network = open_task_network_store();
    let stale_state = target_network.state();
    let winner = CommandRequest {
        command_id: "unrelated-winner".to_string(),
        network_id: stale_state.network_id.clone(),
        base_revision: stale_state.revision,
        base_state_hash: stale_state.state_hash.clone(),
        read_preconditions: vec![
            ReadPrecondition::RevisionIs(stale_state.revision),
            ReadPrecondition::StateHashIs(stale_state.state_hash.clone()),
        ],
        command: TaskNetworkCommand::ApplyMutationSet(Set::empty(
            stale_state.network_id.clone(),
            "unrelated-composition",
            "unrelated-winner",
        )),
    };
    let racing_goals = StaleRaceGoalStore {
        inner: &goal_store,
        command_port: target_network.ports.commands().clone(),
        winner: std::sync::Mutex::new(Some(winner)),
    };
    let mut projection = |request: PlanningWorldStateRequest| {
        let world_state = unsatisfied_state();
        Ok(PlanningWorldStateProjection {
            frame: derived_frame(&request, &world_state),
            world_state,
        })
    };
    let stale = actor
        .run_once(
            &racing_goals,
            &target_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap();
    let PlanningRuntimeActorGoalResult::Submitted {
        command_id: stale_command_id,
        response: stale_response,
        ..
    } = &stale.results[0]
    else {
        panic!("racing planning attempt must persist its stale outcome");
    };
    assert!(matches!(
        stale_response,
        Response::Rejected(Rejection::StaleBase { actual: 1, .. })
    ));
    assert_eq!(stale.committed, 0);
    assert_eq!(stale.retryable_errors.len(), 1);

    let rebased_base = target_network.state();
    assert_eq!(rebased_base.revision, 1);
    let rebased = actor
        .run_once(
            &racing_goals,
            &target_network.ports,
            &mut projection,
            actor_request(None),
        )
        .unwrap();
    let PlanningRuntimeActorGoalResult::Submitted {
        command_id: rebased_command_id,
        response: rebased_response @ Response::Accepted { revision: 2, .. },
        ..
    } = &rebased.results[0]
    else {
        panic!("rebased planning attempt must commit at the current authority head");
    };
    assert_ne!(rebased_command_id, stale_command_id);
    assert_eq!(rebased.committed, 1);
    assert!(rebased.retryable_errors.is_empty());
    let durable_attempt = target_network
        .ports
        .query()
        .command_outcome(rebased_command_id.clone())
        .unwrap()
        .unwrap();
    assert_eq!(
        durable_attempt.request().base_revision,
        rebased_base.revision
    );
    assert_eq!(
        durable_attempt.request().base_state_hash,
        rebased_base.state_hash
    );
    assert_eq!(
        durable_attempt.request().read_preconditions,
        vec![
            ReadPrecondition::RevisionIs(rebased_base.revision),
            ReadPrecondition::StateHashIs(rebased_base.state_hash),
        ]
    );

    target_network.shutdown();
    let factory = TaskNetworkStoreFactory::new(target_network._temp.path());
    let mut reopened = TaskNetworkAuthority::open(&factory, "network-docs", 32).unwrap();
    let replay = actor
        .run_once(
            &goal_store,
            &reopened.ports(),
            &mut projection,
            actor_request(None),
        )
        .unwrap();
    let PlanningRuntimeActorGoalResult::Submitted {
        command_id: replayed_command_id,
        response: replayed_response,
        ..
    } = &replay.results[0]
    else {
        panic!("successful rebased attempt must replay exactly");
    };
    assert_eq!(replayed_command_id, rebased_command_id);
    assert_eq!(replayed_response, rebased_response);
    assert_eq!(reopened.query_port().state().unwrap().revision, 2);
    assert_eq!(reopened.query_port().journal().unwrap().len(), 2);
    reopened.shutdown().unwrap();
}

#[test]
fn planning_actor_marks_final_revision_unverified_after_partial_progress_and_replacement() {
    let goal_store = open_goal_store_with_active_goal(goal_with_ceiling(None));
    let mut second_goal = goal_with_ceiling(None);
    second_goal.goal_id = "goal-docs-z".to_string();
    goal_store
        .add_goal(AddGoalCommand {
            metadata: GoalCommandMetadata {
                command_id: "command-add-goal-docs-z".to_string(),
                source_identity: None,
                seq: 1,
            },
            goal: second_goal,
        })
        .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let factory = TaskNetworkStoreFactory::new(temp.path());
    let mut authority = TaskNetworkAuthority::open(&factory, "network-docs", 16).unwrap();
    let ports = authority.ports();
    let actor = planning_actor();
    let mut projection_count = 0;
    let mut replacement = None;
    let report = {
        let mut projection = |request: PlanningWorldStateRequest| {
            projection_count += 1;
            if projection_count == 2 {
                authority.shutdown().unwrap();
                replacement =
                    Some(TaskNetworkAuthority::open(&factory, "network-docs", 16).unwrap());
            }
            let world_state = unsatisfied_state();
            Ok(PlanningWorldStateProjection {
                frame: derived_frame(&request, &world_state),
                world_state,
            })
        };
        actor
            .run_once(&goal_store, &ports, &mut projection, actor_request(None))
            .unwrap()
    };

    assert_eq!(report.committed, 1);
    assert_eq!(report.output_revision, None);
    assert_eq!(report.last_acknowledged_revision, 1);
    assert!(report
        .retryable_errors
        .iter()
        .any(|issue| issue.code == "task_network_final_query_failed"));
    assert!(matches!(
        report.results[0],
        PlanningRuntimeActorGoalResult::Submitted {
            response: Response::Accepted { revision: 1, .. },
            ..
        }
    ));
    assert!(matches!(
        report.results[1],
        PlanningRuntimeActorGoalResult::CommandFailed {
            retryable: true,
            ..
        }
    ));

    let replacement = replacement.as_mut().unwrap();
    assert_eq!(replacement.ports().query().state().unwrap().revision, 1);
    replacement.shutdown().unwrap();
}
