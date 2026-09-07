//! Native composition has no installed dynamic expansion owner.

use crate::capability::CapabilityCatalog;
use crate::error::ApiError;
use crate::execution::ExecutionRuntimeContext;
use crate::task::contracts::CompiledTaskRecord;
use crate::task::expansion::{CompiledTaskDelta, TaskExpansionRequest};
use meld_execution::task::expansion::TaskExpansionCompilerRegistry;

pub fn compile_task_expansion_request<A>(
    api: &A,
    compiled_task: &CompiledTaskRecord,
    expansion: &TaskExpansionRequest,
    catalog: &CapabilityCatalog,
) -> Result<CompiledTaskDelta, ApiError>
where
    A: ExecutionRuntimeContext + 'static,
{
    let registry = TaskExpansionCompilerRegistry::<ApiError, A>::new();
    registry.compile_task_expansion_request(api, compiled_task, expansion, catalog)
}
