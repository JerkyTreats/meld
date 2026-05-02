//! Workflow profile contracts.

pub mod events;
pub mod gates;
pub mod normalization;
pub mod profile;
pub mod progress;
pub mod registry;
pub mod resolver;

pub use events::{
    workflow_turn_completed_envelope, workflow_turn_failed_envelope,
    workflow_turn_started_envelope, ExecutionWorkflowTurnEventData,
};
pub use gates::{evaluate_gate, GateEvaluationResult, GateOutcome};
pub use normalization::normalize_output_for_gate;
pub use profile::{
    PromptRefKind, WorkflowArtifactPolicy, WorkflowFailurePolicy, WorkflowGate, WorkflowProfile,
    WorkflowThreadPolicy, WorkflowTurn,
};
pub use progress::{
    WorkflowExecutionRequest, WorkflowExecutionSummary, WorkflowForceResetProgressEventData,
    WorkflowTargetProgressEventData, WorkflowTurnProgressEventData,
};
pub use registry::RegisteredWorkflowProfile;
pub use resolver::{
    render_turn_prompt, resolve_prompt_template, resolve_turn_inputs, ResolvedTurnInputs,
};
