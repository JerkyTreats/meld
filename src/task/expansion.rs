//! Task expansion contracts, dispatch, and runtime helpers.

pub mod contracts;
pub mod registry;
pub mod runtime;

pub use contracts::{
    CompiledTaskDelta, TaskExpansionRecord, TaskExpansionRequest, TaskExpansionTemplate,
    TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID, TASK_EXPANSION_SCHEMA_VERSION,
    TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID,
};
pub use registry::{
    compile_task_expansion_request, TRAVERSAL_PREREQUISITE_EXPANSION_KIND,
    WORKSPACE_WRITE_FRAME_HEAD_EXPANSION_KIND,
};
pub use runtime::parse_task_expansion_request_artifact;
