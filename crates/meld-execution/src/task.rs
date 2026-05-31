//! Task contracts and initialization payloads.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::capability::{
//!     ArtifactSchemaVersionRange, InputCardinality, InputSlotSpec,
//! };
//! use meld_execution::task::{
//!     artifact_matches_input_slot, ArtifactProducerRef, ArtifactRecord,
//! };
//! use serde_json::json;
//!
//! let slot = InputSlotSpec {
//!     slot_id: "provider_request".to_string(),
//!     accepted_artifact_type_ids: vec!["provider_execute_request".to_string()],
//!     schema_versions: ArtifactSchemaVersionRange { min: 1, max: 2 },
//!     required: true,
//!     cardinality: InputCardinality::One,
//! };
//!
//! let artifact = ArtifactRecord {
//!     artifact_id: "artifact-a".to_string(),
//!     artifact_type_id: "provider_execute_request".to_string(),
//!     schema_version: 1,
//!     content: json!({
//!         "prompt": "Summarize the selected frame"
//!     }),
//!     producer: ArtifactProducerRef {
//!         task_id: "task-a".to_string(),
//!         capability_instance_id: "prepare-request".to_string(),
//!         invocation_id: Some("invoke-a".to_string()),
//!         output_slot_id: Some("request".to_string()),
//!     },
//! };
//!
//! assert!(artifact_matches_input_slot(&artifact, &slot));
//! ```

pub mod artifact_repo;
pub mod compiler;
pub mod contracts;
pub mod events;
pub mod executor;
pub mod expansion;
pub mod init;
pub mod invocation;
pub mod package;
pub mod readiness;
pub mod runtime;
pub mod templates;

pub use artifact_repo::TaskArtifactRepo;
pub use compiler::{compile_task_definition, TaskCompiler};
pub use contracts::{
    artifact_matches_input_slot, ArtifactLinkRecord, ArtifactLinkRelation, ArtifactProducerRef,
    ArtifactRecord, ArtifactRepoRecord, CapabilityInvocationRecord, CompiledTaskRecord,
    TaskDefinition, TaskDependencyEdge, TaskDependencyKind, TaskInitSlotSpec,
};
pub use events::{
    build_execution_task_envelope, canonical_task_event_type, ExecutionTaskEventData, TaskEvent,
};
pub use executor::TaskExecutor;
pub use expansion::{
    parse_task_expansion_request_artifact, CompiledTaskDelta, TaskExpansionRecord,
    TaskExpansionRequest, TaskExpansionTemplate, TASK_EXPANSION_REQUEST_ARTIFACT_TYPE_ID,
    TASK_EXPANSION_SCHEMA_VERSION, TASK_EXPANSION_TEMPLATE_ARTIFACT_TYPE_ID,
};
pub use init::{
    validate_task_initialization, InitArtifactValue, TaskInitializationPayload, TaskRunContext,
};
pub use invocation::assemble_invocation_payload;
pub use readiness::compute_ready_capability_instances;
pub use runtime::{execute_task_to_completion, TaskRunSummary, WorkflowTaskTelemetry};
pub use templates::{
    prepare_registered_workflow_task_run, workflow_task_run_id_for_target,
    workflow_uses_task_package_path,
};
