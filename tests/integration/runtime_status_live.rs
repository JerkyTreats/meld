//! Concurrent runtime status over a live composition.
//!
//! The runtime survey recorded that `meld runtime status` from a second
//! process failed on the live process's exclusive store locks. The fix is
//! the lock-free workspace description plus the served surface: the live
//! process advertises its loopback address under the product root, and
//! the status bypass answers from that surface without opening any store.

use std::fs;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use meld::config::MerkleConfig;
use meld::harness::boot::{HarnessBootRequest, HarnessRootSelection, HarnessRun};
use meld::runtime::control::{ControlStatus, RuntimeControl};
use meld::runtime::tooling::try_live_runtime_status;
use meld::serve::listener::serve_with_discovery;
use meld::serve::sources::ServeSources;

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
    let mut run = HarnessRun::boot(request).unwrap();

    // No advertisement yet: the bypass declines and the caller falls
    // through to the normal path.
    assert!(try_live_runtime_status(&workspace_root, &config, "text", &[]).is_none());

    // The live process advertises its surface; the bypass answers from it
    // while the sled locks stay held by this process.
    let sources = ServeSources::from_assembly(run.assembly()).unwrap();
    let supervisor_store = run.assembly().supervisor_store().clone();
    let mut driver = run.driver().unwrap();
    driver.step(1_100).unwrap();
    let handle = serve_with_discovery(
        sources.with_runtime_control(RuntimeControl::new(
            product_root.clone(),
            "harness::status-live".to_string(),
            supervisor_store,
            Arc::new(AtomicBool::new(false)),
        )),
        0,
        &product_root,
    )
    .unwrap();
    let status = try_live_runtime_status(&workspace_root, &config, "text", &[])
        .expect("advertised surface answers")
        .expect("status renders");
    assert!(status.contains("harness::status-live"));
    assert!(status.contains("Product:"));

    let json = try_live_runtime_status(&workspace_root, &config, "json", &[])
        .unwrap()
        .unwrap();
    let record: ControlStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(record.instance.instance_id, "harness::status-live");
    assert_eq!(record.product_root, product_root);

    // Shutdown withdraws the advertisement; the bypass declines again.
    handle.shutdown();
    assert!(try_live_runtime_status(&workspace_root, &config, "text", &[]).is_none());

    // A stale advertisement (dead address) also declines instead of
    // failing, so a crash leftover never breaks status.
    meld::serve::discovery::write(&product_root, "127.0.0.1:1".parse().unwrap()).unwrap();
    assert!(try_live_runtime_status(&workspace_root, &config, "text", &[]).is_none());
}
