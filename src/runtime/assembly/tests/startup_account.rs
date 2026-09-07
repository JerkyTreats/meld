//! Startup inspection follows independent native positions without advancing them.

use super::*;
use crate::harness::startup::{
    EvidenceState, StartupAccountReader, StartupAccountRequest, StartupPosition,
};

#[test]
fn startup_account_names_graph_lag_and_keeps_uncertain_execution_separate() {
    let harness = startup_harness();
    {
        let assembly = harness.assembly();
        harness.run_world_genesis_from(
            &assembly,
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/startup"),
        );
    }
    let assembly = harness.assembly();
    let loss = Arc::new(std::sync::atomic::AtomicBool::new(true));
    harness.bind_production_routes_with_loss(&assembly, Some((loss.clone(), false, None)));
    let reader = StartupAccountReader::over_assembly(&assembly);
    let request = StartupAccountRequest {
        agent_id: "startup-agent".into(),
        ..Default::default()
    };
    let prepared = reader.inspect(&request).unwrap();
    assert_eq!(prepared.read_state, EvidenceState::Available);
    assert_eq!(
        prepared.first_missing,
        Some(StartupPosition::GenerationCurrent)
    );
    assert!(assembly
        .stores()
        .agent_store
        .reconciliation_goals_for_agent("startup-agent")
        .unwrap()
        .is_empty());
    let mut supervisor = harness.start_supervisor(&assembly);
    assert_eq!(
        reader.inspect(&request).unwrap().first_missing,
        Some(StartupPosition::NonceInstantiated)
    );
    let mut admitted = false;
    for _ in 0..24 {
        for owner in [
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
        if reader.inspect(&request).unwrap().first_missing == Some(StartupPosition::AttemptRecorded)
        {
            admitted = true;
            break;
        }
    }
    assert!(admitted, "{:?}", reader.inspect(&request).unwrap());
    let before = reader.inspect(&request).unwrap();
    assert_eq!(reader.inspect(&request).unwrap(), before);
    supervisor.step_owner_for_test("execution.task_dispatch", WorkBudget { max_items: 1 });
    let lagged = reader.inspect(&request).unwrap();
    assert_eq!(
        lagged.first_missing,
        Some(StartupPosition::GraphVisibility),
        "{lagged:?}"
    );
    assert_eq!(lagged.parallel_obligations.len(), 1);
    assert!(lagged
        .positions
        .iter()
        .any(|p| p.position == StartupPosition::NonceEvent && p.state == EvidenceState::Available));
    assert_eq!(reader.inspect(&request).unwrap(), lagged);
    for pass in 0..40 {
        supervisor.tick(1_100 + pass * 10).unwrap();
    }
    let complete = reader.inspect(&request).unwrap();
    assert_eq!(complete.read_state, EvidenceState::Available);
    assert_eq!(complete.first_missing, None, "{complete:?}");
    assert_eq!(complete.positions.len(), 19);
    assert_eq!(complete.parallel_obligations.len(), 1);
    assert!(complete.execution_outcomes.is_empty());
    let exact = StartupAccountRequest {
        agent_id: request.agent_id.clone(),
        generation_id: complete.generation_id.clone(),
        admission_epoch: complete.admission_epoch.clone(),
        nonce_id: complete.nonce_id.clone(),
        inspection_fence: Some(complete.inspection_fence.clone()),
    };
    assert_eq!(reader.inspect(&exact).unwrap(), complete);
    let sources = crate::serve::sources::ServeSources::from_assembly(&assembly).unwrap();
    let addressed = crate::serve::routes::StartupAccountReadRequest {
        product_root: assembly.product_root().to_path_buf(),
        request: exact.clone(),
    };
    let response = crate::serve::routes::dispatch(
        &sources,
        "POST",
        "/v1/projections/startup_nonce_account",
        &serde_json::to_vec(&addressed).unwrap(),
    );
    assert_eq!(response.status, 200);
    let mut foreign_root = addressed.clone();
    foreign_root.product_root = harness._external.path().join("foreign-product");
    assert_ne!(
        crate::serve::routes::dispatch(
            &sources,
            "POST",
            "/v1/projections/startup_nonce_account",
            &serde_json::to_vec(&foreign_root).unwrap()
        )
        .status,
        200
    );
    let server =
        crate::serve::listener::serve_with_discovery(sources, 0, assembly.product_root()).unwrap();
    let mut config = MerkleConfig::default();
    config.system.storage.product_root = Some(harness.binding.storage_root.clone());
    let package = &harness.binding.package;
    config.stewardship.declarations.insert(
        "startup".into(),
        crate::config::StewardshipDeclaration {
            bindings: Default::default(),
            expression: package.expression.clone(),
            target_root: None,
            subject: harness.binding.subject.clone(),
            agent_id: harness.binding.agent_id.clone(),
            principal_id: package.principal_id.clone(),
            provider_id: None,
            theory: TheorySelection {
                belief_family_id: package.belief_family_id.clone(),
                evidence_mapping_id: package.evidence_mapping_id.clone(),
                curation_rule_id: package.curation_rule_id.clone(),
                maintained_condition_id: package.maintained_condition_id.clone(),
                strategy_theory_id: package.strategy_theory_id.clone(),
                authority_policy_id: package.authority_policy_id.clone(),
                claim_policy_id: String::new(),
            },
        },
    );
    let live = crate::runtime::tooling::try_live_startup_account(
        harness._workspace.path(),
        &config,
        &exact,
        "json",
    )
    .expect("live read must use the already-open product")
    .unwrap();
    assert_eq!(
        serde_json::from_str::<crate::harness::startup::StartupNonceAccount>(&live).unwrap(),
        complete
    );
    drop(server);
    let cli = crate::cli::RuntimeCommands::StartupAccount {
        agent_id: exact.agent_id.clone(),
        generation_id: exact.generation_id.clone(),
        admission_epoch: exact.admission_epoch.clone(),
        nonce_id: exact.nonce_id.clone(),
        inspection_fence: exact.inspection_fence.clone(),
        format: "json".into(),
    };
    let output = crate::runtime::tooling::handle_cli_command(&assembly, &cli).unwrap();
    assert_eq!(
        serde_json::from_str::<crate::harness::startup::StartupNonceAccount>(&output).unwrap(),
        complete
    );
    let mut foreign = exact.clone();
    foreign.nonce_id = Some("another-nonce".into());
    foreign.inspection_fence = None;
    let conflict = reader.inspect(&foreign).unwrap();
    assert_eq!(
        conflict.first_missing,
        Some(StartupPosition::NonceInstantiated)
    );
    assert!(conflict
        .positions
        .iter()
        .any(|p| p.position == StartupPosition::NonceInstantiated
            && p.state == EvidenceState::Conflicted));
    assert_eq!(reader.inspect(&exact).unwrap(), complete);
    loss.store(false, std::sync::atomic::Ordering::SeqCst);
    for pass in 0..8 {
        supervisor.tick(2_000 + pass * 10).unwrap();
    }
    let recovered = reader.inspect(&request).unwrap();
    assert_eq!(recovered.first_missing, None);
    assert!(recovered.parallel_obligations.is_empty());
    assert_eq!(recovered.execution_outcomes.len(), 1);
    assert_eq!(
        reader.inspect(&exact).unwrap().read_state,
        EvidenceState::Stale
    );
    supervisor.request_shutdown(3_000).unwrap();
    drop(reader);
    drop(supervisor);
    drop(assembly);
    let assembly = harness.assembly();
    harness.bind_production_routes(&assembly);
    let mut command = SupervisorStartCommand::new("startup-account-recovery", 1_000_000);
    command.registration_set = assembly.registration_set().cloned();
    let mut supervisor =
        RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
    let reader = StartupAccountReader::over_assembly(&assembly);
    let fresh = reader.inspect(&request).unwrap();
    assert_ne!(fresh.admission_epoch, complete.admission_epoch);
    assert_eq!(
        fresh.first_missing,
        Some(StartupPosition::NonceInstantiated)
    );
    let mut historical = exact;
    historical.inspection_fence = None;
    let old = reader.inspect(&historical).unwrap();
    assert_eq!(old.nonce_id, complete.nonce_id);
    assert_eq!(old.first_missing, Some(StartupPosition::GenerationCurrent));
    for pass in 0..40 {
        supervisor.tick(1_000_100 + pass * 10).unwrap();
    }
    let successor = reader.inspect(&request).unwrap();
    assert_ne!(successor.nonce_id, complete.nonce_id);
    assert_eq!(successor.first_missing, None, "{successor:?}");
    supervisor.request_shutdown(1_000_600).unwrap();
}
