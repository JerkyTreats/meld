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
    advance_to_dispatch(&mut supervisor, &reader, &request);
    let (waiting_cursor, native_wake) = wait_for_graph_input(&assembly, &mut supervisor);
    let graph = assembly.graph_runtime();
    let waiting_generation = assembly
        .lifecycle_store()
        .unwrap()
        .current_generation(
            &assembly
                .prepared_activation()
                .unwrap()
                .assignment
                .assignment_id,
        )
        .unwrap()
        .unwrap();
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
    // The producer commits while Graph is held at its prior cursor. Neither
    // inspection nor wake resolution consumes the durable input on its behalf.
    let pending = assembly
        .ports()
        .event_replay()
        .read_after_limit(waiting_cursor.after_seq, 128)
        .unwrap();
    let nonce_events: Vec<_> = pending
        .iter()
        .filter(|record| record.envelope.event_type == crate::nonce::EVENT_TYPE)
        .collect();
    assert_eq!(nonce_events.len(), 1);
    let nonce_seq = nonce_events[0].seq;
    assert!(nonce_seq > waiting_cursor.after_seq);
    assert!(graph.resolves_wake(&native_wake).unwrap());
    assert_eq!(graph.durable_event_cursor().unwrap(), waiting_cursor);
    assert_eq!(lagged.generation_id, before.generation_id);
    assert_eq!(lagged.admission_epoch, before.admission_epoch);
    let mut resumed_from_wait = false;
    let mut consumed_nonce = false;
    for pass in 0..40 {
        let tick = supervisor.tick(1_100 + pass * 10).unwrap();
        for action in tick
            .actions
            .iter()
            .filter(|action| action.runtime_id == "world_model.graph_replay")
        {
            assert_eq!(action.generation_id, before.generation_id);
            let incarnation = action.incarnation_id.as_ref().unwrap();
            assert_eq!(
                incarnation,
                &waiting_generation.incarnations["world_model.graph_replay"].incarnation_id
            );
            for checkpoint in &action.checkpoints {
                resumed_from_wait |= checkpoint.input_value == waiting_cursor.after_seq
                    && checkpoint.output_value > waiting_cursor.after_seq;
                consumed_nonce |= checkpoint.output_value >= nonce_seq;
            }
        }
    }
    assert!(resumed_from_wait && consumed_nonce);
    assert!(graph.durable_event_cursor().unwrap().after_seq >= nonce_seq);
    assert_eq!(
        assembly
            .ports()
            .event_replay()
            .read_after_limit(waiting_cursor.after_seq, 1024)
            .unwrap()
            .iter()
            .filter(|record| record.envelope.event_type == crate::nonce::EVENT_TYPE)
            .count(),
        1
    );
    let complete = reader.inspect(&request).unwrap();
    assert_eq!(complete.generation_id, before.generation_id);
    assert_eq!(complete.admission_epoch, before.admission_epoch);
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
            agent_positions: Default::default(),
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
    let stopped = reader.inspect(&request).unwrap();
    assert_eq!(stopped.generation_id, recovered.generation_id);
    assert_eq!(stopped.nonce_id, recovered.nonce_id);
    assert_eq!(
        stopped.first_missing,
        Some(StartupPosition::GenerationCurrent)
    );
    assert!(stopped.positions.iter().any(|position| {
        position.position == StartupPosition::GoalSatisfied
            && position.state == EvidenceState::Available
    }));
    drop(reader);
    drop(supervisor);
    drop(graph);
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
    let stopped_successor = reader.inspect(&request).unwrap();
    assert_eq!(stopped_successor.generation_id, successor.generation_id);
    assert_eq!(stopped_successor.nonce_id, successor.nonce_id);
    assert_ne!(stopped_successor.nonce_id, stopped.nonce_id);
    assert_eq!(
        stopped_successor.first_missing,
        Some(StartupPosition::GenerationCurrent)
    );
}

#[test]
fn startup_reopens_committed_nonce_before_graph_consumption() {
    let harness = startup_harness();
    let request = StartupAccountRequest {
        agent_id: "startup-agent".into(),
        ..Default::default()
    };
    let (waiting_cursor, native_wake, nonce_record, prior, prior_generation);
    {
        let assembly = harness.assembly();
        harness.run_world_genesis_from(
            &assembly,
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/startup"),
        );
    }
    {
        let assembly = harness.assembly();
        harness.bind_production_routes_with_loss(
            &assembly,
            Some((
                Arc::new(std::sync::atomic::AtomicBool::new(true)),
                false,
                None,
            )),
        );
        let reader = StartupAccountReader::over_assembly(&assembly);
        let mut supervisor = harness.start_supervisor(&assembly);
        advance_to_dispatch(&mut supervisor, &reader, &request);
        (waiting_cursor, native_wake) = wait_for_graph_input(&assembly, &mut supervisor);
        supervisor.step_owner_for_test("execution.task_dispatch", WorkBudget { max_items: 1 });
        prior = reader.inspect(&request).unwrap();
        assert_eq!(prior.first_missing, Some(StartupPosition::GraphVisibility));
        assert_eq!(prior.parallel_obligations.len(), 1);
        prior_generation = assembly
            .lifecycle_store()
            .unwrap()
            .current_generation(
                &assembly
                    .prepared_activation()
                    .unwrap()
                    .assignment
                    .assignment_id,
            )
            .unwrap()
            .unwrap();
        let pending = assembly
            .ports()
            .event_replay()
            .read_after_limit(waiting_cursor.after_seq, 128)
            .unwrap();
        let nonces: Vec<_> = pending
            .into_iter()
            .filter(|record| record.envelope.event_type == crate::nonce::EVENT_TYPE)
            .collect();
        assert_eq!(nonces.len(), 1);
        nonce_record = nonces.into_iter().next().unwrap();
        assert_eq!(
            assembly.graph_runtime().durable_event_cursor().unwrap(),
            waiting_cursor
        );
        assembly.flush_product_boundary().unwrap();
        assembly.flush_supervisor_store().unwrap();
        // Drop the live supervisor without orderly shutdown. Recovery must
        // replace expired incarnations while preserving the unconsumed Event.
    }
    let assembly = harness.assembly();
    harness.bind_production_routes(&assembly);
    let graph = assembly.graph_runtime();
    assert_eq!(graph.durable_event_cursor().unwrap(), waiting_cursor);
    assert!(graph.resolves_wake(&native_wake).unwrap());
    let recovered_pending = assembly
        .ports()
        .event_replay()
        .read_after_limit(waiting_cursor.after_seq, 128)
        .unwrap();
    assert!(recovered_pending.contains(&nonce_record));
    let mut command = SupervisorStartCommand::new("startup-unconsumed-recovery", 1_000_000);
    command.registration_set = assembly.registration_set().cloned();
    let mut supervisor =
        RuntimeSupervisor::start(assembly.supervisor_startup_package(), command).unwrap();
    let reader = StartupAccountReader::over_assembly(&assembly);
    let current = reader.inspect(&request).unwrap();
    assert_eq!(current.generation_id, prior.generation_id);
    assert_ne!(current.admission_epoch, prior.admission_epoch);
    assert_eq!(
        current.first_missing,
        Some(StartupPosition::NonceInstantiated)
    );
    assert_eq!(graph.durable_event_cursor().unwrap(), waiting_cursor);
    let recovered_generation = assembly
        .lifecycle_store()
        .unwrap()
        .current_generation(
            &assembly
                .prepared_activation()
                .unwrap()
                .assignment
                .assignment_id,
        )
        .unwrap()
        .unwrap();
    assert_ne!(
        recovered_generation.incarnations["world_model.graph_replay"].incarnation_id,
        prior_generation.incarnations["world_model.graph_replay"].incarnation_id
    );
    let mut consumed = false;
    for pass in 0..40 {
        let tick = supervisor.tick(1_000_100 + pass * 10).unwrap();
        consumed |= tick.actions.iter().any(|action| {
            action.runtime_id == "world_model.graph_replay"
                && action.incarnation_id.as_ref()
                    == Some(
                        &recovered_generation.incarnations["world_model.graph_replay"]
                            .incarnation_id,
                    )
                && action.checkpoints.iter().any(|checkpoint| {
                    checkpoint.input_value == waiting_cursor.after_seq
                        && checkpoint.output_value >= nonce_record.seq
                })
        });
    }
    assert!(consumed);
    let complete = reader.inspect(&request).unwrap();
    assert_eq!(complete.first_missing, None, "{complete:?}");
    assert_ne!(complete.nonce_id, prior.nonce_id);
    let events = assembly
        .ports()
        .event_replay()
        .read_after_limit(waiting_cursor.after_seq, 1024)
        .unwrap();
    let nonces: Vec<_> = events
        .iter()
        .filter(|record| record.envelope.event_type == crate::nonce::EVENT_TYPE)
        .collect();
    assert_eq!(nonces.len(), 2);
    assert_eq!(
        nonces
            .iter()
            .filter(|record| ***record == nonce_record)
            .count(),
        1
    );
    supervisor.request_shutdown(1_000_600).unwrap();
}

fn advance_to_dispatch(
    supervisor: &mut RuntimeSupervisor<'_>,
    reader: &StartupAccountReader,
    request: &StartupAccountRequest,
) {
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
        if reader.inspect(request).unwrap().first_missing == Some(StartupPosition::AttemptRecorded)
        {
            admitted = true;
            break;
        }
    }
    assert!(admitted, "{:?}", reader.inspect(request).unwrap());
}

fn wait_for_graph_input(
    assembly: &ProductRuntimeAssembly,
    supervisor: &mut RuntimeSupervisor<'_>,
) -> (
    meld_events::LedgerCursor,
    meld_world_model::waiting::StructuralWakeAddress,
) {
    let graph = assembly.graph_runtime();
    let mut idle = None;
    for _ in 0..8 {
        let report = supervisor
            .step_owner_for_test("world_model.graph_replay", WorkBudget { max_items: 128 });
        assert!(report.fatal_errors.is_empty() && report.retryable_errors.is_empty());
        if report.items_attempted == 0 && !report.budget_exhausted {
            idle = Some(report);
            break;
        }
    }
    let idle = idle.expect("Graph must reach its durable waiting position before dispatch");
    let waiting_cursor = graph.durable_event_cursor().unwrap();
    let wake_address = format!(
        "event-ledger::{}::after::{}",
        waiting_cursor.ledger_id, waiting_cursor.after_seq
    );
    assert!(idle.waiting_on.iter().any(|wait| {
        wait.condition == "ledger_quiet_past_graph_cursor"
            && wait.wake_refs
                == vec![crate::runtime::lifecycle::StructuralWakeRef::EventPosition(
                    wake_address.clone(),
                )]
    }));
    let native_wake = meld_world_model::waiting::StructuralWakeAddress::EventPosition(wake_address);
    assert!(graph.resolves_wake(&native_wake).unwrap());
    (waiting_cursor, native_wake)
}
