//! Execution-owned planning contracts and first-slice runtime.
//!
//! Planning evaluates shared-language goals mechanically and returns an
//! execution composition artifact. Task lowering and dispatch stay outside this
//! domain.

/// Durable append-only planning attempt audit authority.
pub mod attempts;
/// Planning request, result, diagnostic, and composition contracts.
pub mod contracts;
/// Lowering from execution compositions into task network mutation proposals.
pub mod lowering;
/// Method loading, verification, and deterministic ordering.
pub mod method_library;
/// First-slice planning runtime and bounded actor facade.
pub mod runtime;
/// Execution-side world state projection request contracts.
pub mod world_state;

pub use attempts::{
    planning_task_network_command_id, PlanningAttemptCommandOutcome, PlanningAttemptContinuation,
    PlanningAttemptDecisionAudit, PlanningAttemptDiagnosticDisposition, PlanningAttemptErrorClass,
    PlanningAttemptHead, PlanningAttemptHistory, PlanningAttemptIdentity,
    PlanningAttemptOwnerFence, PlanningAttemptQuery, PlanningAttemptRecord,
    PlanningAttemptRecordKind, PlanningAttemptRecovery, PlanningAttemptRecoverySelection,
    PlanningAttemptResultSummary, PlanningAttemptSelection, PlanningAttemptState,
    PlanningAttemptStorageError, PlanningAttemptStore, PlanningAttemptTerminalDiagnostic,
    PlanningPreparedCommand, PlanningProjectionFailureIdentityInputs,
    MAX_PLANNING_ATTEMPT_QUERY_ITEMS, PLANNING_ATTEMPT_DECISION_SCHEMA_VERSION,
    PLANNING_ATTEMPT_SCHEMA_VERSION,
};
pub use contracts::{
    CandidateStatus, ExecutionComposition, IdentifiedPlanningRequest, InvalidMethodReport,
    MethodCandidateReport, NoApplicableMethod, OperatorResolutionReport, OperatorResolutionStatus,
    PlanningDiagnostic, PlanningDiagnosticCode, PlanningIndeterminate, PlanningInputError,
    PlanningRequest, PlanningRequestIdentity, PlanningRequestIdentityInputs, PlanningResult,
    PlanningSatisfied,
};
pub use lowering::{
    Diagnostic as CompositionLoweringDiagnostic,
    DiagnosticCode as CompositionLoweringDiagnosticCode, Lowerer as ExecutionCompositionLowerer,
    Plan as CompositionLoweringPlan, Request as CompositionLoweringRequest,
};
pub use method_library::{
    MethodLibrary, MethodLibraryLoadError, MethodSourceRef, MethodVerification,
    MethodVerificationDiagnostic, VerifiedMethodEntry,
};
pub use runtime::{
    PlanningProjectionError, PlanningProjectionPort, PlanningRuntime, PlanningRuntimeActor,
    PlanningRuntimeActorError, PlanningRuntimeActorGoalResult, PlanningRuntimeActorIssue,
    PlanningRuntimeActorReport, PlanningRuntimeActorRequest, PlanningWorldStateProjection,
};
pub use world_state::{
    PlanningPerspectiveRef, PlanningProjectionIdentityInputs, PlanningWorldStateFrameRef,
    PlanningWorldStateRequest,
};
