//! The served substrate over a live survey session: `/v1` endpoints whose
//! JSON bodies are the existing contract types verbatim.
//!
//! Proves the phase-three serving layer: the event authority contract and
//! the report store reader served over loopback HTTP, the walks resolving
//! over the wire, machine-readable errors, and the long-poll watch
//! answering while the run is live.

use std::fs;

use super::harness_survey_fixture::{survey_binding, survey_boot_request, AGENT_ID, SUBJECT_ID};
use meld::harness::boot::HarnessRun;
use meld::harness::eligibility::EligibilityChain;
use meld::harness::walk::CausalThread;
use meld::runtime::contracts::RuntimeActionRecord;
use meld::serve::listener::serve;
use meld::serve::sources::ServeSources;
use meld_events::{EventPage, EventWatermark, LedgerCursor, ReplayRequest};
use serde_json::json;

fn post(addr: std::net::SocketAddr, path: &str, body: serde_json::Value) -> ureq::Response {
    ureq::post(&format!("http://{addr}{path}"))
        .send_json(body)
        .unwrap_or_else(|error| match error {
            ureq::Error::Status(_, response) => response,
            other => panic!("transport failure on {path}: {other}"),
        })
}

fn get(addr: std::net::SocketAddr, path: &str) -> ureq::Response {
    ureq::get(&format!("http://{addr}{path}"))
        .call()
        .unwrap_or_else(|error| match error {
            ureq::Error::Status(_, response) => response,
            other => panic!("transport failure on {path}: {other}"),
        })
}

#[test]
fn the_substrate_serves_contract_types_over_loopback_for_a_live_session() {
    let session = tempfile::tempdir().unwrap();
    let workspace_root = session.path().join("workspace");
    fs::create_dir_all(&workspace_root).unwrap();
    let binding = survey_binding(
        workspace_root.canonicalize().unwrap(),
        session.path().join("root"),
    );

    // Two-boot survey structure: init, then a bound composition.
    drop(
        HarnessRun::boot(survey_boot_request(
            session.path(),
            &binding,
            "served-init",
            500,
        ))
        .unwrap(),
    );
    let mut run = HarnessRun::boot(survey_boot_request(
        session.path(),
        &binding,
        "served-run",
        1_000,
    ))
    .unwrap();

    let sources = ServeSources::from_assembly(run.assembly()).unwrap();
    let ledger_id = sources.ledger_identity();
    let handle = serve(sources, 0).unwrap();
    let addr = handle.addr();

    // Drive the run while the surface is up: the stall emerges live.
    let mut driver = run.driver().unwrap();
    for now_ms in [1_100, 1_200, 1_300] {
        driver.step(now_ms).unwrap();
    }

    let served_ledger_id: meld_events::LedgerIdentity =
        get(addr, "/v1/ledger").into_json().unwrap();
    assert_eq!(served_ledger_id, ledger_id);

    // Watermark: the genesis fact is durable and visible over the wire.
    let watermark: EventWatermark = post(
        addr,
        "/v1/events/watermark",
        json!({ "ledger_id": ledger_id }),
    )
    .into_json()
    .unwrap();
    assert!(watermark.tip_seq >= 1);

    // Replay: the wire body is the contract type verbatim.
    let request = ReplayRequest {
        cursor: LedgerCursor {
            ledger_id,
            after_seq: 0,
        },
        limit: 8,
    };
    let page: EventPage = post(
        addr,
        "/v1/events/replay",
        serde_json::to_value(request).unwrap(),
    )
    .into_json()
    .unwrap();
    assert!(page
        .records
        .iter()
        .any(|record| record.event_type == "world_model.unobserved_scope"));

    // The report reader serves the per-tick declarations live.
    let actions: Vec<RuntimeActionRecord> =
        post(addr, "/v1/reports/recent_actions", json!({ "limit": 64 }))
            .into_json()
            .unwrap();
    assert!(actions.iter().any(|action| {
        action
            .waiting_on
            .iter()
            .any(|declaration| declaration.condition == "graph_anchor_absent")
    }));

    // Both walks answer over the wire with the same divergence the
    // in-process walks derive.
    let thread: CausalThread = post(
        addr,
        "/v1/walks/thread",
        json!({
            "subject": {
                "AnchorForSubject": {
                    "subject_domain_id": "workspace_fs",
                    "subject_object_kind": "node",
                    "subject_object_id": SUBJECT_ID,
                    "perspective_kind": "frame_type",
                    // The perspective the composed bindings actually derive,
                    // so the walk explains the specimen's real stall rather
                    // than any absent anchor.
                    "perspective_id": format!("context-{AGENT_ID}"),
                }
            }
        }),
    )
    .into_json()
    .unwrap();
    assert_eq!(thread.cuts.len(), 1);
    assert!(thread.cuts[0]
        .reference
        .contains("appears in no anchor record"));

    let chain: EligibilityChain = post(
        addr,
        "/v1/walks/eligibility",
        json!({
            "kind": "BeliefRevision",
            "subject_key": format!("workspace_fs::node::{SUBJECT_ID}"),
        }),
    )
    .into_json()
    .unwrap();
    assert!(chain
        .divergences
        .iter()
        .any(|divergence| divergence.contains("appears in no anchor record")));

    // Errors are machine-readable, never prose pages.
    let missing = post(addr, "/v1/no/such/route", json!({}));
    assert_eq!(missing.status(), 404);
    let body: serde_json::Value = missing.into_json().unwrap();
    assert!(body["error"].as_str().unwrap().contains("no route"));

    let invalid = post(addr, "/v1/events/watermark", json!({ "nonsense": true }));
    assert_eq!(invalid.status(), 400);

    let outcome = driver.finish(1_400).unwrap();
    run.seal(outcome).unwrap();

    // Byte-consistency across live and playback: capture the sealed
    // session's answers over the live surface, then mount the same
    // handlers over the sealed root and compare bodies byte for byte.
    let scope = json!({
        "name": "consistency-scope",
        "subject_keys": [format!("workspace_fs::node::{SUBJECT_ID}")],
        "actor_ids": ["world_model.belief_assessment"],
    });
    let probes: Vec<(&str, serde_json::Value)> = vec![
        ("/v1/reports/recent_actions", json!({ "limit": 64 })),
        (
            "/v1/walks/eligibility",
            json!({
                "kind": "BeliefRevision",
                "subject_key": format!("workspace_fs::node::{SUBJECT_ID}"),
            }),
        ),
        ("/v1/events/replay", serde_json::to_value(request).unwrap()),
        (
            "/v1/projections/subagent",
            json!({ "scope": scope, "after_seq": 0 }),
        ),
        (
            "/v1/projections/parent",
            json!({ "scope": scope, "after_seq": 0 }),
        ),
        ("/v1/events/watermark", json!({ "ledger_id": ledger_id })),
    ];
    let live_bodies: Vec<String> = probes
        .iter()
        .map(|(path, body)| post(addr, path, body.clone()).into_string().unwrap())
        .collect();
    handle.shutdown();
    drop(run);

    // Playback: the sealed root boots through the same staged path (no
    // init requested, nothing driven) and mounts the identical handlers.
    let mut playback_request =
        survey_boot_request(session.path(), &binding, "served-playback", 2_000);
    playback_request.world_init = None;
    let playback = HarnessRun::boot(playback_request).unwrap();
    // A sealed session root's whole retained history IS the session, so
    // the playback mount lifts the live boot fence explicitly.
    let playback_handle = serve(
        ServeSources::from_assembly(playback.assembly())
            .unwrap()
            .with_full_history(),
        0,
    )
    .unwrap();
    let playback_bodies: Vec<String> = probes
        .iter()
        .map(|(path, body)| {
            post(playback_handle.addr(), path, body.clone())
                .into_string()
                .unwrap()
        })
        .collect();
    assert_eq!(
        live_bodies, playback_bodies,
        "live and playback surfaces serve identical bytes"
    );
    playback_handle.shutdown();
}
