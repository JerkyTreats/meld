use meld::error::ApiError;
use meld::prompt_context::{
    prepare_generated_lineage, PromptContextArtifactStorage, PromptContextLineageInput,
};
use meld::workflow::record_contracts::{
    prompt_link_record_from_contract_v1, validate_prompt_link_record_v1,
    validate_thread_turn_gate_record_v1, GateOutcome, PromptLinkRecordInputV1,
    ThreadTurnGateRecordV1, WORKFLOW_RECORD_SCHEMA_VERSION_V1,
};
use tempfile::TempDir;

#[test]
fn workflow_contracts_reexport_metadata_validators() {
    let record = ThreadTurnGateRecordV1 {
        schema_version: WORKFLOW_RECORD_SCHEMA_VERSION_V1,
        thread_id: "thread-a".to_string(),
        turn_id: "turn-1".to_string(),
        gate_name: "schema_required_fields".to_string(),
        outcome: GateOutcome::Pass,
        reasons: vec![],
        evaluated_at_ms: 1,
    };

    validate_thread_turn_gate_record_v1(&record).unwrap();
}

#[test]
fn context_lineage_maps_to_valid_prompt_link_record() {
    let temp = TempDir::new().unwrap();
    let storage = PromptContextArtifactStorage::new(temp.path()).unwrap();

    let prepared = prepare_generated_lineage(
        &storage,
        &PromptContextLineageInput {
            system_prompt: "system".to_string(),
            user_prompt_template: "template".to_string(),
            rendered_prompt: "rendered".to_string(),
            context_payload: "context".to_string(),
        },
        "writer",
        "provider",
        "model",
        "local",
    )
    .unwrap();

    let input = PromptLinkRecordInputV1 {
        thread_id: "thread-a".to_string(),
        turn_id: "turn-1".to_string(),
        node_id: "0".repeat(64),
        frame_id: "1".repeat(64),
        created_at_ms: 1,
    };

    let record = prompt_link_record_from_contract_v1(&prepared.prompt_link_contract, &input);
    validate_prompt_link_record_v1(&record).unwrap();
}

#[test]
fn invalid_prompt_link_reference_is_typed() {
    let mut record = meld::workflow::record_contracts::PromptLinkRecordV1 {
        schema_version: WORKFLOW_RECORD_SCHEMA_VERSION_V1,
        prompt_link_id: "prompt-link-aaaaaaaaaaaaaaaa".to_string(),
        thread_id: "thread-a".to_string(),
        turn_id: "turn-1".to_string(),
        node_id: "0".repeat(64),
        frame_id: "1".repeat(64),
        system_prompt_artifact_id: "2".repeat(64),
        user_prompt_template_artifact_id: "3".repeat(64),
        rendered_prompt_artifact_id: "4".repeat(64),
        context_artifact_id: "5".repeat(64),
        created_at_ms: 1,
    };
    record.context_artifact_id = "deadbeef".to_string();

    let err = validate_prompt_link_record_v1(&record).unwrap_err();
    assert!(matches!(
        err,
        ApiError::WorkflowRecordReferenceInvalid { .. }
    ));
}
