use meld::cli::{Commands, RunContext, RuntimeCommands, WorldCommands};
use meld::config::{ConfigLoader, PhysicalBinding};
use meld::context::events::head_tombstoned_envelope;
use meld::error::ApiError;
use meld::events::binding::resolve_product_event_authority;
use meld::runtime::assembly::ProductRuntimeAssembly;
use meld::runtime::contracts::{RuntimeLaunchStatus, RuntimeStatusReader};
use meld::runtime::storage::ProductStorageLayout;
use meld::runtime::supervisor::{RuntimeId, SupervisorReportStore};
use meld::runtime::theory::ResolvedStewardshipTheory;
use meld_events::{AppendMode, DomainObjectRef};
use meld_lang::{Condition, Literal, Term};
use serde_json::Value;
use std::path::Path;
use tempfile::TempDir;

use super::docs_provider::DeterministicDocsProvider;
use super::{create_test_agent, with_xdg_env};

#[test]
fn runtime_status_reports_desired_runtimes_without_supervisor_store() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root, None).unwrap();

        let output = run_context
            .execute(&runtime_status_json(Vec::new()))
            .unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();

        assert!(parsed["instance"].is_null());
        assert!(parsed["supervisor_store_path"]
            .as_str()
            .unwrap()
            .ends_with("supervisor.sled"));
        let event_append = runtime_row(&parsed, "event.append");
        assert_eq!(event_append["desired_enabled"], true);
        assert_eq!(event_append["factory_available"], true);
        assert_eq!(event_append["handle_started"], false);
        let task_dispatch = runtime_row(&parsed, "execution.task_dispatch");
        assert_eq!(task_dispatch["desired_enabled"], false);
        assert_eq!(task_dispatch["factory_available"], true);
    });
}

#[test]
fn runtime_run_duration_starts_and_stops_cleanly() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();

        let output = run_context
            .execute(&runtime_run_json(
                Some("runtime-cli-test-a"),
                1,
                Some(1),
                "on-heartbeat-expiry",
            ))
            .unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();
        let started = parsed["started_runtime_count"].as_u64().unwrap();

        assert!(started > 0);
        assert_eq!(parsed["stopped_runtime_count"].as_u64().unwrap(), started);
        drop(run_context);
        let assembly = open_bound_assembly(&workspace_root);
        let runtime_id = RuntimeId::new("event.append").unwrap();
        assert!(assembly
            .supervisor_store()
            .get_active_runtime_lease(&runtime_id)
            .unwrap()
            .is_none());
    });
}

#[test]
fn runtime_run_ticks_graph_replay_handle() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let assembly = open_bound_assembly(&workspace_root);
        let appended = assembly
            .event_authority()
            .append_capability()
            .append_durable(
                head_tombstoned_envelope("session-a", [1; 32], "analysis", None),
                AppendMode::Plain,
            )
            .unwrap();
        drop(assembly);

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&runtime_run_json(
                Some("runtime-cli-test-graph"),
                1,
                Some(10),
                "on-heartbeat-expiry",
            ))
            .unwrap();
        drop(run_context);

        let assembly = open_bound_assembly(&workspace_root);
        let cursor = assembly
            .ports()
            .graph_cursor()
            .current()
            .unwrap()
            .expect("runtime graph replay should report its durable cursor");
        assert_eq!(
            cursor.ledger_id,
            assembly.ports().event_replay().ledger_identity()
        );
        assert_eq!(cursor.name, "world_state.graph.reducer");
        assert!(
            cursor.reported_seq >= appended.seq,
            "graph cursor {} did not cover appended event {}",
            cursor.reported_seq,
            appended.seq
        );
    });
}

fn open_bound_assembly(workspace_root: &std::path::Path) -> ProductRuntimeAssembly {
    let config = ConfigLoader::load(workspace_root).unwrap();
    let branch = meld::branches::locator::resolve_active_branch(workspace_root).unwrap();
    let product_root = config
        .system
        .storage
        .resolve_product_root(workspace_root)
        .unwrap();
    let layout = ProductStorageLayout::from_root(product_root);
    let (legacy_store, _, _) = config.system.storage.resolve_paths(workspace_root).unwrap();
    let resolved =
        resolve_product_event_authority(&branch, &layout.ledger_db, &legacy_store).unwrap();
    ProductRuntimeAssembly::load_for_workspace_with_authority(
        workspace_root,
        &config,
        resolved.authority,
    )
    .unwrap()
}

#[test]
fn runtime_status_after_run_reports_stopped_instance() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root, None).unwrap();
        run_context
            .execute(&runtime_run_json(
                Some("runtime-cli-test-b"),
                1,
                Some(1),
                "on-heartbeat-expiry",
            ))
            .unwrap();

        let output = run_context
            .execute(&runtime_status_json(Vec::new()))
            .unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();

        assert_eq!(parsed["instance"]["status"], "stopped");
        assert!(parsed["runtimes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|runtime| {
                runtime["health_status"] == "stopped" || runtime["active_lease_id"].is_null()
            }));
    });
}

#[test]
fn runtime_status_filters_runtime_ids() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root, None).unwrap();

        let output = run_context
            .execute(&runtime_status_json(vec!["event.append".to_string()]))
            .unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();
        let runtimes = parsed["runtimes"].as_array().unwrap();

        assert_eq!(runtimes.len(), 1);
        assert_eq!(runtimes[0]["runtime_id"], "event.append");
    });
}

#[test]
fn runtime_status_rejects_unknown_runtime_filter() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root, None).unwrap();

        let result = run_context.execute(&runtime_status_json(vec!["future.missing".to_string()]));

        assert_config_error_contains(result, "unknown runtime id filter");
    });
}

#[test]
fn runtime_run_rejects_zero_tick_ms() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root, None).unwrap();

        let result = run_context.execute(&runtime_run_json(
            Some("runtime-cli-test-c"),
            0,
            Some(1),
            "on-heartbeat-expiry",
        ));

        assert_config_error_contains(result, "tick-ms must be greater than 0");
    });
}

#[test]
fn runtime_run_rejects_unknown_restart_policy() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root, None).unwrap();

        let result = run_context.execute(&runtime_run_json(
            Some("runtime-cli-test-d"),
            1,
            Some(1),
            "always",
        ));

        assert_config_error_contains(result, "unknown restart policy");
    });
}

#[test]
fn runtime_run_accounts_distinguish_work_from_active_idle() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let assembly = open_bound_assembly(&workspace_root);
        // One committed graph event makes the first maintenance pass a
        // working pass; later passes are truthfully active-idle.
        assembly
            .event_authority()
            .append_capability()
            .append_durable(
                head_tombstoned_envelope("session-account", [2; 32], "analysis", None),
                AppendMode::Plain,
            )
            .unwrap();

        let mut sink = Vec::new();
        let output = meld::runtime::tooling::handle_cli_command_with_account_writer(
            &assembly,
            &RuntimeCommands::Run {
                instance_id: Some("runtime-cli-account".to_string()),
                tick_ms: 1,
                duration_ms: Some(60),
                format: "json".to_string(),
                restart_policy: "on-heartbeat-expiry".to_string(),
                restart_attempt_limit: 3,
                restart_backoff_ms: 0,
            },
            &mut sink,
        )
        .unwrap();
        // The returned run result stays pure JSON; account lines stream
        // through the writer instead.
        serde_json::from_str::<Value>(&output).unwrap();

        let lines = String::from_utf8(sink).unwrap();
        let accounts: Vec<Value> = lines
            .lines()
            .map(|line| serde_json::from_str(line).expect("each account line is one JSON object"))
            .collect();
        assert!(!accounts.is_empty());
        for account in &accounts {
            assert_eq!(account["type"], "runtime_tick_account");
            assert_eq!(account["instance_id"], "runtime-cli-account");
        }
        let working_pass = accounts.iter().find(|account| {
            account["active_idle"] == false
                && account["actors"].as_array().unwrap().iter().any(|actor| {
                    actor["runtime_id"] == "world_model.graph_replay"
                        && actor["items_committed"].as_u64().unwrap() >= 1
                        && actor["lifecycle"] == "active_working"
                })
        });
        assert!(
            working_pass.is_some(),
            "no working pass observed in accounts: {lines}"
        );
        let active_idle_pass = accounts.iter().find(|account| {
            account["active_idle"] == true
                && account["actors"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|actor| actor["outcome"] == "no_work")
        });
        assert!(
            active_idle_pass.is_some(),
            "no active-idle pass observed in accounts: {lines}"
        );
    });
}

#[test]
fn runtime_run_publishes_durable_lifecycle_snapshots() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let workspace_root = workspace(&temp_dir);
        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        let output = run_context
            .execute(&runtime_run_json(
                Some("runtime-cli-snapshots"),
                1,
                Some(5),
                "on-heartbeat-expiry",
            ))
            .unwrap();
        let parsed: Value = serde_json::from_str(&output).unwrap();
        drop(run_context);

        // The clean-stop envelope is durable and readable through the
        // frozen RuntimeStatusReader contract after the process would have
        // exited.
        let assembly = open_bound_assembly(&workspace_root);
        let reader = SupervisorReportStore::open(assembly.supervisor_store()).unwrap();
        let latest = reader
            .read_latest_snapshot()
            .unwrap()
            .expect("shutdown snapshot must be durable");
        assert_eq!(latest.writer.launch_status, RuntimeLaunchStatus::Stopped);
        assert_eq!(
            latest.writer.instance_id.as_deref(),
            Some("runtime-cli-snapshots")
        );
        let shutdown = latest.snapshot.shutdown.expect("shutdown summary");
        assert_eq!(
            shutdown.shutdown_id.as_deref(),
            parsed["shutdown_id"].as_str()
        );
        assert_eq!(shutdown.status, "stopped");
        assert!(latest
            .snapshot
            .instance
            .as_ref()
            .is_some_and(|instance| instance.instance_id == "runtime-cli-snapshots"));
    });
}

#[test]
fn prepared_product_activates_routes_and_ignores_loose_owner_heads() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        crate::integration::install_legacy_workflow_fixture().unwrap();
        let workspace_root = workspace(&temp_dir);
        std::fs::create_dir_all(workspace_root.join("docs")).unwrap();
        std::fs::write(workspace_root.join("docs/guide.txt"), "guide\n").unwrap();
        create_test_agent("docs-writer", Some("docs_writer_thread_v1"));
        let provider = DeterministicDocsProvider::spawn(&workspace_root);
        write_stewardship_config(&workspace_root, provider.endpoint());

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        run_context
            .execute(&Commands::Scan { force: true })
            .unwrap();
        assert!(run_context.product_runtime().capability_runtime().is_none());
        let first_init = run_context
            .execute(&Commands::World {
                command: WorldCommands::Init {
                    path: workspace_root.clone(),
                    stages: Vec::new(),
                    theory_source: Some(
                        Path::new(env!("CARGO_MANIFEST_DIR"))
                            .join("theory")
                            .join("docs_freshness"),
                    ),
                    format: "json".to_string(),
                },
            })
            .unwrap();
        let first_init: meld::init::world::WorldInitReport =
            serde_json::from_str(&first_init).unwrap();
        assert_eq!(first_init.stage_reports.len(), 4);
        assert!(first_init
            .stage_reports
            .iter()
            .all(|stage| stage.disposition == meld::init::world::StageDisposition::Applied));
        let second_init = run_context
            .execute(&Commands::World {
                command: WorldCommands::Init {
                    path: workspace_root.clone(),
                    stages: Vec::new(),
                    theory_source: Some(
                        Path::new(env!("CARGO_MANIFEST_DIR"))
                            .join("theory")
                            .join("docs_freshness"),
                    ),
                    format: "json".to_string(),
                },
            })
            .unwrap();
        let second_init: meld::init::world::WorldInitReport =
            serde_json::from_str(&second_init).unwrap();
        assert!(second_init
            .stage_reports
            .iter()
            .all(|stage| stage.disposition == meld::init::world::StageDisposition::Unchanged));
        assert_eq!(
            first_init
                .stage_reports
                .iter()
                .map(|stage| &stage.record_ids)
                .collect::<Vec<_>>(),
            second_init
                .stage_reports
                .iter()
                .map(|stage| &stage.record_ids)
                .collect::<Vec<_>>()
        );
        drop(run_context);

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        let product = run_context.product_runtime();
        let selection = PhysicalBinding::resolve(&ConfigLoader::load_global().unwrap())
            .unwrap()
            .package;
        assert_eq!(selection.expression, "documentation_maintenance");
        assert!(product.capability_runtime().is_some());
        let prepared_head = product
            .stores()
            .pds_products
            .prepared_head(&selection.expression)
            .unwrap()
            .unwrap();
        let prepared = product
            .stores()
            .pds_products
            .prepared_closure(&prepared_head.prepared_id)
            .unwrap()
            .unwrap();
        assert!(product
            .stores()
            .pds_products
            .assignment(&prepared.assignment.assignment_id)
            .unwrap()
            .is_some());
        assert!(product
            .stores()
            .pds_products
            .topology_receipt(&prepared.agent_topology_receipt_id)
            .unwrap()
            .is_some());
        assert!(product
            .stores()
            .pds_products
            .capability_preparation(&prepared.capability_preparation_receipt_id)
            .unwrap()
            .is_some());
        let resolved = ResolvedStewardshipTheory::resolve_prepared_product(
            product.stores(),
            &selection,
            &DomainObjectRef::new("workspace_fs", "node", "docs").unwrap(),
        )
        .unwrap();
        let compilation = product
            .stores()
            .pds_products
            .compilation(&prepared.assignment.product_compilation_receipt_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            resolved.package_receipt_ids,
            compilation.package_receipt_ids
        );
        let compilation_receipt_id = resolved
            .product_compilation_receipt_id
            .clone()
            .expect("prepared runtime theory has a compilation");
        assert_eq!(
            compilation_receipt_id,
            prepared.assignment.product_compilation_receipt_id
        );
        let receipt = resolved.receipt;
        let agent = product
            .stores()
            .agent_store
            .get_agent("docs-writer")
            .unwrap()
            .unwrap();
        assert_eq!(
            agent.curation_rule_revision.as_ref(),
            Some(&receipt.curation_rule)
        );
        assert_eq!(
            agent.maintained_condition_revision.as_ref(),
            Some(&receipt.maintained_condition)
        );
        assert_eq!(
            agent
                .maintained_condition
                .as_ref()
                .map(|binding| &binding.revision),
            Some(&receipt.maintained_condition)
        );
        assert!(agent.curation_rule.is_none());
        assert!(product
            .stores()
            .agent_store
            .activations_for_agent("docs-writer")
            .unwrap()
            .is_empty());
        assert!(
            product
                .stores()
                .curation_store
                .active_rule("docs-writer")
                .unwrap()
                .is_none(),
            "native Curation must come from prepared genesis, without an active-rule override"
        );
        product.flush_product_boundary().unwrap();
        drop(run_context);

        let run_context = RunContext::new(workspace_root.clone(), None).unwrap();
        let product = run_context.product_runtime();
        // The stewardship composition carries the route seed but stays a
        // truthful unresolved binding until the foreground run composes the
        // production routes.
        assert!(product.dispatch_route_seed().is_some());
        assert!(!product.dispatch_routes_bound());
        assert!(!product
            .handle_factories()
            .get("execution.task_dispatch")
            .unwrap()
            .has_semantic_body());

        run_context
            .execute(&runtime_run_json(
                Some("runtime-cli-dispatch"),
                1,
                Some(100),
                "on-heartbeat-expiry",
            ))
            .unwrap();

        // A configured product boot resolves the dispatch actor.
        assert!(product.dispatch_routes_bound());
        assert!(product
            .handle_factories()
            .get("execution.task_dispatch")
            .unwrap()
            .has_semantic_body());

        let capability_runtime = product.capability_runtime().unwrap();
        for capability_type in [
            "docs.inspect_scope",
            "docs.draft_patch_set",
            "docs.validate_patch_set",
            "docs.publish_patch_set",
        ] {
            assert!(capability_runtime.catalog.contains(capability_type, 1));
            assert!(capability_runtime
                .registry
                .get(capability_type, 1)
                .is_some());
        }
        assert!(!capability_runtime.catalog.contains("merkle_traversal", 1));

        let curation_a = product
            .stores()
            .curation_rule_registry
            .resolve(
                &receipt.curation_rule.id,
                &receipt.curation_rule.content_hash,
            )
            .unwrap()
            .unwrap();
        let mut curation_b_body = curation_a.rule.clone();
        curation_b_body.desired_summary = "confidence above 0.7".to_string();
        let (_, curation_b) = product
            .stores()
            .curation_rule_registry
            .install(&receipt.curation_rule.id, curation_b_body, 100)
            .unwrap();
        let condition_a = product
            .stores()
            .maintained_condition_registry
            .resolve(
                &receipt.maintained_condition.id,
                &receipt.maintained_condition.content_hash,
            )
            .unwrap()
            .unwrap();
        let mut condition_b_body = condition_a.condition.clone();
        condition_b_body.desired = Condition::Above(Term::Literal(Literal::Number(0.7)));
        condition_b_body.desired_summary = "confidence above 0.7".to_string();
        let (_, condition_b) = product
            .stores()
            .maintained_condition_registry
            .install(condition_b_body, 100)
            .unwrap();
        assert_ne!(curation_b.revision_ref(), receipt.curation_rule);
        assert_ne!(condition_b.revision_ref(), receipt.maintained_condition);
        drop(run_context);

        let run_context_b = RunContext::new(workspace_root.clone(), None).unwrap();
        let product_b = run_context_b.product_runtime();
        let resolved_b = ResolvedStewardshipTheory::resolve_prepared_product(
            product_b.stores(),
            &selection,
            &agent.subject,
        )
        .unwrap();
        assert_eq!(
            resolved_b.product_compilation_receipt_id.as_deref(),
            Some(compilation_receipt_id.as_str())
        );
        assert_eq!(resolved_b.curation_rule, curation_a);
        assert_eq!(resolved_b.maintained_condition, condition_a);
        drop(run_context_b);
        provider.shutdown();
    });
}

#[test]
fn startup_cli_reaches_goal_without_workspace_branch_or_provider() {
    let temp_dir = TempDir::new().unwrap();
    with_xdg_env(&temp_dir, || {
        let absent_workspace = temp_dir.path().join("no-workspace");
        let product_root = temp_dir.path().join("startup-product");
        let config = format!(
            r#"[system.storage]
product_root = "{}"

[stewardship.declarations.startup]
expression = "startup"
subject = {{ domain_id = "runtime", object_kind = "instance", object_id = "meld" }}
agent_id = "startup-agent"
principal_id = "runtime-owner"

[stewardship.declarations.startup.theory]
belief_family_id = "startup_realization"
evidence_mapping_id = "startup_realization_v1"
curation_rule_id = "startup_realization"
maintained_condition_id = "startup_realization"
strategy_theory_id = "startup_realization"
authority_policy_id = "startup_nonce_local"
"#,
            product_root.display()
        );
        let config_dir = Path::new(&std::env::var("XDG_CONFIG_HOME").unwrap()).join("meld");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("config.toml"), config).unwrap();
        let config = ConfigLoader::load_global().unwrap();
        let description =
            ProductRuntimeAssembly::describe_for_workspace(&absent_workspace, &config).unwrap();
        assert_eq!(description.product_root, product_root);
        assert!(!product_root.exists());
        assert!(meld::runtime::tooling::try_live_startup_account(
            &absent_workspace,
            &config,
            &meld::harness::startup::StartupAccountRequest {
                agent_id: "startup-agent".into(),
                ..Default::default()
            },
            "json",
        )
        .unwrap()
        .is_err());
        assert!(!product_root.exists());
        create_test_agent("retired-profile-agent", Some("absent-retired-profile"));
        let legacy_profiles = meld::config::WorkflowConfig::default()
            .resolve_user_profile_dir()
            .unwrap();
        std::fs::create_dir_all(&legacy_profiles).unwrap();
        let malformed_profile = legacy_profiles.join("malformed.yaml");
        std::fs::write(&malformed_profile, "workflow_id: [unfinished").unwrap();
        let run = RunContext::new(absent_workspace.clone(), None).unwrap();
        assert!(run.api().workspace_root().is_none());
        assert!(run
            .execute(&Commands::Workflow {
                command: meld::cli::WorkflowCommands::List {
                    format: "json".into()
                },
            })
            .is_err());
        let retired = run
            .execute(&Commands::Workflow {
                command: meld::cli::WorkflowCommands::Execute {
                    workflow_id: "malformed".into(),
                    node: None,
                    path: None,
                    path_positional: None,
                    agent: "retired-profile-agent".into(),
                    provider: "absent-provider".into(),
                    frame_type: None,
                    force: true,
                },
            })
            .unwrap_err();
        assert!(retired.to_string().contains("retired"));

        run.execute(&Commands::World {
            command: WorldCommands::Init {
                path: absent_workspace.clone(),
                stages: Vec::new(),
                theory_source: Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("theory/startup")),
                format: "json".to_string(),
            },
        })
        .unwrap();
        let head = run
            .product_runtime()
            .stores()
            .pds_products
            .prepared_head("startup")
            .unwrap()
            .unwrap();
        let prepared = run
            .product_runtime()
            .stores()
            .pds_products
            .prepared_closure(&head.prepared_id)
            .unwrap()
            .unwrap();
        assert_eq!(
            prepared.assignment.subject,
            DomainObjectRef::new("runtime", "instance", "meld").unwrap()
        );
        assert_eq!(prepared.participant_plan.participants.len(), 8);
        assert!(!prepared.activation.bindings.contains_key("workspace"));
        assert!(!prepared.activation.bindings.contains_key("provider"));
        let ledger_id = run
            .event_watermark_capability()
            .snapshot()
            .unwrap()
            .ledger_id;
        drop(run);

        let run = RunContext::new(absent_workspace.clone(), None).unwrap();
        assert_eq!(
            run.event_watermark_capability()
                .snapshot()
                .unwrap()
                .ledger_id,
            ledger_id
        );
        run.execute(&runtime_run_json(
            Some("startup-without-workspace"),
            1,
            Some(3_000),
            "on-heartbeat-expiry",
        ))
        .unwrap();
        let store = &run.product_runtime().stores().agent_store;
        let goals = store
            .reconciliation_goals_for_agent("startup-agent")
            .unwrap();
        assert_eq!(goals.len(), 1);
        let goal_id = &goals[0].goal.goal_id;
        let history = store.completed_history_for_goal(goal_id).unwrap();
        assert!(
            history.iter().any(|entry| matches!(
                entry.accepted_milestone,
                meld_world_model::strategy::PlanMilestoneRequirement::CurationTerminal { .. }
            )),
            "missing confirmation: {history:#?}"
        );
        assert!(
            history.iter().any(|entry| matches!(
                entry.accepted_milestone,
                meld_world_model::strategy::PlanMilestoneRequirement::BeliefRevision { .. }
            )),
            "missing Belief return: {history:#?}"
        );
        let plan = store.current_reconciliation_plan(goal_id).unwrap().unwrap();
        let disposition = store
            .goal_disposition_for_plan(&plan.plan_revision_id)
            .unwrap()
            .expect("Startup Goal unsatisfied");
        assert!(!disposition.accepted_milestone_ids.is_empty());
        let products = store.epoch_products(goal_id).unwrap().unwrap();
        let before_inspection = run.event_watermark_capability().snapshot().unwrap();
        run.execute(&Commands::Runtime {
            command: RuntimeCommands::StartupAccount {
                agent_id: "startup-agent".into(),
                generation_id: Some(products.specification.fence.activation_generation.clone()),
                admission_epoch: products.specification.fence.admission_epoch.clone(),
                nonce_id: None,
                inspection_fence: None,
                format: "text".into(),
            },
        })
        .unwrap();
        assert_eq!(
            run.event_watermark_capability().snapshot().unwrap(),
            before_inspection
        );
        drop(run);
        std::fs::remove_file(malformed_profile).unwrap();
        let run = RunContext::new(absent_workspace.clone(), None).unwrap();
        assert_eq!(
            run.product_runtime()
                .stores()
                .agent_store
                .goal_disposition_for_plan(&plan.plan_revision_id)
                .unwrap(),
            Some(disposition)
        );
        assert!(!absent_workspace.exists());
    });
}

fn write_stewardship_config(workspace_root: &Path, endpoint: &str) {
    let config_dir = workspace_root.join("config");
    std::fs::create_dir_all(&config_dir).unwrap();
    let target_root = workspace_root.canonicalize().unwrap();
    let config = format!(
        r#"[providers.steward-provider]
provider_name = "steward-provider"
provider_type = "local"
model = "test-model"
endpoint = "{endpoint}"

[stewardship.declarations.docs]
expression = "documentation_maintenance"
target_root = "{target_root}"
subject = "docs"
agent_id = "docs-writer"
principal_id = "workspace-owner"
provider_id = "steward-provider"

[stewardship.declarations.docs.theory]
belief_family_id = "docs_freshness"
evidence_mapping_id = "docs_freshness_outcome_interpretation_v1"
curation_rule_id = "docs_freshness"
maintained_condition_id = "docs_freshness"
strategy_theory_id = "docs_freshness"
authority_policy_id = "docs_workspace_local"
claim_policy_id = "docs-claims-strict-v1"
"#,
        target_root = target_root.display()
    );
    std::fs::write(config_dir.join("config.toml"), &config).unwrap();
    let global_config_dir = Path::new(&std::env::var("XDG_CONFIG_HOME").unwrap()).join("meld");
    std::fs::create_dir_all(&global_config_dir).unwrap();
    std::fs::write(global_config_dir.join("config.toml"), config).unwrap();
}

fn workspace(temp_dir: &TempDir) -> std::path::PathBuf {
    let workspace_root = temp_dir.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).unwrap();
    workspace_root
}

fn runtime_status_json(runtime_ids: Vec<String>) -> Commands {
    Commands::Runtime {
        command: RuntimeCommands::Status {
            format: "json".to_string(),
            runtime_ids,
        },
    }
}

fn runtime_run_json(
    instance_id: Option<&str>,
    tick_ms: u64,
    duration_ms: Option<u64>,
    restart_policy: &str,
) -> Commands {
    Commands::Runtime {
        command: RuntimeCommands::Run {
            instance_id: instance_id.map(str::to_string),
            tick_ms,
            duration_ms,
            format: "json".to_string(),
            restart_policy: restart_policy.to_string(),
            restart_attempt_limit: 3,
            restart_backoff_ms: 0,
        },
    }
}

fn runtime_row<'a>(parsed: &'a Value, runtime_id: &str) -> &'a Value {
    parsed["runtimes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|runtime| runtime["runtime_id"] == runtime_id)
        .unwrap()
}

fn assert_config_error_contains(result: Result<String, ApiError>, expected: &str) {
    match result {
        Err(ApiError::ConfigError(message)) => {
            assert!(message.contains("Runtime command failed:"));
            assert!(message.contains(expected), "{message}");
        }
        other => panic!("expected config error, got {other:?}"),
    }
}
