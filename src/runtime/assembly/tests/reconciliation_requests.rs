//! Distinct native Goals share Execution without sharing Agent decisions.

use super::*;

#[test]
fn named_requests_incept_separate_goals_and_share_native_work() {
    let provider = super::super::docs_fixture::ProviderServer::new();
    let harness = StewardshipHarness::new();
    std::fs::write(
        harness._workspace.path().join("lib.rs"),
        "pub fn run() {}\n",
    )
    .unwrap();
    {
        let assembly = harness.assembly();
        harness.run_world_genesis(&assembly);
    }
    let assembly = harness.assembly();
    let api = harness.bind_production_routes_with_loss(&assembly, None);
    let mut config =
        stewardship_merkle_config(harness._workspace.path(), &harness.binding.storage_root);
    config.providers.get_mut("main-provider").unwrap().endpoint = Some(provider.endpoint());
    api.provider_registry()
        .write()
        .load_from_config(&config)
        .unwrap();
    assert!(assembly.bind_production_docs_claim_judge(api));
    let store = &assembly.stores().agent_store;
    let command = crate::cli::RuntimeCommands::Request {
        agent_id: STEWARD_AGENT_ID.into(),
        request_key: "independent-docs-review".into(),
        format: "json".into(),
    };
    let output = crate::runtime::tooling::handle_cli_command(&assembly, &command).unwrap();
    let output: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(output["completed"], false);
    let request: meld_world_model::agent::AgentReconciliationRequest =
        serde_json::from_value(output["request"].clone()).unwrap();
    let second_request = store
        .request_reconciliation(STEWARD_AGENT_ID, "another-docs-review")
        .unwrap();
    assert_ne!(request.request_id, second_request.request_id);
    assert_eq!(
        store
            .request_reconciliation(STEWARD_AGENT_ID, "independent-docs-review")
            .unwrap(),
        request
    );
    assert!(store
        .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
        .unwrap()
        .is_empty());
    assert!(!store.reconciliation_request_completed(&request).unwrap());
    let mut supervisor = harness.start_supervisor(&assembly);
    for _ in 0..32 {
        for owner in [
            "docs.observation",
            "world_model.evidence_ingestion",
            "world_model.graph_replay",
            "world_model.standing_curation",
            "world_model.belief_assessment",
            AGENT_RECONCILIATION_RUNTIME_ID,
            "execution.task_admission",
        ] {
            let report = supervisor.step_owner_for_test(owner, WorkBudget { max_items: 8 });
            assert!(
                report.fatal_errors.is_empty() && report.retryable_errors.is_empty(),
                "{owner}: {report:?}"
            );
        }
    }
    let goals = store
        .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
        .unwrap();
    assert_eq!(
        goals.len(),
        3,
        "the standing intent and both requests retain their own Goals: {goals:?}"
    );
    let authorizations: Vec<_> = goals
        .iter()
        .flat_map(|goal| {
            store
                .product_authorizations_for_goal(&goal.goal.goal_id)
                .unwrap()
        })
        .filter(|authorization| {
            matches!(
                authorization.product,
                meld_world_model::AgentAuthorizedProduct::Task(_)
            )
        })
        .collect();
    assert_eq!(authorizations.len(), 3);
    assert_ne!(authorizations[0].goal_id, authorizations[1].goal_id);
    assert_ne!(
        authorizations[0].authorization_id,
        authorizations[1].authorization_id
    );
    let RuntimeSemanticHandleFactory::TaskAdmission(execution) = &assembly
        .handle_factories()
        .get("execution.task_admission")
        .unwrap()
        .semantic
    else {
        unreachable!()
    };
    assert_eq!(
        execution.network.lock().unwrap().state().admissions.len(),
        3
    );
    supervisor.step_owner_for_test("execution.task_dispatch", WorkBudget { max_items: 3 });
    let shared =
        supervisor.step_owner_for_test("execution.task_dispatch", WorkBudget { max_items: 2 });
    assert!(
        shared.fatal_errors.is_empty() && shared.retryable_errors.is_empty(),
        "{shared:?}"
    );
    assert_eq!(
        execution.network.lock().unwrap().state().shared_steps.len(),
        2,
        "{shared:?}"
    );
    for pass in 0..100 {
        supervisor.tick(1_000 + pass * 10).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(harness._workspace.path().join("README.md")).unwrap(),
        super::super::docs_fixture::README
    );
    assert!(store.reconciliation_request_completed(&request).unwrap());
    assert!(store
        .reconciliation_request_completed(&second_request)
        .unwrap());
    for goal in &goals {
        let plan = store
            .current_reconciliation_plan(&goal.goal.goal_id)
            .unwrap()
            .unwrap();
        let disposition = store
            .goal_disposition_for_plan(&plan.plan_revision_id)
            .unwrap()
            .unwrap();
        assert_eq!(disposition.goal_id, goal.goal.goal_id);
        assert!(matches!(
            disposition.lifecycle,
            meld_lang::GoalLifecycle::Satisfied { .. }
        ));
        let milestones = store.milestones_for_goal(&goal.goal.goal_id).unwrap();
        assert!(!disposition.accepted_milestone_ids.is_empty());
        assert!(disposition
            .accepted_milestone_ids
            .iter()
            .all(|id| milestones
                .iter()
                .any(|milestone| &milestone.milestone_id == id)));
        for authorization in authorizations
            .iter()
            .filter(|authorization| authorization.goal_id == goal.goal.goal_id)
        {
            assert!(store
                .product_authorizations_for_goal(&goal.goal.goal_id)
                .unwrap()
                .contains(authorization));
        }
    }
    let accounts: Vec<_> = {
        let network = execution.network.lock().unwrap();
        network
            .state()
            .admissions
            .keys()
            .filter_map(|id| {
                meld_execution::task_network::sharing::admission_discharge_account(
                    network.state(),
                    id,
                )
            })
            .collect()
    };
    assert_eq!(accounts.len(), 3);
    assert_ne!(accounts[0].admission.goal_id, accounts[1].admission.goal_id);
    assert!(!accounts[0].shared_action_decision_ids.is_empty());
    assert!(accounts
        .iter()
        .all(|account| !account.shared_action_decision_ids.is_empty()));
    {
        let network = execution.network.lock().unwrap();
        let shared_nodes: std::collections::BTreeSet<_> = network
            .state()
            .shared_steps
            .values()
            .map(|step| step.sharing.as_ref().unwrap().shared_node_id.clone())
            .collect();
        assert_eq!(shared_nodes.len(), 1);
    }
    supervisor.request_shutdown(3_000).unwrap();
    drop(supervisor);
    drop(assembly);

    let reopened = harness.assembly();
    let store = &reopened.stores().agent_store;
    assert_eq!(
        store
            .request_reconciliation(STEWARD_AGENT_ID, "independent-docs-review")
            .unwrap(),
        request
    );
    assert!(store.reconciliation_request_completed(&request).unwrap());
    assert_eq!(
        store
            .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
            .unwrap(),
        goals
    );
    let api = harness.bind_production_routes_with_loss(&reopened, None);
    api.provider_registry()
        .write()
        .load_from_config(&config)
        .unwrap();
    assert!(reopened.bind_production_docs_claim_judge(api));
    let mut command = SupervisorStartCommand::new("docs-request-recovery", 1_000_000);
    command.registration_set = reopened.registration_set().cloned();
    let mut supervisor =
        RuntimeSupervisor::start(reopened.supervisor_startup_package(), command).unwrap();
    let sources = crate::serve::sources::ServeSources::from_assembly(&reopened).unwrap();
    let addressed = crate::serve::routes::ReconciliationRequest {
        product_root: reopened.product_root().to_path_buf(),
        agent_id: STEWARD_AGENT_ID.into(),
        request_key: "already-correct-docs".into(),
    };
    let body = serde_json::to_vec(&addressed).unwrap();
    assert_ne!(
        crate::serve::routes::dispatch(
            &sources,
            "POST",
            "/v1/agents/reconciliation_requests",
            &body
        )
        .status,
        200
    );
    let sources = sources.with_reconciliation_requests();
    let mut foreign = addressed.clone();
    foreign.product_root = harness._external.path().join("another-product");
    assert_ne!(
        crate::serve::routes::dispatch(
            &sources,
            "POST",
            "/v1/agents/reconciliation_requests",
            &serde_json::to_vec(&foreign).unwrap()
        )
        .status,
        200
    );
    assert_eq!(
        store
            .reconciliation_requests(STEWARD_AGENT_ID)
            .unwrap()
            .len(),
        2
    );
    let server =
        crate::serve::listener::serve_with_discovery(sources, 0, reopened.product_root()).unwrap();
    let output = crate::runtime::tooling::try_live_runtime_request(
        harness._workspace.path(),
        &config,
        STEWARD_AGENT_ID,
        "already-correct-docs",
        "json",
    )
    .expect("live owner must receive the request")
    .unwrap();
    let no_action: meld_world_model::agent::AgentReconciliationRequestStatus =
        serde_json::from_str(&output).unwrap();
    assert!(!no_action.completed);
    for pass in 0..40 {
        supervisor.tick(1_000_100 + pass * 10).unwrap();
    }
    assert!(store
        .reconciliation_request_completed(&no_action.request)
        .unwrap());
    let repeated = crate::runtime::tooling::try_live_runtime_request(
        harness._workspace.path(),
        &config,
        STEWARD_AGENT_ID,
        "already-correct-docs",
        "json",
    )
    .unwrap()
    .unwrap();
    let repeated: meld_world_model::agent::AgentReconciliationRequestStatus =
        serde_json::from_str(&repeated).unwrap();
    assert!(repeated.completed);
    assert_eq!(repeated.request, no_action.request);
    assert_eq!(
        store
            .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
            .unwrap(),
        goals
    );
    supervisor.request_shutdown(1_000_600).unwrap();
    drop(server);
}
