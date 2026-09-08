//! Belief selection, bounded assessment, and planner causality tests.
//!
//! Covers the store-backed family registry, exact-key queries, deterministic
//! bounded selection, the bounded assessment actor (budget, reopen resume,
//! quiescence), and theory-revision lineage through to planner projection.

mod support;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use meld_world_model::belief::{
    configured_belief_key, AssessmentLease, BeliefAssessmentActor, BeliefAssessmentRequest,
    BeliefEvidenceNormalizer, BeliefFamilyRegistry, BeliefFamilyRegistryStore,
    BeliefFamilyRevision, BeliefKey, BeliefQuery, BeliefRuntime, BeliefStore, BeliefSubjectBinding,
    BranchScope, LeaseStatus, PromotedEvidenceRecord, TheoryInstallDisposition,
};
use meld_world_model::events::DomainObjectRef;
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::{EvidenceValue, PerspectiveKey, PlannerSourceRef};

const FAMILY_ID: &str = "artifact_currency";

fn reopen_sled_after_close(path: &Path) -> sled::Result<sled::Db> {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        match sled::open(path) {
            Ok(db) => return Ok(db),
            Err(sled::Error::Io(error))
                if error.kind() == std::io::ErrorKind::WouldBlock
                    && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error),
        }
    }
}

fn object(domain_id: &str, object_kind: &str, object_id: &str) -> DomainObjectRef {
    DomainObjectRef::new(domain_id, object_kind, object_id).unwrap()
}

fn family_json(config_version: &str) -> String {
    format!(
        r#"{{
        "family_id": "{FAMILY_ID}",
        "dimension_id": "{FAMILY_ID}",
        "predicate_id": "confidence",
        "evidence_policy_id": "default_policy",
        "evidence_schemas": [
            {{
                "schema_id": "graph_anchor_signal",
                "required": true,
                "role": "Support",
                "reliability": 1.0,
                "precision": 1.0
            }},
            {{
                "schema_id": "content_written_signal",
                "required": false,
                "role": "Support",
                "reliability": 1.0,
                "precision": 1.0
            }},
            {{
                "schema_id": "content_review_signal",
                "required": false,
                "role": "Support",
                "reliability": 1.0,
                "precision": 1.0
            }}
        ],
        "source_mappings": [
            {{
                "mapping_id": "anchor_to_signal",
                "source_kind": "initial_observation",
                "evidence_schema_id": "graph_anchor_signal",
                "subject_from": "record.subject",
                "value_field": "ended",
                "factor_id": "anchor_signal"
            }},
            {{
                "mapping_id": "content_written_to_signal",
                "source_kind": "content_written",
                "evidence_schema_id": "content_written_signal",
                "subject_from": "record.subject",
                "value_field": "stale_probability",
                "factor_id": "content_written_signal"
            }},
            {{
                "mapping_id": "content_written_to_review",
                "source_kind": "content_written",
                "evidence_schema_id": "content_review_signal",
                "subject_from": "record.subject",
                "value_field": "review_probability",
                "factor_id": "content_review_signal"
            }}
        ],
        "comparator": {{
            "engine_id": "weighted_bayesian",
            "engine_version": "1",
            "factors": [
                {{
                    "factor_id": "anchor_signal",
                    "evidence_schema_id": "graph_anchor_signal",
                    "weight": 1.0,
                    "polarity": "Supports"
                }},
                {{
                    "factor_id": "content_written_signal",
                    "evidence_schema_id": "content_written_signal",
                    "weight": 1.0,
                    "polarity": "Supports"
                }},
                {{
                    "factor_id": "content_review_signal",
                    "evidence_schema_id": "content_review_signal",
                    "weight": 1.0,
                    "polarity": "Supports"
                }}
            ],
            "missing_evidence_uncertainty": 0.9
        }},
        "default_prior": 0.8,
        "planner_projection": {{
            "confidence_field": "confidence",
            "threshold": 0.7,
            "posterior_meaning": "stale_probability"
        }},
        "config_version": "{config_version}"
    }}"#
    )
}

fn family_config(config_version: &str) -> meld_world_model::belief::BeliefFamilyConfig {
    serde_json::from_str(&family_json(config_version)).unwrap()
}

fn seed_subject(store: &BeliefStore, object_id: &str, seq: u64) -> DomainObjectRef {
    let node = object("workspace_fs", "node", object_id);
    let normalizer = BeliefEvidenceNormalizer::new(
        family_config("1"),
        default_perspective(),
        BranchScope::main(),
    );
    let mut record = promoted_record(node.clone(), seq);
    record.source_kind = "initial_observation".into();
    record
        .fields
        .insert("ended".into(), EvidenceValue::Scalar(0.0));
    for item in normalizer.normalize_promoted(&record).unwrap() {
        store.put_evidence_once(&item).unwrap();
        store
            .put_assignment_once(&normalizer.assign(&item).unwrap())
            .unwrap();
    }
    node
}

fn binding(node: &DomainObjectRef) -> BeliefSubjectBinding {
    BeliefSubjectBinding {
        subject: node.clone(),
    }
}

fn default_perspective() -> PerspectiveKey {
    PerspectiveKey::new("default", "default").unwrap()
}

fn promoted_record(node: DomainObjectRef, seq: u64) -> PromotedEvidenceRecord {
    let mut fields = BTreeMap::new();
    fields.insert("stale_probability".to_string(), EvidenceValue::Scalar(0.0));
    fields.insert("review_probability".to_string(), EvidenceValue::Scalar(0.2));
    PromotedEvidenceRecord {
        publication_record_id: None,
        outcome_mapping_revision: None,
        source_kind: "content_written".to_string(),
        source_id: format!("content-written-{seq}"),
        subject: node.clone(),
        source_fact_ids: vec![format!("ledger-content-{seq}")],
        graph_anchor_ids: Vec::new(),
        objects: vec![node],
        relations: Vec::new(),
        source_cursor_start: seq,
        source_cursor_end: seq,
        reference_time: None,
        transaction_seq: seq,
        content_hash: Some(format!("hash-{seq}")),
        fields,
    }
}

struct Fixture {
    _graph_dir: tempfile::TempDir,
    _belief_dir: tempfile::TempDir,
    graph: Arc<TraversalStore>,
    belief: Arc<BeliefStore>,
    registry: BeliefFamilyRegistryStore,
    revision: BeliefFamilyRevision,
    subjects: Vec<DomainObjectRef>,
}

fn fixture(subject_ids: &[&str]) -> Fixture {
    let graph_dir = tempfile::tempdir().unwrap();
    let belief_dir = tempfile::tempdir().unwrap();
    let graph =
        Arc::new(TraversalStore::new(sled::open(graph_dir.path().join("graph")).unwrap()).unwrap());
    let belief_db = sled::open(belief_dir.path().join("belief")).unwrap();
    let belief = Arc::new(BeliefStore::new(belief_db.clone()).unwrap());
    let mut registry = BeliefFamilyRegistryStore::new(belief_db).unwrap();
    let (_, revision) = registry.install(family_config("1"), 1).unwrap();
    let subjects = subject_ids
        .iter()
        .enumerate()
        .map(|(index, id)| seed_subject(belief.as_ref(), id, index as u64 + 1))
        .collect();
    Fixture {
        _graph_dir: graph_dir,
        _belief_dir: belief_dir,
        graph,
        belief,
        registry,
        revision,
        subjects,
    }
}

fn actor(fixture: &Fixture) -> BeliefAssessmentActor {
    BeliefAssessmentActor::new(
        "belief.assessment.test",
        Arc::clone(&fixture.belief),
        Arc::new(fixture.registry.clone()),
        vec![FAMILY_ID.to_string()],
        fixture.subjects.iter().map(binding).collect(),
        default_perspective(),
        BranchScope::main(),
    )
}

#[test]
fn registry_reinstall_same_hash_is_unchanged_and_creates_no_revision() {
    let dir = tempfile::tempdir().unwrap();
    let mut registry =
        BeliefFamilyRegistryStore::new(sled::open(dir.path().join("registry")).unwrap()).unwrap();

    let (first_disposition, first) = registry.install(family_config("1"), 1).unwrap();
    let (second_disposition, second) = registry.install(family_config("1"), 9).unwrap();

    assert_eq!(first_disposition, TheoryInstallDisposition::Installed);
    assert_eq!(second_disposition, TheoryInstallDisposition::Unchanged);
    assert_eq!(second.content_hash, first.content_hash);
    // The no-op reinstall keeps the original installation sequence.
    assert_eq!(second.installed_at_seq, 1);
    assert_eq!(registry.installed_families().unwrap().len(), 1);
    assert_eq!(
        registry.current(FAMILY_ID).unwrap().unwrap().content_hash,
        first.content_hash
    );
}

#[test]
fn registry_changed_config_creates_new_revision_and_keeps_old_resolvable() {
    let dir = tempfile::tempdir().unwrap();
    let mut registry =
        BeliefFamilyRegistryStore::new(sled::open(dir.path().join("registry")).unwrap()).unwrap();

    let (_, first) = registry.install(family_config("1"), 1).unwrap();
    let (disposition, second) = registry.install(family_config("2"), 7).unwrap();

    assert_eq!(disposition, TheoryInstallDisposition::Installed);
    assert_ne!(second.content_hash, first.content_hash);
    assert_eq!(second.installed_at_seq, 7);
    assert_eq!(
        registry.current(FAMILY_ID).unwrap().unwrap().content_hash,
        second.content_hash
    );
    let old = registry
        .resolve(FAMILY_ID, &first.content_hash)
        .unwrap()
        .expect("prior revision stays resolvable");
    assert_eq!(old.installed_at_seq, 1);
    assert_eq!(old.config.config_version, "1");
    assert!(registry
        .resolve(FAMILY_ID, &second.content_hash)
        .unwrap()
        .is_some());
}

#[test]
fn registry_revisions_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("registry");
    let first_hash;
    {
        let mut registry = BeliefFamilyRegistryStore::new(sled::open(&path).unwrap()).unwrap();
        let (_, first) = registry.install(family_config("1"), 1).unwrap();
        registry.install(family_config("2"), 2).unwrap();
        first_hash = first.content_hash;
    }

    let registry = BeliefFamilyRegistryStore::new(reopen_sled_after_close(&path).unwrap()).unwrap();

    assert_eq!(
        registry
            .current(FAMILY_ID)
            .unwrap()
            .unwrap()
            .config
            .config_version,
        "2"
    );
    assert!(registry.resolve(FAMILY_ID, &first_hash).unwrap().is_some());
}

#[test]
fn exact_key_query_returns_the_revision_identity_the_store_holds() {
    let fixture = fixture(&["node-a"]);
    let runtime = BeliefRuntime::from_family_revision(
        Arc::clone(&fixture.belief),
        &fixture.revision,
        default_perspective(),
        BranchScope::main(),
    );
    let result = runtime
        .assess_subject(&fixture.subjects[0], "worker-a")
        .unwrap();

    let key = configured_belief_key(
        &fixture.revision,
        &fixture.subjects[0],
        &default_perspective(),
        &BranchScope::main(),
    );
    let query = BeliefQuery::new(fixture.belief.as_ref());
    let (revision, view) = query
        .current_revision_and_view(&key)
        .unwrap()
        .expect("exact key resolves");

    assert_eq!(revision.revision_id, result.revision_id);
    assert_eq!(view.current_revision_id, Some(result.revision_id.clone()));
    assert_eq!(
        query.current_revision(&key).unwrap().unwrap().revision_id,
        result.revision_id
    );
    assert_eq!(
        revision.theory_revision,
        Some(fixture.revision.revision_ref())
    );
    assert_eq!(view.theory_revision, Some(fixture.revision.revision_ref()));
}

#[test]
fn bounded_selector_returns_deterministic_order_and_respects_budget() {
    // Bindings arrive intentionally unsorted.
    let fixture = fixture(&["node-c", "node-a", "node-b"]);
    let selector = meld_world_model::belief::BeliefWorkSelector::new(fixture.belief.as_ref());
    let families = vec![fixture.revision.clone()];
    let subjects: Vec<_> = fixture.subjects.iter().map(binding).collect();

    let first = selector
        .select(
            &families,
            &subjects,
            &default_perspective(),
            &BranchScope::main(),
            2,
        )
        .unwrap();
    let second = selector
        .select(
            &families,
            &subjects,
            &default_perspective(),
            &BranchScope::main(),
            2,
        )
        .unwrap();
    let unbounded = selector
        .select(
            &families,
            &subjects,
            &default_perspective(),
            &BranchScope::main(),
            10,
        )
        .unwrap();

    assert_eq!(first.items.len(), 2);
    assert!(first.more_available);
    let selected: Vec<_> = first
        .items
        .iter()
        .map(|item| item.key.subject.object_id.clone())
        .collect();
    assert_eq!(selected, vec!["node-a".to_string(), "node-b".to_string()]);
    // Same durable state, same selection.
    assert_eq!(second, first);
    assert_eq!(unbounded.items.len(), 3);
    assert!(!unbounded.more_available);
}

#[test]
fn actor_processes_exactly_budget_items_then_reaches_quiescence() {
    let fixture = fixture(&["node-a", "node-b", "node-c"]);
    let mut actor = actor(&fixture);

    let first = actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 10,
        max_items: 2,
    });
    let second = actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 11,
        max_items: 2,
    });
    let third = actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 12,
        max_items: 2,
    });
    let fourth = actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 13,
        max_items: 2,
    });

    assert_eq!(first.items_attempted, 2);
    assert_eq!(first.items_committed, 2);
    assert!(first.budget_exhausted);
    assert_eq!(first.input_checkpoint, 0);
    assert_eq!(first.output_checkpoint, 10);
    assert_eq!(second.items_attempted, 1);
    assert_eq!(second.items_committed, 1);
    assert!(!second.budget_exhausted);
    assert_eq!(second.input_checkpoint, 10);
    // Unchanged durable state: repeated ticks attempt and commit nothing.
    for report in [&third, &fourth] {
        assert_eq!(report.items_attempted, 0);
        assert_eq!(report.items_committed, 0);
        assert!(!report.budget_exhausted);
        assert!(report.retryable_errors.is_empty());
        assert!(report.fatal_errors.is_empty());
    }

    let query = BeliefQuery::new(fixture.belief.as_ref());
    for subject in &fixture.subjects {
        let key = configured_belief_key(
            &fixture.revision,
            subject,
            &default_perspective(),
            &BranchScope::main(),
        );
        assert_eq!(query.revision_history(&key).unwrap().len(), 1);
    }
}

#[test]
fn actor_resumes_after_reopen_without_double_assessment() {
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_path = belief_dir.path().join("belief");
    let subjects: Vec<DomainObjectRef>;
    let revision: BeliefFamilyRevision;
    {
        let belief_db = sled::open(&belief_path).unwrap();
        let belief = Arc::new(BeliefStore::new(belief_db.clone()).unwrap());
        let mut registry = BeliefFamilyRegistryStore::new(belief_db).unwrap();
        let (_, installed) = registry.install(family_config("1"), 1).unwrap();
        revision = installed;
        subjects = ["node-a", "node-b", "node-c"]
            .iter()
            .enumerate()
            .map(|(index, id)| seed_subject(belief.as_ref(), id, index as u64 + 1))
            .collect();
        let mut actor = BeliefAssessmentActor::new(
            "belief.assessment.test",
            Arc::clone(&belief),
            Arc::new(registry),
            vec![FAMILY_ID.to_string()],
            subjects.iter().map(binding).collect(),
            default_perspective(),
            BranchScope::main(),
        );

        let report = actor.bounded_step(&BeliefAssessmentRequest {
            sequence: 10,
            max_items: 2,
        });
        assert_eq!(report.items_committed, 2);
        assert!(report.budget_exhausted);
    }

    let belief_db = reopen_sled_after_close(&belief_path).unwrap();
    let belief = Arc::new(BeliefStore::new(belief_db.clone()).unwrap());
    let registry = BeliefFamilyRegistryStore::new(belief_db).unwrap();
    let mut actor = BeliefAssessmentActor::new(
        "belief.assessment.test",
        Arc::clone(&belief),
        Arc::new(registry),
        vec![FAMILY_ID.to_string()],
        subjects.iter().map(binding).collect(),
        default_perspective(),
        BranchScope::main(),
    );

    let resumed = actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 11,
        max_items: 2,
    });
    let quiescent = actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 12,
        max_items: 2,
    });

    // Reopen resumes with the durable checkpoint and only the remaining key.
    assert_eq!(resumed.input_checkpoint, 10);
    assert_eq!(resumed.items_attempted, 1);
    assert_eq!(resumed.items_committed, 1);
    assert_eq!(quiescent.items_attempted, 0);
    assert_eq!(quiescent.items_committed, 0);

    let query = BeliefQuery::new(belief.as_ref());
    for subject in &subjects {
        let key = configured_belief_key(
            &revision,
            subject,
            &default_perspective(),
            &BranchScope::main(),
        );
        assert_eq!(query.revision_history(&key).unwrap().len(), 1);
    }
}

#[test]
fn actor_rejects_zero_budget_without_touching_state() {
    let fixture = fixture(&["node-a"]);
    let mut actor = actor(&fixture);

    let report = actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 10,
        max_items: 0,
    });

    assert_eq!(report.items_attempted, 0);
    assert_eq!(report.fatal_errors.len(), 1);
    assert_eq!(report.fatal_errors[0].code, "invalid_budget");
    assert_eq!(report.output_checkpoint, 0);
}

#[test]
fn second_evidence_revision_flows_to_planner_projection_with_theory_ref() {
    let fixture = fixture(&["node-a"]);
    let mut actor = actor(&fixture);
    let key = configured_belief_key(
        &fixture.revision,
        &fixture.subjects[0],
        &default_perspective(),
        &BranchScope::main(),
    );

    let initial = actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 10,
        max_items: 4,
    });
    assert_eq!(initial.items_committed, 1);
    let query = BeliefQuery::new(fixture.belief.as_ref());
    let (first_revision, first_view) = query.current_revision_and_view(&key).unwrap().unwrap();
    let first_projection =
        support::project_belief(fixture.belief.as_ref(), fixture.graph.as_ref(), &key);

    // Second distinct evidence arrives through the durable assignment path
    // and marks the exact key dirty.
    let normalizer = BeliefEvidenceNormalizer::new(
        fixture.revision.config.clone(),
        default_perspective(),
        BranchScope::main(),
    );
    let promoted = promoted_record(fixture.subjects[0].clone(), 5);
    for item in normalizer.normalize_promoted(&promoted).unwrap() {
        fixture.belief.put_evidence_once(&item).unwrap();
        fixture
            .belief
            .put_assignment_once(&normalizer.assign(&item).unwrap())
            .unwrap();
    }

    let (retained, pending) = query.current_revision_and_view(&key).unwrap().unwrap();
    assert_eq!(retained, first_revision);
    assert_eq!(pending.current_revision_id, first_view.current_revision_id);
    assert_eq!(
        pending.status,
        meld_world_model::BeliefStatus::AssessmentPending
    );
    assert!(pending.freshness.stale);
    assert_eq!(
        fixture.belief.current_view(&key).unwrap().unwrap(),
        first_view
    );
    let pending_projection =
        support::project_belief(fixture.belief.as_ref(), fixture.graph.as_ref(), &key);
    assert!(!pending_projection.world_state.propositions().iter().any(|proposition|
        matches!(proposition, meld_lang::Proposition::Holds { dimension: meld_lang::Term::Dimension(dimension), .. }
            if dimension == &key.dimension_id)
    ));

    let reassessed = actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 11,
        max_items: 4,
    });
    assert_eq!(reassessed.items_attempted, 1);
    assert_eq!(reassessed.items_committed, 1);

    let (second_revision, second_view) = query.current_revision_and_view(&key).unwrap().unwrap();
    let second_projection =
        support::project_belief(fixture.belief.as_ref(), fixture.graph.as_ref(), &key);

    // Distinct evidence produced a distinct revision linked to the first.
    assert_ne!(second_revision.revision_id, first_revision.revision_id);
    assert_eq!(
        second_revision.prior_revision_id,
        Some(first_revision.revision_id.clone())
    );
    // First evidence stays below the planner threshold, the second crosses it.
    assert!(first_view.planner_projection.confidence < 0.7);
    assert!(second_view.planner_projection.confidence > 0.7);

    // Each projection cites exactly the revision identity the store held.
    assert!(first_projection
        .source_refs
        .contains(&PlannerSourceRef::BeliefRevision {
            revision_id: first_revision.revision_id.clone(),
        }));
    assert!(second_projection
        .source_refs
        .contains(&PlannerSourceRef::BeliefRevision {
            revision_id: second_revision.revision_id.clone(),
        }));
    assert!(!second_projection
        .source_refs
        .contains(&PlannerSourceRef::BeliefRevision {
            revision_id: first_revision.revision_id.clone(),
        }));
    assert_ne!(second_projection.source_refs, first_projection.source_refs);

    // Theory lineage is durable end to end: revision, view, and projection
    // all cite the installed registry revision.
    let expected_ref = fixture.revision.revision_ref();
    assert_eq!(second_revision.theory_revision, Some(expected_ref.clone()));
    assert_eq!(second_view.theory_revision, Some(expected_ref.clone()));
    assert_eq!(first_projection.theory_revision, Some(expected_ref.clone()));
    assert_eq!(
        second_projection.theory_revision,
        Some(expected_ref.clone())
    );
    assert_eq!(
        fixture
            .registry
            .resolve(&expected_ref.id, &expected_ref.content_hash)
            .unwrap()
            .unwrap()
            .content_hash,
        expected_ref.content_hash
    );
}

fn lease(
    key: &BeliefKey,
    lease_id: &str,
    status: LeaseStatus,
    expires_at_seq: u64,
) -> AssessmentLease {
    AssessmentLease {
        lease_id: lease_id.to_string(),
        belief_key: key.clone(),
        epoch: 1,
        owner_id: "worker-test".to_string(),
        input_cursor_start: 1,
        input_cursor_end: 1,
        started_at_seq: 1,
        expires_at_seq,
        comparator_engine_id: "weighted_bayesian".to_string(),
        config_snapshot_hash: "hash".to_string(),
        status,
    }
}

#[test]
fn lease_recovery_reads_only_the_active_lease_index() {
    let fixture = fixture(&["node-a"]);
    let key = configured_belief_key(
        &fixture.revision,
        &fixture.subjects[0],
        &default_perspective(),
        &BranchScope::main(),
    );

    // A deep audit history of completed leases must stay untouched.
    for index in 0..50 {
        fixture
            .belief
            .put_lease(&lease(
                &key,
                &format!("lease-done-{index}"),
                LeaseStatus::Completed,
                5,
            ))
            .unwrap();
    }
    // A historical Leased-status record with no active-index entry is audit
    // data, not in-flight work; index-driven recovery must not resurrect it.
    fixture
        .belief
        .put_lease(&lease(&key, "lease-ghost", LeaseStatus::Leased, 5))
        .unwrap();
    let active = fixture
        .belief
        .acquire_lease(lease(&key, "lease-live", LeaseStatus::Queued, 5))
        .unwrap();

    let recovered = fixture.belief.recover_expired_leases(100).unwrap();

    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].lease_id, active.lease_id);
    assert_eq!(
        fixture
            .belief
            .get_lease(&active.lease_id)
            .unwrap()
            .unwrap()
            .status,
        LeaseStatus::Abandoned
    );
    assert_eq!(
        fixture
            .belief
            .get_lease("lease-ghost")
            .unwrap()
            .unwrap()
            .status,
        LeaseStatus::Leased
    );
    assert_eq!(
        fixture
            .belief
            .get_lease("lease-done-7")
            .unwrap()
            .unwrap()
            .status,
        LeaseStatus::Completed
    );
    assert!(fixture.belief.dirty_state(&key).unwrap().is_some());
    // A second pass over the now-empty active index recovers nothing.
    assert!(fixture
        .belief
        .recover_expired_leases(100)
        .unwrap()
        .is_empty());
}

#[test]
fn absorbed_dirty_window_clears_without_a_new_revision() {
    let fixture = fixture(&["node-a"]);
    let mut actor = actor(&fixture);
    let key = configured_belief_key(
        &fixture.revision,
        &fixture.subjects[0],
        &default_perspective(),
        &BranchScope::main(),
    );

    let initial = actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 10,
        max_items: 2,
    });
    // Replay marks the key dirty at a sequence the committed revision
    // already covers.
    fixture.belief.mark_dirty(&key, 1).unwrap();
    let absorbed = actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 11,
        max_items: 2,
    });
    let quiescent = actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 12,
        max_items: 2,
    });

    assert_eq!(initial.items_committed, 1);
    assert_eq!(absorbed.items_attempted, 1);
    assert_eq!(absorbed.items_committed, 0);
    assert!(absorbed.retryable_errors.is_empty());
    assert!(fixture.belief.dirty_state(&key).unwrap().is_none());
    assert_eq!(quiescent.items_attempted, 0);
    assert_eq!(
        BeliefQuery::new(fixture.belief.as_ref())
            .revision_history(&key)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn stored_records_without_theory_revision_still_load() {
    let fixture = fixture(&["node-a"]);
    let mut actor = actor(&fixture);
    actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 10,
        max_items: 1,
    });
    let key = configured_belief_key(
        &fixture.revision,
        &fixture.subjects[0],
        &default_perspective(),
        &BranchScope::main(),
    );
    let (revision, view) = BeliefQuery::new(fixture.belief.as_ref())
        .current_revision_and_view(&key)
        .unwrap()
        .unwrap();

    // Simulate records persisted before the lineage field existed.
    let mut revision_json = serde_json::to_value(&revision).unwrap();
    revision_json
        .as_object_mut()
        .unwrap()
        .remove("theory_revision");
    let mut view_json = serde_json::to_value(&view).unwrap();
    view_json.as_object_mut().unwrap().remove("theory_revision");

    let legacy_revision: meld_world_model::belief::BeliefRevision =
        serde_json::from_value(revision_json).unwrap();
    let legacy_view: meld_world_model::BeliefView = serde_json::from_value(view_json).unwrap();

    assert_eq!(legacy_revision.theory_revision, None);
    assert_eq!(legacy_view.theory_revision, None);
    assert_eq!(legacy_revision.revision_id, revision.revision_id);

    // Planner projection outputs recorded before the lineage field existed
    // must also load, with the field defaulting to None.
    let projection = support::project_belief(fixture.belief.as_ref(), fixture.graph.as_ref(), &key);
    let mut projection_json = serde_json::to_value(&projection).unwrap();
    projection_json
        .as_object_mut()
        .unwrap()
        .remove("theory_revision");
    let legacy_projection: meld_world_model::WorldModelView =
        serde_json::from_value(projection_json).unwrap();
    assert_eq!(legacy_projection.theory_revision, None);
    assert_eq!(legacy_projection.source_refs, projection.source_refs);
}

/// An unanchored family assesses an unobserved subject to its prior-based
/// revision: no graph anchor exists, none is consulted, no evidence is
/// cited, and the source cursor window is pinned to zero so the first
/// promoted evidence re-dirties the key.
#[test]
fn unanchored_family_assesses_unobserved_subject_to_prior_revision() {
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_db = sled::open(belief_dir.path().join("belief")).unwrap();
    let belief = Arc::new(BeliefStore::new(belief_db.clone()).unwrap());
    let mut registry = BeliefFamilyRegistryStore::new(belief_db).unwrap();
    let mut config = family_config("1");
    config.initial_assessment = meld_world_model::belief::InitialAssessmentPolicy::PriorAllowed;
    let (_, revision) = registry.install(config, 1).unwrap();
    // The subject is never seeded: no fact, no anchor, nothing observed.
    let subject = object("workspace_fs", "node", "unobserved-node");

    let mut actor = BeliefAssessmentActor::new(
        "belief.assessment.test",
        Arc::clone(&belief),
        Arc::new(registry.clone()),
        vec![FAMILY_ID.to_string()],
        vec![binding(&subject)],
        default_perspective(),
        BranchScope::main(),
    );

    let report = actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 10,
        max_items: 4,
    });

    assert_eq!(report.items_attempted, 1);
    assert_eq!(report.items_committed, 1);
    assert!(report.retryable_errors.is_empty(), "{report:?}");
    assert!(report.fatal_errors.is_empty(), "{report:?}");
    assert!(
        !report
            .waiting_on
            .iter()
            .any(|declaration| declaration.condition == "graph_anchor_absent"),
        "{report:?}"
    );

    let key = configured_belief_key(
        &revision,
        &subject,
        &default_perspective(),
        &BranchScope::main(),
    );
    let query = BeliefQuery::new(belief.as_ref());
    let history = query.revision_history(&key).unwrap();
    assert_eq!(history.len(), 1);
    let committed = &history[0];
    assert!(committed.evidence_ids.is_empty());
    assert_eq!(committed.source_cursor_start, 0);
    assert_eq!(committed.source_cursor_end, 0);
    // Prior-based posture: the posterior stays at the family prior and the
    // required aggregate schema is truthfully missing.
    assert_eq!(committed.posterior.probability, 0.8);
    assert!(committed.observation.is_some());
    assert_eq!(
        committed.theory_revision.as_ref().map(|r| r.id.as_str()),
        Some(FAMILY_ID)
    );

    // Quiescence: the committed revision keeps the key out of selection.
    let second = actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 11,
        max_items: 4,
    });
    assert_eq!(second.items_attempted, 0);
    assert_eq!(second.items_committed, 0);
}

/// The family's anchor declaration decides planner scope semantics: an
/// unanchored family's maintained scope is declared accessible, so the
/// projection carries the Accessible proposition without any anchor.
#[test]
fn prior_only_belief_does_not_fabricate_graph_accessibility() {
    let graph_dir = tempfile::tempdir().unwrap();
    let belief_dir = tempfile::tempdir().unwrap();
    let graph =
        Arc::new(TraversalStore::new(sled::open(graph_dir.path().join("graph")).unwrap()).unwrap());
    let belief_db = sled::open(belief_dir.path().join("belief")).unwrap();
    let belief = Arc::new(BeliefStore::new(belief_db.clone()).unwrap());
    let mut registry = BeliefFamilyRegistryStore::new(belief_db).unwrap();
    let mut config = family_config("1");
    config.initial_assessment = meld_world_model::belief::InitialAssessmentPolicy::PriorAllowed;
    let (_, revision) = registry.install(config, 1).unwrap();
    let subject = object("workspace_fs", "node", "unobserved-node");
    let key = configured_belief_key(
        &revision,
        &subject,
        &default_perspective(),
        &BranchScope::main(),
    );

    // Commit the prior-based revision so the projection carries a belief.
    let mut actor = BeliefAssessmentActor::new(
        "belief.assessment.test",
        Arc::clone(&belief),
        Arc::new(registry.clone()),
        vec![FAMILY_ID.to_string()],
        vec![binding(&subject)],
        default_perspective(),
        BranchScope::main(),
    );
    actor.bounded_step(&BeliefAssessmentRequest {
        sequence: 10,
        max_items: 4,
    });

    let projected = support::project_belief(belief.as_ref(), graph.as_ref(), &key);
    assert!(!projected
        .world_state
        .propositions()
        .iter()
        .any(|proposition| matches!(proposition, meld_lang::Proposition::Accessible { .. })));
}

#[test]
fn evidence_required_waiters_do_not_starve_admitted_evidence_at_budget_one() {
    use meld_world_model::agent::AgentSubscriptionRequestV1;
    use meld_world_model::belief::BeliefSubscriptionAuthority;
    for subscriptions_only in [false, true] {
        let mut fixture = fixture(&["z-ready"]);
        let ready = fixture.subjects[0].clone();
        let empty = object("workspace_fs", "node", "a-empty");
        fixture.subjects.insert(0, empty.clone());
        if subscriptions_only {
            for subject in &fixture.subjects {
                let request = AgentSubscriptionRequestV1::new(
                    "observer".into(),
                    "belief".into(),
                    fixture.revision.revision_ref(),
                    configured_belief_key(
                        &fixture.revision,
                        subject,
                        &default_perspective(),
                        &BranchScope::main(),
                    ),
                    "from_genesis".into(),
                )
                .unwrap();
                BeliefSubscriptionAuthority::new(&fixture.belief)
                    .accept(&request, &fixture.revision)
                    .unwrap();
            }
            fixture.subjects.clear();
        }
        let mut actor = actor(&fixture);
        let first = actor.bounded_step(&BeliefAssessmentRequest {
            sequence: 10,
            max_items: 1,
        });
        assert_eq!(first.items_committed, 1, "{first:?}");
        let query = BeliefQuery::new(&fixture.belief);
        assert!(query
            .current_revision(&configured_belief_key(
                &fixture.revision,
                &ready,
                &default_perspective(),
                &BranchScope::main()
            ))
            .unwrap()
            .is_some());
        assert!(query
            .current_revision(&configured_belief_key(
                &fixture.revision,
                &empty,
                &default_perspective(),
                &BranchScope::main()
            ))
            .unwrap()
            .is_none());
        let quiet = actor.bounded_step(&BeliefAssessmentRequest {
            sequence: 11,
            max_items: 1,
        });
        assert_eq!(quiet.items_attempted, 0);
        assert!(!quiet.budget_exhausted);
    }
}
