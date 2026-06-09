use std::sync::Arc;

use meld::execution::{
    build_docs_task_success_evidence, DocsTaskSuccessEvidenceError, DocsTaskSuccessEvidenceRequest,
};
use meld_events::{DomainObjectRef, EventEnvelope, EventRecord, EventRelation};
use meld_world_model::belief::{
    ingest_promoted_evidence, BeliefConfigLoader, BeliefQuery, BeliefRuntime, BeliefStore,
    BranchScope, EvidenceValue, PromotedEvidenceIngestionRequest,
};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::{AnchorSelectionRecord, PerspectiveKey, TraversalFactRecord};
use serde_json::json;

fn object(domain_id: &str, object_kind: &str, object_id: &str) -> DomainObjectRef {
    DomainObjectRef::new(domain_id, object_kind, object_id).unwrap()
}

fn content_config_json() -> &'static str {
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
            },
            {
                "schema_id": "content_written_signal",
                "required": false,
                "role": "Support",
                "reliability": 1.0,
                "precision": 1.0
            },
            {
                "schema_id": "content_review_signal",
                "required": false,
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
            },
            {
                "mapping_id": "content_written_to_signal",
                "source_kind": "content_written",
                "evidence_schema_id": "content_written_signal",
                "subject_from": "record.subject",
                "value_field": "stale_probability",
                "factor_id": "content_written_signal"
            },
            {
                "mapping_id": "content_written_to_review",
                "source_kind": "content_written",
                "evidence_schema_id": "content_review_signal",
                "subject_from": "record.subject",
                "value_field": "review_probability",
                "factor_id": "content_review_signal"
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
                },
                {
                    "factor_id": "content_written_signal",
                    "evidence_schema_id": "content_written_signal",
                    "weight": 1.0,
                    "polarity": "Supports"
                },
                {
                    "factor_id": "content_review_signal",
                    "evidence_schema_id": "content_review_signal",
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

fn task_event(event_type: &str, seq: u64) -> EventRecord {
    let envelope = EventEnvelope::new_domain(
        "2026-06-09T00:00:00Z".to_string(),
        "session-docs",
        "execution",
        "task_network::network-docs::task::task-alpha",
        event_type,
        Some("hash-a".to_string()),
        json!({
            "outcome_id": "outcome-alpha",
            "task_instance_id": "task-alpha",
            "artifact_records": [
                {
                    "artifact_id": "artifact-alpha",
                    "artifact_type_id": "docs_patch",
                    "schema_version": 1,
                    "content": {
                        "patch": "updated docs"
                    }
                }
            ]
        }),
    )
    .with_record_id("execution::task_network_publication::pub-a");
    EventRecord::from_envelope(envelope, seq)
}

fn unrelated_success_event(seq: u64) -> EventRecord {
    let envelope = EventEnvelope::new_domain(
        "2026-06-09T00:00:00Z".to_string(),
        "session-other",
        "execution",
        "task_network::network-other::task::task-beta",
        "execution.task.succeeded",
        Some("hash-b".to_string()),
        json!({
            "outcome_id": "outcome-beta",
            "task_instance_id": "task-beta",
            "artifact_records": [
                {
                    "artifact_id": "artifact-beta",
                    "artifact_type_id": "metrics_report",
                    "schema_version": 1,
                    "content": {
                        "summary": "not docs"
                    }
                }
            ]
        }),
    )
    .with_record_id("execution::task_network_publication::pub-b");
    EventRecord::from_envelope(envelope, seq)
}

#[test]
fn docs_task_success_event_builds_content_written_promoted_evidence() {
    let subject = object("workspace_fs", "node", "node-a");
    let request = DocsTaskSuccessEvidenceRequest::fresh_content(
        task_event("execution.task.succeeded", 2),
        subject.clone(),
    );

    let promoted = build_docs_task_success_evidence(request).unwrap().unwrap();

    assert_eq!(promoted.source_kind, "content_written");
    assert_eq!(
        promoted.source_id,
        "execution::task_network_publication::pub-a"
    );
    assert_eq!(promoted.subject, subject);
    assert_eq!(promoted.source_cursor_start, 2);
    assert_eq!(promoted.source_cursor_end, 2);
    assert_eq!(promoted.source_fact_ids, vec!["event-spine::2"]);
    assert_eq!(
        promoted.fields.get("stale_probability"),
        Some(&EvidenceValue::Scalar(0.0))
    );
    assert_eq!(
        promoted.fields.get("review_probability"),
        Some(&EvidenceValue::Scalar(0.2))
    );
}

#[test]
fn docs_task_failure_event_creates_no_support_evidence() {
    let subject = object("workspace_fs", "node", "node-a");
    let request = DocsTaskSuccessEvidenceRequest::fresh_content(
        task_event("execution.task.failed", 2),
        subject,
    );

    assert!(build_docs_task_success_evidence(request).unwrap().is_none());
}

#[test]
fn unrelated_task_success_event_creates_no_docs_evidence() {
    let subject = object("workspace_fs", "node", "node-a");
    let request =
        DocsTaskSuccessEvidenceRequest::fresh_content(unrelated_success_event(2), subject);

    assert!(build_docs_task_success_evidence(request).unwrap().is_none());
}

#[test]
fn docs_task_success_rejects_invalid_probability() {
    let subject = object("workspace_fs", "node", "node-a");
    let mut request = DocsTaskSuccessEvidenceRequest::fresh_content(
        task_event("execution.task.succeeded", 2),
        subject,
    );
    request.stale_probability = 2.0;

    let error = build_docs_task_success_evidence(request).unwrap_err();

    assert!(matches!(
        error,
        DocsTaskSuccessEvidenceError::InvalidRequest(message)
            if message.contains("stale probability")
    ));
}

#[test]
fn docs_writer_success_promotes_configured_freshness_evidence() {
    let (_graph_dir, graph, subject) = seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph, content_config_json())
            .unwrap();
    runtime
        .assess_subject(&subject, "frame_type", "analysis", "worker-a")
        .unwrap();
    let first = BeliefQuery::new(belief_store.as_ref())
        .current_views_for_subject(
            &subject,
            &PerspectiveKey::new("default", "default").unwrap(),
        )
        .unwrap()
        .remove(0);
    let map_request = || {
        DocsTaskSuccessEvidenceRequest::fresh_content(
            task_event("execution.task.succeeded", 2),
            subject.clone(),
        )
    };
    let ingest_request = |record| PromotedEvidenceIngestionRequest {
        record,
        config: BeliefConfigLoader::load_json(content_config_json()).unwrap(),
        perspective: PerspectiveKey::new("default", "default").unwrap(),
        branch_scope: BranchScope::main(),
        owner_id: "worker-ingest",
    };

    let promoted = build_docs_task_success_evidence(map_request())
        .unwrap()
        .unwrap();
    let result =
        ingest_promoted_evidence(belief_store.as_ref(), &runtime, ingest_request(promoted))
            .unwrap();
    let second = BeliefQuery::new(belief_store.as_ref())
        .current_view(&first.key)
        .unwrap()
        .unwrap();
    let history_len = BeliefQuery::new(belief_store.as_ref())
        .revision_history(&first.key)
        .unwrap()
        .len();

    let replay_promoted = build_docs_task_success_evidence(map_request())
        .unwrap()
        .unwrap();
    let replay = ingest_promoted_evidence(
        belief_store.as_ref(),
        &runtime,
        ingest_request(replay_promoted),
    )
    .unwrap();
    let replay_history_len = BeliefQuery::new(belief_store.as_ref())
        .revision_history(&first.key)
        .unwrap()
        .len();

    assert_eq!(result.normalized_evidence_count, 2);
    assert_eq!(result.new_assignment_count, 2);
    assert_eq!(result.committed.len(), 1);
    assert!(second.planner_projection.confidence > first.planner_projection.confidence);
    assert!(!second.freshness.stale);
    assert_eq!(replay.new_assignment_count, 0);
    assert!(replay.committed.is_empty());
    assert_eq!(replay_history_len, history_len);
}
