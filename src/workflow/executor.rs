//! Root workflow executor adapter over `meld-execution`.

use async_trait::async_trait;
use std::path::Path;

use crate::context::frame::{Basis, Frame};
use crate::context::generation::contracts::GeneratedMetadataBuilder;
use crate::error::ApiError;
use crate::execution::{
    ExecutionEventContext, ExecutionRuntimeContext, SystemPromptPort, WorldModelQueryPort,
};
use crate::metadata::frame_write_contract::build_generated_metadata;
use crate::task::{
    execute_task_to_completion, load_task_package_spec_for_workflow,
    prepare_registered_workflow_task_run, workflow_task_run_id_for_target, TaskExecutor,
    WorkflowPackageTriggerRequest, WorkflowTaskTelemetry,
};
use crate::types::FrameID;
use crate::workflow::registry::RegisteredWorkflowProfile;
use crate::workflow::state_store::{
    WorkflowStateStore, WorkflowThreadRecord, WorkflowThreadStatus,
};
use crate::workflow::task_path::WorkflowTaskPathRuntime;
pub use meld_execution::workflow::{WorkflowExecutionRequest, WorkflowExecutionSummary};
use meld_execution::workflow::{WorkflowExecutorRuntime, WorkflowTaskPathExecutor};

pub fn execute_registered_workflow<A>(
    api: &A,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowExecutionRequest,
    task_path_runtime: &WorkflowTaskPathRuntime,
    event_context: Option<&ExecutionEventContext>,
) -> Result<WorkflowExecutionSummary, ApiError>
where
    A: ExecutionRuntimeContext + SystemPromptPort + WorldModelQueryPort + 'static,
{
    let rt = tokio::runtime::Runtime::new()
        .map_err(|err| ApiError::ProviderError(format!("Failed to create runtime: {}", err)))?;

    rt.block_on(async move {
        execute_registered_workflow_async(
            api,
            workspace_root,
            registered_profile,
            request,
            task_path_runtime,
            event_context,
        )
        .await
    })
}

pub(crate) async fn execute_registered_workflow_async<A>(
    api: &A,
    workspace_root: &Path,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowExecutionRequest,
    task_path_runtime: &WorkflowTaskPathRuntime,
    event_context: Option<&ExecutionEventContext>,
) -> Result<WorkflowExecutionSummary, ApiError>
where
    A: ExecutionRuntimeContext + SystemPromptPort + WorldModelQueryPort + 'static,
{
    let metadata_builder: &GeneratedMetadataBuilder = &build_generated_metadata;
    let state_store = WorkflowStateStore::new(workspace_root)?.into_inner();
    let runtime = WorkflowExecutorRuntime {
        state_store,
        metadata_builder,
        build_frame: &build_workflow_frame,
        node_not_found: &workflow_node_not_found,
        task_path_executor: task_path_runtime,
        now_millis: crate::telemetry::now_millis,
    };

    meld_execution::workflow::execute_registered_workflow_async(
        api,
        workspace_root,
        registered_profile,
        request,
        &runtime,
        event_context,
    )
    .await
}

fn build_workflow_frame(
    node_id: FrameID,
    content: Vec<u8>,
    frame_type: String,
    agent_id: String,
    metadata: crate::metadata::frame_types::FrameMetadata,
) -> Result<Frame, ApiError> {
    Ok(Frame::new(
        Basis::Node(node_id),
        content,
        frame_type,
        agent_id,
        metadata,
    )?)
}

fn workflow_node_not_found(node_id: FrameID) -> ApiError {
    ApiError::NodeNotFound(node_id)
}

#[async_trait]
impl<A> WorkflowTaskPathExecutor<A, ApiError> for WorkflowTaskPathRuntime
where
    A: ExecutionRuntimeContext + WorldModelQueryPort + 'static,
{
    fn uses_task_package_path(
        &self,
        registered_profile: &RegisteredWorkflowProfile,
    ) -> Result<bool, ApiError> {
        crate::task::workflow_uses_task_package_path(registered_profile)
    }

    fn resolve_completed_task_path_final_frame(
        &self,
        api: &A,
        registered_profile: &RegisteredWorkflowProfile,
        request: &WorkflowExecutionRequest,
        existing: &WorkflowThreadRecord,
    ) -> Result<FrameID, ApiError> {
        let task_run_id = workflow_task_run_id_for_target(registered_profile, request.node_id);
        let anchor = api
            .current_artifact_for_task_run(&task_run_id, "frame_ref")?
            .ok_or_else(|| {
                ApiError::GenerationFailed(format!(
                    "Workflow task path missing required frame_ref artifact anchor for task run '{}'",
                    task_run_id
                ))
            })?;
        if anchor.target_domain_id != "execution" || anchor.target_object_kind != "artifact" {
            return Err(ApiError::GenerationFailed(format!(
                "Workflow task path expected execution artifact anchor target, got '{}::{}::{}'",
                anchor.target_domain_id, anchor.target_object_kind, anchor.target_object_id
            )));
        }
        let frame_id_hex = existing.final_frame_id.as_deref().ok_or_else(|| {
            ApiError::GenerationFailed(format!(
                "Workflow task path completed thread '{}' is missing durable final_frame_id",
                existing.thread_id
            ))
        })?;
        decode_frame_id(frame_id_hex)
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_task_path(
        &self,
        api: &A,
        workspace_root: &Path,
        registered_profile: &RegisteredWorkflowProfile,
        request: &WorkflowExecutionRequest,
        event_context: Option<&ExecutionEventContext>,
        state_store: &meld_execution::workflow::WorkflowStateStore,
        thread_id: &str,
        target_path: &str,
        final_turn_seq: u32,
        now_millis: fn() -> u64,
    ) -> Result<WorkflowExecutionSummary, ApiError> {
        let package_spec =
            load_task_package_spec_for_workflow(registered_profile)?.ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Workflow '{}' does not have a task package route",
                    registered_profile.profile.workflow_id
                ))
            })?;

        let prepared = prepare_registered_workflow_task_run(
            api,
            workspace_root,
            registered_profile,
            &WorkflowPackageTriggerRequest {
                package_id: package_spec.package_id,
                workflow_id: registered_profile.profile.workflow_id.clone(),
                node_id: Some(request.node_id),
                path: None,
                agent_id: request.agent_id.clone(),
                provider: request.provider.clone(),
                frame_type: request.frame_type.clone(),
                force: request.force,
                session_id: event_context.map(|ctx| ctx.session_id.clone()),
            },
            &self.catalog,
        )?;
        let mut executor = TaskExecutor::new(
            prepared.compiled_task,
            prepared.init_payload,
            format!("task_repo::{thread_id}"),
        )?;
        let task_summary = match execute_task_to_completion(
            api,
            &mut executor,
            &self.catalog,
            &self.registry,
            event_context,
            Some(&WorkflowTaskTelemetry {
                workflow_id: registered_profile.profile.workflow_id.clone(),
                thread_id: thread_id.to_string(),
                agent_id: request.agent_id.clone(),
                provider_name: request.provider.provider_name.clone(),
                frame_type: request.frame_type.clone(),
                plan_id: request.plan_id.clone(),
                level_index: request.level_index,
                turn_seq_by_id: registered_profile
                    .profile
                    .ordered_turns()
                    .into_iter()
                    .map(|turn| (turn.turn_id, turn.seq))
                    .collect(),
            }),
        )
        .await
        {
            Ok(summary) => summary,
            Err(err) => {
                state_store.upsert_thread(&WorkflowThreadRecord {
                    thread_id: thread_id.to_string(),
                    workflow_id: registered_profile.profile.workflow_id.clone(),
                    node_id: hex::encode(request.node_id),
                    frame_type: request.frame_type.clone(),
                    status: WorkflowThreadStatus::Failed,
                    next_turn_seq: 1,
                    updated_at_ms: now_millis(),
                    final_frame_id: None,
                })?;
                return Err(err);
            }
        };

        let final_frame_id =
            resolve_final_frame_from_traversal_artifact(api, &executor, &task_summary.task_run_id)?;
        state_store.upsert_thread(&WorkflowThreadRecord {
            thread_id: thread_id.to_string(),
            workflow_id: registered_profile.profile.workflow_id.clone(),
            node_id: hex::encode(request.node_id),
            frame_type: request.frame_type.clone(),
            status: WorkflowThreadStatus::Completed,
            next_turn_seq: final_turn_seq.saturating_add(1),
            updated_at_ms: now_millis(),
            final_frame_id: Some(hex::encode(final_frame_id)),
        })?;

        emit_workflow_target_completed(
            api,
            event_context,
            registered_profile,
            request,
            thread_id,
            target_path,
            final_frame_id,
            task_summary.completed_instances / 3,
        );

        Ok(WorkflowExecutionSummary {
            workflow_id: registered_profile.profile.workflow_id.clone(),
            thread_id: thread_id.to_string(),
            turns_completed: task_summary.completed_instances / 3,
            final_frame_id: Some(final_frame_id),
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_workflow_target_completed(
    api: &(impl ExecutionRuntimeContext + ?Sized),
    event_context: Option<&ExecutionEventContext>,
    registered_profile: &RegisteredWorkflowProfile,
    request: &WorkflowExecutionRequest,
    thread_id: &str,
    target_path: &str,
    final_frame_id: FrameID,
    turns_completed: usize,
) {
    if let Some(ctx) = event_context {
        let _ = api.emit_progress_event(
            ctx,
            "workflow_target_completed",
            serde_json::json!(meld_execution::workflow::WorkflowTargetProgressEventData {
                workflow_id: registered_profile.profile.workflow_id.clone(),
                thread_id: thread_id.to_string(),
                node_id: hex::encode(request.node_id),
                path: target_path.to_string(),
                agent_id: request.agent_id.clone(),
                provider_name: request.provider.provider_name.clone(),
                frame_type: request.frame_type.clone(),
                plan_id: request.plan_id.clone(),
                level_index: request.level_index,
                final_frame_id: Some(hex::encode(final_frame_id)),
                turns_completed: Some(turns_completed),
                reused_existing_head: Some(false),
            }),
        );
    }
}

fn resolve_final_frame_from_traversal_artifact(
    api: &(impl WorldModelQueryPort<Error = ApiError> + ?Sized),
    executor: &TaskExecutor,
    task_run_id: &str,
) -> Result<FrameID, ApiError> {
    let anchor = api
        .current_artifact_for_task_run(task_run_id, "frame_ref")?
        .ok_or_else(|| {
            ApiError::GenerationFailed(format!(
                "Workflow task path missing required frame_ref artifact anchor for task run '{}'",
                task_run_id
            ))
        })?;
    resolve_frame_id_from_artifact_anchor(&anchor, executor)
}

fn resolve_frame_id_from_artifact_anchor(
    anchor: &crate::execution::TaskRunArtifactAnchor,
    executor: &TaskExecutor,
) -> Result<FrameID, ApiError> {
    if anchor.target_domain_id != "execution" || anchor.target_object_kind != "artifact" {
        return Err(ApiError::GenerationFailed(format!(
            "Workflow task path expected execution artifact anchor target, got '{}::{}::{}'",
            anchor.target_domain_id, anchor.target_object_kind, anchor.target_object_id
        )));
    }

    let artifact = executor
        .artifact_repo()
        .get_artifact(&anchor.target_object_id)
        .ok_or_else(|| {
            ApiError::GenerationFailed(format!(
                "Workflow task path could not load artifact '{}'",
                anchor.target_object_id
            ))
        })?;
    if artifact.artifact_type_id != "frame_ref" {
        return Err(ApiError::GenerationFailed(format!(
            "Workflow task path expected frame_ref artifact, got '{}'",
            artifact.artifact_type_id
        )));
    }
    let frame_id_hex = artifact
        .content
        .get("frame_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            ApiError::GenerationFailed(format!(
                "Workflow task path artifact '{}' is missing frame_id",
                artifact.artifact_id
            ))
        })?;
    decode_frame_id(frame_id_hex)
}

fn decode_frame_id(value: &str) -> Result<FrameID, ApiError> {
    let bytes = hex::decode(value).map_err(|err| {
        ApiError::GenerationFailed(format!("Invalid frame_ref artifact frame_id: {}", err))
    })?;
    let array: [u8; 32] = bytes.try_into().map_err(|_| {
        ApiError::GenerationFailed("Invalid frame_ref artifact frame_id length".to_string())
    })?;
    Ok(array)
}
