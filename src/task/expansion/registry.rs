use crate::capability::CapabilityCatalog;
use crate::error::ApiError;
use crate::execution::ExecutionRuntimeContext;
use crate::merkle_traversal::expansion::compile_traversal_prerequisite_expansion;
use crate::task::contracts::CompiledTaskRecord;
use crate::task::expansion::{CompiledTaskDelta, TaskExpansionRequest};
use crate::workspace::publish::compile_workspace_write_frame_head_expansion;
use meld_execution::task::expansion::{
    TaskExpansionCompiler, TaskExpansionCompilerRegistry, TRAVERSAL_PREREQUISITE_EXPANSION_KIND,
    WORKSPACE_WRITE_FRAME_HEAD_EXPANSION_KIND,
};

struct TraversalPrerequisiteExpansionCompiler;

impl TaskExpansionCompiler for TraversalPrerequisiteExpansionCompiler {
    type Error = ApiError;
    type ExecutionApi = dyn ExecutionRuntimeContext;

    fn expansion_kind(&self) -> &'static str {
        TRAVERSAL_PREREQUISITE_EXPANSION_KIND
    }

    fn compile(
        &self,
        api: &Self::ExecutionApi,
        compiled_task: &CompiledTaskRecord,
        expansion: &TaskExpansionRequest,
        catalog: &CapabilityCatalog,
    ) -> Result<CompiledTaskDelta, ApiError> {
        compile_traversal_prerequisite_expansion(api, compiled_task, expansion, catalog)
    }
}

struct WorkspaceWriteFrameHeadExpansionCompiler;

impl TaskExpansionCompiler for WorkspaceWriteFrameHeadExpansionCompiler {
    type Error = ApiError;
    type ExecutionApi = dyn ExecutionRuntimeContext;

    fn expansion_kind(&self) -> &'static str {
        WORKSPACE_WRITE_FRAME_HEAD_EXPANSION_KIND
    }

    fn compile(
        &self,
        _api: &Self::ExecutionApi,
        compiled_task: &CompiledTaskRecord,
        expansion: &TaskExpansionRequest,
        catalog: &CapabilityCatalog,
    ) -> Result<CompiledTaskDelta, ApiError> {
        compile_workspace_write_frame_head_expansion(compiled_task, expansion, catalog)
    }
}

pub fn register_default_task_expansion_compilers(
    registry: &mut TaskExpansionCompilerRegistry<ApiError, dyn ExecutionRuntimeContext>,
) -> Result<(), ApiError> {
    registry.register(TraversalPrerequisiteExpansionCompiler)?;
    registry.register(WorkspaceWriteFrameHeadExpansionCompiler)?;
    Ok(())
}

pub fn compile_task_expansion_request(
    api: &dyn ExecutionRuntimeContext,
    compiled_task: &CompiledTaskRecord,
    expansion: &TaskExpansionRequest,
    catalog: &CapabilityCatalog,
) -> Result<CompiledTaskDelta, ApiError> {
    match expansion.expansion_kind.as_str() {
        TRAVERSAL_PREREQUISITE_EXPANSION_KIND => {
            compile_traversal_prerequisite_expansion(api, compiled_task, expansion, catalog)
        }
        WORKSPACE_WRITE_FRAME_HEAD_EXPANSION_KIND => {
            compile_workspace_write_frame_head_expansion(compiled_task, expansion, catalog)
        }
        other => Err(ApiError::ConfigError(format!(
            "Task '{}' does not support expansion kind '{}'",
            compiled_task.task_id, other
        ))),
    }
}
