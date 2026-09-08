use std::sync::Arc;

use crate::integration::outcome_evidence_support::{
    build_docs_task_success_evidence, DocsTaskSuccessEvidenceError, DocsTaskSuccessEvidenceRequest,
};
use meld_events::{DomainObjectRef, EventRecord};
use meld_world_model::belief::{
    ingest_promoted_evidence, BeliefConfigLoader, BeliefQuery, BeliefRuntime, BeliefStore,
    BranchScope, EvidenceValue, PromotedEvidenceIngestionRequest,
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
        DocsTaskSuccessEvidenceError::InvalidRequest(message)
            if message.contains("stale probability")
    ));
}

#[test]
fn docs_writer_success_promotes_configured_freshness_evidence() {
    let fixture = DocsFreshnessFirstProofFixture::new();
    let subject = fixture.subject();
    let mut config = BeliefConfigLoader::load_json(fixture.belief_config_json())
        .unwrap()
        .config;
    config.initial_assessment = meld_world_model::belief::InitialAssessmentPolicy::PriorAllowed;
    let config_json = serde_json::to_string(&config).unwrap();
    let belief_dir = tempfile::tempdir().unwrap();
    let belief_store =
        Arc::new(BeliefStore::new(sled::open(belief_dir.path().join("belief")).unwrap()).unwrap());
    let runtime = BeliefRuntime::from_json_config(belief_store.clone(), &config_json).unwrap();
    runtime.assess_subject(&subject, "worker-a").unwrap();
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
        config: BeliefConfigLoader::load_json(&config_json).unwrap(),
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
