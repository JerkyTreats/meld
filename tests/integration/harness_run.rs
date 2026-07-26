//! Harness run boot integration: the default temporary boot goes through
//! the staged world-initialization pipeline and seals a replayable
//! manifest.
//!
//! Proves the phase-one boot exit criteria of the runtime harness plan:
//! the default boot lands in a temporary root through the staged pipeline,
//! the manifest records identities only — stage record ids, stimulus
//! sequences, step schedule, closing watermarks — and an existing data
//! root is unreachable without the explicit unsafe flag.

use meld::harness::boot::{
    HarnessBootRequest, HarnessRootSelection, HarnessRun, HarnessWorldInit,
};
use meld::harness::manifest::HarnessManifest;
use meld::init::world::pipeline::WorldInitContent;
use meld::init::world::{WorldInitRequest, WorldInitStage};
use meld::runtime::assembly::RuntimeResource;
use meld::runtime::registration::{RegistrationKind, RegistrationSet, RuntimeRegistration};
use meld_events::{AppendMode, DomainObjectRef, EventEnvelope};
use meld_world_model::agent::AgentCurationRuleConfig;
use meld_world_model::belief::BranchScope;
use meld_world_model::PerspectiveKey;
use serde_json::json;

const FAMILY_ID: &str = "docs_freshness";
const AGENT_ID: &str = "seed.docs_freshness";
const SUBJECT_ID: &str = "node-a";

/// Frozen stage 4 record identity for the fixture subject.
const EXPECTED_GENESIS_RECORD_ID: &str =
    "genesis::world_model::observation::workspace_fs::node::node-a";

fn family_config_json() -> &'static str {
    r#"{
        "family_id": "docs_freshness",
        "dimension_id": "docs_freshness",
        "predicate_id": "confidence",
        "evidence_policy_id": "default_policy",
        "evidence_schemas": [
            {
                "schema_id": "content_written_signal",
                "required": false,
                "role": "Support",
                "reliability": 1.0,
                "precision": 1.0
            }
        ],
        "source_mappings": [
            {
                "mapping_id": "content_written_to_signal",
                "source_kind": "content_written",
                "evidence_schema_id": "content_written_signal",
                "subject_from": "record.subject",
                "value_field": "stale_probability",
                "factor_id": "content_written_signal"
            }
        ],
        "comparator": {
            "engine_id": "weighted_bayesian",
            "engine_version": "1",
            "factors": [
                {
                    "factor_id": "content_written_signal",
                    "evidence_schema_id": "content_written_signal",
                    "weight": 1.0,
                    "polarity": "Supports"
                }
            ],
            "missing_evidence_uncertainty": 0.9
        },
        "default_prior": 0.8,
        "planner_projection": {
            "confidence_field": "confidence",
            "threshold": 0.7,
            "posterior_meaning": "stale_probability"
        },
        "config_version": "1"
    }"#
}

fn world_init() -> HarnessWorldInit {
    HarnessWorldInit {
        request: WorldInitRequest {
            stages: vec![
                WorldInitStage::InstallTheory,
                WorldInitStage::GenesisIdentities,
                WorldInitStage::SeedEpistemicFacts,
            ],
        },
        content: WorldInitContent {
            family_config: serde_json::from_str(family_config_json()).unwrap(),
            curation_rule: AgentCurationRuleConfig {
                dimension_id: FAMILY_ID.to_string(),
                threshold: 0.7,
                priority_urgency: 50,
                desired_summary: "confidence>0.7".to_string(),
                source_kind: "belief_divergence".to_string(),
            },
            agent_id: AGENT_ID.to_string(),
            subject: DomainObjectRef::new("workspace_fs", "node", SUBJECT_ID).unwrap(),
            perspective: PerspectiveKey::new("default", "default").unwrap(),
            branch_scope: BranchScope::main(),
            observation_scope: FAMILY_ID.to_string(),
            directive: "steward docs freshness for node-a".to_string(),
            provenance: "harness run integration".to_string(),
            session_id: "harness-run-test".to_string(),
            observed_seq: 0,
        },
    }
}

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
fn default_boot_lands_in_a_temporary_root_through_the_staged_pipeline() {
    let mut request = HarnessBootRequest::temporary("staged-boot", 1_000);
    request.registration_set = Some(world_model_subset());
    request.world_init = Some(world_init());
    let run = HarnessRun::boot(request).unwrap();

    let manifest = run.manifest();
    assert!(manifest.root.temporary);
    assert!(!manifest.root.existing_data_root);
    assert!(manifest.root.product_root.exists());
    assert!(manifest.root.product_root.join("ledger.sled").exists());

    let stages: Vec<(&str, &str)> = manifest
        .boot
        .world_init
        .iter()
        .map(|stage| (stage.stage.as_str(), stage.disposition.as_str()))
        .collect();
    assert_eq!(
        stages,
        vec![
            ("install-theory", "applied"),
            ("genesis-identities", "applied"),
            ("seed-epistemic-facts", "applied"),
        ]
    );
    let genesis_ids = &manifest.boot.world_init[2].record_ids;
    assert_eq!(genesis_ids, &vec![EXPECTED_GENESIS_RECORD_ID.to_string()]);
}

#[test]
fn a_driven_run_seals_stimuli_steps_and_closing_watermarks() {
    let mut request = HarnessBootRequest::temporary("driven-run", 1_000);
    request.registration_set = Some(world_model_subset());
    request.world_init = Some(world_init());
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
    request.world_init = Some(world_init());
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
    reopen.world_init = Some(world_init());
    reopen.root = HarnessRootSelection::ExistingDataRoot {
        product_root: kept.join("root"),
        branch_home: kept.join("branch-home"),
        legacy_store_path: kept.join("legacy-compat"),
        manifest_path: kept.join("harness_manifest_reopen.json"),
    };
    reopen.unsafe_existing_root = true;
    let mut second = HarnessRun::boot(reopen).unwrap();

    // Same binding, same ledger; the world-init re-run is a no-op.
    assert_eq!(second.manifest().ledger_identity, first.ledger_identity);
    assert!(second.manifest().root.existing_data_root);
    assert!(second
        .manifest()
        .boot
        .world_init
        .iter()
        .all(|stage| stage.disposition == "unchanged"));

    let driver = second.driver().unwrap();
    let outcome = driver.finish(2_100).unwrap();
    second.seal(outcome).unwrap();
    std::fs::remove_dir_all(kept).unwrap();
}
