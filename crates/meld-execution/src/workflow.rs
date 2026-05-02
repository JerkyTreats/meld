//! Workflow profile contracts.

pub mod events;
pub mod executor;
pub mod gates;
pub mod normalization;
pub mod profile;
pub mod progress;
pub mod record_contracts;
pub mod registry;
pub mod resolver;
pub mod state_store;

pub use events::{
    workflow_turn_completed_envelope, workflow_turn_failed_envelope,
    workflow_turn_started_envelope, ExecutionWorkflowTurnEventData,
};
pub use executor::{
    execute_registered_workflow_async, FrameBuilder, NodeNotFoundBuilder, WorkflowExecutorContext,
    WorkflowExecutorRuntime, WorkflowTaskPathExecution, WorkflowTaskPathExecutor,
};
pub use gates::{evaluate_gate, GateEvaluationResult};
pub use normalization::normalize_output_for_gate;
pub use profile::{
    PromptRefKind, WorkflowArtifactPolicy, WorkflowFailurePolicy, WorkflowGate, WorkflowProfile,
    WorkflowThreadPolicy, WorkflowTurn,
};
pub use progress::{
    workflow_thread_id, workflow_turn_frame_type, WorkflowExecutionRequest,
    WorkflowExecutionSummary, WorkflowForceResetProgressEventData, WorkflowTargetProgressEventData,
    WorkflowTurnProgressEventData,
};
pub use record_contracts::{
    prompt_link_record_from_contract_v1, validate_prompt_link_record_references,
    validate_prompt_link_record_v1, validate_thread_turn_gate_record_references,
    validate_thread_turn_gate_record_v1, GateOutcome, PromptLinkRecordInputV1, PromptLinkRecordV1,
    ThreadTurnGateRecordV1, WORKFLOW_RECORD_SCHEMA_VERSION_V1,
};
pub use registry::RegisteredWorkflowProfile;
pub use resolver::{
    render_turn_prompt, resolve_prompt_template, resolve_turn_inputs, ResolvedTurnInputs,
};
pub use state_store::{
    WorkflowStateStore, WorkflowThreadRecord, WorkflowThreadStatus, WorkflowTurnRecord,
    WorkflowTurnStatus,
};
