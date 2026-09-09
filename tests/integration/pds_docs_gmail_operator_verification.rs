use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use meld::cli::{Commands, RunContext, RuntimeCommands, WorldCommands};
use meld::config::{ConfigLoader, PhysicalBinding};
use meld::runtime::storage::ProductStorageLayout;
use meld::runtime::theory::ResolvedStewardshipTheory;
use meld_docs_owner::docs::theory::DOCS_PACKAGE_ID;
use meld_events::DomainObjectRef;
use meld_execution::task_network::store::TaskNetworkStoreFactory;
use serde::Serialize;
use tempfile::TempDir;

use super::docs_provider::DeterministicDocsProvider;
use super::{create_test_agent, with_xdg_env};

const GMAIL_ROOT_ENV: &str = "MELD_GMAIL_OPERATOR_ROOT";
const EVIDENCE_ROOT_ENV: &str = "MELD_PDS_VERIFICATION_OUTPUT";
const LIVE_ENDPOINT_ENV: &str = "MELD_PDS_LIVE_ENDPOINT";
const LIVE_MODEL_ENV: &str = "MELD_PDS_LIVE_MODEL";
const STEWARD_AGENT_ID: &str = "docs-writer";
const PROVIDER_ID: &str = "steward-provider";
const SUBJECT_PATH: &str = "docs";

const EXPECTED_READMES: [&str; 5] = [
    "docs/README.md",
    "docs/gmail_operator/README.md",
    "docs/k8s/README.md",
    "docs/k8s/base/README.md",
    "docs/tests/README.md",
];

#[derive(Serialize)]
struct RoutedGmailEvidence {
    schema: &'static str,
    meld_head: String,
    meld_worktree_dirty: bool,
    gmail_head: String,
    gmail_source_root: String,
    package_id: &'static str,
    package_receipt_id: String,
    enabled_runtime_ids: Vec<String>,
    cold_tick_count: u64,
    reopen_tick_count: u64,
    first_run_provider_requests: usize,
    reopen_provider_requests: usize,
    task_network: TaskStateSummary,
    readme_hashes_before_reopen: BTreeMap<String, String>,
    readme_hashes_after_reopen: BTreeMap<String, String>,
}

#[derive(Clone, Serialize)]
struct TaskStateSummary {
    revision: u64,
    task_count: usize,
    outcome_count: usize,
    publication_count: usize,
    all_tasks_succeeded: bool,
    all_publications_marked: bool,
}

#[derive(Serialize)]
struct LiveRoutedGmailEvidence {
    schema: &'static str,
    meld_head: String,
    meld_worktree_dirty: bool,
    gmail_head: String,
    gmail_source_root: String,
    provider_endpoint: String,
    provider_model: String,
    package_id: &'static str,
    package_receipt_id: String,
    enabled_runtime_ids: Vec<String>,
    runtime_invocations: usize,
    tick_count: u64,
    runtime_error: Option<String>,
    completed_readme_count: usize,
    task_network: TaskStateSummary,
    readme_hashes: BTreeMap<String, String>,
}

#[test]
#[ignore = "requires MELD_GMAIL_OPERATOR_ROOT"]
fn routed_docs_pds_generates_gmail_operator_and_reopens_without_work() {
    let gmail_root = required_directory(GMAIL_ROOT_ENV);
    let temp = TempDir::new().unwrap();
    with_xdg_env(&temp, || {
        crate::integration::install_legacy_workflow_fixture().unwrap();
        let workspace_root = temp.path().join("workspace");
        let subject_root = workspace_root.join(SUBJECT_PATH);
        copy_tracked_repository(&gmail_root, &subject_root);

        create_test_agent(STEWARD_AGENT_ID, Some("docs_writer_thread_v1"));
        let provider = DeterministicDocsProvider::spawn_for_routed_validation(&workspace_root);
        write_global_config(&workspace_root, provider.endpoint());

        let initial = RunContext::new(workspace_root.clone(), None).unwrap();
        initial.execute(&Commands::Scan { force: true }).unwrap();
        initial
            .execute(&Commands::World {
                command: WorldCommands::Init {
                    path: workspace_root.clone(),
                    stages: Vec::new(),
                    theory_source: Some(docs_package_root()),
                    format: "json".to_string(),
                },
            })
            .unwrap();

        let selection = PhysicalBinding::resolve(&ConfigLoader::load_global().unwrap())
            .unwrap()
            .package;
        let stores = initial.product_runtime().stores();
        let package_head = stores
            .pds_packages
            .head(DOCS_PACKAGE_ID)
            .unwrap()
            .expect("routed docs package head");
        let subject = DomainObjectRef::new("workspace_fs", "node", SUBJECT_PATH).unwrap();
        let resolved = ResolvedStewardshipTheory::resolve_prepared_product(
            stores,
            &selection,
            &subject,
            &PhysicalBinding::resolve(&ConfigLoader::load_global().unwrap())
                .unwrap()
                .assignment_scope_id(),
        )
        .unwrap();
        assert_eq!(resolved.package_receipt_ids, vec![package_head.receipt_id]);
        assert_eq!(resolved.executable_contracts.len(), 5);
        let package_receipt_id = resolved.package_receipt_ids[0].clone();
        drop(initial);

        let enabled_runtime_ids = vec!["execution.task_dispatch".to_string()];
        let run =
            RunContext::with_runtime_enablement(workspace_root.clone(), None, &enabled_runtime_ids)
                .unwrap();
        let cold_summary: serde_json::Value = serde_json::from_str(
            &run.execute(&runtime_run("gmail-operator-routed-cold", 1_000))
                .unwrap(),
        )
        .unwrap();
        let product_root = run.product_runtime().product_root().to_path_buf();
        drop(run);
        let first_run_provider_requests = provider.shutdown();
        assert!(
            first_run_provider_requests > 0,
            "cold routed run must reach the provider"
        );
        let before = readme_hashes(&workspace_root);
        let task_state = task_network_state(&product_root);
        assert_expected_readmes(&before, first_run_provider_requests, &task_state);
        assert!(task_state.all_tasks_succeeded);
        assert!(task_state.all_publications_marked);

        let reopen_provider =
            DeterministicDocsProvider::spawn_for_routed_validation(&workspace_root);
        write_global_config(&workspace_root, reopen_provider.endpoint());
        let reopened =
            RunContext::with_runtime_enablement(workspace_root.clone(), None, &enabled_runtime_ids)
                .unwrap();
        let reopened_resolved = ResolvedStewardshipTheory::resolve_prepared_product(
            reopened.product_runtime().stores(),
            &selection,
            &subject,
            &PhysicalBinding::resolve(&ConfigLoader::load_global().unwrap())
                .unwrap()
                .assignment_scope_id(),
        )
        .unwrap();
        assert_eq!(
            reopened_resolved.package_receipt_ids,
            vec![package_receipt_id.clone()]
        );
        let reopen_summary: serde_json::Value = serde_json::from_str(
            &reopened
                .execute(&runtime_run("gmail-operator-routed-reopen", 300))
                .unwrap(),
        )
        .unwrap();
        drop(reopened);
        let reopen_provider_requests = reopen_provider.shutdown();
        let after = readme_hashes(&workspace_root);
        assert_eq!(before, after, "reopen changed published README bytes");
        assert_eq!(
            reopen_provider_requests, 0,
            "quiescent reopen must not reach the provider"
        );

        if let Some(evidence_root) = std::env::var_os(EVIDENCE_ROOT_ENV).map(PathBuf::from) {
            let evidence = RoutedGmailEvidence {
                schema: "meld.docs_freshness.routed_gmail_verification.v1",
                meld_head: git_output(
                    Path::new(env!("CARGO_MANIFEST_DIR")),
                    &["rev-parse", "HEAD"],
                ),
                meld_worktree_dirty: !git_output(
                    Path::new(env!("CARGO_MANIFEST_DIR")),
                    &["status", "--porcelain"],
                )
                .is_empty(),
                gmail_head: git_output(&gmail_root, &["rev-parse", "HEAD"]),
                gmail_source_root: gmail_root.display().to_string(),
                package_id: DOCS_PACKAGE_ID,
                package_receipt_id,
                enabled_runtime_ids,
                cold_tick_count: cold_summary["tick_count"].as_u64().unwrap(),
                reopen_tick_count: reopen_summary["tick_count"].as_u64().unwrap(),
                first_run_provider_requests,
                reopen_provider_requests,
                task_network: task_state,
                readme_hashes_before_reopen: before,
                readme_hashes_after_reopen: after,
            };
            write_evidence(&evidence_root, &workspace_root, &evidence);
        }
    });
}

#[test]
#[ignore = "requires MELD_GMAIL_OPERATOR_ROOT"]
fn routed_docs_pds_rejects_a_missing_required_capability_before_provider_work() {
    let gmail_root = required_directory(GMAIL_ROOT_ENV);
    let temp = TempDir::new().unwrap();
    with_xdg_env(&temp, || {
        crate::integration::install_legacy_workflow_fixture().unwrap();
        let workspace_root = temp.path().join("workspace");
        copy_tracked_repository(&gmail_root, &workspace_root.join(SUBJECT_PATH));
        create_test_agent(STEWARD_AGENT_ID, Some("docs_writer_thread_v1"));
        let provider = DeterministicDocsProvider::spawn_for_routed_validation(&workspace_root);
        write_global_config(&workspace_root, provider.endpoint());

        let broken_package = temp.path().join("broken-docs-package");
        copy_directory_files(&docs_package_root(), &broken_package);
        let manifest_path = broken_package.join("pds-package.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        manifest["components"]
            .as_array_mut()
            .unwrap()
            .retain(|component| component["component_id"] != "docs-capability-draft");
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let run = RunContext::new(workspace_root.clone(), None).unwrap();
        run.execute(&Commands::Scan { force: true }).unwrap();
        let failure = run
            .execute(&Commands::World {
                command: WorldCommands::Init {
                    path: workspace_root,
                    stages: Vec::new(),
                    theory_source: Some(broken_package),
                    format: "json".to_string(),
                },
            })
            .expect_err("missing routed capability must reject initialization");
        assert!(failure.to_string().contains("docs-capability-draft"));
        drop(run);
        let provider_requests = provider.shutdown();
        assert_eq!(provider_requests, 0);
        if let Some(evidence_root) = std::env::var_os(EVIDENCE_ROOT_ENV).map(PathBuf::from) {
            std::fs::create_dir_all(&evidence_root).unwrap();
            std::fs::write(
                evidence_root.join("negative-control.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "schema": "meld.docs_freshness.routed_negative_control.v1",
                    "rejected": true,
                    "provider_requests": provider_requests,
                    "diagnostic": failure.to_string(),
                }))
                .unwrap(),
            )
            .unwrap();
        }
    });
}

#[test]
#[ignore = "requires Gmail source plus an explicitly selected live provider"]
fn routed_docs_pds_generates_gmail_operator_with_live_provider() {
    let gmail_root = required_directory(GMAIL_ROOT_ENV);
    let endpoint = required_value(LIVE_ENDPOINT_ENV);
    let model = required_value(LIVE_MODEL_ENV);
    let evidence_root = required_directory(EVIDENCE_ROOT_ENV);
    let temp = TempDir::new_in(&evidence_root).unwrap();
    with_xdg_env(&temp, || {
        crate::integration::install_legacy_workflow_fixture().unwrap();
        let workspace_root = temp.path().join("workspace");
        copy_tracked_repository(&gmail_root, &workspace_root.join(SUBJECT_PATH));

        create_test_agent(STEWARD_AGENT_ID, Some("docs_writer_thread_v1"));
        write_global_config_with_model(&workspace_root, &model, &endpoint);

        let initial = RunContext::new(workspace_root.clone(), None).unwrap();
        initial.execute(&Commands::Scan { force: true }).unwrap();
        initial
            .execute(&Commands::World {
                command: WorldCommands::Init {
                    path: workspace_root.clone(),
                    stages: Vec::new(),
                    theory_source: Some(docs_package_root()),
                    format: "json".to_string(),
                },
            })
            .unwrap();

        let selection = PhysicalBinding::resolve(&ConfigLoader::load_global().unwrap())
            .unwrap()
            .package;
        let stores = initial.product_runtime().stores();
        let package_head = stores
            .pds_packages
            .head(DOCS_PACKAGE_ID)
            .unwrap()
            .expect("routed docs package head");
        let subject = DomainObjectRef::new("workspace_fs", "node", SUBJECT_PATH).unwrap();
        let resolved = ResolvedStewardshipTheory::resolve_prepared_product(
            stores,
            &selection,
            &subject,
            &PhysicalBinding::resolve(&ConfigLoader::load_global().unwrap())
                .unwrap()
                .assignment_scope_id(),
        )
        .unwrap();
        assert_eq!(resolved.package_receipt_ids, vec![package_head.receipt_id]);
        assert_eq!(resolved.executable_contracts.len(), 5);
        let package_receipt_id = resolved.package_receipt_ids[0].clone();
        drop(initial);

        let enabled_runtime_ids = vec!["execution.task_dispatch".to_string()];
        let run =
            RunContext::with_runtime_enablement(workspace_root.clone(), None, &enabled_runtime_ids)
                .unwrap();
        let product_root = run.product_runtime().product_root().to_path_buf();
        let mut runtime_invocations = 0usize;
        let mut tick_count = 0u64;
        let mut runtime_error = None;
        for step in 0..40 {
            runtime_invocations += 1;
            let output =
                match run.execute(&runtime_run(&format!("gmail-operator-qwen-live-{step}"), 1)) {
                    Ok(output) => output,
                    Err(error) => {
                        runtime_error = Some(error.to_string());
                        break;
                    }
                };
            let summary: serde_json::Value = serde_json::from_str(&output).unwrap();
            tick_count += summary["tick_count"].as_u64().unwrap();
            if readme_hashes(&workspace_root).len() == EXPECTED_READMES.len() {
                break;
            }
        }
        drop(run);

        let hashes = readme_hashes(&workspace_root);
        let task_state = task_network_state(&product_root);

        let evidence = LiveRoutedGmailEvidence {
            schema: "meld.docs_freshness.routed_gmail_live_verification.v1",
            meld_head: git_output(
                Path::new(env!("CARGO_MANIFEST_DIR")),
                &["rev-parse", "HEAD"],
            ),
            meld_worktree_dirty: !git_output(
                Path::new(env!("CARGO_MANIFEST_DIR")),
                &["status", "--porcelain"],
            )
            .is_empty(),
            gmail_head: git_output(&gmail_root, &["rev-parse", "HEAD"]),
            gmail_source_root: gmail_root.display().to_string(),
            provider_endpoint: endpoint,
            provider_model: model,
            package_id: DOCS_PACKAGE_ID,
            package_receipt_id,
            enabled_runtime_ids,
            runtime_invocations,
            tick_count,
            runtime_error: runtime_error.clone(),
            completed_readme_count: hashes.len(),
            task_network: task_state,
            readme_hashes: hashes.clone(),
        };
        write_live_evidence(&evidence_root, &workspace_root, &evidence);
        assert!(
            runtime_error.is_none(),
            "live routed runtime failed: {}",
            runtime_error.unwrap()
        );
        assert_expected_readmes(&hashes, 0, &evidence.task_network);
        assert!(evidence.task_network.all_tasks_succeeded);
        assert!(evidence.task_network.all_publications_marked);
    });
}

fn runtime_run(instance_id: &str, duration_ms: u64) -> Commands {
    Commands::Runtime {
        command: RuntimeCommands::Run {
            instance_id: Some(instance_id.to_string()),
            tick_ms: 10,
            duration_ms: Some(duration_ms),
            format: "json".to_string(),
            restart_policy: "on-heartbeat-expiry".to_string(),
            restart_attempt_limit: 3,
            restart_backoff_ms: 0,
        },
    }
}

fn write_global_config(workspace_root: &Path, endpoint: &str) {
    write_global_config_with_model(workspace_root, "test-model", endpoint);
}

fn write_global_config_with_model(workspace_root: &Path, model: &str, endpoint: &str) {
    let config = format!(
        r#"[providers.{PROVIDER_ID}]
provider_name = "{PROVIDER_ID}"
provider_type = "local"
model = "{model}"
endpoint = "{endpoint}"

[stewardship.declarations.docs]
expression = "documentation_maintenance"
target_root = "{target_root}"
subject = "{SUBJECT_PATH}"
agent_id = "{STEWARD_AGENT_ID}"
principal_id = "workspace-owner"
provider_id = "{PROVIDER_ID}"

[stewardship.declarations.docs.theory]
belief_family_id = "docs_freshness"
evidence_mapping_id = "docs_freshness_outcome_interpretation_v1"
curation_rule_id = "docs_freshness"
maintained_condition_id = "docs_freshness"
strategy_theory_id = "docs_freshness"
authority_policy_id = "docs_workspace_local"
claim_policy_id = "docs-claims-strict-v1"
"#,
        target_root = workspace_root.canonicalize().unwrap().display(),
    );
    let config = super::external_owners::docs_config(&config);
    let config_root = Path::new(&std::env::var("XDG_CONFIG_HOME").unwrap()).join("meld");
    std::fs::create_dir_all(&config_root).unwrap();
    std::fs::write(config_root.join("config.toml"), config).unwrap();
}

fn required_directory(variable: &str) -> PathBuf {
    let path = PathBuf::from(
        std::env::var_os(variable)
            .unwrap_or_else(|| panic!("{variable} must name the source repository")),
    );
    assert!(path.is_dir(), "{} is not a directory", path.display());
    path
}

fn required_value(variable: &str) -> String {
    std::env::var(variable).unwrap_or_else(|_| panic!("{variable} must be set"))
}

fn docs_package_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("theory")
        .join("docs_freshness")
}

fn copy_tracked_repository(source: &Path, destination: &Path) {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(source)
        .output()
        .unwrap();
    assert!(output.status.success(), "git ls-files failed");
    for entry in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        let relative = Path::new(std::str::from_utf8(entry).unwrap());
        let target = destination.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::copy(source.join(relative), target).unwrap();
    }
}

fn copy_directory_files(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            std::fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
        }
    }
}

fn readme_hashes(workspace_root: &Path) -> BTreeMap<String, String> {
    EXPECTED_READMES
        .iter()
        .filter_map(|relative| {
            let path = workspace_root.join(relative);
            path.is_file().then(|| {
                (
                    relative.to_string(),
                    blake3::hash(&std::fs::read(path).unwrap())
                        .to_hex()
                        .to_string(),
                )
            })
        })
        .collect()
}

fn assert_expected_readmes(
    hashes: &BTreeMap<String, String>,
    provider_requests: usize,
    task_state: &TaskStateSummary,
) {
    assert_eq!(
        hashes.len(),
        EXPECTED_READMES.len(),
        "provider requests: {provider_requests}\nREADME paths: {:?}\ntask revision: {}",
        hashes.keys().collect::<Vec<_>>(),
        task_state.revision
    );
    for relative in EXPECTED_READMES {
        assert!(hashes.contains_key(relative), "missing README {relative}");
    }
}

fn task_network_state(product_root: &Path) -> TaskStateSummary {
    let layout = ProductStorageLayout::from_root(product_root);
    let factory = TaskNetworkStoreFactory::new(layout.task_networks_root);
    let network = factory
        .open_network("stewardship.documentation_maintenance")
        .unwrap();
    let state = network.state();
    TaskStateSummary {
        revision: state.revision,
        task_count: state.tasks.len(),
        outcome_count: state.outcomes.len(),
        publication_count: state.publications.len(),
        all_tasks_succeeded: state.statuses.values().all(|status| {
            matches!(
                status,
                meld_execution::task_network::state::TaskStatus::Succeeded { .. }
            )
        }),
        all_publications_marked: state.publications.values().all(|publication| {
            matches!(
                publication.state,
                meld_execution::task_network::outcome::PublicationState::Published { .. }
            )
        }),
    }
}

fn git_output(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "git command failed");
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn write_evidence(root: &Path, workspace_root: &Path, evidence: &RoutedGmailEvidence) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(evidence).unwrap(),
    )
    .unwrap();
    let output_root = root.join("workspace");
    for relative in EXPECTED_READMES {
        let source = workspace_root.join(relative);
        if !source.is_file() {
            continue;
        }
        let target = output_root.join(relative);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(source, target).unwrap();
    }
}

fn write_live_evidence(root: &Path, workspace_root: &Path, evidence: &LiveRoutedGmailEvidence) {
    std::fs::write(
        root.join("live-manifest.json"),
        serde_json::to_vec_pretty(evidence).unwrap(),
    )
    .unwrap();
    let output_root = root.join("live-workspace");
    std::fs::create_dir_all(&output_root).unwrap();
    for relative in EXPECTED_READMES {
        let source = workspace_root.join(relative);
        let target = output_root.join(relative);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(source, target).unwrap();
    }
}
