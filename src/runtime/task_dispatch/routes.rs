//! Concrete execution routes for the first docs freshness flywheel.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use meld_execution::activation::{
    ExecutionForcePolicy, ExecutionTargetKind, ExecutionTargetSelector,
};
use meld_execution::capability::{
    BoundBindingValue, BoundCapabilityInstance, CapabilityInvocationPayload,
    CapabilityInvocationResult, InputValueSource, SuppliedInputValue, SuppliedValueRef,
};
use meld_execution::error::ApiError;
use meld_execution::task::package::{map_task_package_output_artifact, TaskPackageSpec};
use meld_execution::task::{ArtifactProducerRef, ArtifactRecord};

use crate::api::ContextApi;
use crate::provider::{ProviderExecutionBinding, ProviderRuntimeOverrides};
use crate::task::{
    execute_task_to_completion, prepare_registered_workflow_task_run, TaskExecutor,
    WorkflowPackageTriggerRequest,
};
use crate::workflow::registry::RegisteredWorkflowProfile;
use crate::workflow::task_path::build_workflow_task_path_runtime;

use super::{TaskDispatchRouteRequest, TaskDispatchRoutes};

const WORKSPACE_SCAN_CAPABILITY_TYPE_ID: &str = "workspace_scan";
const WORKSPACE_SNAPSHOT_ARTIFACT_TYPE_ID: &str = "workspace_snapshot_ref";
const DOCS_PATCH_ARTIFACT_TYPE_ID: &str = "docs_patch";
const README_OUTPUT_TYPE_ID: &str = "readme_final";

/// Exact activation values consumed by the concrete dispatch routes.
#[derive(Clone)]
pub struct DocsFreshnessDispatchConfig {
    /// Repository workspace inspected and updated by the route.
    pub workspace_root: PathBuf,
    /// Activated workflow profile used by the docs writer package.
    pub workflow: RegisteredWorkflowProfile,
    /// Exact activated task package whose output mapping is authoritative.
    pub task_package: TaskPackageSpec,
    /// Agent authorized to persist generated documentation frames.
    pub agent_id: String,
    /// Repository provider binding selected during activation.
    pub provider_binding_ref: String,
    /// Frame type selected during activation.
    pub frame_type: String,
    /// Existing output posture selected during activation.
    pub force_policy: ExecutionForcePolicy,
    /// Canonical target selected during activation.
    pub target: ExecutionTargetSelector,
}

/// Real workspace scan and docs writer routes over one root execution API.
#[derive(Clone)]
pub struct DocsFreshnessDispatchRoutes {
    api: Arc<ContextApi>,
    config: DocsFreshnessDispatchConfig,
}

impl DocsFreshnessDispatchRoutes {
    /// Bind the activated route values to the already-open root execution API.
    pub fn new(api: Arc<ContextApi>, config: DocsFreshnessDispatchConfig) -> Self {
        Self { api, config }
    }

    fn provider_binding(&self) -> Result<ProviderExecutionBinding, ApiError> {
        ProviderExecutionBinding::new(
            self.config.provider_binding_ref.clone(),
            ProviderRuntimeOverrides::default(),
        )
    }

    fn workflow_target(&self) -> Result<(Option<[u8; 32]>, Option<PathBuf>), ApiError> {
        match self.config.target.kind {
            ExecutionTargetKind::Path => Ok((
                None,
                Some(PathBuf::from(&self.config.target.canonical_value)),
            )),
            ExecutionTargetKind::NodeId => {
                let bytes =
                    hex::decode(self.config.target.canonical_value.trim_start_matches("0x"))
                        .map_err(|error| {
                            ApiError::ConfigError(format!(
                                "activated docs target node id is not hex: {error}"
                            ))
                        })?;
                let node_id: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
                    ApiError::ConfigError(format!(
                        "activated docs target node id has {} bytes, expected 32",
                        bytes.len()
                    ))
                })?;
                Ok((Some(node_id), None))
            }
        }
    }
}

impl TaskDispatchRoutes for DocsFreshnessDispatchRoutes {
    fn dispatch_workspace_scan(
        &self,
        request: TaskDispatchRouteRequest<'_>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        let runtime = build_workflow_task_path_runtime().map_err(route_error)?;
        let session_id = request
            .node
            .task_run_context
            .session_id
            .clone()
            .unwrap_or_else(|| request.claim.claim_id.clone());
        let instance = BoundCapabilityInstance {
            capability_instance_id: request.invocation.capability_instance_id.clone(),
            capability_type_id: WORKSPACE_SCAN_CAPABILITY_TYPE_ID.to_string(),
            capability_version: 1,
            scope_ref: self.config.workspace_root.to_string_lossy().into_owned(),
            scope_kind: "workspace".to_string(),
            binding_values: vec![BoundBindingValue {
                binding_id: "session_id".to_string(),
                value: serde_json::json!(session_id),
            }],
            input_wiring: Vec::new(),
        };
        let runtime_init = runtime.registry.runtime_init_for(&instance)?;
        let invoker = runtime
            .registry
            .get(WORKSPACE_SCAN_CAPABILITY_TYPE_ID, 1)
            .cloned()
            .ok_or_else(|| {
                ApiError::ConfigError(
                    "workspace scan route is missing its registered invoker".to_string(),
                )
            })?;
        let selector = match self.config.target.kind {
            ExecutionTargetKind::Path => serde_json::json!({
                "path": self.config.target.canonical_value,
            }),
            ExecutionTargetKind::NodeId => serde_json::json!({
                "node_id": self.config.target.canonical_value,
            }),
        };
        let payload = CapabilityInvocationPayload {
            invocation_id: request.invocation.invocation_id.clone(),
            capability_instance_id: request.invocation.capability_instance_id.clone(),
            supplied_inputs: vec![SuppliedInputValue {
                slot_id: "target_selector".to_string(),
                source: InputValueSource::InitPayload,
                value: SuppliedValueRef::StructuredValue(selector),
            }],
            upstream_lineage: request.invocation.upstream_lineage.clone(),
            execution_context: request.invocation.execution_context.clone(),
        };
        let result = block_on(invoker.invoke(self.api.as_ref(), &runtime_init, &payload, None))?
            .map_err(route_error)?;
        let emitted_artifacts = result
            .emitted_artifacts
            .into_iter()
            .filter(|artifact| artifact.artifact_type_id == WORKSPACE_SNAPSHOT_ARTIFACT_TYPE_ID)
            .collect::<Vec<_>>();
        if emitted_artifacts.len() != 1 {
            return Err(ApiError::ConfigError(format!(
                "workspace scan route produced {} workspace snapshot artifacts",
                emitted_artifacts.len()
            )));
        }
        Ok(CapabilityInvocationResult { emitted_artifacts })
    }

    fn dispatch_docs_writer(
        &self,
        request: TaskDispatchRouteRequest<'_>,
    ) -> Result<CapabilityInvocationResult, ApiError> {
        let snapshot_inputs = request
            .invocation
            .supplied_inputs
            .iter()
            .filter(|input| {
                matches!(
                    &input.value,
                    SuppliedValueRef::Artifact(artifact)
                        if artifact.artifact_type_id == WORKSPACE_SNAPSHOT_ARTIFACT_TYPE_ID
                            && artifact.schema_version == 1
                )
            })
            .count();
        if snapshot_inputs != 1 {
            return Err(ApiError::ConfigError(format!(
                "docs writer route received {snapshot_inputs} workspace snapshot inputs"
            )));
        }
        let runtime = build_workflow_task_path_runtime().map_err(route_error)?;
        let (node_id, path) = self.workflow_target()?;
        let prepared = prepare_registered_workflow_task_run(
            self.api.as_ref(),
            &self.config.workspace_root,
            &self.config.workflow,
            &WorkflowPackageTriggerRequest {
                package_id: self.config.task_package.package_id.clone(),
                workflow_id: self.config.workflow.profile.workflow_id.clone(),
                node_id,
                path,
                agent_id: self.config.agent_id.clone(),
                provider: self.provider_binding()?,
                frame_type: self.config.frame_type.clone(),
                force: matches!(
                    self.config.force_policy,
                    ExecutionForcePolicy::ReplaceExisting
                ),
                session_id: request.node.task_run_context.session_id.clone(),
            },
            &runtime.catalog,
        )
        .map_err(route_error)?;
        let target_node_id = prepared.target_node_id;
        let mut executor = TaskExecutor::new(
            prepared.compiled_task,
            prepared.init_payload,
            format!("inner-docs::{}", request.claim.claim_id),
        )?;
        block_on(execute_task_to_completion(
            self.api.as_ref(),
            &mut executor,
            &runtime.catalog,
            &runtime.registry,
            None,
            None,
        ))?
        .map_err(route_error)?;

        let target_node_hex = hex::encode(target_node_id);
        let target_producers = executor
            .artifact_repo()
            .record()
            .artifacts
            .iter()
            .filter(|artifact| {
                artifact.artifact_type_id == "frame_ref"
                    && artifact
                        .content
                        .get("node_id")
                        .and_then(serde_json::Value::as_str)
                        == Some(target_node_hex.as_str())
            })
            .map(|artifact| artifact.producer.capability_instance_id.clone())
            .collect::<Vec<_>>();
        if target_producers.len() != 1 {
            return Err(ApiError::ConfigError(format!(
                "docs writer route produced {} target frame references",
                target_producers.len()
            )));
        }
        let target_producer = &target_producers[0];
        let mut outputs = executor
            .artifact_repo()
            .record()
            .artifacts
            .iter()
            .filter(|artifact| {
                artifact.artifact_type_id == README_OUTPUT_TYPE_ID
                    && &artifact.producer.capability_instance_id == target_producer
            })
            .map(|artifact| {
                let content = artifact
                    .content
                    .get("content")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        ApiError::ConfigError(format!(
                            "docs writer artifact '{}' has no markdown content",
                            artifact.artifact_id
                        ))
                    })?;
                Ok((artifact.artifact_id.clone(), content.to_string()))
            })
            .collect::<Result<Vec<_>, ApiError>>()?;
        outputs.sort_by(|left, right| left.0.cmp(&right.0));
        if outputs.len() != 1 {
            return Err(ApiError::ConfigError(format!(
                "docs writer route produced {} target readme outputs",
                outputs.len()
            )));
        }
        let primary_content = outputs[0].1.clone();
        let mapped = map_task_package_output_artifact(
            &self.config.task_package,
            DOCS_PATCH_ARTIFACT_TYPE_ID,
            1,
            &BTreeMap::from([(README_OUTPUT_TYPE_ID.to_string(), primary_content)]),
        )
        .map_err(|error| ApiError::ConfigError(error.to_string()))?;
        let producer = ArtifactProducerRef {
            task_id: request.node.compiled_task.task_id.clone(),
            capability_instance_id: request.invocation.capability_instance_id.clone(),
            invocation_id: Some(request.invocation.invocation_id.clone()),
            output_slot_id: Some(DOCS_PATCH_ARTIFACT_TYPE_ID.to_string()),
        };
        Ok(CapabilityInvocationResult {
            emitted_artifacts: vec![ArtifactRecord {
                artifact_id: format!(
                    "{}::{DOCS_PATCH_ARTIFACT_TYPE_ID}",
                    request.invocation.invocation_id
                ),
                artifact_type_id: mapped.artifact_type_id,
                schema_version: mapped.schema_version,
                content: serde_json::json!({
                    "content": mapped.content,
                    "source_output_type": mapped.source_output_type,
                    "output_count": outputs.len(),
                    "outputs": outputs,
                }),
                producer,
            }],
        })
    }
}

fn block_on<F>(future: F) -> Result<F::Output, ApiError>
where
    F: std::future::Future,
{
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|error| ApiError::ConfigError(format!("task route runtime failed: {error}")))?;
    Ok(runtime.block_on(future))
}

fn route_error(error: crate::error::ApiError) -> ApiError {
    ApiError::ConfigError(error.to_string())
}
