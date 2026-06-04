//! Execution-owned planning contracts and first-slice runtime.
//!
//! Planning evaluates shared-language goals mechanically and returns an
//! execution composition artifact. Task lowering and dispatch stay outside this
//! domain.

/// Planning request, result, diagnostic, and composition contracts.
pub mod contracts;
/// Lowering from execution compositions into task network mutation proposals.
pub mod lowering;
/// Method loading, verification, and deterministic ordering.
pub mod method_library;
/// First-slice planning runtime facade.
pub mod runtime;
/// Execution-side world state projection request contracts.
pub mod world_state;

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
pub use runtime::PlanningRuntime;
pub use world_state::{PlanningWorldStateFrameRef, PlanningWorldStateRequest};
