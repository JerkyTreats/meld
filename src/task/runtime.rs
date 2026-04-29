use crate::capability::{
    BoundCapabilityInstance, CapabilityCatalog, CapabilityExecutorRegistry,
    CapabilityInvocationPayload, CapabilityInvocationResult,
};
use crate::error::ApiError;
use crate::execution::{ExecutionEventContext, ExecutionRuntimeContext};
use futures::future::BoxFuture;

pub use meld_execution::task::runtime::{TaskRunSummary, WorkflowTaskTelemetry};

pub async fn execute_task_to_completion(
    api: &dyn ExecutionRuntimeContext,
    executor: &mut crate::task::TaskExecutor,
    catalog: &CapabilityCatalog,
    registry: &CapabilityExecutorRegistry,
    event_context: Option<&ExecutionEventContext>,
    workflow_telemetry: Option<&WorkflowTaskTelemetry>,
) -> Result<TaskRunSummary, ApiError> {
    meld_execution::task::execute_task_to_completion(
        api,
        executor,
        catalog,
        registry,
        |api, registry, instance, payload, event_context| {
            invoke_registered_capability(api, registry, instance, payload, event_context)
        },
        |api, compiled_task, expansion_request, catalog| {
            crate::task::expansion::compile_task_expansion_request(
                api,
                compiled_task,
                expansion_request,
                catalog,
            )
        },
        event_context,
        workflow_telemetry,
    )
    .await
}

fn invoke_registered_capability<'a>(
    api: &'a dyn ExecutionRuntimeContext,
    registry: &'a CapabilityExecutorRegistry,
    instance: &'a BoundCapabilityInstance,
    payload: &'a CapabilityInvocationPayload,
    event_context: Option<&'a ExecutionEventContext>,
) -> BoxFuture<'a, Result<CapabilityInvocationResult, ApiError>> {
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
