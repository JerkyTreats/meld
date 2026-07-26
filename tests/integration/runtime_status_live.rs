//! Concurrent runtime status over a live composition.
//!
//! The runtime survey recorded that `meld runtime status` from a second
//! process failed on the live process's exclusive store locks. The fix is
//! the lock-free workspace description plus the served surface: the live
//! process advertises its loopback address under the product root, and
//! the status bypass answers from that surface without opening any store.

use std::fs;

use meld::config::MerkleConfig;
use meld::harness::boot::{HarnessBootRequest, HarnessRootSelection, HarnessRun};
use meld::runtime::contracts::{
    RuntimeActionRecord, RuntimeHandleKind, RuntimeLaunchStatus, RuntimeRunMode,
    RuntimeStatusCacheRecord, RuntimeStatusHealthSummary, RuntimeStatusPublisher,
    RuntimeStatusRuntimeRow, RuntimeStatusSnapshot, RuntimeStatusWriterIdentity,
    WaitingOnDeclaration, WorkerCheckpoint, WorkerScope, WorkerTickReport,
};
use meld::runtime::supervisor::SupervisorReportStore;
use meld::runtime::tooling::try_live_runtime_status;
use meld::serve::listener::serve_with_discovery;
use meld::serve::sources::ServeSources;

fn stalled_action() -> RuntimeActionRecord {
    RuntimeActionRecord::from_worker_tick(
        "action-live",
        "world_model.belief_assessment",
        10,
        WorkerTickReport {
            actor_id: "world_model.belief_assessment".to_string(),
            scope: WorkerScope {
                domain_id: "world_model".to_string(),
                stream_id: None,
                work_key: Some("belief_assessment".to_string()),
                agent_id: None,
                perspective_key: None,
                branch_id: None,
                subject_key: Some("workspace_fs::node::docs".to_string()),
            },
            input_checkpoint: WorkerCheckpoint {
                name: "belief_assessment_checkpoint".to_string(),
                value: 0,
            },
            output_checkpoint: WorkerCheckpoint {
                name: "belief_assessment_checkpoint".to_string(),
                value: 1,
            },
            items_attempted: 1,
            items_committed: 0,
            retryable_errors: Vec::new(),
            fatal_errors: Vec::new(),
            budget_exhausted: false,
            waiting_on: vec![WaitingOnDeclaration {
                condition: "graph_anchor_absent".to_string(),
                subject_key: Some("workspace_fs::node::docs".to_string()),
                detail: "no current anchor".to_string(),
            }],
        },
    )
}

#[test]
fn a_second_process_path_reads_live_status_from_the_served_surface() {
    let session = tempfile::tempdir().unwrap();
    let workspace_root = session.path().join("workspace");
    let product_root = session.path().join("root");
    fs::create_dir_all(&workspace_root).unwrap();
    let workspace_root = workspace_root.canonicalize().unwrap();
    let mut config = MerkleConfig::default();
    config.system.storage.product_root = Some(product_root.clone());

    // Boot the composition that owns the store locks.
    let mut request = HarnessBootRequest::temporary("status-live", 1_000);
    request.root = HarnessRootSelection::ExistingDataRoot {
        product_root: product_root.clone(),
        branch_home: session.path().join("branch-home"),
        legacy_store_path: session.path().join("legacy-compat"),
        manifest_path: session.path().join("harness_manifest.json"),
    };
    request.unsafe_existing_root = true;
    let run = HarnessRun::boot(request).unwrap();

    // The live process publishes its snapshot, as the foreground run does
    // at startup, including a stalled actor's waiting-on declaration.
    let action = stalled_action();
    let mut reports = SupervisorReportStore::open(run.assembly().supervisor_store()).unwrap();
    reports
        .publish_startup_snapshot(&RuntimeStatusCacheRecord::new(
            product_root.clone(),
            run.assembly().supervisor_store().path(),
            product_root.join("status"),
            RuntimeStatusWriterIdentity {
                instance_id: Some("live-instance".to_string()),
                process_id: Some(std::process::id()),
                parent_process_id: None,
                run_mode: RuntimeRunMode::Foreground,
                launch_status: RuntimeLaunchStatus::Ready,
            },
            RuntimeStatusSnapshot {
                instance: None,
                process: None,
                shutdown: None,
                runtimes: vec![RuntimeStatusRuntimeRow {
                    runtime_id: "world_model.belief_assessment".to_string(),
                    desired_enabled: true,
                    factory_available: true,
                    handle_kind: RuntimeHandleKind::Concrete,
                    lease: None,
                    heartbeat: None,
                    health: RuntimeStatusHealthSummary {
                        status: "degraded".to_string(),
                        retryable_error_count: 1,
                        fatal_error_count: 0,
                        budget_exhausted: false,
                    },
                    restart_count: 0,
                    last_restart_cause: None,
                    last_lifecycle_event: None,
                    last_progress: None,
                    last_action: Some(action),
                }],
                health_counts: meld::runtime::contracts::RuntimeStatusHealthCounts {
                    unknown: 0,
                    starting: 0,
                    healthy: 0,
                    degraded: 1,
                    unhealthy: 0,
                    stopped: 0,
                },
                ledger: None,
                warnings: Vec::new(),
            },
            Vec::new(),
            11,
        ))
        .unwrap();

    // No advertisement yet: the bypass declines and the caller falls
    // through to the normal path.
    assert!(try_live_runtime_status(&workspace_root, &config, "text", &[]).is_none());

    // The live process advertises its surface; the bypass answers from it
    // while the sled locks stay held by this process.
    let handle = serve_with_discovery(
        ServeSources::from_assembly(run.assembly()).unwrap(),
        0,
        &product_root,
    )
    .unwrap();
    let status = try_live_runtime_status(&workspace_root, &config, "text", &[])
        .expect("advertised surface answers")
        .expect("status renders");
    assert!(status.contains("live surface at"));
    assert!(status.contains("world_model.belief_assessment: degraded"));
    assert!(
        status.contains("waiting on graph_anchor_absent (workspace_fs::node::docs)"),
        "the stalled declaration reaches the operator: {status}"
    );

    let json = try_live_runtime_status(&workspace_root, &config, "json", &[])
        .unwrap()
        .unwrap();
    let record: RuntimeStatusCacheRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(record.snapshot.runtimes.len(), 1);

    // Shutdown withdraws the advertisement; the bypass declines again.
    handle.shutdown();
    assert!(try_live_runtime_status(&workspace_root, &config, "text", &[]).is_none());

    // A stale advertisement (dead address) also declines instead of
    // failing, so a crash leftover never breaks status.
    meld::serve::discovery::write(&product_root, "127.0.0.1:1".parse().unwrap()).unwrap();
    assert!(try_live_runtime_status(&workspace_root, &config, "text", &[]).is_none());
}
