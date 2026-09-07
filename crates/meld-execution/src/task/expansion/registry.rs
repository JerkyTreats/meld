use crate::capability::CapabilityCatalog;
use crate::error::ExecutionInvariantError;
use crate::task::contracts::CompiledTaskRecord;
use crate::task::expansion::contracts::{CompiledTaskDelta, TaskExpansionRequest};
use std::collections::BTreeMap;
use std::sync::Arc;

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
    /// Creates an empty task expansion compiler registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one expansion compiler by its expansion kind.
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

    /// Returns the compiler registered for an expansion kind.
    pub fn get(
        &self,
        expansion_kind: &str,
    ) -> Option<&Arc<dyn TaskExpansionCompiler<Error = E, ExecutionApi = A>>> {
        self.compilers.get(expansion_kind)
    }

    /// Compiles one task expansion request with the matching compiler.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::contracts::TaskInitSlotSpec;

    struct FakeExpansionCompiler;

    impl TaskExpansionCompiler for FakeExpansionCompiler {
        type Error = ExecutionInvariantError;
        type ExecutionApi = ();

        fn expansion_kind(&self) -> &'static str {
            "fake_expansion"
        }

        fn compile(
            &self,
            _api: &Self::ExecutionApi,
            _compiled_task: &CompiledTaskRecord,
            _expansion: &TaskExpansionRequest,
            _catalog: &CapabilityCatalog,
        ) -> Result<CompiledTaskDelta, Self::Error> {
            Ok(CompiledTaskDelta {
                init_slots: vec![TaskInitSlotSpec {
                    init_slot_id: "expanded_init".to_string(),
                    artifact_type_id: "expanded_artifact".to_string(),
                    schema_version: 1,
                    required: true,
                }],
                ..CompiledTaskDelta::default()
            })
        }
    }

    fn compiled_task() -> CompiledTaskRecord {
        CompiledTaskRecord {
            task_id: "task-docs".to_string(),
            task_version: 1,
            init_slots: vec![],
            capability_instances: vec![],
            dependency_edges: vec![],
        }
    }

    fn expansion(expansion_kind: &str) -> TaskExpansionRequest {
        TaskExpansionRequest {
            expansion_id: "expansion-1".to_string(),
            expansion_kind: expansion_kind.to_string(),
            content: serde_json::json!({}),
        }
    }

    #[test]
    fn registry_registers_compiler_and_rejects_duplicate_kind() {
        let mut registry = TaskExpansionCompilerRegistry::<ExecutionInvariantError, ()>::new();

        assert!(registry.get("fake_expansion").is_none());

        registry.register(FakeExpansionCompiler).unwrap();

        assert!(registry.get("fake_expansion").is_some());
        assert!(matches!(
            registry.register(FakeExpansionCompiler),
            Err(ExecutionInvariantError::ConfigError(_))
        ));
    }

    #[test]
    fn registry_dispatches_compile_request_to_matching_compiler() {
        let mut registry = TaskExpansionCompilerRegistry::<ExecutionInvariantError, ()>::new();
        registry.register(FakeExpansionCompiler).unwrap();

        let delta = registry
            .compile_task_expansion_request(
                &(),
                &compiled_task(),
                &expansion("fake_expansion"),
                &CapabilityCatalog::new(),
            )
            .unwrap();

        assert_eq!(delta.init_slots.len(), 1);
        assert_eq!(delta.init_slots[0].init_slot_id, "expanded_init");
        assert!(matches!(
            registry.compile_task_expansion_request(
                &(),
                &compiled_task(),
                &expansion("missing_expansion"),
                &CapabilityCatalog::new(),
            ),
            Err(ExecutionInvariantError::ConfigError(_))
        ));
    }
}
