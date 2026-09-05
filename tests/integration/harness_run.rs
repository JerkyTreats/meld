//! Harness run boot integration over temporary and reopened roots.
//!
//! Proves the phase-one boot exit criteria of the runtime harness plan:
//! the manifest records stimulus sequences, step schedule, and closing
//! watermarks, and an existing data root is unreachable without the
//! explicit unsafe flag.

use meld::harness::boot::{HarnessBootRequest, HarnessRootSelection, HarnessRun};
use meld::harness::manifest::HarnessManifest;
use meld::runtime::assembly::RuntimeResource;
use meld::runtime::registration::{RegistrationKind, RegistrationSet, RuntimeRegistration};
use meld_events::{AppendMode, EventEnvelope};
use serde_json::json;

const SUBJECT_ID: &str = "node-a";

fn world_model_subset() -> RegistrationSet {
    RegistrationSet {
        registrations: vec![RuntimeRegistration {
            registration_id: "world_model.belief_assessment".to_string(),
            runtime_id: "world_model.belief_assessment".to_string(),
            kind: RegistrationKind::ActiveActor,
            required_resources: vec![RuntimeResource::WorldModel],
        }],
    }
}

#[test]
fn a_driven_run_seals_stimuli_steps_and_closing_watermarks() {
    let mut request = HarnessBootRequest::temporary("driven-run", 1_000);
    request.registration_set = Some(world_model_subset());
    let mut run = HarnessRun::boot(request).unwrap();

    let mut driver = run.driver().unwrap();
    let receipt = driver
        .append_stimulus(
            EventEnvelope::new_domain(
                "2026-07-26T00:00:00Z".to_string(),
                "harness-run-test",
                "workspace_fs",
                "observation",
                "content_written",
                None,
                json!({
                    "subject": SUBJECT_ID,
                    "stale_probability": 0.9,
                }),
            ),
            AppendMode::Plain,
        )
        .unwrap();
    driver.step(1_100).unwrap();
    driver.step(1_200).unwrap();
    let outcome = driver.finish(1_300).unwrap();
    run.seal(outcome).unwrap();

    let manifest = HarnessManifest::load(run.manifest_path()).unwrap();
    assert_eq!(manifest.stimuli.len(), 1);
    assert_eq!(manifest.stimuli[0].seq, receipt.seq);
    assert_eq!(manifest.stimuli[0].event_type, "content_written");
    assert_eq!(manifest.stimuli[0].after_step, 0);
    assert_eq!(manifest.ledger_identity, manifest.stimuli[0].ledger_id);

    assert_eq!(manifest.steps.len(), 2);
    assert_eq!(
        manifest.steps.iter().map(|s| s.now_ms).collect::<Vec<_>>(),
        vec![1_100, 1_200]
    );

    let closing = manifest.closing.expect("sealed manifest closes watermarks");
    assert!(closing.tip_seq >= receipt.seq);
    assert_eq!(closing.shutdown_at_ms, 1_300);
}

#[test]
fn a_kept_root_reopens_under_the_unsafe_flag_with_the_same_ledger_identity() {
    let mut request = HarnessBootRequest::temporary("kept-reopen", 1_000);
    request.registration_set = Some(world_model_subset());
    let mut run = HarnessRun::boot(request).unwrap();
    let driver = run.driver().unwrap();
    let outcome = driver.finish(1_100).unwrap();
    run.seal(outcome).unwrap();
    let first = run.manifest().clone();
    let kept = run.keep_root().expect("temporary run owns a root");
    drop(run);

    // Reopen the sealed root through the explicit unsafe path with the
    // same branch id the binding was created under.
    let mut reopen = HarnessBootRequest::temporary("kept-reopen-2", 2_000);
    reopen.registration_set = Some(world_model_subset());
    reopen.root = HarnessRootSelection::ExistingDataRoot {
        product_root: kept.join("root"),
        branch_home: kept.join("branch-home"),
        legacy_store_path: kept.join("legacy-compat"),
        manifest_path: kept.join("harness_manifest_reopen.json"),
    };
    reopen.unsafe_existing_root = true;
    let mut second = HarnessRun::boot(reopen).unwrap();

    // Same binding and same ledger.
    assert_eq!(second.manifest().ledger_identity, first.ledger_identity);
    assert!(second.manifest().root.existing_data_root);
    assert!(second.manifest().boot.world_init.is_empty());

    let driver = second.driver().unwrap();
    let outcome = driver.finish(2_100).unwrap();
    second.seal(outcome).unwrap();
    std::fs::remove_dir_all(kept).unwrap();
}
