use std::sync::Arc;

use meld_lang::{term::Term, Proposition};
use meld_world_model::belief::{
    BeliefProvenanceSummary, BeliefStore, BranchScope, ContradictionState, FreshnessState,
    HydrationRefs, PlannerProjectionSummary, PosteriorSummary,
};
use meld_world_model::events::DomainObjectRef;
use meld_world_model::planner::{
    hash_planner_world_state, PlannerProjectionActor, PlannerProjectionRequest,
    PlannerProjectionRequestStatus, PlannerProjectionStore, PlannerProjectionTickRequest,
    PlannerProjectionWarning, MAX_PLANNER_PROJECTION_ITEMS, PLANNER_PROJECTION_ACTOR_ID,
};
use meld_world_model::world_state::graph::store::TraversalStore;
use meld_world_model::{
    AnchorSelectionRecord, BeliefKey, BeliefStatus, BeliefView, PerspectiveKey,
};

fn subject(id: &str) -> DomainObjectRef {
    DomainObjectRef::new("workspace_fs", "node", id).unwrap()
}

fn perspective() -> PerspectiveKey {
    PerspectiveKey::new("agent", "docs").unwrap()
}

fn request(source_hash: &str, subject_id: &str) -> PlannerProjectionRequest {
    PlannerProjectionRequest::identified(
        source_hash,
        "agent-docs",
        subject(subject_id),
        perspective(),
        BranchScope::main(),
        vec!["docs_freshness".to_string()],
        Vec::new(),
    )
    .unwrap()
}

fn view(subject_id: &str, confidence_field: &str) -> BeliefView {
    let subject = subject(subject_id);
    let key = BeliefKey {
        subject: subject.clone(),
        dimension_id: "docs_freshness".to_string(),
        predicate_id: "confidence".to_string(),
        perspective: perspective(),
        branch_scope: BranchScope::main(),
        evidence_policy_id: "default-policy".to_string(),
    };
    BeliefView {
        view_id: format!("view-{subject_id}"),
        key,
        current_revision_id: Some(format!("revision-{subject_id}")),
        status: BeliefStatus::Settled,
        posterior: PosteriorSummary {
            probability: 0.8,
            meaning: "freshness probability".to_string(),
        },
        planner_projection: PlannerProjectionSummary {
            confidence_field: confidence_field.to_string(),
            confidence: 0.8,
            threshold: 0.7,
        },
        uncertainty: 0.2,
        precision: 1.0,
        freshness: FreshnessState {
            stale: false,
            reasons: Vec::new(),
            high_water_seq: 4,
        },
        contradiction: ContradictionState {
            contradicted: false,
            reasons: Vec::new(),
            supporting_evidence_ids: Vec::new(),
            contradicted_evidence_ids: Vec::new(),
        },
        observation: None,
        assessment_state: "settled".to_string(),
        advisory_posture: "ready".to_string(),
        provenance: BeliefProvenanceSummary {
            evidence_ids: vec![format!("evidence-{subject_id}")],
            source_fact_ids: vec![format!("fact-{subject_id}")],
            graph_anchor_ids: vec![format!("anchor-{subject_id}")],
            objects: vec![subject],
            relations: Vec::new(),
            revision_ids: vec![format!("revision-{subject_id}")],
        },
        hydration: HydrationRefs {
            evidence_ids: vec![format!("evidence-{subject_id}")],
            source_fact_ids: vec![format!("fact-{subject_id}")],
            graph_anchor_ids: vec![format!("anchor-{subject_id}")],
            revision_id: Some(format!("revision-{subject_id}")),
        },
    }
}

fn seed_anchor(store: &TraversalStore, subject_id: &str) {
    let subject = subject(subject_id);
    let anchor = AnchorSelectionRecord {
        anchor_id: format!("anchor-{subject_id}"),
        anchor_ref: DomainObjectRef::new("context", "head", format!("{subject_id}::analysis"))
            .unwrap(),
        subject: subject.clone(),
        perspective: PerspectiveKey::new("frame_type", "analysis").unwrap(),
        target: DomainObjectRef::new("context", "frame", format!("frame-{subject_id}")).unwrap(),
        source_fact_ids: vec![format!("fact-{subject_id}")],
        created_by_fact_id: format!("fact-{subject_id}"),
        selected_at_seq: 4,
        ended_at_seq: None,
        ended_by_anchor_id: None,
        ended_by_fact_id: None,
    };
    store.put_anchor(&anchor).unwrap();
    store.set_current_anchor(&anchor).unwrap();
    store.flush().unwrap();
}

fn actor(
    db: &sled::Db,
) -> (
    Arc<PlannerProjectionStore>,
    Arc<BeliefStore>,
    Arc<TraversalStore>,
    PlannerProjectionActor,
) {
    let projection_store = Arc::new(PlannerProjectionStore::new(db.clone()).unwrap());
    let belief_store = Arc::new(BeliefStore::new(db.clone()).unwrap());
    let traversal_store = Arc::new(TraversalStore::new(db.clone()).unwrap());
    let actor = PlannerProjectionActor::new(
        Arc::clone(&projection_store),
        Arc::clone(&belief_store),
        Arc::clone(&traversal_store),
    );
    (projection_store, belief_store, traversal_store, actor)
}

#[test]
fn actor_completes_exact_frame_and_reopens_without_replaying_work() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("planner-actor");
    let request = request("source-a", "node-a");
    let frame_id;
    {
        let db = sled::open(&path).unwrap();
        let (projection_store, belief_store, traversal_store, actor) = actor(&db);
        belief_store
            .put_view(&view("node-a", "confidence"))
            .unwrap();
        seed_anchor(&traversal_store, "node-a");
        projection_store.put_pending(request.clone(), 5).unwrap();

        let report = actor.tick(PlannerProjectionTickRequest { max_items: 1 });
        assert_eq!(report.actor_id, PLANNER_PROJECTION_ACTOR_ID);
        assert_eq!(report.input_request_sequence, 5);
        assert_eq!(report.output_request_sequence, 6);
        assert_eq!(report.completed_count, 1);
        assert!(report.retryable_errors.is_empty());
        assert!(report.fatal_errors.is_empty());
        let completed = projection_store
            .get_request(&request.request_id)
            .unwrap()
            .unwrap();
        assert_eq!(completed.status, PlannerProjectionRequestStatus::Completed);
        frame_id = completed.frame_id.unwrap();
        let frame = projection_store.get_frame(&frame_id).unwrap().unwrap();
        frame.validate().unwrap();
        assert_eq!(frame.identity.request_id, request.request_id);
        assert_eq!(
            frame.identity.world_state_hash,
            hash_planner_world_state(&frame.output.world_state).unwrap()
        );
        assert!(!frame.output.world_state.propositions().is_empty());
        db.flush().unwrap();
    }

    let db = sled::open(&path).unwrap();
    let (projection_store, _, _, reopened) = actor(&db);
    let report = reopened.tick(PlannerProjectionTickRequest { max_items: 1 });
    assert_eq!(report.selected_count, 0);
    assert_eq!(report.completed_count, 0);
    assert!(projection_store.get_frame(&frame_id).unwrap().is_some());
}

#[test]
fn actor_projects_every_requested_dimension_in_canonical_order() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let (projection_store, belief_store, traversal_store, actor) = actor(&db);
    let mut second_view = view("node-multi", "confidence");
    second_view.key.dimension_id = "quality".to_string();
    second_view.view_id = "view-node-multi-quality".to_string();
    second_view.current_revision_id = Some("revision-node-multi-quality".to_string());
    belief_store
        .put_view(&view("node-multi", "confidence"))
        .unwrap();
    belief_store.put_view(&second_view).unwrap();
    seed_anchor(&traversal_store, "node-multi");
    let required_but_not_established = Proposition::Accessible {
        scope: Term::Object(subject("not-established")),
    };
    let request = PlannerProjectionRequest::identified(
        "source-multi",
        "agent-docs",
        subject("node-multi"),
        perspective(),
        BranchScope::main(),
        vec!["quality".to_string(), "docs_freshness".to_string()],
        vec![required_but_not_established.clone()],
    )
    .unwrap();
    projection_store.put_pending(request.clone(), 4).unwrap();

    let report = actor.tick(PlannerProjectionTickRequest { max_items: 1 });
    assert_eq!(report.completed_count, 1);
    let completed = projection_store
        .get_request(&request.request_id)
        .unwrap()
        .unwrap();
    let frame = projection_store
        .get_frame(completed.frame_id.as_deref().unwrap())
        .unwrap()
        .unwrap();
    let dimensions: Vec<_> = frame
        .output
        .world_state
        .propositions()
        .iter()
        .filter_map(|proposition| match proposition {
            Proposition::Holds {
                dimension: Term::Dimension(dimension),
                ..
            } => Some(dimension.as_str()),
            _ => None,
        })
        .collect();
    assert!(dimensions.contains(&"docs_freshness"));
    assert!(dimensions.contains(&"quality"));
    assert!(!frame
        .output
        .world_state
        .propositions()
        .contains(&required_but_not_established));
}

#[test]
fn missing_belief_and_graph_scope_complete_with_warnings() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let (projection_store, _, _, actor) = actor(&db);
    let request = request("source-missing", "node-missing");
    projection_store.put_pending(request.clone(), 2).unwrap();

    let report = actor.tick(PlannerProjectionTickRequest { max_items: 1 });
    assert_eq!(report.completed_count, 1);
    assert_eq!(report.failed_count, 0);
    let completed = projection_store
        .get_request(&request.request_id)
        .unwrap()
        .unwrap();
    let frame = projection_store
        .get_frame(completed.frame_id.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert!(frame
        .output
        .warnings
        .contains(&PlannerProjectionWarning::MissingBelief {
            subject: subject("node-missing"),
        }));
    assert!(frame
        .output
        .warnings
        .contains(&PlannerProjectionWarning::MissingGraphScope {
            subject: subject("node-missing"),
        }));
}

#[test]
fn deterministic_projection_error_fails_through_the_request_fence() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let (projection_store, belief_store, _, actor) = actor(&db);
    let request = request("source-invalid", "node-invalid");
    belief_store
        .put_view(&view("node-invalid", "invalid field"))
        .unwrap();
    projection_store.put_pending(request.clone(), 8).unwrap();

    let report = actor.tick(PlannerProjectionTickRequest { max_items: 1 });
    assert_eq!(report.failed_count, 1);
    assert_eq!(report.fatal_errors[0].code, "projection_failed");
    let failed = projection_store
        .get_request(&request.request_id)
        .unwrap()
        .unwrap();
    assert_eq!(failed.status, PlannerProjectionRequestStatus::Failed);
    assert!(failed.frame_id.is_none());
}

#[test]
fn actor_enforces_hard_budget_and_reports_remaining_work() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let (projection_store, _, _, actor) = actor(&db);
    for pair in [("source-a", "node-a"), ("source-b", "node-b")] {
        projection_store
            .put_pending(request(pair.0, pair.1), 5)
            .unwrap();
    }

    for max_items in [0, MAX_PLANNER_PROJECTION_ITEMS + 1] {
        let report = actor.tick(PlannerProjectionTickRequest { max_items });
        assert_eq!(report.selected_count, 0);
        assert_eq!(report.fatal_errors[0].code, "invalid_request");
    }
    let report = actor.tick(PlannerProjectionTickRequest { max_items: 1 });
    assert_eq!(report.selected_count, 1);
    assert_eq!(report.completed_count, 1);
    assert!(report.budget_exhausted);
    assert_eq!(
        projection_store
            .pending_requests_bounded(2)
            .unwrap()
            .records
            .len(),
        1
    );
}
