//! The authoritative harness gate: one real stall presented at all three
//! customer altitudes from one session record, with every projection
//! citing shared record identities, and the presentation made by a
//! separate process consuming the served surface.
//!
//! The consumer is this same test binary re-invoked as a child process —
//! an external reader that knows only the published URL and the JSON
//! bodies. That observation proves the boundary is real rather than
//! layered code in one binary.

use std::fs;
use std::process::Command;

use super::harness_survey_fixture::{survey_binding, survey_boot_request, SUBJECT_ID};
use meld::harness::boot::HarnessRun;
use meld::harness::projections::{
    ParentProjection, SubagentProjection, TrajectoryKind, UserProjection,
};
use meld::serve::listener::serve_with_discovery;
use meld::serve::sources::ServeSources;
use serde_json::json;

/// Child-process consumer: runs only when re-invoked with the gate URL,
/// consumes the three projections over HTTP, and prints one JSON line.
#[test]
fn child_consumer() {
    let Ok(base) = std::env::var("MELD_GATE_URL") else {
        // Normal test runs skip silently; only the gate's child does work.
        return;
    };
    let subject_key = std::env::var("MELD_GATE_SUBJECT").expect("gate subject");
    let scope = json!({
        "name": "gate-scope",
        "subject_keys": [subject_key],
        "actor_ids": ["world_model.belief_assessment"],
    });
    let consume = |path: &str, body: Option<serde_json::Value>| -> serde_json::Value {
        let url = format!("{base}{path}");
        let response = match body {
            Some(body) => ureq::post(&url).send_json(body),
            None => ureq::get(&url).call(),
        }
        .expect("gate request succeeds");
        response.into_json().expect("gate response is JSON")
    };
    let result = json!({
        "subagent": consume(
            "/v1/projections/subagent",
            Some(json!({ "scope": scope, "after_seq": 0 })),
        ),
        "parent": consume(
            "/v1/projections/parent",
            Some(json!({ "scope": scope, "after_seq": 0 })),
        ),
        "user": consume("/v1/projections/user", None),
    });
    println!("GATE_RESULT {result}");
}

#[test]
fn the_stall_presents_at_three_altitudes_to_a_separate_process() {
    let session = tempfile::tempdir().unwrap();
    let workspace_root = session.path().join("workspace");
    let product_root = session.path().join("root");
    fs::create_dir_all(&workspace_root).unwrap();
    let binding = survey_binding(workspace_root.canonicalize().unwrap(), product_root.clone());

    // The survey session: one bound boot and three stalled ticks.
    let mut run = HarnessRun::boot(survey_boot_request(
        session.path(),
        &binding,
        "gate-run",
        1_000,
    ))
    .unwrap();
    // Sources own their handles, so the surface can outlive the borrow
    // the driver takes on the run.
    let handle = serve_with_discovery(
        ServeSources::from_assembly(run.assembly()).unwrap(),
        0,
        &product_root,
    )
    .unwrap();
    let mut driver = run.driver().unwrap();
    for now_ms in [1_100, 1_200, 1_300] {
        driver.step(now_ms).unwrap();
    }

    // The separate process: this test binary re-invoked as a consumer of
    // the published URL, with no in-process access to any store.
    let subject_key = format!("workspace_fs::node::{SUBJECT_ID}");
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "integration::harness_three_altitudes::child_consumer",
            "--exact",
            "--nocapture",
        ])
        .env("MELD_GATE_URL", format!("http://{}", handle.addr()))
        .env("MELD_GATE_SUBJECT", &subject_key)
        .output()
        .expect("child consumer spawns");
    assert!(
        output.status.success(),
        "child consumer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find_map(|line| line.strip_prefix("GATE_RESULT "))
        .expect("child printed its gate result");
    let result: serde_json::Value = serde_json::from_str(line).unwrap();

    // Subagent altitude: the per-tick waiting-on naming the absent anchor
    // for its exact subject key.
    let subagent: SubagentProjection = serde_json::from_value(result["subagent"].clone()).unwrap();
    let report = subagent
        .actor_reports
        .iter()
        .find(|report| report.runtime_id == "world_model.belief_assessment")
        .expect("subagent sees its owned actor");
    let declaration = report
        .waiting_on
        .iter()
        .find(|declaration| declaration.condition == "graph_anchor_absent")
        .expect("subagent reads the absent anchor");
    assert_eq!(
        declaration.subject_key.as_deref(),
        Some(subject_key.as_str())
    );
    let cited_action_id = report.action_id.clone();

    // Parent altitude: one repeating failure signature over the same
    // durable records, silent about everything else in scope.
    let parent: ParentProjection = serde_json::from_value(result["parent"].clone()).unwrap();
    let signature = parent
        .trajectory_signatures
        .iter()
        .find(|signature| signature.kind == TrajectoryKind::RepeatingIssue)
        .expect("parent sees the repeating failure signature");
    assert!(signature.signature.contains("assessment_failed"));
    assert!(signature.consecutive_ticks >= 2);
    assert!(
        signature.action_ids.contains(&cited_action_id),
        "parent cites the same durable action the subagent cites"
    );

    // User altitude: the coupling that never flowed, waiting on the same
    // absence, citing the same latest record.
    let user: UserProjection = serde_json::from_value(result["user"].clone()).unwrap();
    let coupling = user
        .couplings
        .iter()
        .find(|coupling| coupling.runtime_id == "world_model.belief_assessment")
        .expect("user sees the assessment coupling");
    assert!(coupling.attempted > 0);
    assert_eq!(coupling.committed, 0, "the coupling never flowed");
    assert!(coupling
        .waiting_on
        .iter()
        .any(|declaration| declaration.condition == "graph_anchor_absent"
            && declaration.subject_key.as_deref() == Some(subject_key.as_str())));
    assert_eq!(
        coupling.latest_action_id.as_deref(),
        Some(cited_action_id.as_str()),
        "user cites the same durable action the other altitudes cite"
    );

    let outcome = driver.finish(1_400).unwrap();
    run.seal(outcome).unwrap();
    handle.shutdown();
}
