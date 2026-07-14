use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use meld::config::ConfigLoader;
use meld::runtime::activation::SemanticRuntimeSelection;
use meld::runtime::assembly::{
    ProductRuntimeAssembly, RuntimeFactoryRegistry, RuntimeLeaseContext,
};
use meld::runtime::contracts::{RuntimeImplementationState, RuntimeRoleClass, WorkBudget};
use meld::runtime::supervisor::{
    RestartCause, RuntimeId, RuntimeInstanceStatus, RuntimeSupervisor, SupervisorStartCommand,
    SupervisorStore,
};
use meld::runtime::tooling::{handle_cli_activation, handle_cli_activation_run};
use meld_events::{
    AppendMode, DomainObjectRef, EventAuthority, EventAuthorityOpenOptions, EventEnvelope,
    EventRelation, LedgerCursor, ReplayRequest,
};
use meld_world_model::{
    AgentBootstrapProgressStatus, AgentStatus, AgentStore, AgentSubscriptionStatus, BeliefStore,
};
use serde_json::Value;

use super::{create_test_agent, create_test_provider, spawn_docs_writer_server, with_xdg_env};

const ACTIVATION: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/runtime/docs_freshness_activation.toml"
));

#[test]
fn supervised_activation_applies_replays_and_reopens_without_recurring_work() {
    let temp = tempfile::tempdir().unwrap();
    with_xdg_env(&temp, || {
        let workspace = workspace(&temp);
        let config_path = write_config(temp.path(), "docs-writer");
        let activation_path = workspace.join("docs_freshness_activation.toml");
        fs::write(&activation_path, ACTIVATION).unwrap();

        let registry = RuntimeFactoryRegistry::first_proof_registry().unwrap();
        let descriptor = registry
            .get("world_model.agent.bootstrap.docs_freshness")
            .unwrap();
        assert!(!descriptor.default_enabled);
        assert_eq!(descriptor.role_class, RuntimeRoleClass::Actor);
        assert_eq!(
            descriptor.implementation_state,
            RuntimeImplementationState::Concrete
        );

        let first = activate(&workspace, &config_path, &activation_path).unwrap();
        assert_eq!(first["application_ready"], true);
        assert_eq!(
            first["validation_scope"],
            "source_owner_packages_execution_assets_and_durable_bootstrap"
        );
        assert!(first["diagnostics"]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["code"] == "supervised_bootstrap_verified"));

        let config = ConfigLoader::load_from_file(&config_path).unwrap();
        let product_root = config
            .system
            .storage
            .resolve_product_root(&workspace)
            .unwrap();
        let layout = meld::runtime::storage::ProductStorageLayout::from_root(&product_root);
        let branch_runtime = meld::branches::BranchRuntime::new();
        let active_branch = branch_runtime.resolve_active_branch(&workspace).unwrap();
        assert!(active_branch.resolved().manifest_path.exists());
        let branch_status = branch_runtime.status().unwrap();
        assert!(branch_status.branches.iter().any(|branch| {
            branch.branch_id == active_branch.resolved().branch_id
                && branch.canonical_locator
                    == active_branch.resolved().canonical_locator.to_string_lossy()
        }));
        let first_receipt = read_receipt(&layout.world_model_db);
        assert_eq!(
            first_receipt.activation_hash,
            first["activation_hash"].as_str().unwrap()
        );
        assert_eq!(first_receipt.activation_id, "docs_freshness");
        assert_eq!(
            first_receipt.bootstrap_id,
            "world_model.agent.bootstrap.docs_freshness"
        );
        let first_progress = read_progress(&layout.world_model_db);
        assert_eq!(
            first_progress.status,
            AgentBootstrapProgressStatus::Completed
        );

        let replay = activate(&workspace, &config_path, &activation_path).unwrap();
        assert_eq!(replay["application_ready"], true);
        let replay_receipt = read_receipt(&layout.world_model_db);
        assert_eq!(replay_receipt, first_receipt);
        assert_eq!(read_progress(&layout.world_model_db), first_progress);

        let agent_store = AgentStore::new(sled::open(&layout.world_model_db).unwrap()).unwrap();
        let agent = agent_store
            .get_agent("seed.docs_freshness")
            .unwrap()
            .unwrap();
        assert_eq!(agent.status, AgentStatus::Registered);
        assert!(agent_store
            .activations_for_agent("seed.docs_freshness")
            .unwrap()
            .is_empty());
        assert!(agent_store
            .get_directive("directive.docs_freshness")
            .unwrap()
            .is_some());
        let rule = agent_store
            .get_curation_rule("curation.docs_freshness.threshold")
            .unwrap()
            .unwrap();
        assert_eq!(rule.agent_id, "seed.docs_freshness");
        assert_eq!(rule.config.dimension_id, "docs_freshness");
        let subscription = agent_store
            .get_subscription(&first_receipt.subscription_id)
            .unwrap()
            .unwrap();
        assert_eq!(subscription.agent_id, "seed.docs_freshness");
        assert_eq!(subscription.status, AgentSubscriptionStatus::Active);
        assert_eq!(subscription.belief_key.dimension_id, "docs_freshness");
        drop(agent_store);

        let belief_store = BeliefStore::new(sled::open(&layout.world_model_db).unwrap()).unwrap();
        assert!(belief_store
            .get_config_snapshot(&first_receipt.belief.config_snapshot_hash)
            .unwrap()
            .is_some());
        drop(belief_store);

        let supervisor = SupervisorStore::open(product_root.join("supervisor.sled")).unwrap();
        let enabled = supervisor
            .list_desired_runtime_state()
            .unwrap()
            .into_iter()
            .filter(|state| state.enabled)
            .map(|state| state.runtime_id.to_string())
            .collect::<Vec<_>>();
        assert_eq!(
            enabled,
            vec![
                "world_model.agent.bootstrap.docs_freshness".to_string(),
                "world_model.graph_replay".to_string(),
            ]
        );
        let instance = supervisor.latest_runtime_instance().unwrap().unwrap();
        assert_eq!(instance.status, RuntimeInstanceStatus::Stopped);
        for runtime_id in enabled {
            assert!(supervisor
                .get_active_runtime_lease(&RuntimeId::new(runtime_id).unwrap())
                .unwrap()
                .is_none());
        }
        drop(supervisor);

        assert!(layout.task_networks_root.exists());
        assert!(layout
            .task_networks_root
            .read_dir()
            .unwrap()
            .next()
            .is_none());

        let reopened = AgentStore::new(sled::open(&layout.world_model_db).unwrap()).unwrap();
        assert_eq!(
            reopened
                .get_bootstrap_receipt("world_model.agent.bootstrap.docs_freshness")
                .unwrap()
                .unwrap(),
            first_receipt
        );
    });
}

#[test]
fn bootstrap_handle_reports_bounded_first_work_and_exact_replay_checkpoints() {
    let temp = tempfile::tempdir().unwrap();
    with_xdg_env(&temp, || {
        let workspace = workspace(&temp);
        let config_path = write_config(temp.path(), "docs-writer");
        let activation_path = workspace.join("docs_freshness_activation.toml");
        fs::write(&activation_path, ACTIVATION).unwrap();
        let prepared = meld::runtime::activation::load_and_prepare_activation(
            &workspace,
            &activation_path,
            Some(&config_path),
        )
        .unwrap();
        let expected_execution_input = prepared.preflight.execution_input.clone();
        let expected_execution_receipt = prepared.preflight.execution_receipt.clone();
        let product_root = prepared
            .repository_config
            .system
            .storage
            .resolve_product_root(&workspace)
            .unwrap();
        let layout = meld::runtime::storage::ProductStorageLayout::from_root(&product_root);
        fs::create_dir_all(&layout.ledger_db).unwrap();
        let authority = std::sync::Arc::new(
            EventAuthority::open(
                sled::open(&layout.ledger_db).unwrap(),
                EventAuthorityOpenOptions::default(),
            )
            .unwrap(),
        );
        let owner = prepared.preflight.activation.runtime_inputs;
        let assembly = ProductRuntimeAssembly::load_activated_with_authority(
            &workspace,
            &prepared.repository_config,
            authority,
            owner.runtime,
            owner.world_model,
            prepared.preflight.execution_input,
            prepared.preflight.execution_receipt,
        )
        .unwrap();
        let retained_execution = assembly.activation_execution().unwrap();
        assert_eq!(retained_execution.input, expected_execution_input);
        assert_eq!(retained_execution.receipt, expected_execution_receipt);
        let configured_network = assembly.configured_task_network().unwrap();
        assert_eq!(configured_network.network_id(), "network-docs");
        assert_eq!(
            configured_network.storage_key(),
            retained_execution.receipt.task_network_storage_key
        );
        let task_network_service = assembly
            .desired_runtime_state()
            .iter()
            .find(|state| state.runtime_id == "execution.task_network_command")
            .unwrap();
        assert_eq!(
            task_network_service.role_class,
            RuntimeRoleClass::PassiveService
        );
        assert!(!task_network_service.enabled);
        assert!(layout
            .task_networks_root
            .read_dir()
            .unwrap()
            .next()
            .is_none());
        let mut handle = assembly
            .handle_factories()
            .get("world_model.agent.bootstrap.docs_freshness")
            .unwrap()
            .build_handle();
        handle
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: "world_model.agent.bootstrap.docs_freshness".to_string(),
                lease_id: "lease-bootstrap-report".to_string(),
            })
            .unwrap();

        let first = handle.tick(WorkBudget { max_items: 1 }).unwrap();
        assert_eq!(first.actor_id, "world_model.agent.bootstrap.docs_freshness");
        assert_eq!(first.input_checkpoint.name, "agent_bootstrap_sequence");
        assert_eq!(first.output_checkpoint.name, "agent_bootstrap_sequence");
        assert!(first.output_checkpoint.value > first.input_checkpoint.value);
        assert_eq!(first.items_attempted, 1);
        assert_eq!(first.items_committed, 1);
        assert!(first.fatal_errors.is_empty());

        let replay = handle.tick(WorkBudget { max_items: 1 }).unwrap();
        assert_eq!(replay.input_checkpoint, replay.output_checkpoint);
        assert_eq!(replay.input_checkpoint.value, first.output_checkpoint.value);
        assert_eq!(replay.items_attempted, 1);
        assert_eq!(replay.items_committed, 0);
        assert!(replay.fatal_errors.is_empty());
    });
}

#[test]
fn divergent_activation_fails_closed_and_preserves_the_first_receipt() {
    let temp = tempfile::tempdir().unwrap();
    with_xdg_env(&temp, || {
        let workspace = workspace(&temp);
        let config_path = write_config(temp.path(), "docs-writer");
        let activation_path = workspace.join("docs_freshness_activation.toml");
        fs::write(&activation_path, ACTIVATION).unwrap();
        activate(&workspace, &config_path, &activation_path).unwrap();

        let config = ConfigLoader::load_from_file(&config_path).unwrap();
        let product_root = config
            .system
            .storage
            .resolve_product_root(&workspace)
            .unwrap();
        let layout = meld::runtime::storage::ProductStorageLayout::from_root(product_root);
        let receipt = read_receipt(&layout.world_model_db);

        fs::write(
            &activation_path,
            ACTIVATION.replace(
                "curate docs freshness goals",
                "curate divergent docs freshness goals",
            ),
        )
        .unwrap();
        let error = activate(&workspace, &config_path, &activation_path).unwrap_err();
        assert!(error.to_string().contains("agent_bootstrap_conflict"));
        assert!(error
            .to_string()
            .contains("bootstrap content conflict at 'bootstrap.activation_hash'"));
        assert_eq!(read_receipt(&layout.world_model_db), receipt);

        let supervisor = SupervisorStore::open(layout.root.join("supervisor.sled")).unwrap();
        assert_eq!(
            supervisor
                .latest_runtime_instance()
                .unwrap()
                .unwrap()
                .status,
            RuntimeInstanceStatus::Stopped
        );
    });
}

#[test]
fn retryable_bootstrap_exhaustion_is_bounded_and_shuts_down_cleanly() {
    let temp = tempfile::tempdir().unwrap();
    with_xdg_env(&temp, || {
        let workspace = workspace(&temp);
        let config_path = write_config(temp.path(), "docs-writer");
        let activation_path = workspace.join("docs_freshness_activation.toml");
        fs::write(&activation_path, ACTIVATION).unwrap();

        let config = ConfigLoader::load_from_file(&config_path).unwrap();
        let product_root = config
            .system
            .storage
            .resolve_product_root(&workspace)
            .unwrap();
        let layout = meld::runtime::storage::ProductStorageLayout::from_root(&product_root);
        let db = sled::open(&layout.world_model_db).unwrap();
        db.open_tree("agent_bootstrap_progress")
            .unwrap()
            .insert(
                "world_model.agent.bootstrap.docs_freshness",
                b"not valid bootstrap progress".as_slice(),
            )
            .unwrap();
        db.flush().unwrap();
        drop(db);

        let error = activate(&workspace, &config_path, &activation_path).unwrap_err();
        let message = error.to_string();
        assert!(message.contains("exhausted 3 attempts"));
        assert!(message.contains("agent_bootstrap_storage_failed"));
        assert!(message.contains("bootstrap storage failure"));

        let supervisor = SupervisorStore::open(layout.root.join("supervisor.sled")).unwrap();
        let schedules = supervisor.list_restart_schedules().unwrap();
        assert_eq!(schedules.len(), 2);
        assert!(schedules.iter().all(|schedule| {
            schedule.restart().runtime_id.as_str() == "world_model.agent.bootstrap.docs_freshness"
                && schedule.restart().cause == RestartCause::RetryableFailure
        }));
        assert_eq!(
            supervisor
                .latest_runtime_instance()
                .unwrap()
                .unwrap()
                .status,
            RuntimeInstanceStatus::Stopped
        );
        for runtime_id in [
            "world_model.agent.bootstrap.docs_freshness",
            "world_model.graph_replay",
        ] {
            assert!(supervisor
                .get_active_runtime_lease(&RuntimeId::new(runtime_id).unwrap())
                .unwrap()
                .is_none());
        }
    });
}

#[test]
fn invalid_provider_and_runtime_fail_before_product_store_creation() {
    let temp = tempfile::tempdir().unwrap();
    with_xdg_env(&temp, || {
        let workspace = workspace(&temp);
        let activation_path = workspace.join("docs_freshness_activation.toml");
        fs::write(&activation_path, ACTIVATION).unwrap();
        let invalid_provider_config = write_config(temp.path(), "different-provider");
        let config = ConfigLoader::load_from_file(&invalid_provider_config).unwrap();
        let product_root = config
            .system
            .storage
            .resolve_product_root(&workspace)
            .unwrap();
        let branch_data_home = meld::branches::BranchRuntime::new()
            .resolve_active_branch(&workspace)
            .unwrap()
            .resolved()
            .data_home_path
            .clone();
        let branch_catalog = meld::config::xdg::data_home()
            .unwrap()
            .join("meld")
            .join("branch_catalog.json");

        assert!(activate(&workspace, &invalid_provider_config, &activation_path).is_err());
        assert!(!product_root.exists());
        assert!(!branch_data_home.exists());
        assert!(!branch_catalog.exists());

        let valid_config = write_config(temp.path(), "docs-writer");
        fs::write(
            &activation_path,
            ACTIVATION.replace("world_model.graph_replay", "world_model.graph.replay"),
        )
        .unwrap();
        assert!(activate(&workspace, &valid_config, &activation_path).is_err());
        assert!(!product_root.exists());
        assert!(!branch_data_home.exists());
        assert!(!branch_catalog.exists());
    });
}

#[test]
fn static_activation_gate_rejects_receipt_tampering_and_runtime_aliases() {
    let temp = tempfile::tempdir().unwrap();
    with_xdg_env(&temp, || {
        let workspace = workspace(&temp);
        let config_path = write_config(temp.path(), "docs-writer");
        let activation_path = workspace.join("docs_freshness_activation.toml");
        fs::write(&activation_path, ACTIVATION).unwrap();
        let prepared = meld::runtime::activation::load_and_prepare_activation(
            &workspace,
            &activation_path,
            Some(&config_path),
        )
        .unwrap();
        let owner = &prepared.preflight.activation.runtime_inputs;
        ProductRuntimeAssembly::validate_activated_inputs(
            &owner.runtime,
            &owner.world_model,
            &prepared.preflight.execution_input,
            &prepared.preflight.execution_receipt,
        )
        .unwrap();

        let mut tampered_receipt = prepared.preflight.execution_receipt.clone();
        tampered_receipt.input_hash = "0".repeat(64);
        assert!(ProductRuntimeAssembly::validate_activated_inputs(
            &owner.runtime,
            &owner.world_model,
            &prepared.preflight.execution_input,
            &tampered_receipt,
        )
        .is_err());

        let mut aliased_runtime = owner.runtime.clone();
        aliased_runtime.enabled_runtime_ids[1] = "world_model.graph.replay".to_string();
        assert!(ProductRuntimeAssembly::validate_activated_inputs(
            &aliased_runtime,
            &owner.world_model,
            &prepared.preflight.execution_input,
            &prepared.preflight.execution_receipt,
        )
        .is_err());

        let product_root = prepared
            .repository_config
            .system
            .storage
            .resolve_product_root(&workspace)
            .unwrap();
        assert!(!product_root.exists());
    });
}

#[test]
fn semantic_runtime_reaches_planned_work_from_initial_observation() {
    let temp = tempfile::tempdir().unwrap();
    with_xdg_env(&temp, || {
        let root = SemanticRoot::activate(temp.path(), "flywheel");
        let evidence = drive_semantic_to_planned_work(&root, &root.actor_order(), "flywheel");

        assert!(evidence.hydration_checkpoint_transition, "{evidence:?}");
        for runtime_id in [
            "world_model.belief_assessment",
            "world_model.agent_hydration",
            "world_model.agent_goal_curation",
            "world_model.planner_projection",
            "execution.planning",
        ] {
            assert!(
                evidence
                    .committed_actor_ids
                    .contains(&runtime_id.to_string()),
                "{runtime_id}: {evidence:?}"
            );
        }

        let snapshot = root.snapshot();
        assert!(!snapshot.agent["agent_bootstrap_receipts"].is_empty());
        assert!(!snapshot.agent["agent_process_hydrations"].is_empty());
        assert!(!snapshot.belief["belief_activation_receipts"].is_empty());
        assert!(!snapshot.belief["belief_revisions"].is_empty());
        assert!(!snapshot.goal["execution_goal_records"].is_empty());
        assert!(!snapshot.planner_projection["planner_projection_frames"].is_empty());
        assert!(!snapshot.task_network["task_network_latest_state"].is_empty());
    });
}

#[test]
fn supervised_semantic_runtime_completes_one_docs_freshness_flywheel_turn() {
    let temp = tempfile::tempdir().unwrap();
    with_xdg_env(&temp, || {
        let (endpoint, provider_server) = spawn_docs_writer_server(4);
        create_test_provider("docs-writer", &endpoint);
        let root = SemanticRoot::activate(temp.path(), "flywheel-e2e");
        let assembly = root.open();
        let started_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let mut supervisor = RuntimeSupervisor::start(
            assembly.supervisor_startup_package(),
            SupervisorStartCommand::new("flywheel-e2e", started_at_ms),
        )
        .unwrap();

        for offset in 1..=24 {
            supervisor.tick(started_at_ms + offset).unwrap();
        }
        supervisor.request_shutdown(started_at_ms + 25).unwrap();
        drop(supervisor);
        assembly.flush_product_boundary().unwrap();
        drop(assembly);

        assert_eq!(provider_server.join().unwrap(), 4);
        let readme = fs::read_to_string(root.workspace.join("README.md")).unwrap();
        assert!(readme.contains("# Workspace Library"));

        let snapshot = root.snapshot();
        assert!(snapshot.belief["belief_revisions"].len() >= 2);
        assert!(snapshot.goal["execution_goal_records"]
            .iter()
            .any(|(_, record)| record.contains("\"Satisfied\"")));
        let task_network = &snapshot.task_network["task_network_latest_state"][0].1;
        assert!(task_network.contains("docs_patch"));
        assert!(task_network.contains("Published"));
        assert!(snapshot
            .events
            .iter()
            .any(|event| event.contains("execution.task.succeeded")));
    });
}

#[test]
fn activated_runtime_run_command_completes_one_docs_freshness_flywheel_turn() {
    let temp = tempfile::tempdir().unwrap();
    with_xdg_env(&temp, || {
        meld::init::initialize_workflows(false).unwrap();
        create_test_agent("docs-writer", Some("docs_writer_thread_v1"));
        let (endpoint, provider_server) = spawn_docs_writer_server(4);
        create_test_provider("docs-writer", &endpoint);

        let workspace = workspace(&temp);
        fs::write(workspace.join("lib.rs"), "pub fn flywheel() {}\n").unwrap();
        let config_path = write_config(temp.path(), "docs-writer");
        let activation_path = workspace.join("docs_freshness_activation.toml");
        fs::write(&activation_path, ACTIVATION).unwrap();

        let output = handle_cli_activation_run(
            &workspace,
            Some(&config_path),
            &activation_path,
            Some("flywheel-cli-e2e".to_string()),
            1,
            Some(1_000),
            "json",
            "never",
            1,
            0,
        )
        .unwrap();
        let result: Value = serde_json::from_str(&output).unwrap();

        assert_eq!(result["instance_id"], "flywheel-cli-e2e");
        assert!(result["tick_count"].as_u64().unwrap() > 0);
        let readme = fs::read_to_string(workspace.join("README.md")).unwrap();
        assert!(readme.contains("# Workspace Library"));
        assert_eq!(provider_server.join().unwrap(), 4);

        let config = ConfigLoader::load_from_file(&config_path).unwrap();
        let product_root = config
            .system
            .storage
            .resolve_product_root(&workspace)
            .unwrap();
        let layout = meld::runtime::storage::ProductStorageLayout::from_root(&product_root);
        let normalizer = SnapshotNormalizer {
            fixture_root: temp.path(),
            workspace: &workspace,
            product_root: &product_root,
            identity_replacements: &[],
        };
        let belief = snapshot_sled(&layout.world_model_db, &["belief_"], &[], &normalizer);
        let goals = snapshot_sled(
            &layout.execution_goals_db,
            &["execution_goal_"],
            &[],
            &normalizer,
        );
        let task_network = snapshot_sled(
            &layout.task_networks_root.join("network-docs.sled"),
            &["task_network_"],
            &["task_network_authority_lifecycle"],
            &normalizer,
        );
        assert!(belief["belief_revisions"].len() >= 2);
        assert!(goals["execution_goal_records"]
            .iter()
            .any(|(_, record)| record.contains("\"Satisfied\"")));
        assert!(task_network["task_network_latest_state"]
            .iter()
            .any(|(_, state)| state.contains("docs_patch") && state.contains("Published")));
        assert!(snapshot_events(&layout.ledger_db, &normalizer)
            .iter()
            .any(|event| event.contains("execution.task.succeeded")));
    });
}

#[test]
fn activated_runtime_run_rejects_invalid_options_before_product_mutation() {
    let temp = tempfile::tempdir().unwrap();
    with_xdg_env(&temp, || {
        let workspace = workspace(&temp);
        let config_path = write_config(temp.path(), "docs-writer");
        let activation_path = workspace.join("docs_freshness_activation.toml");
        fs::write(&activation_path, ACTIVATION).unwrap();
        let config = ConfigLoader::load_from_file(&config_path).unwrap();
        let product_root = config
            .system
            .storage
            .resolve_product_root(&workspace)
            .unwrap();

        let error = handle_cli_activation_run(
            &workspace,
            Some(&config_path),
            &activation_path,
            None,
            0,
            Some(1),
            "json",
            "never",
            1,
            0,
        )
        .unwrap_err();

        assert!(error.to_string().contains("tick-ms must be greater than 0"));
        assert!(!product_root.exists());
    });
}

fn drive_semantic_to_planned_work(
    root: &SemanticRoot,
    actor_runtime_ids: &[String],
    lease_epoch: &str,
) -> SemanticRunEvidence {
    let mut evidence = SemanticRunEvidence::default();
    for round in 1..=128 {
        let round_evidence = run_semantic_round(root, actor_runtime_ids, lease_epoch);
        evidence.absorb(round_evidence);
        evidence.rounds = round;
        let snapshot = root.snapshot();
        if semantic_path_has_planned_work(&evidence, &snapshot) {
            evidence.committed_actor_ids.sort();
            return evidence;
        }
    }
    panic!("semantic products did not reach planned work: {evidence:?}");
}

fn semantic_path_has_planned_work(
    evidence: &SemanticRunEvidence,
    snapshot: &SemanticDurableSnapshot,
) -> bool {
    [
        "world_model.belief_assessment",
        "world_model.agent_hydration",
        "world_model.agent_goal_curation",
        "world_model.planner_projection",
        "execution.planning",
    ]
    .iter()
    .all(|runtime_id| {
        evidence
            .committed_actor_ids
            .contains(&runtime_id.to_string())
    }) && snapshot_tree_nonempty(&snapshot.agent, "agent_process_hydrations")
        && snapshot_tree_nonempty(&snapshot.belief, "belief_revisions")
        && snapshot_tree_nonempty(&snapshot.goal, "execution_goal_records")
        && snapshot_tree_nonempty(&snapshot.planner_projection, "planner_projection_frames")
        && !snapshot.planning_attempt.is_empty()
        && snapshot_tree_nonempty(&snapshot.task_network, "task_network_latest_state")
        && !snapshot.events.is_empty()
}

fn snapshot_tree_nonempty(snapshot: &BTreeMap<String, Vec<(String, String)>>, tree: &str) -> bool {
    snapshot
        .get(tree)
        .is_some_and(|entries| !entries.is_empty())
}

#[derive(Debug, Default)]
struct SemanticRunEvidence {
    rounds: usize,
    items_attempted: usize,
    items_committed: usize,
    last_round_committed: usize,
    readiness_waits: usize,
    hydration_checkpoint_transition: bool,
    committed_actor_ids: Vec<String>,
}

impl SemanticRunEvidence {
    fn absorb(&mut self, other: Self) {
        self.items_attempted += other.items_attempted;
        self.items_committed += other.items_committed;
        self.last_round_committed = other.items_committed;
        self.readiness_waits += other.readiness_waits;
        self.hydration_checkpoint_transition |= other.hydration_checkpoint_transition;
        for runtime_id in other.committed_actor_ids {
            if !self.committed_actor_ids.contains(&runtime_id) {
                self.committed_actor_ids.push(runtime_id);
            }
        }
    }
}

fn run_semantic_round(
    root: &SemanticRoot,
    actor_runtime_ids: &[String],
    lease_epoch: &str,
) -> SemanticRunEvidence {
    let assembly = root.open();
    let service_state = assembly
        .desired_runtime_state()
        .iter()
        .find(|state| state.runtime_id == "execution.task_network_command")
        .unwrap();
    assert!(service_state.enabled);
    assert_eq!(service_state.role_class, RuntimeRoleClass::PassiveService);
    assert_eq!(
        service_state.implementation_state,
        RuntimeImplementationState::Concrete
    );
    let mut service = assembly
        .handle_factories()
        .get("execution.task_network_command")
        .unwrap()
        .build_handle();
    assert!(service.tick(WorkBudget { max_items: 1 }).is_none());
    service
        .start_after_lease(RuntimeLeaseContext {
            runtime_id: "execution.task_network_command".to_string(),
            lease_id: format!("lease-{lease_epoch}-task-network"),
        })
        .unwrap();
    assert!(service.poll_passive_health().unwrap().healthy);

    let mut actors = BTreeMap::new();
    for runtime_id in actor_runtime_ids {
        let state = assembly
            .desired_runtime_state()
            .iter()
            .find(|state| &state.runtime_id == runtime_id)
            .unwrap();
        assert!(state.enabled);
        assert_eq!(state.role_class, RuntimeRoleClass::Actor);
        assert_eq!(
            state.implementation_state,
            RuntimeImplementationState::Concrete
        );
        let mut actor = assembly
            .handle_factories()
            .get(runtime_id)
            .unwrap()
            .build_handle();
        assert!(actor.tick(WorkBudget { max_items: 1 }).is_none());
        actor
            .start_after_lease(RuntimeLeaseContext {
                runtime_id: runtime_id.clone(),
                lease_id: format!("lease-{lease_epoch}-{runtime_id}"),
            })
            .unwrap();
        actors.insert(runtime_id.clone(), actor);
    }
    assert!(service.poll_passive_health().unwrap().healthy);

    let mut evidence = SemanticRunEvidence {
        rounds: 1,
        ..SemanticRunEvidence::default()
    };
    for runtime_id in actor_runtime_ids {
        let actor = actors.get_mut(runtime_id).unwrap();
        let report = actor.tick(WorkBudget { max_items: 1 }).unwrap();
        let expected_actor_id = if runtime_id == "world_model.graph_replay" {
            "world_state.graph.reducer"
        } else {
            runtime_id
        };
        assert_eq!(report.actor_id, expected_actor_id);
        assert!(report.fatal_errors.is_empty(), "{runtime_id}: {report:?}");
        for issue in &report.retryable_errors {
            assert!(
                matches!(
                    (runtime_id.as_str(), issue.code.as_str()),
                    ("world_model.agent_hydration", "belief_not_ready")
                        | ("world_model.belief_assessment", "missing_graph_anchor")
                        | ("execution.planning", "planning_projection_failed")
                ),
                "unexpected retryable issue from {runtime_id}: {report:?}"
            );
            if runtime_id == "world_model.agent_hydration" {
                evidence.readiness_waits += 1;
            }
        }
        if runtime_id == "world_model.agent_hydration"
            && report.input_checkpoint != report.output_checkpoint
        {
            evidence.hydration_checkpoint_transition = true;
        }
        evidence.items_attempted += report.items_attempted;
        evidence.items_committed += report.items_committed;
        evidence.last_round_committed += report.items_committed;
        if report.items_committed > 0 && !evidence.committed_actor_ids.contains(runtime_id) {
            evidence.committed_actor_ids.push(runtime_id.clone());
        }
    }

    for runtime_id in actor_runtime_ids.iter().rev() {
        let actor = actors.get_mut(runtime_id).unwrap();
        assert!(actor.request_stop().was_started);
        assert!(actor.wait_for_safe_point().safe_for_flush);
        actor.flush_resources().unwrap();
    }
    assert!(service.poll_passive_health().unwrap().healthy);
    assert!(service.request_stop().was_started);
    assert!(service.wait_for_safe_point().safe_for_flush);
    service.flush_resources().unwrap();
    assembly.flush_product_boundary().unwrap();
    drop(assembly);
    evidence.committed_actor_ids.sort();
    evidence
}

struct SemanticRoot {
    fixture_root: PathBuf,
    workspace: PathBuf,
    config_path: PathBuf,
    activation_path: PathBuf,
    layout: meld::runtime::storage::ProductStorageLayout,
    identity_replacements: Vec<(String, String)>,
}

impl SemanticRoot {
    fn activate(root: &Path, marker: &str) -> Self {
        meld::init::initialize_workflows(false).unwrap();
        create_test_agent("docs-writer", Some("docs_writer_thread_v1"));
        let fixture_root = root.join(marker);
        let workspace = fixture_root.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("lib.rs"), "pub fn flywheel() {}\n").unwrap();
        let config_path = write_config(&fixture_root, "docs-writer");
        let activation_path = workspace.join("docs_freshness_activation.toml");
        fs::write(&activation_path, ACTIVATION).unwrap();
        activate(&workspace, &config_path, &activation_path).unwrap();
        let config = ConfigLoader::load_from_file(&config_path).unwrap();
        let product_root = config
            .system
            .storage
            .resolve_product_root(&workspace)
            .unwrap();
        let layout = meld::runtime::storage::ProductStorageLayout::from_root(product_root);
        let receipt = read_receipt(&layout.world_model_db);
        let branch_id = meld::branches::BranchRuntime::new()
            .resolve_active_branch(&workspace)
            .unwrap()
            .resolved()
            .branch_id
            .clone();
        let authority = EventAuthority::open(
            sled::open(&layout.ledger_db).unwrap(),
            EventAuthorityOpenOptions::default(),
        )
        .unwrap();
        let subject = DomainObjectRef::new("workspace_fs", "node", "node-a").unwrap();
        let frame = DomainObjectRef::new("context", "frame", "frame-initial-docs").unwrap();
        let head = DomainObjectRef::new("context", "head", "node-a::docs").unwrap();
        authority
            .append_capability()
            .append_durable(
                EventEnvelope::with_now_domain(
                    "docs-freshness-validation",
                    "context",
                    "docs-freshness-initial-observation",
                    "context.head_selected",
                    None,
                    serde_json::json!({ "source": "validation_observation" }),
                )
                .with_record_id("docs-freshness::initial-observation")
                .with_graph(
                    vec![head, subject.clone(), frame.clone()],
                    vec![EventRelation::new("selected", subject, frame).unwrap()],
                ),
                AppendMode::Idempotent,
            )
            .unwrap();
        let ledger_id = authority.replay_capability().ledger_identity().to_string();
        drop(authority);
        let world_model = sled::open(&layout.world_model_db).unwrap();
        let authority_meta = world_model.open_tree("belief_authority_meta").unwrap();
        let belief_store_instance_id = String::from_utf8(
            authority_meta
                .get("store_instance_id")
                .unwrap()
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        drop(authority_meta);
        drop(world_model);
        Self {
            fixture_root,
            workspace,
            config_path,
            activation_path,
            layout,
            identity_replacements: vec![
                (receipt.activation_hash, "<activation-hash>".to_string()),
                (receipt.input_hash, "<world-model-input-hash>".to_string()),
                (branch_id, "<branch-id>".to_string()),
                (ledger_id, "<ledger-id>".to_string()),
                (
                    belief_store_instance_id,
                    "<belief-store-instance-id>".to_string(),
                ),
            ],
        }
    }

    fn open(&self) -> ProductRuntimeAssembly {
        let prepared = meld::runtime::activation::load_and_prepare_activation(
            &self.workspace,
            &self.activation_path,
            Some(&self.config_path),
        )
        .unwrap();
        let owner = prepared.preflight.activation.runtime_inputs;
        let receipt = read_receipt(&self.layout.world_model_db);
        let selection = SemanticRuntimeSelection::docs_freshness_v1(
            &owner.runtime,
            &receipt,
            &prepared.preflight.execution_receipt,
        )
        .unwrap();
        let authority = std::sync::Arc::new(
            EventAuthority::open(
                sled::open(&self.layout.ledger_db).unwrap(),
                EventAuthorityOpenOptions::default(),
            )
            .unwrap(),
        );
        ProductRuntimeAssembly::load_semantic_with_authority(
            &self.workspace,
            &prepared.repository_config,
            authority,
            owner.runtime,
            owner.world_model,
            prepared.preflight.execution_input,
            prepared.preflight.execution_receipt,
            receipt,
            selection,
        )
        .unwrap()
    }

    fn actor_order(&self) -> Vec<String> {
        let assembly = self.open();
        assembly
            .activation_semantic()
            .unwrap()
            .actor_runtime_ids
            .clone()
    }

    fn snapshot(&self) -> SemanticDurableSnapshot {
        let normalizer = SnapshotNormalizer {
            fixture_root: &self.fixture_root,
            workspace: &self.workspace,
            product_root: &self.layout.root,
            identity_replacements: &self.identity_replacements,
        };
        SemanticDurableSnapshot {
            agent: snapshot_sled(&self.layout.world_model_db, &["agent_"], &[], &normalizer),
            belief: snapshot_sled(&self.layout.world_model_db, &["belief_"], &[], &normalizer),
            goal: snapshot_sled(
                &self.layout.execution_goals_db,
                &["execution_goal_"],
                &[],
                &normalizer,
            ),
            planner_projection: snapshot_sled(
                &self.layout.world_model_db,
                &["planner_"],
                &[],
                &normalizer,
            ),
            planning_attempt: snapshot_sled(
                &self.layout.execution_planning_attempts_db,
                &["execution_planning_attempt_"],
                &["execution_planning_attempt_owner"],
                &normalizer,
            ),
            task_network: snapshot_sled(
                &self.layout.task_networks_root.join("network-docs.sled"),
                &["task_network_"],
                &["task_network_authority_lifecycle"],
                &normalizer,
            ),
            events: snapshot_events(&self.layout.ledger_db, &normalizer),
        }
    }
}

#[derive(Debug)]
struct SemanticDurableSnapshot {
    agent: BTreeMap<String, Vec<(String, String)>>,
    belief: BTreeMap<String, Vec<(String, String)>>,
    goal: BTreeMap<String, Vec<(String, String)>>,
    planner_projection: BTreeMap<String, Vec<(String, String)>>,
    planning_attempt: BTreeMap<String, Vec<(String, String)>>,
    task_network: BTreeMap<String, Vec<(String, String)>>,
    events: Vec<String>,
}

struct SnapshotNormalizer<'a> {
    fixture_root: &'a Path,
    workspace: &'a Path,
    product_root: &'a Path,
    identity_replacements: &'a [(String, String)],
}

fn snapshot_sled(
    path: &Path,
    included_prefixes: &[&str],
    excluded_trees: &[&str],
    normalizer: &SnapshotNormalizer<'_>,
) -> BTreeMap<String, Vec<(String, String)>> {
    let db = sled::open(path).unwrap();
    let mut snapshot = BTreeMap::new();
    for tree_name in db.tree_names() {
        let name = String::from_utf8(tree_name.to_vec()).unwrap();
        if !included_prefixes
            .iter()
            .any(|prefix| name.starts_with(prefix))
            || excluded_trees.contains(&name.as_str())
        {
            continue;
        }
        let tree = db.open_tree(&name).unwrap();
        let entries = tree
            .iter()
            .map(|entry| {
                let (key, value) = entry.unwrap();
                (
                    normalize_bytes(&key, normalizer),
                    normalize_tree_value(&name, &key, &value, normalizer),
                )
            })
            .collect();
        snapshot.insert(name, entries);
    }
    drop(db);
    snapshot
}

fn normalize_tree_value(
    tree_name: &str,
    key: &[u8],
    value: &[u8],
    normalizer: &SnapshotNormalizer<'_>,
) -> String {
    if tree_name == "belief_runtime_meta" && key == b"assessment_lease_clock" {
        return "<lease-clock>".to_string();
    }
    if matches!(
        tree_name,
        "agent_curation_runtime_state" | "agent_hydration_runtime_state"
    ) {
        let mut value: Value = serde_json::from_slice(value).unwrap();
        if let Some(object) = value.as_object_mut() {
            object.remove("generation");
        }
        normalize_json(&mut value, normalizer);
        return serde_json::to_string(&value).unwrap();
    }
    normalize_bytes(value, normalizer)
}

fn snapshot_events(path: &Path, normalizer: &SnapshotNormalizer<'_>) -> Vec<String> {
    let authority = EventAuthority::open(
        sled::open(path).unwrap(),
        EventAuthorityOpenOptions::default(),
    )
    .unwrap();
    let replay = authority.replay_capability();
    let ledger_id = replay.ledger_identity();
    let mut after_seq = 0;
    let mut events = Vec::new();
    loop {
        let page = replay
            .replay(ReplayRequest {
                cursor: LedgerCursor {
                    ledger_id,
                    after_seq,
                },
                limit: 1_024,
            })
            .unwrap();
        if page.records.is_empty() {
            break;
        }
        for record in page.records {
            after_seq = record.seq;
            let mut envelope = serde_json::to_value(record.envelope).unwrap();
            let object = envelope.as_object_mut().unwrap();
            object.remove("ts");
            object.remove("recorded_at");
            object.remove("occurred_at");
            normalize_json(&mut envelope, normalizer);
            events.push(serde_json::to_string(&envelope).unwrap());
        }
    }
    events
}

fn normalize_json(value: &mut Value, normalizer: &SnapshotNormalizer<'_>) {
    match value {
        Value::String(text) => *text = normalize_text(text, normalizer),
        Value::Array(values) => {
            for value in values {
                normalize_json(value, normalizer);
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                normalize_json(value, normalizer);
            }
        }
        _ => {}
    }
}

fn normalize_bytes(bytes: &[u8], normalizer: &SnapshotNormalizer<'_>) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => normalize_text(text, normalizer),
        Err(_) => format!("hex:{}", hex_bytes(bytes)),
    }
}

fn normalize_text(text: &str, normalizer: &SnapshotNormalizer<'_>) -> String {
    let mut normalized = text
        .replace(
            &normalizer.workspace.to_string_lossy().to_string(),
            "<workspace>",
        )
        .replace(
            &normalizer.product_root.to_string_lossy().to_string(),
            "<product-root>",
        )
        .replace(
            &normalizer.fixture_root.to_string_lossy().to_string(),
            "<fixture-root>",
        );
    for (identity, replacement) in normalizer.identity_replacements {
        normalized = normalized.replace(identity, replacement);
    }
    normalized
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn activate(
    workspace: &Path,
    config_path: &Path,
    activation_path: &Path,
) -> Result<Value, meld::error::ApiError> {
    let output =
        handle_cli_activation(workspace, Some(config_path), activation_path, false, "json")?;
    Ok(serde_json::from_str(&output).unwrap())
}

fn read_receipt(path: &Path) -> meld_world_model::activation::AgentBootstrapReceipt {
    let store = AgentStore::new(sled::open(path).unwrap()).unwrap();
    store
        .get_bootstrap_receipt("world_model.agent.bootstrap.docs_freshness")
        .unwrap()
        .unwrap()
}

fn read_progress(path: &Path) -> meld_world_model::AgentBootstrapProgress {
    let store = AgentStore::new(sled::open(path).unwrap()).unwrap();
    store
        .get_bootstrap_progress("world_model.agent.bootstrap.docs_freshness")
        .unwrap()
        .unwrap()
}

fn workspace(temp: &tempfile::TempDir) -> PathBuf {
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    workspace
}

fn write_config(root: &Path, provider_id: &str) -> PathBuf {
    let path = root.join(format!("config-{provider_id}.toml"));
    fs::write(
        &path,
        format!(
            r#"
[providers.{provider_id}]
provider_type = "ollama"
model = "test-model"
"#
        ),
    )
    .unwrap();
    path
}
