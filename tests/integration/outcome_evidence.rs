use std::sync::Arc;

use meld_events::{DomainObjectRef, EventRecord};
use meld_world_model::belief::{
    build_docs_task_success_evidence, ingest_promoted_evidence, BeliefConfigLoader, BeliefQuery,
    BeliefRuntime, BeliefStore, BranchScope, DocsTaskEvidenceError, DocsTaskSuccessEvidenceRequest,
    EvidenceValue, PromotedEvidenceIngestionRequest,
};
use meld_world_model::PerspectiveKey;

use super::docs_freshness_fixture::{
    DocsFreshnessFirstProofFixture, CONTENT_SOURCE_KIND, PUBLICATION_ID, REQUIRED_ARTIFACT_TYPE_ID,
};

fn docs_success_request(
    event: EventRecord,
    subject: DomainObjectRef,
) -> DocsTaskSuccessEvidenceRequest {
    DocsTaskSuccessEvidenceRequest {
        event,
        subject,
        stale_probability: 0.0,
        review_probability: 0.2,
        source_kind: CONTENT_SOURCE_KIND.to_string(),
        required_artifact_type_id: Some(REQUIRED_ARTIFACT_TYPE_ID.to_string()),
    }
}

#[test]
fn docs_task_success_event_builds_content_written_promoted_evidence() {
    let fixture = DocsFreshnessFirstProofFixture::new();
    let subject = fixture.subject();
    let request = docs_success_request(fixture.task_success_event(2), subject.clone());

    let promoted = build_docs_task_success_evidence(request).unwrap().unwrap();

    assert_eq!(promoted.source_kind, CONTENT_SOURCE_KIND);
    assert_eq!(
        promoted.source_id,
        DocsFreshnessFirstProofFixture::publication_record_id(PUBLICATION_ID)
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
    let fixture = DocsFreshnessFirstProofFixture::new();
    let request = docs_success_request(fixture.task_failure_event(2), fixture.subject());

    assert!(build_docs_task_success_evidence(request).unwrap().is_none());
}

#[test]
fn unrelated_task_success_event_creates_no_docs_evidence() {
    let fixture = DocsFreshnessFirstProofFixture::new();
    let request = docs_success_request(fixture.unrelated_task_success_event(2), fixture.subject());

    assert!(build_docs_task_success_evidence(request).unwrap().is_none());
}

#[test]
fn docs_task_success_rejects_invalid_probability() {
    let fixture = DocsFreshnessFirstProofFixture::new();
    let mut request = docs_success_request(fixture.task_success_event(2), fixture.subject());
    request.stale_probability = 2.0;

    let error = build_docs_task_success_evidence(request).unwrap_err();

    assert!(matches!(
        error,
        DocsTaskEvidenceError::InvalidRequest(message)
            if message.contains("stale probability")
    ));
}

#[test]
fn root_production_contains_no_docs_evidence_mapping_or_belief_mutation() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    collect_rust_sources(&root.join("src"), &mut sources);
    let forbidden = [
        ["ingest", "_promoted_evidence"].concat(),
        ["build_docs_task", "_success_evidence"].concat(),
        ["stale", "_probability"].concat(),
        ["review", "_probability"].concat(),
        ["content", "_written"].concat(),
    ];

    for path in sources {
        let source = std::fs::read_to_string(&path).unwrap();
        for fragment in &forbidden {
            assert!(
                !source.contains(fragment),
                "root production source {} contains world-model evidence policy fragment {fragment}",
                path.display()
            );
        }
    }
}

fn collect_rust_sources(directory: &std::path::Path, sources: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rust_sources(&path, sources);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
}

#[test]
fn docs_writer_success_promotes_configured_freshness_evidence() {
    let fixture = DocsFreshnessFirstProofFixture::new();
    let (_graph_dir, graph, subject) = fixture.seeded_graph();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime =
        BeliefRuntime::from_json_config(belief_store.clone(), graph, fixture.belief_config_json())
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
    let map_request = || docs_success_request(fixture.task_success_event(2), subject.clone());
    let ingest_request = |record| PromotedEvidenceIngestionRequest {
        record,
        config: BeliefConfigLoader::load_json(fixture.belief_config_json()).unwrap(),
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
