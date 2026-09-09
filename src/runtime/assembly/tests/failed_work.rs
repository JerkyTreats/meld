//! Native repair recovery after an admitted operational failure.

use super::*;

#[test]
fn docs_failed_draft_recovers_after_changed_source_without_false_satisfaction() {
    assert_draft_recovery(400);
}

#[test]
fn docs_transient_provider_failure_recovers_the_original_claim() {
    assert_draft_recovery(503);
}

fn assert_draft_recovery(status: u16) {
    let provider = super::super::docs_fixture::ProviderServer::new();
    provider.draft_status(status);
    let harness = StewardshipHarness::new();
    let source = harness._workspace.path().join("lib.rs");
    std::fs::write(&source, "pub fn run() {}\n").unwrap();
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
    assert!(assembly.bind_owner_provider(api));
    let mut supervisor = harness.start_supervisor(&assembly);
    let RuntimeSemanticHandleFactory::TaskAdmission(execution) = &assembly
        .handle_factories()
        .get("execution.task_admission")
        .unwrap()
        .semantic
    else {
        unreachable!()
    };
    for pass in 0..45 {
        supervisor.tick(1_000 + pass * 10).unwrap();
    }
    let failed_outcomes: Vec<_> = execution
        .network
        .lock()
        .unwrap()
        .state()
        .outcomes
        .values()
        .filter(|outcome| {
            outcome.status == meld_execution::task_network::dispatch::OutcomeStatus::Failed
        })
        .map(|outcome| outcome.outcome_id.clone())
        .collect();
    if status == 400 {
        assert_eq!(
            provider
                .calls()
                .iter()
                .filter(|call| *call == "draft")
                .count(),
            1,
            "a rejected request must not be replayed"
        );
        assert!(
            !failed_outcomes.is_empty(),
            "native Execution must return the rejected provider call; calls: {:?}; statuses: {:?}",
            provider.calls(),
            execution.network.lock().unwrap().state().statuses
        );
    } else {
        assert!(
            failed_outcomes.is_empty(),
            "a transient response must not close the claim"
        );
        assert!(execution
            .network
            .lock()
            .unwrap()
            .state()
            .statuses
            .values()
            .any(|state| matches!(
                state,
                meld_execution::task_network::TaskStatus::Running { .. }
            )));
    }
    assert!(!harness._workspace.path().join("README.md").exists());
    let store = &assembly.stores().agent_store;
    let goals = store
        .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
        .unwrap();
    assert_eq!(goals.len(), 1);
    let goal_id = &goals[0].goal.goal_id;
    let plan = store.current_reconciliation_plan(goal_id).unwrap().unwrap();
    assert!(
        store
            .goal_disposition_for_plan(&plan.plan_revision_id)
            .unwrap()
            .is_none(),
        "operational failure must not satisfy the native maintenance Goal"
    );
    let history = store.completed_history_for_goal(goal_id).unwrap();
    if status == 400 {
        assert!(
            history
                .iter()
                .any(|entry| failed_outcomes.contains(&entry.owner_position_id)),
            "Agent must retain the native failed return"
        );
    }
    let prior_grants = store.product_authorizations_for_goal(goal_id).unwrap();
    provider.draft_status(200);
    if status == 400 {
        std::fs::write(
            &source,
            "pub fn run() {}\n// revised source after failed drafting\n",
        )
        .unwrap();
    }
    for pass in 0..60 {
        supervisor.tick(2_000 + pass * 10).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(harness._workspace.path().join("README.md"))
            .ok()
            .as_deref(),
        Some(super::super::docs_fixture::README),
        "changed source must permit a fresh native repair"
    );
    assert_eq!(
        store
            .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
            .unwrap(),
        goals
    );
    let plan = store.current_reconciliation_plan(goal_id).unwrap().unwrap();
    assert!(
        store
            .goal_disposition_for_plan(&plan.plan_revision_id)
            .unwrap()
            .is_some(),
        "recovered owner evidence must satisfy the original Goal"
    );
    let recovered = store.completed_history_for_goal(goal_id).unwrap();
    assert!(history.iter().all(|entry| recovered.contains(entry)));
    let grants = store.product_authorizations_for_goal(goal_id).unwrap();
    if status == 400 {
        assert!(grants.iter().any(|grant| !prior_grants.contains(grant)
            && matches!(
                grant.product,
                meld_world_model::AgentAuthorizedProduct::Task(_)
            )));
    } else {
        assert_eq!(
            grants
                .iter()
                .filter(|grant| matches!(
                    grant.product,
                    meld_world_model::AgentAuthorizedProduct::Task(_)
                ))
                .collect::<Vec<_>>(),
            prior_grants
                .iter()
                .filter(|grant| matches!(
                    grant.product,
                    meld_world_model::AgentAuthorizedProduct::Task(_)
                ))
                .collect::<Vec<_>>(),
            "transient recovery must complete the original authorized Task"
        );
    }
}

#[test]
fn docs_changed_knowledge_during_confirmation_can_repeat_the_repair() {
    assert_changed_input_repair(true);
}

#[test]
fn docs_target_deleted_during_confirmation_can_repeat_the_repair() {
    assert_changed_input_repair(false);
}

fn assert_changed_input_repair(change_source: bool) {
    let provider = super::super::docs_fixture::ProviderServer::new();
    let harness = StewardshipHarness::new();
    let source = harness._workspace.path().join("lib.rs");
    let readme = harness._workspace.path().join("README.md");
    std::fs::write(&source, "pub fn run() {}\n").unwrap();
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
    assert!(assembly.bind_owner_provider(api));
    let mut supervisor = harness.start_supervisor(&assembly);
    for pass in 0..70 {
        supervisor.tick(1_000 + pass * 10).unwrap();
        if readme.exists() {
            break;
        }
    }
    assert!(readme.exists());
    let store = &assembly.stores().agent_store;
    let goals = store
        .reconciliation_goals_for_agent(STEWARD_AGENT_ID)
        .unwrap();
    assert_eq!(goals.len(), 1);
    let goal_id = &goals[0].goal.goal_id;
    let plan = store.current_reconciliation_plan(goal_id).unwrap().unwrap();
    assert!(store
        .goal_disposition_for_plan(&plan.plan_revision_id)
        .unwrap()
        .is_none());
    // The owner has new source and the repaired document is gone before confirmation.
    if change_source {
        std::fs::write(
            &source,
            "pub fn run() {}\n// changed while the first repair awaited confirmation\n",
        )
        .unwrap();
    }
    std::fs::remove_file(&readme).unwrap();
    for pass in 0..100 {
        supervisor.tick(2_000 + pass * 10).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&readme).ok().as_deref(),
        Some(super::super::docs_fixture::README)
    );
    let plan = store.current_reconciliation_plan(goal_id).unwrap().unwrap();
    assert!(store
        .goal_disposition_for_plan(&plan.plan_revision_id)
        .unwrap()
        .is_some());
    let drafts = provider
        .calls()
        .iter()
        .filter(|call| call.as_str() == "draft")
        .count();
    assert_eq!(
        drafts, 2,
        "new knowledge permits one new repair; old completed work remains history"
    );
}
