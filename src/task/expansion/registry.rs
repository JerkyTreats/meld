//! Task expansion dispatch into domain-owned implementations.

use crate::api::ContextApi;
use crate::capability::CapabilityCatalog;
use crate::error::ApiError;
use crate::merkle_traversal::expansion::compile_traversal_prerequisite_expansion;
use crate::task::contracts::CompiledTaskRecord;
use crate::task::expansion::contracts::{CompiledTaskDelta, TaskExpansionRequest};

pub const TRAVERSAL_PREREQUISITE_EXPANSION_KIND: &str = "traversal_prerequisite_expansion";

/// Compiles one task expansion request into an append-only task delta.
pub fn compile_task_expansion_request(
    api: &ContextApi,
    compiled_task: &CompiledTaskRecord,
    expansion: &TaskExpansionRequest,
    catalog: &CapabilityCatalog,
) -> Result<CompiledTaskDelta, ApiError> {
    match expansion.expansion_kind.as_str() {
        TRAVERSAL_PREREQUISITE_EXPANSION_KIND => {
            compile_traversal_prerequisite_expansion(api, compiled_task, expansion, catalog)
        }
        other => Err(ApiError::ConfigError(format!(
            "Task '{}' does not support expansion kind '{}'",
            compiled_task.task_id, other
        ))),
    }
}
