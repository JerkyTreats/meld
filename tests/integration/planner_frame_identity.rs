use std::path::Path;
use std::time::Duration;

use meld_execution::planning::{
    PlanningPerspectiveRef, PlanningProjectionIdentityInputs, PlanningWorldStateFrameRef,
    PlanningWorldStateRequest,
};
use meld_lang::{Proposition, Term, WorldState};
use meld_world_model::belief::BranchScope;
use meld_world_model::events::DomainObjectRef;
use meld_world_model::planner::{
    project_world_state, PlannerFieldProjectionConfig, PlannerGraphScope, PlannerProjectionContext,
    PlannerProjectionFrame, PlannerProjectionFrameIdentity, PlannerProjectionInput,
    PlannerProjectionRequest, PlannerProjectionStore, PlannerSourceRef, PLANNER_PROJECTION_VERSION,
};
use meld_world_model::PerspectiveKey;

fn reopen_sled_after_close(path: &Path) -> sled::Result<sled::Db> {
    const MAX_ATTEMPTS: usize = 50;

    for attempt in 0..MAX_ATTEMPTS {
        match sled::open(path) {
            Ok(db) => return Ok(db),
            Err(error)
                if error.to_string().contains("could not acquire lock")
                    && attempt + 1 < MAX_ATTEMPTS =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(error),
        }
    }

    unreachable!("the final open attempt returns its error")
}

fn execution_source_refs(source_refs: &[PlannerSourceRef]) -> Vec<String> {
    let mut encoded = source_refs
        .iter()
        .map(|source_ref| serde_json::to_string(source_ref).unwrap())
        .collect::<Vec<_>>();
    encoded.sort();
    encoded.dedup();
    encoded
}

#[test]
fn durable_world_model_frame_identity_requires_the_exact_execution_projection() {
    let subject = DomainObjectRef::new("workspace_fs", "node", "readme").unwrap();
    let perspective = PerspectiveKey::new("agent", "docs").unwrap();
    let branch_scope = BranchScope::main();
    let world_state_request = PlanningWorldStateRequest {
        goal_id: "goal-docs-freshness".to_string(),
        agent_id: "agent-docs".to_string(),
        subject: subject.clone(),
        source_seq: 7,
        target: Proposition::Accessible {
            scope: Term::Object(subject.clone()),
        },
        perspective: PlanningPerspectiveRef::new(
            perspective.perspective_kind.clone(),
            perspective.perspective_id.clone(),
        )
        .unwrap(),
        branch_id: branch_scope.branch_id.clone(),
        requested_dimensions: vec!["docs_freshness".to_string()],
        required_preconditions: Vec::new(),
    };
    let source_request_hash = world_state_request.canonical_hash().unwrap();
    let projection_request = PlannerProjectionRequest::identified(
        source_request_hash.clone(),
        world_state_request.agent_id.clone(),
        subject.clone(),
        perspective.clone(),
        branch_scope.clone(),
        world_state_request.requested_dimensions.clone(),
        world_state_request.required_preconditions.clone(),
    )
    .unwrap();
    let projection_output = project_world_state(PlannerProjectionInput {
        context: PlannerProjectionContext {
            subject,
            perspective,
            branch_scope,
            projection_version: PLANNER_PROJECTION_VERSION.to_string(),
        },
        belief_view: None,
        graph_scope: Some(PlannerGraphScope {
            accessible: true,
            anchor_ids: vec!["anchor-readme".to_string()],
            source_fact_ids: vec!["fact-readme".to_string()],
        }),
        field_config: PlannerFieldProjectionConfig::default(),
    })
    .unwrap();
    let projection_frame = PlannerProjectionFrame {
        identity: PlannerProjectionFrameIdentity::identified(
            &projection_request,
            &projection_output,
        )
        .unwrap(),
        output: projection_output,
        completed_at_seq: 12,
    };

    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("planner");
    let db = sled::open(&path).unwrap();
    let store = PlannerProjectionStore::new(db.clone()).unwrap();
    store.put_pending(projection_request.clone(), 11).unwrap();
    store
        .complete(
            &projection_request.request_id,
            11,
            projection_frame.clone(),
            12,
        )
        .unwrap();
    db.flush().unwrap();
    drop(store);
    drop(db);

    let reopened = PlannerProjectionStore::new(reopen_sled_after_close(&path).unwrap()).unwrap();
    let durable_frame = reopened
        .get_frame(&projection_frame.identity.frame_id)
        .unwrap()
        .unwrap();
    assert_eq!(durable_frame, projection_frame);
    let source_refs = execution_source_refs(&durable_frame.identity.source_refs);
    let mut round_tripped_refs = source_refs
        .iter()
        .map(|source_ref| serde_json::from_str::<PlannerSourceRef>(source_ref).unwrap())
        .collect::<Vec<_>>();
    round_tripped_refs.sort();
    assert_eq!(round_tripped_refs, durable_frame.identity.source_refs);

    let execution_frame = PlanningWorldStateFrameRef::from_authority(
        durable_frame.identity.frame_id.clone(),
        durable_frame.identity.request_id.clone(),
        durable_frame.identity.source_request_hash.clone(),
        durable_frame.identity.projection_version.clone(),
        durable_frame.identity.projection_hash.clone(),
        durable_frame.identity.world_state_hash.clone(),
        &world_state_request,
        &durable_frame.output.world_state,
        source_refs,
        Vec::new(),
    )
    .unwrap();
    let identity = PlanningProjectionIdentityInputs::from_projection(
        &world_state_request,
        &durable_frame.output.world_state,
        &execution_frame,
    )
    .unwrap();

    assert_eq!(
        execution_frame.source_request_hash,
        world_state_request.canonical_hash().unwrap()
    );
    assert_eq!(execution_frame.frame_id, durable_frame.identity.frame_id);
    assert_eq!(
        execution_frame.request_id,
        durable_frame.identity.request_id
    );
    assert_eq!(
        execution_frame.projection_version,
        durable_frame.identity.projection_version
    );
    assert_eq!(
        execution_frame.projection_hash,
        durable_frame.identity.projection_hash
    );
    assert_eq!(
        execution_frame.world_state_hash,
        durable_frame.identity.world_state_hash
    );
    assert_eq!(
        identity.frame_ref(Vec::new()).unwrap().frame_id,
        durable_frame.identity.frame_id
    );

    let mutated_world_state = WorldState::empty();
    assert_ne!(mutated_world_state, durable_frame.output.world_state);
    assert!(PlanningProjectionIdentityInputs::from_projection(
        &world_state_request,
        &mutated_world_state,
        &execution_frame,
    )
    .is_err());

    let mut mutated_hash_frame = execution_frame;
    mutated_hash_frame.world_state_hash = "mutated-world-state-hash".to_string();
    assert!(PlanningProjectionIdentityInputs::from_projection(
        &world_state_request,
        &durable_frame.output.world_state,
        &mutated_hash_frame,
    )
    .is_err());
}
