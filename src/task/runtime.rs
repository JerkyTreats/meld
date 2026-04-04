//! Task runtime loop over ready capability invocations.

use crate::api::ContextApi;
use crate::capability::{CapabilityExecutionContext, CapabilityExecutorRegistry};
use crate::error::ApiError;
use crate::task::executor::TaskExecutor;
use crate::task::ArtifactRecord;
use futures::stream::{FuturesUnordered, StreamExt};

/// Summary for one completed task runtime execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRunSummary {
    pub task_id: String,
    pub task_run_id: String,
    pub completed_instances: usize,
    pub invocation_count: usize,
    pub artifact_count: usize,
}

/// Executes one task to completion using the registered capability invokers.
pub async fn execute_task_to_completion(
    api: &ContextApi,
    executor: &mut TaskExecutor,
    registry: &CapabilityExecutorRegistry,
) -> Result<TaskRunSummary, ApiError> {
    loop {
        if executor.is_complete() {
            return Ok(TaskRunSummary {
                task_id: executor.compiled_task().task_id.clone(),
                task_run_id: executor.init_payload().task_run_context.task_run_id.clone(),
                completed_instances: executor.completed_count(),
                invocation_count: executor.invocation_records().len(),
                artifact_count: executor.artifact_repo().record().artifacts.len(),
            });
        }

        let ready = executor.release_ready_invocations(CapabilityExecutionContext::default())?;
        if ready.is_empty() {
            return Err(ApiError::GenerationFailed(format!(
                "Task '{}' is blocked with no ready capability instances",
                executor.compiled_task().task_id
            )));
        }

        let mut futures = FuturesUnordered::new();
        for payload in ready {
            let compiled_task = executor.compiled_task().clone();
            let instance = compiled_task
                .capability_instances
                .iter()
                .find(|instance| instance.capability_instance_id == payload.capability_instance_id)
                .ok_or_else(|| {
                    ApiError::ConfigError(format!(
                        "Task '{}' is missing capability instance '{}'",
                        compiled_task.task_id, payload.capability_instance_id
                    ))
                })?
                .clone();
            let runtime_init = registry.runtime_init_for(&instance)?;
            let invoker = registry
                .get(&instance.capability_type_id, instance.capability_version)
                .cloned()
                .ok_or_else(|| {
                    ApiError::ConfigError(format!(
                        "Task '{}' is missing invoker for '{}' version '{}'",
                        compiled_task.task_id,
                        instance.capability_type_id,
                        instance.capability_version
                    ))
                })?;
            let invocation_id = payload.invocation_id.clone();
            futures.push(async move {
                let outcome = invoker.invoke(api, &runtime_init, &payload).await;
                (
                    invocation_id,
                    payload.capability_instance_id.clone(),
                    outcome,
                )
            });
        }

        while let Some((invocation_id, capability_instance_id, outcome)) = futures.next().await {
            match outcome {
                Ok(result) => executor.record_success(&invocation_id, result.emitted_artifacts)?,
                Err(err) => {
                    executor.record_failure(
                        &invocation_id,
                        failure_artifact(
                            executor.compiled_task().task_id.clone(),
                            capability_instance_id,
                            invocation_id.clone(),
                            err.to_string(),
                        ),
                        err.to_string(),
                    )?;
                    return Err(err);
                }
            }
        }
    }
}

fn failure_artifact(
    task_id: String,
    capability_instance_id: String,
    invocation_id: String,
    message: String,
) -> ArtifactRecord {
    ArtifactRecord {
        artifact_id: format!("{invocation_id}::failure"),
        artifact_type_id: "capability_failure".to_string(),
        schema_version: 1,
        content: serde_json::json!({
            "message": message,
        }),
        producer: crate::task::ArtifactProducerRef {
            task_id,
            capability_instance_id,
            invocation_id: Some(invocation_id),
            output_slot_id: Some("failure".to_string()),
        },
    }
}
