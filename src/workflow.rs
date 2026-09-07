//! Legacy Workflow inspection and historical contracts; execution is retired.

pub mod commands;
pub mod events;
pub mod gates;
pub mod normalization;
pub mod profile;
pub mod record_contracts;
pub mod registry;
pub mod resolver;
pub mod summary;
pub mod tooling;

pub use commands::{
    WorkflowCommandService, WorkflowInspectResult, WorkflowListItem, WorkflowListResult,
    WorkflowValidateResult,
};
pub use meld_execution::workflow::profile::{
    PromptRefKind, WorkflowArtifactPolicy, WorkflowFailurePolicy, WorkflowGate, WorkflowProfile,
    WorkflowThreadPolicy, WorkflowTurn,
};
pub use meld_execution::workflow::registry::RegisteredWorkflowProfile;
pub use meld_execution::workflow::{
    workflow_turn_completed_envelope, workflow_turn_failed_envelope,
    workflow_turn_started_envelope, ExecutionWorkflowTurnEventData,
};
pub use registry::WorkflowRegistry;
/// Reject old execution addresses while profiles remain readable for migration.
/// Remove this rejection boundary when persisted callers no longer name Workflow.
pub fn retired_execution_error() -> crate::error::ApiError {
    crate::error::ApiError::ConfigError(
        "Legacy Workflow execution is retired. Install a native reconciliation product and use meld runtime run or meld runtime request; Legacy commands can inspect profiles but cannot execute them.".into(),
    )
}
