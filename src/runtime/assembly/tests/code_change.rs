//! A shipped product delegates construction, authority and confirmation to native owners.

use super::*;
use crate::code_change::{acquisition, capability::*, contracts::*, publication};

#[test]
fn installed_code_change_constructs_authorizes_and_confirms_native_work() {
    prove_native_code_change(false);
}

#[test]
fn code_change_request_survives_restart_without_repeating_materialization() {
    prove_native_code_change(true);
}

fn prove_native_code_change(restart: bool) {
    let mut harness = StewardshipHarness::new();
    harness.binding.subject = DomainObjectRef::new("workspace_fs", "node", "repo").unwrap();
    harness.binding.agent_id = "code-agent".into();
    harness.binding.provider_id = None;
    harness.binding.package = crate::config::SelectedStewardshipPackage {
        expression: "code_change".into(),
        principal_id: "workspace-owner".into(),
        belief_family_id: "code_materialization".into(),
        evidence_mapping_id: "code_materialization_v1".into(),
        curation_rule_id: "code_materialization".into(),
        maintained_condition_id: "code_materialization".into(),
        strategy_theory_id: "code_materialization".into(),
        authority_policy_id: "code_change_local".into(),
        claim_policy_id: String::new(),
    };
    let path = harness._workspace.path().join("lib.rs");
    std::fs::write(&path, "pub const VERSION: u8 = 1;\n").unwrap();
    let change = CodeChangeSet::new(
        harness.binding.subject.clone(),
        vec![],
        vec![FileReplacement {
            relative_path: "lib.rs".into(),
            expected_content_hash: blake3::hash(&std::fs::read(&path).unwrap())
                .to_hex()
                .to_string(),
            replacement: "pub const VERSION: u8 = 2;\n".into(),
        }],
    )
    .unwrap();
    let source = harness._external.path().join("proposal.json");
    std::fs::write(&source, serde_json::to_vec(&change).unwrap()).unwrap();
    harness.binding.bindings.insert(
        acquisition::SOURCE.into(),
        crate::config::PhysicalBindingRef::EndpointRef(source.display().to_string()),
    );
    let assembly = harness.assembly();
    harness.run_world_genesis_from(
        &assembly,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/code_change"),
    );
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "pub const VERSION: u8 = 1;\n"
    );
    assert!(
        records(&harness)
            .iter()
            .all(|record| !record.event_type.starts_with("code_change.")),
        "preparation must remain inert"
    );
    drop(assembly);
    let assembly = harness.assembly();
    let loss = Arc::new(std::sync::atomic::AtomicBool::new(true));
    harness.bind_production_routes_with_loss(&assembly, Some((loss.clone(), false, Some(APPLY))));
    let mut supervisor = harness.start_supervisor(&assembly);
    for pass in 0..30 {
        supervisor.tick(1_100 + pass * 10).unwrap();
    }
    let accounts: Vec<_> = records(&harness)
        .into_iter()
        .filter(|record| record.event_type == publication::EVENT)
        .collect();
    assert_eq!(
        accounts.len(),
        1,
        "native Agent did not reach code materialization"
    );
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        change.files[0].replacement
    );
    let store = &assembly.stores().agent_store;
    let goals = store.reconciliation_goals_for_agent("code-agent").unwrap();
    assert_eq!(goals.len(), 1);
    let goal_id = goals[0].goal.goal_id.clone();
    assert!(
        !satisfied(store, &goal_id),
        "owner publication cannot replace this product's required Task return"
    );
    let authorizations = store.product_authorizations_for_goal(&goal_id).unwrap();
    let tasks: Vec<_> = authorizations
        .iter()
        .filter_map(|authorization| match &authorization.product {
            meld_world_model::AgentAuthorizedProduct::Task(task) => Some((authorization, task)),
            _ => None,
        })
        .collect();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].1.composition.steps.len(), 2);
    assert!(tasks[0].1.initial_inputs.is_empty());
    assert!(tasks[0].0.authority_decision.is_some());
    assert_eq!(
        tasks[0].1.authority_requirements,
        vec![APPLY.to_string(), acquisition::READ.to_string()]
    );
    let operation: meld_world_model::world_state::graph::contracts::OwnerPublicationOperation =
        serde_json::from_value(accounts[0].data.clone()).unwrap();
    let (scope, root) =
        publication::request_observation_source("code-agent", &harness.binding.subject, &goal_id)
            .unwrap();
    assert_eq!(operation.batch.scope, scope);
    assert_eq!(operation.batch.objects[0].object_ref, root);
    assert_ne!(
        publication::request_observation_source(
            "code-agent",
            &harness.binding.subject,
            "foreign-request"
        )
        .unwrap()
        .0,
        scope
    );
    assert_eq!(accounts[0].provenance.source_records.len(), 2);
    assert!(accounts[0]
        .provenance
        .source_records
        .iter()
        .all(|position| position.seq < accounts[0].seq));

    // An interrupted callback recovers retained owner evidence, not a fresh source read or write.
    std::fs::remove_file(source).unwrap();
    std::fs::write(&path, "later user edit\n").unwrap();
    if restart {
        supervisor.request_shutdown(1_450).unwrap();
        drop(supervisor);
        drop(assembly);
        let assembly = harness.assembly();
        harness.bind_production_routes(&assembly);
        let mut command = SupervisorStartCommand::new("code-successor", 1_000_000);
        command.registration_set = assembly.registration_set().cloned();
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        finish(
            &harness,
            &assembly,
            &mut supervisor,
            &goal_id,
            accounts[0].seq,
            1_000_100,
        );
        let store = &assembly.stores().agent_store;
        assert_eq!(
            store
                .reconciliation_goals_for_agent("code-agent")
                .unwrap()
                .len(),
            1
        );
        let retained = store.epoch_products(&goal_id).unwrap().unwrap();
        let history = store.completed_history_for_goal(&goal_id).unwrap();
        assert!(history.iter().any(|entry| matches!(
            entry.accepted_milestone,
            meld_world_model::strategy::PlanMilestoneRequirement::ExecutionTerminal { .. }
        )));
        let plan = store
            .current_reconciliation_plan(&goal_id)
            .unwrap()
            .unwrap();
        let disposition = store
            .goal_disposition_for_plan(&plan.plan_revision_id)
            .unwrap()
            .unwrap();
        assert_ne!(
            disposition.activation_generation,
            retained.specification.fence.activation_generation
        );
        drop(supervisor);
        drop(assembly);
        let assembly = harness.assembly();
        harness.bind_production_routes(&assembly);
        let mut command = SupervisorStartCommand::new("code-completed-request", 2_000_000);
        command.registration_set = assembly.registration_set().cloned();
        let mut supervisor =
            RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
        finish(
            &harness,
            &assembly,
            &mut supervisor,
            &goal_id,
            accounts[0].seq,
            2_000_100,
        );
        let store = &assembly.stores().agent_store;
        assert_eq!(
            store
                .reconciliation_goals_for_agent("code-agent")
                .unwrap()
                .len(),
            1
        );
        assert_eq!(store.epoch_products(&goal_id).unwrap(), Some(retained));
    } else {
        loss.store(false, std::sync::atomic::Ordering::SeqCst);
        finish(
            &harness,
            &assembly,
            &mut supervisor,
            &goal_id,
            accounts[0].seq,
            1_500,
        );
    }
}

fn finish(
    harness: &StewardshipHarness,
    assembly: &ProductRuntimeAssembly,
    supervisor: &mut RuntimeSupervisor<'_>,
    goal_id: &str,
    account_seq: u64,
    now: u64,
) {
    let store = &assembly.stores().agent_store;
    let path = harness._workspace.path().join("lib.rs");
    for pass in 0..35 {
        supervisor.tick(now + pass * 10).unwrap();
    }
    assert!(
        satisfied(store, goal_id),
        "native confirmation did not satisfy the materialization Goal"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "later user edit\n");
    let history = store.completed_history_for_goal(goal_id).unwrap();
    use meld_world_model::strategy::PlanMilestoneRequirement;
    for kind in ["execution", "curation", "belief"] {
        assert!(
            history.iter().any(|entry| matches!(
                (&entry.accepted_milestone, kind),
                (
                    PlanMilestoneRequirement::ExecutionTerminal { .. },
                    "execution"
                ) | (
                    PlanMilestoneRequirement::CurationTerminal { .. },
                    "curation"
                ) | (PlanMilestoneRequirement::BeliefRevision { .. }, "belief")
            )),
            "missing {kind} evidence"
        );
    }
    let records = records(harness);
    for event in [
        acquisition::EVENT,
        "code_change.intent.v1",
        "code_change.materialized.v1",
        publication::EVENT,
    ] {
        assert_eq!(
            records
                .iter()
                .filter(|record| record.event_type == event)
                .count(),
            1,
            "repeated {event}"
        );
    }
    assert!(records.iter().any(
        |record| record.event_type == "world_model.curation.result.v1" && record.seq > account_seq
    ));
    supervisor.request_shutdown(now + 500).unwrap();
}

fn records(harness: &StewardshipHarness) -> Vec<meld_events::EventRecord> {
    harness
        .authority
        .replay_capability()
        .newest_page(1024)
        .unwrap()
        .records
}

fn satisfied(store: &meld_world_model::agent::AgentStore, goal_id: &str) -> bool {
    store
        .current_reconciliation_plan(goal_id)
        .unwrap()
        .and_then(|plan| {
            store
                .goal_disposition_for_plan(&plan.plan_revision_id)
                .unwrap()
        })
        .is_some_and(|disposition| {
            matches!(
                disposition.lifecycle,
                meld_lang::GoalLifecycle::Satisfied { .. }
            )
        })
}
