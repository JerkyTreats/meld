//! Root task runtime adapters over extracted execution behavior.

use crate::capability::{
    BoundCapabilityInstance, CapabilityCatalog, CapabilityExecutorRegistry,
    CapabilityInvocationPayload, CapabilityInvocationResult,
};
use crate::error::ApiError;
use crate::execution::{ExecutionEventContext, ExecutionRuntimeContext};
use crate::task::contracts::CompiledTaskRecord;
use crate::task::expansion::{CompiledTaskDelta, TaskExpansionRequest};
use futures::future::BoxFuture;

pub use meld_execution::task::runtime::{TaskRunSummary, WorkflowTaskTelemetry};

pub async fn execute_task_to_completion<A>(
    api: &A,
    executor: &mut crate::task::TaskExecutor,
    catalog: &CapabilityCatalog,
    registry: &CapabilityExecutorRegistry,
    event_context: Option<&ExecutionEventContext>,
    workflow_telemetry: Option<&WorkflowTaskTelemetry>,
) -> Result<TaskRunSummary, ApiError>
where
    A: ExecutionRuntimeContext + 'static,
{
    meld_execution::task::execute_task_to_completion(
        api,
        executor,
        catalog,
        registry,
        |api, registry, instance, payload, event_context| {
            invoke_capability_via_root_registry(api, registry, instance, payload, event_context)
        },
        |api, compiled_task, expansion_request, catalog| {
            compile_expansion_via_root_registry(api, compiled_task, expansion_request, catalog)
        },
        event_context,
        workflow_telemetry,
    )
    .await
}

fn compile_expansion_via_root_registry<A>(
    api: &A,
    compiled_task: &CompiledTaskRecord,
    expansion_request: &TaskExpansionRequest,
    catalog: &CapabilityCatalog,
) -> Result<CompiledTaskDelta, ApiError>
where
    A: ExecutionRuntimeContext + 'static,
{
    crate::task::expansion::compile_task_expansion_request(
        api,
        compiled_task,
        expansion_request,
        catalog,
    )
}

fn invoke_capability_via_root_registry<'a, A>(
    api: &'a A,
    registry: &'a CapabilityExecutorRegistry,
    instance: &'a BoundCapabilityInstance,
    payload: &'a CapabilityInvocationPayload,
    event_context: Option<&'a ExecutionEventContext>,
) -> BoxFuture<'a, Result<CapabilityInvocationResult, ApiError>>
where
    A: ExecutionRuntimeContext + 'static,
{
    Box::pin(async move {
        let runtime_init = registry.runtime_init_for(instance)?;
        let invoker = registry
            .get(&instance.capability_type_id, instance.capability_version)
            .cloned()
            .ok_or_else(|| {
                ApiError::ConfigError(format!(
                    "Task '{}' is missing invoker for '{}' version '{}'",
                    payload
                        .upstream_lineage
                        .as_ref()
                        .map(|lineage| lineage.task_id.clone())
                        .unwrap_or_default(),
                    instance.capability_type_id,
                    instance.capability_version
                ))
            })?;
        invoker
            .invoke(api, &runtime_init, payload, event_context)
            .await
    })
}
