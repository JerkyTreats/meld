//! Task run initialization contracts.

use crate::error::ApiError;
use crate::task::contracts::CompiledTaskRecord;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

/// External structured artifact supplied at task run creation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InitArtifactValue {
    /// Task initialization slot that supplies this artifact.
    pub init_slot_id: String,
    /// Artifact type identifier used for contract validation and routing.
    pub artifact_type_id: String,
    /// Schema version for the serialized contract or artifact shape.
    pub schema_version: u32,
    /// Structured artifact content supplied for this init slot.
    pub content: Value,
}

/// Ephemeral run context for one task instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRunContext {
    /// Concrete task run identifier within the execution runtime.
    pub task_run_id: String,
    /// Session identifier carried across the execution boundary.
    pub session_id: Option<String>,
    /// Trigger name or source that created this task run.
    pub trigger: String,
}

/// Structured payload required to create one task run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskInitializationPayload {
    /// Authored or compiled task identifier within the execution domain.
    pub task_id: String,
    /// Reference to the compiled task artifact or registry entry.
    pub compiled_task_ref: String,
    /// External artifacts supplied for task init slots.
    pub init_artifacts: Vec<InitArtifactValue>,
    /// Runtime context assigned to this task run.
    pub task_run_context: TaskRunContext,
}

/// Validates that the initialization payload satisfies the compiled task init contract.
pub fn validate_task_initialization(
    compiled_task: &CompiledTaskRecord,
    payload: &TaskInitializationPayload,
) -> Result<(), ApiError> {
    if payload.task_id != compiled_task.task_id {
        return Err(ApiError::ConfigError(format!(
            "Task initialization payload targets '{}' but compiled task is '{}'",
            payload.task_id, compiled_task.task_id
        )));
    }

    let mut supplied_init_slots = HashSet::new();
    for artifact in &payload.init_artifacts {
        if !supplied_init_slots.insert(artifact.init_slot_id.as_str()) {
            return Err(ApiError::ConfigError(format!(
                "Task initialization payload contains duplicate init slot '{}'",
                artifact.init_slot_id
            )));
        }
        let expected = compiled_task
            .init_slots
            .iter()
            .find(|slot| slot.init_slot_id == artifact.init_slot_id)
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Task initialization payload supplies unknown init slot '{}'",
                    artifact.init_slot_id
                ))
            })?;
        if expected.artifact_type_id != artifact.artifact_type_id
            || expected.schema_version != artifact.schema_version
        {
            return Err(ApiError::ConfigError(format!(
                "Task initialization payload init slot '{}' does not match the compiled task contract",
                artifact.init_slot_id
            )));
        }
    }

    for init_slot in &compiled_task.init_slots {
        if init_slot.required && !supplied_init_slots.contains(init_slot.init_slot_id.as_str()) {
            return Err(ApiError::ConfigError(format!(
                "Task initialization payload is missing required init slot '{}'",
                init_slot.init_slot_id
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::contracts::{CompiledTaskRecord, TaskInitSlotSpec};
    use serde_json::json;

    fn compiled_task() -> CompiledTaskRecord {
        CompiledTaskRecord {
            task_id: "task_docs_writer".to_string(),
            task_version: 1,
            init_slots: vec![TaskInitSlotSpec {
                init_slot_id: "target_selector".to_string(),
                artifact_type_id: "target_selector".to_string(),
                schema_version: 1,
                required: true,
            }],
            capability_instances: Vec::new(),
            dependency_edges: Vec::new(),
        }
    }

    #[test]
    fn validate_init_payload_accepts_matching_artifact() {
        let compiled_task = compiled_task();
        let payload = TaskInitializationPayload {
            task_id: compiled_task.task_id.clone(),
            compiled_task_ref: "compiled_task_docs_writer_v1".to_string(),
            init_artifacts: vec![InitArtifactValue {
                init_slot_id: "target_selector".to_string(),
                artifact_type_id: "target_selector".to_string(),
                schema_version: 1,
                content: json!({ "path": "docs" }),
            }],
            task_run_context: TaskRunContext {
                task_run_id: "taskrun_1".to_string(),
                session_id: Some("session_1".to_string()),
                trigger: "workflow.execute".to_string(),
            },
        };

        validate_task_initialization(&compiled_task, &payload).unwrap();
    }

    #[test]
    fn validate_init_payload_rejects_missing_required_slot() {
        let compiled_task = compiled_task();
        let payload = TaskInitializationPayload {
            task_id: compiled_task.task_id.clone(),
            compiled_task_ref: "compiled_task_docs_writer_v1".to_string(),
            init_artifacts: vec![],
            task_run_context: TaskRunContext {
                task_run_id: "taskrun_1".to_string(),
                session_id: Some("session_1".to_string()),
                trigger: "workflow.execute".to_string(),
            },
        };

        let error = validate_task_initialization(&compiled_task, &payload).unwrap_err();

        assert!(matches!(error, ApiError::ConfigError(_)));
        assert!(error.to_string().contains("missing required init slot"));
    }

    #[test]
    fn validate_init_payload_rejects_artifact_type_or_schema_mismatch() {
        let compiled_task = compiled_task();
        let cases = [("other_selector", 1_u32), ("target_selector", 2_u32)];

        for (artifact_type_id, schema_version) in cases {
            let payload = TaskInitializationPayload {
                task_id: compiled_task.task_id.clone(),
                compiled_task_ref: "compiled_task_docs_writer_v1".to_string(),
                init_artifacts: vec![InitArtifactValue {
                    init_slot_id: "target_selector".to_string(),
                    artifact_type_id: artifact_type_id.to_string(),
                    schema_version,
                    content: json!({ "path": "docs" }),
                }],
                task_run_context: TaskRunContext {
                    task_run_id: "taskrun_1".to_string(),
                    session_id: Some("session_1".to_string()),
                    trigger: "workflow.execute".to_string(),
                },
            };

            let error = validate_task_initialization(&compiled_task, &payload).unwrap_err();

            assert!(matches!(error, ApiError::ConfigError(_)));
            assert!(error.to_string().contains("compiled task contract"));
        }
    }
}
