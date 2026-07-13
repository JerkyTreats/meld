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

use crate::planning::world_state::{
    PlanningProjectionIdentityInputs, PlanningWorldStateFrameRef, PlanningWorldStateRequest,
};
use serde::{Deserialize, Serialize};

/// Complete immutable inputs for deterministic planning request identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningRequestIdentityInputs {
    /// Stable goal id.
    pub goal_id: String,
    /// Goal update sequence observed by planning.
    pub goal_updated_at_seq: u64,
    /// Canonical inputs that derive the durable projection frame id.
    pub projection_identity: PlanningProjectionIdentityInputs,
    /// Digest of the verified method library.
    pub method_library_digest: String,
    /// Digest of the capability catalog visible to lowering.
    pub capability_catalog_digest: String,
    /// Planning algorithm version.
    pub planning_version: String,
}

/// Durable identity derived from complete planning inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningRequestIdentity {
    /// Stable request id derived from the complete input record.
    request_id: String,
    /// Immutable source inputs retained for replay diagnostics.
    inputs: PlanningRequestIdentityInputs,
}

impl PlanningRequestIdentity {
    /// Derive one request identity from complete immutable inputs.
    pub fn derive(inputs: PlanningRequestIdentityInputs) -> Result<Self, String> {
        let encoded = serde_json::to_vec(&inputs).map_err(|error| error.to_string())?;
        Ok(Self {
            request_id: blake3::hash(&encoded).to_hex().to_string(),
            inputs,
        })
    }

    /// Borrow the derived request id.
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Borrow the complete immutable identity inputs.
    pub fn inputs(&self) -> &PlanningRequestIdentityInputs {
        &self.inputs
    }

    /// Reject a request id that disagrees with its immutable inputs.
    pub fn validate(&self) -> Result<(), String> {
        let expected = Self::derive(self.inputs.clone())?;
        if expected.request_id == self.request_id {
            Ok(())
        } else {
            Err("planning request id does not match its immutable inputs".to_string())
        }
    }
}

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

/// Planning request bound to complete deterministic replay identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "IdentifiedPlanningRequestWire")]
pub struct IdentifiedPlanningRequest {
    /// Canonical planning request product.
    request: PlanningRequest,
    /// Identity derived from every required replay input.
    identity: PlanningRequestIdentity,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct IdentifiedPlanningRequestWire {
    request: PlanningRequest,
    identity: PlanningRequestIdentity,
}

impl IdentifiedPlanningRequest {
    /// Bind a canonical planning request to complete immutable identity inputs.
    pub fn bind(
        mut request: PlanningRequest,
        inputs: PlanningRequestIdentityInputs,
    ) -> Result<Self, String> {
        validate_request_projection_binding(&request, &inputs)?;
        let identity = PlanningRequestIdentity::derive(inputs)?;
        request.request_id = identity.request_id().to_string();
        Ok(Self { request, identity })
    }

    /// Validate the attached identity against the canonical request.
    pub fn validate(&self) -> Result<(), String> {
        self.identity.validate()?;
        validate_request_projection_binding(&self.request, &self.identity.inputs)?;
        if self.request.request_id != self.identity.request_id {
            return Err("planning request tracing id does not match derived identity".to_string());
        }
        Ok(())
    }

    /// Borrow the canonical planning request.
    pub fn request(&self) -> &PlanningRequest {
        &self.request
    }

    /// Borrow the attached deterministic identity.
    pub fn identity(&self) -> &PlanningRequestIdentity {
        &self.identity
    }
}

fn validate_request_projection_binding(
    request: &PlanningRequest,
    inputs: &PlanningRequestIdentityInputs,
) -> Result<(), String> {
    if inputs.goal_id != request.goal.goal_id
        || request.world_state_request.goal_id != request.goal.goal_id
        || request.world_state_request.agent_id != request.goal.agent_id
        || request.world_state_request.target != request.goal.target
    {
        return Err("planning projection request does not match the selected goal".to_string());
    }
    inputs
        .projection_identity
        .validate_for_projection(&request.world_state_request, &request.world_state)?;
    let expected_frame = inputs
        .projection_identity
        .frame_ref(request.world_state_frame.warnings.clone())?;
    if expected_frame != request.world_state_frame {
        return Err("planning identity projection does not match request projection".to_string());
    }
    Ok(())
}

impl TryFrom<IdentifiedPlanningRequestWire> for IdentifiedPlanningRequest {
    type Error = String;

    fn try_from(value: IdentifiedPlanningRequestWire) -> Result<Self, Self::Error> {
        let identified = Self {
            request: value.request,
            identity: value.identity,
        };
        identified.validate()?;
        Ok(identified)
    }
}

/// Runtime planning outcome for one goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlanningResult {
    /// Goal already satisfied by the projected world state.
    Satisfied(PlanningSatisfied),
    /// Executable composition selected for the goal.
    Composed(ExecutionComposition),
    /// No verified method could satisfy the goal.
    NoApplicableMethod(NoApplicableMethod),
    /// World state was insufficient to evaluate the goal.
    Indeterminate(PlanningIndeterminate),
    /// Method loading or verification failed before planning could complete.
    InvalidMethod(InvalidMethodReport),
}

/// Goal satisfaction outcome that did not require method selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanningSatisfied {
    /// Goal proven satisfied by the supplied world state.
    pub goal: meld_lang::Goal,
    /// World state frame used for the satisfaction check.
    pub world_state_frame: PlanningWorldStateFrameRef,
    /// Deterministic diagnostics emitted during satisfaction checking.
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
    /// Goal that no verified candidate could satisfy.
    pub goal: meld_lang::Goal,
    /// World state frame used while evaluating candidates.
    pub world_state_frame: PlanningWorldStateFrameRef,
    /// Per-method candidate reports from trigger and verification checks.
    pub candidates: Vec<MethodCandidateReport>,
    /// Deterministic diagnostics explaining why no method applied.
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// The projected world state lacked required facts for goal evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanningIndeterminate {
    /// Goal that could not be decided from the projected world state.
    pub goal: meld_lang::Goal,
    /// World state frame used for the indeterminate check.
    pub world_state_frame: PlanningWorldStateFrameRef,
    /// Terms needed before the goal can be evaluated deterministically.
    pub missing: Vec<meld_lang::Term>,
    /// Deterministic diagnostics explaining the missing world state.
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// Method verification or composition preparation failed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvalidMethodReport {
    /// Method source reference when the invalid source is known.
    pub source_ref: Option<String>,
    /// Method identifier carried across the execution boundary.
    pub method_id: Option<String>,
    /// Diagnostics emitted while loading or verifying the method.
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// Deterministic planning diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanningDiagnostic {
    /// Stable machine-readable diagnostic code.
    pub code: PlanningDiagnosticCode,
    /// Human-readable diagnostic message.
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
    /// Planning request omitted its tracing identifier.
    RequestMissingId,
    /// Target goal was not active.
    GoalNotActive,
    /// Target goal still had unbound variables.
    GoalNotGround,
    /// Projected world state already satisfied the goal.
    GoalSatisfied,
    /// Projected world state lacked terms needed to evaluate the goal.
    GoalIndeterminate,
    /// Method library contained invalid entries.
    MethodLibraryInvalid,
    /// Method did not declare a stable identifier.
    MethodIdMissing,
    /// Method identifier was duplicated.
    MethodDuplicateId,
    /// Trigger bindings were derived for a candidate method.
    MethodTriggerDerived,
    /// Method template referenced a variable not bound by its trigger.
    MethodVariableNotBoundByTrigger,
    /// Method template failed structural validation.
    MethodTemplateInvalid,
    /// Method trigger did not match the target goal.
    MethodTriggerMiss,
    /// Method precondition evaluated false.
    MethodPreconditionUnsatisfied,
    /// Method precondition could not be decided.
    MethodPreconditionIndeterminate,
    /// Method exceeded the configured cost ceiling.
    MethodCostCeilingExceeded,
    /// Method effects could not be projected.
    MethodEffectProjectionFailed,
    /// Projected effects did not satisfy the target goal.
    MethodEffectMiss,
    /// Template substitution failed while building a composition.
    CompositionSubstitutionFailed,
    /// Concrete composition failed validation.
    CompositionValidationFailed,
    /// Operator matched a published capability contract.
    OperatorResolved,
    /// Operator had no compatible capability contract.
    OperatorUnresolved,
    /// Operator tags produced an informational diagnostic.
    OperatorTagsDiagnosticOnly,
    /// No candidate survived planning checks.
    NoApplicableMethod,
}

#[cfg(test)]
mod contract_freeze_tests {
    use super::*;

    #[test]
    fn planning_request_identity_retains_every_replay_input() {
        let subject = meld_events::DomainObjectRef::new("workspace", "node", "readme").unwrap();
        let projection_request = PlanningWorldStateRequest {
            goal_id: "goal-a".to_string(),
            agent_id: "agent-a".to_string(),
            subject: subject.clone(),
            source_seq: 9,
            target: meld_lang::Proposition::Accessible {
                scope: meld_lang::Term::Object(subject),
            },
            perspective: crate::planning::world_state::PlanningPerspectiveRef::new(
                "agent", "agent-a",
            )
            .unwrap(),
            branch_id: "main".to_string(),
            requested_dimensions: vec!["docs_freshness".to_string()],
            required_preconditions: Vec::new(),
        };
        let world_state = meld_lang::WorldState::empty();
        let frame = PlanningWorldStateFrameRef::identified_from_authority(
            "projection-v1",
            blake3::hash(b"projection-a").to_hex().to_string(),
            crate::planning::world_state::canonical_world_state_hash(&world_state).unwrap(),
            &projection_request,
            &world_state,
            vec![serde_json::json!({"ProjectionRule": {"rule_id": "source-a"}}).to_string()],
            Vec::new(),
        )
        .unwrap();
        let inputs = PlanningRequestIdentityInputs {
            goal_id: "goal-a".to_string(),
            goal_updated_at_seq: 9,
            projection_identity: PlanningProjectionIdentityInputs::from_projection(
                &projection_request,
                &world_state,
                &frame,
            )
            .unwrap(),
            method_library_digest: "method-a".to_string(),
            capability_catalog_digest: "catalog-a".to_string(),
            planning_version: "planning-v1".to_string(),
        };
        let identity = PlanningRequestIdentity::derive(inputs).unwrap();

        let encoded = serde_json::to_vec(&identity).unwrap();
        let decoded: PlanningRequestIdentity = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, identity);
        assert!(decoded.validate().is_ok());
    }

    #[test]
    fn identified_request_rejects_projection_identity_from_another_request() {
        let subject = meld_events::DomainObjectRef::new("workspace", "node", "readme").unwrap();
        let goal = meld_lang::Goal {
            goal_id: "goal-a".to_string(),
            agent_id: "agent-a".to_string(),
            target: meld_lang::Proposition::Accessible {
                scope: meld_lang::Term::Object(subject.clone()),
            },
            priority: meld_lang::GoalPriority {
                urgency: 1,
                cost_ceiling: None,
            },
            source: meld_lang::GoalSource::UserDirected {
                directive: "test".to_string(),
            },
            lifecycle: meld_lang::GoalLifecycle::Active,
        };
        let request = PlanningWorldStateRequest {
            goal_id: goal.goal_id.clone(),
            agent_id: goal.agent_id.clone(),
            subject,
            source_seq: 9,
            target: goal.target.clone(),
            perspective: crate::planning::world_state::PlanningPerspectiveRef::new(
                "agent", "agent-a",
            )
            .unwrap(),
            branch_id: "main".to_string(),
            requested_dimensions: Vec::new(),
            required_preconditions: Vec::new(),
        };
        let mut other_request = request.clone();
        other_request.branch_id = "other".to_string();
        let world_state = meld_lang::WorldState::empty();
        let frame = PlanningWorldStateFrameRef::identified_from_authority(
            "projection-v1",
            blake3::hash(b"projection-other").to_hex().to_string(),
            crate::planning::world_state::canonical_world_state_hash(&world_state).unwrap(),
            &other_request,
            &world_state,
            vec![serde_json::json!({"ProjectionRule": {"rule_id": "source-a"}}).to_string()],
            Vec::new(),
        )
        .unwrap();
        let projection_identity =
            PlanningProjectionIdentityInputs::from_projection(&other_request, &world_state, &frame)
                .unwrap();
        let planning_request = PlanningRequest {
            request_id: String::new(),
            goal,
            world_state,
            world_state_frame: frame,
            world_state_request: request,
        };
        let inputs = PlanningRequestIdentityInputs {
            goal_id: "goal-a".to_string(),
            goal_updated_at_seq: 9,
            projection_identity,
            method_library_digest: "method-a".to_string(),
            capability_catalog_digest: "catalog-a".to_string(),
            planning_version: "planning-v1".to_string(),
        };

        assert!(IdentifiedPlanningRequest::bind(planning_request, inputs).is_err());
    }
}

/// Per-method candidate evaluation report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodCandidateReport {
    /// Method identifier carried across the execution boundary.
    pub method_id: String,
    /// Lifecycle status assigned by the owning runtime.
    pub status: CandidateStatus,
    /// Trigger bindings derived for the candidate when matching succeeded.
    pub bindings: Option<meld_lang::Bindings>,
    /// Diagnostics emitted while evaluating this candidate.
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// Candidate state after matching and mechanical checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateStatus {
    /// Trigger did not match the target goal.
    TriggerMiss,
    /// Preconditions evaluated false.
    PreconditionsUnsatisfied,
    /// Preconditions could not be decided from world state.
    PreconditionsIndeterminate,
    /// Candidate exceeded planning cost limits.
    CostRejected,
    /// Candidate effects could not be projected.
    EffectProjectionFailed,
    /// Projected effects did not satisfy the goal.
    EffectMiss,
    /// Candidate passed all mechanical planning checks.
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
    /// Tags considered while matching operators to capabilities.
    pub tags: Vec<String>,
    /// Diagnostics emitted during operator resolution.
    pub diagnostics: Vec<PlanningDiagnostic>,
}

/// Catalog resolution status for a composition operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatorResolutionStatus {
    /// Operator resolved to a compatible capability.
    Resolved,
    /// Operator did not resolve to a compatible capability.
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
    /// Deterministic projection or planning identity did not match its inputs.
    IdentityMismatch {
        /// Stable validation detail for diagnostics.
        message: String,
    },
}
