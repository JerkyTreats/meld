/// Task-owned expansion request, record, and delta contracts.
pub mod contracts;
/// Expansion compiler registry keyed by expansion kind.
pub mod registry;
/// Runtime helpers for parsing expansion artifacts.
pub mod runtime;

pub use contracts::{
    CompiledTaskDelta, TaskExpansionRecord, TaskExpansionRequest, TaskExpansionTemplate,
    TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID, TASK_EXPANSION_SCHEMA_VERSION,
    TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID,
};
pub use registry::{TaskExpansionCompiler, TaskExpansionCompilerRegistry};
pub use runtime::parse_task_expansion_request_artifact;
