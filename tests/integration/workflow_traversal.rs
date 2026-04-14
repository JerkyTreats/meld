use meld::workflow::events::{
    workflow_turn_completed_envelope, workflow_turn_started_envelope,
    ExecutionWorkflowTurnEventData,
};

fn sample_turn_data() -> ExecutionWorkflowTurnEventData {
    ExecutionWorkflowTurnEventData {
        workflow_id: "wf_a".to_string(),
        thread_id: "thread_a".to_string(),
        turn_id: "turn_a".to_string(),
        turn_seq: 1,
        node_id: "node_a".to_string(),
        path: "/tmp/a.md".to_string(),
        agent_id: "writer".to_string(),
        provider_name: "mock".to_string(),
        frame_type: "analysis".to_string(),
        attempt: 1,
        plan_id: Some("plan_a".to_string()),
        level_index: Some(0),
        final_frame_id: Some("frame_a".to_string()),
        error: None,
    }
}

#[test]
fn workflow_turn_event_uses_workflow_object_refs() {
    let envelope = workflow_turn_started_envelope("session_a", sample_turn_data());

    assert!(envelope
        .objects
        .iter()
        .any(|object| object.object_kind == "workflow"));
    assert!(envelope
        .objects
        .iter()
        .any(|object| object.object_kind == "workflow_thread"));
    assert!(envelope
        .objects
        .iter()
        .any(|object| object.object_kind == "workflow_turn"));
    assert!(envelope
        .objects
        .iter()
        .any(|object| object.object_kind == "node"));
}

#[test]
fn workflow_turn_completed_links_turn_node_and_frame() {
    let envelope = workflow_turn_completed_envelope("session_a", sample_turn_data());

    assert!(envelope
        .relations
        .iter()
        .any(|relation| relation.relation_type == "targets"));
    assert!(envelope
        .relations
        .iter()
        .any(|relation| relation.relation_type == "produced"));
    assert!(envelope
        .relations
        .iter()
        .filter(|relation| relation.relation_type == "belongs_to")
        .count()
        >= 2);
}
