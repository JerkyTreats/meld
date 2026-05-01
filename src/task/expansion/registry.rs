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
use std::marker::PhantomData;

struct TraversalPrerequisiteExpansionCompiler<A: ?Sized>(PhantomData<fn() -> A>);

impl<A> TaskExpansionCompiler for TraversalPrerequisiteExpansionCompiler<A>
where
    A: ExecutionRuntimeContext + ?Sized,
{
    type Error = ApiError;
    type ExecutionApi = A;

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

struct WorkspaceWriteFrameHeadExpansionCompiler<A: ?Sized>(PhantomData<fn() -> A>);

impl<A> TaskExpansionCompiler for WorkspaceWriteFrameHeadExpansionCompiler<A>
where
    A: ExecutionRuntimeContext + ?Sized,
{
    type Error = ApiError;
    type ExecutionApi = A;

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

pub fn register_default_task_expansion_compilers<A>(
    registry: &mut TaskExpansionCompilerRegistry<ApiError, A>,
) -> Result<(), ApiError>
where
    A: ExecutionRuntimeContext + 'static,
{
    registry.register(TraversalPrerequisiteExpansionCompiler(PhantomData))?;
    registry.register(WorkspaceWriteFrameHeadExpansionCompiler(PhantomData))?;
    Ok(())
}

pub fn compile_task_expansion_request<A>(
    api: &A,
    compiled_task: &CompiledTaskRecord,
    expansion: &TaskExpansionRequest,
    catalog: &CapabilityCatalog,
) -> Result<CompiledTaskDelta, ApiError>
where
    A: ExecutionRuntimeContext + 'static,
{
    let mut registry = TaskExpansionCompilerRegistry::new();
    register_default_task_expansion_compilers(&mut registry)?;
    registry.compile_task_expansion_request(api, compiled_task, expansion, catalog)
}
