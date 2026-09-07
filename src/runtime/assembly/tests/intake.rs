//! Native intake refusals return under their original authorization without effects.

use super::*;

type IntakeAttempt = (
    meld_world_model::AgentProductAuthorization,
    meld_world_model::AgentExecutionPosition,
);

struct ClosingExecutionIntake {
    inner: Arc<dyn meld_world_model::AgentExecutionPort>,
    lifecycle: crate::runtime::lifecycle::ActivationLifecycleStore,
    assignment_id: String,
    lose_return: bool,
    attempted: Arc<Mutex<Option<IntakeAttempt>>>,
}

impl meld_world_model::AgentExecutionPort for ClosingExecutionIntake {
    fn submit(
        &self,
        authorization: &meld_world_model::AgentProductAuthorization,
    ) -> Result<meld_world_model::AgentExecutionPosition, meld_world_model::error::StorageError>
    {
        self.lifecycle
            .begin_drain(&self.assignment_id, &authorization.activation_generation)
            .map_err(|error| {
                meld_world_model::error::StorageError::InvalidPath(error.to_string())
            })?;
        let position = self.inner.submit(authorization)?;
        *self.attempted.lock().unwrap() = Some((authorization.clone(), position.clone()));
        if self.lose_return {
            Err(meld_world_model::error::StorageError::InvalidPath(
                "injected loss after durable intake refusal".into(),
            ))
        } else {
            Ok(position)
        }
    }

    fn advance(
        &self,
        authorization: &meld_world_model::AgentProductAuthorization,
    ) -> Result<meld_world_model::AgentExecutionPosition, meld_world_model::error::StorageError>
    {
        self.inner.advance(authorization)
    }

    fn observe(
        &self,
        authorization: &meld_world_model::AgentProductAuthorization,
    ) -> Result<
        Option<meld_world_model::AgentExecutionPosition>,
        meld_world_model::error::StorageError,
    > {
        self.inner.observe(authorization)
    }
}

#[test]
fn refused_task_intake_returns_without_an_operational_outcome() {
    for lose_return in [false, true] {
        let harness = StewardshipHarness::new();
        {
            let assembly = harness.assembly();
            harness.run_workspace_fixture_genesis(&assembly);
            assembly
                .ports()
                .event_append()
                .append_envelope_idempotent(workspace_owner_publication(
                    stewardship_subject_ref(&harness.binding).unwrap(),
                    "refused-task-source",
                ))
                .unwrap();
            assembly.graph_runtime().catch_up().unwrap();
        }
        let attempted = Arc::new(Mutex::new(None));
        let (authorization, position, history);
        {
            let mut assembly = harness.assembly();
            let lifecycle = assembly.lifecycle_store().unwrap().clone();
            let assignment_id = assembly
                .prepared_activation()
                .unwrap()
                .assignment
                .assignment_id
                .clone();
            let RuntimeSemanticHandleFactory::AgentActor(factory) = &mut assembly
                .handle_factories
                .factories
                .get_mut(AGENT_RECONCILIATION_RUNTIME_ID)
                .unwrap()
                .semantic
            else {
                unreachable!()
            };
            factory.execution = Arc::new(ClosingExecutionIntake {
                inner: Arc::clone(&factory.execution),
                lifecycle,
                assignment_id,
                lose_return,
                attempted: Arc::clone(&attempted),
            });
            let mut supervisor = harness.start_supervisor(&assembly);
            for _ in 0..32 {
                for owner in [
                    "world_model.graph_replay",
                    "world_model.belief_assessment",
                    "world_model.standing_curation",
                    AGENT_RECONCILIATION_RUNTIME_ID,
                ] {
                    let report = supervisor.step_owner_for_test(owner, WorkBudget { max_items: 8 });
                    assert!(report.fatal_errors.is_empty(), "{report:?}");
                }
                if attempted.lock().unwrap().is_some() {
                    break;
                }
            }
            (authorization, position) = attempted
                .lock()
                .unwrap()
                .clone()
                .expect("native Agent must offer its authorized Task");
            assert_eq!(
                position.admission_decision,
                meld_world_model::AgentExecutionAdmissionDecision::StaleFence
            );
            assert!(position.outcome_id.is_none() && position.network_commit_revision.is_none());
            for _ in 0..4 {
                let report = supervisor.step_owner_for_test(
                    AGENT_RECONCILIATION_RUNTIME_ID,
                    WorkBudget { max_items: 1 },
                );
                assert!(
                    report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
                    "{report:?}"
                );
            }
            history = assembly
                .stores()
                .agent_store
                .completed_history_for_goal(&authorization.goal_id)
                .unwrap();
            assert!(history.iter().any(|entry| entry.owner_position_id == position.admission_id
                && entry.product_id == authorization.product_id
                && entry.source_plan_revision_id == authorization.plan_revision_id
                && matches!(&entry.accepted_milestone, meld_world_model::PlanMilestoneRequirement::ExecutionNotAdmitted { task_id } if task_id == &authorization.product_id)),
                "Agent discarded a terminal intake refusal because no operational outcome exists: {history:?}");
            assert_no_pending_products(&assembly);
            let RuntimeSemanticHandleFactory::TaskAdmission(execution) = &assembly
                .handle_factories()
                .get("execution.task_admission")
                .unwrap()
                .semantic
            else {
                unreachable!()
            };
            let network = execution.network.lock().unwrap();
            assert_eq!(network.state().admissions.len(), 1);
            assert!(
                network.state().tasks.is_empty()
                    && network.state().claims.is_empty()
                    && network.state().outcomes.is_empty()
            );
            drop(network);
            assembly.flush_product_boundary().unwrap();
        }
        let assembly = harness.assembly();
        assert_no_pending_products(&assembly);
        assert_eq!(
            assembly
                .stores()
                .agent_store
                .completed_history_for_goal(&authorization.goal_id)
                .unwrap(),
            history
        );
        let RuntimeSemanticHandleFactory::AgentActor(factory) = &assembly
            .handle_factories()
            .get(AGENT_RECONCILIATION_RUNTIME_ID)
            .unwrap()
            .semantic
        else {
            unreachable!()
        };
        assert_eq!(
            factory.execution.observe(&authorization).unwrap(),
            Some(position)
        );
        harness.bind_production_routes(&assembly);
        let mut command = SupervisorStartCommand::new("refused-task-recovery", 1_000_000);
        command.registration_set = assembly.registration_set().cloned();
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        let mut successor = None;
        for _ in 0..32 {
            for owner in [
                "world_model.graph_replay",
                "world_model.belief_assessment",
                "world_model.standing_curation",
                AGENT_RECONCILIATION_RUNTIME_ID,
            ] {
                let report = supervisor.step_owner_for_test(owner, WorkBudget { max_items: 8 });
                assert!(
                    report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
                    "{report:?}"
                );
            }
            successor = assembly
                .stores()
                .agent_store
                .product_authorizations_for_goal(&authorization.goal_id)
                .unwrap()
                .into_iter()
                .find(|grant| {
                    grant.authorization_id != authorization.authorization_id
                        && matches!(
                            grant.product,
                            meld_world_model::AgentAuthorizedProduct::Task(_)
                        )
                });
            if successor.is_some() {
                break;
            }
        }
        let successor =
            successor.expect("an intake refusal must leave fresh successor Task work eligible");
        assert_ne!(successor.plan_revision_id, authorization.plan_revision_id);
        assert_ne!(successor.admission_epoch, authorization.admission_epoch);
        assert_eq!(
            factory
                .execution
                .observe(&successor)
                .unwrap()
                .unwrap()
                .admission_decision,
            meld_world_model::AgentExecutionAdmissionDecision::Admitted
        );
    }
}

#[test]
fn rejected_predecessor_curation_is_consumed_without_a_terminal_result() {
    assert_rejected_predecessor_return(false);
}

#[test]
fn rejected_predecessor_requires_fresh_successful_curation_before_successor_task() {
    assert_rejected_predecessor_return(true);
}

fn assert_rejected_predecessor_return(continue_successor: bool) {
    let harness = StewardshipHarness::new();
    {
        let assembly = harness.assembly();
        harness.run_workspace_fixture_genesis(&assembly);
        assembly
            .ports()
            .event_append()
            .append_envelope_idempotent(workspace_owner_publication(
                stewardship_subject_ref(&harness.binding).unwrap(),
                "rejected-prerequisite-source",
            ))
            .unwrap();
        assembly.graph_runtime().catch_up().unwrap();
    }
    let (operation, original_authorization);
    {
        let assembly = harness.assembly();
        harness.bind_production_routes(&assembly);
        let mut supervisor = harness.start_supervisor(&assembly);
        for _ in 0..8 {
            for owner in [
                "world_model.belief_assessment",
                AGENT_RECONCILIATION_RUNTIME_ID,
            ] {
                let report = supervisor.step_owner_for_test(owner, WorkBudget { max_items: 8 });
                assert!(report.fatal_errors.is_empty(), "{report:?}");
            }
            if assembly
                .stores()
                .curation_store
                .next_planned_operation(STEWARD_AGENT_ID)
                .unwrap()
                .is_some()
            {
                break;
            }
        }
        operation = assembly
            .stores()
            .curation_store
            .next_planned_operation(STEWARD_AGENT_ID)
            .unwrap()
            .unwrap();
        let goal_id = &operation.planned_authorization.as_ref().unwrap().goal_id;
        let grants = assembly
            .stores()
            .agent_store
            .product_authorizations_for_goal(goal_id)
            .unwrap();
        assert_eq!(grants.len(), 1);
        original_authorization = grants[0].clone();
        assert!(assembly
            .stores()
            .curation_store
            .acceptance_for_planned_operation(&operation.operation_id)
            .unwrap()
            .is_none());
        assembly.flush_product_boundary().unwrap();
        assembly.flush_supervisor_store().unwrap();
        // Interrupt after durable submission and before native Curation intake.
    }
    let acceptance_id;
    {
        let assembly = harness.assembly();
        harness.bind_production_routes(&assembly);
        let mut command = SupervisorStartCommand::new("rejected-prerequisite-recovery", 1_000_000);
        command.registration_set = assembly.registration_set().cloned();
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        let report = supervisor
            .step_owner_for_test("world_model.standing_curation", WorkBudget { max_items: 1 });
        assert!(report.fatal_errors.is_empty(), "{report:?}");
        let acceptance = assembly
            .stores()
            .curation_store
            .acceptance_for_planned_operation(&operation.operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            acceptance.decision,
            meld_world_model::CurationAdmissionDecision::Rejected
        );
        assert_eq!(acceptance.authority, operation.authority);
        assert_ne!(
            acceptance
                .observed_authority
                .as_ref()
                .unwrap()
                .admission_epoch,
            operation.authority.admission_epoch
        );
        acceptance_id = acceptance.acceptance_id;
        assert!(assembly
            .stores()
            .curation_store
            .result_for_operation(&operation.operation_id)
            .unwrap()
            .is_none());
        let report = supervisor
            .step_owner_for_test(AGENT_RECONCILIATION_RUNTIME_ID, WorkBudget { max_items: 1 });
        assert!(
            report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
            "{report:?}"
        );
        let history = assembly
            .stores()
            .agent_store
            .completed_history_for_goal(&original_authorization.goal_id)
            .unwrap();
        assert!(history.iter().any(|entry| entry.owner_position_id == acceptance_id
            && entry.source_plan_revision_id == original_authorization.plan_revision_id
            && entry.product_id == original_authorization.product_id
            && matches!(&entry.accepted_milestone,
                meld_world_model::PlanMilestoneRequirement::CurationRejected { operation_id }
                    if operation_id == &operation.operation_id)),
            "Agent did not consume the native rejection under its original authorization: {history:?}");
        assert_eq!(
            assembly
                .stores()
                .agent_store
                .product_authorizations_for_goal(&original_authorization.goal_id)
                .unwrap(),
            vec![original_authorization.clone()]
        );
        assert_no_pending_products(&assembly);
        if continue_successor {
            let mut successor = None;
            for _ in 0..8 {
                for owner in [
                    "world_model.graph_replay",
                    "world_model.belief_assessment",
                    AGENT_RECONCILIATION_RUNTIME_ID,
                ] {
                    let report =
                        supervisor.step_owner_for_test(owner, WorkBudget { max_items: 16 });
                    assert!(
                        report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
                        "{report:?}"
                    );
                }
                let grants = assembly
                    .stores()
                    .agent_store
                    .product_authorizations_for_goal(&original_authorization.goal_id)
                    .unwrap();
                assert!(
                    grants.iter().all(|grant| matches!(
                        grant.product,
                        meld_world_model::AgentAuthorizedProduct::Epistemic(_)
                    )),
                    "rejected intake enabled a dependent Task before fresh Curation"
                );
                if let Some(grant) = grants
                    .into_iter()
                    .find(|grant| grant.authorization_id != original_authorization.authorization_id)
                {
                    successor = Some(grant);
                    break;
                }
            }
            let successor = successor.expect("successor epoch must authorize its own prerequisite");
            assert_ne!(
                successor.admission_epoch,
                original_authorization.admission_epoch
            );
            let meld_world_model::AgentAuthorizedProduct::Epistemic(epistemic) = &successor.product
            else {
                unreachable!()
            };
            assert_ne!(epistemic.operation.operation_id, operation.operation_id);
            for _ in 0..16 {
                for owner in [
                    "world_model.standing_curation",
                    "world_model.graph_replay",
                    "world_model.belief_assessment",
                    AGENT_RECONCILIATION_RUNTIME_ID,
                ] {
                    let report =
                        supervisor.step_owner_for_test(owner, WorkBudget { max_items: 16 });
                    assert!(
                        report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
                        "{report:?}"
                    );
                }
                let grants = assembly
                    .stores()
                    .agent_store
                    .product_authorizations_for_goal(&original_authorization.goal_id)
                    .unwrap();
                let tasks: Vec<_> = grants
                    .iter()
                    .filter(|grant| {
                        matches!(
                            grant.product,
                            meld_world_model::AgentAuthorizedProduct::Task(_)
                        )
                    })
                    .collect();
                if tasks.is_empty() {
                    continue;
                }
                assert_eq!(tasks.len(), 1);
                assert_eq!(tasks[0].admission_epoch, successor.admission_epoch);
                let history = assembly
                    .stores()
                    .agent_store
                    .completed_history_for_goal(&original_authorization.goal_id)
                    .unwrap();
                assert!(history
                    .iter()
                    .any(|entry| matches!(&entry.accepted_milestone,
                    meld_world_model::PlanMilestoneRequirement::CurationVisible { operation_id }
                        if operation_id == &epistemic.operation.operation_id)));
                assert_eq!(
                    history
                        .iter()
                        .filter(|entry| entry.owner_position_id == acceptance_id)
                        .count(),
                    1
                );
                assert_native_pending_products(&assembly, &serde_json::to_vec(&tasks).unwrap());
                return;
            }
            panic!("fresh visible Curation did not enable the successor Task");
        }
        let prepared = assembly.prepared_activation().unwrap();
        assembly
            .lifecycle_store()
            .unwrap()
            .begin_drain(
                &prepared.assignment.assignment_id,
                &original_authorization.activation_generation,
            )
            .unwrap();
        for _ in 0..3 {
            let report = supervisor
                .step_owner_for_test(AGENT_RECONCILIATION_RUNTIME_ID, WorkBudget { max_items: 1 });
            assert!(
                report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
                "{report:?}"
            );
            assert_eq!(
                report.items_committed, 0,
                "rejected return was absorbed twice: {report:?}"
            );
        }
        assert_eq!(
            assembly
                .stores()
                .agent_store
                .completed_history_for_goal(&original_authorization.goal_id)
                .unwrap(),
            history
        );
        assert_no_pending_products(&assembly);
        assembly.flush_product_boundary().unwrap();
    }
    let assembly = harness.assembly();
    assert_no_pending_products(&assembly);
    assert_eq!(
        assembly
            .stores()
            .agent_store
            .completed_history_for_goal(&original_authorization.goal_id)
            .unwrap()
            .iter()
            .filter(|entry| entry.owner_position_id == acceptance_id)
            .count(),
        1
    );
    assert!(assembly
        .stores()
        .curation_store
        .result_for_operation(&operation.operation_id)
        .unwrap()
        .is_none());
}

fn assert_no_pending_products(assembly: &ProductRuntimeAssembly) {
    assert_native_pending_products(assembly, b"[]");
}

fn assert_native_pending_products(assembly: &ProductRuntimeAssembly, expected: &[u8]) {
    let handle = assembly
        .handle_factories()
        .get(AGENT_RECONCILIATION_RUNTIME_ID)
        .unwrap()
        .build_handle();
    let RuntimeSemanticHandle::AgentActor(native) = &handle.semantic else {
        unreachable!()
    };
    assert_eq!(
        native
            .actor
            .lifecycle_evidence()
            .unwrap()
            .unresolved_operation_summary_ref,
        format!(
            "agent-unresolved-products::{}",
            blake3::hash(expected).to_hex()
        ),
        "native pending products must exclude returned intake refusals"
    );
}
