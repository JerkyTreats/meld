use crate::capability::CapabilityCatalog;
use crate::error::ExecutionInvariantError;
use crate::task::contracts::CompiledTaskRecord;
use crate::task::expansion::contracts::{CompiledTaskDelta, TaskExpansionRequest};
use std::collections::BTreeMap;
use std::sync::Arc;

pub const TRAVERSAL_PREREQUISITE_EXPANSION_KIND: &str = "traversal_prerequisite_expansion";
pub const WORKSPACE_WRITE_FRAME_HEAD_EXPANSION_KIND: &str = "workspace_write_frame_head_expansion";

pub trait TaskExpansionCompiler: Send + Sync {
    type Error;
    type ExecutionApi: ?Sized;

    fn expansion_kind(&self) -> &'static str;

    fn compile(
        &self,
        api: &Self::ExecutionApi,
        compiled_task: &CompiledTaskRecord,
        expansion: &TaskExpansionRequest,
        catalog: &CapabilityCatalog,
    ) -> Result<CompiledTaskDelta, Self::Error>;
}

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
    pub fn new() -> Self {
        Self::default()
    }

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

    pub fn get(
        &self,
        expansion_kind: &str,
    ) -> Option<&Arc<dyn TaskExpansionCompiler<Error = E, ExecutionApi = A>>> {
        self.compilers.get(expansion_kind)
    }

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
