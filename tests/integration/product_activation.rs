use std::fs;
use std::path::{Path, PathBuf};

use meld::config::ConfigLoader;
use meld::runtime::assembly::{
    ProductRuntimeAssembly, RuntimeFactoryRegistry, RuntimeLeaseContext,
};
use meld::runtime::contracts::{RuntimeImplementationState, RuntimeRoleClass, WorkBudget};
use meld::runtime::supervisor::{RuntimeId, RuntimeInstanceStatus, SupervisorStore};
use meld::runtime::tooling::handle_cli_activation;
use meld_events::{EventAuthority, EventAuthorityOpenOptions};
use meld_world_model::{
    AgentBootstrapProgressStatus, AgentStatus, AgentStore, AgentSubscriptionStatus, BeliefStore,
};
use serde_json::Value;

use super::with_xdg_env;

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
        assert!(error.to_string().contains("failed supervisor action"));
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
