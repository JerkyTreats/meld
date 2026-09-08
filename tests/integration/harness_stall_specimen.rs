//! A required family waits for admitted evidence without resurrecting Graph anchors.

use std::fs;

use super::harness_survey_fixture::{survey_binding, survey_boot_request, SUBJECT_ID};
use meld::harness::boot::HarnessRun;
use meld::harness::manifest::HarnessManifest;
use meld::harness::walk::ThreadSubject;
use meld::runtime::assembly::StewardshipActorBindings;
use meld::runtime::contracts::RuntimeActionRecord;

#[test]
fn unobserved_required_family_does_not_invent_anchor_evidence() {
    let session = tempfile::tempdir().unwrap();
    let workspace_root = session.path().join("workspace");
    let product_root = session.path().join("root");
    fs::create_dir_all(&workspace_root).unwrap();
    let workspace_root = workspace_root.canonicalize().unwrap();

    let binding = survey_binding(workspace_root, product_root.clone());
    let actor_bindings = StewardshipActorBindings::derive(&binding).unwrap();

    // The harness owns the session root, so binding.storage_root is known
    // before boot only through the explicit existing-root path.
    let boot_request = |manifest_id: &str, booted_at_ms: u64| {
        survey_boot_request(session.path(), &binding, manifest_id, booted_at_ms)
    };

    // The fixture installs its historical family directly through the
    // Belief owner before assembly, so the harness itself has no alternate
    // world-initialization authority.
    let mut run = HarnessRun::boot(boot_request("stall-specimen", 1_000)).unwrap();

    // Drive the composed runtime with injected time. No stimuli beyond
    // the installed test fixture: the stall must emerge, not be arranged.
    let mut driver = run.driver().unwrap();
    let mut actions: Vec<RuntimeActionRecord> = Vec::new();
    for now_ms in [1_100, 1_200, 1_300] {
        actions.extend(driver.step(now_ms).unwrap().actions);
    }
    let outcome = driver.finish(1_400).unwrap();
    run.seal(outcome).unwrap();

    let belief = run.assembly().stores().belief_store.opened().unwrap();
    assert!(meld_world_model::belief::BeliefQuery::new(belief.as_ref())
        .current_views_for_subject(&actor_bindings.subject, &actor_bindings.perspective)
        .unwrap()
        .is_empty());
    assert!(actions.iter().all(|action| action
        .issues
        .iter()
        .all(|issue| !issue.message.contains("missing graph anchor"))));
    assert!(actions
        .iter()
        .any(|action| action.actor_id.contains("belief")));
    assert_eq!(
        HarnessManifest::load(run.manifest_path())
            .unwrap()
            .steps
            .len(),
        3
    );
    assert!(serde_json::from_value::<ThreadSubject>(serde_json::json!({
        "AnchorForSubject": { "subject_domain_id": "workspace_fs", "subject_object_kind": "node", "subject_object_id": SUBJECT_ID,
            "perspective_kind": "frame_type", "perspective_id": "analysis" }
    })).is_err());
}
