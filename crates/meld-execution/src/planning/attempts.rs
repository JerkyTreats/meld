//! Append-only audit and recovery authority for execution planning attempts.
//!
//! Attempt records explain deterministic planning work and protect command
//! submission with a durable prepared-command barrier. They do not own an
//! executable plan. Accepted task-network commands and their journal remain
//! the execution commitment authority.

mod contracts;
mod query;
mod store;

pub use contracts::{
    planning_task_network_command_id, PlanningAttemptCommandOutcome, PlanningAttemptContinuation,
    PlanningAttemptDecisionAudit, PlanningAttemptDiagnosticDisposition, PlanningAttemptHead,
    PlanningAttemptHistory, PlanningAttemptIdentity, PlanningAttemptOwnerFence,
    PlanningAttemptRecord, PlanningAttemptRecordKind, PlanningAttemptRecovery,
    PlanningAttemptRecoverySelection, PlanningAttemptResultSummary, PlanningAttemptSelection,
    PlanningAttemptState, PlanningAttemptTerminalDiagnostic, PlanningPreparedCommand,
    PlanningProjectionFailureIdentityInputs, MAX_PLANNING_ATTEMPT_QUERY_ITEMS,
    PLANNING_ATTEMPT_DECISION_SCHEMA_VERSION, PLANNING_ATTEMPT_SCHEMA_VERSION,
};
pub use query::PlanningAttemptQuery;
pub use store::{PlanningAttemptErrorClass, PlanningAttemptStorageError, PlanningAttemptStore};
