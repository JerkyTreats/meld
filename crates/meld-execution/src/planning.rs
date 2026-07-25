//! Execution-owned planning contracts and first-slice runtime.
//!
//! Planning evaluates shared-language goals mechanically and returns an
//! execution composition artifact. Task lowering and dispatch stay outside this
//! domain.

/// Affordance-shaped available-action binding contracts.
pub mod action;
/// Planning request, result, diagnostic, and composition contracts.
pub mod contracts;
/// Lowering from execution compositions into task network mutation proposals.
pub mod lowering;
/// Method loading, verification, and deterministic ordering.
pub mod method_library;
/// Realization-route selection from the injected available-action set.
pub mod realization;
/// First-slice planning runtime and bounded actor facade.
pub mod runtime;
/// Execution-side world state projection request contracts.
pub mod world_state;

pub use action::{
    ActionArtifactMeaning, ActionOutcomeContractRef, ActionRealizationRoute,
    AvailableActionBinding, AvailableActionSet,
};
pub use contracts::{
    CandidateStatus, ExecutionComposition, InvalidMethodReport, MethodCandidateReport,
    NoApplicableMethod, OperatorResolutionReport, OperatorResolutionStatus, PlanningDiagnostic,
    PlanningDiagnosticCode, PlanningIndeterminate, PlanningInputError, PlanningRequest,
    PlanningResult, PlanningSatisfied,
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
pub use realization::{
    prepare_task_package_route, select_action_for_method, MethodRealizationBinding,
    RealizationSelectionError, TaskPackageRoutePlan,
};
pub use runtime::{
    PlanningProjectionError, PlanningProjectionPort, PlanningRuntime, PlanningRuntimeActor,
    PlanningRuntimeActorError, PlanningRuntimeActorGoalResult, PlanningRuntimeActorIssue,
    PlanningRuntimeActorReport, PlanningRuntimeActorRequest, PlanningWorldStateProjection,
};
pub use world_state::{PlanningWorldStateFrameRef, PlanningWorldStateRequest};
