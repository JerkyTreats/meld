use crate::capability::{CapabilityCatalog, CapabilityExecutorRegistry};
use crate::error::ApiError;

pub struct WorkflowTaskPathRuntime {
    pub catalog: CapabilityCatalog,
    pub registry: CapabilityExecutorRegistry,
}

pub fn build_workflow_task_path_runtime() -> Result<WorkflowTaskPathRuntime, ApiError> {
    let mut catalog = CapabilityCatalog::new();
    let mut registry = CapabilityExecutorRegistry::new();

    registry.register(
        &mut catalog,
        crate::workspace::capability::WorkspaceResolveNodeIdCapability,
    )?;
    registry.register(
        &mut catalog,
        crate::workspace::capability::WorkspaceFilterFrameHeadPublishCapability,
    )?;
    registry.register(
        &mut catalog,
        crate::workspace::capability::WorkspaceWriteFrameHeadCapability,
    )?;
    registry.register(
        &mut catalog,
        crate::merkle_traversal::capability::MerkleTraversalCapability,
    )?;
    registry.register(
        &mut catalog,
        crate::context::capability::ContextGeneratePrepareCapability,
    )?;
    registry.register(
        &mut catalog,
        crate::provider::capability::ProviderExecuteChatCapability,
    )?;
    registry.register(
        &mut catalog,
        crate::context::capability::ContextGenerateFinalizeCapability,
    )?;

    Ok(WorkflowTaskPathRuntime { catalog, registry })
}
