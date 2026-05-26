use std::sync::Arc;

use meld_world_model::belief::EvidenceRejection;
use meld_world_model::belief::{
    BayesianComparator, BeliefConfigLoader, BeliefEvidenceNormalizer, BeliefQuery, BeliefRuntime,
    BeliefStore, BranchScope, ComparatorInput, LeaseStatus,
};
use meld_world_model::belief::{
    BeliefProvenanceSummary, BeliefRevision, ContradictionState, EvidencePolarity, FreshnessState,
    HydrationRefs, ObservationOpportunity, PlannerProjectionSummary, PosteriorSummary,
};
use meld_world_model::events::{DomainObjectRef, EventRelation};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::{
    AnchorSelectionRecord, BeliefStatus, EvidenceValue, PerspectiveKey, TraversalFactRecord,
};
use proptest::prelude::*;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.000_001,
        "expected {expected}, got {actual}"
    );
}

fn object(domain_id: &str, object_kind: &str, object_id: &str) -> DomainObjectRef {
    DomainObjectRef::new(domain_id, object_kind, object_id).unwrap()
}

fn config_json() -> &'static str {
    r#"{
        "family_id": "docs_freshness",
        "dimension_id": "docs_freshness",
        "predicate_id": "confidence",
        "evidence_policy_id": "default_policy",
        "evidence_schemas": [
            {
                "schema_id": "graph_anchor_signal",
                "required": true,
                "role": "Support",
                "reliability": 1.0,
                "precision": 1.0
            }
        ],
        "source_mappings": [
            {
                "mapping_id": "anchor_to_signal",
                "source_kind": "graph_anchor",
                "evidence_schema_id": "graph_anchor_signal",
                "subject_from": "anchor.subject",
                "value_field": "ended",
                "factor_id": "freshness_signal"
            }
        ],
        "comparator": {
            "engine_id": "weighted_bayesian",
            "engine_version": "1",
            "factors": [
                {
                    "factor_id": "freshness_signal",
                    "evidence_schema_id": "graph_anchor_signal",
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

fn seeded_graph() -> (tempfile::TempDir, Arc<TraversalStore>, DomainObjectRef) {
    let temp_dir = tempfile::tempdir().unwrap();
    let store =
        Arc::new(TraversalStore::new(sled::open(temp_dir.path().join("graph")).unwrap()).unwrap());
    let node = object("workspace_fs", "node", "node-a");
    let frame = object("context", "frame", "frame-a");
    let anchor_ref = object("context", "head", "node-a::analysis");
    let relation = EventRelation::new("selected", node.clone(), frame.clone()).unwrap();
    let fact = TraversalFactRecord {
        fact_id: "fact-a".to_string(),
        source_spine_fact_id: "spine-a".to_string(),
        seq: 1,
        event_type: "context.head.selected".to_string(),
        objects: vec![node.clone(), frame.clone()],
        relations: vec![relation],
    };
    let anchor = AnchorSelectionRecord {
        anchor_id: "anchor-a".to_string(),
        anchor_ref,
        subject: node.clone(),
        perspective: PerspectiveKey::new("frame_type", "analysis").unwrap(),
        target: frame,
        source_fact_ids: vec!["spine-a".to_string()],
        created_by_fact_id: "fact-a".to_string(),
        selected_at_seq: 1,
        ended_at_seq: None,
        ended_by_anchor_id: None,
        ended_by_fact_id: None,
    };
    store.put_fact(&fact).unwrap();
    store.put_anchor(&anchor).unwrap();
    store.set_current_anchor(&anchor).unwrap();
    (temp_dir, store, node)
}

#[test]
fn belief_config_loads_runtime_family_and_hashes() {
    let snapshot = BeliefConfigLoader::load_json(config_json()).unwrap();
    assert_eq!(snapshot.config.family_id, "docs_freshness");
    assert_eq!(snapshot.config.comparator.engine_id, "weighted_bayesian");
    assert_eq!(snapshot.hash, "e73f2f84e30146b3");
    assert!(!snapshot.hash.is_empty());
}

#[test]
fn belief_config_rejects_invalid_probability() {
    let invalid = config_json().replace("\"default_prior\": 0.8", "\"default_prior\": 1.8");
    assert!(BeliefConfigLoader::load_json(&invalid).is_err());
}

#[test]
fn belief_config_rejects_duplicate_schema_ids() {
    let duplicate = config_json().replace(
        "\"evidence_schemas\": [",
        "\"evidence_schemas\": [
            {
                \"schema_id\": \"graph_anchor_signal\",
                \"required\": true,
                \"role\": \"Support\",
                \"reliability\": 1.0,
                \"precision\": 1.0
            },",
    );

    assert!(BeliefConfigLoader::load_json(&duplicate).is_err());
}

#[test]
fn belief_config_rejects_invalid_factor_weight() {
    let invalid = config_json().replace("\"weight\": 1.0", "\"weight\": -1.0");

    assert!(BeliefConfigLoader::load_json(&invalid).is_err());
}

#[test]
fn belief_config_accepts_zero_factor_weight() {
    let valid = config_json().replace("\"weight\": 1.0", "\"weight\": 0.0");

    assert!(BeliefConfigLoader::load_json(&valid).is_ok());
}

#[test]
fn belief_config_hash_changes_with_runtime_content() {
    let first = BeliefConfigLoader::load_json(config_json()).unwrap();
    let changed = config_json().replace("\"config_version\": \"1\"", "\"config_version\": \"2\"");
    let second = BeliefConfigLoader::load_json(&changed).unwrap();

    assert_ne!(first.hash, second.hash);
}

#[test]
fn belief_evidence_normalizes_anchor_and_preserves_provenance() {
    let (_temp_dir, graph, node) = seeded_graph();
    let snapshot = BeliefConfigLoader::load_json(config_json()).unwrap();
    let query = meld_world_model::TraversalQuery::new(graph.as_ref());
    let anchor = query
        .current_frame_head(&node, "analysis")
        .unwrap()
        .unwrap();
    let provenance = query.provenance_for_anchor(&anchor.anchor_id).unwrap();
    let normalizer = BeliefEvidenceNormalizer::new(
        snapshot.config,
        PerspectiveKey::new("default", "default").unwrap(),
        BranchScope::main(),
    );

    let evidence = normalizer.normalize_anchor(&anchor, &provenance).unwrap();
    let item = evidence.first().unwrap();

    assert_eq!(item.candidate_key.subject, node);
    assert!(item.source_fact_ids.contains(&"spine-a".to_string()));
    assert!(item.source_fact_ids.contains(&"fact-a".to_string()));
    assert_eq!(item.graph_anchor_ids, vec!["anchor-a"]);
    assert_eq!(item.provenance.objects.len(), 2);
    assert_eq!(item.typed_value, EvidenceValue::Scalar(0.0));
}

#[test]
fn belief_assignment_uses_explicit_key_fields() {
    let (_temp_dir, graph, node) = seeded_graph();
    let snapshot = BeliefConfigLoader::load_json(config_json()).unwrap();
    let query = meld_world_model::TraversalQuery::new(graph.as_ref());
    let anchor = query
        .current_frame_head(&node, "analysis")
        .unwrap()
        .unwrap();
    let provenance = query.provenance_for_anchor(&anchor.anchor_id).unwrap();
    let normalizer = BeliefEvidenceNormalizer::new(
        snapshot.config,
        PerspectiveKey::new("default", "default").unwrap(),
        BranchScope::main(),
    );
    let item = normalizer
        .normalize_anchor(&anchor, &provenance)
        .unwrap()
        .remove(0);

    let assignment = normalizer.assign(&item).unwrap();

    assert_eq!(assignment.belief_key.dimension_id, "docs_freshness");
    assert_eq!(assignment.belief_key.predicate_id, "confidence");
    assert_eq!(assignment.belief_key.perspective.perspective_id, "default");
    assert_eq!(assignment.belief_key.branch_scope.branch_id, "main");
}

#[test]
fn configured_bayesian_comparator_is_deterministic() {
    let (_temp_dir, graph, node) = seeded_graph();
    let snapshot = BeliefConfigLoader::load_json(config_json()).unwrap();
    let query = meld_world_model::TraversalQuery::new(graph.as_ref());
    let anchor = query
        .current_frame_head(&node, "analysis")
        .unwrap()
        .unwrap();
    let provenance = query.provenance_for_anchor(&anchor.anchor_id).unwrap();
    let normalizer = BeliefEvidenceNormalizer::new(
        snapshot.config.clone(),
        PerspectiveKey::new("default", "default").unwrap(),
        BranchScope::main(),
    );
    let evidence = normalizer.normalize_anchor(&anchor, &provenance).unwrap();

    let first = BayesianComparator::assess(ComparatorInput {
        config: snapshot.config.clone(),
        config_snapshot_hash: snapshot.hash.clone(),
        prior_revision: None,
        evidence: evidence.clone(),
        source_cursor_start: 1,
        source_cursor_end: 1,
    })
    .unwrap();
    let second = BayesianComparator::assess(ComparatorInput {
        config: snapshot.config,
        config_snapshot_hash: snapshot.hash,
        prior_revision: None,
        evidence,
        source_cursor_start: 1,
        source_cursor_end: 1,
    })
    .unwrap();

    assert_eq!(first.revision, second.revision);
    assert_close(first.revision.posterior.probability, 0.4);
    assert_close(first.revision.confidence, 0.6);
    assert!(first.revision.confidence < 0.7);
}

#[test]
fn configured_bayesian_comparator_uses_weights_reliability_and_precision() {
    let (_temp_dir, graph, node) = seeded_graph();
    let mut snapshot = BeliefConfigLoader::load_json(config_json()).unwrap();
    snapshot.config.evidence_schemas[0].reliability = 0.5;
    snapshot.config.evidence_schemas[0].precision = 0.5;
    snapshot.config.comparator.factors[0].weight = 0.5;
    let query = meld_world_model::TraversalQuery::new(graph.as_ref());
    let anchor = query
        .current_frame_head(&node, "analysis")
        .unwrap()
        .unwrap();
    let provenance = query.provenance_for_anchor(&anchor.anchor_id).unwrap();
    let normalizer = BeliefEvidenceNormalizer::new(
        snapshot.config.clone(),
        PerspectiveKey::new("default", "default").unwrap(),
        BranchScope::main(),
    );
    let mut evidence = normalizer.normalize_anchor(&anchor, &provenance).unwrap();
    evidence[0].typed_value = EvidenceValue::Scalar(0.25);

    let output = BayesianComparator::assess(ComparatorInput {
        config: snapshot.config,
        config_snapshot_hash: snapshot.hash,
        prior_revision: None,
        evidence,
        source_cursor_start: 1,
        source_cursor_end: 1,
    })
    .unwrap();

    assert_close(output.revision.posterior.probability, 0.525);
    assert_close(output.revision.confidence, 0.475);
    assert_close(output.revision.uncertainty, 0.875);
    assert_close(output.revision.precision, 0.5);
}

#[test]
fn comparator_provenance_merges_duplicate_refs_once() {
    let (_temp_dir, graph, node) = seeded_graph();
    let snapshot = BeliefConfigLoader::load_json(config_json()).unwrap();
    let query = meld_world_model::TraversalQuery::new(graph.as_ref());
    let anchor = query
        .current_frame_head(&node, "analysis")
        .unwrap()
        .unwrap();
    let provenance = query.provenance_for_anchor(&anchor.anchor_id).unwrap();
    let normalizer = BeliefEvidenceNormalizer::new(
        snapshot.config.clone(),
        PerspectiveKey::new("default", "default").unwrap(),
        BranchScope::main(),
    );
    let first = normalizer
        .normalize_anchor(&anchor, &provenance)
        .unwrap()
        .remove(0);
    let mut second = first.clone();
    second.evidence_id = "second-evidence".to_string();

    let output = BayesianComparator::assess(ComparatorInput {
        config: snapshot.config,
        config_snapshot_hash: snapshot.hash,
        prior_revision: None,
        evidence: vec![first, second],
        source_cursor_start: 1,
        source_cursor_end: 1,
    })
    .unwrap();

    assert_eq!(output.revision.provenance.source_fact_ids.len(), 2);
    assert_eq!(output.revision.provenance.graph_anchor_ids.len(), 1);
    assert_eq!(output.revision.provenance.objects.len(), 2);
    assert_eq!(output.revision.provenance.relations.len(), 1);
}

#[test]
fn configured_bayesian_comparator_reports_missing_required_evidence() {
    let (_temp_dir, graph, node) = seeded_graph();
    let snapshot = BeliefConfigLoader::load_json(config_json()).unwrap();
    let query = meld_world_model::TraversalQuery::new(graph.as_ref());
    let anchor = query
        .current_frame_head(&node, "analysis")
        .unwrap()
        .unwrap();
    let provenance = query.provenance_for_anchor(&anchor.anchor_id).unwrap();
    let normalizer = BeliefEvidenceNormalizer::new(
        snapshot.config.clone(),
        PerspectiveKey::new("default", "default").unwrap(),
        BranchScope::main(),
    );
    let mut evidence = normalizer.normalize_anchor(&anchor, &provenance).unwrap();
    evidence[0].evidence_schema_id = "other_schema".to_string();

    let output = BayesianComparator::assess(ComparatorInput {
        config: snapshot.config,
        config_snapshot_hash: snapshot.hash,
        prior_revision: None,
        evidence,
        source_cursor_start: 1,
        source_cursor_end: 1,
    })
    .unwrap();

    assert_eq!(output.revision.status, BeliefStatus::NeedsObservation);
    assert_eq!(
        output
            .revision
            .observation
            .as_ref()
            .unwrap()
            .target_evidence_schema_id,
        "graph_anchor_signal"
    );
}

#[test]
fn configured_bayesian_comparator_reports_missing_assessment() {
    let (_temp_dir, graph, node) = seeded_graph();
    let mut snapshot = BeliefConfigLoader::load_json(config_json()).unwrap();
    let query = meld_world_model::TraversalQuery::new(graph.as_ref());
    let anchor = query
        .current_frame_head(&node, "analysis")
        .unwrap()
        .unwrap();
    let provenance = query.provenance_for_anchor(&anchor.anchor_id).unwrap();
    let normalizer = BeliefEvidenceNormalizer::new(
        snapshot.config.clone(),
        PerspectiveKey::new("default", "default").unwrap(),
        BranchScope::main(),
    );
    let evidence = normalizer.normalize_anchor(&anchor, &provenance).unwrap();
    snapshot.config.comparator.engine_id = "unknown_engine".to_string();

    let output = BayesianComparator::assess(ComparatorInput {
        config: snapshot.config,
        config_snapshot_hash: snapshot.hash,
        prior_revision: None,
        evidence,
        source_cursor_start: 1,
        source_cursor_end: 1,
    })
    .unwrap();

    assert_eq!(output.revision.status, BeliefStatus::NeedsAssessment);
    assert_eq!(output.revision.uncertainty, 1.0);
}

#[test]
fn configured_bayesian_comparator_uses_prior_when_no_factor_matches() {
    let (_temp_dir, graph, node) = seeded_graph();
    let mut snapshot = BeliefConfigLoader::load_json(config_json()).unwrap();
    snapshot.config.comparator.factors[0].evidence_schema_id = "other_schema".to_string();
    let query = meld_world_model::TraversalQuery::new(graph.as_ref());
    let anchor = query
        .current_frame_head(&node, "analysis")
        .unwrap()
        .unwrap();
    let provenance = query.provenance_for_anchor(&anchor.anchor_id).unwrap();
    let normalizer = BeliefEvidenceNormalizer::new(
        snapshot.config.clone(),
        PerspectiveKey::new("default", "default").unwrap(),
        BranchScope::main(),
    );
    let evidence = normalizer.normalize_anchor(&anchor, &provenance).unwrap();

    let output = BayesianComparator::assess(ComparatorInput {
        config: snapshot.config,
        config_snapshot_hash: snapshot.hash,
        prior_revision: None,
        evidence,
        source_cursor_start: 1,
        source_cursor_end: 1,
    })
    .unwrap();

    assert!(output.revision.posterior.probability.is_finite());
    assert_close(output.revision.posterior.probability, 0.8);
    assert_close(output.revision.confidence, 0.2);
}

#[test]
fn comparator_polarity_keeps_counterevidence_separate() {
    let (_temp_dir, graph, node) = seeded_graph();
    let mut snapshot = BeliefConfigLoader::load_json(config_json()).unwrap();
    snapshot.config.comparator.factors[0].polarity = EvidencePolarity::Contradicts;
    let query = meld_world_model::TraversalQuery::new(graph.as_ref());
    let anchor = query
        .current_frame_head(&node, "analysis")
        .unwrap()
        .unwrap();
    let provenance = query.provenance_for_anchor(&anchor.anchor_id).unwrap();
    let normalizer = BeliefEvidenceNormalizer::new(
        snapshot.config.clone(),
        PerspectiveKey::new("default", "default").unwrap(),
        BranchScope::main(),
    );
    let mut evidence = normalizer.normalize_anchor(&anchor, &provenance).unwrap();
    evidence[0].typed_value = EvidenceValue::Scalar(0.25);

    let output = BayesianComparator::assess(ComparatorInput {
        config: snapshot.config,
        config_snapshot_hash: snapshot.hash,
        prior_revision: None,
        evidence,
        source_cursor_start: 1,
        source_cursor_end: 1,
    })
    .unwrap();

    assert!(output.revision.contradiction.contradicted);
    assert_eq!(
        output.revision.contradiction.reason.as_deref(),
        Some("contradicting evidence present")
    );
    assert_eq!(output.revision.supporting_evidence_ids.len(), 0);
    assert_eq!(output.revision.contradicted_evidence_ids.len(), 1);
    assert_close(output.revision.posterior.probability, 0.775);
    assert_close(output.revision.confidence, 0.225);
}

#[test]
fn belief_runtime_persists_revision_and_view() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph, config_json()).unwrap();

    let result = runtime
        .assess_subject(&node, "frame_type", "analysis", "worker-a")
        .unwrap();

    let query = BeliefQuery::new(belief_store.as_ref());
    let views = query
        .current_views_for_subject(&node, &PerspectiveKey::new("default", "default").unwrap())
        .unwrap();

    assert_eq!(result.evidence_count, 1);
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].status, BeliefStatus::Settled);
    assert!(views[0].confidence < 0.7);
    assert_eq!(views[0].hydration.evidence_ids.len(), 1);
    assert_eq!(
        query
            .evidence_by_revision(&result.revision_id)
            .unwrap()
            .len(),
        1
    );
    assert!(query
        .provenance_by_revision(&result.revision_id)
        .unwrap()
        .unwrap()
        .source_fact_ids
        .contains(&"spine-a".to_string()));
    assert!(belief_store
        .current_revision(&views[0].key)
        .unwrap()
        .is_some());
}

#[test]
fn belief_store_persists_rejections_and_config_snapshots() {
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap();
    let rejection = EvidenceRejection {
        rejection_id: "rejection-a".to_string(),
        source_id: "source-a".to_string(),
        reason: "unsupported".to_string(),
        source_cursor_start: 1,
        source_cursor_end: 1,
    };

    belief_store.put_rejection(&rejection).unwrap();
    belief_store
        .put_config_snapshot("config-a", "{\"ok\":true}")
        .unwrap();

    assert_eq!(
        belief_store
            .get_rejection(&rejection.rejection_id)
            .unwrap()
            .unwrap(),
        rejection
    );
    assert_eq!(
        belief_store
            .get_config_snapshot("config-a")
            .unwrap()
            .as_deref(),
        Some("{\"ok\":true}")
    );
}

#[test]
fn belief_store_reopens_current_view_and_revision_history() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    {
        let belief_store = Arc::new(
            BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap(),
        );
        let runtime =
            BeliefRuntime::from_json_config(belief_store, graph.clone(), config_json()).unwrap();
        runtime
            .assess_subject(&node, "frame_type", "analysis", "worker-a")
            .unwrap();
    }

    let reopened = BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap();
    let query = BeliefQuery::new(&reopened);
    let views = query
        .current_views_for_subject(&node, &PerspectiveKey::new("default", "default").unwrap())
        .unwrap();
    let view = views.first().unwrap();

    assert_eq!(views.len(), 1);
    assert_eq!(query.revision_history(&view.key).unwrap().len(), 1);
    assert_eq!(
        query
            .evidence_by_revision(view.current_revision_id.as_ref().unwrap())
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn belief_store_marks_stale_when_newer_evidence_arrives() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph.clone(), config_json())
            .unwrap();
    runtime
        .assess_subject(&node, "frame_type", "analysis", "worker-a")
        .unwrap();

    let snapshot = BeliefConfigLoader::load_json(config_json()).unwrap();
    let query = meld_world_model::TraversalQuery::new(graph.as_ref());
    let anchor = query
        .current_frame_head(&node, "analysis")
        .unwrap()
        .unwrap();
    let provenance = query.provenance_for_anchor(&anchor.anchor_id).unwrap();
    let normalizer = BeliefEvidenceNormalizer::new(
        snapshot.config,
        PerspectiveKey::new("default", "default").unwrap(),
        BranchScope::main(),
    );
    let mut item = normalizer
        .normalize_anchor(&anchor, &provenance)
        .unwrap()
        .remove(0);
    item.evidence_id = "evidence-newer".to_string();
    item.source_cursor_end = 2;
    belief_store.put_evidence(&item).unwrap();
    belief_store
        .put_assignment(&normalizer.assign(&item).unwrap())
        .unwrap();
    let stale = belief_store
        .mark_stale_if_newer_evidence(&item.candidate_key)
        .unwrap()
        .unwrap();

    assert_eq!(stale.status, BeliefStatus::Stale);
    assert!(stale.freshness.stale);
}

#[test]
fn belief_store_keeps_view_current_for_equal_or_older_evidence() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph.clone(), config_json())
            .unwrap();
    runtime
        .assess_subject(&node, "frame_type", "analysis", "worker-a")
        .unwrap();

    let first = BeliefQuery::new(belief_store.as_ref())
        .current_views_for_subject(&node, &PerspectiveKey::new("default", "default").unwrap())
        .unwrap()
        .remove(0);
    let equal = belief_store
        .mark_stale_if_newer_evidence(&first.key)
        .unwrap()
        .unwrap();
    assert_eq!(equal.status, BeliefStatus::Settled);
    assert!(!equal.freshness.stale);

    let snapshot = BeliefConfigLoader::load_json(config_json()).unwrap();
    let query = meld_world_model::TraversalQuery::new(graph.as_ref());
    let anchor = query
        .current_frame_head(&node, "analysis")
        .unwrap()
        .unwrap();
    let provenance = query.provenance_for_anchor(&anchor.anchor_id).unwrap();
    let normalizer = BeliefEvidenceNormalizer::new(
        snapshot.config,
        PerspectiveKey::new("default", "default").unwrap(),
        BranchScope::main(),
    );
    let mut older = normalizer
        .normalize_anchor(&anchor, &provenance)
        .unwrap()
        .remove(0);
    older.evidence_id = "evidence-older".to_string();
    older.source_cursor_start = 0;
    older.source_cursor_end = 0;
    belief_store.put_evidence(&older).unwrap();
    belief_store
        .put_assignment(&normalizer.assign(&older).unwrap())
        .unwrap();

    let still_current = belief_store
        .mark_stale_if_newer_evidence(&first.key)
        .unwrap()
        .unwrap();
    assert_eq!(still_current.status, BeliefStatus::Settled);
    assert!(!still_current.freshness.stale);
}

#[test]
fn belief_recovery_abandons_expired_lease() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph, config_json()).unwrap();
    runtime
        .assess_subject(&node, "frame_type", "analysis", "worker-a")
        .unwrap();

    let query = BeliefQuery::new(belief_store.as_ref());
    let view = query
        .current_views_for_subject(&node, &PerspectiveKey::new("default", "default").unwrap())
        .unwrap()
        .remove(0);
    let lease = meld_world_model::AssessmentLease {
        lease_id: "lease-expired".to_string(),
        belief_key: view.key,
        epoch: 2,
        owner_id: "worker-b".to_string(),
        input_cursor_start: 1,
        input_cursor_end: 2,
        started_at_seq: 2,
        expires_at_seq: 3,
        comparator_engine_id: "weighted_bayesian".to_string(),
        config_snapshot_hash: "hash".to_string(),
        status: LeaseStatus::Queued,
    };
    belief_store.acquire_lease(lease).unwrap();

    assert_eq!(runtime.recover_expired_leases(4).unwrap(), 1);
    assert_eq!(query.dirty_keys().unwrap().len(), 1);
}

#[test]
fn belief_recovery_ignores_unexpired_leases() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph, config_json()).unwrap();
    runtime
        .assess_subject(&node, "frame_type", "analysis", "worker-a")
        .unwrap();
    let view = BeliefQuery::new(belief_store.as_ref())
        .current_views_for_subject(&node, &PerspectiveKey::new("default", "default").unwrap())
        .unwrap()
        .remove(0);
    belief_store
        .acquire_lease(test_lease(&view, "lease-future", 2, 10))
        .unwrap();

    assert_eq!(runtime.recover_expired_leases(4).unwrap(), 0);
    assert!(BeliefQuery::new(belief_store.as_ref())
        .dirty_keys()
        .unwrap()
        .is_empty());
}

#[test]
fn belief_store_clears_dirty_keys() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph, config_json()).unwrap();
    runtime
        .assess_subject(&node, "frame_type", "analysis", "worker-a")
        .unwrap();
    let view = BeliefQuery::new(belief_store.as_ref())
        .current_views_for_subject(&node, &PerspectiveKey::new("default", "default").unwrap())
        .unwrap()
        .remove(0);
    belief_store.mark_dirty(&view.key, 99).unwrap();

    assert_eq!(
        belief_store.dirty_keys().unwrap(),
        vec![view.key.index_key()]
    );

    belief_store.clear_dirty(&view.key).unwrap();

    assert!(belief_store.dirty_keys().unwrap().is_empty());
}

#[test]
fn belief_store_persists_runtime_meta() {
    let belief_dir = tempfile::tempdir().unwrap();
    let path = belief_dir.path().join("belief");
    {
        let belief_store = BeliefStore::new(sled::open(&path).unwrap()).unwrap();
        belief_store.put_runtime_meta("last_seq", "42").unwrap();
        belief_store.flush().unwrap();
    }

    let reopened = BeliefStore::new(sled::open(&path).unwrap()).unwrap();

    assert_eq!(
        reopened.get_runtime_meta("last_seq").unwrap().as_deref(),
        Some("42")
    );
}

#[test]
fn belief_store_returns_open_observation_opportunities() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph, config_json()).unwrap();
    runtime
        .assess_subject(&node, "frame_type", "analysis", "worker-a")
        .unwrap();
    let view = BeliefQuery::new(belief_store.as_ref())
        .current_views_for_subject(&node, &PerspectiveKey::new("default", "default").unwrap())
        .unwrap()
        .remove(0);
    let mut revision = test_revision(&view, 1, 1);
    revision.status = BeliefStatus::NeedsObservation;
    revision.observation = Some(ObservationOpportunity {
        opportunity_id: "observe-a".to_string(),
        belief_key: view.key.clone(),
        target_evidence_schema_id: "graph_anchor_signal".to_string(),
        reason: "missing anchor".to_string(),
        source_revision_id: Some(revision.revision_id.clone()),
        open: true,
    });
    let projected = belief_store.project_view(
        &revision,
        HydrationRefs {
            evidence_ids: Vec::new(),
            source_fact_ids: Vec::new(),
            graph_anchor_ids: Vec::new(),
            revision_id: Some(revision.revision_id.clone()),
        },
    );
    belief_store.put_view(&projected).unwrap();

    let opportunities = BeliefQuery::new(belief_store.as_ref())
        .open_observation_opportunities(&node, &PerspectiveKey::new("default", "default").unwrap())
        .unwrap();

    assert_eq!(opportunities.len(), 1);
    assert_eq!(opportunities[0].opportunity_id, "observe-a");
}

#[test]
fn belief_view_projection_uses_confidence_threshold_boundary() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph, config_json()).unwrap();
    runtime
        .assess_subject(&node, "frame_type", "analysis", "worker-a")
        .unwrap();
    let view = BeliefQuery::new(belief_store.as_ref())
        .current_views_for_subject(&node, &PerspectiveKey::new("default", "default").unwrap())
        .unwrap()
        .remove(0);
    let mut revision = test_revision(&view, 1, 1);
    revision.confidence = 0.7;
    revision.planner_projection.confidence = 0.7;
    revision.planner_projection.threshold = 0.7;

    let projected = belief_store.project_view(
        &revision,
        HydrationRefs {
            evidence_ids: Vec::new(),
            source_fact_ids: Vec::new(),
            graph_anchor_ids: Vec::new(),
            revision_id: Some(revision.revision_id.clone()),
        },
    );

    assert_eq!(projected.advisory_posture, "ready");

    revision.confidence = 0.69;
    revision.planner_projection.confidence = 0.69;

    let projected = belief_store.project_view(
        &revision,
        HydrationRefs {
            evidence_ids: Vec::new(),
            source_fact_ids: Vec::new(),
            graph_anchor_ids: Vec::new(),
            revision_id: Some(revision.revision_id.clone()),
        },
    );

    assert_eq!(projected.advisory_posture, "observe_or_repair");
}

#[test]
fn belief_store_blocks_second_active_lease() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph.clone(), config_json())
            .unwrap();
    runtime
        .assess_subject(&node, "frame_type", "analysis", "worker-a")
        .unwrap();
    let view = BeliefQuery::new(belief_store.as_ref())
        .current_views_for_subject(&node, &PerspectiveKey::new("default", "default").unwrap())
        .unwrap()
        .remove(0);
    let first = test_lease(&view, "lease-first", 1, 2);
    let second = test_lease(&view, "lease-second", 1, 2);

    let acquired = belief_store.acquire_lease(first).unwrap();

    assert_eq!(
        belief_store
            .get_lease(&acquired.lease_id)
            .unwrap()
            .unwrap()
            .status,
        meld_world_model::LeaseStatus::Leased
    );
    assert!(belief_store.acquire_lease(second).is_err());
}

#[test]
fn belief_store_rejects_revision_commit_outside_lease_window() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph.clone(), config_json())
            .unwrap();
    runtime
        .assess_subject(&node, "frame_type", "analysis", "worker-a")
        .unwrap();
    let view = BeliefQuery::new(belief_store.as_ref())
        .current_views_for_subject(&node, &PerspectiveKey::new("default", "default").unwrap())
        .unwrap()
        .remove(0);
    let lease = meld_world_model::AssessmentLease {
        lease_id: "lease-window".to_string(),
        belief_key: view.key.clone(),
        epoch: 2,
        owner_id: "worker-b".to_string(),
        input_cursor_start: 10,
        input_cursor_end: 12,
        started_at_seq: 10,
        expires_at_seq: 20,
        comparator_engine_id: "weighted_bayesian".to_string(),
        config_snapshot_hash: "hash".to_string(),
        status: LeaseStatus::Queued,
    };
    let lease = belief_store.acquire_lease(lease).unwrap();
    let revision = test_revision(&view, 1, 13);

    assert!(belief_store.commit_revision(&lease, &revision).is_err());
}

#[test]
fn belief_store_rejects_revision_commit_below_lease_window() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph.clone(), config_json())
            .unwrap();
    runtime
        .assess_subject(&node, "frame_type", "analysis", "worker-a")
        .unwrap();
    let view = BeliefQuery::new(belief_store.as_ref())
        .current_views_for_subject(&node, &PerspectiveKey::new("default", "default").unwrap())
        .unwrap()
        .remove(0);
    let lease = belief_store
        .acquire_lease(test_lease(&view, "lease-below", 10, 12))
        .unwrap();
    let revision = test_revision(&view, 9, 12);

    assert!(belief_store.commit_revision(&lease, &revision).is_err());
}

#[test]
fn belief_store_rejects_revision_commit_above_lease_window() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph.clone(), config_json())
            .unwrap();
    runtime
        .assess_subject(&node, "frame_type", "analysis", "worker-a")
        .unwrap();
    let view = BeliefQuery::new(belief_store.as_ref())
        .current_views_for_subject(&node, &PerspectiveKey::new("default", "default").unwrap())
        .unwrap()
        .remove(0);
    let lease = belief_store
        .acquire_lease(test_lease(&view, "lease-above", 10, 12))
        .unwrap();
    let revision = test_revision(&view, 10, 13);

    assert!(belief_store.commit_revision(&lease, &revision).is_err());
}

#[test]
fn belief_store_rejects_stale_lease_owner() {
    let (_graph_dir, graph, node) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph.clone(), config_json())
            .unwrap();
    runtime
        .assess_subject(&node, "frame_type", "analysis", "worker-a")
        .unwrap();
    let view = BeliefQuery::new(belief_store.as_ref())
        .current_views_for_subject(&node, &PerspectiveKey::new("default", "default").unwrap())
        .unwrap()
        .remove(0);
    let lease = meld_world_model::AssessmentLease {
        lease_id: "lease-active".to_string(),
        belief_key: view.key.clone(),
        epoch: 2,
        owner_id: "worker-b".to_string(),
        input_cursor_start: 1,
        input_cursor_end: 2,
        started_at_seq: 1,
        expires_at_seq: 20,
        comparator_engine_id: "weighted_bayesian".to_string(),
        config_snapshot_hash: "hash".to_string(),
        status: LeaseStatus::Queued,
    };
    let active = belief_store.acquire_lease(lease).unwrap();
    let mut stale = active.clone();
    stale.lease_id = "lease-stale".to_string();
    let revision = test_revision(&view, 1, 2);

    assert!(belief_store.commit_revision(&stale, &revision).is_err());
}

#[test]
fn no_mod_rs_is_required_for_belief() {
    assert!(!std::path::Path::new("src/belief/mod.rs").exists());
}

fn test_revision(view: &meld_world_model::BeliefView, start: u64, end: u64) -> BeliefRevision {
    BeliefRevision {
        revision_id: format!("revision-test-{start}-{end}"),
        belief_key: view.key.clone(),
        prior_revision_id: view.current_revision_id.clone(),
        comparator_engine_id: "weighted_bayesian".to_string(),
        comparator_engine_version: "1".to_string(),
        config_snapshot_hash: "hash".to_string(),
        evidence_ids: Vec::new(),
        supporting_evidence_ids: Vec::new(),
        contradicted_evidence_ids: Vec::new(),
        source_cursor_start: start,
        source_cursor_end: end,
        posterior: PosteriorSummary {
            probability: 0.5,
            meaning: "stale_probability".to_string(),
        },
        planner_projection: PlannerProjectionSummary {
            confidence_field: "confidence".to_string(),
            confidence: 0.5,
            threshold: 0.7,
        },
        confidence: 0.5,
        uncertainty: 0.5,
        precision: 0.5,
        freshness: FreshnessState {
            stale: false,
            reason: None,
            high_water_seq: end,
        },
        contradiction: ContradictionState {
            contradicted: false,
            reason: None,
            supporting_evidence_ids: Vec::new(),
            contradicted_evidence_ids: Vec::new(),
        },
        status: BeliefStatus::Settled,
        observation: None,
        provenance: BeliefProvenanceSummary::empty(),
    }
}

fn test_lease(
    view: &meld_world_model::BeliefView,
    lease_id: &str,
    start: u64,
    end: u64,
) -> meld_world_model::AssessmentLease {
    meld_world_model::AssessmentLease {
        lease_id: lease_id.to_string(),
        belief_key: view.key.clone(),
        epoch: end,
        owner_id: "worker-b".to_string(),
        input_cursor_start: start,
        input_cursor_end: end,
        started_at_seq: start,
        expires_at_seq: end + 10,
        comparator_engine_id: "weighted_bayesian".to_string(),
        config_snapshot_hash: "hash".to_string(),
        status: meld_world_model::LeaseStatus::Queued,
    }
}

proptest! {
    #[test]
    fn config_probability_fuzz_rejects_out_of_range_prior(prior in -1000.0f64..1000.0) {
        let json = config_json().replace("\"default_prior\": 0.8", &format!("\"default_prior\": {prior}"));
        let parsed = BeliefConfigLoader::load_json(&json);
        if (0.0..=1.0).contains(&prior) && prior.is_finite() {
            prop_assert!(parsed.is_ok());
        } else {
            prop_assert!(parsed.is_err());
        }
    }

    #[test]
    fn config_string_fuzz_rejects_empty_required_ids(raw in "\\PC*") {
        let escaped = serde_json::to_string(&raw).unwrap();
        let json = config_json().replace("\"family_id\": \"docs_freshness\"", &format!("\"family_id\": {escaped}"));
        let parsed = BeliefConfigLoader::load_json(&json);
        if raw.trim().is_empty() {
            prop_assert!(parsed.is_err());
        } else {
            prop_assert!(parsed.is_ok());
        }
    }
}
