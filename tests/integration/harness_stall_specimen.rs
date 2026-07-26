//! The runtime survey's anchor stall, reproduced as a recorded harness
//! session over the shipped theory body with no manufactured anchor.
//!
//! Discipline: this specimen must not borrow the happy-path fixture's
//! seeded anchor or its inline `graph_anchor` source mapping. The theory
//! installed is exactly `theory/docs_freshness/belief_family.docs_freshness.json`
//! as shipped, the composition is stewardship-derived, and the stall is
//! observed, never worked around (no synthetic events, no store pokes, no
//! forced cursor advances).
//!
//! Phase-one exit evidence for the runtime harness plan: the thread walk
//! resolves the anchor-stall chain end to end from the session's stores —
//! genesis fact to absent anchor, including the subject-key dead end.

use std::fs;

use meld::config::{PhysicalBinding, SelectedStewardshipPackage};
use meld::harness::boot::{HarnessBootRequest, HarnessRootSelection, HarnessRun, HarnessWorldInit};
use meld::harness::manifest::HarnessManifest;
use meld::harness::walk::{ThreadCutReason, ThreadSubject, ThreadWalker};
use meld::init::world::pipeline::WorldInitContent;
use meld::init::world::{WorldInitRequest, WorldInitStage};
use meld::runtime::assembly::{
    StewardshipActorBindings, StewardshipComposition, StewardshipTheoryBindings,
};
use meld::runtime::contracts::RuntimeActionRecord;
use meld_events::DomainObjectRef;
use meld_world_model::agent::AgentCurationRuleConfig;
use meld_world_model::belief::BranchScope;
use meld_world_model::PerspectiveKey;

const SUBJECT_ID: &str = "docs";
const AGENT_ID: &str = "seed.docs_freshness";
const FAMILY_ID: &str = "docs_freshness";

/// The shipped theory body, byte for byte; no fixture-local mappings.
fn shipped_family_json() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/theory/docs_freshness/belief_family.docs_freshness.json"
    );
    fs::read_to_string(path).expect("shipped theory body exists")
}

fn binding(
    workspace_root: std::path::PathBuf,
    storage_root: std::path::PathBuf,
) -> PhysicalBinding {
    PhysicalBinding {
        workspace_root,
        subject: SUBJECT_ID.to_string(),
        agent_id: AGENT_ID.to_string(),
        provider_id: "specimen-provider".to_string(),
        package: SelectedStewardshipPackage {
            expression: "docs_freshness".to_string(),
            belief_family_id: FAMILY_ID.to_string(),
            evidence_mapping_id: "docs_freshness".to_string(),
            curation_rule_id: "docs_freshness".to_string(),
        },
        storage_root,
    }
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
            family_config: serde_json::from_str(&shipped_family_json()).unwrap(),
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
            directive: format!("steward 'docs_freshness' for subject '{SUBJECT_ID}'"),
            provenance: "meld world init".to_string(),
            session_id: "stall-specimen".to_string(),
            observed_seq: 0,
        },
    }
}

#[test]
fn the_anchor_stall_is_recorded_and_the_walk_names_the_dead_end() {
    let session = tempfile::tempdir().unwrap();
    let workspace_root = session.path().join("workspace");
    let product_root = session.path().join("root");
    fs::create_dir_all(&workspace_root).unwrap();
    let workspace_root = workspace_root.canonicalize().unwrap();

    let binding = binding(workspace_root, product_root.clone());
    let actor_bindings = StewardshipActorBindings::derive(&binding).unwrap();

    // The harness owns the session root, so binding.storage_root is known
    // before boot only through the explicit existing-root path.
    let boot_request = |manifest_id: &str, booted_at_ms: u64| {
        let mut request = HarnessBootRequest::temporary(manifest_id, booted_at_ms);
        request.root = HarnessRootSelection::ExistingDataRoot {
            product_root: product_root.clone(),
            branch_home: session.path().join("branch-home"),
            legacy_store_path: session.path().join("legacy-compat"),
            manifest_path: session.path().join(format!("{manifest_id}.json")),
        };
        request.unsafe_existing_root = true;
        request.stewardship = Some(StewardshipComposition {
            binding: binding.clone(),
            theory: StewardshipTheoryBindings::default(),
        });
        request.world_init = Some(world_init());
        request
    };

    // First boot initializes the world; actor binding checks installed
    // theory at assembly, before this init runs, exactly as the product
    // does across `meld world init` and a later `meld runtime run`.
    let first_boot = HarnessRun::boot(boot_request("stall-specimen-init", 500)).unwrap();
    assert!(first_boot
        .manifest()
        .boot
        .world_init
        .iter()
        .all(|stage| stage.disposition == "applied"));
    drop(first_boot);

    // Second boot re-assembles over the initialized root: the family now
    // resolves at assembly time, the staged init re-runs unchanged, and
    // the belief assessment actor binds.
    let mut run = HarnessRun::boot(boot_request("stall-specimen", 1_000)).unwrap();
    assert!(run
        .manifest()
        .boot
        .world_init
        .iter()
        .all(|stage| stage.disposition == "unchanged"));

    // Drive the composed runtime with injected time. No stimuli beyond
    // the stage-4 genesis fact: the stall must emerge, not be arranged.
    let mut driver = run.driver().unwrap();
    let mut actions: Vec<RuntimeActionRecord> = Vec::new();
    for now_ms in [1_100, 1_200, 1_300] {
        actions.extend(driver.step(now_ms).unwrap().actions);
    }
    let outcome = driver.finish(1_400).unwrap();
    run.seal(outcome).unwrap();

    // The stalled actor truthfully reports the anchor dead end per tick.
    let stalled: Vec<&RuntimeActionRecord> = actions
        .iter()
        .filter(|action| {
            action.actor_id.contains("belief")
                && action.issues.iter().any(|issue| {
                    // Exactly the absent-anchor stall from the survey, not
                    // the normalizer's mapping-vocabulary rejection.
                    issue.code == "assessment_failed"
                        && issue.message.contains("missing graph anchor")
                })
        })
        .collect();
    assert!(
        !stalled.is_empty(),
        "belief assessment must stall on the absent anchor; observed actions: {:?}",
        actions
            .iter()
            .map(|action| (
                action.actor_id.clone(),
                action
                    .issues
                    .iter()
                    .map(|issue| issue.message.clone())
                    .collect::<Vec<_>>()
            ))
            .collect::<Vec<_>>()
    );

    // The manifest records the stalled step schedule as the durable
    // session artifact.
    let manifest = HarnessManifest::load(run.manifest_path()).unwrap();
    assert_eq!(manifest.steps.len(), 3);
    let recorded_ids: Vec<&String> = manifest
        .steps
        .iter()
        .flat_map(|step| step.action_ids.iter())
        .collect();
    assert!(stalled
        .iter()
        .all(|action| recorded_ids.contains(&&action.action_id)));

    // End of the chain: the genesis fact exists on the ledger and the
    // walk resolves it.
    let walker = ThreadWalker::over_assembly(run.assembly());
    let genesis = walker.walk(ThreadSubject::Event { seq: 1 }).unwrap();
    assert!(
        genesis.nodes[0]
            .summary
            .contains("world_model.unobserved_scope"),
        "seq 1 is the seeded genesis fact: {}",
        genesis.nodes[0].summary
    );

    // Start of the absence: the current anchor for the exact subject key
    // the assessment queried, under the composition's hardcoded anchor
    // perspective, does not exist — and the subject key appears in no
    // anchor record at all, which is the survey's vocabulary mismatch.
    let thread = walker
        .walk(ThreadSubject::AnchorForSubject {
            subject_domain_id: "workspace_fs".to_string(),
            subject_object_kind: "node".to_string(),
            subject_object_id: SUBJECT_ID.to_string(),
            perspective_kind: actor_bindings.anchor_perspective_kind.clone(),
            perspective_id: actor_bindings.anchor_perspective_id.clone(),
        })
        .unwrap();
    assert_eq!(thread.cuts.len(), 1);
    let cut = &thread.cuts[0];
    assert_eq!(cut.reason, ThreadCutReason::AbsentRecord);
    assert!(
        cut.reference
            .contains(&format!("workspace_fs::node::{SUBJECT_ID}")),
        "the cut names the exact subject key: {}",
        cut.reference
    );
    assert!(
        cut.reference.contains("appears in no anchor record"),
        "the subject-key dead end is explicit: {}",
        cut.reference
    );

    // Re-derivation yields the identical thread: no second source of truth.
    let rederived = walker
        .walk(ThreadSubject::AnchorForSubject {
            subject_domain_id: "workspace_fs".to_string(),
            subject_object_kind: "node".to_string(),
            subject_object_id: SUBJECT_ID.to_string(),
            perspective_kind: actor_bindings.anchor_perspective_kind.clone(),
            perspective_id: actor_bindings.anchor_perspective_id.clone(),
        })
        .unwrap();
    assert_eq!(thread, rederived);
}
