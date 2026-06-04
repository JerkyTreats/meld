use crate::error::ApiError;
use crate::generation::PromptLinkContractView;
use crate::workflow::record_contracts::id_validation::{
    validate_hex64, validate_prefixed_id, validate_timestamp_ms,
};
use crate::workflow::record_contracts::schema_version::{
    validate_schema_version, WORKFLOW_RECORD_SCHEMA_VERSION_V1,
};
use serde::{Deserialize, Serialize};

const RECORD_TYPE: &str = "prompt_link";

/// Prompt link record v1 contract used by execution runtimes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptLinkRecordV1 {
    /// Schema version for the serialized contract or artifact shape.
    pub schema_version: u32,
    /// Prompt lineage identifier associated with generated frame metadata.
    pub prompt_link_id: String,
    /// Workflow thread identifier within workflow runtime state.
    pub thread_id: String,
    /// Workflow turn identifier within the owning workflow profile or thread.
    pub turn_id: String,
    /// Workspace node identifier carried across execution boundaries.
    pub node_id: String,
    /// Context frame identifier associated with this execution record.
    pub frame_id: String,
    /// Artifact identifier for the system prompt lineage record.
    pub system_prompt_artifact_id: String,
    /// Artifact identifier for the user prompt template lineage record.
    pub user_prompt_template_artifact_id: String,
    /// Artifact identifier for the rendered prompt lineage record.
    pub rendered_prompt_artifact_id: String,
    /// Artifact identifier for the context payload lineage record.
    pub context_artifact_id: String,
    /// Creation time in milliseconds since the Unix epoch.
    pub created_at_ms: u64,
}

/// Prompt link record input v1 contract used by execution runtimes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptLinkRecordInputV1 {
    /// Workflow thread identifier within workflow runtime state.
    pub thread_id: String,
    /// Workflow turn identifier within the owning workflow profile or thread.
    pub turn_id: String,
    /// Workspace node identifier carried across execution boundaries.
    pub node_id: String,
    /// Context frame identifier associated with this execution record.
    pub frame_id: String,
    /// Creation time in milliseconds since the Unix epoch.
    pub created_at_ms: u64,
}

/// Builds a versioned prompt link record from lineage and workflow context.
pub fn prompt_link_record_from_contract_v1(
    contract: &PromptLinkContractView,
    input: &PromptLinkRecordInputV1,
) -> PromptLinkRecordV1 {
    PromptLinkRecordV1 {
        schema_version: WORKFLOW_RECORD_SCHEMA_VERSION_V1,
        prompt_link_id: contract.prompt_link_id.clone(),
        thread_id: input.thread_id.clone(),
        turn_id: input.turn_id.clone(),
        node_id: input.node_id.clone(),
        frame_id: input.frame_id.clone(),
        system_prompt_artifact_id: contract.system_prompt_artifact_id.clone(),
        user_prompt_template_artifact_id: contract.user_prompt_template_artifact_id.clone(),
        rendered_prompt_artifact_id: contract.rendered_prompt_artifact_id.clone(),
        context_artifact_id: contract.context_artifact_id.clone(),
        created_at_ms: input.created_at_ms,
    }
}

/// Validates the persisted prompt link record shape.
pub fn validate_prompt_link_record_v1(record: &PromptLinkRecordV1) -> Result<(), ApiError> {
    validate_schema_version(RECORD_TYPE, record.schema_version)?;
    validate_prefixed_id(
        RECORD_TYPE,
        "prompt_link_id",
        &record.prompt_link_id,
        "prompt-link-",
    )?;
    validate_prefixed_id(RECORD_TYPE, "thread_id", &record.thread_id, "thread-")?;
    validate_prefixed_id(RECORD_TYPE, "turn_id", &record.turn_id, "turn-")?;
    validate_timestamp_ms(RECORD_TYPE, "created_at_ms", record.created_at_ms)?;
    validate_prompt_link_record_references(record)?;
    Ok(())
}

/// Validates hex references carried by a prompt link record.
pub fn validate_prompt_link_record_references(record: &PromptLinkRecordV1) -> Result<(), ApiError> {
    validate_hex64(RECORD_TYPE, "node_id", &record.node_id).map_err(map_reference_error)?;
    validate_hex64(RECORD_TYPE, "frame_id", &record.frame_id).map_err(map_reference_error)?;
    validate_hex64(
        RECORD_TYPE,
        "system_prompt_artifact_id",
        &record.system_prompt_artifact_id,
    )
    .map_err(map_reference_error)?;
    validate_hex64(
        RECORD_TYPE,
        "user_prompt_template_artifact_id",
        &record.user_prompt_template_artifact_id,
    )
    .map_err(map_reference_error)?;
    validate_hex64(
        RECORD_TYPE,
        "rendered_prompt_artifact_id",
        &record.rendered_prompt_artifact_id,
    )
    .map_err(map_reference_error)?;
    validate_hex64(
        RECORD_TYPE,
        "context_artifact_id",
        &record.context_artifact_id,
    )
    .map_err(map_reference_error)?;
    Ok(())
}

fn map_reference_error(err: ApiError) -> ApiError {
    ApiError::ConfigError(format!(
        "Workflow record references for '{}' invalid: {}",
        RECORD_TYPE, err
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mutation closure used by prompt link validation table cases.
    type PromptLinkMutation = Box<dyn FnOnce(&mut PromptLinkRecordV1)>;

    fn hex64(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn record() -> PromptLinkRecordV1 {
        PromptLinkRecordV1 {
            schema_version: WORKFLOW_RECORD_SCHEMA_VERSION_V1,
            prompt_link_id: "prompt-link-1".to_string(),
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            node_id: hex64('a'),
            frame_id: hex64('b'),
            system_prompt_artifact_id: hex64('c'),
            user_prompt_template_artifact_id: hex64('d'),
            rendered_prompt_artifact_id: hex64('e'),
            context_artifact_id: hex64('f'),
            created_at_ms: 1,
        }
    }

    #[test]
    fn prompt_link_record_round_trips_and_validates() {
        let record = record();
        let encoded = serde_json::to_string(&record).unwrap();
        let decoded = serde_json::from_str::<PromptLinkRecordV1>(&encoded).unwrap();

        validate_prompt_link_record_v1(&decoded).unwrap();
        assert_eq!(decoded, record);
    }

    #[test]
    fn prompt_link_record_rejects_required_field_violations() {
        let cases: Vec<(&str, PromptLinkMutation, &str)> = vec![
            (
                "bad schema",
                Box::new(|record| record.schema_version = 2),
                "schema_version",
            ),
            (
                "bad prompt link id",
                Box::new(|record| record.prompt_link_id = "bad".to_string()),
                "prompt_link_id",
            ),
            (
                "bad thread id",
                Box::new(|record| record.thread_id = "bad".to_string()),
                "thread_id",
            ),
            (
                "bad node id",
                Box::new(|record| record.node_id = "bad".to_string()),
                "references",
            ),
            (
                "zero timestamp",
                Box::new(|record| record.created_at_ms = 0),
                "created_at_ms",
            ),
        ];

        for (case_name, mutate, expected) in cases {
            let mut record = record();
            mutate(&mut record);

            let error = match validate_prompt_link_record_v1(&record) {
                Ok(()) => panic!("{case_name} should fail validation"),
                Err(error) => error,
            };

            assert!(
                error.to_string().contains(expected),
                "{case_name} expected error containing '{expected}', got '{error}'"
            );
        }
    }
}
