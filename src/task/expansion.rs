//! Root task expansion adapters and compatibility exports.

pub mod contracts;
pub mod registry;
pub mod runtime;

pub use contracts::{
    CompiledTaskDelta, TaskExpansionRecord, TaskExpansionRequest, TaskExpansionTemplate,
    TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID, TASK_EXPANSION_SCHEMA_VERSION,
    TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID,
};
pub use meld_execution::task::expansion::{TaskExpansionCompiler, TaskExpansionCompilerRegistry};
pub use registry::compile_task_expansion_request;
pub use runtime::parse_task_expansion_request_artifact;
