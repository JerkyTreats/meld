//! The runtime survey's anchor stall, reproduced as a recorded harness
//! session over the shipped theory body with no manufactured anchor.
//!
//! Discipline: this specimen must not borrow the happy-path fixture's
//! seeded anchor or its inline `graph_anchor` source mapping. The theory
//! installed is exactly the shipped body, the composition is
//! stewardship-derived, and the stall is observed, never worked around
//! (no synthetic events, no store pokes, no forced cursor advances).
//!
//! Phase-one and phase-two exit evidence for the runtime harness plan:
//! the thread walk resolves the anchor-stall chain end to end, and the
//! recorded waiting-on declarations resolve the absence through the
//! eligibility walk.

use std::fs;

use super::harness_survey_fixture::{survey_binding, survey_boot_request, SUBJECT_ID};
use meld::harness::boot::HarnessRun;
use meld::harness::manifest::HarnessManifest;
use meld::harness::walk::{ThreadCutReason, ThreadSubject, ThreadWalker};
use meld::runtime::assembly::StewardshipActorBindings;
use meld::runtime::contracts::RuntimeActionRecord;

#[test]
fn the_anchor_stall_is_recorded_and_the_walk_names_the_dead_end() {
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

    // DBG-016 exit evidence: every stalled tick carries a waiting-on
    // declaration naming the absent anchor and the exact subject key.
    for action in &stalled {
        let declaration = action
            .waiting_on
            .iter()
            .find(|declaration| declaration.condition == "graph_anchor_absent")
            .expect("stalled tick declares the absent anchor");
        assert_eq!(
            declaration.subject_key.as_deref(),
            Some(format!("workspace_fs::node::{SUBJECT_ID}").as_str()),
            "the declaration names the exact subject key"
        );
        assert!(
            declaration
                .detail
                .contains(&actor_bindings.anchor_perspective_id),
            "the declaration names the anchor perspective: {}",
            declaration.detail
        );
    }

    // The declarations are durable: the report store serves them back
    // after the run, which is what the eligibility walk reads.
    {
        use meld::harness::eligibility::{
            AbsentRecordKind, EligibilityQuestion, EligibilityWalker,
        };
        use meld::runtime::contracts::RuntimeStatusReader;
        let reports = meld::runtime::supervisor::SupervisorReportStore::open(
            run.assembly().supervisor_store(),
        )
        .unwrap();
        let durable = reports.read_recent_actions(64).unwrap();
        assert!(durable.iter().any(|action| {
            action
                .waiting_on
                .iter()
                .any(|declaration| declaration.condition == "graph_anchor_absent")
        }));

        // Phase-two exit evidence: the eligibility walk resolves the
        // absent revision to its declaration chain, presenting the absent
        // anchor and the subject-vocabulary mismatch as the reason the
        // assessed revision does not exist.
        let traversal = run
            .assembly()
            .stores()
            .traversal_store
            .opened()
            .expect("traversal store is open in the survey composition");
        let chain = EligibilityWalker::new(&reports)
            .with_traversal(traversal)
            .why_absent(EligibilityQuestion {
                kind: AbsentRecordKind::BeliefRevision,
                subject_key: Some(format!("workspace_fs::node::{SUBJECT_ID}")),
            })
            .unwrap();
        assert!(chain
            .links
            .iter()
            .any(|link| link.runtime_id == "world_model.belief_assessment"
                && link.declaration.condition == "graph_anchor_absent"));
        let divergence = chain
            .divergences
            .iter()
            .find(|divergence| divergence.starts_with("graph_anchor_absent"))
            .expect("the chain names the anchor divergence");
        assert!(
            divergence.contains("appears in no anchor record"),
            "the divergence presents the subject-vocabulary mismatch: {divergence}"
        );

        // A real coupling hop over live emissions: an absent goal command
        // walks through curation's quiet selection to the same anchor
        // divergence, so the condition vocabulary the walk compiles
        // against is proven to match what the domains actually emit.
        let goal_chain = EligibilityWalker::new(&reports)
            .with_traversal(traversal)
            .why_absent(EligibilityQuestion {
                kind: AbsentRecordKind::GoalCommand,
                subject_key: Some(format!("workspace_fs::node::{SUBJECT_ID}")),
            })
            .unwrap();
        let hops: Vec<&str> = goal_chain
            .links
            .iter()
            .map(|link| link.runtime_id.as_str())
            .collect();
        assert_eq!(
            hops,
            vec![
                "world_model.agent_goal_curation",
                "world_model.belief_assessment",
            ],
            "the goal absence walks one live coupling hop; divergences: {:?}",
            goal_chain.divergences
        );
        assert!(goal_chain
            .divergences
            .iter()
            .any(|divergence| divergence.contains("appears in no anchor record")));
    }

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
