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
    /// Satisfied variant for this execution contract.
    Satisfied(PlanningSatisfied),
    /// Composed variant for this execution contract.
    Composed(ExecutionComposition),
    /// No applicable method variant for this execution contract.
    NoApplicableMethod(NoApplicableMethod),
    /// Indeterminate variant for this execution contract.
    Indeterminate(PlanningIndeterminate),
    /// Invalid method variant for this execution contract.
    InvalidMethod(InvalidMethodReport),
}

/// Goal satisfaction outcome that did not require method selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanningSatisfied {
    /// Goal owned by this execution contract.
    pub goal: meld_lang::Goal,
    /// World state frame owned by this execution contract.
    pub world_state_frame: PlanningWorldStateFrameRef,
    /// Diagnostics owned by this execution contract.
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
    /// Goal owned by this execution contract.
    pub goal: meld_lang::Goal,
    /// World state frame owned by this execution contract.
    pub world_state_frame: PlanningWorldStateFrameRef,
    /// Candidates owned by this execution contract.
    pub candidates: Vec<MethodCandidateReport>,
    /// Diagnostics owned by this execution contract.
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// The projected world state lacked required facts for goal evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanningIndeterminate {
    /// Goal owned by this execution contract.
    pub goal: meld_lang::Goal,
    /// World state frame owned by this execution contract.
    pub world_state_frame: PlanningWorldStateFrameRef,
    /// Missing owned by this execution contract.
    pub missing: Vec<meld_lang::Term>,
    /// Diagnostics owned by this execution contract.
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// Method verification or composition preparation failed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvalidMethodReport {
    /// Source reference owned by this execution contract.
    pub source_ref: Option<String>,
    /// Method identifier carried across the execution boundary.
    pub method_id: Option<String>,
    /// Diagnostics owned by this execution contract.
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// Deterministic planning diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningDiagnostic {
    /// Code owned by this execution contract.
    pub code: PlanningDiagnosticCode,
    /// Message owned by this execution contract.
    pub message: String,
    /// Method identifier carried across the execution boundary.
    pub method_id: Option<String>,
    /// Step identifier carried across the execution boundary.
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
    /// Request missing identifier variant for this execution contract.
    RequestMissingId,
    /// Goal not active variant for this execution contract.
    GoalNotActive,
    /// Goal not ground variant for this execution contract.
    GoalNotGround,
    /// Goal satisfied variant for this execution contract.
    GoalSatisfied,
    /// Goal indeterminate variant for this execution contract.
    GoalIndeterminate,
    /// Method library invalid variant for this execution contract.
    MethodLibraryInvalid,
    /// Method identifier missing variant for this execution contract.
    MethodIdMissing,
    /// Method duplicate identifier variant for this execution contract.
    MethodDuplicateId,
    /// Method trigger derived variant for this execution contract.
    MethodTriggerDerived,
    /// Method variable not bound by trigger variant for this execution contract.
    MethodVariableNotBoundByTrigger,
    /// Method template invalid variant for this execution contract.
    MethodTemplateInvalid,
    /// Method trigger miss variant for this execution contract.
    MethodTriggerMiss,
    /// Method precondition unsatisfied variant for this execution contract.
    MethodPreconditionUnsatisfied,
    /// Method precondition indeterminate variant for this execution contract.
    MethodPreconditionIndeterminate,
    /// Method cost ceiling exceeded variant for this execution contract.
    MethodCostCeilingExceeded,
    /// Method effect projection failed variant for this execution contract.
    MethodEffectProjectionFailed,
    /// Method effect miss variant for this execution contract.
    MethodEffectMiss,
    /// Composition substitution failed variant for this execution contract.
    CompositionSubstitutionFailed,
    /// Composition validation failed variant for this execution contract.
    CompositionValidationFailed,
    /// Operator resolved variant for this execution contract.
    OperatorResolved,
    /// Operator unresolved variant for this execution contract.
    OperatorUnresolved,
    /// Operator tags diagnostic only variant for this execution contract.
    OperatorTagsDiagnosticOnly,
    /// No applicable method variant for this execution contract.
    NoApplicableMethod,
}

/// Per-method candidate evaluation report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodCandidateReport {
    /// Method identifier carried across the execution boundary.
    pub method_id: String,
    /// Lifecycle status assigned by the owning runtime.
    pub status: CandidateStatus,
    /// Bindings owned by this execution contract.
    pub bindings: Option<meld_lang::Bindings>,
    /// Diagnostics owned by this execution contract.
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// Candidate state after matching and mechanical checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateStatus {
    /// Trigger miss variant for this execution contract.
    TriggerMiss,
    /// Preconditions unsatisfied variant for this execution contract.
    PreconditionsUnsatisfied,
    /// Preconditions indeterminate variant for this execution contract.
    PreconditionsIndeterminate,
    /// Cost rejected variant for this execution contract.
    CostRejected,
    /// Effect projection failed variant for this execution contract.
    EffectProjectionFailed,
    /// Effect miss variant for this execution contract.
    EffectMiss,
    /// Applicable variant for this execution contract.
    Applicable,
}

/// Operator resolution diagnostic generated during method verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorResolutionReport {
    /// Operator identifier carried across the execution boundary.
    pub operator_id: String,
    /// Lifecycle status assigned by the owning runtime.
    pub status: OperatorResolutionStatus,
    /// Stable capability type identifier published by the owning domain.
    pub capability_type_id: Option<String>,
    /// Version of the published capability contract.
    pub capability_version: Option<u32>,
    /// Tags owned by this execution contract.
    pub tags: Vec<String>,
    /// Diagnostics owned by this execution contract.
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// Catalog resolution status for a composition operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatorResolutionStatus {
    /// Resolved variant for this execution contract.
    Resolved,
    /// Unresolved variant for this execution contract.
    Unresolved,
}

/// Invalid request errors stay outside successful planning outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanningInputError {
    /// Planning request did not provide a request identifier.
    MissingRequestId,
    /// Planning request targeted a goal that is not active.
    NonActiveGoal {
        /// Goal identifier rejected by lifecycle validation.
        goal_id: String,
    },
    /// Planning request targeted a goal with an unbound variable.
    NonGroundGoal {
        /// Goal identifier rejected by groundness validation.
        goal_id: String,
        /// Variable that remained unbound in the target goal.
        variable: String,
    },
    /// World state projection was built for a different goal.
    MismatchedWorldStateRequestGoal {
        /// Goal identifier requested from the world state projection.
        request_goal_id: String,
        /// Goal identifier supplied to planning.
        goal_id: String,
    },
}
