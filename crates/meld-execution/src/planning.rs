//! Execution-owned planning contracts and first-slice runtime.
//!
//! Planning evaluates shared-language goals mechanically and returns an
//! execution composition artifact. Task lowering and dispatch stay outside this
//! domain.

pub mod contracts;
pub mod method_library;
pub mod runtime;
pub mod world_state;

pub use contracts::{
    CandidateStatus, ExecutionComposition, InvalidMethodReport, MethodCandidateReport,
    NoApplicableMethod, OperatorResolutionReport, OperatorResolutionStatus, PlanningDiagnostic,
    PlanningDiagnosticCode, PlanningIndeterminate, PlanningInputError, PlanningRequest,
    PlanningResult, PlanningSatisfied,
};
pub use method_library::{
    MethodLibrary, MethodLibraryLoadError, MethodSourceRef, MethodVerification,
    MethodVerificationDiagnostic, VerifiedMethodEntry,
};
pub use runtime::PlanningRuntime;
pub use world_state::{PlanningWorldStateFrameRef, PlanningWorldStateRequest};
