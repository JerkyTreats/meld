use crate::capability::CapabilityCatalog;
use crate::error::ExecutionInvariantError;
use crate::task::contracts::CompiledTaskRecord;
use crate::task::expansion::contracts::{CompiledTaskDelta, TaskExpansionRequest};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Constant used by the traversal prerequisite expansion kind execution contract.
pub const TRAVERSAL_PREREQUISITE_EXPANSION_KIND: &str = "traversal_prerequisite_expansion";
/// Constant used by the workspace write frame head expansion kind execution contract.
pub const WORKSPACE_WRITE_FRAME_HEAD_EXPANSION_KIND: &str = "workspace_write_frame_head_expansion";

/// Boundary trait for task expansion compiler adapters used by execution runtimes.
pub trait TaskExpansionCompiler: Send + Sync {
    /// Error type returned by the expansion compiler adapter.
    type Error;
    /// Execution API boundary supplied by the caller.
    type ExecutionApi: ?Sized;

    /// Returns the expansion kind handled by this compiler.
    fn expansion_kind(&self) -> &'static str;

    /// Compiles one expansion request into a task delta.
    fn compile(
        &self,
        api: &Self::ExecutionApi,
        compiled_task: &CompiledTaskRecord,
        expansion: &TaskExpansionRequest,
        catalog: &CapabilityCatalog,
    ) -> Result<CompiledTaskDelta, Self::Error>;
}

/// Task expansion compiler registry contract used by execution runtimes.
#[derive(Clone)]
pub struct TaskExpansionCompilerRegistry<E, A: ?Sized> {
    compilers: BTreeMap<String, Arc<dyn TaskExpansionCompiler<Error = E, ExecutionApi = A>>>,
}

impl<E, A: ?Sized> Default for TaskExpansionCompilerRegistry<E, A> {
    fn default() -> Self {
        Self {
            compilers: BTreeMap::new(),
        }
    }
}

impl<E, A: ?Sized> TaskExpansionCompilerRegistry<E, A> {
    /// Execution helper for new.
    pub fn new() -> Self {
        Self::default()
    }

    /// Execution helper for register.
    pub fn register<C>(&mut self, compiler: C) -> Result<(), ExecutionInvariantError>
    where
        C: TaskExpansionCompiler<Error = E, ExecutionApi = A> + 'static,
    {
        let expansion_kind = compiler.expansion_kind();
        if self.compilers.contains_key(expansion_kind) {
            return Err(ExecutionInvariantError::ConfigError(format!(
                "Task expansion compiler registry already contains '{}'",
                expansion_kind
            )));
        }
        self.compilers
            .insert(expansion_kind.to_string(), Arc::new(compiler));
        Ok(())
    }

    /// Execution helper for get.
    pub fn get(
        &self,
        expansion_kind: &str,
    ) -> Option<&Arc<dyn TaskExpansionCompiler<Error = E, ExecutionApi = A>>> {
        self.compilers.get(expansion_kind)
    }

    /// Execution helper for compile task expansion request.
    pub fn compile_task_expansion_request(
        &self,
        api: &A,
        compiled_task: &CompiledTaskRecord,
        expansion: &TaskExpansionRequest,
        catalog: &CapabilityCatalog,
    ) -> Result<CompiledTaskDelta, E>
    where
        E: From<ExecutionInvariantError>,
    {
        let compiler = self.get(&expansion.expansion_kind).ok_or_else(|| {
            E::from(ExecutionInvariantError::ConfigError(format!(
                "Task '{}' does not support expansion kind '{}'",
                compiled_task.task_id, expansion.expansion_kind
            )))
        })?;
        compiler.compile(api, compiled_task, expansion, catalog)
    }
}
