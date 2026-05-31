//! Public planning request, result, and diagnostic contracts.
//!
//! # Example
//!
//! ```rust
//! use meld_execution::planning::{
//!     PlanningDiagnostic, PlanningDiagnosticCode,
//! };
//!
//! let diagnostic = PlanningDiagnostic::new(
//!     PlanningDiagnosticCode::MethodPreconditionUnsatisfied,
//!     "method precondition did not match projected world state",
//! )
//! .with_method("refresh-docs")
//! .with_step("check-current-state");
//!
//! assert_eq!(diagnostic.method_id.as_deref(), Some("refresh-docs"));
//! assert_eq!(diagnostic.step_id.as_deref(), Some("check-current-state"));
//! ```

use crate::planning::world_state::{PlanningWorldStateFrameRef, PlanningWorldStateRequest};
use serde::{Deserialize, Serialize};

/// Complete first-slice planning input for one active goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanningRequest {
    /// Caller supplied id for tracing one planning request.
    pub request_id: String,
    /// Active ground goal selected from execution goal storage.
    pub goal: meld_lang::Goal,
    /// Goal scoped world state projection supplied by the world model boundary.
    pub world_state: meld_lang::WorldState,
    /// Provenance for the projected world state.
    pub world_state_frame: PlanningWorldStateFrameRef,
    /// Projection request that produced the supplied world state.
    pub world_state_request: PlanningWorldStateRequest,
}

/// Runtime planning outcome for one goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlanningResult {
    Satisfied(PlanningSatisfied),
    Composed(ExecutionComposition),
    NoApplicableMethod(NoApplicableMethod),
    Indeterminate(PlanningIndeterminate),
    InvalidMethod(InvalidMethodReport),
}

/// Goal satisfaction outcome that did not require method selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanningSatisfied {
    pub goal: meld_lang::Goal,
    pub world_state_frame: PlanningWorldStateFrameRef,
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// Concrete composition plus the planning context needed by later lowering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionComposition {
    /// Deterministic handoff id derived from goal and method identity.
    pub composition_id: String,
    /// Goal this composition is intended to satisfy.
    pub goal: meld_lang::Goal,
    /// World state frame used when selecting the method.
    pub world_state_frame: PlanningWorldStateFrameRef,
    /// Selected method id.
    pub method_id: String,
    /// Trigger bindings applied to the method template.
    pub bindings: meld_lang::Bindings,
    /// Concrete composition after substitution.
    pub composition: meld_lang::Composition,
    /// Effects projected during achievement checking.
    pub projected_effects: Vec<meld_lang::Effect>,
    /// Operator to capability resolution diagnostics from method verification.
    pub operator_resolutions: Vec<OperatorResolutionReport>,
    /// Structural validation result for the concrete composition.
    pub validation: meld_lang::ValidationResult,
    /// Deterministic diagnostics explaining selection and preparation.
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// No verified method passed all mechanical planning checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoApplicableMethod {
    pub goal: meld_lang::Goal,
    pub world_state_frame: PlanningWorldStateFrameRef,
    pub candidates: Vec<MethodCandidateReport>,
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// The projected world state lacked required facts for goal evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanningIndeterminate {
    pub goal: meld_lang::Goal,
    pub world_state_frame: PlanningWorldStateFrameRef,
    pub missing: Vec<meld_lang::Term>,
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// Method verification or composition preparation failed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvalidMethodReport {
    pub source_ref: Option<String>,
    pub method_id: Option<String>,
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// Deterministic planning diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningDiagnostic {
    pub code: PlanningDiagnosticCode,
    pub message: String,
    pub method_id: Option<String>,
    pub step_id: Option<String>,
}

impl PlanningDiagnostic {
    /// Build a diagnostic without method or step context.
    pub fn new(code: PlanningDiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            method_id: None,
            step_id: None,
        }
    }

    /// Attach method context to this diagnostic.
    pub fn with_method(mut self, method_id: impl Into<String>) -> Self {
        self.method_id = Some(method_id.into());
        self
    }

    /// Attach step context to this diagnostic.
    pub fn with_step(mut self, step_id: impl Into<String>) -> Self {
        self.step_id = Some(step_id.into());
        self
    }
}

/// Stable diagnostic code for planning and method reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanningDiagnosticCode {
    RequestMissingId,
    GoalNotActive,
    GoalNotGround,
    GoalSatisfied,
    GoalIndeterminate,
    MethodLibraryInvalid,
    MethodIdMissing,
    MethodDuplicateId,
    MethodTriggerDerived,
    MethodVariableNotBoundByTrigger,
    MethodTemplateInvalid,
    MethodTriggerMiss,
    MethodPreconditionUnsatisfied,
    MethodPreconditionIndeterminate,
    MethodCostCeilingExceeded,
    MethodEffectProjectionFailed,
    MethodEffectMiss,
    CompositionSubstitutionFailed,
    CompositionValidationFailed,
    OperatorResolved,
    OperatorUnresolved,
    OperatorTagsDiagnosticOnly,
    NoApplicableMethod,
}

/// Per-method candidate evaluation report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodCandidateReport {
    pub method_id: String,
    pub status: CandidateStatus,
    pub bindings: Option<meld_lang::Bindings>,
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// Candidate state after matching and mechanical checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateStatus {
    TriggerMiss,
    PreconditionsUnsatisfied,
    PreconditionsIndeterminate,
    CostRejected,
    EffectProjectionFailed,
    EffectMiss,
    Applicable,
}

/// Operator resolution diagnostic generated during method verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorResolutionReport {
    pub operator_id: String,
    pub status: OperatorResolutionStatus,
    pub capability_type_id: Option<String>,
    pub capability_version: Option<u32>,
    pub tags: Vec<String>,
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// Catalog resolution status for a composition operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatorResolutionStatus {
    Resolved,
    Unresolved,
}

/// Invalid request errors stay outside successful planning outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanningInputError {
    MissingRequestId,
    NonActiveGoal {
        goal_id: String,
    },
    NonGroundGoal {
        goal_id: String,
        variable: String,
    },
    MismatchedWorldStateRequestGoal {
        request_goal_id: String,
        goal_id: String,
    },
}
